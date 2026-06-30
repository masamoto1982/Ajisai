"""The Ajisai interpreter: stack, modifiers, words, modules (Sections 5-11).

Written from SPECIFICATION.html alone. Where the spec is ambiguous the chosen
behaviour is marked with a ``SPEC-GAP`` comment and recorded in SPEC_GAPS.md.
"""

from __future__ import annotations

import hashlib
import secrets
import time
from fractions import Fraction
from typing import List, Optional

from . import errors as err
from .lexer import Block, parse
from .numbers import AlgebraicReal, DomainLimitation
from .values import (CONTINUED_FRACTION, RAW_NUMBER, TEXT, Boolean, CodeBlock,
                     Nil, ProcessHandle, Record, Scalar, SupervisorHandle,
                     Unknown, Value, Vector, is_nil, is_text, make_text,
                     render, text_to_str)

DEFAULT_STEP_LIMIT = 100_000
COMPARISON_BUDGET = 64  # implementation-defined (Section 7.4.1)

MODULE_SPECS = {"MUSIC", "JSON", "IO", "TIME", "CRYPTO", "ALGO", "MATH", "SERIAL"}


def _frac(s: Scalar) -> Fraction:
    return s.num.rational_value()


class ChildRuntime:
    def __init__(self, state="completed", result_stack=None, error=None):
        self.state = state
        self.result_stack = result_stack or []
        self.error = error
        self.monitored = False


class Interp:
    def __init__(self, step_limit=DEFAULT_STEP_LIMIT):
        self.stack: List[Value] = []
        self.user_dict = {}            # name -> CodeBlock (definition)
        self.user_deps = {}            # name -> set(referenced names)
        self.imported = []             # module names imported (order matters)
        self.output: List[str] = []    # host effect log (pi_Output)
        self.input_buffer: List[str] = []
        self.step_limit = step_limit
        self.steps = 0
        self.warnings: List[str] = []

    # -- stack helpers ----------------------------------------------------
    def push(self, v: Value):
        self.stack.append(v)

    def pop(self) -> Value:
        if not self.stack:
            raise err.StackUnderflow("stack underflow")
        return self.stack.pop()

    def pop_n(self, n: int) -> List[Value]:
        if len(self.stack) < n:
            raise err.StackUnderflow(f"need {n} operands")
        if n == 0:
            return []
        items = self.stack[-n:]
        del self.stack[-n:]
        return items

    def _tick(self):
        self.steps += 1
        if self.steps > self.step_limit:
            raise err.ExecutionLimitExceeded("step budget exhausted")

    # -- evaluation -------------------------------------------------------
    def run_source(self, src: str):
        block = parse(src)
        self.run_block(block)

    def run_block(self, block: Block):
        for line in block.lines:
            self.run_line(line)

    def run_line(self, nodes: list):
        pending_target = None
        pending_consume = None
        for node in nodes:
            kind = node[0]
            if kind == "mod":
                t, c = node[1]
                if t:
                    pending_target = t
                if c:
                    pending_consume = c
                continue
            if kind == "num":
                self.push(node[1])
                continue
            if kind == "str":
                self.push(make_text(node[1]))
                continue
            if kind == "vec":
                self.push(self._build_vector(node[1]))
                continue
            if kind == "block":
                self.push(self._build_block(node[1]))
                continue
            if kind == "pipe":
                # top-level | outside a COND clause has no meaning
                raise err.StructureError("'|' is only valid inside a COND clause")
            if kind == "word":
                name = node[1]
                if name in ("TOP", "STAK"):
                    pending_target = name
                    continue
                if name in ("EAT", "KEEP"):
                    pending_consume = name
                    continue
                if name == "FLOW":  # visual pipeline marker, no-op
                    continue
                self._tick()
                self.exec_word(name, pending_target or "TOP", pending_consume or "EAT")
                pending_target = None
                pending_consume = None
                continue
            raise err.StructureError(f"bad node {node!r}")

    def _build_vector(self, item_nodes) -> Vector:
        sub = Interp(self.step_limit)
        sub.user_dict = self.user_dict
        sub.user_deps = self.user_deps
        sub.imported = self.imported
        sub.run_line(list(item_nodes))
        return Vector(list(sub.stack))

    def _build_block(self, body: Block) -> CodeBlock:
        src = " | ".join(
            " ".join(_node_src(n) for n in line) for line in body.lines
        )
        return CodeBlock(body.lines, source=src)

    # -- word resolution --------------------------------------------------
    def exec_word(self, name: str, target: str, consume: str):
        if "@" in name:
            mod, _, w = name.partition("@")
            if mod not in MODULE_SPECS:
                raise err.UnknownModule(f"unknown module {mod}")
            if mod not in self.imported:
                raise err.UnknownWord(f"{name}: module {mod} not imported")
            fn = MODULE_WORDS.get(mod, {}).get(w)
            if fn is None:
                raise err.UnknownWord(f"unknown word {name}")
            fn(self, target, consume)
            return
        # bare name: Core first, then imported modules, then user dict
        if name in CORE_WORDS:
            CORE_WORDS[name](self, target, consume)
            return
        for mod in self.imported:
            if name in MODULE_WORDS.get(mod, {}):
                MODULE_WORDS[mod][name](self, target, consume)
                return
        if name in self.user_dict:
            self._call_user(name, target, consume)
            return
        raise err.UnknownWord(f"unknown word {name}")

    def _call_user(self, name, target, consume):
        block = self.user_dict[name]
        # modifiers do not generally apply to user words; execute body
        self.run_block(Block(block.lines))


# ===========================================================================
# Generic operand dispatch for fixed-arity numeric / comparison / logic words
# ===========================================================================

def _nil_inherit(operands) -> Nil:
    for o in operands:
        if is_nil(o) and o.reason is not None:
            return Nil(reason=o.reason, origin="nilPropagation")
    return Nil(origin="nilPropagation")


def binary_numeric(interp: Interp, target, consume, fn, passthrough=True):
    """ADD SUB MUL DIV MOD-style fixed binary word with modifier handling."""
    if target == "STAK":
        n = _stak_count(interp)
        operands = interp.pop_n(n)
        if passthrough and any(is_nil(o) for o in operands):
            result = _nil_inherit(operands)
        else:
            if n == 0:
                raise err.StackUnderflow("STAK fold needs operands")
            acc = operands[0]
            for o in operands[1:]:
                acc = fn(interp, acc, o)
            result = acc
        if consume == "KEEP":
            interp.stack.extend(operands)
        interp.push(result)
        return
    # TOP
    operands = interp.pop_n(2)
    a, b = operands
    if consume == "KEEP":
        interp.stack.extend(operands)
    if passthrough and (is_nil(a) or is_nil(b)):
        interp.push(_nil_inherit(operands))
        return
    interp.push(fn(interp, a, b))


