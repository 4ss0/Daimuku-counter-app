"""Illustrated backgrounds (1080x1920), each in a light and a dark version.

    python3 tools/backgrounds/make.py      # writes tools/backgrounds/svg/*.svg

Then render every SVG to static/backgrounds/<name>.jpg at 1080x1920, e.g.
with Chrome: chrome --headless --window-size=1080,1920
--screenshot=out.png file.svg, and save as JPEG (quality ~82).

The subject sits in the upper middle, behind the counter's ring (as in the
original lotus), and the lower part stays calm for the buttons.
"""
import math
import os
import random

W, H = 1080, 1920
OUT = os.path.join(os.path.dirname(__file__), "svg")


def svg(defs, body, bg):
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">'
        f"<defs>{defs}"
        '<filter id="blur8"><feGaussianBlur stdDeviation="8"/></filter>'
        '<filter id="blur20"><feGaussianBlur stdDeviation="20"/></filter>'
        '<filter id="blur40"><feGaussianBlur stdDeviation="40"/></filter>'
        '<filter id="blur70"><feGaussianBlur stdDeviation="70"/></filter>'
        f"</defs>{bg}{body}</svg>"
    )


def vgrad(gid, stops):
    s = "".join(f'<stop offset="{o}" stop-color="{c}"/>' for o, c in stops)
    return f'<linearGradient id="{gid}" x1="0" y1="0" x2="0" y2="1">{s}</linearGradient>'


def rgrad(gid, stops, cx=0.5, cy=0.5, r=0.5):
    s = "".join(f'<stop offset="{o}" stop-color="{c}" stop-opacity="{a}"/>' for o, c, a in stops)
    return f'<radialGradient id="{gid}" cx="{cx}" cy="{cy}" r="{r}">{s}</radialGradient>'


def bokeh(rng, n, colors, y0=0, y1=H, rmin=40, rmax=160, op=(0.08, 0.25)):
    out = []
    for _ in range(n):
        x, y = rng.uniform(-50, W + 50), rng.uniform(y0, y1)
        r = rng.uniform(rmin, rmax)
        out.append(f'<circle cx="{x:.0f}" cy="{y:.0f}" r="{r:.0f}" fill="{rng.choice(colors)}" opacity="{rng.uniform(*op):.2f}"/>')
    return f'<g filter="url(#blur40)">{"".join(out)}</g>'


def ridge(rng, y, amp, freq, color, op=1.0, blur=None, seed=0):
    """A mountain range: a smooth curve across the width, filled down."""
    pts = []
    ph = [rng.uniform(0, 6.28) for _ in range(3)]
    for i in range(0, W + 41, 40):
        t = i / W
        v = (math.sin(t * freq * 6.28 + ph[0]) * 0.55 + math.sin(t * freq * 2.3 * 6.28 + ph[1]) * 0.3
             + math.sin(t * freq * 5.1 * 6.28 + ph[2]) * 0.15)
        pts.append((i, y - amp * (0.5 + 0.5 * v)))
    d = f"M-20 {H} L-20 {pts[0][1]:.0f} " + " ".join(f"L{x} {yy:.0f}" for x, yy in pts) + f" L{W + 20} {H} Z"
    f = f' filter="url(#{blur})"' if blur else ""
    return f'<path d="{d}" fill="{color}" opacity="{op}"{f}/>'


def stars(rng, n, y1=1000):
    out = []
    for _ in range(n):
        x, y, r = rng.uniform(0, W), rng.uniform(0, y1), rng.uniform(0.8, 2.6)
        out.append(f'<circle cx="{x:.0f}" cy="{y:.0f}" r="{r:.1f}" fill="#fff" opacity="{rng.uniform(0.25, 0.85):.2f}"/>')
    return "".join(out)


