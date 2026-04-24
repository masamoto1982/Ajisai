// AQ-VER-001: Fraction MC/DC tests for QL-A boolean decisions.
//
// Scope: numeric core (`Fraction`) — boolean decisions whose
// independent atomic conditions can each cause an incorrect
// arithmetic or comparison result if mis-evaluated.
//
// Each submodule below documents:
//   * DUT (decision under test) — file:line and the boolean expression
//   * Conditions — atomic predicates A, B, ...
//   * MC/DC table — pairs of rows that demonstrate each condition
//     independently flipping the outcome
//
// Trace: docs/quality/TRACEABILITY_MATRIX.md, requirement AQ-REQ-001.

#![cfg(test)]

use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use std::str::FromStr;

fn small(n: i64, d: i64) -> Fraction {
    Fraction::new(BigInt::from(n), BigInt::from(d))
}

fn big_int(n: i64) -> Fraction {
    let big = BigInt::from(i64::MAX) * BigInt::from(2i64) + BigInt::from(n);
    Fraction::new(big, BigInt::from(1))
}

// AQ-VER-001-A
// DUT: rust/src/types/fraction.rs:58-60 in `impl PartialEq for Fraction`
//
//     if self.is_nil() || other.is_nil() {
//         return self.is_nil() && other.is_nil();
//     }
//
// Conditions:
//   A = self.is_nil()
//   B = other.is_nil()
//
// MC/DC for A||B (entry guard):
//   row 1: (A=T, B=F) -> guard taken
//   row 2: (A=F, B=T) -> guard taken
//   row 3: (A=F, B=F) -> guard skipped
//   Pair (1,3) shows A independently flips outcome (B held F).
//   Pair (2,3) shows B independently flips outcome (A held F).
//
// MC/DC for A&&B (returned value when guard taken):
//   row 4: (A=T, B=T) -> true
//   row 5: (A=T, B=F) -> false
//   row 6: (A=F, B=T) -> false
//   Pair (4,5) shows B independently flips outcome (A held T).
//   Pair (4,6) shows A independently flips outcome (B held T).
mod nil_equality_guard {
    use super::*;

    #[test]
    fn aq_ver_001_a_row1_self_nil_other_nonnil_guard_taken_returns_false() {
        // (A=T, B=F): guard taken, inner A&&B = false -> not equal.
        let lhs = Fraction::nil();
        let rhs = small(1, 1);
        assert_ne!(lhs, rhs, "nil and non-nil must not be equal");
    }

    #[test]
    fn aq_ver_001_a_row2_other_nil_self_nonnil_guard_taken_returns_false() {
        // (A=F, B=T): guard taken, inner A&&B = false -> not equal.
        let lhs = small(1, 1);
        let rhs = Fraction::nil();
        assert_ne!(lhs, rhs, "non-nil and nil must not be equal");
    }

    #[test]
    fn aq_ver_001_a_row3_neither_nil_guard_skipped_compares_repr() {
        // (A=F, B=F): guard skipped, falls through to repr comparison.
        let lhs = small(2, 4); // reduces to 1/2
        let rhs = small(1, 2);
        assert_eq!(lhs, rhs, "1/2 and 2/4 must compare equal after reduction");

        let unequal = small(1, 3);
        assert_ne!(lhs, unequal);
    }

    #[test]
    fn aq_ver_001_a_row4_both_nil_returns_true() {
        // (A=T, B=T): inner A&&B = true -> nil equals nil.
        let lhs = Fraction::nil();
        let rhs = Fraction::nil();
        assert_eq!(lhs, rhs, "nil must equal nil");
    }

    // Rows (T,F) and (F,T) coincide with rows 1 and 2 above for the inner
    // A&&B decision: in both cases the inner expression yields false, which
    // is the outcome being verified. No additional cases required.
}

// AQ-VER-001-B
// DUT: rust/src/types/fraction.rs:369-376 in `impl Ord for Fraction::cmp`
//
//     if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(),
//                                            other.extract_i64_pair()) {
//         if b == d { return a.cmp(&c); }
//         ...
//     }
//
// Conditions for the same-denominator branch:
//   A = self.extract_i64_pair().is_some()
//   B = other.extract_i64_pair().is_some()
//   C = b == d
//
// We test the three boundary combinations that exercise the if-let pattern
// (small/small, small/big, big/big) and, within the small/small arm, both
// values of C.
mod cmp_small_fast_path {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn aq_ver_001_b_small_small_same_denominator_compares_numerators() {
        // A=T, B=T, C=T: enters fast path, compares a vs c directly.
        let a = small(3, 7);
        let b = small(5, 7);
        assert_eq!(a.cmp(&b), Ordering::Less);
        assert_eq!(b.cmp(&a), Ordering::Greater);
        assert_eq!(a.cmp(&a.clone()), Ordering::Equal);
    }

