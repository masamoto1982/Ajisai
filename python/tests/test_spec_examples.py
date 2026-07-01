"""Tests asserting the concrete examples written in SPECIFICATION.html.

Each test cites the spec section it verifies. Run with: python -m pytest python/tests
or simply: python python/tests/test_spec_examples.py
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from ajisai import run                      # noqa: E402
from ajisai.values import render            # noqa: E402


def stack_str(src):
    i = run(src)
    return " ".join(render(v, "stack") for v in i.stack)


def out_str(src):
    i = run(src)
    return "\n".join(i.output)


CASES = [
    # Section 6.1 — STAK modifier
    ("1 2 3 3 STAK ADD", "6/1"),
    ("1 2 3 3 STAK SUB", "-4/1"),
    ("1 2 3 3 STAK KEEP ADD", "1/1 2/1 3/1 6/1"),
    ("1 2 3 4 4 STAK LT", "TRUE"),
    ("5 4 3 2 1 5 STAK LT", "FALSE"),
    # Section 7.1.1 — vector ops & inspection retention
    ("[ 1 2 3 ] LENGTH", "[ 1/1 2/1 3/1 ] 3/1"),
    ("[ 1 2 3 ] 1 GET", "[ 1/1 2/1 3/1 ] 2/1"),
    ("[ 1 2 3 ] [ -1 ] GET", "[ 1/1 2/1 3/1 ] 3/1"),
    ("[ 1 2 3 ] [ 1 5 ] REPLACE", "[ 1/1 5/1 3/1 ]"),
    ("[ 1 2 3 ] [ 1 9 ] INSERT", "[ 1/1 9/1 2/1 3/1 ]"),
    ("[ 1 5 ] RANGE", "[ 1/1 2/1 3/1 4/1 5/1 ]"),
    ("[ 1 10 2 ] RANGE", "[ 1/1 3/1 5/1 7/1 9/1 ]"),
    ("1 2 3 3 COLLECT", "[ 1/1 2/1 3/1 ]"),
    ("'ab' 'cd' CONCAT", "[ 97/1 98/1 99/1 100/1 ]"),
    # Section 4.1 — Boolean distinct from Scalar
    ("TRUE 1 EQ", "FALSE"),
    # Section 7.3 — arithmetic, DIV/MOD asymmetry
    ("3 4 ADD", "7/1"),
    ("3 0 DIV", "NIL"),
    ("3 0 DIV 99 VENT", "99/1"),
    # Section 7.3 — ROUND ties away from zero
    ("0.5 ROUND", "1/1"),
    ("-2.5 ROUND", "-3/1"),
    # Section 2.3.1.1 — exact algebraic equality
    ("'math' IMPORT 2 MATH@SQRT 2 MATH@SQRT SUB 0 EQ", "TRUE"),
    ("'math' IMPORT 9 MATH@SQRT", "3/1"),
    ("'math' IMPORT 2 MATH@SQRT 2 MATH@SQRT MUL", "2/1"),
    # Section 4.2.7 / 7.4 — six relations total & exact over the admitted domain D
    # (multiquadratic field), never UNKNOWN. Closed under +,-,*,/.
    ("'math' IMPORT 2 MATH@SQRT 3 MATH@SQRT ADD 3 MATH@SQRT 2 MATH@SQRT ADD EQ", "TRUE"),
    ("'math' IMPORT 2 MATH@SQRT 3 MATH@SQRT LT", "TRUE"),
    ("'math' IMPORT 6 MATH@SQRT 2 MATH@SQRT 3 MATH@SQRT MUL EQ", "TRUE"),
    # division closure: 1/(√2+√3) * (√2+√3) = 1
    ("'math' IMPORT 1 2 MATH@SQRT 3 MATH@SQRT ADD DIV "
     "2 MATH@SQRT 3 MATH@SQRT ADD MUL", "1/1"),
    # Section 7.4.2 — COMPARE-WITHIN decided cases
    ("1 2 3 COMPARE-WITHIN", "-1/1"),
    ("5 5 3 COMPARE-WITHIN", "0/1"),
    # Section 7.2 — tensor
    ("[ [ 1 2 ] [ 3 4 ] ] TRANSPOSE", "[ [ 1/1 3/1 ] [ 2/1 4/1 ] ]"),
    ("[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE", "[ [ 1/1 2/1 3/1 ] [ 4/1 5/1 6/1 ] ]"),
    # Section 7.7 — higher-order
    ("[ 1 2 3 ] { 2 MUL } MAP", "[ 2/1 4/1 6/1 ]"),
    ("[ 1 2 3 4 ] 0 { ADD } FOLD", "10/1"),
]

OUTPUT_CASES = [
    # Section 7.9 — PRINT surface (quote stripping)
    ("'TEST' PRINT", "TEST"),
    ("[ 'AB' 'CD' ] PRINT", "[ 'AB' 'CD' ]"),
    ("42 PRINT", "42/1"),
    ("TRUE PRINT", "TRUE"),
]


def run_all():
    failures = 0
    for src, expected in CASES:
        got = stack_str(src)
        ok = got == expected
        print(("PASS" if ok else "FAIL"), repr(src), "->", repr(got),
              "" if ok else f"(expected {expected!r})")
        failures += not ok
    for src, expected in OUTPUT_CASES:
        got = out_str(src)
        ok = got == expected
        print(("PASS" if ok else "FAIL"), repr(src), "OUT->", repr(got),
              "" if ok else f"(expected {expected!r})")
        failures += not ok
    # UNKNOWN cases (Section 7.4.2): equal irrationals undecided within budget
    i = run("'math' IMPORT 2 MATH@SQRT 2 MATH@SQRT 3 COMPARE-WITHIN")
    from ajisai.values import Unknown
    ok = len(i.stack) == 1 and isinstance(i.stack[0], Unknown)
    print(("PASS" if ok else "FAIL"), "COMPARE-WITHIN equal irrationals -> UNKNOWN")
    failures += not ok
    # Section 4.2.7: SQRT of a non-rational leaves D -> malformed use -> error
    from ajisai.errors import StructureError
    raised = False
    try:
        run("'math' IMPORT 2 MATH@SQRT MATH@SQRT")
    except StructureError:
        raised = True
    print(("PASS" if raised else "FAIL"), "SQRT of a non-rational raises (Section 4.2.7)")
    failures += not raised
    return failures


# pytest hooks
def test_stack_cases():
    for src, expected in CASES:
        assert stack_str(src) == expected, src


def test_output_cases():
    for src, expected in OUTPUT_CASES:
        assert out_str(src) == expected, src


if __name__ == "__main__":
    sys.exit(1 if run_all() else 0)