def lotus(cx, cy, s, petal, inner, edge, op=1.0):
    """A front-facing lotus flower, petals fanned around (cx, cy)."""
    out = []
    for layer, (n, length, width, rot0) in enumerate([(7, 230, 95, 0), (5, 190, 80, 18), (3, 150, 70, 0)]):
        for k in range(n):
            a = -90 + (k - (n - 1) / 2) * (150 / max(1, n - 1)) + (rot0 if layer == 1 else 0) * 0
            L, Wd = length * s, width * s
            d = f"M0 0 C {-Wd:.0f} {-L * 0.35:.0f} {-Wd * 0.7:.0f} {-L * 0.85:.0f} 0 {-L:.0f} C {Wd * 0.7:.0f} {-L * 0.85:.0f} {Wd:.0f} {-L * 0.35:.0f} 0 0 Z"
            fill = petal if layer < 2 else inner
            out.append(
                f'<path d="{d}" transform="translate({cx} {cy}) rotate({a + 90:.1f})" fill="{fill}" '
                f'stroke="{edge}" stroke-width="{2 * s:.1f}" opacity="{op}"/>'
            )
    return "".join(out)


def blossom(cx, cy, r, rot, fill, center, op):
    """A five-petal cherry blossom (notched petals)."""
    p = []
    for k in range(5):
        a = rot + k * 72
        d = (f"M0 0 C {-0.55 * r:.1f} {-0.3 * r:.1f} {-0.6 * r:.1f} {-0.9 * r:.1f} {-0.18 * r:.1f} {-r:.1f} "
             f"L0 {-0.82 * r:.1f} L{0.18 * r:.1f} {-r:.1f} C {0.6 * r:.1f} {-0.9 * r:.1f} {0.55 * r:.1f} {-0.3 * r:.1f} 0 0 Z")
        p.append(f'<path d="{d}" transform="translate({cx:.1f} {cy:.1f}) rotate({a:.1f})" fill="{fill}"/>')
    p.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r * 0.18:.1f}" fill="{center}"/>')
    return f'<g opacity="{op:.2f}">{"".join(p)}</g>'


# ------------------------------------------------------------------ scenes


def sutra(dark):
    rng = random.Random(11)
    if dark:
        sky = [(0, "#0d0b1f"), (0.45, "#1d1638"), (1, "#0a0910")]
        glow, paper, ink, rod, gold, petal = "#f2c46d", "#e9dcc0", "#2a1d12", "#3a2414", "#d9a441", "#f3a9c4"
    else:
        sky = [(0, "#fbe9d0"), (0.5, "#f6d7c3"), (1, "#efe6da")]
        glow, paper, ink, rod, gold, petal = "#ffd88a", "#fffaf0", "#3b2a1a", "#6b4426", "#c9902e", "#f4a6c0"
    defs = vgrad("sky", sky) + rgrad("halo", [(0, glow, 0.85), (0.5, glow, 0.25), (1, glow, 0)])
    bg = f'<rect width="{W}" height="{H}" fill="url(#sky)"/>'
    body = bokeh(rng, 18, [glow, petal, "#ffffff"], 0, 1500, op=(0.06, 0.18) if dark else (0.1, 0.3))
    if dark:
        body += stars(rng, 90, 900)
    # light rays from behind the scroll
    rays = []
    for k in range(18):
        a = math.radians(k * 20 + 5)
        x2, y2 = 540 + math.cos(a) * 1300, 700 + math.sin(a) * 1300
        rays.append(f'<path d="M540 700 L{x2 - 60 * math.sin(a):.0f} {y2 + 60 * math.cos(a):.0f} L{x2 + 60 * math.sin(a):.0f} {y2 - 60 * math.cos(a):.0f} Z" fill="{glow}"/>')
    body += f'<g opacity="{0.12 if dark else 0.18}" filter="url(#blur20)">{"".join(rays)}</g>'
    body += f'<circle cx="540" cy="700" r="520" fill="url(#halo)"/>'
    # hanging scroll with the title of the sutra
    body += f'<rect x="395" y="300" width="290" height="820" rx="6" fill="{paper}" opacity="0.96"/>'
    body += f'<rect x="420" y="330" width="240" height="760" fill="none" stroke="{gold}" stroke-width="3" opacity="0.7"/>'
    body += f'<rect x="372" y="282" width="336" height="26" rx="13" fill="{rod}"/><circle cx="372" cy="295" r="16" fill="{gold}"/><circle cx="708" cy="295" r="16" fill="{gold}"/>'
    body += f'<rect x="372" y="1110" width="336" height="30" rx="15" fill="{rod}"/><circle cx="372" cy="1125" r="18" fill="{gold}"/><circle cx="708" cy="1125" r="18" fill="{gold}"/>'
    body += f'<path d="M540 282 L470 210 M540 282 L610 210" stroke="{rod}" stroke-width="5" fill="none"/><circle cx="540" cy="205" r="9" fill="{gold}"/>'
    chars = "妙法蓮華経"
    for i, ch in enumerate(chars):
        body += (f'<text x="540" y="{470 + i * 140}" text-anchor="middle" font-family="IPAGothic, \'Noto Sans CJK JP\', sans-serif" '
                 f'font-size="112" fill="{ink}" opacity="0.88">{ch}</text>')
    # lotus at the foot of the scroll, petals drifting
    body += lotus(540, 1290, 0.95, petal, "#fde3ec" if not dark else "#f7c7d8", "#ffffff55", 0.92)
    for _ in range(14):
        x, y, r = rng.uniform(80, 1000), rng.uniform(200, 1500), rng.uniform(12, 26)
        body += f'<ellipse cx="{x:.0f}" cy="{y:.0f}" rx="{r:.0f}" ry="{r * 0.45:.0f}" transform="rotate({rng.uniform(0, 180):.0f} {x:.0f} {y:.0f})" fill="{petal}" opacity="{rng.uniform(0.35, 0.7):.2f}"/>'
    return svg(defs, body, bg)