    #[test]
    fn aq_ver_001_b_small_small_different_denominator_cross_multiplies() {
        // A=T, B=T, C=F: skips a.cmp(&c), takes the i128 cross-multiply path.
        // 1/3 vs 1/4 -> 4 vs 3 -> Greater.
        let a = small(1, 3);
        let b = small(1, 4);
        assert_eq!(a.cmp(&b), Ordering::Greater);
    }

    #[test]
    fn aq_ver_001_b_big_path_when_extraction_fails() {
        // A=F or B=F: at least one side is BigInt, falls through to BigInt
        // arithmetic. The same-denominator BigInt branch must also work.
        let a = big_int(0); // 2*i64::MAX, denominator 1
        let b = big_int(1); // 2*i64::MAX + 1, denominator 1
        assert_eq!(a.cmp(&b), Ordering::Less);
        assert_eq!(b.cmp(&a), Ordering::Greater);

        // Different-denominator BigInt path.
        let c = Fraction::new(BigInt::from(1), BigInt::from(i64::MAX) * BigInt::from(2));
        let d = Fraction::new(BigInt::from(1), BigInt::from(i64::MAX) * BigInt::from(3));
        assert_eq!(c.cmp(&d), Ordering::Greater);
    }
}

// AQ-VER-001-C
// DUT: rust/src/types/fraction.rs:232 in `Fraction::as_usize` (Small arm)
//
//     if *d == 1 && *n >= 0 { Some(*n as usize) } else { None }
//
// Conditions:
//   A = (d == 1)
//   B = (n >= 0)
//
// MC/DC table:
//   row 1: (T, T) -> Some
//   row 2: (T, F) -> None  (proves B flips outcome with A held T)
//   row 3: (F, T) -> None  (proves A flips outcome with B held T)
//   row 4: (F, F) -> None  (boundary; not required for MC/DC but documented)
//
// Note on reachability: a normalized Small fraction with d != 1 always has
// gcd(n, d) = 1, so non-integer values (rows 3, 4) are constructible via
// Fraction::new and remain in Small form when both numerator and denominator
// fit in i64.
mod as_usize_small {
    use super::*;

    #[test]
    fn aq_ver_001_c_row1_integer_nonneg_returns_some() {
        let f = small(42, 1);
        assert_eq!(f.as_usize(), Some(42));
    }

    #[test]
    fn aq_ver_001_c_row2_integer_negative_returns_none() {
        let f = small(-1, 1);
        assert_eq!(f.as_usize(), None, "negative integer must not coerce to usize");
    }

    #[test]
    fn aq_ver_001_c_row3_nonintegral_positive_returns_none() {
        let f = small(3, 4);
        assert_eq!(f.as_usize(), None, "non-integer fraction must not coerce");
    }

    #[test]
    fn aq_ver_001_c_row4_nonintegral_negative_returns_none() {
        let f = small(-3, 4);
        assert_eq!(f.as_usize(), None);
    }
}

// AQ-VER-001-D
// DUT: rust/src/types/fraction-arithmetic.rs:54-58 in `Fraction::add`
//
//     if b == 1 && d == 1 {
//         return Self::create_from_i128((a as i128) + (c as i128), 1);
//     }
//     if b == d {
//         return Self::create_from_i128((a as i128) + (c as i128), b as i128);
//     }
//
// Decision 1 conditions (integer fast path):
//   A = (b == 1)
//   B = (d == 1)
//
// MC/DC table for A&&B:
//   row 1: (T, T) -> integer fast path
//   row 2: (T, F) -> falls through (B flips outcome with A held T)
//   row 3: (F, T) -> falls through (A flips outcome with B held T)
//
// Decision 2 conditions (same-denominator fast path):
//   C = (b == d)  reached when not (A && B)
//   row 4: C = T  -> common-denominator fast path
//   row 5: C = F  -> generic cross-multiply path
mod add_small_fast_paths {
    use super::*;

    #[test]
    fn aq_ver_001_d_decision1_row1_both_integer() {
        // (A=T, B=T): integer + integer fast path.
        let a = small(7, 1);
        let b = small(11, 1);
        let sum = a.add(&b);
        assert_eq!(sum, small(18, 1));
    }

    #[test]
    fn aq_ver_001_d_decision1_row2_self_integer_other_fraction() {
        // (A=T, B=F): self denominator 1, other not -> falls through to
        // Decision 2; b=1, d=3 -> b != d -> generic path. 7 + 1/3 = 22/3.
        let a = small(7, 1);
        let b = small(1, 3);
        let sum = a.add(&b);
        assert_eq!(sum, small(22, 3));
    }

    #[test]
    fn aq_ver_001_d_decision1_row3_self_fraction_other_integer() {
        // (A=F, B=T): symmetric to row 2. 1/3 + 7 = 22/3.
        let a = small(1, 3);
        let b = small(7, 1);
        let sum = a.add(&b);
        assert_eq!(sum, small(22, 3));
    }

