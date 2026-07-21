//! TIME module words built on the BigQuery date/time philosophy: a timezone
//! is never stored in a value, only supplied at the instant <-> civil boundary
//! as a UTC offset in hours (an exact rational).
//!
//! Value shapes (all timezone-free except the instant):
//! - TIMESTAMP (instant): a scalar = exact seconds since the Unix epoch (UTC).
//! - DATETIME (civil): `[year month day hour minute second]` (second exact).
//! - DATE (civil): `[year month day]`.
//! - TIME (civil): `[hour minute second]` (second exact).

use num_bigint::BigInt;

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::cast::cast_value_helpers::is_string_value_with_hint;
use crate::interpreter::time_calendar::{
    add_months_civil, civil_from_days, civil_to_instant, days_from_civil, instant_to_civil,
    iso_weekday, Civil,
};
use crate::interpreter::value_extraction_helpers::{
    extract_operands, push_result, value_as_string,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::semantic::{AbsenceOrigin, Recoverability};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value};

fn require_stack_top(interp: &Interpreter, word: &str) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: word.into(),
            mode: "Stack".into(),
        });
    }
    Ok(())
}

fn restore(interp: &mut Interpreter, operands: Vec<Value>) {
    if interp.consumption_mode != ConsumptionMode::Keep {
        interp.stack.extend(operands);
    }
}

fn scalar(value: &Value, word: &str, what: &str) -> Result<Fraction> {
    value
        .as_scalar()
        .cloned()
        .ok_or_else(|| AjisaiError::from(format!("{}: {} must be a number", word, what)))
}

fn integer_field(value: &Value, word: &str, what: &str) -> Result<i64> {
    let f = scalar(value, word, what)?;
    if !f.is_integer() {
        return Err(AjisaiError::from(format!(
            "{}: {} must be an integer",
            word, what
        )));
    }
    f.to_i64()
        .ok_or_else(|| AjisaiError::from(format!("{}: {} out of range", word, what)))
}

/// Read a civil vector of `expected` length, returning a borrowed view.
fn civil_components(value: &Value, word: &str, expected: &[usize]) -> Result<Vec<Value>> {
    let view = value
        .as_vector_view()
        .ok_or_else(|| AjisaiError::from(format!("{}: expected a civil vector", word)))?;
    if !expected.contains(&view.len()) {
        return Err(AjisaiError::from(format!(
            "{}: civil vector must have {} elements",
            word,
            expected
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(" or ")
        )));
    }
    Ok(view.into_owned())
}

fn civil_from_datetime(components: &[Value], word: &str) -> Result<Civil> {
    Ok(Civil {
        year: integer_field(&components[0], word, "year")?,
        month: integer_field(&components[1], word, "month")?,
        day: integer_field(&components[2], word, "day")?,
        hour: integer_field(&components[3], word, "hour")?,
        minute: integer_field(&components[4], word, "minute")?,
        second: scalar(&components[5], word, "second")?,
    })
}

fn datetime_value(civil: &Civil) -> Value {
    Value::from_vector(vec![
        Value::from_int(civil.year),
        Value::from_int(civil.month),
        Value::from_int(civil.day),
        Value::from_int(civil.hour),
        Value::from_int(civil.minute),
        Value::from_fraction(civil.second.clone()),
    ])
}

// --- Instant <-> civil conversions (require a UTC-offset-hours timezone) ----

/// `timestamp offset -- datetime`. Render an instant as civil fields at the
/// given UTC offset in hours.
pub fn op_datetime(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "DATETIME")?;
    let operands = extract_operands(interp, 2)?;
    let result = (|| {
        let instant = scalar(&operands[0], "DATETIME", "timestamp")?;
        let offset = scalar(&operands[1], "DATETIME", "offset")?;
        instant_to_civil(&instant, &offset)
            .map_err(|e| AjisaiError::from(format!("DATETIME: {}", e)))
    })();
    match result {
        Ok(civil) => {
            interp.stack.push(datetime_value(&civil));
            Ok(())
        }
        Err(e) => {
            restore(interp, operands);
            Err(e)
        }
    }
}

