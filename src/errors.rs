use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Yaml {
        path: PathBuf,
        source: serde_yaml::Error,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io { path, source } => {
                write!(
                    f,
                    "Failed to access config file at '{}': {}",
                    path.display(),
                    source
                )
            }
            ConfigError::Yaml { path, source } => {
                write!(
                    f,
                    "Invalid YAML in config file '{}': {}",
                    path.display(),
                    source
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io { source, .. } => Some(source),
            ConfigError::Yaml { source, .. } => Some(source),
        }
    }
}

impl ConfigError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        ConfigError::Io {
            path: path.into(),
            source,
        }
    }

    pub fn yaml(path: impl Into<PathBuf>, source: serde_yaml::Error) -> Self {
        ConfigError::Yaml {
            path: path.into(),
            source,
        }
    }
}
