use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use clap::Parser;

use grove::scanner::{self, ScanOptions};
use grove::git;
use grove::model::{self, RepoInfo};
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

/// Truncate a display string to `max_len` chars, adding "..." prefix for the tail.
fn truncate_display(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let skip = char_count - (max_len.saturating_sub(3));
        format!("...{}", s.chars().skip(skip).collect::<String>())
    }
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

    if interactive {
        let cached_paths = grove::cache::load(&scan_path);

        let mut repos: Vec<RepoInfo> = if let Some(ref paths) = cached_paths {
            // Cache hit: quick-verify and inspect cached repos
            let valid: Vec<&std::path::Path> = paths
                .iter()
                .filter(|p| p.join(".git").exists())
                .map(|p| p.as_path())
                .collect();

            let total = valid.len();
            let mut last_update = Instant::now();
            let mut result = Vec::with_capacity(total);

            for (i, p) in valid.iter().enumerate() {
                let now = Instant::now();
                if now.duration_since(last_update).as_millis() >= 80 {
                    last_update = now;
                    let path_display = model::display_path(p, home_dir.as_deref());
                    eprint!("\r\x1b[KLoading repo {}/{}: {}", i + 1, total, path_display);
                    let _ = std::io::stderr().flush();
                }
                if let Ok(info) = git::inspect_repo(p) {
                    result.push(info);
                }
            }
            eprint!("\r\x1b[K");
            let _ = std::io::stderr().flush();
            result
        } else {
            // No cache: full scan with progress
            let mut last_update = Instant::now();
            let repo_paths = scanner::scan_repos_with_progress(&scan_path, &opts, |progress| {
                let now = Instant::now();
                if now.duration_since(last_update).as_millis() < 80 {
                    return;
                }
                last_update = now;
                let dir_display = model::display_path(progress.current_dir, home_dir.as_deref());
                let dir_short = truncate_display(&dir_display, 60);
                eprint!(
                    "\r\x1b[KScanning: {} dirs | {} repos found | {}",
                    progress.dirs_scanned, progress.repos_found, dir_short
                );
                let _ = std::io::stderr().flush();
            });
            eprint!("\r\x1b[K");
            let _ = std::io::stderr().flush();

            let total = repo_paths.len();
            let mut last_update = Instant::now();
            let mut result = Vec::with_capacity(total);
            for (i, p) in repo_paths.iter().enumerate() {
                let now = Instant::now();
                if now.duration_since(last_update).as_millis() >= 80 {
                    last_update = now;
                    eprint!("\r\x1b[KInspecting repo {}/{}: {}", i + 1, total, model::display_path(p, home_dir.as_deref()));
                    let _ = std::io::stderr().flush();
                }
                match git::inspect_repo(p) {
                    Ok(info) => result.push(info),
                    Err(e) => {
                        eprint!("\r\x1b[K");
                        eprintln!("Warning: failed to inspect {}: {}", p.display(), e);
                    }
                }
            }
            eprint!("\r\x1b[K");
            let _ = std::io::stderr().flush();

            // Write initial cache
            let paths: Vec<PathBuf> = result.iter().map(|r| r.path.clone()).collect();
            grove::cache::save(&scan_path, &paths);

            result
        };

        repos.sort_by_key(|r| r.risk_level());
        let mut app = grove::tui::App::new(repos, scan_path, opts, home_dir);

        if cached_paths.is_some() {
            app.start_background_scan();
        }

        grove::tui::run(&mut app)?;
    } else {
        // Non-interactive: always full scan
        let mut last_update = Instant::now();
        let repo_paths = scanner::scan_repos_with_progress(&scan_path, &opts, |progress| {
            let now = Instant::now();
            if now.duration_since(last_update).as_millis() < 80 {
                return;
            }
            last_update = now;
            let dir_display = model::display_path(progress.current_dir, home_dir.as_deref());
            let dir_short = truncate_display(&dir_display, 60);
            eprint!(
                "\r\x1b[KScanning: {} dirs | {} repos found | {}",
                progress.dirs_scanned, progress.repos_found, dir_short
            );
            let _ = std::io::stderr().flush();
        });
        eprint!("\r\x1b[K");
        let _ = std::io::stderr().flush();

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

        repos.sort_by_key(|r| r.risk_level());
        static_output::print_static(&repos, home_dir.as_deref());
    }

    Ok(())
}
