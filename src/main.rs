mod duplicate;
mod hash;
mod inventory;
mod metadata;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use unicode_width::UnicodeWidthChar;

use crate::duplicate::{ScanFilter, StatusReport};
use crate::hash::CompareMode;
use crate::inventory::{DuplicateFile, DuplicateGroup, InventoryReader, InventoryWriter};
use duplicate::{DefaultFilter, Duplicate};

const DEFAULT_COMPARE_SIZE: &str = "1M";
const DEFAULT_OUTPUT_FORMAT: OutputFormat = OutputFormat::Script;

#[derive(Parser)]
#[command(name = "d2fn")]
#[command(author = "sunnysab <i@sunnysab.cn>")]
#[command(version = "0.1")]
#[command(about = "DeDuplicate File on NAS")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    /// Generate a web-page report.
    Html,
    /// Output a shell script that can dedup files.
    Script,
    /// Duplicates inventory
    Inventory,
}

#[derive(Args)]
struct ScanArg {
    /// The directory to scan
    path: PathBuf,
    /// Verify the full content to file
    #[arg(long, default_value_t = false)]
    verify: bool,
    /// Compare size
    #[arg(long, default_value_t = DEFAULT_COMPARE_SIZE.to_string())]
    compare_size: String,
    /// Output format
    #[arg(short, long, value_enum, default_value_t = DEFAULT_OUTPUT_FORMAT)]
    format: OutputFormat,
    /// Output path
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct DedupArg {
    inventory: PathBuf,
}

#[derive(Args)]
struct HashArg {
    /// The file to hash
    file: String,

