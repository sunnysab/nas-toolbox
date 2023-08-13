mod duplicate;
mod hash;
mod metadata;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;

use duplicate::{DefaultFilter, Duplicate};

const DEFAULT_COMPARE_SIZE: &str = "1M";

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
    path: std::path::PathBuf,
    /// Compare complete file content
    #[arg(long, default_value_t = false)]
    compare_full: bool,
    /// Compare size
    #[arg(long, default_value_t = DEFAULT_COMPARE_SIZE.to_string())]
    compare_size: String,
    /// Output format
    #[arg(value_enum)]
    format: OutputFormat,
    /// Output path
    output: std::path::PathBuf,
}

#[derive(Args)]
struct DedupArg {
    list: std::path::PathBuf,
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

fn report(duplicate: &Duplicate, output: &Path, format: OutputFormat) -> Result<()> {
    if let OutputFormat::Html = format {
        unimplemented!()
    }

    let script =
        std::fs::File::create(output).with_context(|| format!("failed to open output file."))?;
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
            writeln!(
                &mut buffer,
                "# Keep {}: {}",
                first.metadata.ino,
                first.path.display()
            )?;
            let source = first.path.display();
            for &file_to_del in rest {
                let destination = file_to_del.path.display();
                writeln!(
                    &mut buffer,
                    "# Remove {}: {}",
                    file_to_del.metadata.ino, destination
                )?;
                writeln!(&mut buffer, "ln -f '{source}' '{destination}'")?;
            }
        }
    }
    Ok(())
}

fn scan(arg: ScanArg) {
    println!("Scanning on {}...", arg.path.display());
    println!("File type filter: {:?}", DefaultFilter::ext_set());
    let mut duplicate = Duplicate::new(&arg.path).custom_filter(DefaultFilter::new());

    let time = std::time::SystemTime::now();
    let instant = std::time::Instant::now();
    println!("Task started on {:?}", time);
    duplicate
        .discover()
        .expect("Error occurred while discovering.");

    let duration = instant.elapsed();
    println!("Discovering finished, {:.2}s elapsed.", duration.as_secs());
}

fn dedup() {}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Scan(arg) => scan(arg),
        Commands::Dedup(arg) => {}
    }

    Ok(())
}
