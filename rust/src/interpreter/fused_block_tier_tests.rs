//! Which tier of the fused walk (`fused_block`) answers a walk: the
//! small-rational tier (`fused_block_rat`) takes the fractional walks the
//! integer tier declines, numbers over zero among them, and steps aside for
//! what does not fit it. `fused_block_tests` holds the tiers' answers to the
//! interpreted walk's.

use crate::interpreter::Interpreter;

fn rat_runs(source: &str) -> u64 {
    let before = crate::interpreter::fused_block_rat::rat_runs_on_this_thread();
    let mut interp = Interpreter::new();
    let _ = crate::agent::block_on(interp.execute(source));
    crate::interpreter::fused_block_rat::rat_runs_on_this_thread() - before
}

/// The small-rational tier answers the fractional walks the integer tier
/// declines, and steps aside for what does not fit it.
#[test]
fn the_rational_tier_is_taken_where_it_applies() {
    assert_eq!(rat_runs("1 1 1000 RANGE DIV [ 2 MUL ] MAP"), 1);
    assert_eq!(rat_runs("1 1000 RANGE [ 7 DIV 1/3 ADD FLOOR ] MAP"), 1);
    assert_eq!(rat_runs("1 1 1000 RANGE DIV [ 1/2 GT ] FILTER"), 1);
    assert_eq!(rat_runs("1 1 30 RANGE DIV 0 [ ADD ] FOLD"), 1);
    // Integers stay on the integer tier; overflow leaves.
    assert_eq!(rat_runs("1 1000 RANGE [ 2 MUL ] MAP"), 0);
    assert_eq!(rat_runs("1 1 60 RANGE DIV 0 [ ADD ] FOLD"), 0);
    // A zero divisor answers a point over zero, which is a pair like any
    // other here (`small_rational::div_total`): the walk stays on this tier.
    // It used to leave for the general tier, when the pair laws assumed a
    // positive denominator.
    assert_eq!(rat_runs("1 1 60 RANGE DIV [ 0 DIV ] MAP"), 1);
}

/// Elements over zero, and a zero divisor the integer tier meets, stay on the
/// small-rational tier (`small_rational`'s total forms); an order asked of
/// `0/0` has no answer on any fused tier.
#[test]
fn points_over_zero_stay_on_the_rational_tier() {
    assert_eq!(rat_runs("1 [ 0 1 2 ] DIV [ 2 MUL ] MAP"), 1);
    assert_eq!(rat_runs("[ 1 0 2 ] [ 'X' BIND 1 X DIV ] MAP"), 1);
    assert_eq!(rat_runs("[ 1/0 -1/0 0/0 5/2 ] [ FLOOR ] MAP"), 1);
    assert_eq!(rat_runs("[ 1/0 -1/0 2 ] [ 1/2 GT ] FILTER"), 1);
    assert_eq!(rat_runs("[ 1 2 -3 ] 1/0 [ MUL ] SCAN"), 1);
    assert_eq!(rat_runs("[ 1/0 0/0 2 ] [ 1/2 GT ] FILTER"), 0);
    assert_eq!(rat_runs("[ 0/0 1 ] [ 3 MIN ] MAP"), 0);
}