    /// Compare complete file content
    #[arg(long, default_value_t = false)]
    full: bool,
    /// Compare size
    #[arg(long, default_value_t = DEFAULT_COMPARE_SIZE.to_string())]
    hash_size: String,
}

#[derive(Subcommand)]
enum Commands {
    Scan(ScanArg),
    Dedup(DedupArg),
    Hash(HashArg),
}

fn display_duration(secs: u64) -> String {
    let (hour, min, sec) = (secs / 3600, secs % 3600 / 60, secs % 60);
    let mut result = String::new();

    if hour != 0 {
        result.push_str(&format!("{hour}h"));
    }
    if min != 0 {
        result.push_str(&format!("{min}m"));
    }
    if sec > 0 || result.is_empty() {
        result.push_str(&format!("{sec}s"));
    }
    result
}

fn display_file_size(len: u64) -> String {
    let mut n: u64 = 1024 * 1024 * 1024;
    let mut r = len / n;
    let t = ["GB", "MB", "KB", "Byte"];

    if len == 0 {
        return "0B".to_string();
    }
    let mut i: usize = 0;
    while r == 0 {
        n /= 1024;
        r = len / n;
        i += 1;
    }
    format!("{}{}", r, t[i])
}

/// Parse user input size "1G", "1GB", "1MB"... to a usize.
fn parse_file_size(text: &str) -> usize {
    let mut num = 0usize;
    let mut last_i = 0usize;
    for (i, c) in text.char_indices() {
        if c.is_ascii_digit() {
            num = num * 10 + (c as usize) - 48;
        } else {
            last_i = i;
            break;
        }
    }

    let unit = text[last_i..].to_lowercase();
    let unit = match unit.as_str() {
        "g" | "gb" => 1024 * 1024 * 1024usize,
        "m" | "mb" => 1024 * 1024usize,
        "k" | "kb" => 1024usize,
        _ => panic!("unexpected size {unit}"),
    };
    num * unit
}

fn generate_dedup_script<F: ScanFilter>(duplicate: &Duplicate<F>, output: &Path) -> Result<()> {
    let script = std::fs::File::create(output).with_context(|| format!("failed to create {}.", output.display()))?;
    let mut buffer = BufWriter::new(script);
    writeln!(&mut buffer, "#/usr/bin/bash")?;
    writeln!(&mut buffer, "set -e")?;
    writeln!(&mut buffer)?;

    let (mut group, mut dup_count) = (0, 0);
    let mut total_size_across_group = 0;
    let mut block_size_across_group = 0;
    for file_group in duplicate.result() {
        group += 1;

        let del_count = file_group.len() as u64 - 1;
        let size = display_file_size(file_group[0].metadata.size);
        let total_size = display_file_size(file_group[0].metadata.size * del_count);
        let occupied = display_file_size(file_group[0].metadata.blocks * 512 * del_count);
        writeln!(
            &mut buffer,
            "# group {group}, {del_count} * {size} = {total_size} ({occupied} in disk) can be saved."
        )?;

        if let [first, rest @ ..] = file_group.as_slice() {
            writeln!(&mut buffer, "# Keep {}: {}", first.metadata.ino, first.path.display())?;
            let source = first.path.display();
            for &file_to_del in rest {
                let destination = file_to_del.path.display();
                writeln!(&mut buffer, "# Remove {}: {}", file_to_del.metadata.ino, destination)?;
                writeln!(&mut buffer, "ln -f '{source}' '{destination}'")?;
                writeln!(&mut buffer)?;
                dup_count += 1;

                if dup_count % 50 == 0 {
                    writeln!(&mut buffer, "echo -n -e '{dup_count}'")?;
                }
            }
        }

        total_size_across_group += file_group[0].metadata.size * del_count;
        block_size_across_group += file_group[0].metadata.blocks * 512 * del_count;
    }

    println!(
        "{} files ({} on disk) can be cleaned.",
        display_file_size(total_size_across_group),
        display_file_size(block_size_across_group)
    );
    println!("Script has been written to {}", output.display());
    println!("Remember to grant execute permission before you run it.");

    let inventory_path = Path::new("inventory.d2fn");
    generate_inventory(duplicate, inventory_path)?;
    Ok(())
}

fn generate_html<F: ScanFilter>(duplicate: &Duplicate<F>, output: &Path, scan: &ScanArg) -> Result<()> {
    let mut html = std::fs::File::create(output).with_context(|| format!("failed to create {}.", output.display()))?;
    let html_template: &'static str = include_str!("../template/report.html");

    #[derive(serde::Serialize)]
    struct FileSummary {
        ino: u64,
        path: String,
        size: String,
    }

    #[derive(serde::Serialize)]
    struct Group {
        index: usize,
        files: Vec<FileSummary>,
    }
    let mut mapped_groups = Vec::new();
    for (group_index, group) in duplicate.result().enumerate() {
        let files = group
            .into_iter()
            .map(|file_ref| {
                let path = file_ref.path.strip_prefix(&scan.path).unwrap_or(&file_ref.path);
                FileSummary {
                    ino: file_ref.metadata.ino,
                    path: path.to_string_lossy().to_string(),
                    size: display_file_size(file_ref.metadata.size),
                }
            })
            .collect::<Vec<_>>();
        mapped_groups.push(Group {
            index: group_index + 1,
            files,
        });
    }

    let mut context = tera::Context::new();
    context.insert("path", &scan.path.to_string_lossy().to_string());
    context.insert("group_count", &mapped_groups.len());
    context.insert("groups", &mapped_groups);
    let parameter = if scan.verify {
        "快速 + 完整内容验证".to_string()
    } else {
        format!("快速，仅比较前 {}", scan.compare_size)
    };
    context.insert("parameter", &parameter);

    let content =
        tera::Tera::one_off(html_template, &context, false).with_context(|| "unable to render html".to_string())?;
    html.write_all(content.as_bytes())
        .with_context(|| "when write to file".to_string())?;
    println!("Report has been written to {}.", output.display());

    let inventory_path = Path::new("inventory.d2fn");
    generate_inventory(duplicate, inventory_path)?;
    Ok(())
}

fn generate_inventory<F: ScanFilter>(duplicate: &Duplicate<F>, output: &Path) -> Result<()> {
    println!("Writing result inventory....");

    let mut writer = InventoryWriter::create(output)?;
    let iter = duplicate.result().map(|group| {
        let files = group
            .iter()
            .map(|&file_ref| DuplicateFile {
                ino: file_ref.metadata.ino,
                path: file_ref.path.clone(),
            })
            .collect::<Vec<_>>();

        DuplicateGroup { files }
    });

    writer.export(iter)?;
    println!("Inventory exported.");
    Ok(())
}

fn report<F: ScanFilter>(duplicate: &Duplicate<F>, arg: &ScanArg) -> Result<()> {
    let path = arg.output.clone();

    match arg.format {
        OutputFormat::Html => {
            let path = path.unwrap_or_else(|| PathBuf::from("report.html"));
            generate_html(duplicate, &path, arg).expect("unable to generate report page.");
        }
        OutputFormat::Script => {
            let path = path.unwrap_or_else(|| PathBuf::from("dedup.sh"));
            generate_dedup_script(duplicate, &path).expect("unable to generate script.");
        }
        OutputFormat::Inventory => {
            let path = path.unwrap_or_else(|| PathBuf::from("inventory.d2fn"));
            generate_inventory(duplicate, &path).expect("unable to generate inventory file.");
        }
    }
    Ok(())
}

fn print_progress(status: StatusReport, width: usize) {
    let blank_line = " ".repeat(width);
    let clear_line = || print!("\r{blank_line}\r");

    fn get_truncated_content(text: &str, mut remaining_width: usize) -> &str {
        let mut len = 0usize;
        for ch in text.chars() {
            let ch_width = ch.width().unwrap_or(0);
            if ch_width > remaining_width {
                break;
            } else {
                remaining_width -= ch_width;
                len += ch.len_utf8();
            }
        }
        &text[..len]
    }

    clear_line();
    let count = format!("S {}/D {}: ", status.scanned, status.duplicated);
    print!("{count}{}", get_truncated_content(&status.last_file, width - count.len()));

    std::io::stdout().flush().unwrap();
}

fn scan(arg: ScanArg) {
    println!("Scanning on {}...", arg.path.display());
    println!("File type filter: {:?}", DefaultFilter::ext_set());
    let mut duplicate = Duplicate::new(&arg.path).custom_filter(DefaultFilter::new());

    let rx = duplicate.enable_status_channel(30);
    std::thread::spawn(move || {
        let start = Instant::now();
        let mut delta_milli_sec = 0;

        let (terminal_size::Width(width), _) =
            terminal_size::terminal_size().unwrap_or((terminal_size::Width(80), terminal_size::Height(25)));

        println!("S = Scanned files, D = Duplicates");
        // 当 scan 函数结束后, channel 会关闭, 由此子线程 recv 也会关闭.
        while let Ok(status) = rx.recv() {
            if start.elapsed().as_millis() > delta_milli_sec {
                print_progress(status, width as usize);
                delta_milli_sec += 250; // 平均一秒最多刷新 4 次.
            }
        }
    });

    let compare_size = parse_file_size(&arg.compare_size);
    let instant = Instant::now();
    duplicate.discover(compare_size).expect("Error occurred while discovering.");
    let duration = instant.elapsed();
    println!("\nDiscovering finished, {} elapsed.", display_duration(duration.as_secs()));

    if arg.verify {
        println!("Trying to verify duplicate list, which may take a while...");
        let instant = Instant::now();
        let conflict_count = duplicate.verify().expect("Error occurred while verifying.");
        let duration = instant.elapsed();
        println!(
            "{conflict_count} conflicts detected, costs {}.",
            display_duration(duration.as_secs())
        );
    }
    report(&duplicate, &arg).expect("report failed");
}

fn dedup(arg: DedupArg) {
    let path = &arg.inventory.as_path();
    let reader = InventoryReader::open(path).expect("unable to open inventory.");

    println!("{} in total..", reader.total());
    for group in reader {
        let group = match group {
            Ok(g) => g,
            Err(e) => {
                eprintln!("error: when read duplicate group, {e}");
                continue;
            }
        };

        if let [first, rest @ ..] = group.files.as_slice() {
            let source = &first.path;
            for dup in rest {
                let destination = &dup.path;

                let result = std::fs::remove_file(destination).and_then(|_| std::fs::hard_link(source, destination));
                if let Err(e) = result {
                    eprintln!("failed on {} :{e}", dup.ino);
                }
            }
        }
    }
}

fn hash(arg: HashArg) {
    let hash_mode = match (arg.full, arg.hash_size) {
        (true, _) => CompareMode::Full,
        (_, size_str) => {
            let size_value = parse_file_size(&size_str);
            CompareMode::Part(size_value)
        }
    };

    let checksum = hash::checksum_file(arg.file, hash_mode).expect("failed on hash::checksum_file.");
    println!("{checksum}");
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Scan(arg) => scan(arg),
        Commands::Dedup(arg) => dedup(arg),
        Commands::Hash(arg) => hash(arg),
    }
    println!("Done.");
}
