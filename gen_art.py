#!/usr/bin/env python3
"""Generate the imp Unicode art HTML with guaranteed alignment.

Each line is built from (text, css_class) segments that must sum
to exactly W characters. The script validates before output.

Color classes map 1:1 to ratatui Color::Rgb — see CSS comments.
"""

W = 85  # inner content width (between ▓░▒░ ... ░▒░▓ frame)

# ── helpers ──────────────────────────────────────────────────

def S(text, cls=''):
    """A colored segment."""
    return (text, cls)

def line(*segs):
    """Build a content line from segments. Pads to W with spaces."""
    total = sum(len(t) for t, _ in segs)
    if total > W:
        raise ValueError(f"Line too wide ({total} > {W}): {''.join(t for t,_ in segs)}")
    if total < W:
        segs = list(segs) + [S(' ' * (W - total))]
    return list(segs)

def ctr(inner_segs):
    """Center inner segments within W, padding with spaces."""
    inner_len = sum(len(t) for t, _ in inner_segs)
    left = (W - inner_len) // 2
    right = W - inner_len - left
    return [S(' ' * left)] + list(inner_segs) + [S(' ' * right)]

def sym(left_segs, center_segs, gap=0):
    """Symmetric line: left + gap + center + gap + mirror(left)."""
    left_len = sum(len(t) for t, _ in left_segs)
    center_len = sum(len(t) for t, _ in center_segs)
    total_inner = left_len * 2 + gap * 2 + center_len
    margin = (W - total_inner) // 2
    right_margin = W - total_inner - margin
    # mirror left segments (reverse order)
    right_segs = list(reversed(left_segs))
    return ([S(' ' * margin)] + list(left_segs) + [S(' ' * gap)] +
            list(center_segs) + [S(' ' * gap)] +
            right_segs + [S(' ' * right_margin)])

# ── the imp ──────────────────────────────────────────────────

# Shorthand for common patterns
def blank():
    return line()

