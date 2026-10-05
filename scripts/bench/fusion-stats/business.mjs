// The business-calculation and cross-language comparison programs of the two
// earlier speed reports, verbatim (sizes as reported there).
const F = "earlier reports (session)";
export const BUSINESS = [
  { origin: "business: ledger 100k lines (price x qty x tax, floor, total)", program: "1 100000 RANGE [ 'N' BIND N N 37 DIV FLOOR 37 MUL SUB 1.99 MUL 1.1 MUL FLOOR ] MAP [ 0 ] [ ADD ] FOLD" },
  { origin: "business: variance of 100k decimals", program: "1 100000 RANGE [ 0.01 MUL ] MAP 'V' BIND V [ 0 ] [ ADD ] FOLD 100000 DIV 'M' BIND V M SUB [ 'D' BIND D D MUL ] MAP [ 0 ] [ ADD ] FOLD 100000 DIV" },
  { origin: "business: harmonic sum 1..2000", program: "1 2000 RANGE 'X' BIND 1 X DIV [ 0 ] [ ADD ] FOLD" },
  { origin: "business: compound interest 360 months", program: "1 360 RANGE [ 1000000 ] [ 'E' BIND 'X' BIND X 1.005 MUL ] FOLD" },
  { origin: "business: logistic map 14 steps", program: "1 14 RANGE [ 1/10 ] [ 'E' BIND 'X' BIND X 1 X SUB MUL 7/2 MUL ] FOLD" },
  { origin: "business: sum of square roots 1..20", program: "1 20 RANGE SQRT [ 0 ] [ ADD ] FOLD" },
  { origin: "business: 0.1 ten times equals 1", program: "0.1 0.1 0.1 0.1 0.1 0.1 0.1 0.1 0.1 0.1 ADD ADD ADD ADD ADD ADD ADD ADD ADD 1 EQ" },
  { origin: "comparison: sum of 0..999999", program: "0 999999 RANGE 0 [ ADD ] FOLD" },
  { origin: "comparison: 3x+1 over 1e6", program: "0 999999 RANGE [ 3 MUL 1 ADD ] MAP" },
  { origin: "comparison: filter > 500000 over 1e6", program: "0 999999 RANGE [ 500000 GT ] FILTER" },
  { origin: "comparison: collatz step over 1e6", program: "0 999999 RANGE [ 'N' BIND N 2 DIV N 3 MUL 1 ADD N 2 DIV FLOOR 2 MUL N EQ SELECT ] MAP" },
  { origin: "comparison: (x+1/3)*2 over 1e6", program: "0 999999 RANGE [ 1/3 ADD 2 MUL ] MAP" },
].map((b) => ({ ...b, file: F }));
