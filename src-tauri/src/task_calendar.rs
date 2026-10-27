//! Calendar recurrence in the timezone captured at creation; never fixed 24-hour UTC intervals.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Civil {
    pub y: i32,
    pub m: u32,
    pub d: u32,
    pub h: u32,
    pub min: u32,
}
fn leap(y: i32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}
fn days(y: i32, m: u32) -> u32 {
    match m {
        2 => {
            if leap(y) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}
pub fn parse(s: &str) -> Result<Civil, String> {
    if s.len() != 16
        || !s.is_ascii()
        || &s[4..5] != "-"
        || &s[7..8] != "-"
        || &s[10..11] != "T"
        || &s[13..14] != ":"
    {
        return Err("Choose a valid local date and time".into());
    }
    let n = |a: usize, b: usize| {
        s[a..b]
            .parse::<u32>()
            .map_err(|_| "Invalid date".to_string())
    };
    let c = Civil {
        y: n(0, 4)? as i32,
        m: n(5, 7)?,
        d: n(8, 10)?,
        h: n(11, 13)?,
        min: n(14, 16)?,
    };
    if !(1970..=2100).contains(&c.y)
        || !(1..=12).contains(&c.m)
        || c.d == 0
        || c.d > days(c.y, c.m)
        || c.h > 23
        || c.min > 59
    {
        return Err("Invalid calendar date".into());
    }
    Ok(c)
}
pub fn text(c: Civil) -> String {
    format!("{:04}-{:02}-{:02}T{:02}:{:02}", c.y, c.m, c.d, c.h, c.min)
}
pub fn advance(mut c: Civil, repeat: &str, anchor: u32) -> Result<Civil, String> {
    if repeat == "monthly" {
        c.m += 1;
        if c.m == 13 {
            c.m = 1;
            c.y += 1;
        }
        c.d = anchor.min(days(c.y, c.m));
    } else {
        let count = match repeat {
            "daily" => 1,
            "weekly" => 7,
            _ => return Err("Not a recurring schedule".into()),
        };
        for _ in 0..count {
            c.d += 1;
            if c.d > days(c.y, c.m) {
                c.d = 1;
                c.m += 1;
                if c.m == 13 {
                    c.m = 1;
                    c.y += 1;
                }
            }
        }
    }
    if c.y > 2100 {
        return Err("Schedule exceeds supported calendar range".into());
    }
    Ok(c)
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    #[serde(default)]
    pub anchor_day: u32,
    pub local: String,
    pub zone: String,
    pub repeat: String,
    pub next_local: String,
    pub next_ms: u64,
}
impl Schedule {
    pub fn create(local: String, zone: String, repeat: String, now: u64) -> Result<Self, String> {
        if !matches!(repeat.as_str(), "once" | "daily" | "weekly" | "monthly") {
            return Err("Choose one-time, daily, weekly or monthly".into());
        }
        let next_ms = to_utc(parse(&local)?, &zone)?;
        if next_ms <= now {
            return Err("Choose a future schedule time".into());
        }
        Ok(Self {
            anchor_day: parse(&local)?.d,
            next_local: local.clone(),
            local,
            zone,
            repeat,
            next_ms,
        })
    }
    pub fn next_after(&mut self, now: u64) -> Result<bool, String> {
        if self.repeat == "once" {
            return Ok(false);
        }
        let anchor = if self.anchor_day > 0 {
            self.anchor_day
        } else {
            parse(&self.local)?.d
        };
        let mut c = parse(&self.next_local)?;
        for _ in 0..50000 {
            c = advance(c, &self.repeat, anchor)?;
            let at = to_utc(c, &self.zone)?;
            if at > now {
                self.next_local = text(c);
                self.next_ms = at;
                return Ok(true);
            }
        }
        Err("Review this schedule before continuing".into())
    }
}
#[cfg(windows)]
fn zone(key: &str) -> Result<windows::Win32::System::Time::DYNAMIC_TIME_ZONE_INFORMATION, String> {
    use windows::Win32::System::Time::*;
    static ZONES: std::sync::OnceLock<
        std::collections::HashMap<String, DYNAMIC_TIME_ZONE_INFORMATION>,
    > = std::sync::OnceLock::new();
    let zones = ZONES.get_or_init(|| {
        let mut zones = std::collections::HashMap::new();
        for i in 0..1000 {
            let mut z = DYNAMIC_TIME_ZONE_INFORMATION::default();
            let status = unsafe { EnumDynamicTimeZoneInformation(i, &mut z) };
            if status != 0 {
                break;
            }
            zones.insert(
                String::from_utf16_lossy(&z.TimeZoneKeyName)
                    .trim_end_matches('\0')
                    .to_string(),
                z,
            );
        }
        zones
    });
    zones
        .get(key)
        .copied()
        .ok_or("The saved timezone is unavailable; edit the schedule".into())
}
pub fn current_zone() -> Result<String, String> {
    #[cfg(windows)]
    {
        use windows::Win32::System::Time::*;
        let mut z = DYNAMIC_TIME_ZONE_INFORMATION::default();
        if unsafe { GetDynamicTimeZoneInformation(&mut z) } == u32::MAX {
            return Err("Cannot read device timezone".into());
        }
        Ok(String::from_utf16_lossy(&z.TimeZoneKeyName)
            .trim_end_matches('\0')
            .into())
    }
    #[cfg(not(windows))]
    {
        Err("Calendar scheduling is currently supported on Windows".into())
    }
}
#[cfg(windows)]
pub fn local_now(key: &str) -> Result<String, String> {
    use windows::Win32::{
        Foundation::{FILETIME, SYSTEMTIME},
        System::Time::*,
    };
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;
    let ticks = (ms + 11_644_473_600_000) * 10000;
    let ft = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    let mut utc = SYSTEMTIME::default();
    let mut local = SYSTEMTIME::default();
    let z = zone(key)?;
    unsafe {
        FileTimeToSystemTime(&ft, &mut utc)
            .and_then(|_| SystemTimeToTzSpecificLocalTimeEx(Some(&z), &utc, &mut local))
    }
    .map_err(|e| e.to_string())?;
    Ok(text(Civil {
        y: local.wYear as i32,
        m: local.wMonth as u32,
        d: local.wDay as u32,
        h: local.wHour as u32,
        min: local.wMinute as u32,
    }))
}
#[cfg(not(windows))]
pub fn local_now(_: &str) -> Result<String, String> {
    Err("Calendar scheduling requires Windows".into())
}
#[cfg(windows)]
pub fn to_utc(c: Civil, key: &str) -> Result<u64, String> {
    use windows::Win32::{
        Foundation::{FILETIME, SYSTEMTIME},
        System::Time::*,
    };
    let z = zone(key)?;
    let input = SYSTEMTIME {
        wYear: c.y as u16,
        wMonth: c.m as u16,
        wDay: c.d as u16,
        wHour: c.h as u16,
        wMinute: c.min as u16,
        ..Default::default()
    };
    let mut utc = SYSTEMTIME::default();
    unsafe { TzSpecificLocalTimeToSystemTimeEx(Some(&z), &input, &mut utc) }
        .map_err(|e| e.to_string())?;
    let mut check = SYSTEMTIME::default();
    unsafe { SystemTimeToTzSpecificLocalTimeEx(Some(&z), &utc, &mut check) }
        .map_err(|e| e.to_string())?;
    if check.wHour != input.wHour || check.wMinute != input.wMinute || check.wDay != input.wDay {
        return Err(
            "This local time does not exist due to a clock change; choose another time".into(),
        );
    }
    let mut ft = FILETIME::default();
    unsafe { SystemTimeToFileTime(&utc, &mut ft) }.map_err(|e| e.to_string())?;
    let ticks = ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64;
    let ms = (ticks / 10000)
        .checked_sub(11_644_473_600_000)
        .ok_or("Choose a date after the Unix epoch")?;
    // Reject repeated wall-clock times instead of silently choosing an occurrence.
    let delta = (z.DaylightBias - z.StandardBias).unsigned_abs() as u64 * 60_000;
    if delta > 0 {
        for candidate in [ms.checked_add(delta), ms.checked_sub(delta)]
            .into_iter()
            .flatten()
        {
            let ticks = (candidate + 11_644_473_600_000) * 10000;
            let ft = FILETIME {
                dwLowDateTime: ticks as u32,
                dwHighDateTime: (ticks >> 32) as u32,
            };
            let mut u = SYSTEMTIME::default();
            let mut l = SYSTEMTIME::default();
            unsafe {
                FileTimeToSystemTime(&ft, &mut u)
                    .and_then(|_| SystemTimeToTzSpecificLocalTimeEx(Some(&z), &u, &mut l))
            }
            .map_err(|e| e.to_string())?;
            if l.wYear == input.wYear
                && l.wMonth == input.wMonth
                && l.wDay == input.wDay
                && l.wHour == input.wHour
                && l.wMinute == input.wMinute
            {
                return Err(
                    "This time occurs twice due to a clock change; choose an unambiguous time"
                        .into(),
                );
            }
        }
    }
    Ok(ms)
}
#[cfg(not(windows))]
pub fn to_utc(_: Civil, _: &str) -> Result<u64, String> {
    Err("Calendar scheduling requires Windows".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn clock_changes_are_explicit_and_recurring_dates_keep_local_time() {
        assert!(to_utc(parse("2026-03-08T02:30").unwrap(), "Eastern Standard Time").is_err());
        assert!(to_utc(parse("2026-11-01T01:30").unwrap(), "Eastern Standard Time").is_err());
        let mut schedule = Schedule::create(
            "2026-10-30T07:00".into(),
            "Eastern Standard Time".into(),
            "daily".into(),
            0,
        )
        .unwrap();
        let after = to_utc(parse("2026-11-02T12:00").unwrap(), "UTC").unwrap();
        assert!(schedule.next_after(after).unwrap());
        assert_eq!(schedule.next_local, "2026-11-03T07:00");
        assert_eq!(
            schedule.next_ms,
            to_utc(parse("2026-11-03T12:00").unwrap(), "UTC").unwrap()
        );
    }
    #[cfg(windows)]
    #[test]
    fn monthly_catchup_keeps_original_day() {
        let mut schedule =
            Schedule::create("2027-01-31T07:00".into(), "UTC".into(), "monthly".into(), 0).unwrap();
        let after = to_utc(parse("2027-02-28T08:00").unwrap(), "UTC").unwrap();
        assert!(schedule.next_after(after).unwrap());
        assert_eq!(schedule.next_local, "2027-03-31T07:00");
        assert!(
            Schedule::create(
                "2027-01-31T07:00".into(),
                "UTC".into(),
                "once".into(),
                after
            )
            .is_err()
        );
    }
    #[test]
    fn recurrence_preserves_monthly_anchor_and_leap_year() {
        let c = parse("2028-01-31T07:00").unwrap();
        let feb = advance(c, "monthly", 31).unwrap();
        assert_eq!(text(feb), "2028-02-29T07:00");
        assert_eq!(
            text(advance(feb, "monthly", 31).unwrap()),
            "2028-03-31T07:00"
        );
        assert!(parse("2027-02-29T07:00").is_err());
        assert_eq!(
            text(advance(parse("2026-12-28T07:00").unwrap(), "weekly", 28).unwrap()),
            "2027-01-04T07:00"
        );
    }
}
