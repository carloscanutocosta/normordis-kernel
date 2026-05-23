use support_errors::{MiniError, PublicError};

use crate::event::LogEvent;
use crate::level::LogLevel;

pub trait TechnicalLogger: Send + Sync {
    fn log(&self, event: LogEvent) -> Result<(), MiniError>;

    fn trace(&self, component: &str, message: impl Into<String>)
    where
        Self: Sized,
    {
        let _ = self.log(LogEvent::new(LogLevel::Trace, component, message));
    }

    fn debug(&self, component: &str, message: impl Into<String>)
    where
        Self: Sized,
    {
        let _ = self.log(LogEvent::new(LogLevel::Debug, component, message));
    }

    fn info(&self, component: &str, message: impl Into<String>)
    where
        Self: Sized,
    {
        let _ = self.log(LogEvent::new(LogLevel::Info, component, message));
    }

    fn warn(&self, component: &str, message: impl Into<String>)
    where
        Self: Sized,
    {
        let _ = self.log(LogEvent::new(LogLevel::Warn, component, message));
    }

    fn error(&self, component: &str, message: impl Into<String>)
    where
        Self: Sized,
    {
        let _ = self.log(LogEvent::new(LogLevel::Error, component, message));
    }
}

pub fn log_mini_error(logger: &dyn TechnicalLogger, error: &MiniError) {
    let PublicError { code, message } = error.to_public();
    let event = LogEvent::new(LogLevel::Error, error.component.as_str(), message).with_code(code);
    let _ = logger.log(event);
}