art = [
    # ── empty top padding ──
    blank(),
    blank(),

    # ── crown of the head ──
    line(S(' '*32), S(',,,,,,,,,,,,,,,,,,,', 'sh')),
    line(S(' '*26), S(',,,', 'sh'), S('▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒', 'sm'), S(',,,', 'sh')),
    line(S(' '*22), S(',,,', 'sh'), S('▒▒▒', 'sm'), S('▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓', 'd'), S('▒▒▒', 'sm'), S(',,', 'sh')),

    # ── head forming, horns appear ──
    line(S(' '*9),
         S(',', 'sh'), S('▒', 'sm'), S('▓', 'd'), S('▓', 'g'), S(',', 'sh'),
         S(' '*8),
         S('▒▒', 'sm'), S('▓▓', 'd'), S('░', 'g'),
         S('░░░░░░░░░░░░░░░░░░░░░░░', 'a'),
         S('░', 'g'), S('▓▓', 'd'), S('▒▒', 'sm')),

    line(S(' '*7),
         S('▒', 'sm'), S('▓▓', 'd'), S('▓▓▓', 'g'), S(',', 'sh'),
         S(' '*7),
         S('▓▓', 'd'), S('░', 'g'), S('░', 'a'),
         S('▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓', 'gb'),
         S('░', 'a'), S('░', 'g'), S('▓▓', 'd'),
         S(' '*8),
         S(',', 'sh'), S('▒', 'sm'), S('▓', 'd'), S('▓', 'g'), S('▓', 'a'), S(',', 'sh')),

    line(S(' '*6),
         S('▓▓', 'd'), S('▓▓▓▓▓▓', 'g'), S('▒', 'sm'),
         S(' '*5),
         S('▓▓', 'd'), S('░', 'a'),
         S('▓▓▓', 'gb'), S('░░░░░░░░░░░░░░░░░', 'gl'), S('▓▓▓', 'gb'),
         S('░', 'a'), S('▓▓', 'd'),
         S(' '*6),
         S('▒', 'sm'), S('▓▓', 'd'), S('▓▓▓▓▓▓', 'g'), S('▒', 'sm')),

    line(S(' '*5),
         S('▓▓▓', 'g'), S('░░░', 'a'), S('▓▓▓', 'gb'), S('▒', 'sm'),
         S(' '*4),
         S('▓', 'g'), S('░', 'a'),
         S('▓▓', 'gb'), S('░░░', 'gl'),
         S('░░░░░░░░░░░░░', 'br'),
         S('░░░', 'gl'), S('▓▓', 'gb'),
         S('░', 'a'), S('▓', 'g'),
         S(' '*5),
         S('▒', 'sm'), S('▓▓▓', 'd'), S('░░░', 'g'), S('▓▓▓', 'a'), S('▒', 'sm')),

    line(S(' '*4),
         S('▓▓', 'g'), S('░░░░░░', 'a'), S('▓▓', 'gb'), S('▒', 'sm'),
         S(' '*3),
         S('░', 'a'),
         S('▓▓', 'gb'), S('░░░', 'gl'),
         S('░░░░░░░░░░░░░░░', 'br'),
         S('░░░', 'gl'), S('▓▓', 'gb'),
         S('░', 'a'),
         S(' '*4),
         S('▒', 'sm'), S('▓▓', 'd'), S('░░░░░░', 'g'), S('▓▓', 'a'), S('▒', 'sm')),

    # ── face — upper cheeks ──
    line(S(' '*3),
         S('░░░░░░░░░░░', 'a'), S('░', 'gb'), S('▒', 'sm'),
         S(' '*2),
         S('▓▓', 'gb'),
         S('░░░', 'gl'), S('░░░░░', 'br'),
         S('░░░░░░░', 'wh'),
         S('░░░░░', 'br'), S('░░░', 'gl'),
         S('▓▓', 'gb'),
         S(' '*3),
         S('▒', 'sm'), S('░', 'g'), S('░░░░░░░░░░░', 'a')),

    line(S(' '*3),
         S('░░░', 'a'), S('░░░░░░░', 'gb'), S('░', 'gl'), S('▒', 'sm'),
         S(' '),
         S('▓▓', 'gb'),
         S('░░░', 'gl'), S('░░░░░', 'br'),
         S('░░', 'wh'), S(' '), S('░░░', 'hw'), S(' '), S('░░', 'wh'),
         S('░░░░░', 'br'), S('░░░', 'gl'),
         S('▓▓', 'gb'),
         S(' '*2),
         S('▒', 'sm'), S('░', 'gl'), S('░░░░░░░', 'gb'), S('░░░', 'a')),

    # ── forehead → eyes ──
    line(S(' '*2),
         S('░░░', 'gb'), S('░░░░░░░░', 'gl'), S('░', 'br'), S('▒', 'sm'),
         S('▓▓', 'gl'),
         S('░░░', 'br'), S('░░░░', 'wh'),
         S(' '), S('░', 'hw'), S('░░', 'gl'), S(' '),
         S('░░░', 'hw'),
         S(' '), S('░░', 'gl'), S('░', 'hw'), S(' '),
         S('░░░░', 'wh'), S('░░░', 'br'),
         S('▓▓', 'gl'),
         S('▒', 'sm'), S('░', 'br'), S('░░░░░░░░', 'gl'), S('░░░', 'gb')),

    # ── eyes ◈ ──
    line(S(' '*2),
         S('░░░░░░░░░░░░░', 'gl'), S('▒', 'sm'),
         S('▓▓', 'br'),
         S('░░░', 'wh'), S('░░░░', 'hw'),
         S('  '), S('░', 'gl'),
         S('◈', 'eg'),  # LEFT EYE
         S('░', 'gl'),
         S('  '), S('░░░', 'hw'), S('  '),
         S('░', 'gl'),
         S('◈', 'eg'),  # RIGHT EYE
         S('░', 'gl'),
         S('  '), S('░░░░', 'hw'), S('░░░', 'wh'),
         S('▓▓', 'br'),
         S('▒', 'sm'), S('░░░░░░░░░░░░░', 'gl')),

    # ── eyes ◉ ──
    line(S(' '*2),
         S('░░░░░░░░░░░░░', 'gl'), S('▒', 'sm'),
         S('▓▓', 'br'),
         S('░░░', 'wh'), S('░░░', 'hw'),
         S('  '),
         S('░░', 'gl'), S('◉', 'ew'), S('░░', 'gl'),
         S('  '), S('░░', 'hw'), S('  '),
         S('░░', 'gl'), S('◉', 'ew'), S('░░', 'gl'),
         S('  '),
         S('░░░', 'hw'), S('░░░', 'wh'),
         S('▓▓', 'br'),
         S('▒', 'sm'), S('░░░░░░░░░░░░░', 'gl')),

    # ── below eyes ──
    line(S(' '*2),
         S('░░░░░░░░░░░░', 'gl'), S('░', 'wh'), S('▒', 'sm'),
         S('▓▓', 'wh'),
         S('░░░', 'hw'), S('░░', 'wh'),
         S('   '), S('░░░░', 'br'), S('░░', 'wh'),
         S('   '), S('░░', 'br'),
         S('   '), S('░░░░', 'br'), S('░░', 'wh'),
         S('   '),
         S('░░', 'wh'), S('░░░', 'hw'),
         S('▓▓', 'wh'),
         S('▒', 'sm'), S('░', 'wh'), S('░░░░░░░░░░░░', 'gl')),

    line(S(' '*2),
         S('░░░░░░░░░░░░░', 'br'), S('▒', 'sm'),
         S('▓▓', 'hw'),
         S('░░░', 'wh'), S('░', 'hw'),
         S('   '), S('░', 'gl'), S('░░░░░', 'br'), S('░', 'gl'),
         S('  '), S('░░', 'gl'),
         S('  '), S('░', 'gl'), S('░░░░░', 'br'), S('░', 'gl'),
         S('   '),
         S('░', 'hw'), S('░░░', 'wh'),
         S('▓▓', 'hw'),
         S('▒', 'sm'), S('░░░░░░░░░░░░░', 'br')),

    # ── nose ──
    line(S(' '*3),
         S('░░░░░░░░░░░░░', 'br'), S('▒', 'sm'),
         S('▓', 'hw'), S('░░░', 'wh'),
         S('     '),
         S('░', 'wh'), S('░░░░░░', 'hw'), S('░', 'wh'),
         S('░░░░░', 'hw'),
         S('░', 'wh'), S('░░░░░░', 'hw'), S('░', 'wh'),
         S('     '),
         S('░░░', 'wh'), S('▓', 'hw'),
         S('▒', 'sm'), S('░░░░░░░░░░░░░', 'br')),

    line(S(' '*3),
         S('░░░░░░░░░░░░░', 'wh'), S('▒', 'sm'),
         S('▓', 'hw'), S('░░', 'wh'),
         S('       '),
         S('░░', 'hw'), S('░░░░░░░░░░░░░░░', 'wh'), S('░░', 'hw'),
         S('       '),
         S('░░', 'wh'), S('▓', 'hw'),
         S('▒', 'sm'), S('░░░░░░░░░░░░░', 'wh')),

    # ── nose detail ──
    line(S(' '*4),
         S('░░░░░░░░░░░░', 'hw'), S('▒', 'sm'),
         S('▓', 'hw'), S('░', 'wh'),
         S('        '),
         S('▓', 'gl'), S('░░░░░░', 'br'),
         S('░', 'hw'), S('/\\', 'wh'), S('░', 'hw'),
         S('░░░░░░', 'br'), S('▓', 'gl'),
         S('        '),
         S('░', 'wh'), S('▓', 'hw'),
         S('▒', 'sm'), S('░░░░░░░░░░░░', 'hw')),

    line(S(' '*5),
         S('░░░░░░░░░░░', 'hw'), S('▒▒', 'sm'),
         S('▓', 'hw'), S('░', 'wh'),
         S('        '),
         S('░░░░░', 'br'), S('░', 'hw'), S('╱__╲', 'wh'), S('░', 'hw'), S('░░░░░', 'br'),
         S('        '),
         S('░', 'wh'), S('▓', 'hw'),
         S('▒▒', 'sm'), S('░░░░░░░░░░░', 'hw')),

    # ── mouth / chin ──
    line(S(' '*6),
         S('░░░░░░░░░', 'wh'), S('▒▒', 'sm'),
         S('▓▓', 'gl'), S('░', 'hw'),
         S('          '),
         S('░░░░░░░░░░░░░', 'br'),
         S('          '),
         S('░', 'hw'), S('▓▓', 'gl'),
         S('▒▒', 'sm'), S('░░░░░░░░░', 'wh')),

    line(S(' '*7),
         S('░░░░░░░', 'gl'), S('▒▒', 'sm'),
         S('▓▓', 'gb'), S('░', 'gl'),
         S('          '),
         S('░', 'wh'), S('░', 'hw'), S('░░░░░░░░░░░', 'wh'), S('░', 'hw'), S('░', 'wh'),
         S('          '),
         S('░', 'gl'), S('▓▓', 'gb'),
         S('▒▒', 'sm'), S('░░░░░░░', 'gl')),

    # ── wings begin ──
    line(S(' '*3),
         S('▓', 'r'), S('▓', 'rd'), S('▓', 'or'), S('▒', 'sm'),
         S('   '),
         S('░░░', 'a'), S('▒▒', 'sm'), S('▓▓', 'a'), S('░', 'gb'),
         S('       '),
         S('░░░░░░░░░░░░░░░░░░░', 'wh'),
         S('       '),
         S('░', 'gb'), S('▓▓', 'a'), S('▒▒', 'sm'), S('░░░', 'a'),
         S('   '),
         S('▒', 'sm'), S('▓', 'or'), S('▓', 'rd'), S('▓', 'r')),

    line(S(' '*2),
         S('▓▓▓', 'r'), S('▓▓▓', 'rd'), S('▓', 'or'), S('▒', 'sm'),
         S('  '),
         S('░░', 'gb'), S('▒▒', 'sm'), S('▓▓', 'g'), S('░', 'a'),
         S('       '),
         S('░░░░░░░░░░░░░░░░░░░', 'br'),
         S('       '),
         S('░', 'a'), S('▓▓', 'g'), S('▒▒', 'sm'), S('░░', 'gb'),
         S('  '),
         S('▒', 'sm'), S('▓', 'or'), S('▓▓▓', 'rd'), S('▓▓▓', 'r')),

    line(S(' '),
         S('▓▓▓▓▓▓▓▓', 'rd'), S('▓', 'or'), S('▒', 'sm'),
         S('  '),
         S('░', 'g'), S('▒▒', 'sm'), S('▓▓', 'd'), S('░', 'g'),
         S('       '),
         S('░░░░░░░░░░░░░░░░░░░', 'gl'),
         S('       '),
         S('░', 'g'), S('▓▓', 'd'), S('▒▒', 'sm'), S('░', 'g'),
         S('  '),
         S('▒', 'sm'), S('▓', 'or'), S('▓▓▓▓▓▓▓▓', 'rd')),

    line(S(' '),
         S('▓▓▓▓▓', 'or'), S('▓▓▓▓', 'f1'), S('▒', 'sm'),
         S('  '),
         S('▓▓', 'd'), S('▒▒', 'sm'), S('▓', 'sh'), S('░', 'a'),
         S('      '),
         S('░░░░░░░░░░░░░░░░░░░░░', 'a'),
         S('      '),
         S('░', 'a'), S('▓', 'sh'), S('▒▒', 'sm'), S('▓▓', 'd'),
         S('  '),
         S('▒', 'sm'), S('▓▓▓▓', 'f1'), S('▓▓▓▓▓', 'or')),

    line(S(' '),
         S('▓▓▓▓▓▓▓▓▓▓', 'f1'), S('▒', 'sm'),
         S('  '),
         S('▓▓▓', 'sm'), S('▒', 'sm'), S('▓', 'd'), S('░', 'g'),
         S('     '),
         S('░░░░░░░░░░░░░░░░░░░░░░░', 'gb'),
         S('     '),
         S('░', 'g'), S('▓', 'd'), S('▒', 'sm'), S('▓▓▓', 'sm'),
         S('  '),
         S('▒', 'sm'), S('▓▓▓▓▓▓▓▓▓▓', 'f1')),

    line(S(' '*2),
         S('▓▓▓', 'f1'), S('▓▓▓▓▓', 'f2'), S('▓', 'or'), S('▒', 'sm'),
         S('   '),
         S('▓▓▓', 'sm'), S('▒▒', 'sm'), S('▓', 'sh'),
         S('    '),
         S('░░░░░░░░░░░░░░░░░░░░░░░', 'gl'),
         S('    '),
         S('▓', 'sh'), S('▒▒', 'sm'), S('▓▓▓', 'sm'),
         S('   '),
         S('▒', 'sm'), S('▓', 'or'), S('▓▓▓▓▓', 'f2'), S('▓▓▓', 'f1')),

    line(S(' '*3),
         S('▓▓▓▓▓▓▓', 'f2'), S('▓', 'f3'), S('▒', 'sm'),
         S('     '),
         S('▒▒', 'sm'), S('▓▓', 'sm'), S('▒▒', 'sm'), S('▓', 'sh'),
         S('   '),
         S('░░░░░░░░░░░░░░░░░░░░░', 'br'),
         S('   '),
         S('▓', 'sh'), S('▒▒', 'sm'), S('▓▓', 'sm'), S('▒▒', 'sm'),
         S('     '),
         S('▒', 'sm'), S('▓', 'f3'), S('▓▓▓▓▓▓▓', 'f2')),

    line(S(' '*4),
         S('▓▓▓▓▓', 'f2'), S('▓▓▓', 'f3'), S('▒', 'sm'),
         S('      '),
         S('▒▒', 'sm'), S('▓▓', 'sm'), S('▒▒▒▒', 'sm'),
         S('   '),
         S('░░░░░░░░░░░░░░░░░', 'wh'),
         S('   '),
         S('▒▒▒▒', 'sm'), S('▓▓', 'sm'), S('▒▒', 'sm'),
         S('      '),
         S('▒', 'sm'), S('▓▓▓', 'f3'), S('▓▓▓▓▓', 'f2')),

    line(S(' '*6),
         S('▓▓▓▓▓▓', 'f3'), S('▒', 'sm'),
         S('         '),
         S('▒▒▒▒', 'sm'), S('▓▓▓', 'sm'), S('▒▒', 'sm'),
         S('  '),
         S('░░░░░░░░░░░░░', 'gl'),
         S('  '),
         S('▒▒', 'sm'), S('▓▓▓', 'sm'), S('▒▒▒▒', 'sm'),
         S('         '),
         S('▒', 'sm'), S('▓▓▓▓▓▓', 'f3')),

    line(S(' '*8),
         S('▓▓▓▓', 'f3'), S('▓', 'f4'), S('▒', 'sm'),
         S('            '),
         S('▒▒▒▒', 'sm'), S('▓▓▓▓', 'sm'), S('▒▒▒▒', 'sm'),
         S('░░░░░░░░░', 'a'),
         S('▒▒▒▒', 'sm'), S('▓▓▓▓', 'sm'), S('▒▒▒▒', 'sm'),
         S('            '),
         S('▒', 'sm'), S('▓', 'f4'), S('▓▓▓▓', 'f3')),

    line(S(' '*10),
         S('▓▓▓', 'f4'), S('▒', 'sm'),
         S('                 '),
         S('▒▒▒▒▒', 'sm'), S('▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓', 'sm'), S('▒▒▒▒▒', 'sm'),
         S('                 '),
         S('▒', 'sm'), S('▓▓▓', 'f4')),

    line(S(' '*40),
         S('▒▒▒▒▒▒▒▒▒▒▒', 'sm')),

    # ── empty bottom padding ──
    blank(),
    blank(),
]


