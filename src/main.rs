use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::time::Instant;

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

fn display_path(path: &std::path::Path, home: Option<&std::path::Path>) -> String {
    if let Some(home) = home
        && let Ok(stripped) = path.strip_prefix(home)
    {
        return format!("~/{}", stripped.display());
    }
    path.display().to_string()
}

fn main() -> Result<()> {
    let args = Args::parse();
    let interactive = !args.no_interactive && std::io::stdout().is_terminal();
    let scan_path = args.path.canonicalize()?;
    let home_dir = std::env::var("HOME").ok().map(PathBuf::from);

    let opts = ScanOptions {
        include_hidden: args.hidden,
        max_depth: args.max_depth,
        cross_filesystems: args.all_filesystems,
    };

    // Phase 1: scan for repos with progress
    let mut last_update = Instant::now();
    let repo_paths = scanner::scan_repos_with_progress(&scan_path, &opts, |progress| {
        // Throttle updates to avoid excessive writes
        let now = Instant::now();
        if now.duration_since(last_update).as_millis() < 80 {
            return;
        }
        last_update = now;

        let dir_display = display_path(progress.current_dir, home_dir.as_deref());
        // Truncate long paths to keep output on one line
        let max_len = 60;
        let dir_short = if dir_display.len() > max_len {
            format!("...{}", &dir_display[dir_display.len() - max_len + 3..])
        } else {
            dir_display
        };
        eprint!(
            "\r\x1b[KScanning: {} dirs | {} repos found | {}",
            progress.dirs_scanned, progress.repos_found, dir_short
        );
        let _ = std::io::stderr().flush();
    });

    // Clear the progress line
    eprint!("\r\x1b[K");
    let _ = std::io::stderr().flush();

    // Phase 2: inspect each repo with progress
    let total = repo_paths.len();
    let mut repos: Vec<RepoInfo> = Vec::with_capacity(total);
    let mut last_update = Instant::now();

    for (i, p) in repo_paths.iter().enumerate() {
        let now = Instant::now();
        if now.duration_since(last_update).as_millis() >= 80 {
            last_update = now;
            let path_display = display_path(p, home_dir.as_deref());
            eprint!("\r\x1b[KInspecting repo {}/{}: {}", i + 1, total, path_display);
            let _ = std::io::stderr().flush();
        }

        match git::inspect_repo(p) {
            Ok(info) => repos.push(info),
            Err(e) => {
                eprint!("\r\x1b[K");
                eprintln!("Warning: failed to inspect {}: {}", p.display(), e);
            }
        }
    }

    // Clear the progress line
    eprint!("\r\x1b[K");
    let _ = std::io::stderr().flush();

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
