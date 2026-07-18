//! Content-addressed word identity (Section 8.6).
//!
//! Each user word has a stable identity derived from its content, not its name:
//! `id = H(normalize(body) ⊕ { id(dep) })`. Every reference inside a body is
//! replaced by the identity of its target before hashing, so identity is
//! independent of which names happen to be in scope. A recursive group (a
//! strongly connected component of the dependency graph) is hashed as a unit
//! with its members referenced by position within the component.
//!
//! The digest below is a deterministic 256-bit-class polynomial hash built on
//! the same big-integer primitives `hash.rs` already uses. Its exact byte
//! encoding is an implementation contract (Section 2.1/2.3) and may be replaced
//! by a standard cryptographic hash without changing identity semantics
//! elsewhere.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use num_bigint::BigInt;

use crate::core_word_aliases::canonicalize_core_word_name;
use crate::types::fraction::Fraction;
use crate::types::{Token, WordDefinition};

use super::Interpreter;

lazy_static::lazy_static! {
    // Two distinct ~127-bit moduli; concatenating the two residues yields a
    // 256-bit-class digest (32 hex chars each).
    static ref ID_PRIME_A: BigInt =
        BigInt::parse_bytes(b"170141183460469231731687303715884105727", 10).unwrap();
    static ref ID_PRIME_B: BigInt =
        BigInt::parse_bytes(b"170141183460469231731687303715884105619", 10).unwrap();
    static ref ID_BASE: BigInt = BigInt::from(257u32);
}

fn poly_hash(bytes: &[u8], modulus: &BigInt) -> BigInt {
    let mut acc = BigInt::from(1u32);
    for &b in bytes {
        acc = (&acc * &*ID_BASE + BigInt::from(b as u32 + 1)) % modulus;
    }
    acc
}

/// Deterministic content digest. Returns a `#`-prefixed 64-hex-char string.
/// Reused for execution-receipt source and result identity (Phase 6) so those
/// identities share the same content-hash family as word identities (§8.6).
pub(crate) fn content_digest(bytes: &[u8]) -> String {
    let a = poly_hash(bytes, &ID_PRIME_A);
    let b = poly_hash(bytes, &ID_PRIME_B);
    format!("#{:0>32}{:0>32}", a.to_str_radix(16), b.to_str_radix(16))
}

/// Canonical content key for a word body, independent of references' identities
/// (references are keyed by their canonical spelling). Two textually identical
/// bodies — e.g. the same definition exported and re-imported into another
/// dictionary — produce the same key and can share one stored body (§8.6
/// content store). Exact-rational numbers are normalized so `1` and `1/1` agree.
pub(crate) fn body_content_key(lines: &[crate::types::ExecutionLine]) -> String {
    let mut bytes = Vec::new();
    for line in lines {
        bytes.push(0x1d);
        for tok in line.body_tokens.iter() {
            bytes.push(0x1f);
            match tok {
                Token::Number(n) => match Fraction::from_str(n) {
                    Ok(frac) => {
                        let (num, den) = frac.to_bigint_pair();
                        bytes.push(b'N');
                        bytes.extend_from_slice(num.to_str_radix(16).as_bytes());
                        bytes.push(b'/');
                        bytes.extend_from_slice(den.to_str_radix(16).as_bytes());
                    }
                    Err(_) => {
                        bytes.push(b'n');
                        bytes.extend_from_slice(n.as_bytes());
                    }
                },
                Token::String(s) => {
                    bytes.push(b'S');
                    bytes.extend_from_slice(s.as_bytes());
                }
                Token::Symbol(s) => {
                    bytes.push(b'Y');
                    bytes.extend_from_slice(canonicalize_core_word_name(s).as_bytes());
                }
                Token::VectorStart => bytes.push(b'['),
                Token::VectorEnd => bytes.push(b']'),
                Token::BlockStart => bytes.push(b'{'),
                Token::BlockEnd => bytes.push(b'}'),
                Token::Pipeline => bytes.push(b'~'),
                Token::NilCoalesce => bytes.push(b'^'),
                Token::CondClauseSep => bytes.push(b'|'),
                Token::LineBreak => bytes.push(b'\n'),
            }
        }
    }
    content_digest(&bytes)
}