def unary_numeric(interp: Interp, target, consume, fn, passthrough=True):
    a = interp.pop()
    if consume == "KEEP":
        interp.push(a)
    if passthrough and is_nil(a):
        interp.push(_nil_inherit([a]))
        return
    interp.push(fn(interp, a))


def _stak_count(interp: Interp) -> int:
    c = interp.pop()
    if not isinstance(c, Scalar) or not c.num.is_integer() or c.num.sign() < 0:
        raise err.StructureError("STAK requires a non-negative integer count")
    return int(c.num.rational_value())


def _need_scalar(v: Value) -> Scalar:
    if not isinstance(v, Scalar):
        raise err.StructureError("numeric operand required")
    return v


# -- arithmetic ops ----------------------------------------------------------

def _add(interp, a, b):
    return Scalar(_need_scalar(a).num + _need_scalar(b).num)


def _sub(interp, a, b):
    return Scalar(_need_scalar(a).num - _need_scalar(b).num)


def _mul(interp, a, b):
    return Scalar(_need_scalar(a).num * _need_scalar(b).num)


def _div(interp, a, b):
    an, bn = _need_scalar(a).num, _need_scalar(b).num
    if bn.is_zero():
        return Nil(reason="divisionByZero", origin="nilPropagation")
    try:
        return Scalar(an / bn)
    except DomainLimitation as e:
        raise err.StructureError(str(e))


def _mod(interp, a, b):
    an, bn = _need_scalar(a).num, _need_scalar(b).num
    if bn.is_zero():
        raise err.CustomError("Modulo by zero")
    # x - floor(x/y)*y  (Section 7.3)
    q = (an / bn).floor()
    return Scalar(an - AlgebraicReal.from_rational(q) * bn)


# ===========================================================================
# Comparison
# ===========================================================================

def _pair_relation(name, a: Scalar, b: Scalar):
    """Return Boolean for a relation over two scalars (exact domain)."""
    c = a.num.compare(b.num)  # -1, 0, 1
    table = {
        "LT": c < 0, "LTE": c <= 0, "GT": c > 0, "GTE": c >= 0,
        "EQ": c == 0, "NEQ": c != 0,
    }
    return Boolean(table[name])


def _value_equal(a: Value, b: Value) -> bool:
    if isinstance(a, Scalar) and isinstance(b, Scalar):
        return a.num.compare(b.num) == 0
    if isinstance(a, Boolean) and isinstance(b, Boolean):
        return a.value == b.value
    if is_text(a) and is_text(b):
        return text_to_str(a) == text_to_str(b)
    if isinstance(a, Vector) and isinstance(b, Vector):
        if a.role == TEXT or b.role == TEXT:
            return False
        return len(a.items) == len(b.items) and all(
            _value_equal(x, y) for x, y in zip(a.items, b.items))
    if isinstance(a, Record) and isinstance(b, Record):
        return [k for k, _ in a.fields] == [k for k, _ in b.fields] and all(
            _value_equal(av, bv) for (_, av), (_, bv) in zip(a.fields, b.fields))
    if isinstance(a, Nil) and isinstance(b, Nil):
        return True
    return False


def comparison_word(name):
    def impl(interp: Interp, target, consume):
        if target == "STAK":
            n = _stak_count(interp)
            operands = interp.pop_n(n)
            if any(is_nil(o) for o in operands):  # NIL priority (4.5.2)
                result = _nil_inherit(operands)
            else:
                result = _stak_chained(name, operands)
            if consume == "KEEP":
                interp.stack.extend(operands)
            interp.push(result)
            return
        operands = interp.pop_n(2)
        a, b = operands
        if consume == "KEEP":
            interp.stack.extend(operands)
        if is_nil(a) or is_nil(b):
            interp.push(_nil_inherit(operands))
            return
        if name in ("EQ", "NEQ") and not (isinstance(a, Scalar) and isinstance(b, Scalar)):
            eq = _value_equal(a, b)
            interp.push(Boolean(eq if name == "EQ" else not eq))
            return
        interp.push(_pair_relation(name, _need_scalar(a), _need_scalar(b)))
    return impl


def _stak_chained(name, operands):
    if name in ("EQ", "NEQ"):
        if name == "EQ":
            ok = all(_value_equal(operands[0], o) for o in operands[1:])
            return Boolean(ok)
        ok = all(not _value_equal(operands[i], operands[i + 1])
                 for i in range(len(operands) - 1))
        return Boolean(ok)
    # ordering: chained over adjacent pairs
    for i in range(len(operands) - 1):
        r = _pair_relation(name, _need_scalar(operands[i]), _need_scalar(operands[i + 1]))
        if isinstance(r, Unknown):
            return Unknown()
        if not r.value:
            return Boolean(False)
    return Boolean(True)


def _compare_within(interp: Interp, target, consume):
    budget_v = interp.pop()
    b_v = interp.pop()
    a_v = interp.pop()
    if is_nil(a_v) or is_nil(b_v):  # passthrough for a,b
        interp.push(_nil_inherit([a_v, b_v]))
        return
    if not (isinstance(budget_v, Scalar) and budget_v.num.is_integer()
            and budget_v.num.sign() > 0):
        raise err.StructureError("COMPARE-WITHIN budget must be a positive integer")
    a, b = _need_scalar(a_v), _need_scalar(b_v)
    budget = int(budget_v.num.rational_value())
    both_rational = a.num.is_rational() and b.num.is_rational()
    if a.num == b.num:
        if both_rational:
            interp.push(Scalar.of(0))
        else:
            interp.push(Unknown(agreed_prefix=budget))
        return
    if both_rational:
        interp.push(Scalar.of(a.num.compare(b.num)))
        return
    # at least one irrational: honour the budget (Section 7.4.2)
    depth = _nicf_agreement_depth(a.num, b.num, budget)
    if depth < budget:
        interp.push(Scalar.of(a.num.compare(b.num)))
    else:
        interp.push(Unknown(agreed_prefix=budget))


