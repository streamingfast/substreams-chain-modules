//! NYSE session calendar in America/New_York without a timezone crate: US DST
//! rule (second Sunday of March 02:00 -> first Sunday of November 02:00) plus
//! the exchange holiday list.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Session {
    Regular,
    Extended,
    Closed,
}

impl Session {
    pub fn as_str(self) -> &'static str {
        match self {
            Session::Regular => "regular",
            Session::Extended => "extended",
            Session::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EtTime {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    /// 0 = Sunday .. 6 = Saturday
    pub weekday: u32,
    pub minute_of_day: u32,
    pub dst: bool,
}

const REGULAR_OPEN: u32 = 9 * 60 + 30;
const REGULAR_CLOSE: u32 = 16 * 60;
const EXTENDED_OPEN: u32 = 4 * 60;
const EXTENDED_CLOSE: u32 = 20 * 60;

const EST_OFFSET: i64 = -5 * 3600;
const EDT_OFFSET: i64 = -4 * 3600;

// Full-day NYSE closures, (year, month, day). Static table through 2027;
// extend it when the exchange publishes the next year's calendar.
const HOLIDAYS: &[(i64, u32, u32)] = &[
    (2025, 1, 1),
    (2025, 1, 9),
    (2025, 1, 20),
    (2025, 2, 17),
    (2025, 4, 18),
    (2025, 5, 26),
    (2025, 6, 19),
    (2025, 7, 4),
    (2025, 9, 1),
    (2025, 11, 27),
    (2025, 12, 25),
    (2026, 1, 1),
    (2026, 1, 19),
    (2026, 2, 16),
    (2026, 4, 3),
    (2026, 5, 25),
    (2026, 6, 19),
    (2026, 7, 3),
    (2026, 9, 7),
    (2026, 11, 26),
    (2026, 12, 25),
    (2027, 1, 1),
    (2027, 1, 18),
    (2027, 2, 15),
    (2027, 3, 26),
    (2027, 5, 31),
    (2027, 6, 18),
    (2027, 7, 5),
    (2027, 9, 6),
    (2027, 11, 25),
    (2027, 12, 24),
];

// Early-close days: the regular session ends at 13:00 ET instead of 16:00.
const EARLY_CLOSES: &[(i64, u32, u32)] = &[(2026, 11, 27), (2026, 12, 24), (2027, 11, 26)];
const EARLY_REGULAR_CLOSE: u32 = 13 * 60;

pub fn classify(unix_ts: u64) -> Session {
    let et = to_et(unix_ts);
    if et.weekday == 0 || et.weekday == 6 || is_holiday(et.year, et.month, et.day) {
        return Session::Closed;
    }
    let m = et.minute_of_day;
    let regular_close = if is_early_close(et.year, et.month, et.day) {
        EARLY_REGULAR_CLOSE
    } else {
        REGULAR_CLOSE
    };
    if (REGULAR_OPEN..regular_close).contains(&m) {
        Session::Regular
    } else if (EXTENDED_OPEN..REGULAR_OPEN).contains(&m) || (regular_close..EXTENDED_CLOSE).contains(&m) {
        Session::Extended
    } else {
        Session::Closed
    }
}

pub fn is_holiday(year: i64, month: u32, day: u32) -> bool {
    HOLIDAYS.contains(&(year, month, day))
}

pub fn is_early_close(year: i64, month: u32, day: u32) -> bool {
    EARLY_CLOSES.contains(&(year, month, day))
}

pub fn to_et(unix_ts: u64) -> EtTime {
    let ts = unix_ts as i64;
    let (utc_year, _, _) = civil_from_days(ts.div_euclid(86_400));
    let dst = is_dst(ts, utc_year);
    let local = ts + if dst { EDT_OFFSET } else { EST_OFFSET };
    let days = local.div_euclid(86_400);
    let secs = local.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    EtTime {
        year,
        month,
        day,
        weekday: weekday_from_days(days),
        minute_of_day: (secs / 60) as u32,
        dst,
    }
}

// DST window in UTC for a given year. The local UTC date can only differ from
// the ET date near midnight, never near the 02:00 transitions, so choosing the
// year from the UTC date is safe.
fn is_dst(ts: i64, year: i64) -> bool {
    let (start, end) = dst_bounds_utc(year);
    ts >= start && ts < end
}

pub fn dst_bounds_utc(year: i64) -> (i64, i64) {
    let start = nth_sunday(year, 3, 2) * 86_400 + 7 * 3600; // 02:00 EST
    let end = nth_sunday(year, 11, 1) * 86_400 + 6 * 3600; // 02:00 EDT
    (start, end)
}

// Days since epoch of the n-th Sunday of a month.
fn nth_sunday(year: i64, month: u32, n: i64) -> i64 {
    let first = days_from_civil(year, month, 1);
    let first_sunday = first + ((7 - weekday_from_days(first) as i64) % 7);
    first_sunday + (n - 1) * 7
}

fn weekday_from_days(days: i64) -> u32 {
    // 1970-01-01 was a Thursday.
    (days + 4).rem_euclid(7) as u32
}

// Howard Hinnant's proleptic Gregorian algorithms.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(y: i64, m: u32, d: u32, hh: i64, mm: i64) -> u64 {
        (days_from_civil(y, m, d) * 86_400 + hh * 3600 + mm * 60) as u64
    }

