//! The termination argument, held against the implementation.
//!
//! `spec/termination.json` states why every Ajisai evaluation is finite: the
//! places evaluation descends into more evaluation, what strictly decreases at
//! each one, and the invariant underneath — that a Word runs only when a Symbol
//! naming it is written in source, so the DEF-time acyclicity check sees every
//! call a body can make. `scripts/check-termination.mjs` keeps that file closed
//! against the vocabulary; this file runs it.
//!
//! The acyclicity check is the load-bearing part and had no test of its own: no
//! test named it, `tests/conformance` had no case for it, and its only witness
//! was the simplest direct self-reference. Soundness alone is not the property
//! either — a check that refused everything would be sound and would make the
//! language useless — so completeness is tested at the same time, over random
//! definition graphs.

use ajisai_core::interpreter::Interpreter;
use proptest::prelude::*;
use serde_json::Value as Json;
use std::collections::{HashMap, HashSet};

fn spec() -> Json {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../spec/termination.json");
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read the termination argument at {path}: {e}"));
    serde_json::from_str(&text).expect("spec/termination.json is valid JSON")
}

/// Run a program on a fresh interpreter, returning the error text if it failed.
fn run(src: &str) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let mut interp = Interpreter::new();
        interp
            .execute(src)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
}

fn witnesses<'a>(node: &'a Json, path: &[&str]) -> &'a Vec<Json> {
    let mut current = node;
    for key in path {
        current = &current[key];
    }
    current
        .as_array()
        .unwrap_or_else(|| panic!("expected an array of witnesses at {path:?}"))
}

fn source_of(w: &Json) -> &str {
    w["source"].as_str().expect("witness source")
}

/// Every recursion site's witness runs, and finishes without any of the
/// ceilings the argument says are not what stops it. A site whose witness
/// needed a ceiling would mean the descent it names is not actually bounded.
#[test]
fn every_recursion_site_terminates_without_a_ceiling() {
    let s = spec();
    let sites = s["recursionSites"].as_array().expect("recursionSites");
    assert!(
        !sites.is_empty(),
        "the argument must declare recursion sites"
    );

    let ceilings: Vec<String> = s["ceilings"]["notLoadBearing"]
        .as_array()
        .expect("notLoadBearing")
        .iter()
        .map(|v| v.as_str().expect("ceiling id").to_string())
        .collect();

    for site in sites {
        let id = site["id"].as_str().expect("site id");
        let witness = site["witness"].as_str().expect("site witness");
        match run(witness) {
            Ok(()) => {}
            Err(message) => {
                for ceiling in &ceilings {
                    assert!(
                        !message.contains(ceiling.as_str()),
                        "site {id}: witness {witness:?} was stopped by {ceiling}, so what ends it \
                         is a ceiling and not the measure this site claims to decrease",
                    );
                }
                panic!("site {id}: witness {witness:?} failed: {message}");
            }
        }
    }
}

/// The invariant: a String is not a code operand, so a Word cannot be called by
/// a name the program computed. This is what makes the DEF-time check's view of
/// the call graph complete.
#[test]
fn a_string_cannot_name_a_word_to_call() {
    let s = spec();

    for w in witnesses(&s, &["invariant", "witnesses", "refused"]) {
        let src = source_of(w);
        let expect = w["expect"].as_str().expect("expected outcome");
        let category = expect.strip_prefix("error:").expect("an error: outcome");
        let message = run(src).expect_err(&format!("{src:?} must be refused"));
        assert!(
            message.contains("code operand"),
            "{src:?} should be refused as a bad code operand ({category}), got: {message}",
        );
    }

    for w in witnesses(&s, &["invariant", "witnesses", "accepted"]) {
        let src = source_of(w);
        run(src).unwrap_or_else(|e| panic!("{src:?} must still be accepted: {e}"));
    }
}

/// Every acyclicity witness behaves as the argument says: the refused ones are
/// refused for being self-referential, the accepted ones still work.
#[test]
fn acyclicity_witnesses_hold() {
    let s = spec();

    for w in witnesses(&s, &["acyclicity", "witnesses", "refused"]) {
        let src = source_of(w);
        let message = run(src).expect_err(&format!("{src:?} must be refused"));
        assert!(
            message.contains("itself") || message.contains("cycle") || message.contains("self"),
            "{src:?} should be refused as self-referential, got: {message}",
        );
    }

    for w in witnesses(&s, &["acyclicity", "witnesses", "accepted"]) {
        let src = source_of(w);
        run(src).unwrap_or_else(|e| panic!("{src:?} must be accepted: {e}"));
    }
}

