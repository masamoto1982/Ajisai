#!/usr/bin/env python3
"""
Ajisai reproduction — an independent re-implementation built *only* from
SPECIFICATION.html (the canonical source), in Python.

Purpose: serve as a spec-faithful oracle. Running the same programs through
this interpreter and through the original Rust `ajisai` CLI and diffing the
results surfaces places where the original implementation diverges from the
specification, or where the specification is under-specified.

Scope: the host-independent Core Profile (Section "Portability Profiles") plus
the parts of MATH needed for the spec's own examples. No host effects, no child
runtime, no audio/serial/JSON.

Design choices follow the spec literally:
  * Numbers are exact reals. Rationals use Python's Fraction (exact).
    sqrt(non-negative rational) is carried as a lazy continued fraction.
  * Display: RawNumber -> reduced "num/den"; booleans -> TRUE/FALSE/UNKNOWN;
    NIL -> NIL; Text on the Stack -> wrapped in single quotes.
  * Modifiers TOP/STAK x EAT/KEEP per Sections 5-6.
  * Three-valued (Kleene K3) logic and NIL bubble rule per Sections 4.5, 7.4, 7.5.
"""
from __future__ import annotations
from fractions import Fraction
from dataclasses import dataclass, field
from typing import Optional, List, Any
import sys, json

# Native recursion headroom. This reference interpreter evaluates user-word
# recursion by plain Python recursion (it does not implement the guarded
# tail-call backward jump of SPEC Section 8.4), so its recursion-depth guard
# *is* Python's recursion limit — an implementation-defined depth, which the
# spec permits. Raise it far enough that the conformance suite's guarded
# tail-recursion case (depth 1000, ~7 Python frames per Ajisai level) fits;
# unbounded recursion still trips RecursionError, reported as
# recursionLimitExceeded (SPEC Section 11.1).
sys.setrecursionlimit(20000)

# --------------------------------------------------------------------------
# Value model (Section 4)
# --------------------------------------------------------------------------

class Bool:
    """A Boolean truth value — a value kind distinct from a number (Section 4.1)."""
    __slots__ = ("v",)
    def __init__(self, v): self.v = v          # True / False / "U"
    def __repr__(self): return f"Bool({self.v})"

TRUE = Bool(True); FALSE = Bool(False); UNKNOWN = Bool("U")

@dataclass
class Nil:
    """Operational absence (Section 4.5). Carries a diagnostic reason/origin."""
    reason: Optional[str] = None
    origin: str = "literal"
    agreedPrefix: Optional[int] = None

@dataclass
class Vec:
    items: List[Any]

@dataclass
class Str:
    s: str

@dataclass
class Rec:
    """A Record (Section 4.4): key-indexed [key, value] pairs, as produced by
    JSON@PARSE. There is no record-specific interpretation role (Section 12.2),
    so it renders in its raw structural form, exactly like a vector of pairs."""
    items: List[Any]                 # each a Vec([Str(key), value])

@dataclass
class Block:
    """A CodeBlock: a list of source lines (Section 3.4, 4.6)."""
    lines: List[List[str]]            # each line is a token list

@dataclass
class Rational:
    f: Fraction

@dataclass
class Interval:
    """A sound rational interval [lo, hi] (MATH@INTERVAL / MATH@SQRT-EPS).
    Displayed as `[lo, hi]` with both endpoints in n/d form, mirroring the
    Rust Interval Display."""
    lo: Fraction
    hi: Fraction

@dataclass
class Sqrt:
    """Lazy sqrt of a non-negative rational (Section 4.2.2 AlgebraicSqrt)."""
    radicand: Fraction               # value is sqrt(radicand)

@dataclass
class Alg:
    """A non-rational, non-single-root element of the admitted domain D
    (Section 4.2.7) in multiquadratic normal form: a map from square-root
    monomial (an integer product of pairwise-coprime basis elements; 1 keys
    the rational part) to its non-zero rational coefficient. Values that
    demote to Rational or Sqrt never stay in this class (see _alg_demote)."""
    terms: dict                      # {int monomial: Fraction coeff}

class AjisaiError(Exception):
    def __init__(self, kind, msg=""):
        super().__init__(msg or kind); self.kind = kind

# --------------------------------------------------------------------------
# Continued fractions (Section 4.2)
# --------------------------------------------------------------------------

def rcf_terms_rational(num: int, den: int):
    """Regular continued fraction partial quotients of num/den (floor based)."""
    terms = []
    while den != 0:
        q = num // den
        terms.append(q)
        num, den = den, num - q * den
    # canonical: drop trailing 1 (Section 4.2.1)
    if len(terms) >= 2 and terms[-1] == 1:
        terms.pop(); terms[-1] += 1
    return terms

def sqrt_cf_terms(radicand: Fraction, limit: int):
    """Lazy RCF of sqrt(radicand) up to `limit` terms (periodic; Section 4.2.2)."""
    # General algorithm for sqrt of a rational p/q -> sqrt(p*q)/q
    p, q = radicand.numerator, radicand.denominator
    N = p * q                       # sqrt(N)/q
    import math
    a0floor = math.isqrt(N)
    # value = sqrt(N)/q ; expand directly via interval/Möbius-free numeric-free method:
    # Use the standard algorithm on x = (sqrt(N))/q by tracking (m,d): x=(sqrt(N)+m)/d
    # but our x already has denominator q and offset 0. Transform: a = floor(sqrt(N)/q)
    terms = []
    # represent x_k = (sqrt(N) + m)/d  ; start x_0 = (sqrt(N) + 0)/q
    if N == a0floor * a0floor:
        # perfect square -> rational
        return rcf_terms_rational(a0floor, q)
    m, d = 0, q
    a0 = (a0floor + m) // d
    terms.append(a0)
    seen = {}
    while len(terms) < limit:
        m = d * terms[-1] - m
        d = (N - m * m) // d
        if d == 0:
            break
        a = (a0floor + m) // d
        terms.append(a)
    return terms

# 32 terms, matching the production CF display budget (the conformance
# suite pins the exact term count of the truncated lazy rendering).
CF_DISPLAY_BUDGET = 32

def cf_terms(val, limit=CF_DISPLAY_BUDGET):
    if isinstance(val, Rational):
        return rcf_terms_rational(val.f.numerator, val.f.denominator), True
    if isinstance(val, Sqrt):
        return sqrt_cf_terms(val.radicand, limit), False
    raise AjisaiError("structureError", "not a scalar")

def cf_nested_display(val):
    terms, finite = cf_terms(val)
    if finite:
        s = ""
        for t in reversed(terms):
            s = f"( {t}{(' ' + s) if s else ' '})"
        # build properly
        return build_nested(terms, truncated=False)
    else:
        return build_nested(terms, truncated=True)

def build_nested(terms, truncated):
    # ( a0 ( a1 ( a2 ) ) ) ; a truncated lazy CF renders its innermost term
    # as "( aN ...)" — the ")" of "...)" is that term's own closer, matching
    # the production rendering exactly.
    acc = ""
    for t in reversed(terms):
        if acc == "":
            acc = f"( {t} ...)" if truncated else f"( {t} )"
        else:
            acc = f"( {t} {acc} )"
    return acc

# --------------------------------------------------------------------------
# Numeric helpers — exact value semantics
# --------------------------------------------------------------------------

def as_fraction(val):
    """Return exact Fraction if val is a finite rational scalar, else None."""
    if isinstance(val, Rational):
        return val.f
    return None

def num_value(val):
    """Best exact handle for arithmetic: Fraction or ('sqrt', radicand)."""
    if isinstance(val, Rational): return val.f
    if isinstance(val, Sqrt): return val
    return None

def is_scalar(v): return isinstance(v, (Rational, Sqrt, Alg))

# --------------------------------------------------------------------------
# Multiquadratic normal form over the admitted domain D (Section 4.2.7)
#
# Mirror of rust/src/types/multiquadratic.rs: every value the current
# Coreword set constructs lies in Q(sqrt(d1), sqrt(d2), ...), whose elements
# have a unique normal form sum(c_d * sqrt(d)). Equality and order over D
# are decided exactly and totally on that form (Section 7.4), which is why
# the six relations never return UNKNOWN over D. Radicands are refined into
# a GCD-free basis (pairwise coprime, none a perfect square) instead of
# factored into primes, so no unbounded factorization is ever needed.
# --------------------------------------------------------------------------

