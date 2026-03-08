use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use grove::scanner::{self, ScanOptions};
use grove::git;
use grove::model::RepoInfo;
use grove::static_output;

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
    let home_dir = dirs::home_dir();

    let opts = ScanOptions {
        include_hidden: args.hidden,
        max_depth: args.max_depth,
        cross_filesystems: args.all_filesystems,
    };

    eprintln!("Scanning {}...", scan_path.display());

    // Scan for repos
    let repo_paths = scanner::scan_repos(&scan_path, &opts);

    // Inspect each repo
    let mut repos: Vec<RepoInfo> = repo_paths
        .iter()
        .filter_map(|p| match git::inspect_repo(p) {
            Ok(info) => Some(info),
            Err(e) => {
                eprintln!("Warning: failed to inspect {}: {}", p.display(), e);
                None
            }
        })
        .collect();

    // Sort by risk level (at-risk first)
    repos.sort_by_key(|r| r.risk_level());

    if interactive {
        let mut app = grove::tui::App::new(repos, scan_path, opts, home_dir);
        grove::tui::run(&mut app)?;
    } else {
        static_output::print_static(&repos, home_dir.as_deref());
    }

    Ok(())
}