# ── validate ─────────────────────────────────────────────────

for i, row in enumerate(art):
    w = sum(len(t) for t, _ in row)
    if w != W:
        text = ''.join(t for t, _ in row)
        raise ValueError(f"Art line {i} is {w} chars (need {W}):\n  |{text}|")

print(f"✓ All {len(art)} art lines are exactly {W} chars")


# ── frame helpers ────────────────────────────────────────────

FRAME_L = [('▓', 'st'), ('░', 'sh'), ('▒', 'sm'), ('░', 'd')]
FRAME_R = [('░', 'd'), ('▒', 'sm'), ('░', 'sh'), ('▓', 'st')]
TOTAL_W = W + 8  # 4 frame chars each side

def frame_border(char, cls):
    return [(char * TOTAL_W, cls)]

def frame_fill(char, cls, outer_cls):
    return [(outer_cls[0], outer_cls[1]), (char * (TOTAL_W - 2), cls), (outer_cls[0], outer_cls[1])]


# ── HTML generation ──────────────────────────────────────────

def span(text, cls):
    if not cls:
        return text
    return f'<span class="{cls}">{text}</span>'

def render_line(segs):
    return ''.join(span(t, c) for t, c in segs)

def render_framed_line(inner_segs):
    parts = FRAME_L + inner_segs + FRAME_R
    return render_line(parts)


