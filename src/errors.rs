use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(err) => write!(f, "I/O error: {err}"),
            ConfigError::Yaml(err) => write!(f, "YAML error: {err}"),
        }
    }
}

impl std::error::Error for ConfigError {}
impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> ConfigError {
        ConfigError::Io(err)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(err: serde_yaml::Error) -> ConfigError {
        ConfigError::Yaml(err)
    }
}
