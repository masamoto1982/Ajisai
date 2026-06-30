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
class Block:
    """A CodeBlock: a list of source lines (Section 3.4, 4.6)."""
    lines: List[List[str]]            # each line is a token list

@dataclass
class Rational:
    f: Fraction

@dataclass
class Sqrt:
    """Lazy sqrt of a non-negative rational (Section 4.2.2 AlgebraicSqrt)."""
    radicand: Fraction               # value is sqrt(radicand)

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

CF_DISPLAY_BUDGET = 30

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
    # ( a0 ( a1 ( a2 ... ) ) )  ; truncated lazy CF ends with ...)
    s = "...)" if truncated else None
    # build right to left
    rev = list(reversed(terms))
    acc = ""
    if truncated:
        acc = "...)"
    for i, t in enumerate(rev):
        if acc == "":
            acc = f"( {t} )"
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

# For arithmetic we keep rationals exact; sqrt mixed with rationals is kept
# symbolic only where the spec's examples need it (sqrt2 - sqrt2 = 0).

def is_scalar(v): return isinstance(v, (Rational, Sqrt))

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
            canon = ALIAS.get(word, word).upper()
            if canon in ("TOP", "STAK", "EAT", "KEEP"):
                pending_mods.append(canon); idx += 1; continue
            # an actual operation word
            self.exec_word(canon, pending_mods)
            pending_mods = []
            idx += 1
        # trailing modifiers with no word: ignore (no-op markers)

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
        # module words available only if imported (we model MATH minimally)
        if w in MODULE_WORDS:
            home = MODULE_WORDS[w]
            if home in self.imported:
                MODULE_IMPL[w](self, mods); return
            raise AjisaiError("unknownWord", w)
        if "@" in w:
            mod, _, ww = w.partition("@")
            if ww in MODULE_IMPL and MODULE_WORDS.get(ww) == mod:
                MODULE_IMPL[ww](self, mods); return
            raise AjisaiError("unknownWord", w)
        raise AjisaiError("unknownWord", w)

# --------------------------------------------------------------------------
# Display (Section 12)
# --------------------------------------------------------------------------

def display(v):
    if isinstance(v, Rational):
        return f"{v.f.numerator}/{v.f.denominator}"
    if isinstance(v, Sqrt):
        return build_nested(sqrt_cf_terms(v.radicand, CF_DISPLAY_BUDGET), truncated=True)
    if isinstance(v, Bool):
        return {True: "TRUE", False: "FALSE", "U": "UNKNOWN"}[v.v]
    if isinstance(v, Nil):
        return "NIL"
    if isinstance(v, Str):
        return "'" + v.s + "'"
    if isinstance(v, Vec):
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

def binop_arith(name, fn):
    def impl(it: Interp, mods):
        ops, keep = it.operands(mods, 2)
        a, b = ops[-2], ops[-1]
        # NIL passthrough (Section 4.5.1 / 7.12)
        if isinstance(a, Nil) or isinstance(b, Nil):
            it.push(Nil(reason=leftmost_nil_reason([a, b]), origin="nilPropagation")); return
        fa, fb = as_fraction(a), as_fraction(b)
        if fa is None or fb is None:
            # Exact algebraic identities the spec's own examples rely on
            # (sqrt x - sqrt x = 0 ; sqrt x * sqrt x = x). A fuller port would
            # carry a general Gosper bihomographic engine here.
            if isinstance(a, Sqrt) and isinstance(b, Sqrt) and a.radicand == b.radicand:
                if name == "SUB":
                    it.push(Rational(Fraction(0))); return
                if name == "MUL":
                    it.push(Rational(a.radicand)); return
                if name == "ADD":
                    it.push(Sqrt(a.radicand * 4)); return  # 2*sqrt(x)=sqrt(4x)
            raise AjisaiError("structureError", f"{name} needs numbers")
        try:
            it.push(Rational(fn(fa, fb)))
        except ZeroDivisionError:
            it.push(Nil(reason="divisionByZero", origin="nilPropagation"))
    return impl