def _nicf_agreement_depth(x: AlgebraicReal, y: AlgebraicReal, cap: int) -> int:
    gx = x.nicf_terms(cap + 1)
    gy = y.nicf_terms(cap + 1)
    depth = 0
    while depth < cap:
        tx = next(gx, None)
        ty = next(gy, None)
        if tx is None and ty is None:
            return depth  # both terminated identically
        if tx != ty:
            return depth
        depth += 1
    return cap


# ===========================================================================
# Logic (strong Kleene K3, Section 7.5)
# ===========================================================================

def _truth(v: Value):
    """Return 'T','F','U' or None (not a truth value)."""
    if isinstance(v, Boolean):
        return "T" if v.value else "F"
    if isinstance(v, Unknown):
        return "U"
    return None


def _logic_and(interp, target, consume):
    operands = interp.pop_n(2)
    a, b = operands
    if consume == "KEEP":
        interp.stack.extend(operands)
    # absorbing F (even with NIL/U)
    if _truth(a) == "F" or _truth(b) == "F":
        interp.push(Boolean(False))
        return
    if is_nil(a) or is_nil(b):
        interp.push(_nil_inherit(operands))
        return
    ta, tb = _truth(a), _truth(b)
    if ta == "T" and tb == "T":
        interp.push(Boolean(True))
    else:
        interp.push(Unknown())


def _logic_or(interp, target, consume):
    operands = interp.pop_n(2)
    a, b = operands
    if consume == "KEEP":
        interp.stack.extend(operands)
    if _truth(a) == "T" or _truth(b) == "T":
        interp.push(Boolean(True))
        return
    if is_nil(a) or is_nil(b):
        interp.push(_nil_inherit(operands))
        return
    ta, tb = _truth(a), _truth(b)
    if ta == "F" and tb == "F":
        interp.push(Boolean(False))
    else:
        interp.push(Unknown())


def _logic_not(interp, target, consume):
    a = interp.pop()
    if consume == "KEEP":
        interp.push(a)
    if is_nil(a):
        interp.push(_nil_inherit([a]))
        return
    t = _truth(a)
    if t == "T":
        interp.push(Boolean(False))
    elif t == "F":
        interp.push(Boolean(True))
    elif t == "U":
        interp.push(Unknown())
    else:
        raise err.StructureError("NOT requires a truth value")


# ===========================================================================
# Vector / tensor operations (Section 7.1, 7.2)
# ===========================================================================

def _need_vector(v: Value) -> Vector:
    if not isinstance(v, Vector):
        raise err.StructureError("vector operand required")
    return v


def _index_value(v: Value) -> int:
    if isinstance(v, Vector) and len(v.items) == 1:
        v = v.items[0]
    if not isinstance(v, Scalar) or not v.num.is_integer():
        raise err.StructureError("index must be an integer")
    return int(v.num.rational_value())


def _w_length(interp, target, consume):
    # inspection word: does not consume its source (Section 7.1.1)
    top = interp.stack[-1] if interp.stack else None
    if isinstance(top, Vector):
        interp.push(Scalar.of(len(top.items)))
    else:
        interp.push(Scalar.of(len(interp.stack)))


def _w_get(interp, target, consume):
    idx = interp.pop()
    vec = interp.stack[-1] if interp.stack else None  # inspection: keep source
    vec = _need_vector(vec)
    i = _index_value(idx)
    n = len(vec.items)
    if i < 0:
        i += n
    if i < 0 or i >= n:
        interp.push(Nil(reason="indexOutOfBounds", origin="nilPropagation"))
        return
    interp.push(vec.items[i])


def _w_insert(interp, target, consume):
    pair = _need_vector(interp.pop())
    vec = _need_vector(interp.pop())
    if len(pair.items) != 2:
        raise err.StructureError("INSERT needs [ index element ]")
    i = _index_value(pair.items[0])
    items = list(vec.items)
    if i < 0:
        i += len(items) + 1
    items.insert(i, pair.items[1])
    interp.push(Vector(items, role=vec.role))


def _w_replace(interp, target, consume):
    pair = _need_vector(interp.pop())
    vec = _need_vector(interp.pop())
    if len(pair.items) != 2:
        raise err.StructureError("REPLACE needs [ index element ]")
    i = _index_value(pair.items[0])
    items = list(vec.items)
    n = len(items)
    if i < 0:
        i += n
    if i < 0 or i >= n:
        interp.push(Nil(reason="indexOutOfBounds", origin="nilPropagation"))
        return
    items[i] = pair.items[1]
    interp.push(Vector(items, role=vec.role))


def _w_remove(interp, target, consume):
    idx = interp.pop()
    vec = _need_vector(interp.pop())
    i = _index_value(idx)
    items = list(vec.items)
    n = len(items)
    if i < 0:
        i += n
    if i < 0 or i >= n:
        interp.push(Nil(reason="indexOutOfBounds", origin="nilPropagation"))
        return
    del items[i]
    interp.push(Vector(items, role=vec.role))


def _w_concat(interp, target, consume):
    if target == "STAK":
        n = _stak_count(interp)
        vecs = interp.pop_n(n)
    else:
        vecs = interp.pop_n(2)
    items = []
    for v in vecs:
        if is_text(v):
            items.extend(v.items)  # coerced to codepoints (Section 7.1.1)
        elif isinstance(v, Vector):
            items.extend(v.items)
        else:
            raise err.StructureError("CONCAT requires vectors")
    interp.push(Vector(items))


def _w_reverse(interp, target, consume):
    vec = _need_vector(interp.pop())
    interp.push(Vector(list(reversed(vec.items)), role=vec.role))


def _w_range(interp, target, consume):
    spec = _need_vector(interp.pop())
    vals = [int(_need_scalar(x).num.rational_value()) for x in spec.items]
    if len(vals) == 2:
        start, end = vals
        step = 1
    elif len(vals) == 3:
        start, end, step = vals
    else:
        raise err.StructureError("RANGE needs [ start end ] or [ start end step ]")
    if step == 0:
        raise err.StructureError("RANGE step must be non-zero")
    items = []
    x = start
    if step > 0:
        while x <= end:
            items.append(Scalar.of(x))
            x += step
    else:
        while x >= end:
            items.append(Scalar.of(x))
            x += step
    interp.push(Vector(items))


def _w_take(interp, target, consume):
    n_v = interp.pop()
    vec = _need_vector(interp.pop())
    n = int(_need_scalar(n_v).num.rational_value())
    interp.push(Vector(list(vec.items[:n]), role=vec.role))