/// `datetime offset -- timestamp`. Resolve a civil datetime to an instant at
/// the given UTC offset in hours.
pub fn op_timestamp(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "TIMESTAMP")?;
    let operands = extract_operands(interp, 2)?;
    let result = (|| {
        let components = civil_components(&operands[0], "TIMESTAMP", &[6])?;
        let civil = civil_from_datetime(&components, "TIMESTAMP")?;
        let offset = scalar(&operands[1], "TIMESTAMP", "offset")?;
        Ok(civil_to_instant(&civil, &offset))
    })();
    match result {
        Ok(instant) => {
            push_result(interp, Value::from_fraction(instant));
            interp.stack.set_last_role(Interpretation::Timestamp);
            Ok(())
        }
        Err(e) => {
            restore(interp, operands);
            Err(e)
        }
    }
}

// --- Civil extraction (timezone-free) --------------------------------------

fn unary_civil<F>(interp: &mut Interpreter, word: &str, expected: &[usize], f: F) -> Result<()>
where
    F: Fn(&[Value]) -> Result<Value>,
{
    require_stack_top(interp, word)?;
    let operands = extract_operands(interp, 1)?;
    match civil_components(&operands[0], word, expected).and_then(|c| f(&c)) {
        Ok(result) => {
            interp.stack.push(result);
            Ok(())
        }
        Err(e) => {
            restore(interp, operands);
            Err(e)
        }
    }
}

pub fn op_date(interp: &mut Interpreter) -> Result<()> {
    unary_civil(interp, "DATE", &[6], |c| {
        Ok(Value::from_vector(vec![
            c[0].clone(),
            c[1].clone(),
            c[2].clone(),
        ]))
    })
}

pub fn op_time(interp: &mut Interpreter) -> Result<()> {
    unary_civil(interp, "TIME", &[6], |c| {
        Ok(Value::from_vector(vec![
            c[3].clone(),
            c[4].clone(),
            c[5].clone(),
        ]))
    })
}

pub fn op_year(interp: &mut Interpreter) -> Result<()> {
    unary_civil(interp, "YEAR", &[3, 6], |c| Ok(c[0].clone()))
}

pub fn op_month(interp: &mut Interpreter) -> Result<()> {
    unary_civil(interp, "MONTH", &[3, 6], |c| Ok(c[1].clone()))
}

pub fn op_day(interp: &mut Interpreter) -> Result<()> {
    unary_civil(interp, "DAY", &[3, 6], |c| Ok(c[2].clone()))
}

/// HOUR/MINUTE/SECOND read a `[h m s]` TIME (length 3) or the time fields of a
/// `[Y M D h m s]` DATETIME (length 6).
fn time_field_index(len: usize, base: usize) -> usize {
    if len == 6 {
        base + 3
    } else {
        base
    }
}

pub fn op_hour(interp: &mut Interpreter) -> Result<()> {
    unary_civil(interp, "HOUR", &[3, 6], |c| {
        Ok(c[time_field_index(c.len(), 0)].clone())
    })
}

pub fn op_minute(interp: &mut Interpreter) -> Result<()> {
    unary_civil(interp, "MINUTE", &[3, 6], |c| {
        Ok(c[time_field_index(c.len(), 1)].clone())
    })
}

pub fn op_second(interp: &mut Interpreter) -> Result<()> {
    unary_civil(interp, "SECOND", &[3, 6], |c| {
        Ok(c[time_field_index(c.len(), 2)].clone())
    })
}

pub fn op_weekday(interp: &mut Interpreter) -> Result<()> {
    unary_civil(interp, "WEEKDAY", &[3, 6], |c| {
        let y = integer_field(&c[0], "WEEKDAY", "year")?;
        let m = integer_field(&c[1], "WEEKDAY", "month")?;
        let d = integer_field(&c[2], "WEEKDAY", "day")?;
        Ok(Value::from_int(iso_weekday(y, m, d)))
    })
}

// --- Civil arithmetic (exact) ----------------------------------------------