def w_div(it, mods):
    ops, keep = it.operands(mods, 2)
    a, b = ops[-2], ops[-1]
    if isinstance(a, Nil) or isinstance(b, Nil):
        it.push(Nil(reason=leftmost_nil_reason([a, b]), origin="nilPropagation")); return
    fa, fb = as_fraction(a), as_fraction(b)
    if fa is None or fb is None:
        raise AjisaiError("structureError", "DIV needs numbers")
    if fb == 0:
        it.push(Nil(reason="divisionByZero", origin="nilPropagation")); return
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
        it.push(Nil(reason="divisionByZero", origin="nilPropagation")); return
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
def _round_half_even(f):  # ROUND — spec says "round to nearest integer"
    return int(_math.floor(f + Fraction(1, 2)))   # round half up

# comparison ----------------------------------------------------------------

def cmp_sign(a: Fraction, b: Fraction):
    if a < b: return -1
    if a > b: return 1
    return 0

def cmp_values(a, b):
    """Return -1/0/1, or 'U' if undecidable. NIL handled by caller."""
    fa, fb = as_fraction(a), as_fraction(b)
    if fa is not None and fb is not None:
        return cmp_sign(fa, fb)
    # sqrt vs sqrt or sqrt vs rational: compare by CF prefix within budget
    BUD = 64
    ta = sqrt_cf_terms(a.radicand, BUD) if isinstance(a, Sqrt) else rcf_terms_rational(fa.numerator, fa.denominator)
    tb = sqrt_cf_terms(b.radicand, BUD) if isinstance(b, Sqrt) else rcf_terms_rational(fb.numerator, fb.denominator)
    for k in range(min(len(ta), len(tb))):
        if ta[k] != tb[k]:
            # CF comparison: even index -> larger term means larger value
            d = 1 if ta[k] > tb[k] else -1
            return d if k % 2 == 0 else -d
    # one is prefix of the other
    if len(ta) == len(tb):
        # identical within budget but lazy -> undecidable
        if isinstance(a, Sqrt) and isinstance(b, Sqrt) and a.radicand == b.radicand:
            return 0
        return "U"
    # finite (shorter) decided
    longer_a = len(ta) > len(tb)
    k = min(len(ta), len(tb))
    return "U"

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
    keep = "KEEP" in mods
    ops = list(it.stack)
    if not keep: it.stack = []
    # NIL priority
    if any(isinstance(o, Nil) for o in ops):
        it.push(Nil(reason=leftmost_nil_reason(ops), origin="nilPropagation")); return
    # sequence property over adjacent pairs (Section 7.4)
    result = TRUE
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
    it.push(result)

DECIDERS = {
    "LT": lambda s: s < 0, "LTE": lambda s: s <= 0,
    "GT": lambda s: s > 0, "GTE": lambda s: s >= 0,
    "EQ": lambda s: s == 0, "NEQ": lambda s: s != 0,
}

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

def w_vent(it, mods):
    # If top is NIL, replace with next value (Section 6.4)
    it.need(1)
    top = it.stack.pop()
    if isinstance(top, Nil):
        it.need(1)
        # replace: pop the fallback and push it
        fb = it.stack.pop()
        it.push(fb)
    else:
        it.push(top)

# vector words --------------------------------------------------------------

def w_length(it, mods):
    if "STAK" in mods:
        n = len(it.stack)
        if "KEEP" not in mods: pass  # LENGTH STAK counts all; spec: total count
        it.push(Rational(Fraction(n))); 
        if "KEEP" not in mods:
            # default EAT would consume operands; STAK total-count consumes all
            it.stack = it.stack[:-1] if False else it.stack
        return
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Vec):
        it.push(Rational(Fraction(len(v.items))))
    elif isinstance(v, Str):
        it.push(Rational(Fraction(len(v.s))))
    else:
        raise AjisaiError("structureError", "LENGTH needs a vector")

def norm_index(i, n):
    if i < 0: i += n
    return i

def w_get(it, mods):
    ops, keep = it.operands(mods, 2)
    vec, idx = ops[-2], ops[-1]
    if isinstance(vec, Nil) or isinstance(idx, Nil):
        it.push(Nil(reason=leftmost_nil_reason([vec, idx]), origin="nilPropagation")); return
    if not isinstance(vec, Vec): raise AjisaiError("structureError", "GET needs a vector")
    fi = as_fraction(idx)
    if fi is None or fi.denominator != 1: raise AjisaiError("structureError", "GET index")
    i = norm_index(int(fi), len(vec.items))
    if 0 <= i < len(vec.items):
        it.push(vec.items[i])
    else:
        it.push(Nil(reason="indexOutOfBounds", origin="nilPropagation"))