def _w_split(interp, target, consume):
    sizes_v = _need_vector(interp.pop())
    vec = _need_vector(interp.pop())
    sizes = [int(_need_scalar(s).num.rational_value()) for s in sizes_v.items]
    out = []
    pos = 0
    for sz in sizes:
        out.append(Vector(list(vec.items[pos:pos + sz])))
        pos += sz
    interp.push(Vector(out))


def _w_reorder(interp, target, consume):
    idx_v = _need_vector(interp.pop())
    vec = _need_vector(interp.pop())
    n = len(vec.items)
    out = []
    for s in idx_v.items:
        i = int(_need_scalar(s).num.rational_value())
        if i < 0:
            i += n
        if i < 0 or i >= n:
            raise err.IndexOutOfBounds("REORDER index out of range")
        out.append(vec.items[i])
    interp.push(Vector(out, role=vec.role))


def _w_collect(interp, target, consume):
    n = _stak_count(interp)
    items = interp.pop_n(n)
    interp.push(Vector(items))


def _sort_key_compare(a: Value, b: Value):
    return _need_scalar(a).num.compare(_need_scalar(b).num)


def _w_sort(interp, target, consume):
    vec = _need_vector(interp.pop())
    items = list(vec.items)
    if any(is_nil(x) for x in items):
        interp.push(_nil_inherit(items))
        return
    # exact domain: comparisons always decide -> insertion sort by exact compare
    out = []
    for x in items:
        lo, hi = 0, len(out)
        while lo < hi:
            mid = (lo + hi) // 2
            if _sort_key_compare(x, out[mid]) < 0:
                hi = mid
            else:
                lo = mid + 1
        out.insert(lo, x)
    interp.push(Vector(out, role=vec.role))


def _shape_of(v: Value):
    dims = []
    cur = v
    while isinstance(cur, Vector) and cur.role != TEXT:
        dims.append(len(cur.items))
        if cur.items and isinstance(cur.items[0], Vector):
            cur = cur.items[0]
        else:
            break
    return dims


def _w_shape(interp, target, consume):
    v = interp.stack[-1] if interp.stack else None
    dims = _shape_of(_need_vector(v))
    interp.push(Vector([Scalar.of(d) for d in dims]))


def _w_rank(interp, target, consume):
    v = interp.stack[-1] if interp.stack else None
    interp.push(Scalar.of(len(_shape_of(_need_vector(v)))))


def _flatten(v: Vector):
    out = []
    for it in v.items:
        if isinstance(it, Vector) and it.role != TEXT:
            out.extend(_flatten(it))
        else:
            out.append(it)
    return out


def _build_nested(flat, dims):
    if len(dims) == 1:
        return Vector(list(flat[:dims[0]]))
    size = 1
    for d in dims[1:]:
        size *= d
    out = []
    pos = 0
    for _ in range(dims[0]):
        out.append(_build_nested(flat[pos:pos + size], dims[1:]))
        pos += size
    return Vector(out)


def _w_reshape(interp, target, consume):
    dims_v = _need_vector(interp.pop())
    vec = _need_vector(interp.pop())
    dims = [int(_need_scalar(d).num.rational_value()) for d in dims_v.items]
    flat = _flatten(vec)
    total = 1
    for d in dims:
        total *= d
    if total != len(flat):
        raise err.VectorLengthMismatch("RESHAPE size mismatch")
    interp.push(_build_nested(flat, dims))


def _w_transpose(interp, target, consume):
    vec = _need_vector(interp.pop())
    rows = vec.items
    if not rows or not all(isinstance(r, Vector) for r in rows):
        raise err.StructureError("TRANSPOSE requires a 2D tensor")
    ncols = len(rows[0].items)
    if not all(len(r.items) == ncols for r in rows):
        raise err.VectorLengthMismatch("ragged tensor")
    out = [Vector([r.items[c] for r in rows]) for c in range(ncols)]
    interp.push(Vector(out))


def _w_fill(interp, target, consume):
    val = interp.pop()
    dims_v = _need_vector(interp.pop())
    dims = [int(_need_scalar(d).num.rational_value()) for d in dims_v.items]
    total = 1
    for d in dims:
        total *= d
    flat = [val] * total
    interp.push(_build_nested(flat, dims))


# ===========================================================================
# String / type conversion (Section 7.6)
# ===========================================================================

def _w_str(interp, target, consume):
    v = interp.pop()
    if is_text(v):
        interp.push(v)
        return
    interp.push(make_text(render(v, "output")))


def _w_num(interp, target, consume):
    v = interp.pop()
    if not is_text(v):
        raise err.StructureError("NUM requires text")
    s = text_to_str(v).strip()
    from .lexer import _NUM_RE, _parse_number
    if not _NUM_RE.match(s):
        interp.push(Nil(reason="invalidEncoding", origin="nilPropagation"))
        return
    try:
        interp.push(_parse_number(s))
    except Exception:
        interp.push(Nil(reason="invalidEncoding", origin="nilPropagation"))


def _w_bool(interp, target, consume):
    v = interp.pop()
    if isinstance(v, Boolean):
        interp.push(v)
    elif isinstance(v, Scalar):
        interp.push(Boolean(v.num.sign() != 0))
    elif is_nil(v):
        interp.push(Boolean(False))
    else:
        interp.push(Boolean(True))


def _w_chr(interp, target, consume):
    v = interp.pop()
    if not isinstance(v, Scalar):
        raise err.StructureError("CHR requires a number")
    if not v.num.is_integer():
        raise err.StructureError("CHR requires an integer code point")
    cp = int(v.num.rational_value())
    if cp < 0 or cp > 0x10FFFF or 0xD800 <= cp <= 0xDFFF:
        interp.push(Nil(reason="invalidEncoding", origin="nilPropagation"))
        return
    interp.push(make_text(chr(cp)))


def _w_chars(interp, target, consume):
    v = interp.pop()
    if not is_text(v):
        raise err.StructureError("CHARS requires text")
    s = text_to_str(v)
    interp.push(Vector([make_text(ch) for ch in s]))


def _w_join(interp, target, consume):
    top = interp.pop()
    sep = ""
    if is_text(top):
        sep = text_to_str(top)
        vec = _need_vector(interp.pop())
    else:
        vec = _need_vector(top)
    parts = []
    for it in vec.items:
        if is_text(it):
            parts.append(text_to_str(it))
        else:
            parts.append(render(it, "output"))
    interp.push(make_text(sep.join(parts)))


