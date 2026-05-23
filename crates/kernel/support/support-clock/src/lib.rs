use chrono::{DateTime, Utc};

pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Debug, Clone)]
pub struct FixedClock {
    fixed: DateTime<Utc>,
}

impl FixedClock {
    pub fn new(fixed: DateTime<Utc>) -> Self {
        Self { fixed }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.fixed
    }
}

pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}