def w_concat(it, mods):
    if "STAK" in mods:
        ops = list(it.stack); 
        if "KEEP" not in mods: it.stack = []
        items = []
        for o in ops:
            if isinstance(o, Vec): items += o.items
            else: raise AjisaiError("structureError", "CONCAT needs vectors")
        it.push(Vec(items)); return
    ops, keep = it.operands(mods, 2)
    a, b = ops[-2], ops[-1]
    if isinstance(a, Vec) and isinstance(b, Vec):
        it.push(Vec(a.items + b.items)); return
    if isinstance(a, Str) and isinstance(b, Str):
        it.push(Str(a.s + b.s)); return
    raise AjisaiError("structureError", "CONCAT needs vectors")

def w_reverse(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Vec): it.push(Vec(list(reversed(v.items))))
    else: raise AjisaiError("structureError", "REVERSE needs a vector")

def w_range(it, mods):
    ops, keep = it.operands(mods, 2)
    a, b = ops[-2], ops[-1]
    fa, fb = as_fraction(a), as_fraction(b)
    if fa is None or fb is None or fa.denominator != 1 or fb.denominator != 1:
        raise AjisaiError("structureError", "RANGE needs integers")
    it.push(Vec([Rational(Fraction(x)) for x in range(int(fa), int(fb))]))

def w_take(it, mods):
    ops, keep = it.operands(mods, 2)
    v, k = ops[-2], ops[-1]
    if not isinstance(v, Vec): raise AjisaiError("structureError", "TAKE")
    fk = as_fraction(k); 
    if fk is None: raise AjisaiError("structureError", "TAKE")
    it.push(Vec(v.items[:int(fk)]))

def w_collect(it, mods):
    items = list(it.stack); it.stack = []
    it.push(Vec(items))

def w_insert(it, mods):
    ops, keep = it.operands(mods, 3)
    v, idx, val = ops[-3], ops[-2], ops[-1]
    if not isinstance(v, Vec): raise AjisaiError("structureError", "INSERT")
    i = norm_index(int(as_fraction(idx)), len(v.items))
    items = list(v.items); items.insert(i, val); it.push(Vec(items))

def w_replace(it, mods):
    ops, keep = it.operands(mods, 3)
    v, idx, val = ops[-3], ops[-2], ops[-1]
    if not isinstance(v, Vec): raise AjisaiError("structureError", "REPLACE")
    i = norm_index(int(as_fraction(idx)), len(v.items))
    if not (0 <= i < len(v.items)):
        it.push(Nil(reason="indexOutOfBounds", origin="nilPropagation")); return
    items = list(v.items); items[i] = val; it.push(Vec(items))

def w_remove(it, mods):
    ops, keep = it.operands(mods, 2)
    v, idx = ops[-2], ops[-1]
    if not isinstance(v, Vec): raise AjisaiError("structureError", "REMOVE")
    i = norm_index(int(as_fraction(idx)), len(v.items))
    if not (0 <= i < len(v.items)):
        it.push(Nil(reason="indexOutOfBounds", origin="nilPropagation")); return
    items = list(v.items); del items[i]; it.push(Vec(items))

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
    it.push(Vec([Rational(Fraction(x)) for x in shape_of(v)]))

