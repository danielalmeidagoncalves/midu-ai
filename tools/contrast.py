#!/usr/bin/env python3
"""Check the OKLch token pairs Horizon actually prints. WCAG 2.x ratios."""

import math


def oklch_to_srgb(L, C, h):
    h = math.radians(h)
    a, b = C * math.cos(h), C * math.sin(h)
    l_ = L + 0.3963377774 * a + 0.2158037573 * b
    m_ = L - 0.1055613458 * a - 0.0638541728 * b
    s_ = L - 0.0894841775 * a - 1.2914855480 * b
    l_lin, m_lin, s_lin = l_**3, m_**3, s_**3
    r = 4.0767416621 * l_lin - 3.3077115913 * m_lin + 0.2309699292 * s_lin
    g = -1.2684380046 * l_lin + 2.6097574011 * m_lin - 0.3413193965 * s_lin
    bb = -0.0041960863 * l_lin - 0.7034186147 * m_lin + 1.7076147010 * s_lin
    def f(x):
        x = max(0.0, min(1.0, x))
        return 12.92 * x if x <= 0.0031308 else 1.055 * x ** (1 / 2.4) - 0.055
    return tuple(f(x) for x in (r, g, bb))

def luminance(rgb):
    def lin(c):
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    r, g, b = map(lin, rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b

def ratio(a, b):
    x, y = luminance(a), luminance(b)
    hi, lo = max(x, y), min(x, y)
    return (hi + 0.05) / (lo + 0.05)

T = {
    "bg": (0.98, 0.004, 95),
    "surface": (1.0, 0.002, 95),
    "fg": (0.20, 0.018, 70),
    "muted": (0.44, 0.012, 70),
    "border": (0.90, 0.006, 95),
    "accent": (0.48, 0.20, 28.5),
    "ink-2": (0.34, 0.016, 70),
    "ink-3": (0.46, 0.012, 70),
    "rule-soft": (0.94, 0.005, 95),
    "paper-dim": (0.965, 0.005, 95),
    "wash-1": (0.945, 0.006, 95),
    "wash-2": (0.915, 0.007, 95),
    "hover": (0.955, 0.007, 95),
    "accent-ink": (0.41, 0.19, 28.5),
    "accent-on-ink": (0.79, 0.09, 28.5),
    "tip-bg": (0.22, 0.018, 70),
    "tip-fg": (0.97, 0.004, 95),
    "land-hover": (0.28, 0.018, 70),
    "l2-hover": (0.89, 0.008, 95),
}
R = {k: oklch_to_srgb(*v) for k, v in T.items()}

PAIRS = [
    ("normal text", "fg", "bg"),
    ("normal text on surface", "fg", "surface"),
    ("normal text on paper-dim", "fg", "paper-dim"),
    ("normal text on wash-1", "fg", "wash-1"),
    ("normal text on wash-2", "fg", "wash-2"),
    ("normal text on hover", "fg", "hover"),
    ("normal text on l2 hover", "fg", "l2-hover"),
    ("muted text", "muted", "bg"),
    ("muted text on surface", "muted", "surface"),
    ("muted text on paper-dim", "muted", "paper-dim"),
    ("muted text on wash-2", "muted", "wash-2"),
    ("muted text on hover", "muted", "hover"),
    ("metadata", "ink-3", "bg"),
    ("metadata on surface", "ink-3", "surface"),
    ("metadata on paper-dim", "ink-3", "paper-dim"),
    ("metadata on hover", "ink-3", "hover"),
    ("accent-ink metadata", "accent-ink", "bg"),
    ("accent-ink on surface", "accent-ink", "surface"),
    ("accent-ink on paper-dim", "accent-ink", "paper-dim"),
    ("paper on ink", "bg", "fg"),
    ("paper on land hover", "bg", "land-hover"),
    ("accent-on-ink on ink", "accent-on-ink", "fg"),
    ("accent-on-ink on land hover", "accent-on-ink", "land-hover"),
    ("paper on encarnado stamp", "bg", "accent-ink"),
    ("encarnado on paper (frame/accent)", "accent", "bg"),
    ("menu trigger", "ink-2", "paper-dim"),
    ("menu entry", "fg", "surface"),
    ("menu entry, selected", "bg", "fg"),
    ("menu where", "muted", "surface"),
    ("menu hint", "ink-3", "bg"),
    ("tooltip text", "tip-fg", "tip-bg"),
    ("tooltip label", "bg", "tip-bg"),
    ("sync state", "fg", "bg"),
    ("sync meta", "muted", "bg"),
    ("sync history message", "fg", "bg"),
    ("sync history file", "muted", "bg"),
    ("sync file kind", "accent-ink", "bg"),
    ("sync error", "accent-ink", "bg"),
    ("sync version body", "ink-2", "surface"),
    ("sync input placeholder", "muted", "surface"),
]

failed = False
print(f"{'pair':42} {'ratio':>6}  {'need':>4}  result")
for label, fg, bg in PAIRS:
    r = ratio(R[fg], R[bg])
    # Small mono metadata is still normal text; require 4.5.
    need = 4.5
    ok = r >= need
    failed |= not ok
    print(f"{label:42} {r:6.2f}  {need:4.1f}  {'ok' if ok else 'FAIL'}")
print()
print("All normal-text pairs meet 4.5:1" if not failed else "CONTRAST FAILURES")
raise SystemExit(1 if failed else 0)