def _text_unary(fn):
    def impl(interp, target, consume):
        v = interp.pop()
        if not is_text(v):
            raise err.StructureError("text operand required")
        interp.push(make_text(fn(text_to_str(v))))
    return impl


def _w_tokenize(interp, target, consume):
    sep = interp.pop()
    vec = interp.pop()
    if not is_text(sep) or not is_text(vec):
        raise err.StructureError("TOKENIZE requires text")
    s = text_to_str(vec)
    d = text_to_str(sep)
    parts = s.split(d) if d else list(s)
    interp.push(Vector([make_text(p) for p in parts]))


def _w_substitute(interp, target, consume):
    repl = interp.pop()
    pat = interp.pop()
    s = interp.pop()
    if not (is_text(repl) and is_text(pat) and is_text(s)):
        raise err.StructureError("SUBSTITUTE requires text")
    interp.push(make_text(text_to_str(s).replace(text_to_str(pat), text_to_str(repl))))


def _w_starts_with(interp, target, consume):
    pre = interp.pop()
    s = interp.pop()
    if not (is_text(pre) and is_text(s)):
        raise err.StructureError("STARTS-WITH? requires text")
    interp.push(Boolean(text_to_str(s).startswith(text_to_str(pre))))


def _w_ends_with(interp, target, consume):
    suf = interp.pop()
    s = interp.pop()
    if not (is_text(suf) and is_text(s)):
        raise err.StructureError("ENDS-WITH? requires text")
    interp.push(Boolean(text_to_str(s).endswith(text_to_str(suf))))


def _w_to_cf(interp, target, consume):
    v = interp.pop()
    if not isinstance(v, Scalar):
        raise err.StructureError(">CF requires a number")
    interp.push(v.with_role(CONTINUED_FRACTION))


# ===========================================================================
# Control & higher-order (Section 7.7)
# ===========================================================================

def _need_block(v) -> CodeBlock:
    if not isinstance(v, CodeBlock):
        raise err.StructureError("code block required")
    return v


def _exec_block_on(interp: Interp, block: CodeBlock):
    interp.run_block(Block(block.lines))


def _w_exec(interp, target, consume):
    block = _need_block(interp.pop())
    _exec_block_on(interp, block)


def _w_eval(interp, target, consume):
    v = interp.pop()
    if not is_text(v):
        raise err.StructureError("EVAL requires text")
    interp.run_source(text_to_str(v))


def _apply_block_to_value(interp: Interp, block: CodeBlock, value: Value) -> Value:
    interp.push(value)
    _exec_block_on(interp, block)
    return interp.pop()


def _w_map(interp, target, consume):
    block = _need_block(interp.pop())
    vec = _need_vector(interp.pop())
    out = [_apply_block_to_value(interp, block, x) for x in vec.items]
    interp.push(Vector(out))


def _w_filter(interp, target, consume):
    block = _need_block(interp.pop())
    vec = _need_vector(interp.pop())
    out = []
    for x in vec.items:
        r = _apply_block_to_value(interp, block, x)
        if isinstance(r, Boolean) and r.value:
            out.append(x)
    interp.push(Vector(out, role=vec.role))


def _w_fold(interp, target, consume):
    block = _need_block(interp.pop())
    init = interp.pop()
    vec = _need_vector(interp.pop())
    acc = init
    for x in vec.items:
        interp.push(acc)
        interp.push(x)
        _exec_block_on(interp, block)
        acc = interp.pop()
    interp.push(acc)


def _w_scan(interp, target, consume):
    block = _need_block(interp.pop())
    init = interp.pop()
    vec = _need_vector(interp.pop())
    acc = init
    out = [acc]
    for x in vec.items:
        interp.push(acc)
        interp.push(x)
        _exec_block_on(interp, block)
        acc = interp.pop()
        out.append(acc)
    interp.push(Vector(out))


def _w_unfold(interp, target, consume):
    block = _need_block(interp.pop())
    seed = interp.pop()
    out = []
    state = seed
    guard = 0
    while True:
        guard += 1
        if guard > interp.step_limit:
            raise err.ExecutionLimitExceeded("UNFOLD did not terminate")
        r = _apply_block_to_value(interp, block, state)
        if is_nil(r):
            break
        if not (isinstance(r, Vector) and len(r.items) == 2):
            raise err.StructureError("UNFOLD block must yield [ value newstate ] or NIL")
        out.append(r.items[0])
        state = r.items[1]
    interp.push(Vector(out))


def _w_any(interp, target, consume):
    block = _need_block(interp.pop())
    vec = _need_vector(interp.pop())
    for x in vec.items:
        r = _apply_block_to_value(interp, block, x)
        if isinstance(r, Boolean) and r.value:
            interp.push(Boolean(True))
            return
    interp.push(Boolean(False))


def _w_all(interp, target, consume):
    block = _need_block(interp.pop())
    vec = _need_vector(interp.pop())
    for x in vec.items:
        r = _apply_block_to_value(interp, block, x)
        if not (isinstance(r, Boolean) and r.value):
            interp.push(Boolean(False))
            return
    interp.push(Boolean(True))


def _w_count(interp, target, consume):
    block = _need_block(interp.pop())
    vec = _need_vector(interp.pop())
    cnt = 0
    for x in vec.items:
        r = _apply_block_to_value(interp, block, x)
        if isinstance(r, Boolean) and r.value:
            cnt += 1
    interp.push(Scalar.of(cnt))


def _w_idle(interp, target, consume):
    pass


def _w_cond(interp, target, consume):
    # SPEC-GAP: collection of clauses is under-specified (Section 3.6, 7.7).
    # We consume consecutive CodeBlocks from the top as clauses (written order),
    # each clause being `guard | body`; the remaining stack is the working base.
    clauses = []
    while interp.stack and isinstance(interp.stack[-1], CodeBlock):
        clauses.append(interp.stack.pop())
    clauses.reverse()  # written order
    if not clauses:
        raise err.StructureError("COND requires clause code blocks")
    base = list(interp.stack)
    for clause in clauses:
        guard_nodes, body_nodes = _split_clause(clause)
        is_else = (len(guard_nodes) == 1 and guard_nodes[0] == ("word", "IDLE")) \
            or len(guard_nodes) == 0
        if is_else:
            interp.stack = list(base)
            interp.run_line(body_nodes)
            return
        interp.stack = list(base)
        interp.run_line(guard_nodes)
        verdict = interp.pop() if interp.stack else None
        if isinstance(verdict, Boolean) and verdict.value:
            interp.stack = list(base)
            interp.run_line(body_nodes)
            return
        # FALSE, UNKNOWN, or NIL guard -> fall through (Section 7.4.3)
    raise err.CondExhausted("COND: no matching clause")


