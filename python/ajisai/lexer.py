"""Tokenizer and parser (Section 3).

Surface forms are mapped to canonical concepts here (Section 3.9, 7.0). The
parser produces a nested ``Block`` structure: a block is a list of *lines* and
each line is a list of nodes, because a multi-line block executes one source
line at a time (Section 3.4) and each ``|`` COND clause occupies one line
(Section 3.6).
"""

from __future__ import annotations

import re
from fractions import Fraction
from typing import List, Tuple

from .errors import TokenizerError
from .numbers import AlgebraicReal
from .values import CodeBlock, Scalar, make_text

STRUCTURAL = set("[]{}|")
# Word-alias sugar -> canonical name (Section 3.9, 7.0)
ALIASES = {
    "+": "ADD", "-": "SUB", "*": "MUL", "/": "DIV", "%": "MOD",
    "=": "EQ", "<>": "NEQ", "<": "LT", "<=": "LTE", ">": "GT", ">=": "GTE",
    "&": "AND", "!": "FORC", "?": "LOOKUP",
    "~": "FLOW", "^": "VENT",
}

_NUM_RE = re.compile(
    r"""^[+-]?(
        \d+/\d+            # fraction
      | (\d+\.\d*|\.\d+|\d+)([eE][+-]?\d+)?   # int / decimal / scientific
    )$""",
    re.VERBOSE,
)
_MOD_RE = re.compile(r"^[.,;]+$")


class Tok:
    __slots__ = ("kind", "value", "line")

    def __init__(self, kind, value, line):
        self.kind = kind
        self.value = value
        self.line = line

    def __repr__(self):
        return f"Tok({self.kind},{self.value!r},L{self.line})"


def tokenize(src: str) -> List[Tok]:
    toks: List[Tok] = []
    i = 0
    n = len(src)
    line = 1
    while i < n:
        ch = src[i]
        if ch == "\n":
            line += 1
            i += 1
            continue
        if ch in " \t\r":
            i += 1
            continue
        if ch == "#":  # line comment (Section 3.1)
            while i < n and src[i] != "\n":
                i += 1
            continue
        if ch in "()":  # reserved markers (Section 3.4)
            raise TokenizerError(f"reserved character {ch!r} is not valid in source")
        if ch in STRUCTURAL:
            toks.append(Tok("struct", ch, line))
            i += 1
            continue
        if ch == "'":  # string literal (Section 3.3)
            j = i + 1
            last_quote = -1
            while j < n:
                c = src[j]
                if c == "'":
                    # boundary follows? whitespace, EOF, or special char != '
                    nxt = src[j + 1] if j + 1 < n else ""
                    if nxt == "" or nxt in " \t\r\n" or nxt in STRUCTURAL or nxt == "#":
                        last_quote = j
                        break
                    # else: literal quote inside content; keep scanning
                if c == "\n":
                    line += 1
                j += 1
            if last_quote < 0:
                raise TokenizerError("unterminated string literal")
            content = src[i + 1:last_quote]
            toks.append(Tok("str", content, line))
            i = last_quote + 1
            continue
        # symbol / number / modifier run: up to whitespace or structural char or '
        j = i
        while j < n and src[j] not in " \t\r\n" and src[j] not in STRUCTURAL \
                and src[j] not in "'#()":
            j += 1
        word = src[i:j]
        toks.append(_classify(word, line))
        i = j
    return toks


def _classify(word: str, line: int) -> Tok:
    if _NUM_RE.match(word):
        return Tok("num", _parse_number(word), line)
    if _MOD_RE.match(word) and word not in (".", ".."):
        # pure modifier sugar (excluding bare . / .. which are TOP/STAK words too,
        # but we treat them as modifiers uniformly)
        return Tok("mod", _decode_mod(word), line)
    if word in (".", ".."):
        return Tok("mod", _decode_mod(word), line)
    # conversion word >CF etc: '>' + letters
    if len(word) >= 2 and word[0] == ">" and word[1].isalpha():
        return Tok("word", word.upper(), line)
    if word in ALIASES:
        return Tok("word", ALIASES[word], line)
    return Tok("word", word.upper(), line)


