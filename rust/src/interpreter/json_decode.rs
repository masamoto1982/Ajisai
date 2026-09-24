//! `JSON-DECODE` — read JSON text into values and Records
//! (LANG.VALUES.DISJOINT, LANG.RECORDS.STRUCTURE, LANG.VALUES.EXACT).
//!
//! JSON is what structured data from a host arrives as, and its two
//! containers land on the two domains built for them: an object is a Record
//! keyed by its member names in order, an array a Vector. A number becomes
//! the exact rational it spells — the JSON number grammar is a subset of the
//! language's own, and `0.1` is one tenth, never the nearest float. `null` is
//! a NIL, the absent value.
//!
//! The decoder is iterative: an explicit stack of open containers replaces
//! recursion, so nesting is bounded by the text alone rather than by the
//! host's call stack. This is also why the Word is native and not a
//! definition — nesting is input-dependent repetition, and the language
//! repeats only over a Vector that already exists. Text that is not exactly
//! one JSON value projects `invalidEncoding`, `NUM`'s reason for text that
//! spells no number; the outcome is never a guess at what was meant.
//!
//! Nesting is bounded all the same, by the machine rather than by the
//! grammar: a value nested past `MAX_NESTING` is well-formed JSON the machine
//! will not hold (its own value representation walks structure recursively
//! when it is compared, rendered or released), so such text projects
//! `spaceExhausted`, the outcome every other materialization past a ceiling
//! reaches (LANG.MACHINE.LIMITS), not `invalidEncoding`.

use num_bigint::BigInt;
use num_traits::Zero;

use super::ordering_ops::{restore, take_operand};
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::collection_meter;
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::fraction::Fraction;
use crate::types::{RecordData, Value};

/// Deepest container nesting one decoded value may hold. Past it the text
/// is refused as too large, not as malformed.
pub(crate) const MAX_NESTING: usize = 512;

/// Why text did not decode.
#[derive(Debug, PartialEq, Eq)]
enum Reject {
    /// Not exactly one JSON value.
    Malformed,
    /// One JSON value, nested deeper than `MAX_NESTING`.
    TooDeep,
}

/// An open container on the decoder's own stack.
enum Frame {
    Array(Vec<Value>),
    Object {
        keys: Vec<Value>,
        values: Vec<Value>,
        pending_key: Option<Value>,
    },
}