def _split_clause(clause: CodeBlock):
    # clause is a single-line block: nodes with one top-level pipe
    nodes = clause.lines[0] if clause.lines else []
    if len(clause.lines) > 1:
        # flatten (each | clause on its own line per Section 3.6); join lines
        nodes = [n for line in clause.lines for n in line]
    if ("pipe",) in nodes:
        i = nodes.index(("pipe",))
        return nodes[:i], nodes[i + 1:]
    return nodes, []


def _w_precompute(interp, target, consume):
    # SPEC-GAP: PRECOMPUTE is a definition-time staging marker; outside a DEF
    # body it is an error (Section 7.7). We approximate it as immediate
    # evaluation, which matches its result-splicing effect for simple cases.
    block = _need_block(interp.pop())
    _exec_block_on(interp, block)


# ===========================================================================
# VENT / FORC / user dictionary / IO
# ===========================================================================

def _w_vent(interp, target, consume):
    top = interp.pop()
    if interp.stack:
        nxt = interp.pop()
    else:
        nxt = None
    if is_nil(top):
        if nxt is None:
            raise err.StackUnderflow("VENT needs a fallback value")
        interp.push(nxt)
    else:
        interp.push(top)


def _w_forc(interp, target, consume):
    # FORC as a standalone word is a modifier-like marker; no-op on the stack.
    pass


def _collect_user_deps(block: CodeBlock):
    deps = set()
    for line in block.lines:
        for node in line:
            if node[0] == "word":
                deps.add(node[1])
    return deps


def _w_def(interp, target, consume):
    name_v = interp.pop()
    body = interp.pop()
    if not is_text(name_v):
        raise err.StructureError("DEF name must be a string")
    if not isinstance(body, CodeBlock):
        raise err.StructureError("DEF body must be a code block")
    name = text_to_str(name_v).upper()
    if name in CORE_WORDS or _is_module_word(name):
        raise err.BuiltinProtection(f"cannot redefine built-in {name}")
    if name in interp.user_dict and target != "FORC":
        # redefining with active dependents requires FORC (Section 8.2)
        dependents = [w for w, d in interp.user_deps.items() if name in d and w != name]
        if dependents:
            raise err.BuiltinProtection(
                f"{name} has dependents {dependents}; use ! (FORC) to redefine")
    interp.user_dict[name] = body
    interp.user_deps[name] = _collect_user_deps(body)
    _warn_naming(interp, name)


def _w_del(interp, target, consume):
    name_v = interp.pop()
    if not is_text(name_v):
        raise err.StructureError("DEL name must be a string")
    name = text_to_str(name_v).upper()
    if "@" in name:
        name = name.split("@", 1)[1]
    if name in CORE_WORDS or _is_module_word(name):
        raise err.BuiltinProtection("cannot delete built-in word")
    if name not in interp.user_dict:
        raise err.UnknownWord(f"no user word {name}")
    dependents = [w for w, d in interp.user_deps.items() if name in d and w != name]
    if dependents and target != "FORC":
        raise err.BuiltinProtection(
            f"{name} has dependents {dependents}; use ! (FORC) to delete")
    del interp.user_dict[name]
    interp.user_deps.pop(name, None)


def _w_lookup(interp, target, consume):
    name_v = interp.pop()
    if not is_text(name_v):
        raise err.StructureError("LOOKUP name must be a string")
    name = text_to_str(name_v).upper()
    if name in interp.user_dict:
        interp.output.append(f"{name}: {{ {interp.user_dict[name].source} }}")
    elif name in CORE_WORDS:
        interp.output.append(f"{name}: <core word>")
    elif _is_module_word(name):
        interp.output.append(f"{name}: <module word>")
    else:
        raise err.UnknownWord(f"unknown word {name}")


def _warn_naming(interp, name):
    bad_prefix = ("DO-", "HANDLE-", "PROCESS-", "MANAGE-", "UTIL-", "HELPER-")
    bad_names = {"CALC", "RUN", "EXEC2", "TEMP", "MAIN", "TEST", "STUFF", "THING"}
    if name in bad_names or name.startswith(bad_prefix):
        interp.warnings.append(f"ambiguous word name: {name}")


def _w_print(interp, target, consume):
    if not interp.stack:
        raise err.StackUnderflow("PRINT needs a value")
    if consume == "KEEP":
        v = interp.stack[-1]
    else:
        v = interp.pop()
    interp.output.append(render(v, "output"))


# ===========================================================================
# Module loading (Section 7.10, 9.2)
# ===========================================================================

def _is_module_word(name):
    return any(name in MODULE_WORDS.get(m, {}) for m in MODULE_SPECS)


def _w_import(interp, target, consume):
    name_v = interp.pop()
    if not is_text(name_v):
        raise err.StructureError("IMPORT name must be a string")
    mod = text_to_str(name_v).upper()
    if mod not in MODULE_SPECS:
        raise err.UnknownModule(f"unknown module {mod}")
    if mod not in interp.imported:
        interp.imported.append(mod)


def _w_import_only(interp, target, consume):
    sel = interp.pop()
    name_v = interp.pop()
    if not is_text(name_v) or not isinstance(sel, Vector):
        raise err.StructureError("IMPORT-ONLY requires module name and selector vector")
    mod = text_to_str(name_v).upper()
    if mod not in MODULE_SPECS:
        raise err.UnknownModule(f"unknown module {mod}")
    # SPEC-GAP: per-word import scoping is modelled coarsely as a full import.
    if mod not in interp.imported:
        interp.imported.append(mod)


def _w_unimport(interp, target, consume):
    name_v = interp.pop()
    mod = text_to_str(name_v).upper()
    if mod in interp.imported:
        interp.imported.remove(mod)


def _w_unimport_only(interp, target, consume):
    interp.pop()  # selector
    name_v = interp.pop()
    mod = text_to_str(name_v).upper()
    if mod in interp.imported:
        interp.imported.remove(mod)


# ===========================================================================
# Child runtime (Section 10) — synchronous model
# ===========================================================================

