use std::collections::HashMap;
use std::path::PathBuf;

/// Application configuration loaded from `~/.config/grove/config`.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub host_aliases: HashMap<String, String>,
}

impl Config {
    /// Load configuration from `$HOME/.config/grove/config`.
    /// Returns an empty config if the file is missing or unreadable.
    pub fn load() -> Self {
        let path = match std::env::var("HOME") {
            Ok(home) => PathBuf::from(home).join(".config/grove/config"),
            Err(_) => return Self::default(),
        };
        Self::load_from(&path)
    }

    fn load_from(path: &std::path::Path) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        Self::parse(&content)
    }

    fn parse(content: &str) -> Self {
        let mut host_aliases = HashMap::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if let Some(hostname) = key.strip_prefix("host.") {
                    if !hostname.is_empty() && !value.is_empty() {
                        host_aliases.insert(hostname.to_string(), value.to_string());
                    }
                }
            }
        }
        Self { host_aliases }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_host_aliases() {
        let content = r#"
# Custom host aliases
host.git.iguana-galaxy.ts.net = iguana
host.my-gitlab.internal = work
"#;
        let config = Config::parse(content);
        assert_eq!(config.host_aliases.get("git.iguana-galaxy.ts.net").unwrap(), "iguana");
        assert_eq!(config.host_aliases.get("my-gitlab.internal").unwrap(), "work");
    }

    #[test]
    fn parse_empty_and_comments() {
        let content = "# just a comment\n\n  \n";
        let config = Config::parse(content);
        assert!(config.host_aliases.is_empty());
    }

    #[test]
    fn parse_ignores_non_host_keys() {
        let content = "something.else = value\nhost. = empty\nhost.ok = \n";
        let config = Config::parse(content);
        assert!(config.host_aliases.is_empty());
    }

    #[test]
    fn parse_whitespace_around_equals() {
        let content = "  host.example.com  =  myhost  \n";
        let config = Config::parse(content);
        assert_eq!(config.host_aliases.get("example.com").unwrap(), "myhost");
    }

    #[test]
    fn load_missing_file() {
        let config = Config::load_from(std::path::Path::new("/nonexistent/path/config"));
        assert!(config.host_aliases.is_empty());
    }
}