def buddha(dark):
    rng = random.Random(22)
    if dark:
        sky = [(0, "#08101f"), (0.5, "#13223a"), (1, "#070b12")]
        disc, figure, rim, leaf, throne = "#e8e2c8", "#05070c", "#f3d38a", "#0e1a14", "#0b0f16"
    else:
        sky = [(0, "#fde7b8"), (0.45, "#f8caa0"), (1, "#f2e4d4")]
        disc, figure, rim, leaf, throne = "#fff4cf", "#7a4a2a", "#ffe7a3", "#5f7d4a", "#8a5a36"
    defs = vgrad("sky", sky) + rgrad("disc", [(0, disc, 1), (0.6, disc, 0.85), (1, disc, 0)])
    defs += rgrad("aura", [(0, rim, 0.55), (1, rim, 0)])
    bg = f'<rect width="{W}" height="{H}" fill="url(#sky)"/>'
    body = ""
    if dark:
        body += stars(rng, 140, 1100)
    body += bokeh(rng, 14, [rim, "#ffffff"], 0, 1300, op=(0.05, 0.15) if dark else (0.08, 0.22))
    body += f'<circle cx="540" cy="640" r="430" fill="url(#aura)"/>'
    body += f'<circle cx="540" cy="620" r="230" fill="url(#disc)"/>'
    # distant hills
    body += ridge(rng, 1330, 120, 1.2, "#1a2636" if dark else "#e7b98f", 0.9, "blur8")
    body += ridge(rng, 1450, 90, 1.7, "#0f1724" if dark else "#d9a37c", 1.0)
    # bodhi leaves framing the top
    for side in (-1, 1):
        for k in range(16):
            x = 540 + side * rng.uniform(260, 620)
            y = rng.uniform(-20, 420)
            s = rng.uniform(0.6, 1.25)
            a = rng.uniform(-40, 40) + (180 if y < 80 else 160 if side < 0 else 200)
            d = "M0 0 C -40 -20 -46 -70 0 -110 C 46 -70 40 -20 0 0 Z M0 -110 L0 -150"
            body += f'<path d="{d}" transform="translate({x:.0f} {y:.0f}) rotate({a:.0f}) scale({s:.2f})" fill="{leaf}" stroke="{leaf}" stroke-width="3" opacity="{0.85 if dark else 0.7}"/>'
    # lotus throne
    for k in range(9):
        x = 540 + (k - 4) * 62
        body += f'<path d="M{x - 40} 1180 C {x - 40} 1130 {x} 1105 {x} 1100 C {x} 1105 {x + 40} 1130 {x + 40} 1180 Z" fill="{throne}"/>'
    body += f'<rect x="300" y="1176" width="480" height="34" rx="14" fill="{throne}"/>'
    # the seated Buddha (dhyana mudra, legs crossed)
    g = []
    g.append(
        '<path d="M486 688 C 446 694 404 708 386 742 C 362 792 356 852 358 906 C 344 954 304 994 286 1046 '
        'C 272 1090 292 1122 344 1128 L 736 1128 C 788 1122 808 1090 794 1046 C 776 994 736 954 722 906 '
        'C 724 852 718 792 694 742 C 676 708 634 694 594 688 Z"/>'
    )
    g.append('<path d="M500 640 L500 700 L580 700 L580 640 Z"/>')
    g.append('<ellipse cx="540" cy="596" rx="58" ry="70"/>')
    g.append('<ellipse cx="540" cy="522" rx="28" ry="24"/>')
    g.append('<ellipse cx="480" cy="616" rx="13" ry="42"/><ellipse cx="600" cy="616" rx="13" ry="42"/>')
    body += f'<g fill="{figure}" stroke="{rim}" stroke-width="3" stroke-opacity="0.55">{"".join(g)}</g>'
    # robe folds and the hands resting in the lap
    body += (f'<g fill="none" stroke="{rim}" stroke-opacity="0.35" stroke-width="3">'
             '<path d="M392 748 C 470 820 560 880 690 900"/><path d="M362 912 C 420 950 470 980 500 1000"/>'
             '<path d="M718 912 C 660 950 610 980 580 1000"/><path d="M300 1060 C 420 1040 660 1040 780 1060"/></g>')
    body += f'<ellipse cx="540" cy="1004" rx="86" ry="30" fill="{figure}" stroke="{rim}" stroke-opacity="0.45" stroke-width="2"/>'
    return svg(defs, body, bg)