def _w_spawn(interp, target, consume):
    block = _need_block(interp.pop())
    child = ChildRuntime(state="running")
    sub = Interp(interp.step_limit)
    sub.user_dict = dict(interp.user_dict)
    sub.user_deps = dict(interp.user_deps)
    sub.imported = list(interp.imported)
    try:
        sub.run_block(Block(block.lines))
        child.state = "completed"
        child.result_stack = list(sub.stack)
    except err.ExecutionLimitExceeded:
        child.state = "timeout"
    except err.AjisaiError as e:
        child.state = "failed"
        child.error = e
    interp.push(ProcessHandle(child))


def _w_await(interp, target, consume):
    h = interp.pop()
    if not isinstance(h, ProcessHandle):
        raise err.StructureError("AWAIT requires a process handle")
    status = make_text(h.child.state)
    interp.push(Vector([status, Vector(list(h.child.result_stack))]))


def _w_status(interp, target, consume):
    h = interp.pop()
    if not isinstance(h, ProcessHandle):
        raise err.StructureError("STATUS requires a process handle")
    interp.push(make_text(h.child.state))


def _w_kill(interp, target, consume):
    h = interp.pop()
    if not isinstance(h, ProcessHandle):
        raise err.StructureError("KILL requires a process handle")
    if h.child.state == "running":
        h.child.state = "killed"


def _w_monitor(interp, target, consume):
    h = interp.pop()
    if not isinstance(h, ProcessHandle):
        raise err.StructureError("MONITOR requires a process handle")
    h.child.monitored = True
    interp.push(h)


def _w_supervise(interp, target, consume):
    grp = interp.pop()
    if not isinstance(grp, Vector):
        raise err.StructureError("SUPERVISE requires a vector of handles")
    interp.push(SupervisorHandle(list(grp.items)))


# ===========================================================================
# Module words: MATH, ALGO, TIME, CRYPTO, IO, SERIAL
# ===========================================================================

def _w_sqrt(interp, target, consume):
    v = interp.pop()
    if is_nil(v):
        interp.push(_nil_inherit([v]))
        return
    s = _need_scalar(v)
    if not s.num.is_rational():
        raise err.StructureError("SQRT domain limited to rationals in this port")
    if s.num.sign() < 0:
        interp.push(Nil(reason="negativeRoot", origin="nilPropagation"))
        return
    interp.push(Scalar(AlgebraicReal.sqrt_of(s.num)))


def _w_abs(interp, target, consume):
    unary_numeric(interp, target, consume,
                  lambda i, a: Scalar(_need_scalar(a).num if _need_scalar(a).num.sign() >= 0
                                      else -_need_scalar(a).num))


def _w_neg(interp, target, consume):
    unary_numeric(interp, target, consume, lambda i, a: Scalar(-_need_scalar(a).num))


def _w_sign(interp, target, consume):
    unary_numeric(interp, target, consume,
                  lambda i, a: Scalar.of(_need_scalar(a).num.sign()))


def _select_minmax(name):
    def impl(interp, target, consume):
        if target == "STAK":
            n = _stak_count(interp)
            operands = interp.pop_n(n)
        else:
            operands = interp.pop_n(2)
        if any(is_nil(o) for o in operands):
            result = _nil_inherit(operands)
        else:
            scalars = [_need_scalar(o) for o in operands]
            best = scalars[0]
            for s in scalars[1:]:
                c = s.num.compare(best.num)
                if (name == "MIN" and c < 0) or (name == "MAX" and c > 0):
                    best = s
            result = best
        if consume == "KEEP":
            interp.stack.extend(operands)
        interp.push(result)
    return impl


def _w_pow(interp, target, consume):
    def fn(i, a, b):
        base = _need_scalar(a).num
        exp_s = _need_scalar(b)
        if not exp_s.num.is_integer():
            raise err.StructureError("POW exponent must be an integer in this port")
        e = int(exp_s.num.rational_value())
        if e >= 0:
            acc = AlgebraicReal.from_rational(1)
            for _ in range(e):
                acc = acc * base
            return Scalar(acc)
        if base.is_zero():
            return Nil(reason="divisionByZero", origin="nilPropagation")
        acc = AlgebraicReal.from_rational(1)
        for _ in range(-e):
            acc = acc * base
        return Scalar(acc.reciprocal())
    binary_numeric(interp, target, consume, fn)


def _int_pair(a, b):
    sa, sb = _need_scalar(a), _need_scalar(b)
    if not (sa.num.is_integer() and sb.num.is_integer()):
        raise err.StructureError("integer operands required")
    return int(sa.num.rational_value()), int(sb.num.rational_value())


def _w_gcd(interp, target, consume):
    def fn(i, a, b):
        from math import gcd
        x, y = _int_pair(a, b)
        return Scalar.of(gcd(x, y))
    binary_numeric(interp, target, consume, fn)


