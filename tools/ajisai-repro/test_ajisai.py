#!/usr/bin/env python3
"""Regression tests for the Python Core reference interpreter (`ajisai.py`).

Standard-library `unittest` only; no external dependencies (matching the
differential driver's Core/Hosted constraint in README.md). The directory name
contains a hyphen, so it is not an importable package; run from this directory:

    cd tools/ajisai-repro && python3 -m unittest test_ajisai
    cd tools/ajisai-repro && python3 test_ajisai.py

These lock the behaviours the reference implementation must share with the
canonical SPECIFICATION.html and the production Rust CLI. The reference
implementation is the "executable shadow of the spec" and the differential
oracle (README.md), so a divergence here is a defect in the reference, not a
new semantics.
"""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ajisai as repro  # noqa: E402


def stack_of(src):
    """Run `src` through the reference interpreter and return the displayed
    stack, asserting a clean (non-error) run."""
    result = repro.run_program(src)
    assert result["status"] == "ok", f"unexpected error for {src!r}: {result}"
    return result["stack"]


class CompareWithinExactness(unittest.TestCase):
    """SPEC §7.4.1 / §7.4.2: over the admitted domain D (Section 4.2.7) —
    everything the current Coreword set can construct — COMPARE-WITHIN is total
    and exact and decides regardless of the named budget. The logical UNKNOWN
    is reserved for the Tier 2 reals (Section 4.2.2), which no current word can
    construct, so it is unreachable here. See SPECIFICATION.html lines that
    state "including composed equal operands, deciding regardless of budget".
    """

    def test_equal_algebraic_decides_to_zero(self):
        # (√2 + 1) vs (√2 + 1): equal Tier 1 operands built through the same
        # history must decide to 0, not stagnate to UNKNOWN, at any budget.
        self.assertEqual(
            stack_of("'MATH' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD 8 COMPARE-WITHIN"),
            ["0/1"],
        )

    def test_equal_algebraic_decides_at_budget_one(self):
        # The budget is irrelevant over D: even budget 1 decides equality.
        self.assertEqual(
            stack_of("'MATH' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD 1 COMPARE-WITHIN"),
            ["0/1"],
        )

    def test_distinct_algebraic_decides_at_tiny_budget(self):
        # √2 < √3: distinct Tier 1 operands decide even at the smallest budget,
        # because order is read off the normal form, not a streamed CF prefix.
        self.assertEqual(
            stack_of("'MATH' IMPORT 2 SQRT 3 SQRT 1 COMPARE-WITHIN"),
            ["-1/1"],
        )
        self.assertEqual(
            stack_of("'MATH' IMPORT 3 SQRT 2 SQRT 1 COMPARE-WITHIN"),
            ["1/1"],
        )

    def test_equal_composed_through_different_histories_decides(self):
        # √8 vs √2 + √2 are equal in D though built differently (SPEC §7.4.1
        # "equal values built through different histories"): must decide to 0.
        self.assertEqual(
            stack_of("'MATH' IMPORT 8 SQRT 2 SQRT 2 SQRT ADD 4 COMPARE-WITHIN"),
            ["0/1"],
        )

    def test_rational_pairs_decide(self):
        self.assertEqual(stack_of("3 3 10 COMPARE-WITHIN"), ["0/1"])
        self.assertEqual(stack_of("3 5 10 COMPARE-WITHIN"), ["-1/1"])
        self.assertEqual(stack_of("5 3 10 COMPARE-WITHIN"), ["1/1"])


class CompareWithinContracts(unittest.TestCase):
    """The budget, NIL, and malformed-use contracts are preserved by the fix."""

    def test_nil_operand_passthrough(self):
        # NIL a or b propagates as NIL (Section 7.12), not UNKNOWN.
        result = repro.run_program("1 0 DIV 2 10 COMPARE-WITHIN")
        self.assertEqual(result["status"], "ok")
        self.assertEqual(len(result["stack"]), 1)
        self.assertIn("NIL", result["stack"][0].upper())

    def test_non_positive_budget_is_error(self):
        self.assertEqual(
            repro.run_program("2 3 0 COMPARE-WITHIN")["status"], "error"
        )

    def test_non_integer_budget_is_error(self):
        self.assertEqual(
            repro.run_program("2 3 5 2 DIV COMPARE-WITHIN")["status"], "error"
        )

    def test_non_numeric_operand_is_error(self):
        self.assertEqual(
            repro.run_program("'x' 3 10 COMPARE-WITHIN")["status"], "error"
        )


if __name__ == "__main__":
    unittest.main()
