//! # ajisai-wasm
//!
//! A WebAssembly binding for Ajisai Core, for browser hosts.
//!
//! **This is a host, not part of the language.** Ajisai Core does not depend on
//! this crate and does not know it exists; nothing here changes what a program
//! means. Everything below is presentation plumbing in the sense of
//! `SPECIFICATION.md` §14 and `docs/playground-ui.md`: it moves strings across
//! the WebAssembly boundary and says what changed, so a user interface can
//! decide what to show.
//!
//! ## Why there is no `wasm-bindgen`
//!
//! The whole interface is a string in and a JSON string out. That needs an
//! allocator, a pointer, and a length — which is a hand-written C ABI over
//! linear memory, about a hundred lines on each side. Taking `wasm-bindgen`
//! instead would mean a proc-macro dependency, a `wasm-pack`/`wasm-bindgen-cli`
//! build step, and a generated JavaScript shim, in exchange for ergonomics this
//! interface does not need. `cargo build --target wasm32-unknown-unknown` is
//! the only build step there is.
//!
//! ## The protocol
//!
//! Every call follows the same shape:
//!
//! 1. JS calls [`ajisai_alloc`] and writes UTF-8 source into linear memory.
//! 2. JS calls one of the entry points with that pointer and length. The call
//!    returns the byte length of a JSON reply.
//! 3. JS reads that many bytes from [`ajisai_reply`] and decodes UTF-8.
//! 4. JS calls [`ajisai_free`] on the argument buffer.
//!
//! The reply buffer belongs to this module and stays valid until the next call.

use std::alloc::{alloc, dealloc, Layout};
use std::cell::RefCell;

use ajisai_core::lint::{self, Severity};
use ajisai_core::{manifest, syntax, Interpreter};

/// The longest rendering of one value the host will be handed.
///
/// `0 1000000 RANGE` is a legal program and its vector renders to megabytes of
/// text that no panel can show. Truncating is a presentation decision and it
/// belongs here rather than in the renderer: the value is untouched, and only
/// the string crossing the boundary is shortened.
const RENDER_LIMIT: usize = 4000;

thread_local! {
    static SESSION: RefCell<Interpreter> = RefCell::new(Interpreter::new());
    static REPLY: RefCell<String> = const { RefCell::new(String::new()) };
}

// --------------------------------------------------------------- memory

/// Allocate `len` bytes of linear memory for the caller to write into.
///
/// # Safety
/// The caller must pass the same `len` to [`ajisai_free`], and must not use the
/// pointer afterwards.
#[no_mangle]
pub unsafe extern "C" fn ajisai_alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return std::ptr::null_mut();
    }
    match Layout::from_size_align(len, 1) {
        Ok(layout) => alloc(layout),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Release a buffer from [`ajisai_alloc`].
///
/// # Safety
/// `ptr` must have come from [`ajisai_alloc`] with the same `len`.
#[no_mangle]
pub unsafe extern "C" fn ajisai_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    if let Ok(layout) = Layout::from_size_align(len, 1) {
        dealloc(ptr, layout);
    }
}

/// A pointer to the JSON reply from the most recent call.
#[no_mangle]
pub extern "C" fn ajisai_reply() -> *const u8 {
    REPLY.with(|reply| reply.borrow().as_ptr())
}

// ------------------------------------------------------------ entry points

/// Run a source fragment against the session, and describe what happened.
///
/// # Safety
/// `ptr` must point to `len` bytes of UTF-8.
#[no_mangle]
pub unsafe extern "C" fn ajisai_execute(ptr: *const u8, len: usize) -> usize {
    let source = read_source(ptr, len);
    reply(SESSION.with(|session| execute_json(&mut session.borrow_mut(), &source)))
}

/// Lint a source fragment. The session is not touched.
///
/// # Safety
/// `ptr` must point to `len` bytes of UTF-8.
#[no_mangle]
pub unsafe extern "C" fn ajisai_lint(ptr: *const u8, len: usize) -> usize {
    let source = read_source(ptr, len);
    reply(SESSION.with(|session| lint_json(&session.borrow(), &source)))
}