/// `date|datetime n -- date|datetime`. Shift the date part by `n` whole days,
/// preserving any time-of-day fields.
pub fn op_add_days(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "ADD-DAYS")?;
    let operands = extract_operands(interp, 2)?;
    let result = (|| {
        let components = civil_components(&operands[0], "ADD-DAYS", &[3, 6])?;
        let n = integer_field(&operands[1], "ADD-DAYS", "day count")?;
        let y = integer_field(&components[0], "ADD-DAYS", "year")?;
        let m = integer_field(&components[1], "ADD-DAYS", "month")?;
        let d = integer_field(&components[2], "ADD-DAYS", "day")?;
        let (ny, nm, nd) = civil_from_days(days_from_civil(y, m, d) + n);
        let mut out = vec![
            Value::from_int(ny),
            Value::from_int(nm),
            Value::from_int(nd),
        ];
        for elem in components.iter().skip(3) {
            out.push(elem.clone());
        }
        Ok(Value::from_vector(out))
    })();
    match result {
        Ok(value) => {
            interp.stack.push(value);
            Ok(())
        }
        Err(e) => {
            restore(interp, operands);
            Err(e)
        }
    }
}

/// `a b -- n`. Whole-day difference `a - b` between two dates/datetimes.
pub fn op_diff_days(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "DIFF-DAYS")?;
    let operands = extract_operands(interp, 2)?;
    let result = (|| {
        let a = civil_components(&operands[0], "DIFF-DAYS", &[3, 6])?;
        let b = civil_components(&operands[1], "DIFF-DAYS", &[3, 6])?;
        let days_a = days_from_civil(
            integer_field(&a[0], "DIFF-DAYS", "year")?,
            integer_field(&a[1], "DIFF-DAYS", "month")?,
            integer_field(&a[2], "DIFF-DAYS", "day")?,
        );
        let days_b = days_from_civil(
            integer_field(&b[0], "DIFF-DAYS", "year")?,
            integer_field(&b[1], "DIFF-DAYS", "month")?,
            integer_field(&b[2], "DIFF-DAYS", "day")?,
        );
        Ok(days_a - days_b)
    })();
    match result {
        Ok(days) => {
            push_result(interp, Value::from_int(days));
            interp.stack.set_last_role(Interpretation::RawNumber);
            Ok(())
        }
        Err(e) => {
            restore(interp, operands);
            Err(e)
        }
    }
}

// --- Formatting ------------------------------------------------------------

/// `date|datetime -- text`. ISO-8601 string: `YYYY-MM-DD` for a date,
/// `YYYY-MM-DDThh:mm:ss` for a datetime. The integer second is rendered;
/// any exact sub-second part is omitted from this human-readable form.
pub fn op_format(interp: &mut Interpreter) -> Result<()> {
    unary_civil(interp, "FORMAT", &[3, 6], |c| {
        let y = integer_field(&c[0], "FORMAT", "year")?;
        let m = integer_field(&c[1], "FORMAT", "month")?;
        let d = integer_field(&c[2], "FORMAT", "day")?;
        let date = format!("{:04}-{:02}-{:02}", y, m, d);
        let text = if c.len() == 6 {
            let h = integer_field(&c[3], "FORMAT", "hour")?;
            let mi = integer_field(&c[4], "FORMAT", "minute")?;
            let s = scalar(&c[5], "FORMAT", "second")?
                .floor()
                .to_i64()
                .ok_or_else(|| AjisaiError::from("FORMAT: second out of range"))?;
            format!("{}T{:02}:{:02}:{:02}", date, h, mi, s)
        } else {
            date
        };
        Ok(Value::from_string(&text))
    })?;
    interp.stack.set_last_role(Interpretation::Text);
    Ok(())
}

/// Shared body for ADD-MONTHS / ADD-YEARS: shift the date part by `months`,
/// clamping to the target month end, and preserve any time-of-day fields.
fn shift_months(interp: &mut Interpreter, word: &str, months_per_unit: i64) -> Result<()> {
    require_stack_top(interp, word)?;
    let operands = extract_operands(interp, 2)?;
    let result = (|| {
        let components = civil_components(&operands[0], word, &[3, 6])?;
        let units = integer_field(&operands[1], word, "amount")?;
        let y = integer_field(&components[0], word, "year")?;
        let m = integer_field(&components[1], word, "month")?;
        let d = integer_field(&components[2], word, "day")?;
        let (ny, nm, nd) = add_months_civil(y, m, d, units * months_per_unit);
        let mut out = vec![
            Value::from_int(ny),
            Value::from_int(nm),
            Value::from_int(nd),
        ];
        for elem in components.iter().skip(3) {
            out.push(elem.clone());
        }
        Ok(Value::from_vector(out))
    })();
    match result {
        Ok(value) => {
            interp.stack.push(value);
            Ok(())
        }
        Err(e) => {
            restore(interp, operands);
            Err(e)
        }
    }
}

