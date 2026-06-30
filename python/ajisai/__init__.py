"""A Python port of Ajisai built solely from SPECIFICATION.html.

See SPEC_GAPS.md for ambiguities discovered while implementing from the spec.
"""

from .interpreter import Interp
from .values import render

__all__ = ["Interp", "render", "run"]


def run(src: str):
    """Evaluate Ajisai source and return the interpreter (with stack/output)."""
    interp = Interp()
    interp.run_source(src)
    return interp