/// Render a fragment in canonical form — the formatter, over the boundary.
///
/// # Safety
/// `ptr` must point to `len` bytes of UTF-8.
#[no_mangle]
pub unsafe extern "C" fn ajisai_format(ptr: *const u8, len: usize) -> usize {
    let source = read_source(ptr, len);
    reply(match syntax::parse(&source) {
        Ok(program) => object(&[
            ("ok", "true".to_string()),
            ("text", quote(&syntax::render_program(&program))),
        ]),
        Err(error) => object(&[
            ("ok", "false".to_string()),
            ("error", quote(&error.to_string())),
        ]),
    })
}

/// Split a fragment into the steps a host can run one at a time.
///
/// A step is one source unit, using [`ajisai_core::interpreter::unit_len`] —
/// the same rule `VENT` uses. That matters: splitting `TRUE VENT { 1 }` into
/// three steps would hand `VENT` a body with no unit to release, so a stepper
/// that segmented naively would fail on programs that run perfectly.
///
/// # Safety
/// `ptr` must point to `len` bytes of UTF-8.
#[no_mangle]
pub unsafe extern "C" fn ajisai_steps(ptr: *const u8, len: usize) -> usize {
    let source = read_source(ptr, len);
    reply(match syntax::parse(&source) {
        Err(error) => object(&[
            ("ok", "false".to_string()),
            ("error", quote(&error.to_string())),
        ]),
        Ok(program) => {
            let mut steps = Vec::new();
            let mut index = 0;
            while index < program.len() {
                let span = match ajisai_core::interpreter::unit_len(&program, index) {
                    Ok(span) => span,
                    // A trailing `VENT` has no unit. Hand the remainder over as
                    // one step and let the evaluator raise the real error.
                    Err(_) => program.len() - index,
                };
                let end = (index + span).min(program.len());
                steps.push(quote(&syntax::render_program(&program[index..end])));
                index = end;
            }
            object(&[("ok", "true".to_string()), ("steps", array(&steps))])
        }
    })
}

/// The vocabulary manifest: every registered word's contract, as JSON.
#[no_mangle]
pub extern "C" fn ajisai_vocabulary() -> usize {
    reply(SESSION.with(|session| manifest::vocabulary_json(&session.borrow())))
}

/// Discard the session and start a fresh one.
#[no_mangle]
pub extern "C" fn ajisai_reset() -> usize {
    SESSION.with(|session| *session.borrow_mut() = Interpreter::new());
    reply(SESSION.with(|session| snapshot_json(&session.borrow())))
}

/// The session's current state, without running anything.
#[no_mangle]
pub extern "C" fn ajisai_snapshot() -> usize {
    reply(SESSION.with(|session| snapshot_json(&session.borrow())))
}

// ------------------------------------------------------------------ inner

unsafe fn read_source(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    let bytes = std::slice::from_raw_parts(ptr, len);
    String::from_utf8_lossy(bytes).into_owned()
}

fn reply(json: String) -> usize {
    REPLY.with(|slot| {
        let mut slot = slot.borrow_mut();
        *slot = json;
        slot.len()
    })
}

fn execute_json(interpreter: &mut Interpreter, source: &str) -> String {
    match interpreter.execute(source) {
        Ok(()) => {
            let mut fields = vec![("ok", "true".to_string()), ("error", "null".to_string())];
            fields.extend(state_fields(interpreter));
            object(&fields)
        }
        Err(error) => {
            let mut fields = vec![
                ("ok", "false".to_string()),
                ("error", quote(&error.to_string())),
            ];
            fields.extend(state_fields(interpreter));
            object(&fields)
        }
    }
}

fn snapshot_json(interpreter: &Interpreter) -> String {
    let mut fields = vec![("ok", "true".to_string()), ("error", "null".to_string())];
    fields.extend(state_fields(interpreter));
    object(&fields)
}

/// The two observable surfaces a run can change: the flow, and the dictionary.
fn state_fields(interpreter: &Interpreter) -> Vec<(&'static str, String)> {
    let stack: Vec<String> = interpreter
        .stack()
        .iter()
        .map(|value| quote(&truncate(&value.to_string())))
        .collect();
    let definitions: Vec<String> = interpreter
        .definitions()
        .iter()
        .map(|(name, body)| {
            object(&[
                ("name", quote(name)),
                (
                    "body",
                    quote(&truncate(&format!(
                        "{{ {} }}",
                        syntax::render_program(body)
                    ))),
                ),
            ])
        })
        .collect();
    vec![
        ("stack", array(&stack)),
        ("definitions", array(&definitions)),
    ]
}