    #[test]
    fn civil_roundtrip_and_known_epochs() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2026, 1, 1), 20_454);
        assert_eq!(civil_from_days(20_454), (2026, 1, 1));
        for days in [-1, 0, 59, 20_454, 20_620, 21_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(utc(2026, 6, 17, 14, 30), 1_781_706_600);
    }

    #[test]
    fn dst_transitions_2026() {
        let (start, end) = dst_bounds_utc(2026);
        assert_eq!(start, utc(2026, 3, 8, 7, 0) as i64);
        assert_eq!(end, utc(2026, 11, 1, 6, 0) as i64);

        let before = to_et(utc(2026, 3, 8, 6, 59));
        assert!(!before.dst);
        assert_eq!((before.minute_of_day, before.day), (1 * 60 + 59, 8));
        let after = to_et(utc(2026, 3, 8, 7, 0));
        assert!(after.dst);
        assert_eq!(after.minute_of_day, 3 * 60);

        let last_edt = to_et(utc(2026, 11, 1, 5, 59));
        assert!(last_edt.dst);
        assert_eq!(last_edt.minute_of_day, 1 * 60 + 59);
        let first_est = to_et(utc(2026, 11, 1, 6, 0));
        assert!(!first_est.dst);
        assert_eq!(first_est.minute_of_day, 1 * 60);
    }

    #[test]
    fn et_date_rolls_back_across_utc_midnight() {
        // 2026-06-18 02:00 UTC is 2026-06-17 22:00 EDT, a Wednesday.
        let et = to_et(utc(2026, 6, 18, 2, 0));
        assert_eq!((et.month, et.day, et.weekday), (6, 17, 3));
        assert_eq!(et.minute_of_day, 22 * 60);
    }

    #[test]
    fn regular_session_boundaries_in_edt() {
        // Wednesday 2026-06-17.
        assert_eq!(classify(utc(2026, 6, 17, 13, 29)), Session::Extended); // 09:29
        assert_eq!(classify(utc(2026, 6, 17, 13, 30)), Session::Regular); // 09:30
        assert_eq!(classify(utc(2026, 6, 17, 14, 30)), Session::Regular);
        assert_eq!(classify(utc(2026, 6, 17, 19, 59)), Session::Regular); // 15:59
        assert_eq!(classify(utc(2026, 6, 17, 20, 0)), Session::Extended); // 16:00
        assert_eq!(classify(utc(2026, 6, 17, 23, 59)), Session::Extended); // 19:59
        assert_eq!(classify(utc(2026, 6, 18, 0, 0)), Session::Closed); // 20:00
        assert_eq!(classify(utc(2026, 6, 17, 7, 59)), Session::Closed); // 03:59
        assert_eq!(classify(utc(2026, 6, 17, 8, 0)), Session::Extended); // 04:00
    }

    #[test]
    fn regular_session_in_est() {
        // Tuesday 2026-01-20, 10:00 EST.
        assert_eq!(classify(utc(2026, 1, 20, 15, 0)), Session::Regular);
        // 16:00 EST is extended, 20:00 EST is closed.
        assert_eq!(classify(utc(2026, 1, 20, 21, 0)), Session::Extended);
        assert_eq!(classify(utc(2026, 1, 21, 1, 0)), Session::Closed);
    }

    #[test]
    fn weekends_and_holidays_are_closed() {
        assert_eq!(classify(utc(2026, 6, 20, 14, 30)), Session::Closed); // Saturday
        assert_eq!(classify(utc(2026, 6, 21, 14, 30)), Session::Closed); // Sunday
        assert_eq!(classify(utc(2026, 1, 19, 15, 0)), Session::Closed); // MLK Day
        assert_eq!(classify(utc(2026, 7, 3, 15, 0)), Session::Closed); // Independence Day observed
        assert_eq!(classify(utc(2026, 11, 26, 15, 0)), Session::Closed); // Thanksgiving
        assert_eq!(classify(utc(2026, 12, 25, 15, 0)), Session::Closed);
        // A holiday is closed in extended hours too.
        assert_eq!(classify(utc(2026, 7, 3, 12, 0)), Session::Closed);
        // The day after a holiday trades normally.
        assert_eq!(classify(utc(2026, 11, 27, 15, 0)), Session::Regular);
    }

    #[test]
    fn holiday_2027_is_closed() {
        // New Year's Day 2027, 10:00 EST.
        assert_eq!(classify(utc(2027, 1, 1, 15, 0)), Session::Closed);
    }

    #[test]
    fn early_close_day_extended_after_1pm() {
        // Black Friday 2026, 14:00 EST is past the 13:00 early close.
        assert_eq!(classify(utc(2026, 11, 27, 19, 0)), Session::Extended);
    }

    #[test]
    fn early_close_day_regular_before_1pm() {
        // Black Friday 2026, 12:00 EST is still within the shortened session.
        assert_eq!(classify(utc(2026, 11, 27, 17, 0)), Session::Regular);
    }

    #[test]
    fn session_strings() {
        assert_eq!(Session::Regular.as_str(), "regular");
        assert_eq!(Session::Extended.as_str(), "extended");
        assert_eq!(Session::Closed.as_str(), "closed");
    }
}
