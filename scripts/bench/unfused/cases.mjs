// Cases shared by breakdown.mjs (timing) and count-calls.mjs (call counts).
export const INT = (n) => `0 ${n - 1} RANGE`;
export const FRAC = (n) => `0 ${n - 1} RANGE [ 1/3 ADD ] MAP`;
export const BIG = (n) => `0 ${n - 1} RANGE [ 100000000000000000000000 ADD ] MAP`; // 10^23 > i64
export const SUM = "[ 0 ] [ ADD ] FOLD";

// kind "map": setup(n) builds the operand; src is timed; ref is a fused program
// over the same operand that must give the same Vector.
// kind "top": src(n) is a whole top-level program that leaves one scalar.
export const CASE_TABLE = [
  { id: "A",   kind: "map", what: "fused MAP, integer +1 (baseline)", setup: INT, src: "[ 1 ADD ] MAP", ref: "[ 1 ADD ] MAP" },
  { id: "B",   kind: "map", what: "unfused MAP: +1 then 1 POW (POW is outside the fused subset)", setup: INT, src: "[ 1 ADD 1 POW ] MAP", ref: "[ 1 ADD ] MAP" },
  { id: "B2",  kind: "map", what: "B with a second 1 ADD (marginal quickened ADD)", setup: INT, src: "[ 1 ADD 1 ADD 1 POW ] MAP", ref: "[ 2 ADD ] MAP" },
  { id: "BB",  kind: "map", what: "B with 'X' BIND X in front (BIND in an unfused block)", setup: INT, src: "[ 'X' BIND X 1 ADD 1 POW ] MAP", ref: "[ 1 ADD ] MAP" },
  { id: "C",   kind: "map", what: "B without the arithmetic: 1 POW only", setup: INT, src: "[ 1 POW ] MAP", ref: "[ 0 ADD ] MAP" },
  { id: "C2",  kind: "map", what: "C with a second 1 POW (marginal fully-dispatched Word)", setup: INT, src: "[ 1 POW 1 POW ] MAP", ref: "[ 0 ADD ] MAP" },
  { id: "K",   kind: "map", what: "identity through a cheaper decliner: [ ] LENGTH ADD (adds 0)", setup: INT, src: "[ [ ] LENGTH ADD ] MAP", ref: "[ 0 ADD ] MAP" },
  { id: "K2",  kind: "map", what: "K twice (marginal literal + LENGTH + quickened ADD)", setup: INT, src: "[ [ ] LENGTH ADD [ ] LENGTH ADD ] MAP", ref: "[ 0 ADD ] MAP" },
  { id: "D",   kind: "map", what: "B on small fractions (k+1/3)", setup: FRAC, src: "[ 1 ADD 1 POW ] MAP", ref: "[ 1 ADD ] MAP" },
  { id: "DC",  kind: "map", what: "C on small fractions", setup: FRAC, src: "[ 1 POW ] MAP", ref: "[ 0 ADD ] MAP" },
  { id: "E",   kind: "map", what: "B on integers past i64 (10^23+k)", setup: BIG, src: "[ 1 ADD 1 POW ] MAP", ref: "[ 1 ADD ] MAP" },
  { id: "EC",  kind: "map", what: "C on integers past i64", setup: BIG, src: "[ 1 POW ] MAP", ref: "[ 0 ADD ] MAP" },
  { id: "F",   kind: "map", what: "vector literal: [ 1 ] LENGTH ADD", setup: INT, src: "[ [ 1 ] LENGTH ADD ] MAP", ref: "[ 1 ADD ] MAP" },
  { id: "F8",  kind: "map", what: "8-element vector literal: [ 1 x8 ] LENGTH ADD", setup: INT, src: "[ [ 1 1 1 1 1 1 1 1 ] LENGTH ADD ] MAP", ref: "[ 8 ADD ] MAP" },
  { id: "FR",  kind: "map", what: "F's shape with 1 1 POW in place of [ 1 ] LENGTH", setup: INT, src: "[ 1 1 POW ADD ] MAP", ref: "[ 1 ADD ] MAP" },
  { id: "G",   kind: "top", what: "top level: 0 then N x ' 1 ADD' on one line", src: (n) => "0" + " 1 ADD".repeat(n), refValue: (n) => n },
  { id: "GL",  kind: "top", what: "top level: 0 then N lines of '1 ADD'", src: (n) => "0" + "\n1 ADD".repeat(n), refValue: (n) => n },
  { id: "GP",  kind: "top", what: "top level: 0 then N x ' 1 POW' (dispatch only)", src: (n) => "0" + " 1 POW".repeat(n), refValue: () => 0 },
];