# ── build the complete pre block ─────────────────────────────

lines = []
# top border
lines.append(render_line([('▓' * TOTAL_W, 'br2')]))
# fill rows
lines.append(render_line([('▓', 'br2'), ('░' * (TOTAL_W - 2), 'sh'), ('▓', 'br2')]))
lines.append(render_line([('▓', 'br2'), ('░', 'sh'), ('▒' * (TOTAL_W - 4), 'sm'), ('░', 'sh'), ('▓', 'br2')]))
lines.append(render_line([('▓', 'br2'), ('░', 'sh'), ('▒', 'sm'), ('░' * (TOTAL_W - 6), 'd'), ('▒', 'sm'), ('░', 'sh'), ('▓', 'br2')]))

# content lines
for row in art:
    lines.append(render_framed_line(row))

# bottom fill rows
lines.append(render_line([('▓', 'br2'), ('░', 'sh'), ('▒', 'sm'), ('░' * (TOTAL_W - 6), 'd'), ('▒', 'sm'), ('░', 'sh'), ('▓', 'br2')]))
lines.append(render_line([('▓', 'br2'), ('░', 'sh'), ('▒' * (TOTAL_W - 4), 'sm'), ('░', 'sh'), ('▓', 'br2')]))
lines.append(render_line([('▓', 'br2'), ('░' * (TOTAL_W - 2), 'sh'), ('▓', 'br2')]))
# bottom border
lines.append(render_line([('▓' * TOTAL_W, 'br2')]))