def _w_lcm(interp, target, consume):
    def fn(i, a, b):
        from math import gcd
        x, y = _int_pair(a, b)
        if x == 0 or y == 0:
            return Scalar.of(0)
        return Scalar.of(abs(x * y) // gcd(x, y))
    binary_numeric(interp, target, consume, fn)


def _w_index_of(interp, target, consume):
    needle = interp.pop()
    vec = _need_vector(interp.pop())
    for i, it in enumerate(vec.items):
        if _value_equal(it, needle):
            interp.push(Scalar.of(i))
            return
    interp.push(Nil(reason="missingField", origin="nilPropagation"))


def _w_now(interp, target, consume):
    interp.push(Scalar.of(Fraction(time.time_ns(), 1_000_000_000)))


def _w_datetime(interp, target, consume):
    offset = interp.pop()
    instant = interp.pop()
    off_h = int(_need_scalar(offset).num.rational_value())
    secs = int(_need_scalar(instant).num.floor())
    import datetime as _dt
    dt = _dt.datetime.utcfromtimestamp(secs) + _dt.timedelta(hours=off_h)
    interp.push(make_text(dt.strftime("%Y-%m-%dT%H:%M:%S")))


def _w_timestamp(interp, target, consume):
    offset = interp.pop()
    civil = interp.pop()
    off_h = int(_need_scalar(offset).num.rational_value())
    if not is_text(civil):
        raise err.StructureError("TIMESTAMP requires a civil datetime string")
    import datetime as _dt
    dt = _dt.datetime.strptime(text_to_str(civil), "%Y-%m-%dT%H:%M:%S")
    epoch = _dt.datetime(1970, 1, 1)
    secs = int((dt - epoch).total_seconds()) - off_h * 3600
    interp.push(Scalar.of(secs))


def _w_parse_iso(interp, target, consume):
    v = interp.pop()
    if not is_text(v):
        raise err.StructureError("PARSE-ISO requires text")
    import datetime as _dt
    try:
        dt = _dt.datetime.strptime(text_to_str(v), "%Y-%m-%dT%H:%M:%S")
        epoch = _dt.datetime(1970, 1, 1)
        interp.push(Scalar.of(int((dt - epoch).total_seconds())))
    except ValueError:
        interp.push(Nil(reason="invalidEncoding", origin="nilPropagation"))


def _w_csprng(interp, target, consume):
    interp.push(Scalar.of(Fraction(secrets.randbits(53), 1 << 53)))


def _w_hash(interp, target, consume):
    v = interp.pop()
    digest = hashlib.sha256(render(v, "output").encode("utf-8")).hexdigest()
    interp.push(Scalar.of(int(digest[:16], 16)))


def _w_io_input(interp, target, consume):
    if interp.input_buffer:
        interp.push(make_text(interp.input_buffer.pop(0)))
    else:
        interp.push(make_text(""))


def _w_io_output(interp, target, consume):
    v = interp.pop()
    interp.output.append(render(v, "output"))


# ===========================================================================
# Word tables
# ===========================================================================

def _arith(fn):
    return lambda i, t, c: binary_numeric(i, t, c, fn)


CORE_WORDS = {
    "ADD": _arith(_add), "SUB": _arith(_sub), "MUL": _arith(_mul),
    "DIV": _arith(_div), "MOD": _arith(_mod),
    "FLOOR": lambda i, t, c: unary_numeric(i, t, c,
              lambda ii, a: Scalar.of(_need_scalar(a).num.floor())),
    "CEIL": lambda i, t, c: unary_numeric(i, t, c,
              lambda ii, a: Scalar.of(_need_scalar(a).num.ceil())),
    "ROUND": lambda i, t, c: unary_numeric(i, t, c,
              lambda ii, a: Scalar.of(_need_scalar(a).num.round_half_away())),
    "LT": comparison_word("LT"), "LTE": comparison_word("LTE"),
    "GT": comparison_word("GT"), "GTE": comparison_word("GTE"),
    "EQ": comparison_word("EQ"), "NEQ": comparison_word("NEQ"),
    "COMPARE-WITHIN": _compare_within,
    "AND": _logic_and, "OR": _logic_or, "NOT": _logic_not,
    "TRUE": lambda i, t, c: i.push(Boolean(True)),
    "FALSE": lambda i, t, c: i.push(Boolean(False)),
    "NIL": lambda i, t, c: i.push(Nil(origin="literal")),
    "IDLE": _w_idle,
    "LENGTH": _w_length, "GET": _w_get, "INSERT": _w_insert, "REPLACE": _w_replace,
    "REMOVE": _w_remove, "CONCAT": _w_concat, "REVERSE": _w_reverse,
    "RANGE": _w_range, "TAKE": _w_take, "SPLIT": _w_split, "REORDER": _w_reorder,
    "COLLECT": _w_collect, "SORT": _w_sort,
    "SHAPE": _w_shape, "RANK": _w_rank, "RESHAPE": _w_reshape,
    "TRANSPOSE": _w_transpose, "FILL": _w_fill,
    "STR": _w_str, "NUM": _w_num, "BOOL": _w_bool, "CHR": _w_chr, "CHARS": _w_chars,
    "JOIN": _w_join, "TRIM": _text_unary(str.strip),
    "TRIM-LEFT": _text_unary(str.lstrip), "TRIM-RIGHT": _text_unary(str.rstrip),
    "TOKENIZE": _w_tokenize, "SUBSTITUTE": _w_substitute,
    "STARTS-WITH?": _w_starts_with, "ENDS-WITH?": _w_ends_with, ">CF": _w_to_cf,
    "MAP": _w_map, "FILTER": _w_filter, "FOLD": _w_fold, "UNFOLD": _w_unfold,
    "ANY": _w_any, "ALL": _w_all, "COUNT": _w_count, "SCAN": _w_scan,
    "COND": _w_cond, "EXEC": _w_exec, "EVAL": _w_eval, "PRECOMPUTE": _w_precompute,
    "DEF": _w_def, "DEL": _w_del, "LOOKUP": _w_lookup,
    "PRINT": _w_print, "VENT": _w_vent, "FORC": _w_forc,
    "IMPORT": _w_import, "IMPORT-ONLY": _w_import_only,
    "UNIMPORT": _w_unimport, "UNIMPORT-ONLY": _w_unimport_only,
    "SPAWN": _w_spawn, "AWAIT": _w_await, "STATUS": _w_status, "KILL": _w_kill,
    "MONITOR": _w_monitor, "SUPERVISE": _w_supervise,
}

MODULE_WORDS = {
    "MATH": {
        "SQRT": _w_sqrt, "ABS": _w_abs, "NEG": _w_neg, "SIGN": _w_sign,
        "MIN": _select_minmax("MIN"), "MAX": _select_minmax("MAX"),
        "POW": _w_pow, "GCD": _w_gcd, "LCM": _w_lcm,
    },
    "ALGO": {"SORT": _w_sort, "INDEX-OF": _w_index_of},
    "TIME": {"NOW": _w_now, "DATETIME": _w_datetime, "TIMESTAMP": _w_timestamp,
             "PARSE-ISO": _w_parse_iso},
    "CRYPTO": {"CSPRNG": _w_csprng, "HASH": _w_hash},
    "IO": {"INPUT": _w_io_input, "OUTPUT": _w_io_output},
    "SERIAL": {},
    "JSON": {},
    "MUSIC": {},
}


def _node_src(node):
    k = node[0]
    if k == "word":
        return node[1]
    if k == "num":
        return render(node[1], "stack")
    if k == "str":
        return f"'{node[1]}'"
    if k == "pipe":
        return "|"
    if k == "mod":
        t, c = node[1]
        return {"TOP": ".", "STAK": ".."}.get(t, "") + {"EAT": ",", "KEEP": ",,"}.get(c, "")
    if k == "vec":
        return "[ " + " ".join(_node_src(n) for n in node[1]) + " ]"
    if k == "block":
        return "{ ... }"
    return "?"
