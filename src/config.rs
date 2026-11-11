use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    pub tab_width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "sakura".to_owned(),
            tab_width: 4,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Read(io::Error),
    Parse(toml::de::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "config read failed: {error}"),
            Self::Parse(error) => write!(formatter, "config parse failed: {error}"),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(ConfigError::Read(error)),
        };
        let mut config: Self = toml::from_str(&text).map_err(ConfigError::Parse)?;
        config.tab_width = config.tab_width.max(1);
        Ok(config)
    }
}

pub fn default_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|base| base.join("shizu").join("config.toml"))
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(base.join("shizu").join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Config;
    use crate::test_support::TempDir;

    #[test]
    fn missing_file_means_defaults() {
        let dir = TempDir::new();
        let config = Config::load(&dir.join("missing.toml")).expect("default config");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn parses_theme_and_tab_width() {
        let dir = TempDir::new();
        let path = dir.join("config.toml");
        fs::write(&path, "theme = \"sakura-light\"\ntab_width = 2\n").expect("write config");
        let config = Config::load(&path).expect("parse config");
        assert_eq!(config.theme, "sakura-light");
        assert_eq!(config.tab_width, 2);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let dir = TempDir::new();
        let path = dir.join("config.toml");
        fs::write(&path, "tab_width = 8\n").expect("write config");
        let config = Config::load(&path).expect("parse config");
        assert_eq!(config.theme, "sakura");
        assert_eq!(config.tab_width, 8);
    }

    #[test]
    fn zero_tab_width_is_clamped() {
        let dir = TempDir::new();
        let path = dir.join("config.toml");
        fs::write(&path, "tab_width = 0\n").expect("write config");
        let config = Config::load(&path).expect("parse config");
        assert_eq!(config.tab_width, 1);
    }

    #[test]
    fn invalid_toml_is_an_error() {
        let dir = TempDir::new();
        let path = dir.join("config.toml");
        fs::write(&path, "theme = [\n").expect("write config");
        assert!(Config::load(&path).is_err());
    }
}