# ── now inject animation classes into the rendered HTML ──────

import re

# Replace static eye classes with animated ones
# ◈ eyes: eg → ep (eye-pulse animation)
# ◉ eyes: ew → eg2 (eye-glow animation), wrapped in blink
animated_lines = []
for ln in lines:
    # Wrap ◈ in blink + pulse
    ln = ln.replace(
        '<span class="eg">◈</span>',
        '<span class="bl2"><span class="ep">◈</span></span>'
    )
    # Wrap ◉ in blink + glow
    ln = ln.replace(
        '<span class="ew">◉</span>',
        '<span class="bl2"><span class="eg2">◉</span></span>'
    )
    # Fire animations: f1→a1, f2→a2, f3→a3, f4→a4
    ln = ln.replace('class="f1"', 'class="a1"')
    ln = ln.replace('class="f2"', 'class="a2"')
    ln = ln.replace('class="f3"', 'class="a3"')
    ln = ln.replace('class="f4"', 'class="a4"')
    # Shimmer on hw (hot white) - only some occurrences (face highlights)
    # We'll add shimmer to a few strategic hw spans
    animated_lines.append(ln)

pre_content = '\n'.join(animated_lines)


# ── logotype block ───────────────────────────────────────────

logo_art = [
    blank(),
    blank(),
    line(S('   '), S('▄', 'sh'), S('▄', 'd'), S('▄', 'sm'), S('  '),
         S('▄', 'd'), S('▄', 'sm'), S('▄▄', 'd'), S('▄', 'sm'), S('  '),
         S('▄▄▄', 'sm')),
    line(S('  '), S('▓▓▓', 'd'), S(' '), S('▓▓▓▓▓▓▓', 'sm'), S(' '), S('▓▓▓', 'd')),
    line(S('  '), S('███', 'a'), S(' '), S('█████████', 'gb'), S(' '), S('███', 'a')),
    line(S('  '), S('░░░', 'gl'), S(' '), S('░░░░░░░░░', 'br'), S(' '), S('░░░', 'gl'),
         S('      '), S('·', 'd'), S(' '), S('the worker engine', 'sm'), S(' '), S('·', 'd')),
    line(S('  '), S('▓▓▓', 'gl'), S(' '), S('▓▓▓▓▓▓▓▓▓', 'br'), S(' '), S('▓▓▓', 'gl')),
    line(S('  '), S('▓▓▓', 'gb'), S(' '), S('▓▓▓▓▓▓▓▓▓', 'gl'), S(' '), S('▓▓▓', 'gb')),
    line(S('  '), S('▓▓▓', 'a'), S(' '), S('▓▓▓▓▓▓▓▓▓', 'gb'), S(' '), S('▓▓▓', 'a')),
    line(S('  '), S('▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓', 'g')),
    line(S('   '), S('▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀', 'd')),
    blank(),
    blank(),
]

