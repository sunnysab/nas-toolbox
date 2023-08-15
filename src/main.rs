mod duplicate;
mod hash;
mod metadata;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::io::BufWriter;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use crate::duplicate::{ScanFilter, StatusReport};
use crate::hash::CompareMode;
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
}

#[derive(Args)]
struct ScanArg {
    /// The directory to scan
    path: PathBuf,
    /// Compare complete file content
    #[arg(long, default_value_t = false)]
    compare_full: bool,
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
    list: PathBuf,
}
#[derive(Subcommand)]
enum Commands {
    Scan(ScanArg),
    Dedup(DedupArg),
}

fn display_file_size(len: u64) -> String {
    let mut n: u64 = 1024 * 1024 * 1024;
    let mut r = len / n;
    let t = ["GB", "MB", "KB", "Byte"];

    if len == 0 {
        return String::new();
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

    let mut count = 0;
    for file_group in duplicate.result() {
        count += 1;

        let del_count = file_group.len() as u64 - 1;
        let size = display_file_size(file_group[0].metadata.size);
        let total_size = display_file_size(file_group[0].metadata.size * del_count);
        let occupied = display_file_size(file_group[0].metadata.blocks * 512 * del_count);
        writeln!(
            &mut buffer,
            "# group {count}, {del_count} * {size} = {total_size} ({occupied} in disk) can be saved."
        )?;

        if let [first, rest @ ..] = file_group.as_slice() {
            writeln!(&mut buffer, "# Keep {}: {}", first.metadata.ino, first.path.display())?;
            let source = first.path.display();
            for &file_to_del in rest {
                let destination = file_to_del.path.display();
                writeln!(&mut buffer, "# Remove {}: {}", file_to_del.metadata.ino, destination)?;
                writeln!(&mut buffer, "ln -f '{source}' '{destination}'")?;
                writeln!(&mut buffer)?;
            }
        }
    }
    Ok(())
}

fn report<F: ScanFilter>(duplicate: &Duplicate<F>, output: Option<PathBuf>, format: OutputFormat) -> Result<()> {
    if let OutputFormat::Html = format {
        unimplemented!()
    }

    match format {
        OutputFormat::Html => {}
        OutputFormat::Script => {
            let path = output.unwrap_or_else(|| PathBuf::from("./dedup.sh"));
            generate_dedup_script(duplicate, &path).expect("unable to generate script.");
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
            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if ch_width > remaining_width {
                break;
            } else {
                remaining_width -= ch_width;
                len += ch_width;
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

    let compare_mode = match (arg.compare_full, arg.compare_size) {
        (true, _) => CompareMode::Full,
        (_, size_str) => {
            let size_value = parse_file_size(&size_str);
            CompareMode::Part(size_value)
        }
    };

    let rx = duplicate.enable_status_channel(200);
    thread::spawn(move || {
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

    let instant = Instant::now();
    duplicate.discover(compare_mode).expect("Error occurred while discovering.");
    let duration = instant.elapsed();
    println!("\nDiscovering finished, {}s elapsed.", duration.as_secs());

    report(&duplicate, arg.output, arg.format).expect("report failed");
}

fn dedup() {}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Scan(arg) => scan(arg),
        Commands::Dedup(_arg) => {}
    }
}