fn lint_json(interpreter: &Interpreter, source: &str) -> String {
    match lint::lint(interpreter, source) {
        Ok(findings) => {
            let items: Vec<String> = findings
                .iter()
                .map(|finding| {
                    object(&[
                        (
                            "severity",
                            quote(match finding.severity {
                                Severity::Error => "error",
                                Severity::Advisory => "advisory",
                            }),
                        ),
                        ("message", quote(&finding.message)),
                    ])
                })
                .collect();
            object(&[("ok", "true".to_string()), ("findings", array(&items))])
        }
        Err(error) => object(&[
            ("ok", "false".to_string()),
            ("error", quote(&error.to_string())),
        ]),
    }
}

fn truncate(text: &str) -> String {
    if text.chars().count() <= RENDER_LIMIT {
        return text.to_string();
    }
    let head: String = text.chars().take(RENDER_LIMIT).collect();
    format!("{head}…")
}

// ------------------------------------------------------------------- JSON
//
// Written out rather than pulled in, for the same reason `ajisai-core` writes
// its own manifest: the shape is small and fixed.

fn object(fields: &[(&str, String)]) -> String {
    let body: Vec<String> = fields
        .iter()
        .map(|(key, value)| format!("{}:{}", quote(key), value))
        .collect();
    format!("{{{}}}", body.join(","))
}

fn array(items: &[String]) -> String {
    format!("[{}]", items.join(","))
}

fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pull the `steps` array out of a reply. Written properly rather than by
    /// splitting on `","`, because a step like `"DOUBLE"` carries escaped
    /// quotes of its own.
    fn decode_steps(json: &str) -> Vec<String> {
        let body = json
            .split(r#""steps":["#)
            .nth(1)
            .expect("a steps array")
            .trim_end_matches("]}");
        let mut steps = Vec::new();
        let mut chars = body.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '"' {
                continue;
            }
            let mut step = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => step.push(chars.next().expect("an escape")),
                    '"' => break,
                    other => step.push(other),
                }
            }
            steps.push(step);
        }
        steps
    }

    fn call(source: &str, entry: unsafe extern "C" fn(*const u8, usize) -> usize) -> String {
        let bytes = source.as_bytes();
        let len = unsafe { entry(bytes.as_ptr(), bytes.len()) };
        REPLY.with(|reply| reply.borrow()[..len].to_string())
    }

    #[test]
    fn execute_reports_the_flow() {
        let json = call("1 2 ADD", ajisai_execute);
        assert!(json.contains("\"ok\":true"), "{json}");
        assert!(json.contains("\"stack\":[\"3\"]"), "{json}");
        ajisai_reset();
    }

    #[test]
    fn execute_reports_an_error_and_the_flow_it_left() {
        ajisai_reset();
        let json = call("7 1 0 DIV", ajisai_execute);
        assert!(json.contains("\"ok\":false"), "{json}");
        assert!(json.contains("division by zero"), "{json}");
        // Word-level atomicity is visible across the boundary too.
        assert!(json.contains("\"stack\":[\"7\",\"1\",\"0\"]"), "{json}");
        ajisai_reset();
    }

    #[test]
    fn definitions_come_back_as_source() {
        ajisai_reset();
        let json = call("{ 2 MUL } \"DOUBLE\" DEF", ajisai_execute);
        assert!(json.contains("\"name\":\"DOUBLE\""), "{json}");
        assert!(json.contains("\"body\":\"{ 2 MUL }\""), "{json}");
        ajisai_reset();
    }

    #[test]
    fn the_session_persists_between_calls() {
        ajisai_reset();
        call("1 2", ajisai_execute);
        let json = call("ADD", ajisai_execute);
        assert!(json.contains("\"stack\":[\"3\"]"), "{json}");
        ajisai_reset();
    }

    #[test]
    fn reset_empties_the_session() {
        call("{ 1 } \"X\" DEF 9", ajisai_execute);
        let len = ajisai_reset();
        let json = REPLY.with(|reply| reply.borrow()[..len].to_string());
        assert!(json.contains("\"stack\":[]"), "{json}");
        assert!(json.contains("\"definitions\":[]"), "{json}");
    }

    #[test]
    fn lint_and_format_do_not_touch_the_session() {
        ajisai_reset();
        call("1 2", ajisai_execute);
        let findings = call("[ 1 ] 2 ADD", ajisai_lint);
        assert!(findings.contains("\"severity\":\"error\""), "{findings}");
        let formatted = call("1 2 & +", ajisai_format);
        assert!(formatted.contains("1 2 KEEP ADD"), "{formatted}");
        let snapshot = {
            let len = ajisai_snapshot();
            REPLY.with(|reply| reply.borrow()[..len].to_string())
        };
        assert!(snapshot.contains("\"stack\":[\"1\",\"2\"]"), "{snapshot}");
        ajisai_reset();
    }

    #[test]
    fn the_vocabulary_crosses_the_boundary() {
        let len = ajisai_vocabulary();
        let json = REPLY.with(|reply| reply.borrow()[..len].to_string());
        assert!(json.contains("\"name\": \"VENT\""), "{json}");
        assert!(json.contains("\"name\": \"STAK\""), "{json}");
    }

    /// A legal program can render to megabytes. The value is untouched; only
    /// the string crossing the boundary is shortened.
    #[test]
    fn an_enormous_rendering_is_truncated() {
        ajisai_reset();
        let json = call("0 200000 RANGE", ajisai_execute);
        assert!(json.contains('…'), "should be truncated");
        assert!(json.len() < RENDER_LIMIT * 4, "reply stayed bounded");
        ajisai_reset();
    }

    #[test]
    fn text_and_quotes_survive_json_encoding() {
        ajisai_reset();
        let json = call("\"a\\\"b\" { 1 ADD }", ajisai_execute);
        assert!(json.contains("\\\\\\\""), "quote should be escaped: {json}");
        ajisai_reset();
    }

    /// A step is one source unit — so a vent and the unit it governs stay
    /// together, and a mode word stays with the word it governs. Splitting them
    /// would break programs that run perfectly.
    #[test]
    fn steps_respect_the_source_unit_rule() {
        let json = call("1 2 ADD", ajisai_steps);
        assert!(json.contains(r#"["1","2","ADD"]"#), "{json}");

        let json = call("TRUE VENT { 1 0 DIV } 7", ajisai_steps);
        assert!(
            json.contains(r#"["TRUE","VENT { 1 0 DIV }","7"]"#),
            "a vent must keep its unit: {json}"
        );

        let json = call("1 2 3 STAK ADD", ajisai_steps);
        assert!(
            json.contains(r#"["1","2","3","STAK ADD"]"#),
            "a mode must keep its word: {json}"
        );

        let json = call("TRUE VENT", ajisai_steps);
        assert!(json.contains("\"ok\":true"), "{json}");
    }

    /// Stepping a program produces the same flow as running it whole.
    #[test]
    fn stepping_and_running_agree() {
        for source in [
            "1 2 ADD 3 MUL",
            "1 2 3 STAK ADD",
            "5 0 GT KEEP VENT { 1 } NOT VENT { 2 }",
            "{ 2 MUL } \"DOUBLE\" DEF 21 DOUBLE",
            "[ 1 2 3 ] { 2 MUL } MAP",
        ] {
            ajisai_reset();
            let whole = call(source, ajisai_execute);

            ajisai_reset();
            let steps_json = call(source, ajisai_steps);
            let steps = decode_steps(&steps_json);
            let mut stepped = String::new();
            for step in steps {
                stepped = call(&step, ajisai_execute);
            }
            let flow = |json: &str| json.split(r#""stack":"#).nth(1).unwrap().to_string();
            assert_eq!(flow(&whole), flow(&stepped), "`{source}` disagreed");
        }
        ajisai_reset();
    }

    #[test]
    fn allocation_round_trips() {
        unsafe {
            let ptr = ajisai_alloc(8);
            assert!(!ptr.is_null());
            std::ptr::write_bytes(ptr, b'x', 8);
            assert_eq!(std::slice::from_raw_parts(ptr, 8), b"xxxxxxxx");
            ajisai_free(ptr, 8);
            assert!(ajisai_alloc(0).is_null());
        }
    }
}