for i, row in enumerate(logo_art):
    w = sum(len(t) for t, _ in row)
    if w != W:
        raise ValueError(f"Logo line {i} is {w} chars (need {W})")

logo_lines = []
logo_lines.append(render_line([('╔' + '═' * (TOTAL_W - 2) + '╗', 'br2')]))
logo_lines.append(render_line([('║', 'br2'), ('░' * (TOTAL_W - 2), 'sh'), ('║', 'br2')]))
logo_lines.append(render_line([('║', 'br2'), ('░', 'sh'), ('▒' * (TOTAL_W - 4), 'sm'), ('░', 'sh'), ('║', 'br2')]))
for row in logo_art:
    logo_lines.append(render_line([('║', 'br2'), ('░', 'sh'), ('▒', 'sm')] + row + [('▒', 'sm'), ('░', 'sh'), ('║', 'br2')]))
logo_lines.append(render_line([('║', 'br2'), ('░', 'sh'), ('▒' * (TOTAL_W - 4), 'sm'), ('░', 'sh'), ('║', 'br2')]))
logo_lines.append(render_line([('║', 'br2'), ('░' * (TOTAL_W - 2), 'sh'), ('║', 'br2')]))
logo_lines.append(render_line([('╚' + '═' * (TOTAL_W - 2) + '╝', 'br2')]))

logo_pre = '\n'.join(logo_lines)


# ── assemble HTML ────────────────────────────────────────────

