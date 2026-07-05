use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::{Error, Result};

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Current instant as RFC3339 with UTC offset.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// RFC3339 UTC from epoch seconds (used by the shell without its own chrono).
pub fn epoch_to_rfc3339(epoch_seconds: i64) -> String {
    DateTime::<Utc>::from_timestamp(epoch_seconds, 0)
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn parse_instant(s: &str) -> Result<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(s)
        .map_err(|e| Error::invalid(format!("invalid timestamp '{s}': {e}")))
}

pub fn parse_tz(tz: &str) -> Result<Tz> {
    tz.parse::<Tz>()
        .map_err(|_| Error::invalid(format!("invalid timezone '{tz}'")))
}

pub fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| Error::invalid(format!("invalid date '{s}': {e}")))
}

pub fn parse_hhmm(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .map_err(|e| Error::invalid(format!("invalid time '{s}': {e}")))
}

/// Local calendar date of an instant in the given IANA timezone.
pub fn local_date_of(instant: &str, tz: &str) -> Result<String> {
    let t = parse_instant(instant)?;
    let z = parse_tz(tz)?;
    Ok(t.with_timezone(&z).date_naive().format("%Y-%m-%d").to_string())
}

pub fn today_in_tz(tz: &str) -> Result<String> {
    let z = parse_tz(tz)?;
    Ok(Utc::now().with_timezone(&z).date_naive().format("%Y-%m-%d").to_string())
}

pub fn now_local_hhmm(tz: &str) -> Result<String> {
    let z = parse_tz(tz)?;
    Ok(Utc::now().with_timezone(&z).format("%H:%M").to_string())
}

/// Convert a local date + HH:MM in a timezone to an RFC3339 instant.
pub fn local_to_instant(date: &str, hhmm: &str, tz: &str) -> Result<String> {
    let d = parse_date(date)?;
    let t = parse_hhmm(hhmm)?;
    let z = parse_tz(tz)?;
    let naive = d.and_time(t);
    let resolved = z
        .from_local_datetime(&naive)
        .earliest()
        .ok_or_else(|| Error::invalid(format!("nonexistent local time {date} {hhmm} in {tz}")))?;
    Ok(resolved.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

/// Monday of the week that contains `date`.
pub fn week_start(date: &str) -> Result<String> {
    let d = parse_date(date)?;
    let monday = d - chrono::Duration::days(d.weekday().num_days_from_monday() as i64);
    Ok(monday.format("%Y-%m-%d").to_string())
}

pub fn add_days(date: &str, days: i64) -> Result<String> {
    let d = parse_date(date)?;
    Ok((d + chrono::Duration::days(days)).format("%Y-%m-%d").to_string())
}

/// Weekday index with Monday = 0.
pub fn weekday_index(date: &str) -> Result<u32> {
    Ok(parse_date(date)?.weekday().num_days_from_monday())
}

/// True when `hhmm_now` falls inside the [start, end) window; handles windows
/// that cross midnight (e.g. quiet hours 22:00–07:00).
pub fn in_window(hhmm_now: &str, start: &str, end: &str) -> Result<bool> {
    let now = parse_hhmm(hhmm_now)?;
    let s = parse_hhmm(start)?;
    let e = parse_hhmm(end)?;
    Ok(if s <= e { now >= s && now < e } else { now >= s || now < e })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_date_conversion() {
        // 23:30 UTC on Jan 1 is already Jan 2 in Zagreb (UTC+1).
        let d = local_date_of("2026-01-01T23:30:00+00:00", "Europe/Zagreb").unwrap();
        assert_eq!(d, "2026-01-02");
    }

    #[test]
    fn week_start_is_monday() {
        assert_eq!(week_start("2026-07-05").unwrap(), "2026-06-29"); // Sunday -> prior Monday
        assert_eq!(week_start("2026-06-29").unwrap(), "2026-06-29");
    }

    #[test]
    fn quiet_hours_cross_midnight() {
        assert!(in_window("23:00", "22:00", "07:00").unwrap());
        assert!(in_window("06:59", "22:00", "07:00").unwrap());
        assert!(!in_window("12:00", "22:00", "07:00").unwrap());
    }
}