/// One canonical element of a word body.
enum Atom {
    /// A reference to another user word (a dependency), by fully-qualified name.
    Ref(String),
    /// Any other token, already serialized to canonical bytes.
    Raw(Vec<u8>),
}

fn structural_atom(tag: u8) -> Atom {
    Atom::Raw(vec![tag])
}

fn number_atom(n: &str) -> Atom {
    match Fraction::from_str(n) {
        Ok(frac) => {
            let (num, den) = frac.to_bigint_pair();
            let mut b = vec![b'N'];
            b.extend_from_slice(num.to_str_radix(16).as_bytes());
            b.push(b'/');
            b.extend_from_slice(den.to_str_radix(16).as_bytes());
            Atom::Raw(b)
        }
        // Unparseable numeric literal: fall back to the raw spelling so the
        // shape is still total and deterministic.
        Err(_) => {
            let mut b = vec![b'n'];
            b.extend_from_slice(n.as_bytes());
            Atom::Raw(b)
        }
    }
}

/// Serialize a body shape to bytes, encoding each user-word reference with the
/// supplied closure (which differs between the pre-ordering pass and the final
/// pass for recursive components).
fn serialize_shape(atoms: &[Atom], encode_ref: &dyn Fn(&str) -> Vec<u8>) -> Vec<u8> {
    let mut out = Vec::new();
    for atom in atoms {
        match atom {
            Atom::Raw(b) => {
                out.push(0x1f);
                out.extend_from_slice(b);
            }
            Atom::Ref(target) => {
                out.push(0x1e);
                out.extend_from_slice(&encode_ref(target));
            }
        }
    }
    out
}

/// Tarjan's SCC algorithm. Returns components in reverse topological order
/// (dependencies before the words that depend on them), with deterministic
/// node iteration so the result does not depend on hash-map ordering.
fn tarjan_sccs(nodes: &HashSet<String>, adj: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    struct State<'a> {
        adj: &'a HashMap<String, Vec<String>>,
        index: usize,
        indices: HashMap<String, usize>,
        lowlink: HashMap<String, usize>,
        on_stack: HashSet<String>,
        stack: Vec<String>,
        out: Vec<Vec<String>>,
    }

    fn strongconnect(st: &mut State, v: &str) {
        st.indices.insert(v.to_string(), st.index);
        st.lowlink.insert(v.to_string(), st.index);
        st.index += 1;
        st.stack.push(v.to_string());
        st.on_stack.insert(v.to_string());

        if let Some(neighbors) = st.adj.get(v) {
            for w in neighbors.clone() {
                if !st.indices.contains_key(&w) {
                    strongconnect(st, &w);
                    let lw = st.lowlink[&w];
                    let lv = st.lowlink[v];
                    st.lowlink.insert(v.to_string(), lv.min(lw));
                } else if st.on_stack.contains(&w) {
                    let iw = st.indices[&w];
                    let lv = st.lowlink[v];
                    st.lowlink.insert(v.to_string(), lv.min(iw));
                }
            }
        }

        if st.lowlink[v] == st.indices[v] {
            let mut comp = Vec::new();
            loop {
                let w = st.stack.pop().unwrap();
                st.on_stack.remove(&w);
                let is_root = w == v;
                comp.push(w);
                if is_root {
                    break;
                }
            }
            st.out.push(comp);
        }
    }

    let mut st = State {
        adj,
        index: 0,
        indices: HashMap::new(),
        lowlink: HashMap::new(),
        on_stack: HashSet::new(),
        stack: Vec::new(),
        out: Vec::new(),
    };

    let mut sorted_nodes: Vec<&String> = nodes.iter().collect();
    sorted_nodes.sort();
    for v in sorted_nodes {
        if !st.indices.contains_key(v) {
            strongconnect(&mut st, v);
        }
    }
    st.out
}