def _gcd_free_basis(rads):
    """Pairwise-coprime, non-perfect-square integers covering every radicand."""
    elems = sorted({int(r) for r in rads if r > 1})
    while True:
        found = None
        for i in range(len(elems)):
            for j in range(i + 1, len(elems)):
                if _math.gcd(elems[i], elems[j]) > 1:
                    found = (i, j)
                    break
            if found:
                break
        if not found:
            break
        i, j = found
        a, b = elems[i], elems[j]
        g = _math.gcd(a, b)
        rest = [e for k, e in enumerate(elems) if k not in (i, j)]
        for part in (g, a // g, b // g):
            if part > 1 and part not in rest:
                rest.append(part)
        elems = sorted(rest)
    out = []
    for e in elems:
        while _math.isqrt(e) ** 2 == e:
            e = _math.isqrt(e)
        out.append(e)
    return sorted(out)

def _decompose_sqrt(n, basis):
    """sqrt(n) = outside * sqrt(monomial) over the basis."""
    rest, outside, mono = int(n), 1, 1
    for b in basis:
        e = 0
        while rest % b == 0:
            rest //= b
            e += 1
        outside *= b ** (e // 2)
        if e % 2:
            mono *= b
    if rest != 1:
        raise AjisaiError("structureError", "radicand outside basis")
    return outside, mono

def _collect_rads(v, out):
    if isinstance(v, Sqrt):
        out.append(v.radicand.numerator * v.radicand.denominator)
    elif isinstance(v, Alg):
        out.extend(m for m in v.terms if m > 1)

def _terms_of(v, basis):
    if isinstance(v, Rational):
        return {1: v.f} if v.f != 0 else {}
    if isinstance(v, Sqrt):
        # sqrt(p/q) = sqrt(p*q)/q
        p, q = v.radicand.numerator, v.radicand.denominator
        outside, mono = _decompose_sqrt(p * q, basis)
        return {mono: Fraction(outside, q)}
    out = {}
    for m, c in v.terms.items():
        if m == 1:
            _t_addterm(out, 1, c)
        else:
            outside, mono = _decompose_sqrt(m, basis)
            _t_addterm(out, mono, c * outside)
    return out

def _t_addterm(t, mono, coeff):
    s = t.get(mono, Fraction(0)) + coeff
    if s == 0:
        t.pop(mono, None)
    else:
        t[mono] = s

def _t_add(ta, tb):
    out = dict(ta)
    for m, c in tb.items():
        _t_addterm(out, m, c)
    return out

def _t_sub(ta, tb):
    out = dict(ta)
    for m, c in tb.items():
        _t_addterm(out, m, -c)
    return out

def _t_mul(ta, tb):
    # sqrt(m1)*sqrt(m2) = g*sqrt(m1*m2/g^2) with g = gcd(m1, m2), again a
    # subset-product monomial because the basis is pairwise coprime.
    out = {}
    for m1, c1 in ta.items():
        for m2, c2 in tb.items():
            g = _math.gcd(m1, m2)
            _t_addterm(out, (m1 // g) * (m2 // g), c1 * c2 * g)
    return out

def _t_inverse(terms, basis):
    """Multiplicative inverse by recursive conjugation; raises
    ZeroDivisionError on the zero value. Splitting y = u + v on a basis
    element b (v = terms whose monomial contains b), y*(u-v) = u^2 - v^2
    has no b in its support, so each step eliminates one basis element."""
    if not terms:
        raise ZeroDivisionError
    if len(terms) == 1 and 1 in terms:
        return {1: 1 / terms[1]}
    b = next(bb for bb in basis if any(m % bb == 0 for m in terms))
    with_b = {m: c for m, c in terms.items() if m % b == 0}
    without_b = {m: c for m, c in terms.items() if m % b != 0}
    conj = _t_sub(without_b, with_b)
    prod = _t_mul(terms, conj)
    return _t_mul(conj, _t_inverse(prod, basis))

def _t_bounds(terms, bits):
    """Rational enclosure at 2^-bits per-monomial precision (no floats)."""
    lo = Fraction(0)
    hi = Fraction(0)
    scale = 1 << bits
    for m, c in terms.items():
        if m == 1:
            lo += c
            hi += c
            continue
        s = _math.isqrt(m << (2 * bits))
        mlo, mhi = Fraction(s, scale), Fraction(s + 1, scale)
        if c >= 0:
            lo += c * mlo
            hi += c * mhi
        else:
            lo += c * mhi
            hi += c * mlo
    return lo, hi

def _t_sign(terms):
    """Exact sign. A non-empty normal form is non-zero (square roots of
    distinct subset products are linearly independent over Q), so interval
    refinement always terminates."""
    if not terms:
        return 0
    if len(terms) == 1:
        c = next(iter(terms.values()))
        return 1 if c > 0 else -1
    bits = 8
    while True:
        lo, hi = _t_bounds(terms, bits)
        if lo > 0:
            return 1
        if hi < 0:
            return -1
        bits *= 2

def _t_floor(terms):
    if all(m == 1 for m in terms):
        return _math.floor(terms.get(1, Fraction(0)))
    bits = 8
    while True:
        lo, hi = _t_bounds(terms, bits)
        fl, fh = _math.floor(lo), _math.floor(hi)
        if fl == fh:
            return fl
        bits *= 2

def _alg_demote(terms):
    """Re-tag a normal form as the canonical value kind: Rational for the
    rational case, Sqrt for a single positive-coefficient root, Alg else."""
    terms = {m: c for m, c in terms.items() if c != 0}
    if not terms:
        return Rational(Fraction(0))
    if len(terms) == 1:
        (m, c), = terms.items()
        if m == 1:
            return Rational(c)
        if c > 0:
            return Sqrt(c * c * m)  # c*sqrt(m) = sqrt(c^2 * m)
    return Alg(terms)

def alg_binop(name, a, b):
    """Field arithmetic over D for at least one non-rational scalar operand.
    Raises ZeroDivisionError for DIV by the exact zero."""
    rads = []
    _collect_rads(a, rads)
    _collect_rads(b, rads)
    basis = _gcd_free_basis(rads)
    ta, tb = _terms_of(a, basis), _terms_of(b, basis)
    if name == "ADD":
        res = _t_add(ta, tb)
    elif name == "SUB":
        res = _t_sub(ta, tb)
    elif name == "MUL":
        res = _t_mul(ta, tb)
    elif name == "DIV":
        res = _t_mul(ta, _t_inverse(tb, basis))
    else:
        raise AjisaiError("structureError", f"{name} over algebraic operands")
    return _alg_demote(res)

def alg_rcf_terms(v, limit):
    """Regular CF terms of an (irrational) Alg value via exact field
    arithmetic: a_k = floor(x_k), x_{k+1} = 1/(x_k - a_k)."""
    rads = []
    _collect_rads(v, rads)
    basis = _gcd_free_basis(rads)
    cur = _terms_of(v, basis)
    out = []
    for _ in range(limit):
        a = _t_floor(cur)
        out.append(a)
        _t_addterm(cur, 1, Fraction(-a))
        if not cur:
            break  # defensive: an Alg value is irrational by demotion
        cur = _t_inverse(cur, basis)
    return out

def value_rcf_terms(v, limit):
    """(terms, finite) of any admitted-domain scalar's regular CF."""
    if isinstance(v, Rational):
        return rcf_terms_rational(v.f.numerator, v.f.denominator), True
    if isinstance(v, Sqrt):
        return sqrt_cf_terms(v.radicand, limit), False
    return alg_rcf_terms(v, limit), False

# --------------------------------------------------------------------------
# Tokenizer (Section 3)
# --------------------------------------------------------------------------

MOD_SUGAR = {".": "TOP", "..": "STAK", ",": "EAT", ",,": "KEEP"}

def tokenize_line(line: str):
    toks = []
    i = 0; n = len(line)
    while i < n:
        c = line[i]
        if c in " \t":
            i += 1; continue
        if c == "#":
            break
        if c in "()":
            raise AjisaiError("tokenizerError", "reserved marker ( or )")
        if c == "'":
            # string: ends at last ' before a token boundary (Section 3.3)
            j = i + 1
            last_quote = None
            while j < n:
                if line[j] == "'":
                    # boundary after this quote?
                    nxt = line[j+1] if j+1 < n else None
                    if nxt is None or nxt in " \t[]{}#=|":
                        last_quote = j; break
                j += 1
            if last_quote is None:
                # unterminated: take rest
                last_quote = n
                toks.append(("str", line[i+1:last_quote]))
                i = n; continue
            toks.append(("str", line[i+1:last_quote]))
            i = last_quote + 1; continue
        if c in "[]{}|":
            toks.append(("sym", c)); i += 1; continue
        # read a run of non-space, non-special chars
        j = i
        while j < n and line[j] not in " \t'[]{}|#":
            j += 1
        word = line[i:j]
        i = j
        toks.append(("word", word))
    return toks

# --------------------------------------------------------------------------
# Parser for vectors / blocks within a token stream
# --------------------------------------------------------------------------

def number_of(word: str):
    """Parse a numeric literal (Section 3.2); return Rational/Sqrt or None."""
    w = word
    sign = 1
    # scientific
    import re
    if re.fullmatch(r"[+-]?\d+", w):
        return Rational(Fraction(int(w)))
    m = re.fullmatch(r"([+-]?\d+)/(\d+)", w)
    if m:
        den = int(m.group(2))
        if den == 0:
            raise AjisaiError("malformedLiteral", "zero denominator")
        return Rational(Fraction(int(m.group(1)), den))
    # decimal (at most one side empty) — but bare '.' is the TOP modifier, excluded earlier
    if re.fullmatch(r"[+-]?(\d+\.\d*|\.\d+|\d+\.)", w):
        return Rational(Fraction(w))
    m = re.fullmatch(r"([+-]?(?:\d+\.?\d*|\.\d+))[eE]([+-]?\d+)", w)
    if m:
        mant = Fraction(m.group(1)); ex = int(m.group(2))
        return Rational(mant * Fraction(10) ** ex)
    return None

# --------------------------------------------------------------------------
# Structured token reader: numbers, strings, vectors, blocks, words/modifiers
# --------------------------------------------------------------------------

ALIAS = {
    "+": "ADD", "-": "SUB", "*": "MUL", "/": "DIV", "%": "MOD",
    "=": "EQ", "<>": "NEQ", "<": "LT", "<=": "LTE", ">": "GT", ">=": "GTE",
    "&": "AND", "!": "FORC", "?": "LOOKUP", "~": "FLOW", "^": "VENT",
    ".": "TOP", "..": "STAK", ",": "EAT", ",,": "KEEP",
}

def split_modifier_prefix(word: str):
    """Per Section 6/3.9: ';' -> '.', ';;' -> '.. ,,'; and a leading run of
    modifier chars (. .. , ,,) attached to a word.  Returns (mods, rest)."""
    mods = []
    # whole-token modifier sugar
    if word == ";": return (["TOP", "EAT"], None)
    if word == ";;": return (["STAK", "KEEP"], None)
    return (None, word)

class Reader:
    """Reads a flat token list into evaluable items, handling [] {} grouping."""
    def __init__(self, toks): self.toks = toks; self.i = 0
    def read_all(self):
        out = []
        while self.i < len(self.toks):
            out.append(self.read_one())
        return out

# Rather than a full AST, the evaluator consumes the token list line by line.

# --------------------------------------------------------------------------
# Interpreter
# --------------------------------------------------------------------------

class Interp:
    def __init__(self):
        self.stack: List[Any] = []
        self.user_words = {}      # name -> Block
        self.imported = set()     # module names imported
        self.visible = set()      # module word surfaces currently visible (§9.2)
        self.forc = False         # FORC (!) pending: next DEL/DEF skips protection
        self.steps = 0
        self.output = []

    # ---- stack helpers ----
    def push(self, v): self.stack.append(v)
    def pop(self):
        if not self.stack: raise AjisaiError("stackUnderflow")
        return self.stack.pop()
    def need(self, k):
        if len(self.stack) < k: raise AjisaiError("stackUnderflow")

    # ---- run ----
    def run_source(self, src: str):
        for raw in src.split("\n"):
            toks = tokenize_line(raw)
            if toks:
                self.run_tokens(toks)

    def run_block(self, block: Block):
        for line in block.lines:
            if line:
                self.run_tokens(list(line))

    def run_tokens(self, toks):
        idx = 0
        pending_mods = []         # accumulated modifier names
        while idx < len(toks):
            self.steps += 1
            if self.steps > 100000:
                raise AjisaiError("executionLimitExceeded")
            kind, val = toks[idx]
            if kind == "val":
                # A definition-time staged value (PRECOMPUTE, Section 7.7).
                self.push(val); idx += 1; continue
            if kind == "str":
                self.push(Str(val)); idx += 1; continue
            if kind == "sym":
                if val == "[":
                    vec, idx = self.read_vector(toks, idx + 1)
                    self.push(vec); continue
                if val == "{":
                    blk, idx = self.read_block(toks, idx + 1)
                    self.push(blk); continue
                if val == "|":
                    raise AjisaiError("structureError", "stray COND clause separator")
                raise AjisaiError("structureError", f"unexpected {val}")
            # word
            word = val
            # number?
            num = number_of(word)
            if num is not None:
                self.push(num); idx += 1; continue
            # modifier sugar tokens, possibly fused to the next word
            # (';ADD' == '; ADD', ';;ADD' == ';; ADD'); Section 3.9 / 6.
            if word == ";" or word == ";;":
                pending_mods += (["TOP", "EAT"] if word == ";" else ["STAK", "KEEP"])
                idx += 1; continue
            if word.startswith(";;") and len(word) > 2:
                pending_mods += ["STAK", "KEEP"]; word = word[2:]
            elif word.startswith(";") and len(word) > 1:
                pending_mods += ["TOP", "EAT"]; word = word[1:]
            if word == "^":
                # VENT (^) is a lazy NIL-coalescing control directive, not a
                # stack word (Section 6.4). It pops the top; if it is non-NIL it
                # is kept and the *following* source unit (one token, or one
                # balanced [ ] / { } group) is skipped unevaluated; if it is NIL
                # the NIL is discarded and the following unit is evaluated as the
                # fallback. Modifiers on ^ are ignored.
                pending_mods = []
                self.need(1)
                top = self.stack.pop()
                if not isinstance(top, Nil):
                    self.push(top)
                    idx = self._skip_one_unit(toks, idx + 1)
                    continue
                idx += 1
                continue
            canon = ALIAS.get(word, word).upper()
            if canon in ("TOP", "STAK", "EAT", "KEEP"):
                pending_mods.append(canon); idx += 1; continue
            # an actual operation word
            self.exec_word(canon, pending_mods)
            pending_mods = []
            idx += 1
        # trailing modifiers with no word: ignore (no-op markers)

    def _skip_one_unit(self, toks, j):
        """Return the index just past one source unit starting at `j`: a single
        token, or one balanced [ ] / { } group. Used by VENT's non-NIL branch to
        skip the fallback unevaluated (Section 6.4)."""
        if j >= len(toks):
            return j
        kind, val = toks[j]
        if kind == "sym" and val in "[{":
            close = "]" if val == "[" else "}"
            depth = 1
            j += 1
            while j < len(toks) and depth:
                k2, v2 = toks[j]
                if k2 == "sym" and v2 == val:
                    depth += 1
                elif k2 == "sym" and v2 == close:
                    depth -= 1
                j += 1
            return j
        return j + 1

    # ---- structural readers ----
    def read_vector(self, toks, idx):
        items = []
        while idx < len(toks):
            kind, val = toks[idx]
            if kind == "sym" and val == "]":
                return Vec(items), idx + 1
            if kind == "sym" and val == "[":
                sub, idx = self.read_vector(toks, idx + 1); items.append(sub); continue
            if kind == "sym" and val == "{":
                blk, idx = self.read_block(toks, idx + 1); items.append(blk); continue
            if kind == "str":
                items.append(Str(val)); idx += 1; continue
            num = number_of(val)
            if num is not None: items.append(num); idx += 1; continue
            # bareword inside vector: treat as literal? spec vectors hold values.
            # Booleans / NIL words allowed.
            up = val.upper()
            if up == "TRUE": items.append(TRUE)
            elif up == "FALSE": items.append(FALSE)
            elif up == "NIL": items.append(Nil())
            else:
                raise AjisaiError("structureError", f"non-value {val} in vector")
            idx += 1
        raise AjisaiError("structureError", "unbalanced [")

    def read_block(self, toks, idx):
        # collect raw tokens until matching }, preserving nothing about lines
        # (single-line blocks in our test battery). Nested blocks tracked by depth.
        depth = 1
        collected = []
        while idx < len(toks):
            kind, val = toks[idx]
            if kind == "sym" and val == "{": depth += 1
            elif kind == "sym" and val == "}":
                depth -= 1
                if depth == 0:
                    return Block([collected]), idx + 1
            collected.append((kind, val)); idx += 1
        raise AjisaiError("structureError", "unbalanced {")

    # ---- operand gathering for modifiers ----
    def operands(self, mods, arity):
        stak = "STAK" in mods
        keep = "KEEP" in mods
        if stak:
            ops = list(self.stack)
            if not keep: self.stack = []
            return ops, keep
        else:
            self.need(arity)
            ops = self.stack[-arity:] if arity else []
            if not keep:
                for _ in range(arity): self.stack.pop()
            return ops, keep

    # ---- word execution ----
    def exec_word(self, w, mods):
        keep = "KEEP" in mods
        stak = "STAK" in mods
        # user words first? Spec: bare resolves Core first, then modules, then user.
        if w in CORE:
            CORE[w](self, mods); return
        if w in self.user_words:
            self.run_block(self.user_words[w]); return
        # module words resolve only while visible (IMPORT/UNIMPORT, §9.2)
        if w in MODULE_WORDS:
            if w in self.visible:
                MODULE_IMPL[w](self, mods); return
            raise AjisaiError("unknownWord", w)
        if "@" in w:
            # Section 9.2 (observable resolution contract): the qualified form
            # is not a backdoor around a partial import — MODULE@WORD resolves
            # only while the word is in the current import set, exactly like
            # the bare name.
            mod, _, ww = w.partition("@")
            if (ww in MODULE_IMPL and MODULE_WORDS.get(ww) == mod
                    and ww in self.visible):
                MODULE_IMPL[ww](self, mods); return
            raise AjisaiError("unknownWord", w)
        raise AjisaiError("unknownWord", w)

    # ---- STAK operand group (Section 6.1) ----
    def stak_group(self, mods):
        """Pop the leading count, then return the N operands beneath it.
        The count is always consumed; the group is consumed under EAT and
        retained under KEEP."""
        keep = "KEEP" in mods
        self.need(1)
        cnt = self.stack.pop()
        f = as_fraction(cnt)
        if f is None or f.denominator != 1 or f < 0:
            raise AjisaiError("structureError", "STAK needs a leading count")
        n = int(f)
        if len(self.stack) < n:
            raise AjisaiError("stackUnderflow")
        group = self.stack[len(self.stack) - n:] if n else []
        if not keep and n:
            del self.stack[len(self.stack) - n:]
        return list(group)

# --------------------------------------------------------------------------
# Display (Section 12)
# --------------------------------------------------------------------------

def display(v):
    if isinstance(v, Rational):
        return f"{v.f.numerator}/{v.f.denominator}"
    if isinstance(v, Sqrt):
        return build_nested(sqrt_cf_terms(v.radicand, CF_DISPLAY_BUDGET), truncated=True)
    if isinstance(v, Alg):
        return build_nested(alg_rcf_terms(v, CF_DISPLAY_BUDGET), truncated=True)
    if isinstance(v, Bool):
        return {True: "TRUE", False: "FALSE", "U": "UNKNOWN"}[v.v]
    if isinstance(v, Nil):
        return "NIL"
    if isinstance(v, Interval):
        return (f"[{v.lo.numerator}/{v.lo.denominator}, "
                f"{v.hi.numerator}/{v.hi.denominator}]")
    if isinstance(v, Str):
        return "'" + v.s + "'"
    if isinstance(v, (Vec, Rec)):
        return "[ " + " ".join(display(x) for x in v.items) + " ]" if v.items else "[ ]"
    if isinstance(v, Block):
        return "{ ... }"
    return str(v)

def output_render(v):
    # Output boundary (Section 7.9): top-level Text loses quotes
    if isinstance(v, Str):
        return v.s
    return display(v)

# --------------------------------------------------------------------------
# Core word implementations
# --------------------------------------------------------------------------

def leftmost_nil_reason(ops):
    for o in ops:
        if isinstance(o, Nil) and o.reason:
            return o.reason
    return None

def _arith_scalar_pair(name, fn, a, b):
    """Exact arithmetic on a single scalar (or NIL) operand pair; returns a
    value (Rational / Sqrt / Nil) or raises structureError."""
    # NIL passthrough (Section 4.5.1 / 7.12)
    if isinstance(a, Nil) or isinstance(b, Nil):
        return Nil(reason=leftmost_nil_reason([a, b]), origin="nilPropagation")
    fa, fb = as_fraction(a), as_fraction(b)
    if fa is None or fb is None:
        # Admitted-domain algebraic arithmetic (Section 4.2.7): D is closed
        # under + - * /, so any mix of Rational/Sqrt/Alg operands stays in
        # normal form (this subsumes the historical special cases
        # sqrt x - sqrt x = 0, sqrt x * sqrt x = x, sqrt x + sqrt x = sqrt 4x).
        if is_scalar(a) and is_scalar(b):
            return alg_binop(name, a, b)
        raise AjisaiError("structureError", f"{name} needs numbers")
    try:
        return Rational(fn(fa, fb))
    except ZeroDivisionError:
        return Nil(reason="divisionByZero", origin="nilPropagation")


def _arith_broadcast(name, fn, a, b):
    """Elementwise broadcast of a binary numeric op (exact vector broadcast):
    scalar op vector, vector op scalar, and equal-length vector op vector all
    map the scalar op over the elements and yield a vector; scalar op scalar is
    the plain scalar case."""
    # Text operands coerce to their code-point vectors at the numeric boundary.
    if isinstance(a, Str):
        a = Vec([Rational(Fraction(ord(c))) for c in a.s])
    if isinstance(b, Str):
        b = Vec([Rational(Fraction(ord(c))) for c in b.s])
    av, bv = isinstance(a, Vec), isinstance(b, Vec)
    if not av and not bv:
        return _arith_scalar_pair(name, fn, a, b)
    if av and bv:
        if len(a.items) != len(b.items):
            # A one-element vector broadcasts across the other operand.
            if len(a.items) == 1:
                return Vec([_arith_broadcast(name, fn, a.items[0], y) for y in b.items])
            if len(b.items) == 1:
                return Vec([_arith_broadcast(name, fn, x, b.items[0]) for x in a.items])
            raise AjisaiError(
                "custom",
                f"Cannot broadcast shapes [{len(a.items)}] and [{len(b.items)}]")
        return Vec([_arith_broadcast(name, fn, x, y)
                    for x, y in zip(a.items, b.items)])
    if av:
        return Vec([_arith_broadcast(name, fn, x, b) for x in a.items])
    return Vec([_arith_broadcast(name, fn, a, y) for y in b.items])


def binop_arith(name, fn):
    def impl(it: Interp, mods):
        if "STAK" in mods:
            # Left fold of the binary word over the counted group (§6.1).
            group = it.stak_group(mods)
            if not group:
                raise AjisaiError("stackUnderflow")
            acc = group[0]
            for x in group[1:]:
                if isinstance(acc, Nil) or isinstance(x, Nil):
                    acc = Nil(reason=leftmost_nil_reason([acc, x]), origin="nilPropagation")
                    continue
                acc = _arith_broadcast(name, fn, acc, x)
            it.push(acc)
            return
        ops, keep = it.operands(mods, 2)
        a, b = ops[-2], ops[-1]
        # An operand that is itself NIL short-circuits the whole result
        # (Section 4.5.1 / 7.12); per-element NIL inside a vector is handled by
        # the scalar-pair helper during broadcast.
        if isinstance(a, Nil) or isinstance(b, Nil):
            it.push(Nil(reason=leftmost_nil_reason([a, b]), origin="nilPropagation")); return
        it.push(_arith_broadcast(name, fn, a, b))
    return impl

def w_div(it, mods):
    ops, keep = it.operands(mods, 2)
    a, b = ops[-2], ops[-1]
    if isinstance(a, Nil) or isinstance(b, Nil):
        it.push(Nil(reason=leftmost_nil_reason([a, b]), origin="nilPropagation")); return
    fa, fb = as_fraction(a), as_fraction(b)
    if fa is None or fb is None:
        # Admitted-domain division (Section 4.2.7): D is a field, so the
        # quotient stays in normal form. A Sqrt/Alg divisor is never zero
        # (exact zeros demote to Rational 0 and are caught below);
        # ZeroDivisionError is kept as a defensive projection.
        if is_scalar(a) and is_scalar(b):
            try:
                it.push(alg_binop("DIV", a, b))
            except ZeroDivisionError:
                it.push(Nil(reason="divisionByZero", origin="executionFailure"))
            return
        raise AjisaiError("structureError", "DIV needs numbers")
    if fb == 0:
        # A direct division failure originates in the operation itself, not in a
        # propagated NIL (origin = executionFailure, matching the production
        # runtime observed through NIL-ORIGIN; SPEC §4.5.0 / §11.2).
        it.push(Nil(reason="divisionByZero", origin="executionFailure")); return
    it.push(Rational(fa / fb))

def w_mod(it, mods):
    ops, keep = it.operands(mods, 2)
    a, b = ops[-2], ops[-1]
    if isinstance(a, Nil) or isinstance(b, Nil):
        it.push(Nil(reason=leftmost_nil_reason([a, b]), origin="nilPropagation")); return
    fa, fb = as_fraction(a), as_fraction(b)
    if fa is None or fb is None:
        raise AjisaiError("structureError", "MOD needs numbers")
    if fb == 0:
        # MOD by zero is malformed use and raises rather than producing NIL
        # (Section 7.3's deliberate asymmetry with DIV).
        raise AjisaiError("custom", "Modulo by zero")
    # FLOOR-based remainder: x - floor(x/y)*y
    import math
    q = fa / fb
    fl = math.floor(q)
    it.push(Rational(fa - fl * fb))

def unary_round(name, fn):
    def impl(it, mods):
        ops, keep = it.operands(mods, 1)
        a = ops[-1]
        if isinstance(a, Nil):
            it.push(Nil(reason=a.reason, origin="nilPropagation")); return
        fa = as_fraction(a)
        if fa is None:
            raise AjisaiError("structureError", f"{name} needs a number")
        it.push(Rational(Fraction(fn(fa))))
    return impl

import math as _math
def _round_half_away(f):
    """ROUND breaks ties away from zero (Section 7.3): -2.5 -> -3, 0.5 -> 1."""
    if f >= 0:
        return int(_math.floor(f + Fraction(1, 2)))
    return -int(_math.floor(-f + Fraction(1, 2)))

# comparison ----------------------------------------------------------------

def cmp_sign(a: Fraction, b: Fraction):
    if a < b: return -1
    if a > b: return 1
    return 0

def cmp_values(a, b):
    """Return the exact -1/0/1 order. Comparison over the admitted domain D
    is total and exact (Section 4.2.7 / 7.4): the sign of the difference is
    decided on the multiquadratic normal form, never on a budgeted CF
    prefix, so 'U' is unreachable for any current-Coreword operands. NIL is
    handled by the caller."""
    fa, fb = as_fraction(a), as_fraction(b)
    if fa is not None and fb is not None:
        return cmp_sign(fa, fb)
    rads = []
    _collect_rads(a, rads)
    _collect_rads(b, rads)
    basis = _gcd_free_basis(rads)
    return _t_sign(_t_sub(_terms_of(a, basis), _terms_of(b, basis)))

def value_equal(a, b):
    """Exact value identity (Section 4.2.4). Distinct value kinds are never
    equal (Section 4.1: 'TRUE 1 EQ is false'). Returns True/False, or None when
    a scalar comparison is undecidable within budget."""
    if isinstance(a, Bool) and isinstance(b, Bool):
        return a.v == b.v
    if is_scalar(a) and is_scalar(b):
        fa, fb = as_fraction(a), as_fraction(b)
        if fa is not None and fb is not None:
            return fa == fb
        s = cmp_values(a, b)
        return None if s == "U" else (s == 0)
    if isinstance(a, Str) and isinstance(b, Str):
        return a.s == b.s
    if isinstance(a, Nil) and isinstance(b, Nil):
        # Structural equality treats all NIL values uniformly (Sections 4.5.0,
        # 7.4 "NIL operands and NIL equality"): diagnostic metadata (reason,
        # origin, diagnosis) never participates in equality. Top-level NIL
        # operands never reach here — the relations pass them through
        # (Section 7.12) — so this rule is observable only for NIL elements
        # embedded in containers.
        return True
    # Scalar vs one-element vector unwraps and compares the element (Rust EQ
    # one-element rule): 2 [ 2 ] EQ -> TRUE, but 2 [ 2 3 ] EQ -> FALSE.
    if is_scalar(a) and isinstance(b, Vec):
        return value_equal(a, b.items[0]) if len(b.items) == 1 else False
    if isinstance(a, Vec) and is_scalar(b):
        return value_equal(a.items[0], b) if len(a.items) == 1 else False
    if isinstance(a, Vec) and isinstance(b, Vec):
        if len(a.items) != len(b.items):
            return False
        for x, y in zip(a.items, b.items):
            if value_equal(x, y) is not True:
                return False
        return True
    return False  # different kinds

def comparison(name, decide):
    def impl(it, mods):
        if "STAK" in mods:
            return stak_comparison(name, decide, it, mods)
        ops, keep = it.operands(mods, 2)
        a, b = ops[-2], ops[-1]
        if isinstance(a, Nil) or isinstance(b, Nil):
            it.push(Nil(reason=leftmost_nil_reason([a, b]), origin="nilPropagation")); return
        if name in ("EQ", "NEQ"):
            eq = value_equal(a, b)
            if eq is None:
                it.push(UNKNOWN); return
            res = eq if name == "EQ" else (not eq)
            it.push(TRUE if res else FALSE); return
        if not (is_scalar(a) and is_scalar(b)):
            raise AjisaiError("structureError", f"{name} needs numbers")
        s = cmp_values(a, b)
        if s == "U":
            it.push(UNKNOWN); return
        it.push(TRUE if decide(s) else FALSE)
    return impl

def stak_comparison(name, decide, it, mods):
    # Chained predicate over the counted group's adjacent pairs (§6.1).
    ops = it.stak_group(mods)
    if len(ops) < 2:
        # Observed production behavior: a group too small to form a pair is
        # vacuously TRUE and is retained on the stack (only the count is
        # consumed): 3 2 1 .. LT -> 3/1 2/1 TRUE.
        if "KEEP" not in mods:
            for o in ops:
                it.push(o)
        it.push(TRUE)
        return
    # NIL priority
    if any(isinstance(o, Nil) for o in ops):
        it.push(Nil(reason=leftmost_nil_reason(ops), origin="nilPropagation")); return
    for i in range(len(ops) - 1):
        s = cmp_values(ops[i], ops[i+1])
        if s == "U":
            it.push(UNKNOWN); return
        if name == "EQ":
            ok = (s == 0)
        elif name == "NEQ":
            ok = (s != 0)
        else:
            ok = decide(s)
        if not ok:
            it.push(FALSE); return
    it.push(TRUE)

DECIDERS = {
    "LT": lambda s: s < 0, "LTE": lambda s: s <= 0,
    "GT": lambda s: s > 0, "GTE": lambda s: s >= 0,
    "EQ": lambda s: s == 0, "NEQ": lambda s: s != 0,
}

def w_compare_within(it, mods):
    """COMPARE-WITHIN (Section 7.4.2): three-way compare under an explicit
    partial-quotient budget. Unlike the six bare relations — total and exact
    over the admitted domain D (Section 4.2.7) — this word deliberately keeps
    budget semantics: it is the one current-Coreword observation window on
    comparison depth, so equal lazily-composed operands whose CF streams
    never diverge yield the logical UNKNOWN at any budget."""
    keep = "KEEP" in mods
    it.need(3)
    budget_v, b_v, a_v = it.stack[-1], it.stack[-2], it.stack[-3]
    fbud = as_fraction(budget_v)
    if fbud is None or fbud.denominator != 1 or fbud <= 0:
        raise AjisaiError("structureError", "COMPARE-WITHIN needs a positive integer budget")
    budget = int(fbud)
    if isinstance(a_v, Nil) or isinstance(b_v, Nil):
        if not keep:
            del it.stack[-3:]
        it.push(Nil(reason=leftmost_nil_reason([a_v, b_v]), origin="nilPropagation"))
        return
    if not (is_scalar(a_v) and is_scalar(b_v)):
        raise AjisaiError("structureError", "COMPARE-WITHIN needs numbers")
    fa, fb = as_fraction(a_v), as_fraction(b_v)
    if not keep:
        del it.stack[-3:]
    if fa is not None and fb is not None:
        # Two finite CFs always decide, regardless of budget.
        it.push(Rational(Fraction(cmp_sign(fa, fb))))
        return
    s = cmp_values(a_v, b_v)  # exact sign, used only to orient a divergence
    if s == 0:
        # Equal values' CF streams never diverge within any budget.
        it.push(UNKNOWN)
        return
    ta, _ = value_rcf_terms(a_v, budget)
    tb, _ = value_rcf_terms(b_v, budget)
    for k in range(budget):
        in_a, in_b = k < len(ta), k < len(tb)
        if in_a and in_b:
            if ta[k] != tb[k]:
                it.push(Rational(Fraction(s)))
                return
        elif in_a != in_b:
            it.push(Rational(Fraction(s)))
            return
        else:
            break
    it.push(UNKNOWN)

# logic (K3) ----------------------------------------------------------------

def truth_of(v):
    if isinstance(v, Bool): return v.v
    return None

def w_and(it, mods):
    ops, keep = it.operands(mods, 2)
    a, b = ops[-2], ops[-1]
    ta, tb = truth_of(a), truth_of(b)
    # absorbing FALSE (even over NIL/U)
    if ta is False or tb is False:
        it.push(FALSE); return
    if isinstance(a, Nil) or isinstance(b, Nil):
        it.push(Nil(reason=leftmost_nil_reason([a, b]), origin="nilPropagation")); return
    if ta == "U" or tb == "U":
        it.push(UNKNOWN); return
    it.push(TRUE if (ta and tb) else FALSE)

def w_or(it, mods):
    ops, keep = it.operands(mods, 2)
    a, b = ops[-2], ops[-1]
    ta, tb = truth_of(a), truth_of(b)
    if ta is True or tb is True:
        it.push(TRUE); return
    if isinstance(a, Nil) or isinstance(b, Nil):
        it.push(Nil(reason=leftmost_nil_reason([a, b]), origin="nilPropagation")); return
    if ta == "U" or tb == "U":
        it.push(UNKNOWN); return
    it.push(TRUE if (ta or tb) else FALSE)

def w_not(it, mods):
    ops, keep = it.operands(mods, 1)
    a = ops[-1]
    if isinstance(a, Nil):
        it.push(Nil(reason=a.reason, origin="nilPropagation")); return
    t = truth_of(a)
    if t == "U": it.push(UNKNOWN); return
    if t is True: it.push(FALSE); return
    if t is False: it.push(TRUE); return
    raise AjisaiError("structureError", "NOT needs a boolean")

# stack/marker words --------------------------------------------------------

def w_true(it, mods): it.push(TRUE)
def w_false(it, mods): it.push(FALSE)
def w_nil(it, mods): it.push(Nil())
def w_idle(it, mods): pass
def w_flow(it, mods): pass

# Diagnostic absence accessors (Section 4.5.0 / 7.15) -----------------------
# All five retain the inspected value on the stack and push their result above
# it (inspection-word rule, Section 7.1.1). They act on operational NIL only:
# the logical Unknown (U) is a Bool here, not a Nil, so it is never reported as
# absent and its internal reason never leaks (Section 2.3 / 7.5 firewall).
# Applied to a non-operational-NIL value, NIL? is FALSE and the other four are
# NIL (Bubble Rule, Section 11.2), never an error.

def _operational_nil_top(it):
    """The retained top-of-stack value when it is an operational NIL, else None.
    A NIL with no reason and origin 'literal' is still operational."""
    it.need(1)
    v = it.stack[-1]
    return v if isinstance(v, Nil) else None

def _recoverability(nil):
    """Recoverability protocol string (Section 4.5.0). This reference models the
    required field as: a reasonless (literal) NIL is 'unknown'; a reasoned
    Bubble/NIL is 'recoverable' — consistent with the production runtime for the
    division-by-zero and out-of-bounds bubbles the conformance suite observes."""
    return "unknown" if nil.reason is None else "recoverable"

def w_nil_check(it, mods):
    it.need(1)
    it.push(TRUE if isinstance(it.stack[-1], Nil) else FALSE)

def w_nil_reason(it, mods):
    nil = _operational_nil_top(it)
    it.push(Str(nil.reason) if nil and nil.reason else Nil())

def w_nil_origin(it, mods):
    nil = _operational_nil_top(it)
    it.push(Str(nil.origin) if nil else Nil())

def w_nil_recoverable(it, mods):
    nil = _operational_nil_top(it)
    it.push(Str(_recoverability(nil)) if nil else Nil())

def w_nil_diagnosis(it, mods):
    # This reference does not attach a structured diagnosis object to operational
    # NILs (Section 4.5.0 makes `diagnosis` optional), so the accessor yields NIL.
    it.need(1)
    it.push(Nil())

# VENT (^) is not a stack word: it is a lazy control directive handled inline in
# `run_tokens` (Section 6.4). The bare canonical name `VENT` is intentionally not
# a dictionary entry, matching the implementation (only the `^` surface form is
# recognized).

# vector words --------------------------------------------------------------

def w_length(it, mods):
    # Inspection word: the operand is retained and the count pushed above it
    # (Section 7.1.1). A NIL operand is retained with count 0.
    it.need(1)
    v = it.stack[-1]
    if isinstance(v, Vec):
        it.push(Rational(Fraction(len(v.items))))
    elif isinstance(v, Str):
        it.push(Rational(Fraction(len(v.s))))
    elif isinstance(v, Nil):
        it.push(Rational(Fraction(0)))
    else:
        raise AjisaiError("structureError", "LENGTH needs a vector")

def norm_index(i, n):
    if i < 0: i += n
    return i

def _int_index(idx):
    """A bare integer scalar or a one-element integer vector; else None."""
    if isinstance(idx, Vec) and len(idx.items) == 1:
        idx = idx.items[0]
    fi = as_fraction(idx)
    if fi is None or fi.denominator != 1:
        return None
    return int(fi)

def w_get(it, mods):
    # GET retains the source vector and pushes the element above it
    # (inspection rule, Section 7.1.1); the index operand is consumed.
    it.need(2)
    idx = it.stack.pop()
    vec = it.stack[-1]
    if not isinstance(vec, Vec):
        raise AjisaiError("structureError", "GET needs a vector")
    i = _int_index(idx)
    if i is None:
        raise AjisaiError("structureError", "GET needs a single-element integer index")
    i = norm_index(i, len(vec.items))
    if 0 <= i < len(vec.items):
        it.push(vec.items[i])
    else:
        it.push(Nil(reason="indexOutOfBounds", origin="nilPropagation"))

def _codepoint_vec(s):
    return Vec([Rational(Fraction(ord(c))) for c in s.s])

def w_concat(it, mods):
    ops, keep = it.operands(mods, 2)
    a, b = ops[-2], ops[-1]
    # Text operands coerce to their code-point vectors (Section 7.1).
    if isinstance(a, Str):
        a = _codepoint_vec(a)
    if isinstance(b, Str):
        b = _codepoint_vec(b)
    a_items = a.items if isinstance(a, Vec) else [a]
    b_items = b.items if isinstance(b, Vec) else [b]
    it.push(Vec(a_items + b_items))

def w_reverse(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Vec): it.push(Vec(list(reversed(v.items))))
    else: raise AjisaiError("structureError", "REVERSE needs a vector")

def w_range(it, mods):
    # [ start end ] RANGE or [ start end step ] RANGE — inclusive of the end
    # point (Section 7.1).
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if not isinstance(v, Vec) or len(v.items) not in (2, 3):
        raise AjisaiError("custom", "RANGE requires [start end] or [start end step]")
    fs = [as_fraction(x) for x in v.items]
    if any(f is None for f in fs):
        raise AjisaiError("custom", "RANGE requires numbers")
    start, end = fs[0], fs[1]
    step = fs[2] if len(fs) == 3 else (Fraction(1) if end >= start else Fraction(-1))
    if step == 0:
        raise AjisaiError("custom", "RANGE step must be non-zero")
    out = []
    x = start
    if step > 0:
        while x <= end:
            out.append(Rational(x)); x += step
    else:
        while x >= end:
            out.append(Rational(x)); x += step
    it.push(Vec(out) if out else Nil())

def w_take(it, mods):
    ops, keep = it.operands(mods, 2)
    v, k = ops[-2], ops[-1]
    if not isinstance(v, Vec): raise AjisaiError("structureError", "TAKE")
    n = _int_index(k)
    if n is None:
        raise AjisaiError("structureError",
                          "expected single-element value with integer, got NIL"
                          if isinstance(k, Nil) else "TAKE count")
    if n == 0:
        it.push(Nil()); return
    if n > len(v.items):
        raise AjisaiError("custom", "Take count exceeds vector length")
    it.push(Vec(v.items[:n]))

def w_collect(it, mods):
    # Gather a leading-count N of stack values (Section 7.1.1).
    it.need(1)
    cnt = it.stack.pop()
    f = as_fraction(cnt)
    if f is None or f.denominator != 1 or f < 0:
        raise AjisaiError("structureError", "COLLECT needs a leading count")
    n = int(f)
    if len(it.stack) < n:
        raise AjisaiError("stackUnderflow")
    items = it.stack[len(it.stack) - n:] if n else []
    if n:
        del it.stack[len(it.stack) - n:]
    it.push(Vec(items) if items else Nil())

def _index_element_pair(pair):
    """The two-element [ index element ] argument of INSERT/REPLACE."""
    if not (isinstance(pair, Vec) and len(pair.items) == 2):
        raise AjisaiError("custom", "expected a two-element [ index element ] vector")
    i = _int_index(pair.items[0])
    if i is None:
        raise AjisaiError("custom", "expected a two-element [ index element ] vector")
    return i, pair.items[1]

def w_insert(it, mods):
    ops, keep = it.operands(mods, 2)
    v, pair = ops[-2], ops[-1]
    if not isinstance(v, Vec):
        raise AjisaiError("custom", "INSERT requires a vector and an [ index element ] vector")
    i, val = _index_element_pair(pair)
    i = norm_index(i, len(v.items))
    if not (0 <= i <= len(v.items)):
        raise AjisaiError("indexOutOfBounds",
                          f"Index {i} out of bounds for vector of length {len(v.items)}")
    items = list(v.items); items.insert(i, val); it.push(Vec(items))

def w_replace(it, mods):
    ops, keep = it.operands(mods, 2)
    v, pair = ops[-2], ops[-1]
    if not isinstance(v, Vec):
        raise AjisaiError("custom", "REPLACE requires a vector and an [ index element ] vector")
    i, val = _index_element_pair(pair)
    i = norm_index(i, len(v.items))
    if not (0 <= i < len(v.items)):
        raise AjisaiError("indexOutOfBounds",
                          f"Index {i} out of bounds for vector of length {len(v.items)}")
    items = list(v.items); items[i] = val; it.push(Vec(items))

def w_remove(it, mods):
    ops, keep = it.operands(mods, 2)
    v, idx = ops[-2], ops[-1]
    if not isinstance(v, Vec): raise AjisaiError("structureError", "REMOVE")
    i = _int_index(idx)
    if i is None: raise AjisaiError("structureError", "REMOVE index")
    j = norm_index(i, len(v.items))
    if not (0 <= j < len(v.items)):
        raise AjisaiError("indexOutOfBounds",
                          f"Index {i} out of bounds for vector of length {len(v.items)}")
    items = list(v.items); del items[j]; it.push(Vec(items))

def w_split(it, mods):
    # vector [ sizes... ] SPLIT -> each sub-vector pushed separately (§7.1).
    ops, keep = it.operands(mods, 2)
    v, sizes = ops[-2], ops[-1]
    if not (isinstance(v, Vec) and isinstance(sizes, Vec)):
        raise AjisaiError("custom", "SPLIT requires a vector and a sizes vector")
    ns = []
    for s in sizes.items:
        f = as_fraction(s)
        if f is None or f.denominator != 1 or f < 0:
            raise AjisaiError("custom", "SPLIT sizes must be non-negative integers")
        ns.append(int(f))
    if sum(ns) > len(v.items):
        raise AjisaiError("custom", "Split sizes sum exceeds vector length")
    at = 0
    for n in ns:
        it.push(Vec(v.items[at:at + n]))
        at += n

def w_reorder(it, mods):
    ops, keep = it.operands(mods, 2)
    v, idxs = ops[-2], ops[-1]
    if not (isinstance(v, Vec) and isinstance(idxs, Vec)):
        raise AjisaiError("custom", "REORDER requires a vector and an index vector")
    out = []
    for ix in idxs.items:
        f = as_fraction(ix)
        if f is None or f.denominator != 1:
            raise AjisaiError("custom", "REORDER indices must be integers")
        i = int(f)
        j = norm_index(i, len(v.items))
        if not (0 <= j < len(v.items)):
            raise AjisaiError("indexOutOfBounds",
                              f"Index {i} out of bounds for vector of length {len(v.items)}")
        out.append(v.items[j])
    it.push(Vec(out))

# tensor --------------------------------------------------------------------

def shape_of(v):
    if isinstance(v, Vec):
        if v.items and all(isinstance(x, Vec) for x in v.items):
            inner = shape_of(v.items[0])
            return [len(v.items)] + inner
        return [len(v.items)]
    return []

def w_shape(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    dims = shape_of(v)
    it.push(Vec([Rational(Fraction(x)) for x in dims]) if dims else Nil())

def w_rank(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    it.push(Rational(Fraction(len(shape_of(v)))))

def _flatten(v, out):
    for x in v.items:
        if isinstance(x, Vec):
            _flatten(x, out)
        else:
            out.append(x)

def _build_shape(flat, dims, at):
    if len(dims) == 1:
        return Vec(flat[at:at + dims[0]]), at + dims[0]
    rows = []
    for _ in range(dims[0]):
        row, at = _build_shape(flat, dims[1:], at)
        rows.append(row)
    return Vec(rows), at

def w_reshape(it, mods):
    ops, keep = it.operands(mods, 2)
    v, shape = ops[-2], ops[-1]
    if not (isinstance(v, Vec) and isinstance(shape, Vec)):
        raise AjisaiError("custom", "RESHAPE requires a vector and a shape vector")
    dims = []
    for d in shape.items:
        f = as_fraction(d)
        if f is None or f.denominator != 1 or f <= 0:
            raise AjisaiError("custom", "RESHAPE shape must be positive integers")
        dims.append(int(f))
    flat = []
    _flatten(v, flat)
    need = 1
    for d in dims:
        need *= d
    if need != len(flat):
        raise AjisaiError(
            "custom",
            f"RESHAPE failed: data length {len(flat)} doesn't match shape {dims}")
    out, _ = _build_shape(flat, dims, 0)
    it.push(out)

def w_transpose(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if not (isinstance(v, Vec) and v.items
            and all(isinstance(r, Vec) for r in v.items)
            and len({len(r.items) for r in v.items}) == 1):
        raise AjisaiError("custom", "TRANSPOSE requires 2D vector")
    rows = [r.items for r in v.items]
    it.push(Vec([Vec(list(col)) for col in zip(*rows)]))

def w_fill(it, mods):
    # [ shape... value ] FILL (Section 7.2): at least one dimension + a value.
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if not (isinstance(v, Vec) and len(v.items) >= 2):
        raise AjisaiError("custom", "FILL requires [shape... value] (at least 2 elements)")
    dims = []
    for d in v.items[:-1]:
        f = as_fraction(d)
        if f is None or f.denominator != 1 or f <= 0:
            raise AjisaiError("custom", "FILL shape must be positive integers")
        dims.append(int(f))
    value = v.items[-1]
    def build(ds):
        if len(ds) == 1:
            return Vec([value for _ in range(ds[0])])
        return Vec([build(ds[1:]) for _ in range(ds[0])])
    it.push(build(dims))

# string/conv ---------------------------------------------------------------

def _str_render(v):
    """STR's Text rendering (Section 7.6.1): integers drop the /1 denominator;
    a Vector or Record flattens to its space-joined leaves."""
    if isinstance(v, Rational):
        if v.f.denominator == 1:
            return str(v.f.numerator)
        return f"{v.f.numerator}/{v.f.denominator}"
    if isinstance(v, (Vec, Rec)):
        return " ".join(_str_leaves(v))
    return output_render(v)

def _str_leaves(v):
    """Flat leaf sequence of a container for STR (Section 7.6.1): scalar
    leaves render by _str_render, Booleans by their spelling, NIL (and U,
    which shares absence storage on this surface) as the letters NIL, and a
    Text element decays to its code-point scalars."""
    out = []
    items = v.items
    for x in items:
        if isinstance(x, Str):
            out.extend(str(ord(c)) for c in x.s)
        elif isinstance(x, (Vec, Rec)):
            out.extend(_str_leaves(x))
        elif isinstance(x, Nil):
            out.append("NIL")
        elif isinstance(x, Bool):
            out.append({True: "TRUE", False: "FALSE", "U": "NIL"}[x.v])
        else:
            out.append(_str_render(x))
    return out

def w_str(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Bool) and v.v == "U":
        # Section 7.6.1: U has no text surface; the result is a fresh NIL and
        # the logical identity is not observable through it.
        it.push(Nil()); return
    if isinstance(v, Nil):
        # Section 7.6.1: STR is not a Section 7.12 passthrough word — the NIL
        # result is fresh and does not carry the operand's reason.
        it.push(Nil()); return
    it.push(Str(_str_render(v) if not isinstance(v, Str) else v.s))

def w_num(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Nil):
        raise AjisaiError("custom", "NUM: expected String, got Nil")
    if not isinstance(v, Str): raise AjisaiError("custom", "NUM: expected String")
    n = number_of(v.s.strip())
    if n is None: it.push(Nil(reason="invalidEncoding", origin="nilPropagation")); return
    it.push(n)

def w_bool(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Bool):
        if v.v == "U":
            # Section 7.6.1: U is malformed use for BOOL, through the same
            # channel as NIL.
            raise AjisaiError("custom", "BOOL: expected String or Number, got Nil")
        it.push(v); return
    if isinstance(v, Nil):
        raise AjisaiError("custom", "BOOL: expected String or Number, got Nil")
    if isinstance(v, Str):
        low = v.s.lower()
        if low == "true": it.push(TRUE); return
        if low == "false": it.push(FALSE); return
        # Section 7.6.1: any other Text — including numeric text like '42' —
        # is a well-formed conversion failure -> NIL. Text is never routed
        # through the numeric zero/non-zero rule.
        it.push(Nil()); return
    fa = as_fraction(v)
    if fa is not None: it.push(TRUE if fa != 0 else FALSE); return
    raise AjisaiError("custom", "BOOL: expected String or Number")

def w_chr(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Nil):
        raise AjisaiError("custom", "CHR: expected Number, got Nil")
    fa = as_fraction(v)
    if fa is None or fa.denominator != 1:
        raise AjisaiError("custom", "CHR: expected Number input")
    cp = int(fa)
    if cp < 0 or cp > 0x10FFFF or (0xD800 <= cp <= 0xDFFF):
        it.push(Nil(reason="invalidEncoding", origin="nilPropagation")); return
    it.push(Str(chr(cp)))

def w_chars(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if not isinstance(v, Str): raise AjisaiError("custom", "CHARS: expected String")
    it.push(Vec([Str(c) for c in v.s]))

def w_join(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Str):
        # Section 7.6.1: a Text target is a code-point vector; it joins back
        # to itself. (This is also why JOIN has no separator operand — a Text
        # on top of the stack is JOIN's target, never a separator.)
        it.push(Str(v.s)); return
    if not isinstance(v, Vec):
        kindname = "Number" if as_fraction(v) is not None else type(v).__name__
        raise AjisaiError("custom", f"JOIN: expected Vector, got {kindname}")
    parts = []
    for i, x in enumerate(v.items):
        if isinstance(x, Str):
            parts.append(x.s)
            continue
        f = as_fraction(x)
        if f is not None:
            # A numeric element is a code point (Section 7.6.1); a scalar
            # outside the valid code-point range is malformed use.
            if f.denominator == 1 and 0 <= int(f) <= 0x10FFFF and not (0xD800 <= int(f) <= 0xDFFF):
                parts.append(chr(int(f)))
                continue
            raise AjisaiError("custom", f"JOIN: invalid character code at index {i}")
        kindname = ("nil" if isinstance(x, Nil)
                    else "boolean" if isinstance(x, Bool)
                    else "other format")
        raise AjisaiError("custom",
                          f"JOIN: all elements must be strings, found {kindname} at index {i}")
    it.push(Str("".join(parts)))

# text words (Section 7.6) ---------------------------------------------------

def _text_op(word, argname="String"):
    def check(v, role="String"):
        if not isinstance(v, Str):
            kindname = "Number" if as_fraction(v) is not None else type(v).__name__
            raise AjisaiError("custom", f"{word}: expected {role}, got {kindname}")
        return v.s
    return check

def w_trim(it, mods):
    ops, keep = it.operands(mods, 1)
    s = _text_op("TRIM")(ops[-1])
    it.push(Str(s.strip()))

def w_trim_left(it, mods):
    ops, keep = it.operands(mods, 1)
    s = _text_op("TRIM-LEFT")(ops[-1])
    it.push(Str(s.lstrip()))

def w_trim_right(it, mods):
    ops, keep = it.operands(mods, 1)
    s = _text_op("TRIM-RIGHT")(ops[-1])
    it.push(Str(s.rstrip()))

def w_tokenize(it, mods):
    ops, keep = it.operands(mods, 2)
    s = _text_op("TOKENIZE")(ops[-2])
    sep = _text_op("TOKENIZE")(ops[-1], "separator as String")
    if sep == "":
        raise AjisaiError("custom", "TOKENIZE: empty separator")
    it.push(Vec([Str(p) for p in s.split(sep)]))

def w_substitute(it, mods):
    ops, keep = it.operands(mods, 3)
    s = _text_op("SUBSTITUTE")(ops[-3])
    frm = _text_op("SUBSTITUTE")(ops[-2], "from as String")
    to = _text_op("SUBSTITUTE")(ops[-1], "to as String")
    it.push(Str(s.replace(frm, to)))

def w_starts_with(it, mods):
    ops, keep = it.operands(mods, 2)
    s = _text_op("STARTS-WITH?")(ops[-2])
    affix = _text_op("STARTS-WITH?")(ops[-1], "affix as String")
    it.push(TRUE if s.startswith(affix) else FALSE)

def w_ends_with(it, mods):
    ops, keep = it.operands(mods, 2)
    s = _text_op("ENDS-WITH?")(ops[-2])
    affix = _text_op("ENDS-WITH?")(ops[-1], "affix as String")
    it.push(TRUE if s.endswith(affix) else FALSE)

def w_tocf(it, mods):
    # >CF changes only the requested display role (Section 3.9), never the
    # value; this reference keeps the value unchanged.
    it.need(1)
    pass

# QUANTIZE family and CONSERVE (Sections 7.13, 13.3) -------------------------

def _quantize_multiple(mode, m):
    """The integer grid multiple n chosen from m = x/step for each mode."""
    if mode == "floor":
        return _math.floor(m)
    if mode == "ceil":
        return _math.ceil(m)
    if mode == "trunc":
        return _math.floor(m) if m >= 0 else _math.ceil(m)
    if mode == "half-away":
        return _round_half_away(m)
    # banker's (round-half-to-even), the QUANTIZE default
    fl = _math.floor(m)
    frac = m - fl
    if frac > Fraction(1, 2):
        return fl + 1
    if frac < Fraction(1, 2):
        return fl
    return fl if fl % 2 == 0 else fl + 1

def quantize_word(name, mode):
    def impl(it, mods):
        ops, keep = it.operands(mods, 2)
        x, step = ops[-2], ops[-1]
        fstep = as_fraction(step)
        if fstep is None or fstep <= 0:
            raise AjisaiError("custom",
                              f"{name} requires a strictly positive rational step")
        if isinstance(x, Nil):
            # NIL passes through to BOTH outputs, carrying its reason (§7.13).
            it.push(Nil(reason=x.reason, origin=x.origin))
            it.push(Nil(reason=x.reason, origin=x.origin))
            return
        fx = as_fraction(x)
        if fx is None:
            raise AjisaiError("custom", f"{name} requires a rational value")
        n = _quantize_multiple(mode, fx / fstep)
        q = n * fstep
        it.push(Rational(q))
        it.push(Rational(fx - q))
    return impl

def w_conserve(it, mods):
    ops, keep = it.operands(mods, 2)
    total, parts = ops[-2], ops[-1]
    if isinstance(total, Nil) or (isinstance(parts, Vec)
                                  and any(isinstance(p, Nil) for p in parts.items)):
        raise AjisaiError("custom",
                          "CONSERVE cannot certify conservation with a NIL operand")
    ftotal = as_fraction(total)
    if ftotal is None or not isinstance(parts, Vec):
        raise AjisaiError("custom", "CONSERVE requires a scalar total and a parts vector")
    s = Fraction(0)
    for p in parts.items:
        fp = as_fraction(p)
        if fp is None:
            raise AjisaiError("custom", "CONSERVE requires scalar parts")
        s += fp
    if s != ftotal:
        raise AjisaiError("custom", "Conservation violated: parts do not sum to the total")
    it.push(parts)

# higher-order --------------------------------------------------------------
#
# Normative stack signatures: SPECIFICATION.html Section 7.7.1. The block
# argument (top of stack) may be either a { ... } code block or a quoted word
# name (a Text value naming a word), e.g. 'DBL' or '+'. Each block runs in an
# isolated evaluation seeded with exactly the contract inputs.

def _new_sub(it):
    sub = Interp()
    sub.user_words = it.user_words
    sub.imported = it.imported
    sub.visible = it.visible
    return sub

def run_callable(sub, callable_val):
    """Execute a block argument: a { ... } code block or a quoted word name."""
    if isinstance(callable_val, Block):
        sub.run_block(callable_val); return
    if isinstance(callable_val, Str):
        canon = ALIAS.get(callable_val.s, callable_val.s).upper()
        sub.exec_word(canon, []); return
    raise AjisaiError("structureError", "higher-order word needs a block or word name")

def _is_callable(v):
    return isinstance(v, (Block, Str))

def _unwrap_one_element(v):
    """A one-element vector result decays to its element (MAP/SCAN rule)."""
    if isinstance(v, Vec) and len(v.items) == 1:
        return v.items[0]
    return v

def _run_block_one(it, callable_val, *inputs):
    """Run the block on a fresh stack seeded with `inputs`; return its single
    top result (raising if it left nothing)."""
    sub = _new_sub(it)
    for x in inputs:
        sub.push(x)
    run_callable(sub, callable_val)
    if not sub.stack:
        raise AjisaiError("structureError", "higher-order block returned no value")
    return sub.stack[-1]

def _predicate_true(res):
    """Interpret a predicate result as a definite truth (FILTER/ANY/ALL/COUNT).
    A definite TRUE (bare, or one-element vector) fires; a truthy scalar fires;
    FALSE / U / zero / NIL do not."""
    if isinstance(res, Bool):
        return res.v is True
    if isinstance(res, Vec) and len(res.items) == 1:
        return _predicate_true(res.items[0])
    fa = as_fraction(res)
    if fa is not None:
        return fa != 0
    return False

def w_map(it, mods):
    ops, keep = it.operands(mods, 2)
    vec, blk = ops[-2], ops[-1]
    if not _is_callable(blk):
        raise AjisaiError("structureError", "MAP needs a block or word name")
    if isinstance(vec, Nil):
        it.push(Nil()); return
    if not isinstance(vec, Vec):
        raise AjisaiError("structureError", "MAP needs a vector")
    res = [_unwrap_one_element(_run_block_one(it, blk, x)) for x in vec.items]
    it.push(Vec(res))

def w_filter(it, mods):
    ops, keep = it.operands(mods, 2)
    vec, blk = ops[-2], ops[-1]
    if not _is_callable(blk):
        raise AjisaiError("structureError", "FILTER needs a block or word name")
    if isinstance(vec, Nil):
        it.push(Nil()); return
    if not isinstance(vec, Vec):
        raise AjisaiError("structureError", "FILTER needs a vector")
    res = [x for x in vec.items if _predicate_true(_run_block_one(it, blk, x))]
    it.push(Vec(res) if res else Nil())

def w_fold(it, mods):
    ops, keep = it.operands(mods, 3)
    vec, init, blk = ops[-3], ops[-2], ops[-1]
    if not _is_callable(blk):
        raise AjisaiError("structureError", "FOLD needs a block or word name")
    if isinstance(vec, Nil):
        it.push(init); return
    if not isinstance(vec, Vec):
        raise AjisaiError("structureError", "FOLD needs a vector")
    acc = init
    for x in vec.items:
        acc = _run_block_one(it, blk, acc, x)   # block sees `acc elem`
    it.push(acc)

def w_scan(it, mods):
    ops, keep = it.operands(mods, 3)
    vec, init, blk = ops[-3], ops[-2], ops[-1]
    if not _is_callable(blk):
        raise AjisaiError("structureError", "SCAN needs a block or word name")
    if isinstance(vec, Nil):
        it.push(Nil()); return
    if not isinstance(vec, Vec):
        raise AjisaiError("structureError", "SCAN needs a vector")
    acc = init
    res = []
    for x in vec.items:
        acc = _run_block_one(it, blk, acc, x)
        res.append(_unwrap_one_element(acc))
    it.push(Vec(res) if res else Nil())

def w_unfold(it, mods):
    ops, keep = it.operands(mods, 2)
    seed, blk = ops[-2], ops[-1]
    if not _is_callable(blk):
        raise AjisaiError("structureError", "UNFOLD needs a block or word name")
    MAX_ITERATIONS = 10000
    state = seed
    res = []
    for _ in range(MAX_ITERATIONS):
        out = _run_block_one(it, blk, state)
        if isinstance(out, Nil):
            break
        if isinstance(out, Vec) and len(out.items) == 2:
            res.append(out.items[0])
            nxt = out.items[1]
            if isinstance(nxt, Nil):
                break
            state = Vec([nxt])
            continue
        raise AjisaiError("structureError",
                          "UNFOLD expected [element, next_state] or NIL")
    else:
        raise AjisaiError("executionLimitExceeded", "UNFOLD non-termination")
    it.push(Vec(res) if res else Nil())

def w_any(it, mods):
    ops, keep = it.operands(mods, 2)
    vec, blk = ops[-2], ops[-1]
    if not _is_callable(blk):
        raise AjisaiError("structureError", "ANY needs a block or word name")
    if isinstance(vec, Nil):
        it.push(FALSE); return
    if not isinstance(vec, Vec):
        raise AjisaiError("structureError", "ANY needs a vector")
    for x in vec.items:
        if _predicate_true(_run_block_one(it, blk, x)):
            it.push(TRUE); return
    it.push(FALSE)

def w_all(it, mods):
    ops, keep = it.operands(mods, 2)
    vec, blk = ops[-2], ops[-1]
    if not _is_callable(blk):
        raise AjisaiError("structureError", "ALL needs a block or word name")
    if isinstance(vec, Nil):
        it.push(TRUE); return
    if not isinstance(vec, Vec):
        raise AjisaiError("structureError", "ALL needs a vector")
    for x in vec.items:
        if not _predicate_true(_run_block_one(it, blk, x)):
            it.push(FALSE); return
    it.push(TRUE)

def w_count(it, mods):
    ops, keep = it.operands(mods, 2)
    vec, blk = ops[-2], ops[-1]
    if not _is_callable(blk):
        raise AjisaiError("structureError", "COUNT needs a block or word name")
    if isinstance(vec, Nil):
        it.push(Rational(Fraction(0))); return   # bare scalar 0/1 on NIL target
    if not isinstance(vec, Vec):
        raise AjisaiError("structureError", "COUNT needs a vector")
    n = sum(1 for x in vec.items if _predicate_true(_run_block_one(it, blk, x)))
    it.push(Vec([Rational(Fraction(n))]))         # [ n ] on a vector target

def w_exec(it, mods):
    ops, keep = it.operands(mods, 1)
    blk = ops[-1]
    if not isinstance(blk, Block): raise AjisaiError("structureError", "EXEC")
    it.run_block(blk)

def w_cond(it, mods):
    # Collect all consecutive CodeBlock clause values on top of the stack
    # (source order), then consume the subject beneath them (Section 7.7.1).
    blocks = []
    while it.stack and isinstance(it.stack[-1], Block):
        blocks.append(it.stack.pop())
    blocks.reverse()
    if not blocks:
        raise AjisaiError("structureError", "COND expected clause blocks")
    clauses = split_cond_blocks(blocks)
    it.need(1)
    subject = it.stack.pop()

    else_body = None
    for guard_toks, body_toks in clauses:
        if guard_toks is None:            # IDLE / else clause
            else_body = body_toks
            continue
        if eval_cond_guard(it, guard_toks, subject):
            run_cond_body(it, body_toks, subject); return
    if else_body is not None:
        run_cond_body(it, else_body, subject); return
    raise AjisaiError("condExhausted", "COND: all guards failed and no else clause")

def _block_tokens(blk):
    """Flatten a clause block's token lines into one token list."""
    toks = []
    for line in blk.lines:
        toks.extend(line)
    return toks

def _is_idle_tokens(toks):
    return len(toks) == 1 and toks[0][0] == "word" and toks[0][1].upper() == "IDLE"

def _split_at_top_bar(toks):
    depth = 0
    for i, (k, v) in enumerate(toks):
        if k == "sym" and v in "[{":
            depth += 1
        elif k == "sym" and v in "]}":
            depth -= 1
        elif k == "sym" and v == "|" and depth == 0:
            return toks[:i], toks[i+1:]
    return None

def split_cond_blocks(blocks):
    """Split collected clause blocks into (guard_toks | None, body_toks) pairs.
    Two styles: every clause is `{ guard | body }`, or every clause is a bare
    `{ guard }` / `{ body }` pair. Styles may not be mixed (Section 7.7.1)."""
    token_lists = [_block_tokens(b) for b in blocks]
    bars = [_split_at_top_bar(t) for t in token_lists]
    if all(b is not None for b in bars):
        clauses = []
        for guard, body in bars:
            if _is_idle_tokens(guard):
                clauses.append((None, body))
            else:
                clauses.append((guard, body))
        return clauses
    if all(b is None for b in bars):
        if len(token_lists) % 2 != 0:
            raise AjisaiError("structureError", "COND expected guard/body pairs")
        clauses = []
        for i in range(0, len(token_lists), 2):
            guard, body = token_lists[i], token_lists[i+1]
            if _is_idle_tokens(guard):
                clauses.append((None, body))
            else:
                clauses.append((guard, body))
        return clauses
    raise AjisaiError("structureError", "COND: mixed clause styles are not allowed")

def eval_cond_guard(it, guard_toks, subject):
    """Run a guard in isolation on a fresh copy of the subject; a definite TRUE
    fires. A U (unknown) guard does not fire (Section 7.4.3)."""
    sub = _new_sub(it)
    sub.push(subject)
    sub.run_tokens(list(guard_toks))
    if not sub.stack:
        raise AjisaiError("structureError", "COND guard returned no value")
    return _predicate_true(sub.stack[-1])

def run_cond_body(it, body_toks, subject):
    """Run the winning body in isolation seeded with the subject; push exactly
    one value (the body's stack top) onto the caller's stack."""
    sub = _new_sub(it)
    sub.push(subject)
    sub.run_tokens(list(body_toks))
    if not sub.stack:
        raise AjisaiError("structureError", "COND body must return a value")
    it.push(sub.stack[-1])

# def / del -----------------------------------------------------------------

# Words whose execution is not definition-time-safe for PRECOMPUTE (§7.7):
# effectful/observable words must not fire at DEF time. Production derives
# this from the Section 7.14 contract registry (purity/effects); this
# Core-only reference interpreter's effectful surface is PRINT.
_NOT_COMPTIME_SAFE = frozenset({"PRINT"})

def _stage_precompute(it, body):
    """Definition-time staging (Section 7.7): each `{ ... } PRECOMPUTE` inside a
    DEF body is evaluated once now and its resulting values are spliced into the
    compiled definition as literal ('val', v) tokens."""
    staged_lines = []
    for line in body.lines:
        out = []
        i = 0
        while i < len(line):
            k, v = line[i]
            if (k == "word" and ALIAS.get(v, v).upper() == "PRECOMPUTE"
                    and out and out[-1] == ("sym", "}")):
                # find the matching '{' backwards in `out`
                depth = 0
                j = len(out) - 1
                while j >= 0:
                    kk, vv = out[j]
                    if kk == "sym" and vv == "}":
                        depth += 1
                    elif kk == "sym" and vv == "{":
                        depth -= 1
                        if depth == 0:
                            break
                    j -= 1
                if j >= 0:
                    inner = out[j + 1:len(out) - 1]
                    del out[j:]
                    # Section 7.7 (observable staging contract): the staged
                    # block must be definition-time-safe — an effectful word
                    # would fire its effect at DEF rather than at call time.
                    # Production decides this from the Section 7.14 registry;
                    # this Core-only reference's effectful surface is PRINT.
                    for kk, vv in inner:
                        if kk == "word" and ALIAS.get(vv, vv).upper() in _NOT_COMPTIME_SAFE:
                            raise AjisaiError(
                                "custom",
                                f"PRECOMPUTE rejected: word {ALIAS.get(vv, vv).upper()} is not comptime-safe")
                    sub = _new_sub(it)
                    # Isolated, *empty* definition-time evaluation (§7.7): the
                    # block never sees the definition-time stack, and any
                    # failure is a definition-time error surfaced by DEF.
                    try:
                        sub.run_tokens(list(inner))
                    except AjisaiError as e:
                        detail = e.args[1] if len(e.args) > 1 else e.args[0]
                        raise AjisaiError("custom", f"PRECOMPUTE failed: {detail}")
                    for val in sub.stack:
                        if isinstance(val, Nil):
                            raise AjisaiError(
                                "custom",
                                "PRECOMPUTE failed: result contains unsupported value type")
                    out.extend(("val", val) for val in sub.stack)
                    i += 1
                    continue
            out.append((k, v))
            i += 1
        staged_lines.append(out)
    return Block(staged_lines)

def w_def(it, mods):
    name = it.pop(); body = it.pop()
    if not isinstance(name, Str): raise AjisaiError("structureError", "DEF name must be string")
    if not isinstance(body, Block): raise AjisaiError("structureError", "DEF body must be block")
    nm = name.s.upper()
    if nm in CORE: raise AjisaiError("builtinProtection")
    it.forc = False
    it.user_words[nm] = _stage_precompute(it, body)

def _dependents_of(it, nm):
    """User words whose body references `nm` (dependency protection, §8.2)."""
    out = []
    for other, blk in it.user_words.items():
        if other == nm:
            continue
        for line in blk.lines:
            if any(k == "word" and ALIAS.get(v, v).upper() == nm for k, v in line):
                out.append(other)
                break
    return out

def w_del(it, mods):
    name = it.pop()
    if not isinstance(name, Str):
        raise AjisaiError("structureError", "DEL name must be string")
    nm = name.s.upper()
    force = it.forc
    it.forc = False
    if nm not in it.user_words:
        raise AjisaiError("custom", f"Word '{nm}' is not defined")
    deps = _dependents_of(it, nm)
    if deps and not force:
        raise AjisaiError("custom",
                          f"Cannot delete '{nm}': referenced by {', '.join(deps)}."
                          f" Use ! '{nm}' DEL to force.")
    del it.user_words[nm]

def w_forc(it, mods):
    it.forc = True

def w_lookup(it, mods):
    # LOOKUP consumes the word name; its definition goes to the human-readable
    # output surface, which is not an observation target of the suite.
    name = it.pop()
    if not isinstance(name, Str):
        raise AjisaiError("structureError", "LOOKUP name must be string")

def w_eval(it, mods):
    src = it.pop()
    if not isinstance(src, Str):
        raise AjisaiError("custom", "EVAL: expected String")
    for raw in src.s.split("\n"):
        toks = tokenize_line(raw)
        if toks:
            it.run_tokens(toks)

def w_precompute(it, mods):
    # Reaching the word at runtime means it was not staged by DEF (§7.7).
    raise AjisaiError("custom",
                      "PRECOMPUTE can only be used during definition-time precomputation")

def w_print(it, mods):
    keep = "KEEP" in mods
    it.need(1)
    v = it.stack[-1] if keep else it.stack.pop()
    it.output.append(output_render(v))

def _module_of(name_val):
    if not isinstance(name_val, Str):
        raise AjisaiError("structureError", "module name must be string")
    up = name_val.s.upper()
    if up not in MODULES:
        raise AjisaiError("unknownModule", f"Unknown module: {up}")
    return up

def _selector_names(v):
    if not isinstance(v, Vec) or not all(isinstance(x, Str) for x in v.items):
        raise AjisaiError("structureError", "expected a vector of word names")
    return {x.s.upper() for x in v.items}

def w_import(it, mods):
    up = _module_of(it.pop())
    it.imported.add(up)
    it.visible |= MODULES[up]

def w_import_only(it, mods):
    names = _selector_names(it.pop())
    up = _module_of(it.pop())
    it.imported.add(up)
    it.visible |= (MODULES[up] & names)

def w_unimport(it, mods):
    up = _module_of(it.pop())
    it.visible -= MODULES[up]

def w_unimport_only(it, mods):
    names = _selector_names(it.pop())
    up = _module_of(it.pop())
    it.visible -= (MODULES[up] & names)

CORE = {
    "ADD": binop_arith("ADD", lambda a, b: a + b),
    "SUB": binop_arith("SUB", lambda a, b: a - b),
    "MUL": binop_arith("MUL", lambda a, b: a * b),
    "DIV": w_div, "MOD": w_mod,
    "FLOOR": unary_round("FLOOR", lambda f: _math.floor(f)),
    "CEIL": unary_round("CEIL", lambda f: _math.ceil(f)),
    "ROUND": unary_round("ROUND", _round_half_away),
    "LT": comparison("LT", DECIDERS["LT"]), "LTE": comparison("LTE", DECIDERS["LTE"]),
    "GT": comparison("GT", DECIDERS["GT"]), "GTE": comparison("GTE", DECIDERS["GTE"]),
    "EQ": comparison("EQ", DECIDERS["EQ"]), "NEQ": comparison("NEQ", DECIDERS["NEQ"]),
    "COMPARE-WITHIN": w_compare_within,
    "AND": w_and, "OR": w_or, "NOT": w_not,
    "TRUE": w_true, "FALSE": w_false, "NIL": w_nil, "IDLE": w_idle,
    "FLOW": w_flow,
    "NIL?": w_nil_check, "NIL-REASON": w_nil_reason, "NIL-ORIGIN": w_nil_origin,
    "NIL-RECOVERABLE?": w_nil_recoverable, "NIL-DIAGNOSIS": w_nil_diagnosis,
    "LENGTH": w_length, "GET": w_get, "CONCAT": w_concat, "REVERSE": w_reverse,
    "RANGE": w_range, "TAKE": w_take, "COLLECT": w_collect,
    "INSERT": w_insert, "REPLACE": w_replace, "REMOVE": w_remove,
    "SPLIT": w_split, "REORDER": w_reorder,
    "SHAPE": w_shape, "RANK": w_rank,
    "RESHAPE": w_reshape, "TRANSPOSE": w_transpose, "FILL": w_fill,
    "STR": w_str, "NUM": w_num, "BOOL": w_bool, "CHR": w_chr, "CHARS": w_chars, "JOIN": w_join,
    ">CF": w_tocf,
    "TRIM": w_trim, "TRIM-LEFT": w_trim_left, "TRIM-RIGHT": w_trim_right,
    "TOKENIZE": w_tokenize, "SUBSTITUTE": w_substitute,
    "STARTS-WITH?": w_starts_with, "ENDS-WITH?": w_ends_with,
    "QUANTIZE": quantize_word("QUANTIZE", "even"),
    "QUANTIZE-FLOOR": quantize_word("QUANTIZE-FLOOR", "floor"),
    "QUANTIZE-CEIL": quantize_word("QUANTIZE-CEIL", "ceil"),
    "QUANTIZE-TRUNC": quantize_word("QUANTIZE-TRUNC", "trunc"),
    "QUANTIZE-HALF-AWAY": quantize_word("QUANTIZE-HALF-AWAY", "half-away"),
    "CONSERVE": w_conserve,
    "MAP": w_map, "FILTER": w_filter, "FOLD": w_fold, "SCAN": w_scan,
    "UNFOLD": w_unfold, "ANY": w_any, "ALL": w_all, "COUNT": w_count,
    "EXEC": w_exec, "EVAL": w_eval, "COND": w_cond, "PRECOMPUTE": w_precompute,
    "DEF": w_def, "DEL": w_del, "FORC": w_forc, "LOOKUP": w_lookup,
    "PRINT": w_print,
    "IMPORT": w_import, "IMPORT-ONLY": w_import_only,
    "UNIMPORT": w_unimport, "UNIMPORT-ONLY": w_unimport_only,
    # TOP/STAK/EAT/KEEP are handled as modifiers, not here.
}

# module words (minimal MATH) ----------------------------------------------

def w_sqrt(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    fa = as_fraction(v)
    if fa is None: raise AjisaiError("structureError", "SQRT")
    if fa < 0: it.push(Nil(reason="domain", origin="nilPropagation")); return
    # perfect square stays rational
    p, q = fa.numerator, fa.denominator
    r = _math.isqrt(p*q)
    if r*r == p*q and q != 0:
        # sqrt(p/q) rational only if both p and q perfect squares scaled
        sp = _math.isqrt(p); sq = _math.isqrt(q)
        if sp*sp == p and sq*sq == q:
            it.push(Rational(Fraction(sp, sq))); return
    it.push(Sqrt(fa))

def w_neg(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Nil): it.push(v); return
    fa = as_fraction(v)
    if fa is None: raise AjisaiError("structureError", "NEG")
    it.push(Rational(-fa))

def w_abs(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Nil): it.push(v); return
    fa = as_fraction(v)
    if fa is None: raise AjisaiError("structureError", "ABS")
    it.push(Rational(abs(fa)))

def _sort_cmp(a, b):
    if isinstance(a, Str) and isinstance(b, Str):
        return -1 if a.s < b.s else (1 if a.s > b.s else 0)
    return cmp_values(a, b)

def w_sort(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if not isinstance(v, Vec):
        raise AjisaiError("structureError", "SORT: expected vector")
    import functools
    items = sorted(v.items, key=functools.cmp_to_key(_sort_cmp))
    it.push(Vec(items))

# MATH scalar utilities ------------------------------------------------------

def _first_nil(ops):
    for o in ops:
        if isinstance(o, Nil):
            return o
    return None

def w_math_sign(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Nil): it.push(v); return
    fa = as_fraction(v)
    if fa is None: raise AjisaiError("custom", "SIGN: expected a number")
    it.push(Rational(Fraction(-1 if fa < 0 else (1 if fa > 0 else 0))))

def _minmax_word(name, pick):
    def impl(it, mods):
        ops, keep = it.operands(mods, 2)
        nil = _first_nil(ops[-2:])
        if nil is not None: it.push(nil); return
        fa, fb = as_fraction(ops[-2]), as_fraction(ops[-1])
        if fa is None or fb is None:
            raise AjisaiError("custom", f"{name}: expected two numbers")
        it.push(Rational(pick(fa, fb)))
    return impl

def w_math_pow(it, mods):
    ops, keep = it.operands(mods, 2)
    nil = _first_nil(ops[-2:])
    if nil is not None: it.push(nil); return
    fa, fb = as_fraction(ops[-2]), as_fraction(ops[-1])
    if fa is None or fb is None:
        raise AjisaiError("custom", "POW: expected two numbers")
    if fb.denominator != 1:
        raise AjisaiError("custom", "POW: exponent must be an integer")
    e = int(fb)
    if e < 0 and fa == 0:
        it.push(Nil(reason="divisionByZero", origin="executionFailure")); return
    it.push(Rational(fa ** e))

def _gcd_lcm_word(name, fn):
    def impl(it, mods):
        ops, keep = it.operands(mods, 2)
        nil = _first_nil(ops[-2:])
        if nil is not None: it.push(nil); return
        fa, fb = as_fraction(ops[-2]), as_fraction(ops[-1])
        if fa is None or fb is None or fa.denominator != 1 or fb.denominator != 1:
            raise AjisaiError("custom", f"{name}: expected two integers")
        it.push(Rational(Fraction(fn(abs(int(fa)), abs(int(fb))))))
    return impl

# MATH interval words (exact-rational interval arithmetic) -------------------

def _value_to_interval(v):
    if isinstance(v, Rational):
        return Interval(v.f, v.f)
    if isinstance(v, Interval):
        return v
    return None

def w_math_interval(it, mods):
    ops, keep = it.operands(mods, 2)
    lo_v, hi_v = ops[-2], ops[-1]
    if not isinstance(lo_v, Rational):
        raise AjisaiError("custom", "INTERVAL: lower bound must be scalar")
    if not isinstance(hi_v, Rational):
        raise AjisaiError("custom", "INTERVAL: upper bound must be scalar")
    if lo_v.f > hi_v.f:
        raise AjisaiError("custom", "invalid interval: lo must be <= hi")
    it.push(Interval(lo_v.f, hi_v.f))

def _interval_accessor(pick):
    def impl(it, mods):
        ops, keep = it.operands(mods, 1)
        iv = _value_to_interval(ops[-1])
        if iv is None:
            raise AjisaiError(
                "custom", "interval accessor: expected Number or Interval")
        it.push(Rational(pick(iv)))
    return impl

def w_math_is_exact(it, mods):
    ops, keep = it.operands(mods, 1)
    iv = _value_to_interval(ops[-1])
    if iv is None:
        raise AjisaiError("custom", "IS_EXACT: expected Number or Interval")
    it.push(TRUE if iv.lo == iv.hi else FALSE)

def _exact_rational_sqrt(q: Fraction):
    if q < 0:
        return None
    sp = _math.isqrt(q.numerator); sq = _math.isqrt(q.denominator)
    if sp * sp == q.numerator and sq * sq == q.denominator:
        return Fraction(sp, sq)
    return None

def _sqrt_rational_interval(q: Fraction, eps: Fraction):
    """Mirror of the Rust bisection: lo=0, hi=max(q,1), halve until <= eps."""
    if q < 0:
        raise AjisaiError("custom", "sqrt of negative value")
    if q == 0:
        return Interval(Fraction(0), Fraction(0))
    exact = _exact_rational_sqrt(q)
    if exact is not None:
        return Interval(exact, exact)
    if eps <= 0:
        raise AjisaiError("custom", "sqrt precision must be positive")
    lo = Fraction(0)
    hi = q if q >= 1 else Fraction(1)
    while hi - lo > eps:
        mid = (lo + hi) / 2
        if mid * mid <= q:
            lo = mid
        else:
            hi = mid
    return Interval(lo, hi)

def _sqrt_interval_with_eps(iv: Interval, eps: Fraction):
    if iv.hi < 0:
        raise AjisaiError("custom", "sqrt of negative value")
    if iv.lo < 0:
        hi = _sqrt_rational_interval(iv.hi, eps)
        return Interval(Fraction(0), hi.hi)
    lo = _sqrt_rational_interval(iv.lo, eps)
    hi = _sqrt_rational_interval(iv.hi, eps)
    return Interval(lo.lo, hi.hi)

def w_math_sqrt_eps(it, mods):
    ops, keep = it.operands(mods, 2)
    iv = _value_to_interval(ops[-2])
    if iv is None:
        raise AjisaiError(
            "custom", "SQRT_EPS: expected Number or Interval as first arg")
    if not isinstance(ops[-1], Rational):
        raise AjisaiError("custom", "SQRT_EPS: eps must be scalar rational")
    result = _sqrt_interval_with_eps(iv, ops[-1].f)
    it.push(Rational(result.lo) if result.lo == result.hi else result)

# ALGO words -----------------------------------------------------------------

def _algo_eq(a, b):
    if isinstance(a, Nil) and isinstance(b, Nil):
        return True
    return value_equal(a, b) is True

def w_algo_unique(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if not isinstance(v, Vec):
        raise AjisaiError("structureError", "UNIQUE: expected vector")
    out = []
    for x in v.items:
        if not any(_algo_eq(x, y) for y in out):
            out.append(x)
    it.push(Vec(out))

def w_algo_contains(it, mods):
    ops, keep = it.operands(mods, 2)
    v, needle = ops[-2], ops[-1]
    if not isinstance(v, Vec):
        raise AjisaiError(
            "structureError", "CONTAINS: expected vector as first operand")
    it.push(TRUE if any(_algo_eq(x, needle) for x in v.items) else FALSE)

def w_algo_index_of(it, mods):
    ops, keep = it.operands(mods, 2)
    v, needle = ops[-2], ops[-1]
    if not isinstance(v, Vec):
        raise AjisaiError(
            "structureError", "INDEX-OF: expected vector as first operand")
    for i, x in enumerate(v.items):
        if _algo_eq(x, needle):
            it.push(Rational(Fraction(i))); return
    it.push(Nil(reason="missingField", origin="executionFailure"))

# TIME words (exact, timezone-free civil calendar; §9.1) ---------------------
# days_from_civil / civil_from_days follow the standard proleptic-Gregorian
# arithmetic (Howard Hinnant's algorithms), mirroring rust time_calendar.

def _days_from_civil(y, m, d):
    y -= m <= 2
    era = (y if y >= 0 else y - 399) // 400
    yoe = y - era * 400
    doy = (153 * (m + (-3 if m > 2 else 9)) + 2) // 5 + d - 1
    doe = yoe * 365 + yoe // 4 - yoe // 100 + doy
    return era * 146097 + doe - 719468

def _civil_from_days(z):
    z += 719468
    era = (z if z >= 0 else z - 146096) // 146097
    doe = z - era * 146097
    yoe = (doe - doe // 1460 + doe // 36524 - doe // 146096) // 365
    y = yoe + era * 400
    doy = doe - (365 * yoe + yoe // 4 - yoe // 100)
    mp = (5 * doy + 2) // 153
    d = doy - (153 * mp + 2) // 5 + 1
    m = mp + (3 if mp < 10 else -9)
    return (y + (m <= 2), m, d)

def _is_leap(y):
    return y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)

def _days_in_month(y, m):
    if m == 2:
        return 29 if _is_leap(y) else 28
    return 31 if m in (1, 3, 5, 7, 8, 10, 12) else 30

def _civil_components(v, word, lens):
    if not isinstance(v, Vec) or not all(
        isinstance(x, Rational) for x in v.items
    ):
        raise AjisaiError("custom", f"{word}: expected a civil vector")
    if len(v.items) not in lens:
        want = " or ".join(str(n) for n in sorted(lens))
        raise AjisaiError(
            "custom", f"{word}: civil vector must have {want} elements")
    return v.items

def _int_field(x, word, what):
    if not isinstance(x, Rational):
        raise AjisaiError("custom", f"{word}: {what} must be a number")
    if x.f.denominator != 1:
        raise AjisaiError("custom", f"{word}: {what} must be an integer")
    return int(x.f)

def _scalar_field(x, word, what):
    if not isinstance(x, Rational):
        raise AjisaiError("custom", f"{word}: {what} must be a number")
    return x.f

def _civil_date_fields(comps, word):
    return (_int_field(comps[0], word, "year"),
            _int_field(comps[1], word, "month"),
            _int_field(comps[2], word, "day"))

def _rvec(nums):
    return Vec([Rational(Fraction(n)) for n in nums])

def w_time_datetime(it, mods):
    ops, keep = it.operands(mods, 2)
    if not isinstance(ops[-2], Rational):
        raise AjisaiError("custom", "DATETIME: timestamp must be a number")
    if not isinstance(ops[-1], Rational):
        raise AjisaiError("custom", "DATETIME: offset must be a number")
    local = ops[-2].f + ops[-1].f * 3600
    days = _math.floor(local / 86400)
    rem = local - days * 86400
    hour = int(rem // 3600); rem -= hour * 3600
    minute = int(rem // 60); second = rem - minute * 60
    y, m, d = _civil_from_days(days)
    it.push(Vec([Rational(Fraction(y)), Rational(Fraction(m)),
                 Rational(Fraction(d)), Rational(Fraction(hour)),
                 Rational(Fraction(minute)), Rational(second)]))

def w_time_timestamp(it, mods):
    ops, keep = it.operands(mods, 2)
    comps = _civil_components(ops[-2], "TIMESTAMP", {6})
    if not isinstance(ops[-1], Rational):
        raise AjisaiError("custom", "TIMESTAMP: offset must be a number")
    y, m, d = _civil_date_fields(comps, "TIMESTAMP")
    h = _int_field(comps[3], "TIMESTAMP", "hour")
    mi = _int_field(comps[4], "TIMESTAMP", "minute")
    s = _scalar_field(comps[5], "TIMESTAMP", "second")
    instant = (Fraction(_days_from_civil(y, m, d)) * 86400
               + h * 3600 + mi * 60 + s - ops[-1].f * 3600)
    it.push(Rational(instant))

def w_time_date(it, mods):
    ops, keep = it.operands(mods, 1)
    comps = _civil_components(ops[-1], "DATE", {6})
    it.push(Vec(list(comps[0:3])))

def w_time_time(it, mods):
    ops, keep = it.operands(mods, 1)
    comps = _civil_components(ops[-1], "TIME", {6})
    it.push(Vec(list(comps[3:6])))

def _time_field_word(word, date_index):
    """YEAR/MONTH/DAY read indices 0-2 of a date or datetime."""
    def impl(it, mods):
        ops, keep = it.operands(mods, 1)
        comps = _civil_components(ops[-1], word, {3, 6})
        it.push(comps[date_index])
    return impl

def _time_of_day_word(word, tod_index):
    """HOUR/MINUTE/SECOND read the time fields of a time or datetime."""
    def impl(it, mods):
        ops, keep = it.operands(mods, 1)
        comps = _civil_components(ops[-1], word, {3, 6})
        it.push(comps[tod_index + 3] if len(comps) == 6 else comps[tod_index])
    return impl

def w_time_weekday(it, mods):
    ops, keep = it.operands(mods, 1)
    comps = _civil_components(ops[-1], "WEEKDAY", {3, 6})
    y, m, d = _civil_date_fields(comps, "WEEKDAY")
    it.push(Rational(Fraction((_days_from_civil(y, m, d) + 3) % 7 + 1)))

def w_time_add_days(it, mods):
    ops, keep = it.operands(mods, 2)
    comps = _civil_components(ops[-2], "ADD-DAYS", {3, 6})
    if not isinstance(ops[-1], Rational):
        raise AjisaiError("custom", "ADD-DAYS: day count must be a number")
    if ops[-1].f.denominator != 1:
        raise AjisaiError("custom", "ADD-DAYS: day count must be an integer")
    y, m, d = _civil_date_fields(comps, "ADD-DAYS")
    y2, m2, d2 = _civil_from_days(_days_from_civil(y, m, d) + int(ops[-1].f))
    out = [Rational(Fraction(y2)), Rational(Fraction(m2)), Rational(Fraction(d2))]
    if len(comps) == 6:
        out += list(comps[3:6])
    it.push(Vec(out))

def w_time_diff_days(it, mods):
    ops, keep = it.operands(mods, 2)
    ca = _civil_components(ops[-2], "DIFF-DAYS", {3, 6})
    cb = _civil_components(ops[-1], "DIFF-DAYS", {3, 6})
    ya, ma, da = _civil_date_fields(ca, "DIFF-DAYS")
    yb, mb, db = _civil_date_fields(cb, "DIFF-DAYS")
    it.push(Rational(Fraction(
        _days_from_civil(ya, ma, da) - _days_from_civil(yb, mb, db))))

def w_time_format(it, mods):
    ops, keep = it.operands(mods, 1)
    comps = _civil_components(ops[-1], "FORMAT", {3, 6})
    y, m, d = _civil_date_fields(comps, "FORMAT")
    text = f"{y:04d}-{m:02d}-{d:02d}"
    if len(comps) == 6:
        h = _int_field(comps[3], "FORMAT", "hour")
        mi = _int_field(comps[4], "FORMAT", "minute")
        s = _scalar_field(comps[5], "FORMAT", "second")
        text += f"T{h:02d}:{mi:02d}:{int(s):02d}"
    it.push(Str(text))

_ISO_RE = None

def w_time_parse_iso(it, mods):
    import re
    global _ISO_RE
    if _ISO_RE is None:
        _ISO_RE = re.compile(
            r"^(\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2}):(\d{2}))?$")
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if not isinstance(v, Str):
        raise AjisaiError("custom", "PARSE: expected an ISO-8601 text value")
    m = _ISO_RE.match(v.s)
    if not m:
        it.push(Nil(reason="invalidEncoding", origin="invalidEncoding")); return
    parts = [int(m.group(1)), int(m.group(2)), int(m.group(3)),
             int(m.group(4) or 0), int(m.group(5) or 0), int(m.group(6) or 0)]
    it.push(_rvec(parts))

def _time_add_months_civil(y, m, d, n):
    total = y * 12 + (m - 1) + n
    y2, m2 = total // 12, total % 12 + 1
    return y2, m2, min(d, _days_in_month(y2, m2))

def _add_months_word(word, factor):
    def impl(it, mods):
        ops, keep = it.operands(mods, 2)
        comps = _civil_components(ops[-2], word, {3, 6})
        if not isinstance(ops[-1], Rational) or ops[-1].f.denominator != 1:
            raise AjisaiError("custom", f"{word}: count must be an integer")
        y, m, d = _civil_date_fields(comps, word)
        y2, m2, d2 = _time_add_months_civil(y, m, d, int(ops[-1].f) * factor)
        out = [Rational(Fraction(y2)), Rational(Fraction(m2)),
               Rational(Fraction(d2))]
        if len(comps) == 6:
            out += list(comps[3:6])
        it.push(Vec(out))
    return impl

# CRYPTO@HASH (pure multi-prime polynomial hash) -----------------------------

_HASH_P1 = 170141183460469231731687303715884105727
_HASH_P2 = 170141183460469231731687303715884105655
_HASH_P3 = 170141183460469231731687303715884104993

def _hash_serialize(v, out: bytearray):
    if isinstance(v, Nil):
        out.append(0x06); return
    if isinstance(v, Bool) and v.v in (True, False):
        out.append(0x07); out.append(0x01 if v.v else 0x00); return
    if isinstance(v, Rational):
        out.append(0x01)
        num, den = v.f.numerator, v.f.denominator
        out.append(0x00 if num < 0 else 0x01)
        for part in (abs(num), den):
            b = part.to_bytes(max(1, (part.bit_length() + 7) // 8), "little")
            out.extend(len(b).to_bytes(4, "little")); out.extend(b)
        return
    if isinstance(v, Str):
        # Text is a code-point vector at the value level (Section 4.3).
        out.append(0x04)
        out.extend(len(v.s).to_bytes(4, "little"))
        for ch in v.s:
            _hash_serialize(Rational(Fraction(ord(ch))), out)
        return
    if isinstance(v, (Vec, Rec)):
        out.append(0x04)
        out.extend(len(v.items).to_bytes(4, "little"))
        for x in v.items:
            _hash_serialize(x, out)

def _hash_poly(data: bytes, prime: int):
    h, power = 0, 1
    for byte in data:
        h = (h + power * byte) % prime
        power = (power * 257) % prime
    return h

def _hash_bits_of(v):
    if isinstance(v, Rational):
        f = v.f
    elif isinstance(v, Vec) and len(v.items) == 1 and isinstance(v.items[0], Rational):
        f = v.items[0].f
    else:
        return None
    if f.denominator != 1 or f <= 0 or int(f) > 0xFFFFFFFF:
        return None
    return int(f)

def w_crypto_hash(it, mods):
    keep = "KEEP" in mods
    if not it.stack:
        raise AjisaiError("custom", "HASH requires a value to hash")
    if keep:
        target = it.stack[-1]
        bits = (_hash_bits_of(it.stack[-2])
                if len(it.stack) >= 2 else None) or 256
    else:
        target = it.stack.pop()
        bits = None
        if it.stack:
            bits = _hash_bits_of(it.stack[-1])
            if bits is not None:
                it.stack.pop()
        bits = bits or 256
    if bits < 32 or bits > 1024:
        raise AjisaiError("custom", "HASH: output bits must be between 32 and 1024")
    data = bytearray()
    _hash_serialize(target, data)
    h1 = _hash_poly(bytes(data), _HASH_P1)
    h2 = _hash_poly(bytes(data), _HASH_P2)
    h3 = _hash_poly(bytes(data), _HASH_P3)
    r = h1 + (h2 << 127) + (h3 << 254)
    r ^= r >> (bits // 3)
    r ^= r >> (bits * 2 // 3)
    r %= 1 << bits
    it.push(Vec([Rational(Fraction(r, 1 << bits))]))

# JSON module (pure words only; Section 12.1: the role of every produced
# value is decided at construction — a parsed object is a Record rendered
# structurally, its keys stay Text, and a failed parse is a reasoned
# Bubble/NIL (invalidEncoding, Section 11.2) with no output effect.

def _json_to_value(j):
    """Mirror of the Rust `json_to_arena_node` mapping. Objects keep their
    keys in canonical sorted order (serde_json uses an ordered map)."""
    if j is None: return Nil()
    if isinstance(j, bool): return TRUE if j else FALSE
    if isinstance(j, int): return Rational(Fraction(j))
    if isinstance(j, float): return Rational(Fraction(str(j)))
    if isinstance(j, str): return Str(j)
    if isinstance(j, list):
        if not j: return Nil()
        return Vec([_json_to_value(x) for x in j])
    # dict: Record as a vector of [key, value] pairs, keys sorted
    if not j: return Nil()
    return Rec([Vec([Str(k), _json_to_value(v)]) for k, v in sorted(j.items())])

def _value_to_json(v):
    """Mirror of the Rust `arena_node_to_json` mapping (record -> object)."""
    if isinstance(v, Nil): return None
    if isinstance(v, Bool): return bool(v.v) if v.v != "U" else "unknown"
    if isinstance(v, Rational):
        f = v.f
        return int(f) if f.denominator == 1 else f.numerator / f.denominator
    if isinstance(v, Str): return v.s
    if isinstance(v, Rec):
        out = {}
        for pair in v.items:
            if isinstance(pair, Vec) and len(pair.items) == 2 and isinstance(pair.items[0], Str):
                out[pair.items[0].s] = _value_to_json(pair.items[1])
        return out
    if isinstance(v, Vec): return [_value_to_json(x) for x in v.items]
    return None

def _record_pairs(v):
    """[key, value] pairs of an object value (canonical Rec or raw Vec form)."""
    if isinstance(v, (Rec, Vec)):
        return [p for p in v.items
                if isinstance(p, Vec) and len(p.items) == 2 and isinstance(p.items[0], Str)]
    return None

def w_json_parse(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    text = v.s if isinstance(v, Str) else display(v)
    try:
        it.push(_json_to_value(json.loads(text)))
    except ValueError:
        it.push(Nil(reason="invalidEncoding", origin="invalidEncoding"))

def w_json_stringify(it, mods):
    ops, keep = it.operands(mods, 1)
    it.push(Str(json.dumps(_value_to_json(ops[-1]), sort_keys=True,
                           separators=(",", ":"))))

def w_json_get(it, mods):
    ops, keep = it.operands(mods, 2)
    obj, key = ops[-2], ops[-1]
    key_s = key.s if isinstance(key, Str) else display(key)
    pairs = _record_pairs(obj)
    if pairs is not None:
        for p in pairs:
            if p.items[0].s == key_s:
                it.push(p.items[1]); return
    it.push(Nil())

def w_json_keys(it, mods):
    ops, keep = it.operands(mods, 1)
    pairs = _record_pairs(ops[-1])
    if pairs:
        it.push(Vec([p.items[0] for p in pairs]))
    else:
        it.push(Nil())

def _key_text(key):
    return key.s if isinstance(key, Str) else display(key)

def _pairs_as_sorted_rec(mapping):
    return Rec([Vec([Str(k), mapping[k]]) for k in sorted(mapping)])

def _pairs_dict(v):
    pairs = _record_pairs(v)
    if pairs is None:
        return None
    return {p.items[0].s: p.items[1] for p in pairs}

def w_json_set(it, mods):
    ops, keep = it.operands(mods, 3)
    obj, key, val = ops[-3], ops[-2], ops[-1]
    mapping = _pairs_dict(obj) or {}
    mapping[_key_text(key)] = val
    it.push(_pairs_as_sorted_rec(mapping))

def w_json_has(it, mods):
    ops, keep = it.operands(mods, 2)
    mapping = _pairs_dict(ops[-2]) or {}
    it.push(TRUE if _key_text(ops[-1]) in mapping else FALSE)

def w_json_values(it, mods):
    ops, keep = it.operands(mods, 1)
    pairs = _record_pairs(ops[-1])
    if pairs:
        it.push(Vec([p.items[1] for p in pairs]))
    else:
        it.push(Nil())

def w_json_merge(it, mods):
    ops, keep = it.operands(mods, 2)
    mapping = _pairs_dict(ops[-2]) or {}
    mapping.update(_pairs_dict(ops[-1]) or {})
    it.push(_pairs_as_sorted_rec(mapping))

def w_json_delete(it, mods):
    ops, keep = it.operands(mods, 2)
    obj, key = ops[-2], ops[-1]
    if isinstance(obj, Nil):
        it.push(obj); return
    mapping = _pairs_dict(obj) or {}
    mapping.pop(_key_text(key), None)
    it.push(_pairs_as_sorted_rec(mapping))

MODULE_IMPL = {
    # MATH
    "SQRT": w_sqrt, "NEG": w_neg, "ABS": w_abs, "SIGN": w_math_sign,
    "MIN": _minmax_word("MIN", min), "MAX": _minmax_word("MAX", max),
    "POW": w_math_pow,
    "GCD": _gcd_lcm_word("GCD", _math.gcd), "LCM": _gcd_lcm_word("LCM", _math.lcm),
    "INTERVAL": w_math_interval,
    "LOWER": _interval_accessor(lambda iv: iv.lo),
    "UPPER": _interval_accessor(lambda iv: iv.hi),
    "WIDTH": _interval_accessor(lambda iv: iv.hi - iv.lo),
    "IS-EXACT": w_math_is_exact, "SQRT-EPS": w_math_sqrt_eps,
    # ALGO
    "SORT": w_sort, "UNIQUE": w_algo_unique,
    "CONTAINS": w_algo_contains, "INDEX-OF": w_algo_index_of,
    # JSON
    "PARSE": w_json_parse, "STRINGIFY": w_json_stringify,
    "GET": w_json_get, "KEYS": w_json_keys, "SET": w_json_set,
    "HAS": w_json_has, "VALUES": w_json_values, "MERGE": w_json_merge,
    "DELETE": w_json_delete,
    # TIME (pure civil words; the Hosted TIME@NOW is out of Core scope)
    "DATETIME": w_time_datetime, "TIMESTAMP": w_time_timestamp,
    "DATE": w_time_date, "TIME": w_time_time,
    "YEAR": _time_field_word("YEAR", 0),
    "MONTH": _time_field_word("MONTH", 1),
    "DAY": _time_field_word("DAY", 2),
    "HOUR": _time_of_day_word("HOUR", 0),
    "MINUTE": _time_of_day_word("MINUTE", 1),
    "SECOND": _time_of_day_word("SECOND", 2),
    "WEEKDAY": w_time_weekday,
    "ADD-DAYS": w_time_add_days, "DIFF-DAYS": w_time_diff_days,
    "FORMAT": w_time_format, "PARSE-ISO": w_time_parse_iso,
    "ADD-MONTHS": _add_months_word("ADD-MONTHS", 1),
    "ADD-YEARS": _add_months_word("ADD-YEARS", 12),
    # CRYPTO
    "HASH": w_crypto_hash,
}
MODULE_WORDS = {
    "SQRT": "MATH", "NEG": "MATH", "ABS": "MATH", "SIGN": "MATH",
    "MIN": "MATH", "MAX": "MATH", "POW": "MATH", "GCD": "MATH", "LCM": "MATH",
    "INTERVAL": "MATH", "LOWER": "MATH", "UPPER": "MATH", "WIDTH": "MATH",
    "IS-EXACT": "MATH", "SQRT-EPS": "MATH",
    "SORT": "ALGO", "UNIQUE": "ALGO", "CONTAINS": "ALGO", "INDEX-OF": "ALGO",
    "PARSE": "JSON", "STRINGIFY": "JSON", "GET": "JSON", "KEYS": "JSON",
    "SET": "JSON", "HAS": "JSON", "VALUES": "JSON", "MERGE": "JSON",
    "DELETE": "JSON",
    "DATETIME": "TIME", "TIMESTAMP": "TIME", "DATE": "TIME", "TIME": "TIME",
    "YEAR": "TIME", "MONTH": "TIME", "DAY": "TIME", "HOUR": "TIME",
    "MINUTE": "TIME", "SECOND": "TIME", "WEEKDAY": "TIME",
    "ADD-DAYS": "TIME", "DIFF-DAYS": "TIME", "FORMAT": "TIME",
    "PARSE-ISO": "TIME", "ADD-MONTHS": "TIME", "ADD-YEARS": "TIME",
    "HASH": "CRYPTO",
}
MODULES = {
    m: {w for w, mod in MODULE_WORDS.items() if mod == m}
    for m in set(MODULE_WORDS.values())
}
# Importable modules whose words are host-mediated and out of the Core
# reference's scope (PORTABILITY.md Core/Hosted split): IMPORT succeeds,
# their words stay unimplemented here.
for _hosted_mod in ("MUSIC", "SERIAL", "IO"):
    MODULES.setdefault(_hosted_mod, set())

# --------------------------------------------------------------------------
# CLI: run a file, emit compact JSON comparable to probe.py
# --------------------------------------------------------------------------

def run_program(src):
    it = Interp()
    try:
        it.run_source(src)
        return {"status": "ok",
                "stack": [display(v) for v in it.stack],
                "output": list(it.output)}
    except AjisaiError as e:
        return {"status": "error", "kind": e.kind}
    except RecursionError:
        # Native recursion-depth guard (SPEC Section 8.4): a runtime safety
        # control of the same rank as the step budget, with its own user-level
        # category (Section 11.1).
        return {"status": "error", "kind": "recursionLimitExceeded"}

if __name__ == "__main__":
    if len(sys.argv) >= 3 and sys.argv[1] == "run":
        src = open(sys.argv[2]).read()
        print(json.dumps(run_program(src)))
    else:
        for s in sys.argv[1:]:
            print(repr(s), "->", run_program(s))
