use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn severity(self) -> u8 {
        match self {
            Self::Trace => 10,
            Self::Debug => 20,
            Self::Info => 30,
            Self::Warn => 40,
            Self::Error => 50,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        };
        f.write_str(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_severity() {
        assert!(LogLevel::Error.severity() > LogLevel::Warn.severity());
        assert!(LogLevel::Info.severity() > LogLevel::Debug.severity());
    }

    #[test]
    fn displays_uppercase() {
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }
}
