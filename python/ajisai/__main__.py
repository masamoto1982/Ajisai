"""Command-line entry point: a small REPL / file runner.

Usage:
    python -m ajisai            # interactive REPL
    python -m ajisai file.aji   # run a source file
    echo '1 2 ADD' | python -m ajisai -
"""

import sys

from .errors import AjisaiError
from .interpreter import Interp
from .values import render


def _show(interp):
    stack = " ".join(render(v, "stack") for v in interp.stack)
    print(f"stack: [ {stack} ]")
    for line in interp.output:
        print(f"out: {line}")
    interp.output.clear()
    for w in interp.warnings:
        print(f"warn: {w}")
    interp.warnings.clear()


def main(argv):
    if len(argv) >= 2 and argv[1] != "-":
        src = open(argv[1], encoding="utf-8").read()
        interp = Interp()
        try:
            interp.run_source(src)
        except AjisaiError as e:
            print(f"error[{e.category}]: {e.message}")
            sys.exit(1)
        _show(interp)
        return
    if len(argv) >= 2 and argv[1] == "-":
        src = sys.stdin.read()
        interp = Interp()
        try:
            interp.run_source(src)
        except AjisaiError as e:
            print(f"error[{e.category}]: {e.message}")
            sys.exit(1)
        _show(interp)
        return
    interp = Interp()
    print("Ajisai (Python port). Ctrl-D to exit.")
    while True:
        try:
            line = input("aji> ")
        except EOFError:
            print()
            break
        try:
            interp.run_source(line)
        except AjisaiError as e:
            print(f"error[{e.category}]: {e.message}")
            continue
        _show(interp)


if __name__ == "__main__":
    main(sys.argv)