def temple(dark):
    rng = random.Random(33)
    if dark:
        sky = [(0, "#0b1022"), (0.55, "#26203f"), (1, "#0b0b12")]
        moon, far, mid, near, wood, roof, mist = "#f1ead2", "#2b2c48", "#1a1b2e", "#0d0e18", "#120d0c", "#0a0809", "#8d87b6"
    else:
        sky = [(0, "#f9d9b8"), (0.5, "#f3c3a6"), (1, "#efe1d3")]
        moon, far, mid, near, wood, roof, mist = "#fff1d6", "#d8a99a", "#b98276", "#7d4e45", "#5a2f25", "#3b2620", "#ffffff"
    defs = vgrad("sky", sky) + rgrad("moon", [(0, moon, 1), (0.7, moon, 0.9), (1, moon, 0)])
    bg = f'<rect width="{W}" height="{H}" fill="url(#sky)"/>'
    body = stars(rng, 110, 900) if dark else ""
    body += f'<circle cx="330" cy="560" r="150" fill="url(#moon)"/>'
    body += f'<circle cx="330" cy="560" r="330" fill="{moon}" opacity="0.12" filter="url(#blur40)"/>'
    body += ridge(rng, 1060, 260, 0.9, far, 0.9, "blur8")
    body += f'<rect y="900" width="{W}" height="240" fill="{mist}" opacity="{0.10 if dark else 0.35}" filter="url(#blur40)"/>'
    body += ridge(rng, 1230, 200, 1.3, mid, 1.0)
    # five-storey pagoda
    cx, y, w, h = 690, 1280, 330, 120
    parts = []
    for k in range(5):
        wb = w * 0.6
        parts.append(f'<rect x="{cx - wb / 2:.0f}" y="{y - h * 0.5:.0f}" width="{wb:.0f}" height="{h * 0.5:.0f}" fill="{wood}"/>')
        ry = y - h * 0.5
        e = w * 0.16
        parts.append(
            f'<path d="M{cx - w / 2 - e:.0f} {ry - 4:.0f} Q {cx - w / 2:.0f} {ry + 10:.0f} {cx - w / 2 + 30:.0f} {ry + 12:.0f} '
            f'L {cx + w / 2 - 30:.0f} {ry + 12:.0f} Q {cx + w / 2:.0f} {ry + 10:.0f} {cx + w / 2 + e:.0f} {ry - 4:.0f} '
            f'L {cx + w / 2 - 20:.0f} {ry - 26:.0f} L {cx - w / 2 + 20:.0f} {ry - 26:.0f} Z" fill="{roof}"/>'
        )
        y -= h
        w *= 0.84
        h *= 0.9
    parts.append(f'<rect x="{cx - 5}" y="{y - 210:.0f}" width="10" height="{230}" fill="{roof}"/>')
    for k in range(9):
        parts.append(f'<ellipse cx="{cx}" cy="{y - 30 - k * 18:.0f}" rx="{20 - k:.0f}" ry="4" fill="{roof}"/>')
    parts.append(f'<circle cx="{cx}" cy="{y - 215:.0f}" r="10" fill="{roof}"/>')
    body += "".join(parts)
    # temple hall on the left
    body += f'<rect x="110" y="1220" width="300" height="110" fill="{wood}"/>'
    body += f'<path d="M40 1225 Q 120 1240 160 1236 L 360 1236 Q 400 1240 480 1225 L 400 1150 L 120 1150 Z" fill="{roof}"/>'
    body += f'<path d="M150 1150 L 260 1090 L 370 1150 Z" fill="{roof}"/>'
    body += ridge(rng, 1400, 90, 1.8, near, 1.0)
    # pine branches
    for side, x0 in ((-1, 0), (1, W)):
        for k in range(7):
            yy = 1380 + k * 26
            body += f'<ellipse cx="{x0 + side * -1 * (60 + k * 22):.0f}" cy="{yy}" rx="{110 - k * 8}" ry="20" fill="{near}"/>'
    body += f'<rect y="1440" width="{W}" height="{H - 1440}" fill="{near}"/>'
    return svg(defs, body, bg)