impl Interpreter {
    /// Content identity of a user word, if it has been computed.
    pub(crate) fn word_identity(&self, fq_name: &str) -> Option<&String> {
        self.word_identities.get(fq_name)
    }

    /// Reclaim content-store bodies no longer referenced by any definition.
    /// An entry whose only remaining strong reference is the store itself
    /// (`strong_count == 1`) is orphaned — deleted or replaced by a redefine —
    /// and is dropped. A body still shared by one or more live definitions has
    /// a higher count and is kept. Run only at quiescent points (after a
    /// definition, deletion, or dependency rebuild), where no transient body
    /// clones are in flight; deferred during bulk operations like the rest of
    /// the content-store maintenance.
    pub(crate) fn gc_body_store(&mut self) {
        if self.defer_identity_recompute {
            return;
        }
        self.body_store
            .retain(|_, body| std::sync::Arc::strong_count(body) > 1);
    }

    fn build_word_shape(&self, def: &WordDefinition, user_set: &HashSet<String>) -> Vec<Atom> {
        let mut atoms = Vec::new();
        for line in def.lines.iter() {
            atoms.push(structural_atom(b'\n'));
            for tok in line.body_tokens.iter() {
                let atom = match tok {
                    Token::Number(n) => number_atom(n),
                    Token::String(s) => {
                        let mut b = vec![b'S'];
                        b.extend_from_slice(s.as_bytes());
                        Atom::Raw(b)
                    }
                    Token::Symbol(s) => {
                        let canon = canonicalize_core_word_name(s);
                        match self.resolve_word_entry_readonly(&canon) {
                            Some((resolved, rdef)) => {
                                if def.dependencies.contains(&resolved) {
                                    // A user-word dependency fixed at definition time:
                                    // encode the recorded target rather than treating a
                                    // later same-named word as a fresh capture.
                                    Atom::Ref(resolved)
                                } else if rdef.is_builtin {
                                    // Core / module word: stable global vocabulary,
                                    // encoded by its canonical resolved name.
                                    let mut b = vec![b'G'];
                                    b.extend_from_slice(resolved.as_bytes());
                                    Atom::Raw(b)
                                } else if user_set.contains(&resolved) {
                                    // A user word not recorded in this definition's
                                    // dependency set was not resolved when the word was
                                    // authored. Keep it as a free symbol so adding an
                                    // unrelated word cannot recapture existing content.
                                    let mut b = vec![b'F'];
                                    b.extend_from_slice(canon.as_bytes());
                                    Atom::Raw(b)
                                } else {
                                    Atom::Ref(resolved)
                                }
                            }
                            // Free / unresolved symbol: encoded by canonical name.
                            None => {
                                let mut b = vec![b'F'];
                                b.extend_from_slice(canon.as_bytes());
                                Atom::Raw(b)
                            }
                        }
                    }
                    Token::VectorStart => structural_atom(b'['),
                    Token::VectorEnd => structural_atom(b']'),
                    Token::BlockStart => structural_atom(b'{'),
                    Token::BlockEnd => structural_atom(b'}'),
                    Token::Pipeline => structural_atom(b'~'),
                    Token::NilCoalesce => structural_atom(b'^'),
                    Token::CondClauseSep => structural_atom(b'|'),
                    Token::LineBreak => structural_atom(b'\n'),
                };
                atoms.push(atom);
            }
        }
        atoms
    }