    #[test]
    fn aq_ver_001_d_decision2_row4_same_denominator_nonunit() {
        // (A=F, B=F, C=T): both have denominator 5 -> common-den fast path.
        let a = small(2, 5);
        let b = small(1, 5);
        let sum = a.add(&b);
        assert_eq!(sum, small(3, 5));
    }

    #[test]
    fn aq_ver_001_d_decision2_row5_different_denominator() {
        // (A=F, B=F, C=F): generic cross-multiply path.
        // 1/3 + 1/4 = 7/12.
        let a = small(1, 3);
        let b = small(1, 4);
        let sum = a.add(&b);
        assert_eq!(sum, small(7, 12));
    }

    #[test]
    fn aq_ver_001_d_nil_short_circuit_takes_precedence() {
        // Documented invariant: nil propagates regardless of which side.
        // Outside the decisions above but covers the entry guard.
        let nil = Fraction::nil();
        let one = small(1, 1);
        assert!(nil.add(&one).is_nil());
        assert!(one.add(&nil).is_nil());
    }
}

// AQ-VER-001-E
// DUT: rust/src/types/fraction-arithmetic.rs:268 in `Fraction::floor` (Small)
//
//     let floored = if *n < 0 && r != 0 { q - 1 } else { q };
//
// Conditions:
//   A = (n < 0)
//   B = (r != 0)
//
// Reachability note: this code runs only after `is_integer()` returns false,
// so when `repr` is `Small` we have d > 1. Because `Fraction::new` reduces by
// gcd, any Small with d > 1 satisfies gcd(n, d) = 1, hence n % d != 0. So in
// normal use B = T always, and rows (T,F)/(F,F) are unreachable. We still
// exercise both A values to demonstrate independent effect of A, and we
// exercise the early-return path for integers as a separate row.
mod floor_negative_remainder {
    use super::*;

    #[test]
    fn aq_ver_001_e_row1_negative_with_remainder_rounds_toward_neg_inf() {
        // (A=T, B=T): -7/3 -> q=-2, r=-1, floored = -3.
        let f = small(-7, 3);
        assert_eq!(f.floor(), small(-3, 1));
    }

    #[test]
    fn aq_ver_001_e_row2_positive_with_remainder_truncates() {
        // (A=F, B=T): 7/3 -> q=2, r=1, floored = 2 (else branch).
        // Pair (row1, row2) holds B=T and flips A, demonstrating A's
        // independent effect on the outcome.
        let f = small(7, 3);
        assert_eq!(f.floor(), small(2, 1));
    }

    #[test]
    fn aq_ver_001_e_integer_short_circuits_before_decision() {
        // is_integer() short-circuit returns self before the decision is
        // evaluated. Documents the only realistic way to reach B = F.
        let f = small(-6, 1);
        assert_eq!(f.floor(), small(-6, 1));
    }

    #[test]
    fn aq_ver_001_e_big_path_negative_with_remainder() {
        // BigInt twin of row 1 to cover the parallel decision in the Big arm.
        // -18446744073709551616 (= -(2*i64::MAX + 2)) does not fit i64 so the
        // value stays in the Big representation. Divided by 3 it has trunc
        // quotient -6148914691236517205 with remainder -1, so floor = q - 1.
        let big_neg = BigInt::from_str("-18446744073709551616").unwrap();
        let f = Fraction::new(big_neg, BigInt::from(3));
        let expected = Fraction::new(
            BigInt::from_str("-6148914691236517206").unwrap(),
            BigInt::from(1),
        );
        assert_eq!(f.floor(), expected);
    }
}

// AQ-VER-001-F
// DUT: rust/src/types/fraction-arithmetic.rs:293 in `Fraction::ceil` (Small)
//
//     let ceiled = if *n > 0 && r != 0 { q + 1 } else { q };
//
// Conditions:
//   A = (n > 0)
//   B = (r != 0)
//
// Same reachability caveat as AQ-VER-001-E.
mod ceil_positive_remainder {
    use super::*;

    #[test]
    fn aq_ver_001_f_row1_positive_with_remainder_rounds_toward_pos_inf() {
        // (A=T, B=T): 7/3 -> q=2, r=1, ceiled = 3.
        let f = small(7, 3);
        assert_eq!(f.ceil(), small(3, 1));
    }

    #[test]
    fn aq_ver_001_f_row2_negative_with_remainder_truncates() {
        // (A=F, B=T): -7/3 -> q=-2, r=-1, ceiled = -2 (else branch).
        // Pair (row1, row2) flips A with B held T -> independent effect of A.
        let f = small(-7, 3);
        assert_eq!(f.ceil(), small(-2, 1));
    }

    #[test]
    fn aq_ver_001_f_zero_short_circuits_via_is_integer() {
        // Zero is is_integer() == true (d == 1 after reduction), short-circuits.
        let f = small(0, 5);
        assert_eq!(f.ceil(), small(0, 1));
    }
}