/// `date|datetime n -- date|datetime`. Add `n` months, clamping the day to the
/// target month's last day (Jan 31 + 1 -> Feb 28/29).
pub fn op_add_months(interp: &mut Interpreter) -> Result<()> {
    shift_months(interp, "ADD-MONTHS", 1)
}

/// `date|datetime n -- date|datetime`. Add `n` years, clamping Feb 29 to
/// Feb 28 in non-leap target years.
pub fn op_add_years(interp: &mut Interpreter) -> Result<()> {
    shift_months(interp, "ADD-YEARS", 12)
}

// --- Parsing ---------------------------------------------------------------

/// Parse an ISO-8601 civil string into `[Y M D h m s]`. Accepts a bare date
/// `YYYY-MM-DD` (time defaults to `00:00:00`) or a datetime with a `T` or
/// space separator and optional fractional seconds. Returns `None` for any
/// shape or out-of-range field it cannot interpret.
fn parse_iso_civil(text: &str) -> Option<Civil> {
    let text = text.trim();
    let (date_part, time_part) = match text.split_once(['T', ' ']) {
        Some((d, t)) => (d, Some(t)),
        None => (text, None),
    };

    let mut date_fields = date_part.split('-');
    let year: i64 = date_fields.next()?.parse().ok()?;
    let month: i64 = date_fields.next()?.parse().ok()?;
    let day: i64 = date_fields.next()?.parse().ok()?;
    if date_fields.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let (hour, minute, second) = match time_part {
        None => (0, 0, Fraction::from(0)),
        Some(t) => {
            let mut time_fields = t.split(':');
            let hour: i64 = time_fields.next()?.parse().ok()?;
            let minute: i64 = time_fields.next()?.parse().ok()?;
            let second = parse_second(time_fields.next()?)?;
            if time_fields.next().is_some()
                || !(0..=23).contains(&hour)
                || !(0..=59).contains(&minute)
            {
                return None;
            }
            (hour, minute, second)
        }
    };

    Some(Civil {
        year,
        month,
        day,
        hour,
        minute,
        second,
    })
}

/// Parse a second field `ss` or `ss.fff` into an exact rational in `[0, 61)`.
fn parse_second(field: &str) -> Option<Fraction> {
    let (int_part, frac_part) = match field.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (field, None),
    };
    let secs: i64 = int_part.parse().ok()?;
    if !(0..=60).contains(&secs) {
        return None;
    }
    let mut value = Fraction::from(secs);
    if let Some(frac) = frac_part {
        if frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let numerator: BigInt = frac.parse().ok()?;
        let denominator = BigInt::from(10).pow(frac.len() as u32);
        value = value.add(&Fraction::new(numerator, denominator));
    }
    Some(value)
}

/// `text -- datetime`. Parse an ISO-8601 civil string. Non-text input is
/// malformed use and raises an error (cf. `NUM`); a well-formed string that is
/// not a valid ISO civil value projects to Bubble/NIL (`reason =
/// invalidEncoding`).
pub fn op_parse(interp: &mut Interpreter) -> Result<()> {
    require_stack_top(interp, "PARSE")?;
    let hint = interp.stack.last_role();
    let operands = extract_operands(interp, 1)?;
    if !is_string_value_with_hint(&operands[0], hint) {
        restore(interp, operands);
        return Err(AjisaiError::from("PARSE: expected an ISO-8601 text value"));
    }
    let text = value_as_string(&operands[0]).unwrap_or_default();
    match parse_iso_civil(&text) {
        Some(civil) => interp.stack.push(datetime_value(&civil)),
        None => interp.stack.push(Value::bubble_with_reason(
            NilReason::InvalidEncoding,
            AbsenceOrigin::InvalidEncoding,
            Recoverability::Recoverable,
        )),
    }
    Ok(())
}
