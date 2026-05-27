//! Exact, host-independent civil-calendar arithmetic for the TIME module.
//!
//! Following the BigQuery date/time philosophy, a timezone is never part of a
//! value: it is supplied only at the instant <-> civil boundary, here as a
//! UTC offset in hours (an exact rational). Every computation is performed on
//! the exact-rational (continued-fraction) representation with no floating
//! point, so conversions are deterministic and reproducible across hosts.
//!
//! - An *instant* (TIMESTAMP) is an exact number of seconds since the Unix
//!   epoch `1970-01-01T00:00:00Z`.
//! - A *civil* value (DATE / TIME / DATETIME) carries no timezone.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::ToPrimitive;

use crate::types::fraction::Fraction;

const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_HOUR: i64 = 3_600;

/// Days since `1970-01-01` for a proleptic-Gregorian civil date.
/// Howard Hinnant's branch-free `days_from_civil`; integer-exact.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 }.div_euclid(400);
    let yoe = y - era * 400; // [0, 399]
    let mp = if m > 2 { m - 3 } else { m + 9 }; // [0, 11]
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Inverse of [`days_from_civil`]: `(year, month, day)` from a day count.
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// ISO-8601 weekday: Monday = 1 .. Sunday = 7.
/// Day 0 (`1970-01-01`) is a Thursday, so `(days + 3) mod 7` puts Monday at 0.
pub fn iso_weekday(y: i64, m: i64, d: i64) -> i64 {
    (days_from_civil(y, m, d) + 3).rem_euclid(7) + 1
}

/// Number of days in a civil month, leap years included.
pub fn days_in_month(y: i64, m: i64) -> i64 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    days_from_civil(ny, nm, 1) - days_from_civil(y, m, 1)
}

/// Add `n` whole months to a civil date, clamping the day to the last day of
/// the target month (e.g. Jan 31 + 1 month -> Feb 28/29). The `12 * years`
/// case yields year arithmetic with the same end-of-month clamping.
pub fn add_months_civil(y: i64, m: i64, d: i64, n: i64) -> (i64, i64, i64) {
    let total = (m - 1) + n;
    let new_year = y + total.div_euclid(12);
    let new_month = total.rem_euclid(12) + 1;
    let clamped_day = d.min(days_in_month(new_year, new_month));
    (new_year, new_month, clamped_day)
}

/// A timezone offset in hours east of UTC, converted to an exact number of
/// seconds. `+9` (Asia/Tokyo) -> `32400`, `+5.5` (India) -> `19800`.
pub fn offset_seconds(offset_hours: &Fraction) -> Fraction {
    offset_hours.mul(&Fraction::from(SECONDS_PER_HOUR))
}

/// Civil wall-clock fields with an exact (possibly fractional) second.
pub struct Civil {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub hour: i64,
    pub minute: i64,
    pub second: Fraction,
}

/// Split an instant (seconds since epoch) into civil fields at `offset_hours`.
pub fn instant_to_civil(instant: &Fraction, offset_hours: &Fraction) -> Result<Civil, String> {
    let local = instant.add(&offset_seconds(offset_hours));
    let floor = local.floor();
    let frac = local.sub(&floor);
    let total_secs = floor.numerator(); // denominator is 1

    let days = total_secs.div_floor(&BigInt::from(SECONDS_PER_DAY));
    let sod = &total_secs - &days * BigInt::from(SECONDS_PER_DAY); // [0, 86400)

    let days_i64 = days
        .to_i64()
        .ok_or_else(|| "instant out of supported civil range".to_string())?;
    let sod_i64 = sod.to_i64().expect("seconds-of-day fits i64");

    let (year, month, day) = civil_from_days(days_i64);
    let hour = sod_i64 / SECONDS_PER_HOUR;
    let minute = (sod_i64 % SECONDS_PER_HOUR) / 60;
    let second_int = sod_i64 % 60;
    let second = Fraction::from(second_int).add(&frac);

    Ok(Civil {
        year,
        month,
        day,
        hour,
        minute,
        second,
    })
}

/// Inverse of [`instant_to_civil`]: civil fields at `offset_hours` -> instant.
pub fn civil_to_instant(civil: &Civil, offset_hours: &Fraction) -> Fraction {
    let days = days_from_civil(civil.year, civil.month, civil.day);
    let second_floor = civil.second.floor();
    let second_frac = civil.second.sub(&second_floor);
    let second_int = second_floor.numerator();

    let int_secs = BigInt::from(days) * BigInt::from(SECONDS_PER_DAY)
        + BigInt::from(civil.hour) * BigInt::from(SECONDS_PER_HOUR)
        + BigInt::from(civil.minute) * BigInt::from(60)
        + second_int;

    let local = Fraction::new(int_secs, BigInt::from(1)).add(&second_frac);
    local.sub(&offset_seconds(offset_hours))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_thursday() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(iso_weekday(1970, 1, 1), 4); // Thursday
    }

    #[test]
    fn civil_roundtrip_across_epoch() {
        for &(y, m, d) in &[(1969, 12, 31), (1970, 1, 1), (2000, 2, 29), (2024, 11, 25)] {
            let days = days_from_civil(y, m, d);
            assert_eq!(civil_from_days(days), (y, m, d));
        }
    }

    #[test]
    fn known_weekdays() {
        assert_eq!(iso_weekday(2024, 11, 25), 1); // Monday
        assert_eq!(iso_weekday(2000, 1, 1), 6); // Saturday
    }

    #[test]
    fn month_lengths_and_leap() {
        assert_eq!(days_in_month(2024, 2), 29); // leap
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 4), 30);
    }

    #[test]
    fn add_months_clamps_to_month_end() {
        assert_eq!(add_months_civil(2024, 1, 31, 1), (2024, 2, 29)); // leap Feb
        assert_eq!(add_months_civil(2023, 1, 31, 1), (2023, 2, 28));
        assert_eq!(add_months_civil(2024, 12, 15, 1), (2025, 1, 15)); // year rollover
        assert_eq!(add_months_civil(2024, 3, 15, -4), (2023, 11, 15)); // backward
        assert_eq!(add_months_civil(2024, 2, 29, 12), (2025, 2, 28)); // +1 year clamps
    }
}