def nature(dark):
    rng = random.Random(44)
    if dark:
        sky = [(0, "#071526"), (0.5, "#12304a"), (1, "#06101a")]
        sun, l1, l2, l3, water, pine = "#e9f1f2", "#2d4a63", "#1d3449", "#10202f", "#0b1a27", "#06111a"
    else:
        sky = [(0, "#cfe9f2"), (0.5, "#e6f2e8"), (1, "#e9efe2")]
        sun, l1, l2, l3, water, pine = "#fff6d6", "#a9c8c9", "#7fa9a2", "#4f7f6e", "#bcd8d6", "#2f5a48"
    defs = vgrad("sky", sky) + rgrad("sun", [(0, sun, 1), (0.6, sun, 0.8), (1, sun, 0)])
    defs += vgrad("water", [(0, water), (1, l3)])
    bg = f'<rect width="{W}" height="{H}" fill="url(#sky)"/>'
    body = stars(rng, 120, 900) if dark else ""
    body += f'<circle cx="680" cy="520" r="130" fill="url(#sun)"/>'
    body += f'<circle cx="680" cy="520" r="320" fill="{sun}" opacity="0.18" filter="url(#blur40)"/>'
    body += ridge(rng, 1000, 330, 0.7, l1, 0.85, "blur8")
    body += f'<rect y="840" width="{W}" height="200" fill="#ffffff" opacity="{0.06 if dark else 0.35}" filter="url(#blur40)"/>'
    body += ridge(rng, 1130, 230, 1.1, l2, 1.0)
    body += ridge(rng, 1220, 120, 1.9, l3, 1.0)
    # lake with the reflection of the sun
    body += f'<rect y="1215" width="{W}" height="{H - 1215}" fill="url(#water)"/>'
    for k in range(14):
        yy = 1250 + k * 22
        wdt = 160 - k * 9
        body += f'<rect x="{680 - wdt / 2:.0f}" y="{yy}" width="{wdt}" height="4" rx="2" fill="{sun}" opacity="{0.5 - k * 0.03:.2f}"/>'
    # lotus leaves on the water
    for _ in range(9):
        x, y, r = rng.uniform(60, 1020), rng.uniform(1420, 1800), rng.uniform(40, 90)
        body += f'<ellipse cx="{x:.0f}" cy="{y:.0f}" rx="{r:.0f}" ry="{r * 0.32:.0f}" fill="{pine}" opacity="0.75"/>'
    # pines at the sides
    for x0, sgn in ((70, 1), (1010, -1)):
        for k in range(10):
            yy = 560 + k * 70
            wdt = 40 + k * 18
            body += f'<path d="M{x0} {yy - 70} L{x0 - wdt} {yy + 30} L{x0 + wdt} {yy + 30} Z" fill="{pine}"/>'
        body += f'<rect x="{x0 - 10}" y="1250" width="20" height="110" fill="{pine}"/>'
    # birds
    for _ in range(5):
        x, y, s = rng.uniform(300, 900), rng.uniform(260, 420), rng.uniform(0.6, 1.2)
        body += f'<path d="M{x - 20 * s:.0f} {y:.0f} Q {x - 10 * s:.0f} {y - 10 * s:.0f} {x:.0f} {y:.0f} Q {x + 10 * s:.0f} {y - 10 * s:.0f} {x + 20 * s:.0f} {y:.0f}" stroke="{pine}" stroke-width="3" fill="none"/>'
    return svg(defs, body, bg)