/// Build `DEF`s for a graph over W0..Wn where `edges[i]` are the words Wi's
/// body names, and return them in `order`.
fn definition_program(
    n: usize,
    edges: &HashMap<usize, Vec<usize>>,
    order: &[usize],
) -> Vec<String> {
    order
        .iter()
        .map(|&i| {
            let body: String = edges
                .get(&i)
                .map(|targets| {
                    targets
                        .iter()
                        .map(|&t| format!("W{t}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            let body = if body.is_empty() {
                "1".to_string()
            } else {
                body
            };
            format!("[ | {body} ] 'W{i}' DEF")
        })
        .collect::<Vec<_>>()
        .into_iter()
        .take(n.max(order.len()))
        .collect()
}

/// Does the graph restricted to `defined` contain a cycle reachable from any
/// node? Plain DFS; the graph is tiny.
fn has_cycle(edges: &HashMap<usize, Vec<usize>>, defined: &HashSet<usize>) -> bool {
    fn visit(
        node: usize,
        edges: &HashMap<usize, Vec<usize>>,
        defined: &HashSet<usize>,
        stack: &mut HashSet<usize>,
        done: &mut HashSet<usize>,
    ) -> bool {
        if stack.contains(&node) {
            return true;
        }
        if done.contains(&node) || !defined.contains(&node) {
            return false;
        }
        stack.insert(node);
        for &next in edges.get(&node).map(|v| v.as_slice()).unwrap_or(&[]) {
            if visit(next, edges, defined, stack, done) {
                return true;
            }
        }
        stack.remove(&node);
        done.insert(node);
        false
    }

    let mut done = HashSet::new();
    for &node in defined {
        let mut stack = HashSet::new();
        if visit(node, edges, defined, &mut stack, &mut done) {
            return true;
        }
    }
    false
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// Soundness and completeness of the DEF-time acyclicity check, over random
    /// definition graphs defined in random order.
    ///
    /// Soundness: the User dictionary never ends up holding a cycle — whenever
    /// a DEF would close one it is refused, and refused as self-referential.
    /// Completeness: a DEF that would *not* close a cycle always succeeds, so
    /// the check refuses no acyclic program. Definition order is part of the
    /// generated input because a forward reference to a Word that does not
    /// exist yet is a dead end at the moment it is written, and the cycle only
    /// becomes visible at the later DEF that closes it.
    #[test]
    fn def_accepts_exactly_the_acyclic_definitions(
        adjacency in prop::collection::vec(prop::collection::vec(0usize..5, 0..3), 5..=5),
        order in Just((0..5usize).collect::<Vec<_>>()).prop_shuffle(),
    ) {
        let edges: HashMap<usize, Vec<usize>> =
            adjacency.iter().enumerate().map(|(i, t)| (i, t.clone())).collect();

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("tokio current-thread runtime");

        rt.block_on(async {
            let mut interp = Interpreter::new();
            let mut defined: HashSet<usize> = HashSet::new();

            for (index, program) in order
                .iter()
                .zip(definition_program(5, &edges, &order))
                .map(|(i, p)| (*i, p))
            {
                let mut candidate = defined.clone();
                candidate.insert(index);
                let would_cycle = has_cycle(&edges, &candidate);

                let result = interp.execute(&program).await;

                if would_cycle {
                    let message = result
                        .err()
                        .unwrap_or_else(|| panic!("{program} closes a cycle but was accepted"))
                        .to_string();
                    prop_assert!(
                        message.contains("itself")
                            || message.contains("cycle")
                            || message.contains("self"),
                        "{program} closes a cycle but was refused for another reason: {message}",
                    );
                } else {
                    prop_assert!(
                        result.is_ok(),
                        "{program} closes no cycle but was refused: {:?}",
                        result.err().map(|e| e.to_string()),
                    );
                    defined.insert(index);
                }
            }
            Ok(())
        })?;
    }
}