html = f'''<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>imp — the worker engine</title>
<style>
  @import url('https://fonts.googleapis.com/css2?family=Share+Tech+Mono&display=swap');
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    background: #080808;
    color: #b09060;
    font-family: 'Share Tech Mono', 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.15;
    padding: 60px 40px;
  }}
  .page {{ max-width: 900px; margin: 0 auto; }}

  /* ─── INK PALETTE ────────────────────────────────────────
   *  class  hex        ratatui Color::Rgb(r,g,b)
   * ────────────────────────────────────────────────────── */
  .k  {{ color: #0d0a06; }}  /* near-black       Rgb(13,10,6)     */
  .dk {{ color: #1a1208; }}  /* deep shadow      Rgb(26,18,8)     */
  .sh {{ color: #2a1e0e; }}  /* shadow           Rgb(42,30,14)    */
  .st {{ color: #3d2e16; }}  /* stone            Rgb(61,46,22)    */
  .sm {{ color: #5a4428; }}  /* smoke            Rgb(90,68,40)    */
  .d  {{ color: #6b5030; }}  /* dim              Rgb(107,80,48)   */
  .g  {{ color: #7a6038; }}  /* ground           Rgb(122,96,56)   */
  .a  {{ color: #9a7840; }}  /* amber            Rgb(154,120,64)  */
  .gb {{ color: #b09050; }}  /* gold base        Rgb(176,144,80)  */
  .gl {{ color: #c8a860; }}  /* gold light       Rgb(200,168,96)  */
  .br {{ color: #d4b870; }}  /* bright gold      Rgb(212,184,112) */
  .wh {{ color: #e8d498; }}  /* near-white       Rgb(232,212,152) */
  .hw {{ color: #f5ecc0; }}  /* hot white        Rgb(245,236,192) */
  .r  {{ color: #8a2010; }}  /* rust             Rgb(138,32,16)   */
  .rd {{ color: #b03018; }}  /* red              Rgb(176,48,24)   */
  .or {{ color: #c85020; }}  /* orange           Rgb(200,80,32)   */
  .f1 {{ color: #e06820; }}  /* fire lo          Rgb(224,104,32)  */
  .f2 {{ color: #f08828; }}  /* fire mid         Rgb(240,136,40)  */
  .f3 {{ color: #ffaa30; }}  /* fire hi          Rgb(255,170,48)  */
  .f4 {{ color: #ffd060; }}  /* flame tip        Rgb(255,208,96)  */
  .eg {{ color: #40c060; }}  /* eye green        Rgb(64,192,96)   */
  .ew {{ color: #80e8a0; }}  /* eye glow         Rgb(128,232,160) */

  pre {{
    white-space: pre;
    font-family: inherit;
    line-height: 1;
    overflow-x: auto;
    position: relative;
  }}
  hr.rule {{ border: none; border-top: 1px solid #2a1e0e; margin: 80px 0; }}

  /* ─── ANIMATIONS ─────────────────────────────────── */
  @keyframes epulse  {{ 0%,100% {{ color:#40c060 }} 50% {{ color:#a0ffc0 }} }}
  @keyframes eglow   {{ 0%,100% {{ color:#80e8a0 }} 50% {{ color:#d0ffe0 }} }}
  @keyframes blink   {{ 0%,89%,94%,100% {{ opacity:1 }} 91.5% {{ opacity:.05 }} }}
  @keyframes af1     {{ 0%,100% {{ color:#e06820 }} 50% {{ color:#e87830 }} }}
  @keyframes af2     {{ 0%,100% {{ color:#f08828 }} 50% {{ color:#f89838 }} }}
  @keyframes af3     {{ 0%,100% {{ color:#ffaa30 }} 50% {{ color:#ffbc48 }} }}
  @keyframes af4     {{ 0%,100% {{ color:#ffd060 }} 50% {{ color:#ffe088 }} }}
  @keyframes breathe {{ 0%,100% {{ color:#3d2e16 }} 50% {{ color:#4a3820 }} }}
  @keyframes scan    {{ 0% {{ background-position:0 -200% }} 100% {{ background-position:0 300% }} }}

  .ep  {{ animation: epulse  3.5s ease-in-out infinite; }}
  .eg2 {{ animation: eglow   3.5s ease-in-out infinite; }}
  .bl2 {{ animation: blink   7s   ease-in-out infinite; }}
  .a1  {{ animation: af1     2.8s ease-in-out infinite; }}
  .a2  {{ animation: af2     2.4s ease-in-out infinite; }}
  .a3  {{ animation: af3     2.0s ease-in-out infinite; }}
  .a4  {{ animation: af4     1.6s ease-in-out infinite; }}
  .br2 {{ animation: breathe 6s   ease-in-out infinite; color: #3d2e16; }}

  .scan::after {{
    content: '';
    position: absolute;
    top: 0; left: 0; right: 0; bottom: 0;
    background: linear-gradient(transparent 0%,rgba(180,140,60,.01) 48%,rgba(180,140,60,.03) 50%,rgba(180,140,60,.01) 52%,transparent 100%);
    background-size: 100% 60px;
    animation: scan 10s linear infinite;
    pointer-events: none;
  }}

  .controls {{ margin-top: 40px; text-align: center; }}
  .controls button {{
    background:#0d0a06; color:#5a4428; border:1px solid #2a1e0e;
    padding:6px 14px; font-family:inherit; font-size:10px;
    cursor:pointer; margin:0 4px; border-radius:2px;
  }}
  .controls button:hover {{ color:#c8a860; border-color:#5a4428; }}
  .toast {{ color:#c8a860; font-size:10px; margin-top:8px; opacity:0; transition:opacity .3s; }}
  .toast.show {{ opacity:1; }}
</style>
</head>
<body>
<div class="page">

<pre id="imp-art" class="scan">
{pre_content}
</pre>

<hr class="rule">

<pre>
{logo_pre}
</pre>

<div class="controls">
  <button onclick="copyRaw()">copy raw</button>
  <button onclick="copyAnsi()">copy ANSI</button>
  <button onclick="tog()">toggle animation</button>
  <div id="toast" class="toast"></div>
</div>

</div>
<script>
function copyRaw(){{
  navigator.clipboard.writeText(document.getElementById('imp-art').textContent)
    .then(()=>toast('copied'));
}}
const A={{
  k:'\\x1b[38;2;13;10;6m',dk:'\\x1b[38;2;26;18;8m',sh:'\\x1b[38;2;42;30;14m',
  st:'\\x1b[38;2;61;46;22m',sm:'\\x1b[38;2;90;68;40m',d:'\\x1b[38;2;107;80;48m',
  g:'\\x1b[38;2;122;96;56m',a:'\\x1b[38;2;154;120;64m',gb:'\\x1b[38;2;176;144;80m',
  gl:'\\x1b[38;2;200;168;96m',br:'\\x1b[38;2;212;184;112m',wh:'\\x1b[38;2;232;212;152m',
  hw:'\\x1b[38;2;245;236;192m',r:'\\x1b[38;2;138;32;16m',rd:'\\x1b[38;2;176;48;24m',
  or:'\\x1b[38;2;200;80;32m',f1:'\\x1b[38;2;224;104;32m',f2:'\\x1b[38;2;240;136;40m',
  f3:'\\x1b[38;2;255;170;48m',f4:'\\x1b[38;2;255;208;96m',
  eg:'\\x1b[38;2;64;192;96m',ew:'\\x1b[38;2;128;232;160m',
  ep:'\\x1b[38;2;64;192;96m',eg2:'\\x1b[38;2;128;232;160m',
  a1:'\\x1b[38;2;224;104;32m',a2:'\\x1b[38;2;240;136;40m',
  a3:'\\x1b[38;2;255;170;48m',a4:'\\x1b[38;2;255;208;96m',
  br2:'\\x1b[38;2;61;46;22m',bl2:'',scan:'',
}};
function s2a(el){{
  let o='';
  for(const n of el.childNodes){{
    if(n.nodeType===3){{o+=n.textContent;continue;}}
    if(n.nodeType!==1||n.tagName!=='SPAN')continue;
    const c=n.className.split(' ').find(x=>A[x]);
    o+=(c&&A[c]?A[c]:'')+s2a(n)+(c&&A[c]?'\\x1b[0m':'');
  }}
  return o;
}}
function copyAnsi(){{
  navigator.clipboard.writeText(s2a(document.getElementById('imp-art')))
    .then(()=>toast('ANSI copied'));
}}
let on=true;
function tog(){{
  on=!on;
  const s=on?'running':'paused';
  document.querySelectorAll('.ep,.eg2,.bl2,.a1,.a2,.a3,.a4,.br2')
    .forEach(e=>e.style.animationPlayState=s);
  document.querySelector('.scan')?.classList.toggle('scan',on);
  toast(on?'on':'paused');
}}
function toast(m){{
  const e=document.getElementById('toast');
  e.textContent=m;e.classList.add('show');
  setTimeout(()=>e.classList.remove('show'),1500);
}}
</script>
</body>
</html>'''

with open('art.html', 'w') as f:
    f.write(html)

print(f"✓ Written art.html ({len(html)} bytes, {TOTAL_W} chars wide)")
