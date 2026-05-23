use chrono::{TimeZone, Utc};
use support_clock::{Clock, FixedClock};

#[test]
fn fixed_clock_returns_fixed_instant() {
    let instant = Utc.with_ymd_and_hms(2026, 3, 20, 12, 0, 0).unwrap();
    let clock = FixedClock::new(instant);
    assert_eq!(clock.now(), instant);
}
