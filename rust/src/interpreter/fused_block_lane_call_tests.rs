//! Fused walks over a one-lane seed and through User Word calls, against
//! the interpreted walk (`fused_block_tests` holds the comparison).

use crate::interpreter::fused_block_tests::{assert_same, Limits};

#[test]
fn lane_and_call_programs_agree() {
    for source in [
        // A one-lane seed, the idiomatic `[ 0 ]`: the lane beside a scalar
        // (no fast-path hit) and beside itself (a hit), on both tiers, as
        // FOLD and SCAN; and the Words that do not treat a lane as its value.
        "1 600 RANGE [ 0 ] [ ADD ] FOLD",
        "1 600 RANGE [ 0 ] [ ADD ] SCAN",
        "1 30 RANGE [ 1 ] [ MUL ] FOLD",
        "1 70 RANGE [ 1 ] [ MUL ] FOLD",
        "1 600 RANGE [ 7 ] [ SUB ] FOLD",
        "1 600 RANGE [ 7 ] [ 'E' BIND 'A' BIND E A SUB ] FOLD",
        "1 600 RANGE [ 0 ] [ 'E' BIND 'A' BIND A A ADD E SUB ] SCAN",
        "1 600 RANGE [ 0 ] [ 'E' BIND 'A' BIND A E E MUL ADD ] FOLD",
        "1 600 RANGE [ 0 ] [ 'E' BIND 'A' BIND E ] FOLD",
        "1 600 RANGE [ 0 ] [ 'E' BIND ] FOLD",
        "1 600 RANGE [ 1/2 ] [ ADD ] FOLD",
        "1 1 60 RANGE DIV [ 0 ] [ ADD ] FOLD",
        "1 1 60 RANGE DIV [ 0 ] [ ADD ] SCAN",
        "1 1 30 RANGE DIV [ 0 ] [ ADD ] FOLD",
        "[ 1 2 3 ] [ 10 ] [ DIV ] FOLD",
        "[ 1 0 2 ] [ 1 ] [ DIV ] FOLD",
        "[ 3 0 2 ] [ 1 ] [ 'E' BIND 'A' BIND E A DIV ] FOLD",
        "1 600 RANGE [ 9223372036854775000 ] [ ADD ] FOLD",
        "[ 1 NIL 2 ] [ 0 ] [ ADD ] FOLD",
        "[ 1 2 ] [ 0 ] [ 2 SQRT ADD ] FOLD",
        "1 20 RANGE [ 0 ] [ 'E' BIND 'A' BIND A 10 LT ] FOLD",
        "1 20 RANGE [ 0 ] [ 'E' BIND 'A' BIND A 5 EQ ] FOLD",
        "1 20 RANGE [ 0 ] [ ADD FLOOR ] FOLD",
        "1 20 RANGE [ 0 ] [ 'E' BIND 'A' BIND A A A TRUE SELECT ] FOLD",
        "1 20 RANGE [ 0 ] [ 'E' BIND 'A' BIND E 1 ADD ] FOLD",
        "1 20 RANGE [ 0 ] [ 'E' BIND 'A' BIND A 5 LT 'T' BIND A E ADD ] FOLD",
        "1 20 RANGE [ 0 ] [ 'E' BIND 'A' BIND A FLOOR 'T' BIND A E ADD ] FOLD",
        // A lane that outgrows a machine word part way and comes back.
        "[ 9223372036854775807 2 0 ] [ 1 ] [ MUL ] FOLD",
        "[ 9223372036854775807 2 0 ] [ 1/3 ] [ MUL ] SCAN",
        "1 20 RANGE [ 0 1 ] [ ADD ] FOLD",
        // User Words, inlined: a first call that builds the plan, a plan
        // already current, a Word called twice and one calling another, a
        // body binding the same name as the block, a body reading a name
        // only the caller bound (an ERROR across the barrier), a body that
        // consumes the caller's values, and bodies outside the subset.
        "[ 3 MUL 1 ADD ] 'F' DEF 1 600 RANGE [ F ] MAP",
        "[ 3 MUL 1 ADD ] 'F' DEF 1 2 RANGE [ F ] MAP 1 600 RANGE [ F ] MAP",
        "[ 2 MUL ] 'D' DEF [ D 1 ADD D ] 'G' DEF 1 600 RANGE [ G ] MAP",
        "[ 2 MUL ] 'D' DEF [ D 1 ADD D ] 'G' DEF 1 600 RANGE [ G D ] MAP",
        "[ 'X' BIND X X MUL ] 'SQ' DEF 1 600 RANGE [ 'X' BIND X SQ X ADD ] MAP",
        "[ X 1 ADD ] 'BADX' DEF 1 20 RANGE [ 'X' BIND X BADX ] MAP",
        "[ K 1 ADD ] 'BADK' DEF 5 'K' BIND 1 20 RANGE [ BADK ] MAP",
        "[ ADD ] 'PLUS' DEF 1 600 RANGE 0 [ PLUS ] FOLD",
        "[ ADD ] 'PLUS' DEF 1 600 RANGE [ 0 ] [ PLUS ] FOLD",
        "[ ADD ] 'PLUS' DEF 1 600 RANGE [ 5 PLUS ] MAP",
        "[ ADD ] 'PLUS' DEF 1 20 RANGE [ PLUS ] MAP",
        "[ 3 GT ] 'BIG' DEF 1 600 RANGE [ BIG ] FILTER",
        "[ 1 SWAP DIV ] 'INV' DEF [ 1 0 2 ] [ INV ] MAP",
        "[ 1 DIV ] 'INV' DEF [ 1 0 2 ] [ 'X' BIND 1 X INV ] MAP",
        "[ [ 1 ] ADD ] 'VADD' DEF 1 20 RANGE [ VADD ] MAP",
        "[ 2 MUL ] 'D' DEF 1 20 RANGE [ D ] MAP [ 3 ADD ] 'D' DEF 1 20 RANGE [ D ] MAP",
        // A plan made stale by an unrelated DEF is rebuilt, not reused.
        "[ 2 MUL ] 'D' DEF 1 2 RANGE [ D ] MAP [ 1 ] 'OTHER' DEF 1 600 RANGE [ D ] MAP",
        "[ 2 MUL ] 'D' DEF [ D ] 'E' DEF [ E ] 'F' DEF [ F ] 'G' DEF 1 600 RANGE [ G ] MAP",
        "1 20 RANGE [ [ 0 ] ] [ ADD ] FOLD",
        "1 20 RANGE [ TRUE ] [ ADD ] FOLD",
    ] {
        assert_same(source, Limits::default());
    }
}
