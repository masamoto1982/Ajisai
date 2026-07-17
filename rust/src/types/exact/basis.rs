//! GCD-free radicand basis for the Tier 1 algebraic normal form.
//!
//! Instead of factoring radicands into primes (unbounded cost for large
//! semiprimes), the normal form works over a **GCD-free basis**:
//! pairwise-coprime integers ≥ 2, none a perfect square, covering every
//! radicand as a product of powers. Distinct subset products of such a
//! basis are never perfect squares — exactly the hypothesis the
//! linear-independence theorem needs — so a coefficient map keyed by
//! subset products is a true normal form; primality is never required.
//! (Same construction as the §4.2.7 comparison pre-pass this module's
//! type grew out of.)

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Zero};

/// Pairwise-coprime positive integers, each ≥ 2 and none a perfect
/// square, kept sorted. Every monomial of an [`super::algebraic::Algebraic`]
/// value is a product of a distinct subset of its basis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Basis(Vec<BigInt>);

impl Basis {
    /// The empty basis (rational values only).
    pub(crate) fn empty() -> Basis {
        Basis(Vec::new())
    }

    pub(crate) fn elements(&self) -> &[BigInt] {
        &self.0
    }

    /// Build a GCD-free basis covering every input integer. Inputs ≤ 1
    /// are ignored (they carry no radical content).
    pub(crate) fn build(inputs: Vec<BigInt>) -> Basis {
        let one = BigInt::one();
        let mut elems: Vec<BigInt> = inputs.into_iter().filter(|n| *n > one).collect();
        elems.sort();
        elems.dedup();
        // GCD-free refinement: replace any pair sharing a factor by
        // {gcd, a/gcd, b/gcd}. Each replacement strictly decreases the
        // product of all elements, so the loop terminates.
        loop {
            let mut shared: Option<(usize, usize)> = None;
            'search: for i in 0..elems.len() {
                for j in (i + 1)..elems.len() {
                    if !elems[i].gcd(&elems[j]).is_one() {
                        shared = Some((i, j));
                        break 'search;
                    }
                }
            }
            let Some((i, j)) = shared else { break };
            let a = elems.swap_remove(j);
            let b = elems.swap_remove(i);
            let g = a.gcd(&b);
            for part in [&a / &g, &b / &g, g] {
                if part > one && !elems.contains(&part) {
                    elems.push(part);
                }
            }
        }
        // Square reduction: a basis element that is a perfect square would
        // make √element rational, breaking independence. Replacing b = c²
        // by c preserves pairwise coprimality (any common factor of c and
        // another element would divide b too) and keeps every input a
        // product of basis-element powers.
        for e in &mut elems {
            loop {
                let root = e.sqrt();
                if &(&root * &root) == e {
                    *e = root;
                } else {
                    break;
                }
            }
        }
        elems.sort();
        Basis(elems)
    }

    /// The common refinement of two bases plus any extra radicands: the
    /// GCD-free basis over which every monomial of either side (and every
    /// extra radicand) decomposes.
    pub(crate) fn merged(a: &Basis, b: &Basis, extra: &[BigInt]) -> Basis {
        let mut inputs: Vec<BigInt> = Vec::with_capacity(a.0.len() + b.0.len() + extra.len());
        inputs.extend(a.0.iter().cloned());
        inputs.extend(b.0.iter().cloned());
        inputs.extend(extra.iter().cloned());
        Self::build(inputs)
    }

    /// √n as `outside · √monomial` over this basis: factor n into
    /// basis-element powers, halve even exponents into `outside`, and keep
    /// the odd-exponent elements' product as the monomial key. `None` when
    /// n is not a product of basis-element powers — the callers all build
    /// the basis from the very integers they decompose, so `None` is a
    /// defensive impossibility, never a wrong answer.
    pub(crate) fn decompose_sqrt(&self, n: &BigInt) -> Option<(BigInt, BigInt)> {
        let mut rest = n.clone();
        let mut outside = BigInt::one();
        let mut monomial = BigInt::one();
        for b in &self.0 {
            let mut exp = 0u32;
            while (&rest % b).is_zero() {
                rest /= b;
                exp += 1;
            }
            outside *= b.pow(exp / 2);
            if exp % 2 == 1 {
                monomial *= b;
            }
        }
        if !rest.is_one() {
            return None;
        }
        Some((outside, monomial))
    }

    /// Drop the elements dividing none of `monomials`, keeping merges
    /// between long-lived values from accreting dead radicands.
    pub(crate) fn pruned_to<'a>(
        &self,
        monomials: impl Iterator<Item = &'a BigInt> + Clone,
    ) -> Basis {
        let kept: Vec<BigInt> = self
            .0
            .iter()
            .filter(|b| monomials.clone().any(|m| (m % *b).is_zero()))
            .cloned()
            .collect();
        Basis(kept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bi(n: i64) -> BigInt {
        BigInt::from(n)
    }

    #[test]
    fn build_refines_shared_factors_and_squares() {
        // {6, 10} share 2 → {2, 3, 5}.
        let b = Basis::build(vec![bi(6), bi(10)]);
        assert_eq!(b.elements(), &[bi(2), bi(3), bi(5)]);
        // A perfect-square input reduces to its root.
        let b = Basis::build(vec![bi(49)]);
        assert_eq!(b.elements(), &[bi(7)]);
        // Inputs ≤ 1 are ignored.
        let b = Basis::build(vec![bi(1), bi(0)]);
        assert_eq!(b.elements(), &[] as &[BigInt]);
    }

    #[test]
    fn decompose_sqrt_splits_square_part_from_monomial() {
        let b = Basis::build(vec![bi(2), bi(3)]);
        // √72 = √(2³·3²) = 2·3·√2.
        let (outside, monomial) = b.decompose_sqrt(&bi(72)).unwrap();
        assert_eq!(outside, bi(6));
        assert_eq!(monomial, bi(2));
        // √1 = 1·√1.
        let (outside, monomial) = b.decompose_sqrt(&bi(1)).unwrap();
        assert_eq!(outside, bi(1));
        assert_eq!(monomial, bi(1));
        // 5 is not covered by {2, 3}.
        assert!(b.decompose_sqrt(&bi(5)).is_none());
    }

    #[test]
    fn merged_covers_monomials_of_both_sides() {
        // √12 alone keeps the coarse basis {12}; meeting √3 refines it.
        let a = Basis::build(vec![bi(12)]);
        assert_eq!(a.elements(), &[bi(12)]);
        let b = Basis::build(vec![bi(3)]);
        let m = Basis::merged(&a, &b, &[]);
        assert_eq!(m.elements(), &[bi(2), bi(3)]);
        // The old coarse monomial still decomposes over the refinement.
        let (outside, monomial) = m.decompose_sqrt(&bi(12)).unwrap();
        assert_eq!(outside, bi(2));
        assert_eq!(monomial, bi(3));
    }

    #[test]
    fn pruned_to_drops_unused_elements() {
        let b = Basis::build(vec![bi(2), bi(3), bi(5)]);
        let monomials = [bi(6)];
        let p = b.pruned_to(monomials.iter());
        assert_eq!(p.elements(), &[bi(2), bi(3)]);
    }
}
