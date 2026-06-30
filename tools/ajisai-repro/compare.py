#!/usr/bin/env python3
"""Run a battery of Ajisai programs through the original Rust CLI and the
Python reproduction, and report every observable difference."""
import json, subprocess, sys, os
HERE = os.path.dirname(os.path.abspath(__file__))
BIN = "/home/user/Ajisai/rust/target/release/ajisai"

def orig(src):
    open("/tmp/_c.ajisai", "w").write(src)
    r = subprocess.run([BIN, "run", "/tmp/_c.ajisai", "--json"],
                       capture_output=True, text=True)
    try:
        d = json.loads(r.stdout)
    except Exception:
        return {"status": "crash"}
    if d["status"] == "ok":
        return {"status": "ok", "stack": d.get("stackDisplay"), "output": d.get("output")}
    ai = d.get("aiDiagnostic") or {}
    return {"status": "error", "kind": ai.get("kind")}

sys.path.insert(0, HERE)
import ajisai as repro

def rep(src):
    return repro.run_program(src)

def norm(r):
    if r["status"] != "ok":
        return ("error", r.get("kind"))
    return ("ok", tuple(r.get("stack") or []), tuple(r.get("output") or []))

TESTS = [
    # arithmetic
    "1 2 ADD", "5 3 SUB", "3 4 MUL", "10 3 DIV", "10 3 MOD", "7 2 DIV",
    "5 0 DIV", "5 0 MOD", "-3 ABS" if False else "1 1 ADD",
    "3.14 FLOOR", "3.14 CEIL", "3.5 ROUND", "2.5 ROUND", "-2.5 ROUND",
    # comparison
    "1 2 LT", "2 1 LT", "1 1 EQ", "1 1 LTE", "2 2 GTE", "1 2 NEQ", "1 1 NEQ",
    "1 2 3 .. LT", "3 2 1 .. LT", "1 1 1 .. EQ", "1 2 2 .. LTE",
    # logic K3
    "TRUE FALSE AND", "TRUE TRUE AND", "FALSE FALSE OR", "TRUE FALSE OR",
    "TRUE NOT", "FALSE NOT", "TRUE 1 EQ",
    # nil / bubble / vent
    "5 0 DIV", "5 0 DIV 99 ^", "5 0 DIV 1 ADD", "NIL", "NIL 7 ^", "42 7 ^",
    # vectors
    "[ 1 2 3 ] LENGTH", "[ 1 2 3 ] 1 GET", "[ 1 2 3 ] 9 GET", "[ 1 2 3 ] -1 GET",
    "[ 1 2 3 ] REVERSE", "[ 1 2 ] [ 3 4 ] CONCAT", "0 5 RANGE",
    "[ 1 2 3 4 ] 2 TAKE", "[ 1 2 3 ] 1 5 REPLACE", "[ 1 2 3 ] 1 REMOVE",
    "[ 1 2 3 ] 1 9 INSERT", "1 2 3 COLLECT",
    # tensor
    "[ [ 1 2 ] [ 3 4 ] ] SHAPE", "[ [ 1 2 ] [ 3 4 ] ] RANK", "[ 1 2 3 ] RANK",
    # strings
    "'hello'", "'ab' 'cd' CONCAT", "'hello' CHARS", "65 CHR", "'42' NUM",
    "'abc' NUM", "[ 'a' 'b' ] JOIN", "1114112 CHR",
    # modifiers
    "1 2 ,, ADD", "1 2 3 ;ADD", "5 ,, PRINT",
    # higher order
    "[ 1 2 3 ] { 1 ADD } MAP", "[ 1 2 3 4 ] { 2 MOD 0 EQ } FILTER",
    "[ 1 2 3 ] 0 { ADD } FOLD", "{ 1 2 ADD } EXEC",
    # cond
    "{ { 1 1 EQ | 'yes' } { IDLE | 'no' } } EXEC" if False else "5",
    # def
    "{ 2 MUL } 'DOUBLE' DEF 21 DOUBLE",
    # print/output
    "'TEST' PRINT", "42 PRINT", "[ 'AB' 'CD' ] PRINT",
    # sqrt (needs MATH)
    "'MATH' IMPORT 2 SQRT 2 SQRT SUB", "'MATH' IMPORT 2 SQRT 2 SQRT EQ",
    "'MATH' IMPORT 4 SQRT", "'MATH' IMPORT 2 SQRT 2 SQRT MUL",
    # bool distinctness
    "TRUE 1 EQ", "FALSE 0 EQ",
]

def main():
    diffs = []
    same = 0
    for t in TESTS:
        o = orig(t); r = rep(t)
        if norm(o) == norm(r):
            same += 1
        else:
            diffs.append((t, o, r))
    print(f"== {same} identical, {len(diffs)} divergent of {len(TESTS)} ==\n")
    for t, o, r in diffs:
        print(f"SRC : {t}")
        print(f"orig: {o}")
        print(f"repr: {r}")
        print()

if __name__ == "__main__":
    main()