struct Decoder<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn eat(&mut self, expected: u8) -> Option<()> {
        (self.peek() == Some(expected)).then(|| self.pos += 1)
    }

    fn literal(&mut self, word: &[u8], value: Value) -> Option<Value> {
        if self.bytes[self.pos..].starts_with(word) {
            self.pos += word.len();
            Some(value)
        } else {
            None
        }
    }

    fn digits(&mut self) -> Option<&'a [u8]> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        (self.pos > start).then(|| &self.bytes[start..self.pos])
    }

    /// `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`, as an exact rational.
    fn number(&mut self) -> Option<Value> {
        let negative = self.eat(b'-').is_some();
        let int_digits = self.digits()?;
        if int_digits.len() > 1 && int_digits[0] == b'0' {
            return None;
        }
        let mut mantissa = int_digits.to_vec();
        let mut scale: u32 = 0;
        if self.eat(b'.').is_some() {
            let frac = self.digits()?;
            scale = u32::try_from(frac.len()).ok()?;
            mantissa.extend_from_slice(frac);
        }
        let mut exponent: i64 = 0;
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            let sign = match self.peek() {
                Some(b'-') => {
                    self.pos += 1;
                    -1
                }
                Some(b'+') => {
                    self.pos += 1;
                    1
                }
                _ => 1,
            };
            let exp_digits = self.digits()?;
            let magnitude: i64 = std::str::from_utf8(exp_digits).ok()?.parse().ok()?;
            exponent = sign * magnitude;
        }
        let mut numerator = BigInt::parse_bytes(&mantissa, 10)?;
        if negative {
            numerator = -numerator;
        }
        let mut denominator = BigInt::from(10).pow(scale);
        let shift = u32::try_from(exponent.unsigned_abs()).ok()?;
        if numerator.is_zero() {
            // Zero at any scale; skips the (possibly enormous) power.
        } else if exponent >= 0 {
            numerator *= BigInt::from(10).pow(shift);
        } else {
            denominator *= BigInt::from(10).pow(shift);
        }
        Some(Value::from_fraction(Fraction::new(numerator, denominator)))
    }

    fn hex4(&mut self) -> Option<u32> {
        let slice = self.bytes.get(self.pos..self.pos + 4)?;
        let code = u32::from_str_radix(std::str::from_utf8(slice).ok()?, 16).ok()?;
        self.pos += 4;
        Some(code)
    }

    /// A JSON string, the opening quote already seen.
    fn string(&mut self) -> Option<String> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let byte = self.peek()?;
            self.pos += 1;
            match byte {
                b'"' => break,
                b'\\' => {
                    let escaped = self.peek()?;
                    self.pos += 1;
                    match escaped {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0C),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let mut code = self.hex4()?;
                            if (0xD800..0xDC00).contains(&code) {
                                self.eat(b'\\')?;
                                self.eat(b'u')?;
                                let low = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&low) {
                                    return None;
                                }
                                code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                            }
                            let ch = char::from_u32(code)?;
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return None,
                    }
                }
                0x00..=0x1F => return None,
                other => out.push(other),
            }
        }
        String::from_utf8(out).ok()
    }

    /// One scalar value, or the opening of a container (pushed onto `frames`).
    fn open_value(
        &mut self,
        frames: &mut Vec<Frame>,
    ) -> std::result::Result<Option<Value>, Reject> {
        self.skip_whitespace();
        let open = |frames: &mut Vec<Frame>, frame: Frame| {
            if frames.len() >= MAX_NESTING {
                return Err(Reject::TooDeep);
            }
            frames.push(frame);
            Ok(None)
        };
        Ok(match self.peek().ok_or(Reject::Malformed)? {
            b'{' => {
                self.pos += 1;
                return open(
                    frames,
                    Frame::Object {
                        keys: Vec::new(),
                        values: Vec::new(),
                        pending_key: None,
                    },
                );
            }
            b'[' => {
                self.pos += 1;
                return open(frames, Frame::Array(Vec::new()));
            }
            b'"' => Some(Value::from_string(&self.string().ok_or(Reject::Malformed)?)),
            b't' => Some(
                self.literal(b"true", Value::from_bool(true))
                    .ok_or(Reject::Malformed)?,
            ),
            b'f' => Some(
                self.literal(b"false", Value::from_bool(false))
                    .ok_or(Reject::Malformed)?,
            ),
            b'n' => Some(
                self.literal(b"null", Value::nil())
                    .ok_or(Reject::Malformed)?,
            ),
            b'-' | b'0'..=b'9' => Some(self.number().ok_or(Reject::Malformed)?),
            _ => return Err(Reject::Malformed),
        })
    }

    /// Parse the whole text as exactly one JSON value.
    fn decode(&mut self) -> std::result::Result<Value, Reject> {
        let mut frames: Vec<Frame> = Vec::new();
        // A completed value waiting to be attached to its container, or to be
        // the answer when no container is open.
        let mut completed: Option<Value> = self.open_value(&mut frames)?;
        loop {
            if let Some(value) = completed.take() {
                let Some(frame) = frames.last_mut() else {
                    self.skip_whitespace();
                    return if self.pos == self.bytes.len() {
                        Ok(value)
                    } else {
                        Err(Reject::Malformed)
                    };
                };
                match frame {
                    Frame::Array(items) => items.push(value),
                    Frame::Object {
                        keys,
                        values,
                        pending_key,
                    } => {
                        keys.push(pending_key.take().ok_or(Reject::Malformed)?);
                        values.push(value);
                    }
                }
                self.skip_whitespace();
                let closer = match frame {
                    Frame::Array(_) => b']',
                    Frame::Object { .. } => b'}',
                };
                match self.peek().ok_or(Reject::Malformed)? {
                    b',' => {
                        self.pos += 1;
                        completed = self.open_member(&mut frames)?;
                    }
                    byte if byte == closer => {
                        self.pos += 1;
                        completed = Some(Self::close(frames.pop().ok_or(Reject::Malformed)?)?);
                    }
                    _ => return Err(Reject::Malformed),
                }
                continue;
            }
            // A container was just opened: it is empty, or its first member
            // begins here.
            self.skip_whitespace();
            let closer = match frames.last().ok_or(Reject::Malformed)? {
                Frame::Array(_) => b']',
                Frame::Object { .. } => b'}',
            };
            if self.peek().ok_or(Reject::Malformed)? == closer {
                self.pos += 1;
                completed = Some(Self::close(frames.pop().ok_or(Reject::Malformed)?)?);
            } else {
                completed = self.open_member(&mut frames)?;
            }
        }
    }

    /// The next member of the innermost container: a key and a value for an
    /// object, a value for an array.
    fn open_member(
        &mut self,
        frames: &mut Vec<Frame>,
    ) -> std::result::Result<Option<Value>, Reject> {
        if let Some(Frame::Object { pending_key, .. }) = frames.last_mut() {
            self.skip_whitespace();
            let key = self.string().ok_or(Reject::Malformed)?;
            *pending_key = Some(Value::from_string(&key));
            self.skip_whitespace();
            self.eat(b':').ok_or(Reject::Malformed)?;
        }
        self.open_value(frames)
    }

    fn close(frame: Frame) -> std::result::Result<Value, Reject> {
        Ok(match frame {
            Frame::Array(items) => Value::from_vector(items),
            Frame::Object { keys, values, .. } => {
                Value::from_record(RecordData::new(keys, values).map_err(|_| Reject::Malformed)?)
            }
        })
    }
}

