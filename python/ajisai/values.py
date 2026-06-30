"""Value model, interpretation roles, and rendering (Sections 4, 12).

Written from SPECIFICATION.html alone. The data plane carries payloads; the
semantic plane carries an interpretation role per stack position (Section 5.2,
12.1). For this port the role is attached to the value object — the observable
rendering is what matters (Section 12.2: "a pure function of (data, role)").
"""

from __future__ import annotations

from fractions import Fraction
from typing import List, Optional

from .numbers import AlgebraicReal


# Interpretation roles (Section 12.2)
UNASSIGNED = "Unassigned"
RAW_NUMBER = "RawNumber"
CONTINUED_FRACTION = "ContinuedFraction"
INTERVAL = "Interval"
TEXT = "Text"
TRUTH_VALUE = "TruthValue"
TIMESTAMP = "Timestamp"
NIL_ROLE = "Nil"


class Value:
    role = UNASSIGNED


class Scalar(Value):
    """An exact real (Section 4.1, 4.2)."""

    __slots__ = ("num", "role")

    def __init__(self, num: AlgebraicReal, role: str = RAW_NUMBER):
        self.num = num
        self.role = role

    @staticmethod
    def of(value, role: str = RAW_NUMBER) -> "Scalar":
        if isinstance(value, AlgebraicReal):
            return Scalar(value, role)
        return Scalar(AlgebraicReal.from_rational(value), role)

    def with_role(self, role: str) -> "Scalar":
        return Scalar(self.num, role)

    def __repr__(self):
        return f"Scalar({self.num!r})"


class Boolean(Value):
    """Definite truth value true/false (Section 4.1); distinct from Scalar."""

    __slots__ = ("value",)
    role = TRUTH_VALUE

    def __init__(self, value: bool):
        self.value = value

    def __repr__(self):
        return f"Boolean({self.value})"


class Unknown(Value):
    """The logical third truth value U (Sections 7.4.1, 7.5).

    Carries the TruthValue role; observed as truthValue = unknown.
    ``agreed_prefix`` is the diagnostic CF agreed-prefix length (Section 4.5.0).
    """

    __slots__ = ("agreed_prefix",)
    role = TRUTH_VALUE

    def __init__(self, agreed_prefix: Optional[int] = None):
        self.agreed_prefix = agreed_prefix

    def __repr__(self):
        return "Unknown()"


class Vector(Value):
    """Ordered indexable sequence (Section 4.3). Text is a Vector of codepoint
    scalars carrying the Text role (Section 12.2)."""

    __slots__ = ("items", "role")

    def __init__(self, items: List[Value], role: str = UNASSIGNED):
        self.items = items
        self.role = role

    def __repr__(self):
        return f"Vector({self.items!r}, role={self.role})"


class Record(Value):
    """Ordered named fields with string keys (Section 4.4)."""

    __slots__ = ("fields",)
    role = UNASSIGNED

    def __init__(self, fields):
        # list of (key:str, value:Value), insertion order preserved
        self.fields = list(fields)

    def __repr__(self):
        return f"Record({self.fields!r})"


class Nil(Value):
    """Diagnostic absence value (Sections 4.5, 4.5.0)."""

    __slots__ = ("reason", "origin", "recoverability", "diagnosis")
    role = NIL_ROLE

    def __init__(self, reason=None, origin="computed", recoverability="inspect",
                 diagnosis=None):
        self.reason = reason
        self.origin = origin
        self.recoverability = recoverability
        self.diagnosis = diagnosis

    def __repr__(self):
        return f"Nil(reason={self.reason})"


class CodeBlock(Value):
    """First-class executable token sequence (Section 4.6)."""

    __slots__ = ("lines", "source")
    role = UNASSIGNED

    def __init__(self, lines, source=""):
        # lines: List[List[token]] — one statement list per source line
        self.lines = lines
        self.source = source

    def __repr__(self):
        return f"CodeBlock({self.source!r})"


class ProcessHandle(Value):
    __slots__ = ("child",)
    role = UNASSIGNED

    def __init__(self, child):
        self.child = child


class SupervisorHandle(Value):
    __slots__ = ("children",)
    role = UNASSIGNED

    def __init__(self, children):
        self.children = children


# ---------------------------------------------------------------------------
# Constructors / helpers
# ---------------------------------------------------------------------------

def make_text(s: str) -> Vector:
    """A Text value: a Vector of codepoint scalars with the Text role."""
    return Vector([Scalar.of(ord(ch)) for ch in s], role=TEXT)


def text_to_str(v: Vector) -> str:
    return "".join(chr(int(s.num.rational_value())) for s in v.items)


def is_text(v: Value) -> bool:
    return isinstance(v, Vector) and v.role == TEXT


def is_nil(v: Value) -> bool:
    return isinstance(v, Nil)


TRUE = Boolean(True)
FALSE = Boolean(False)


# ---------------------------------------------------------------------------
# Rendering (Section 12.1-12.3): pure function of (data, role)
# ---------------------------------------------------------------------------

def _render_cf(num: AlgebraicReal, budget: int = 24) -> str:
    """Nested right-associative continued-fraction form (Section 4.2.3)."""
    terms = list(num.rcf_terms(budget))
    truncated = not num.is_rational() and len(terms) >= budget
    # build nested ( a0 ( a1 ... ) )
    s = ""
    closing = ""
    for i, a in enumerate(terms):
        s += f"( {a} "
        closing += ")"
    if truncated:
        s += "...) " + closing
    else:
        s += closing
    return s.strip()


def _render_rational(num: AlgebraicReal) -> str:
    if num.is_rational():
        r = num.rational_value()
        return f"{r.numerator}/{r.denominator}"
    # Spec gives no RawNumber surface for irrationals; fall back to CF form.
    return _render_cf(num)


def render(value: Value, surface: str = "stack") -> str:
    """Render a value for the Stack (surface='stack') or Output (surface='output')."""
    if isinstance(value, Scalar):
        if value.role == CONTINUED_FRACTION:
            return _render_cf(value.num)
        return _render_rational(value.num)

    if isinstance(value, Boolean):
        return "TRUE" if value.value else "FALSE"

    if isinstance(value, Unknown):
        return "UNKNOWN"

    if isinstance(value, Nil):
        return "NIL"

    if isinstance(value, Vector):
        if value.role == TEXT:
            s = text_to_str(value)
            if surface == "output":
                return s
            return f"'{s}'"
        inner = " ".join(render(it, "stack") for it in value.items)
        return f"[ {inner} ]" if value.items else "[ ]"

    if isinstance(value, Record):
        parts = []
        for k, v in value.fields:
            parts.append(f"{k}: {render(v, 'stack')}")
        return "{ " + " ".join(parts) + " }" if parts else "{ }"

    if isinstance(value, CodeBlock):
        return "{ " + value.source.strip() + " }"

    if isinstance(value, ProcessHandle):
        return "<process>"
    if isinstance(value, SupervisorHandle):
        return "<supervisor>"

    return repr(value)