def _decode_mod(word: str) -> Tuple:
    # expand ; -> '.,'  ;; -> '..,,' (Section 3.9)
    expanded = word.replace(";;", "..,,").replace(";", ".,")
    dots = expanded.count(".")
    commas = expanded.count(",")
    target = "STAK" if dots >= 2 else ("TOP" if dots == 1 else None)
    consume = "KEEP" if commas >= 2 else ("EAT" if commas == 1 else None)
    return (target, consume)


def _parse_number(tok: str) -> Scalar:
    # Fraction() does not parse scientific or "5." forms directly; normalize.
    sign = Fraction(1)
    s = tok
    if s and s[0] in "+-":
        if s[0] == "-":
            sign = Fraction(-1)
        s = s[1:]
    if "/" in s:
        num, den = s.split("/")
        f = Fraction(int(num), int(den))
    else:
        mant = s
        exp = 0
        if "e" in s or "E" in s:
            s2 = s.replace("E", "e")
            mant, e = s2.split("e")
            exp = int(e)
        if mant == "" or mant == ".":
            raise TokenizerError(f"malformed numeric literal {tok!r}")
        if "." in mant:
            ip, fp = mant.split(".")
            ip = ip or "0"
            fp = fp or ""
            f = Fraction(int(ip + fp) if (ip + fp) else 0, 10 ** len(fp))
        else:
            f = Fraction(int(mant))
        if exp >= 0:
            f *= 10 ** exp
        else:
            f /= 10 ** (-exp)
    return Scalar.of(AlgebraicReal.from_rational(f * sign))


# ---------------------------------------------------------------------------
# Parser: tokens -> nested Block structure
# ---------------------------------------------------------------------------

class Block:
    """A parsed code body: list of lines, each line a list of nodes."""

    __slots__ = ("lines", "source")

    def __init__(self, lines, source=""):
        self.lines = lines
        self.source = source


def parse(src: str) -> Block:
    toks = tokenize(src)
    pos = [0]
    block = _parse_block_body(toks, pos, top=True)
    block.source = src
    return block


def _parse_block_body(toks, pos, top=False) -> Block:
    lines: List[list] = []
    current: list = []
    current_line_no = None
    while pos[0] < len(toks):
        t = toks[pos[0]]
        if t.kind == "struct" and t.value in "}]":
            break
        if current_line_no is not None and t.line != current_line_no and current:
            lines.append(current)
            current = []
        current_line_no = t.line
        node = _parse_node(toks, pos)
        current.append(node)
    if current:
        lines.append(current)
    return Block(lines)


def _parse_node(toks, pos):
    t = toks[pos[0]]
    if t.kind == "num":
        pos[0] += 1
        return ("num", t.value)
    if t.kind == "str":
        pos[0] += 1
        return ("str", t.value)
    if t.kind == "mod":
        pos[0] += 1
        return ("mod", t.value)
    if t.kind == "word":
        pos[0] += 1
        return ("word", t.value)
    if t.kind == "struct":
        if t.value == "|":
            pos[0] += 1
            return ("pipe",)
        if t.value == "[":
            pos[0] += 1
            items = _parse_seq(toks, pos, "]")
            return ("vec", items)
        if t.value == "{":
            pos[0] += 1
            body = _parse_block_body(toks, pos)
            _expect_close(toks, pos, "}")
            return ("block", body)
    raise TokenizerError(f"unexpected token {t!r}")


def _parse_seq(toks, pos, close):
    """Parse a flat node sequence (for vectors) until close delimiter."""
    nodes = []
    while pos[0] < len(toks):
        t = toks[pos[0]]
        if t.kind == "struct" and t.value == close:
            pos[0] += 1
            return nodes
        nodes.append(_parse_node(toks, pos))
    raise TokenizerError(f"unbalanced, expected {close!r}")


def _expect_close(toks, pos, close):
    if pos[0] >= len(toks) or not (toks[pos[0]].kind == "struct" and toks[pos[0]].value == close):
        raise TokenizerError(f"unbalanced, expected {close!r}")
    pos[0] += 1