    /// Recompute content identities for every user word (Section 8.6). Cheap
    /// enough to run after any change to the user-word graph.
    pub(crate) fn recompute_word_identities(&mut self) {
        // Deferred during bulk operations; the batch recomputes once at the end.
        if self.defer_identity_recompute {
            return;
        }

        // 1. Snapshot all user words: (fully-qualified name, dictionary, def).
        let mut words: Vec<(String, String, Arc<WordDefinition>)> = Vec::new();
        for (dict_name, dict) in &self.user_dictionaries {
            for (name, def) in &dict.words {
                words.push((
                    format!("{}@{}", dict_name, name),
                    dict_name.clone(),
                    def.clone(),
                ));
            }
        }
        let user_set: HashSet<String> = words.iter().map(|(fq, _, _)| fq.clone()).collect();
        let reg_order: HashMap<String, u64> = words
            .iter()
            .map(|(fq, _, def)| (fq.clone(), def.registration_order))
            .collect();

        // 2. Canonical shape of each body; references to other user words are
        //    captured as Atom::Ref and resolved through the owning dictionary.
        let prev_ctx = self.owning_dictionary_context.take();
        let mut shapes: HashMap<String, Vec<Atom>> = HashMap::new();
        for (fq, dict, def) in &words {
            self.owning_dictionary_context = Some(dict.clone());
            let atoms = self.build_word_shape(def, &user_set);
            shapes.insert(fq.clone(), atoms);
        }
        self.owning_dictionary_context = prev_ctx;

        // 3. Dependency graph restricted to user words.
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for (fq, atoms) in &shapes {
            let mut seen = HashSet::new();
            let mut neighbors = Vec::new();
            for atom in atoms {
                if let Atom::Ref(target) = atom {
                    if user_set.contains(target) && seen.insert(target.clone()) {
                        neighbors.push(target.clone());
                    }
                }
            }
            adj.insert(fq.clone(), neighbors);
        }

        // 4. SCCs in reverse topological order (dependencies first).
        let sccs = tarjan_sccs(&user_set, &adj);

        // 5. Assign identities component by component.
        let mut ids: HashMap<String, String> = HashMap::new();
        for scc in &sccs {
            let scc_set: HashSet<String> = scc.iter().cloned().collect();

            // Pre-ordering digest: intra-SCC references blanked (so the ordering
            // does not depend on which member calls which), extra-SCC references
            // by their already-computed identity.
            let mut pre: HashMap<String, String> = HashMap::new();
            for m in scc {
                let bytes = serialize_shape(&shapes[m], &|target: &str| {
                    if scc_set.contains(target) {
                        b"C?".to_vec()
                    } else {
                        let mut v = vec![b'I'];
                        if let Some(id) = ids.get(target) {
                            v.extend_from_slice(id.as_bytes());
                        }
                        v
                    }
                });
                pre.insert(m.clone(), content_digest(&bytes));
            }

            let mut ordered: Vec<String> = scc.clone();
            ordered.sort_by(|a, b| {
                pre[a]
                    .cmp(&pre[b])
                    .then_with(|| reg_order.get(a).cmp(&reg_order.get(b)))
                    .then_with(|| a.cmp(b))
            });
            let mut position: HashMap<String, usize> = HashMap::new();
            for (i, m) in ordered.iter().enumerate() {
                position.insert(m.clone(), i);
            }

            // Final shape bytes: intra-SCC references by position, extra-SCC by id.
            let mut final_bytes: HashMap<String, Vec<u8>> = HashMap::new();
            for m in scc {
                let bytes = serialize_shape(&shapes[m], &|target: &str| {
                    if scc_set.contains(target) {
                        let mut v = vec![b'P'];
                        v.extend_from_slice(position[target].to_string().as_bytes());
                        v
                    } else {
                        let mut v = vec![b'I'];
                        if let Some(id) = ids.get(target) {
                            v.extend_from_slice(id.as_bytes());
                        }
                        v
                    }
                });
                final_bytes.insert(m.clone(), bytes);
            }

            // Combined digest over members in canonical position order.
            let mut combined_input = Vec::new();
            for (i, m) in ordered.iter().enumerate() {
                combined_input.extend_from_slice(i.to_string().as_bytes());
                combined_input.push(0x1d);
                combined_input.extend_from_slice(&final_bytes[m]);
                combined_input.push(0x1c);
            }
            let combined = content_digest(&combined_input);

            for m in scc {
                let pos = position[m];
                let mut id_bytes = combined.clone().into_bytes();
                id_bytes.push(b':');
                id_bytes.extend_from_slice(pos.to_string().as_bytes());
                ids.insert(m.clone(), content_digest(&id_bytes));
            }
        }

        self.word_identities = ids;
    }
}
