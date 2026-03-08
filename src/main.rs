use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Grove — would you lose work if this machine died?
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Directory to scan (default: current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Static output mode (auto if not a TTY)
    #[arg(short = 'n', long = "no-interactive")]
    no_interactive: bool,

    /// Include hidden directories in traversal
    #[arg(short = 'H', long = "hidden")]
    hidden: bool,

    /// Maximum directory traversal depth
    #[arg(short = 'd', long = "max-depth")]
    max_depth: Option<usize>,

    /// Cross filesystem boundaries
    #[arg(long = "all-filesystems")]
    all_filesystems: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let interactive = !args.no_interactive && atty::is(atty::Stream::Stdout);

    let scan_path = args.path.canonicalize()?;

    println!("Scanning: {}", scan_path.display());
    println!("Mode: {}", if interactive { "interactive" } else { "static" });

    Ok(())
}