def w_rank(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    it.push(Rational(Fraction(len(shape_of(v)))))

# string/conv ---------------------------------------------------------------

def w_str(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    it.push(Str(output_render(v) if not isinstance(v, Str) else v.s))

def w_num(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if not isinstance(v, Str): raise AjisaiError("structureError", "NUM needs text")
    n = number_of(v.s.strip())
    if n is None: it.push(Nil(reason="invalidEncoding", origin="nilPropagation")); return
    it.push(n)

def w_bool(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if isinstance(v, Bool): it.push(v); return
    fa = as_fraction(v)
    if fa is not None: it.push(TRUE if fa != 0 else FALSE); return
    raise AjisaiError("structureError", "BOOL")

def w_chr(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    fa = as_fraction(v)
    if fa is None or fa.denominator != 1:
        raise AjisaiError("structureError", "CHR needs integer")
    cp = int(fa)
    if cp < 0 or cp > 0x10FFFF or (0xD800 <= cp <= 0xDFFF):
        it.push(Nil(reason="invalidEncoding", origin="nilPropagation")); return
    it.push(Str(chr(cp)))

def w_chars(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if not isinstance(v, Str): raise AjisaiError("structureError", "CHARS")
    it.push(Vec([Str(c) for c in v.s]))

def w_join(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]
    if not isinstance(v, Vec): raise AjisaiError("structureError", "JOIN")
    it.push(Str("".join(x.s if isinstance(x, Str) else output_render(x) for x in v.items)))

# higher-order --------------------------------------------------------------

def run_block_on(it, block, value):
    it.push(value); it.run_block(block)

def w_map(it, mods):
    ops, keep = it.operands(mods, 2)
    vec, blk = ops[-2], ops[-1]
    if not isinstance(vec, Vec) or not isinstance(blk, Block):
        raise AjisaiError("structureError", "MAP")
    res = []
    for x in vec.items:
        sub = Interp(); sub.user_words = it.user_words; sub.imported = it.imported
        sub.push(x); sub.run_block(blk)
        res.append(sub.pop())
    it.push(Vec(res))

def w_filter(it, mods):
    ops, keep = it.operands(mods, 2)
    vec, blk = ops[-2], ops[-1]
    res = []
    for x in vec.items:
        sub = Interp(); sub.user_words = it.user_words; sub.imported = it.imported
        sub.push(x); sub.run_block(blk)
        r = sub.pop()
        if isinstance(r, Bool) and r.v is True: res.append(x)
    it.push(Vec(res))

def w_fold(it, mods):
    ops, keep = it.operands(mods, 3)
    vec, init, blk = ops[-3], ops[-2], ops[-1]
    acc = init
    for x in vec.items:
        sub = Interp(); sub.user_words = it.user_words; sub.imported = it.imported
        sub.push(acc); sub.push(x); sub.run_block(blk)
        acc = sub.pop()
    it.push(acc)

def w_exec(it, mods):
    ops, keep = it.operands(mods, 1)
    blk = ops[-1]
    if not isinstance(blk, Block): raise AjisaiError("structureError", "EXEC")
    it.run_block(blk)

def w_cond(it, mods):
    ops, keep = it.operands(mods, 1)
    blk = ops[-1]
    if not isinstance(blk, Block): raise AjisaiError("structureError", "COND")
    # each line inside is a clause: { guard | body } ; we stored as single line list
    # Re-tokenize: clauses are separated by '|' tokens on each line.
    clauses = split_cond_clauses(blk)
    for guard_toks, body_toks in clauses:
        if guard_toks is None:  # else clause (IDLE)
            it.run_tokens(body_toks); return
        # evaluate guard
        sub_before = len(it.stack)
        it.run_tokens(list(guard_toks))
        g = it.pop()
        if isinstance(g, Bool) and g.v is True:
            it.run_tokens(list(body_toks)); return
        # U or FALSE or NIL -> fall through
    raise AjisaiError("condExhausted")

def split_cond_clauses(blk):
    """Spec: clauses separated by | , each on one line. Our Block stores token
    lines. A clause is { guard | body } subblocks, OR inline guard | body."""
    clauses = []
    for line in blk.lines:
        # line is list of (kind,val). A clause may itself be a {..} sub-block.
        # Common form: each clause is a brace block containing  guard | body
        i = 0
        while i < len(line):
            kind, val = line[i]
            if kind == "sym" and val == "{":
                depth = 1; j = i+1; inner = []
                while j < len(line) and depth:
                    k2, v2 = line[j]
                    if k2 == "sym" and v2 == "{": depth += 1
                    elif k2 == "sym" and v2 == "}":
                        depth -= 1
                        if depth == 0: break
                    inner.append((k2, v2)); j += 1
                # split inner by top-level |
                clauses.append(parse_clause(inner))
                i = j + 1
            else:
                i += 1
    return clauses

def parse_clause(inner):
    # split by | at depth 0
    depth = 0; bar = None
    for idx, (k, v) in enumerate(inner):
        if k == "sym" and v in "[{": depth += 1
        elif k == "sym" and v in "]}": depth -= 1
        elif k == "sym" and v == "|" and depth == 0:
            bar = idx; break
    if bar is None:
        return (None, inner)   # else clause
    guard = inner[:bar]; body = inner[bar+1:]
    # IDLE guard -> else
    if len(guard) == 1 and guard[0][1].upper() == "IDLE":
        return (None, body)
    return (guard, body)

# def / del -----------------------------------------------------------------

def w_def(it, mods):
    name = it.pop(); body = it.pop()
    if not isinstance(name, Str): raise AjisaiError("structureError", "DEF name must be string")
    if not isinstance(body, Block): raise AjisaiError("structureError", "DEF body must be block")
    nm = name.s.upper()
    if nm in CORE: raise AjisaiError("builtinProtection")
    it.user_words[nm] = body

def w_del(it, mods):
    name = it.pop()
    nm = name.s.upper()
    if nm in it.user_words: del it.user_words[nm]
    else: raise AjisaiError("unknownWord")

def w_print(it, mods):
    keep = "KEEP" in mods
    it.need(1)
    v = it.stack[-1] if keep else it.stack.pop()
    it.output.append(output_render(v))

def w_import(it, mods):
    name = it.pop()
    if not isinstance(name, Str): raise AjisaiError("structureError")
    it.imported.add(name.s.upper())

CORE = {
    "ADD": binop_arith("ADD", lambda a, b: a + b),
    "SUB": binop_arith("SUB", lambda a, b: a - b),
    "MUL": binop_arith("MUL", lambda a, b: a * b),
    "DIV": w_div, "MOD": w_mod,
    "FLOOR": unary_round("FLOOR", lambda f: _math.floor(f)),
    "CEIL": unary_round("CEIL", lambda f: _math.ceil(f)),
    "ROUND": unary_round("ROUND", _round_half_even),
    "LT": comparison("LT", DECIDERS["LT"]), "LTE": comparison("LTE", DECIDERS["LTE"]),
    "GT": comparison("GT", DECIDERS["GT"]), "GTE": comparison("GTE", DECIDERS["GTE"]),
    "EQ": comparison("EQ", DECIDERS["EQ"]), "NEQ": comparison("NEQ", DECIDERS["NEQ"]),
    "AND": w_and, "OR": w_or, "NOT": w_not,
    "TRUE": w_true, "FALSE": w_false, "NIL": w_nil, "IDLE": w_idle,
    "FLOW": w_flow, "VENT": w_vent,
    "LENGTH": w_length, "GET": w_get, "CONCAT": w_concat, "REVERSE": w_reverse,
    "RANGE": w_range, "TAKE": w_take, "COLLECT": w_collect,
    "INSERT": w_insert, "REPLACE": w_replace, "REMOVE": w_remove,
    "SHAPE": w_shape, "RANK": w_rank,
    "STR": w_str, "NUM": w_num, "BOOL": w_bool, "CHR": w_chr, "CHARS": w_chars, "JOIN": w_join,
    "MAP": w_map, "FILTER": w_filter, "FOLD": w_fold, "EXEC": w_exec, "COND": w_cond,
    "DEF": w_def, "DEL": w_del, "PRINT": w_print, "IMPORT": w_import,
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
    v = ops[-1]; fa = as_fraction(v)
    if fa is None: raise AjisaiError("structureError", "NEG")
    it.push(Rational(-fa))

def w_abs(it, mods):
    ops, keep = it.operands(mods, 1)
    v = ops[-1]; fa = as_fraction(v)
    if fa is None: raise AjisaiError("structureError", "ABS")
    it.push(Rational(abs(fa)))

MODULE_IMPL = {"SQRT": w_sqrt, "NEG": w_neg, "ABS": w_abs}
MODULE_WORDS = {"SQRT": "MATH", "NEG": "MATH", "ABS": "MATH"}

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
        return {"status": "error", "kind": "executionLimitExceeded"}

if __name__ == "__main__":
    if len(sys.argv) >= 3 and sys.argv[1] == "run":
        src = open(sys.argv[2]).read()
        print(json.dumps(run_program(src)))
    else:
        for s in sys.argv[1:]:
            print(repr(s), "->", run_program(s))
