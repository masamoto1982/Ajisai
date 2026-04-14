#!/usr/bin/env python3
"""Generate Ajisai QR code as text-friendly SVG (no binary asset required)."""

from pathlib import Path

URL = "https://masamoto1982.github.io/Ajisai/"
SVG_OUT = Path("public/images/ajisai-qr.svg")

# QR Version 3 (29x29), ECC L, Byte mode
VERSION = 3
SIZE = 21 + 4 * (VERSION - 1)
DATA_CODEWORDS = 55
ECC_CODEWORDS = 15


def gf_mul(x, y):
    z = 0
    for _ in range(8):
        if y & 1:
            z ^= x
        y >>= 1
        x <<= 1
        if x & 0x100:
            x ^= 0x11D
    return z


def rs_generator_poly(degree):
    poly = [1]
    for i in range(degree):
        poly = [gf_mul(c, 2**i) for c in poly] + [0]
        for j in range(len(poly) - 1):
            poly[j] ^= poly[j + 1]
    return poly[:-1]


def rs_compute(data, degree):
    gen = rs_generator_poly(degree)
    rem = [0] * degree
    for b in data:
        factor = b ^ rem[0]
        rem = rem[1:] + [0]
        for i in range(degree):
            rem[i] ^= gf_mul(gen[i], factor)
    return rem


def bits_from_bytes(bs):
    out = []
    for b in bs:
        for i in range(7, -1, -1):
            out.append((b >> i) & 1)
    return out


def make_bitstream(text):
    data = text.encode("utf-8")
    bits = [0, 1, 0, 0]  # byte mode
    n = len(data)
    bits += [(n >> i) & 1 for i in range(7, -1, -1)]
    for b in data:
        bits += [(b >> i) & 1 for i in range(7, -1, -1)]

    capacity = DATA_CODEWORDS * 8
    bits += [0] * min(4, capacity - len(bits))
    while len(bits) % 8 != 0:
        bits.append(0)

    pads = [0xEC, 0x11]
    i = 0
    while len(bits) < capacity:
        p = pads[i % 2]
        bits += [(p >> j) & 1 for j in range(7, -1, -1)]
        i += 1
    return bits


def empty_matrix(size):
    return [[0] * size for _ in range(size)], [[False] * size for _ in range(size)]


def set_module(m, f, x, y, val, func=True):
    if 0 <= x < SIZE and 0 <= y < SIZE:
        m[y][x] = 1 if val else 0
        if func:
            f[y][x] = True


def draw_finder(m, f, x, y):
    for dy in range(-1, 8):
        for dx in range(-1, 8):
            xx, yy = x + dx, y + dy
            if not (0 <= xx < SIZE and 0 <= yy < SIZE):
                continue
            if dx in (-1, 7) or dy in (-1, 7):
                set_module(m, f, xx, yy, 0)
            else:
                v = dx in (0, 6) or dy in (0, 6) or (2 <= dx <= 4 and 2 <= dy <= 4)
                set_module(m, f, xx, yy, v)


def draw_alignment(m, f, cx, cy):
    for dy in range(-2, 3):
        for dx in range(-2, 3):
            d = max(abs(dx), abs(dy))
            set_module(m, f, cx + dx, cy + dy, d != 1)


def draw_timing(m, f):
    for i in range(8, SIZE - 8):
        set_module(m, f, i, 6, i % 2 == 0)
        set_module(m, f, 6, i, i % 2 == 0)


def draw_function_patterns(m, f):
    draw_finder(m, f, 0, 0)
    draw_finder(m, f, SIZE - 7, 0)
    draw_finder(m, f, 0, SIZE - 7)
    draw_timing(m, f)
    for cy in [6, 22]:
        for cx in [6, 22]:
            if (cx, cy) not in [(6, 6), (6, SIZE - 7), (SIZE - 7, 6)]:
                draw_alignment(m, f, cx, cy)
    set_module(m, f, 8, SIZE - 8, 1)


def mask_bit(x, y):
    return (x + y) % 2 == 0


def draw_codewords(m, f, data_bits):
    i = 0
    x, y = SIZE - 1, SIZE - 1
    direction = -1
    while x > 0:
        if x == 6:
            x -= 1
        for _ in range(SIZE):
            for col in (x, x - 1):
                if not f[y][col] and i < len(data_bits):
                    bit = data_bits[i]
                    i += 1
                    if mask_bit(col, y):
                        bit ^= 1
                    m[y][col] = bit
            y += direction
            if y < 0 or y >= SIZE:
                y -= direction
                direction = -direction
                break
        x -= 2


def draw_format_bits(m, f):
    fmt = 0b01000  # ECC L + mask 0
    data = fmt << 10
    g = 0b10100110111
    for i in range(14, 9, -1):
        if (data >> i) & 1:
            data ^= g << (i - 10)
    bits = ((fmt << 10) | data) ^ 0b101010000010010

    for i in range(6):
        set_module(m, f, 8, i, (bits >> i) & 1)
    set_module(m, f, 8, 7, (bits >> 6) & 1)
    set_module(m, f, 8, 8, (bits >> 7) & 1)
    set_module(m, f, 7, 8, (bits >> 8) & 1)
    for i in range(9, 15):
        set_module(m, f, 14 - i, 8, (bits >> i) & 1)

    for i in range(8):
        set_module(m, f, SIZE - 1 - i, 8, (bits >> i) & 1)
    for i in range(8, 15):
        set_module(m, f, 8, SIZE - 15 + i, (bits >> i) & 1)


def make_qr_matrix(text):
    data_bits = make_bitstream(text)
    data_bytes = []
    for i in range(0, len(data_bits), 8):
        b = 0
        for bit in data_bits[i : i + 8]:
            b = (b << 1) | bit
        data_bytes.append(b)
    ecc = rs_compute(data_bytes, ECC_CODEWORDS)

    m, f = empty_matrix(SIZE)
    draw_function_patterns(m, f)
    draw_codewords(m, f, bits_from_bytes(data_bytes + ecc))
    draw_format_bits(m, f)
    return m


def module_color(x, y):
    palette = ("#9A82E6", "#8CC7F2", "#C6B6EE", "#AFA4F0")
    return palette[(x * 3 + y * 5) % len(palette)]


def write_svg(matrix, out_path, module=16, quiet=3):
    n = len(matrix)
    total = (n + quiet * 2) * module
    rects = []
    for y in range(n):
        for x in range(n):
            if matrix[y][x] == 1:
                xx = (x + quiet) * module
                yy = (y + quiet) * module
                rects.append(
                    f'<rect x="{xx}" y="{yy}" width="{module}" height="{module}" fill="{module_color(x,y)}"/>'
                )

    svg = "\n".join(
        [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{total}" height="{total}" viewBox="0 0 {total} {total}">',
            f'<rect width="{total}" height="{total}" fill="#FBFAFF"/>',
            *rects,
            "</svg>",
        ]
    )
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(svg, encoding="utf-8")


def terminal_preview(matrix):
    lines = []
    quiet = 2
    w = len(matrix)
    for _ in range(quiet):
        lines.append("  " * (w + quiet * 2))
    for row in matrix:
        line = ["  "] * quiet
        line += ["██" if v else "  " for v in row]
        line += ["  "] * quiet
        lines.append("".join(line))
    for _ in range(quiet):
        lines.append("  " * (w + quiet * 2))
    return "\n".join(lines)


if __name__ == "__main__":
    matrix = make_qr_matrix(URL)
    write_svg(matrix, SVG_OUT)
    print(f"generated: {SVG_OUT}")
    print("--- terminal preview ---")
    print(terminal_preview(matrix))