def sakura(dark):
    rng = random.Random(55)
    if dark:
        sky = [(0, "#110e24"), (0.5, "#2a1d3d"), (1, "#0d0b14")]
        moon, branch, pet, pet2, center = "#f6eedc", "#1b1113", "#f2b7cf", "#e48fb2", "#ffd36b"
    else:
        sky = [(0, "#dbeefa"), (0.55, "#f8e3ec"), (1, "#f6eef0")]
        moon, branch, pet, pet2, center = "#ffffff", "#5b3a33", "#f9c6d8", "#f19bbd", "#e7a33a"
    defs = vgrad("sky", sky) + rgrad("moon", [(0, moon, 1), (0.7, moon, 0.9), (1, moon, 0)])
    bg = f'<rect width="{W}" height="{H}" fill="url(#sky)"/>'
    body = stars(rng, 100, 900) if dark else ""
    body += f'<circle cx="760" cy="620" r="{140 if dark else 200}" fill="url(#moon)" opacity="{1 if dark else 0.55}"/>'
    body += bokeh(rng, 24, [pet, pet2, "#ffffff"], 0, 1500, op=(0.08, 0.22))

    def bough(x0, y0, ang, length, width, depth, out):
        x1 = x0 + math.cos(ang) * length
        y1 = y0 + math.sin(ang) * length
        cxm = (x0 + x1) / 2 + rng.uniform(-30, 30)
        cym = (y0 + y1) / 2 + rng.uniform(-30, 30)
        out.append(f'<path d="M{x0:.0f} {y0:.0f} Q {cxm:.0f} {cym:.0f} {x1:.0f} {y1:.0f}" stroke="{branch}" stroke-width="{width:.1f}" stroke-linecap="round" fill="none"/>')
        # blossoms along the whole bough, more of them towards its tip
        tips = [(x0 + (x1 - x0) * f, y0 + (y1 - y0) * f) for f in (0.45, 0.75, 1.0)]
        if depth > 0:
            for da in (-0.5, 0.45):
                tips += bough(x1, y1, ang + da + rng.uniform(-0.2, 0.2), length * 0.72, width * 0.62, depth - 1, out)
        return tips

    for x0, y0, ang in ((-40, 140, 0.35), (W + 40, 60, math.pi - 0.45), (-30, 760, -0.25)):
        parts = []
        tips = bough(x0, y0, ang, 300, 26, 3, parts)
        body += "".join(parts)
        for tx, ty in tips:
            for _ in range(rng.randint(1, 3)):
                bx, by = tx + rng.uniform(-45, 45), ty + rng.uniform(-40, 40)
                body += blossom(bx, by, rng.uniform(16, 30), rng.uniform(0, 72), rng.choice([pet, pet2, "#ffffff" if not dark else pet]), center, rng.uniform(0.8, 1))
    # falling petals
    for _ in range(40):
        x, y, r = rng.uniform(0, W), rng.uniform(300, 1850), rng.uniform(7, 14)
        body += f'<ellipse cx="{x:.0f}" cy="{y:.0f}" rx="{r:.0f}" ry="{r * 0.55:.0f}" transform="rotate({rng.uniform(0, 180):.0f} {x:.0f} {y:.0f})" fill="{rng.choice([pet, pet2])}" opacity="{rng.uniform(0.4, 0.85):.2f}"/>'
    return svg(defs, body, bg)


SCENES = {"sutra": sutra, "buddha": buddha, "temple": temple, "nature": nature, "sakura": sakura}

if __name__ == "__main__":
    os.makedirs(OUT, exist_ok=True)
    for name, fn in SCENES.items():
        for dark in (False, True):
            path = os.path.join(OUT, f"{name}-{'dark' if dark else 'light'}.svg")
            with open(path, "w", encoding="utf-8") as f:
                f.write(fn(dark))
            print(path)