/// `JSON-DECODE ( [ text ] -> [ value ] )`: projects `invalidEncoding`.
pub(crate) fn op_json_decode(interp: &mut Interpreter) -> Result<()> {
    let operand = take_operand(interp)?;
    let Some(text) = operand.as_text().map(str::to_owned) else {
        restore(interp, operand);
        return Err(AjisaiError::declared(
            "nonText",
            "JSON-DECODE: expected a String of JSON text",
        ));
    };
    // One pass over the text, charged before it runs.
    if let Err(e) = collection_meter::charge(interp, text.len() as u64 + 1) {
        restore(interp, operand);
        return Err(e);
    }
    let mut decoder = Decoder {
        bytes: text.as_bytes(),
        pos: 0,
    };
    match decoder.decode() {
        Ok(value) => {
            interp.stack.push(value);
        }
        Err(Reject::Malformed) => interp.stack.push(Value::nil_with_reason(
            NilReason::InvalidEncoding,
            Recoverability::Recoverable,
        )),
        Err(Reject::TooDeep) => interp.stack.push(Value::nil_with_reason(
            NilReason::SpaceExhausted,
            Recoverability::Unknown,
        )),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(text: &str) -> Option<Value> {
        Decoder {
            bytes: text.as_bytes(),
            pos: 0,
        }
        .decode()
        .ok()
    }

    fn reject(text: &str) -> Option<Reject> {
        Decoder {
            bytes: text.as_bytes(),
            pos: 0,
        }
        .decode()
        .err()
    }

    #[test]
    fn numbers_are_exact_rationals() {
        assert_eq!(decode("0.1").unwrap().to_string(), "1/10");
        assert_eq!(decode("-2.5e1").unwrap().to_string(), "-25/1");
        assert_eq!(decode("1E-2").unwrap().to_string(), "1/100");
        assert_eq!(decode("0e99999999").unwrap().to_string(), "0/1");
        for bad in ["01", "1.", ".5", "+1", "1e", "--1", "0x1"] {
            assert!(decode(bad).is_none(), "{bad} must not decode");
        }
    }

    #[test]
    fn strings_decode_their_escapes() {
        assert_eq!(
            decode(r#""a\"b\\c\/\né😀""#).unwrap().as_text(),
            Some("a\"b\\c/\né😀")
        );
        assert!(decode("\"tab\there\"").is_none());
        assert!(decode(r#""\ud83d""#).is_none());
        assert!(decode("\"open").is_none());
    }

    #[test]
    fn containers_nest_to_the_bound_and_are_refused_past_it() {
        let deep = "[".repeat(MAX_NESTING) + &"]".repeat(MAX_NESTING);
        assert!(decode(&deep).is_some());
        let deeper = "[".repeat(MAX_NESTING + 1) + &"]".repeat(MAX_NESTING + 1);
        assert_eq!(reject(&deeper), Some(Reject::TooDeep));
        // Too deep is decided at the opening bracket, before the text is
        // known to be well formed: the machine could not hold it either way.
        assert_eq!(reject(&"[".repeat(100_000)), Some(Reject::TooDeep));
        assert_eq!(reject(&"[".repeat(10)), Some(Reject::Malformed));
        assert_eq!(
            decode("[1,[2,[]],{}]").unwrap().to_string(),
            "[ 1/1 [ 2/1 [ ] ] { } ]"
        );
        assert_eq!(
            decode(r#" { "a" : 1 , "b" : [ true , null ] } "#)
                .unwrap()
                .to_string(),
            "{ 'a' 1/1 'b' [ TRUE NIL ] }"
        );
    }

    #[test]
    fn malformed_text_has_no_value() {
        for bad in [
            "",
            " ",
            "[1,]",
            "{\"a\":1,}",
            "{\"a\"}",
            "{a:1}",
            "[1] 2",
            "nul",
            "tru",
            "[1 2]",
            "{\"a\":1 \"b\":2}",
            "{\"a\":1,\"a\":2}",
            "{,}",
            "[,]",
        ] {
            assert!(decode(bad).is_none(), "{bad:?} must not decode");
        }
    }
}
