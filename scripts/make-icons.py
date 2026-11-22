#!/usr/bin/env python3
"""VIDE-DE's icon theme, `vide`: every icon our shell and apps ask for, drawn in one style
(flat, Windoze 10/11-like: manila folders, white pages with a coloured mark, rounded app
tiles, colourful toolbar glyphs). Writes assets/icons/vide/. Run it again after changing
anything here; the SVGs are generated, not edited by hand.

    python3 scripts/make-icons.py
"""

import os
import shutil

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "icons", "vide")

BLUE = "#0A74DA"
BLUE_LIGHT = "#4CA9F0"
GREEN = "#1E9E4A"
RED = "#E5484D"
ORANGE = "#F29D12"
PURPLE = "#8B5CF6"
PINK = "#E0457B"
GRAY = "#6B7280"
DARK = "#30343B"
AMBER = "#C27F12"  # a mark pressed into a folder's front


def svg(body, defs=""):
    d = f"<defs>{defs}</defs>" if defs else ""
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">{d}{body}</svg>\n'


def grad(id, top, bottom):
    return f'<linearGradient id="{id}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient>'


# ------------------------------------------------------------------ folders

FOLDER_DEFS = grad("back", "#E9A93A", "#D48F1F") + grad("front", "#FFD873", "#F6BF45") + grad("paper", "#FFFFFF", "#ECECEC")
FOLDER_BACK = (
    '<path d="M24 58a10 10 0 0 1 10-10h62a12 12 0 0 1 9 4l14 16h103a10 10 0 0 1 10 10v128a10 10 0 0 1-10 10H34a10 10 0 0 1-10-10z" fill="url(#back)"/>'
    '<rect x="42" y="80" width="172" height="110" rx="4" fill="url(#paper)"/>'
)
FOLDER_FRONT = (
    '<path d="M24 106a10 10 0 0 1 10-10h188a10 10 0 0 1 10 10v98a10 10 0 0 1-10 10H34a10 10 0 0 1-10-10z" fill="url(#front)"/>'
    '<path d="M34 97h188" stroke="#FFE7A6" stroke-width="2" stroke-linecap="round"/>'
)
# Open: the front flap swings down and forward.
FOLDER_FRONT_OPEN = (
    '<path d="M44 112a10 10 0 0 1 9.6-7.2h186a8 8 0 0 1 7.7 10.2l-26 92A10 10 0 0 1 211.7 214H30a8 8 0 0 1-7.7-10.2z" fill="url(#front)"/>'
)

# Marks pressed into the front flap, centred on (128, 155).
MARKS = {
    "documents": f'<g fill="none" stroke="{AMBER}" stroke-width="9" stroke-linecap="round"><rect x="102" y="122" width="52" height="66" rx="5"/><path d="M114 142h28M114 156h28M114 170h18"/></g>',
    "download": f'<g fill="none" stroke="{AMBER}" stroke-width="11" stroke-linecap="round" stroke-linejoin="round"><path d="M128 120v52M106 152l22 22 22-22M100 188h56"/></g>',
    "music": f'<g fill="{AMBER}"><path d="M112 128l44-10v50a14 12 0 1 1-9-11.3V136l-26 6v36a14 12 0 1 1-9-11.3z"/></g>',
    "pictures": f'<g><rect x="96" y="124" width="64" height="58" rx="6" fill="none" stroke="{AMBER}" stroke-width="9"/><circle cx="142" cy="140" r="7" fill="{AMBER}"/><path d="M100 178l20-24 14 16 8-8 14 16z" fill="{AMBER}"/></g>',
    "videos": f'<g><rect x="94" y="126" width="68" height="56" rx="8" fill="none" stroke="{AMBER}" stroke-width="9"/><path d="M120 140v28l24-14z" fill="{AMBER}"/></g>',
    "home": f'<g fill="none" stroke="{AMBER}" stroke-width="10" stroke-linejoin="round" stroke-linecap="round"><path d="M98 154l30-28 30 28M108 146v38h40v-38"/></g>',
    "desktop": f'<g fill="none" stroke="{AMBER}" stroke-width="9" stroke-linecap="round" stroke-linejoin="round"><rect x="96" y="124" width="64" height="44" rx="5"/><path d="M128 168v16M112 186h32"/></g>',
}


def plus_badge(cx=200, cy=200, color=GREEN):
    return f'<circle cx="{cx}" cy="{cy}" r="34" fill="{color}" stroke="#fff" stroke-width="8"/><path d="M{cx} {cy - 17}v34M{cx - 17} {cy}h34" stroke="#fff" stroke-width="10" stroke-linecap="round"/>'


def folder(mark="", open_=False, extra=""):
    front = FOLDER_FRONT_OPEN if open_ else FOLDER_FRONT
    m = MARKS.get(mark, "")
    return svg(FOLDER_BACK + front + m + extra, FOLDER_DEFS)


# ------------------------------------------------------------------ pages (file types)

PAGE_DEFS = grad("page", "#FFFFFF", "#F1F1F1")
PAGE = (
    '<path d="M58 24h96l46 46v152a10 10 0 0 1-10 10H58a10 10 0 0 1-10-10V34a10 10 0 0 1 10-10z" fill="url(#page)" stroke="#C9CDD3" stroke-width="3"/>'
    '<path d="M154 24v36a10 10 0 0 0 10 10h36" fill="#E3E6EA" stroke="#C9CDD3" stroke-width="3" stroke-linejoin="round"/>'
)
LINES = '<g stroke="#B7BDC6" stroke-width="8" stroke-linecap="round"><path d="M74 96h80M74 120h104M74 144h104M74 168h104M74 192h64"/></g>'


def page(mark="", lines=True):
    return svg(PAGE + (LINES if lines else "") + mark, PAGE_DEFS)


# Letters as strokes, not <text>: the SVG renderers in vide-desktop and the compositor run
# without fonts, so text would come out blank.
PDF_LETTERS = "M58 198v-34h14a10 10 0 0 1 0 20H58M94 164v34h10a17 17 0 0 0 0-34zM134 198v-34h24M134 180h18"


def question(cx, cy, s, color):
    # A question mark centred on (cx, cy), `s` tall.
    k = s / 100
    return (
        f'<path d="M{cx - 22 * k} {cy - 26 * k}a24 24 0 1 1 34 22c-8 4-12 9-12 17v6" fill="none" stroke="{color}" stroke-width="{16 * k}" stroke-linecap="round" stroke-linejoin="round"/>'
        f'<circle cx="{cx}" cy="{cy + 40 * k}" r="{10 * k}" fill="{color}"/>'
    )


def band(color):
    # A coloured label across the lower page, like Explorer's file-type badges.
    return f'<rect x="40" y="150" width="128" height="62" rx="8" fill="{color}"/>' + stroke(PDF_LETTERS, "#fff", 9)


# ------------------------------------------------------------------ app tiles

def tile(top, bottom, glyph, id="t"):
    return svg(f'<rect x="28" y="28" width="200" height="200" rx="44" fill="url(#{id})"/>' + glyph, grad(id, top, bottom))


GEAR = (
    '<g transform="translate(128 128)"><g fill="{c}">'
    + "".join(f'<rect x="-13" y="-74" width="26" height="30" rx="5" transform="rotate({a})"/>' for a in range(0, 360, 45))
    + '<circle r="50"/></g><circle r="22" fill="{hole}"/></g>'
)


def gear(c="#fff", hole="#5B6B80"):
    return GEAR.replace("{c}", c).replace("{hole}", hole)


def person(c):
    return f'<circle cx="128" cy="98" r="34" fill="{c}"/><path d="M62 196c4-38 32-58 66-58s62 20 66 58z" fill="{c}"/>'


# ------------------------------------------------------------------ toolbar glyphs

def stroke(d, color, w=18, extra=""):
    return f'<path d="{d}" fill="none" stroke="{color}" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round"{extra}/>'


SMALL_PAGE = '<path d="M{x} {y}h{w1}l{f} {f}v{h1}a8 8 0 0 1-8 8h-{w2}a8 8 0 0 1-8-8v-{h2}a8 8 0 0 1 8-8z" fill="#fff" stroke="{c}" stroke-width="12" stroke-linejoin="round"/>'


def small_page(x, y, w, h, c=BLUE, f=26):
    return f'<path d="M{x + 8} {y}h{w - 8 - f}l{f} {f}v{h - f - 8}a8 8 0 0 1-8 8h-{w - 16}a8 8 0 0 1-8-8v-{h - 16}a8 8 0 0 1 8-8z" fill="#fff" stroke="{c}" stroke-width="12" stroke-linejoin="round"/>'


MAGNIFIER = f'<circle cx="108" cy="108" r="58" fill="#E8F3FD" stroke="{BLUE}" stroke-width="20"/>' + stroke("M152 152l58 58", DARK, 26)
ARROW_RIGHT = stroke("M48 128h150M142 72l56 56-56 56", BLUE, 24)
CIRCLE_ARROW = stroke("M196 128a68 68 0 1 1-20-48", BLUE, 22) + f'<path d="M206 40v58h-58z" fill="{BLUE}"/>'
SCISSORS = (
    stroke("M92 40l92 132M164 40L72 172", GRAY, 16)
    + f'<circle cx="66" cy="196" r="28" fill="none" stroke="{RED}" stroke-width="18"/><circle cx="190" cy="196" r="28" fill="none" stroke="{RED}" stroke-width="18"/>'
)
CLIPBOARD = (
    f'<rect x="52" y="44" width="152" height="184" rx="14" fill="#C48A4A"/><rect x="72" y="72" width="112" height="138" rx="6" fill="#fff"/>'
    f'<rect x="96" y="30" width="64" height="34" rx="10" fill="{GRAY}"/>'
    '<g stroke="#B7BDC6" stroke-width="10" stroke-linecap="round"><path d="M92 112h72M92 140h72M92 168h48"/></g>'
)
PENCIL = '<g transform="rotate(45 128 128)"><rect x="104" y="24" width="48" height="150" rx="6" fill="#F6BF45"/><rect x="104" y="24" width="48" height="28" rx="6" fill="#E5484D"/><path d="M104 174h48l-24 50z" fill="#F3D6A8"/><path d="M118 205h20l-10 19z" fill="#30343B"/></g>'
FLOPPY = f'<path d="M44 52a12 12 0 0 1 12-12h126l34 34v126a12 12 0 0 1-12 12H56a12 12 0 0 1-12-12z" fill="{BLUE}"/><rect x="80" y="40" width="88" height="56" rx="4" fill="#E8F3FD"/><rect x="140" y="50" width="18" height="36" rx="3" fill="{BLUE}"/><rect x="72" y="132" width="112" height="80" rx="6" fill="#fff"/>'
CHAIN = (
    '<g fill="none" stroke-width="22" stroke-linecap="round">'
    f'<path d="M112 152l-28 28a30 30 0 0 1-42-42l28-28a30 30 0 0 1 42 0" stroke="{GRAY}"/>'
    f'<path d="M144 104l28-28a30 30 0 0 1 42 42l-28 28a30 30 0 0 1-42 0" stroke="{BLUE}"/>'
    f'<path d="M104 152l48-48" stroke="{DARK}"/></g>'
)
POWER = stroke("M88 76a76 76 0 1 0 80 0", RED, 24) + stroke("M128 40v84", RED, 24)
WINDOW = f'<rect x="32" y="48" width="192" height="160" rx="14" fill="#fff" stroke="{BLUE}" stroke-width="14"/><path d="M39 92h178" stroke="{BLUE}" stroke-width="30"/>'
GRID = "".join(f'<rect x="{x}" y="{y}" width="72" height="72" rx="10" fill="{BLUE}"/>' for x in (44, 140) for y in (44, 140))
DOTTED = f'<rect x="44" y="44" width="168" height="168" rx="12" fill="none" stroke="{BLUE}" stroke-width="14" stroke-dasharray="26 18"/>'


def actions():
    a = {}
    a["edit-copy"] = svg(small_page(40, 32, 120, 150, GRAY) + small_page(96, 74, 120, 150, BLUE))
    a["edit-cut"] = svg(SCISSORS)
    a["edit-paste"] = svg(CLIPBOARD)
    a["edit-delete"] = svg(stroke("M64 64l128 128M192 64L64 192", RED, 34))
    a["edit-rename"] = svg(f'<rect x="28" y="84" width="200" height="88" rx="12" fill="#fff" stroke="{GRAY}" stroke-width="12"/>' + stroke("M72 112v32", DARK, 12) + stroke("M150 60v136M128 60h44M128 196h44", BLUE, 12))
    a["edit-undo"] = svg(stroke("M76 104h96a52 52 0 0 1 0 104h-60", BLUE, 24) + stroke("M108 64l-40 40 40 40", BLUE, 24))
    a["edit-redo"] = svg(stroke("M180 104H84a52 52 0 0 0 0 104h60", BLUE, 24) + stroke("M148 64l40 40-40 40", BLUE, 24))
    a["edit-find"] = svg(MAGNIFIER)
    a["edit-select-all"] = svg(DOTTED + stroke("M84 130l30 30 58-62", GREEN, 22))
    a["edit-select-none"] = svg(DOTTED)
    a["edit-select-invert"] = svg(DOTTED + f'<path d="M128 56h72a12 12 0 0 1 12 12v120a12 12 0 0 1-12 12h-72z" fill="{BLUE}"/>')
    a["edit-move"] = svg(small_page(28, 44, 112, 140, GRAY) + stroke("M128 168h96M188 132l36 36-36 36", BLUE, 22))
    a["edit-copy-path"] = svg(small_page(40, 24, 124, 156, GRAY) + '<g transform="translate(104 104) scale(.58)">' + CHAIN + "</g>")
    a["edit-clear"] = svg('<g transform="rotate(-40 128 128)"><rect x="56" y="88" width="144" height="80" rx="12" fill="#F2A7B4"/><rect x="56" y="88" width="56" height="80" rx="12" fill="#E5484D"/></g>' + stroke("M60 212h136", GRAY, 12))
    a["document-new"] = svg(small_page(48, 24, 140, 190, GRAY) + plus_badge(184, 186))
    a["document-open"] = folder(open_=True)
    a["document-properties"] = svg(small_page(48, 24, 140, 190, GRAY) + '<g stroke="#B7BDC6" stroke-width="10" stroke-linecap="round"><path d="M76 92h72M76 120h84M76 148h56"/></g>' + stroke("M140 186l22 22 46-50", RED, 20))
    a["document-save-as"] = svg(FLOPPY + '<g transform="translate(84 84) scale(.68)">' + PENCIL + "</g>")
    a["document-edit"] = svg(small_page(40, 24, 140, 190, GRAY) + '<g transform="translate(70 70) scale(.72)">' + PENCIL + "</g>")
    a["document-revert"] = svg(CIRCLE_ARROW)
    a["folder-new"] = folder(extra=plus_badge())
    a["go-up"] = svg(stroke("M128 208V58M72 114l56-56 56 56", BLUE, 24))
    a["go-previous"] = svg(stroke("M208 128H58M114 72l-56 56 56 56", BLUE, 24))
    a["go-next"] = svg(ARROW_RIGHT)
    a["view-refresh"] = svg(CIRCLE_ARROW)
    a["view-list-icons"] = svg(GRID)
    a["view-list-details"] = svg("".join(f'<rect x="40" y="{y}" width="40" height="28" rx="5" fill="{BLUE}"/><rect x="96" y="{y + 4}" width="120" height="20" rx="5" fill="{GRAY}"/>' for y in (48, 114, 180)))
    a["view-list-text"] = svg("".join(stroke(f"M48 {y}h160", GRAY, 18) for y in (64, 108, 152, 196)))
    tree = f'<rect x="36" y="40" width="64" height="44" rx="8" fill="{BLUE}"/><rect x="132" y="104" width="88" height="40" rx="8" fill="{GRAY}"/><rect x="132" y="176" width="88" height="40" rx="8" fill="{GRAY}"/>' + stroke("M68 84v112h64M68 124h64", DARK, 10)
    a["view-list-tree"] = svg(tree)
    a["view-sidetree"] = svg(f'<rect x="28" y="44" width="200" height="168" rx="14" fill="#fff" stroke="{GRAY}" stroke-width="12"/><rect x="34" y="50" width="62" height="156" fill="{BLUE}" opacity=".85"/>')
    a["view-sort"] = svg(stroke("M80 48v160M48 176l32 32 32-32", BLUE, 20) + stroke("M144 64h72M144 112h56M144 160h40", GRAY, 18))
    a["zoom-in"] = svg(MAGNIFIER + stroke("M108 80v56M80 108h56", BLUE, 16))
    a["zoom-fit-best"] = svg(stroke("M40 92V40h52M164 40h52v52M216 164v52h-52M92 216H40v-52", BLUE, 20) + f'<rect x="88" y="88" width="80" height="80" rx="8" fill="{GRAY}"/>')
    a["window-new"] = svg(WINDOW + plus_badge(192, 186))
    a["window-close"] = svg(f'<rect x="32" y="32" width="192" height="192" rx="40" fill="{RED}"/>' + stroke("M88 88l80 80M168 88l-80 80", "#fff", 22))
    a["window-pin"] = svg('<g transform="rotate(45 128 128)">' + f'<rect x="96" y="32" width="64" height="88" rx="10" fill="{RED}"/><rect x="72" y="116" width="112" height="24" rx="10" fill="{RED}"/>' + stroke("M128 140v84", GRAY, 14) + "</g>")
    a["insert-link"] = svg(CHAIN)
    a["emblem-symbolic-link"] = svg(f'<rect x="20" y="132" width="104" height="104" rx="18" fill="#fff" stroke="{GRAY}" stroke-width="8"/>' + stroke("M48 206c4-30 22-44 52-44M80 140l22 22-22 22", BLUE, 14))
    a["system-shutdown"] = svg(POWER)
    a["system-reboot"] = svg(CIRCLE_ARROW.replace(BLUE, ORANGE))
    a["system-log-out"] = svg(f'<path d="M60 36h92v184H60z" fill="#C48A4A"/><circle cx="132" cy="132" r="9" fill="#F6D58E"/>' + stroke("M140 128h80M188 96l32 32-32 32", BLUE, 20))
    a["mail-message-new"] = svg(f'<rect x="24" y="56" width="192" height="136" rx="14" fill="#fff" stroke="{BLUE}" stroke-width="12"/>' + stroke("M30 66l90 70 90-70", BLUE, 12) + plus_badge(200, 192))
    return a


# ------------------------------------------------------------------ the rest

def everything():
    icons = {}
    places = icons.setdefault("places", {})
    places["folder"] = folder()
    places["folder-open"] = folder(open_=True)
    places["folder-documents"] = folder("documents")
    places["folder-download"] = folder("download")
    places["folder-music"] = folder("music")
    places["folder-pictures"] = folder("pictures")
    places["folder-videos"] = folder("videos")
    places["user-home"] = folder("home")
    places["user-desktop"] = folder("desktop")
    places["folder-new"] = folder(extra=plus_badge())

    bin_body = (
        '<path d="M64 76h128l-12 140a12 12 0 0 1-12 11H88a12 12 0 0 1-12-11z" fill="url(#bin)"/>'
        '<g stroke="#ffffff" stroke-opacity=".55" stroke-width="8" stroke-linecap="round"><path d="M104 100l4 104M152 100l-4 104M128 100v104"/></g>'
        '<rect x="50" y="54" width="156" height="24" rx="10" fill="#8FA3B8"/><rect x="104" y="36" width="48" height="22" rx="8" fill="#8FA3B8"/>'
    )
    bin_defs = grad("bin", "#C9D6E3", "#9DB0C4")
    places["user-trash"] = svg(bin_body, bin_defs)
    # Full: the lid's off, sitting askew, and crumpled paper pokes out of the top.
    full_body = (
        '<path d="M64 76h128l-12 140a12 12 0 0 1-12 11H88a12 12 0 0 1-12-11z" fill="url(#bin)"/>'
        '<path d="M70 84l20-44 34 18 20-30 30 28 14 30z" fill="#fff" stroke="#C9CDD3" stroke-width="4" stroke-linejoin="round"/>'
        '<path d="M110 82l18-34 40 22-12 14z" fill="#FFE7A6" stroke="#E2C27A" stroke-width="4" stroke-linejoin="round"/>'
        '<g stroke="#ffffff" stroke-opacity=".55" stroke-width="8" stroke-linecap="round"><path d="M104 104l4 100M152 104l-4 100M128 104v100"/></g>'
        '<g transform="rotate(-14 128 66)"><rect x="50" y="62" width="156" height="22" rx="10" fill="#8FA3B8"/></g>'
    )
    places["user-trash-full"] = svg(full_body, bin_defs)
    computer = (
        f'<rect x="24" y="40" width="208" height="140" rx="14" fill="{DARK}"/><rect x="36" y="52" width="184" height="116" rx="6" fill="url(#screen)"/>'
        f'<path d="M104 180h48l8 30H96z" fill="{GRAY}"/><rect x="72" y="206" width="112" height="16" rx="8" fill="{DARK}"/>'
    )
    screen = grad("screen", "#3DA5F4", "#0A5FB4")
    places["computer"] = svg(computer, screen)
    places["vide-pc"] = places["computer"]
    globe = f'<circle cx="128" cy="128" r="96" fill="url(#g)"/><g fill="none" stroke="#fff" stroke-opacity=".85" stroke-width="9"><circle cx="128" cy="128" r="96"/><ellipse cx="128" cy="128" rx="44" ry="96"/><path d="M36 100h184M36 156h184"/></g>'
    globe_defs = grad("g", "#4CB4F5", "#0A64C2")
    places["network-workgroup"] = svg(globe, globe_defs)
    places["network"] = places["network-workgroup"]

    devices = icons.setdefault("devices", {})
    devices["drive-harddisk"] = svg(
        '<rect x="24" y="84" width="208" height="96" rx="16" fill="url(#hd)"/><rect x="24" y="148" width="208" height="32" rx="0" fill="#000" opacity=".08"/>'
        '<circle cx="196" cy="156" r="9" fill="#38D16B"/><rect x="44" y="150" width="96" height="12" rx="6" fill="#7B8594"/>',
        grad("hd", "#D9DEE5", "#AEB6C1"),
    )
    devices["drive-removable-media"] = svg(
        f'<rect x="88" y="28" width="80" height="56" rx="6" fill="#B8C0CB"/><rect x="104" y="44" width="16" height="16" fill="{DARK}"/><rect x="136" y="44" width="16" height="16" fill="{DARK}"/>'
        f'<rect x="68" y="80" width="120" height="152" rx="22" fill="{BLUE}"/><rect x="100" y="120" width="56" height="10" rx="5" fill="#fff" opacity=".7"/>'
    )
    devices["computer"] = places["computer"]
    devices["input-keyboard"] = svg(
        f'<rect x="16" y="68" width="224" height="124" rx="16" fill="{DARK}"/>'
        + "".join(f'<rect x="{x}" y="{y}" width="26" height="24" rx="4" fill="#E5E7EB"/>' for y in (84, 116) for x in range(32, 224, 32))
        + '<rect x="64" y="150" width="128" height="24" rx="4" fill="#E5E7EB"/>'
    )
    devices["keyboard"] = devices["input-keyboard"]
    devices["input-mouse"] = svg(f'<rect x="72" y="32" width="112" height="192" rx="56" fill="#E5E7EB" stroke="{GRAY}" stroke-width="10"/>' + stroke("M128 36v76M76 112h104", GRAY, 8) + f'<rect x="120" y="58" width="16" height="32" rx="8" fill="{DARK}"/>')
    devices["mouse"] = devices["input-mouse"]
    devices["camera-photo"] = svg(f'<rect x="24" y="72" width="208" height="144" rx="20" fill="{DARK}"/><path d="M84 72l18-28h52l18 28z" fill="{DARK}"/><circle cx="128" cy="144" r="48" fill="#5B6B80" stroke="#E5E7EB" stroke-width="10"/><circle cx="128" cy="144" r="20" fill="{BLUE_LIGHT}"/><circle cx="196" cy="100" r="8" fill="#F6BF45"/>')
    devices["camera"] = devices["camera-photo"]
    devices["bluetooth"] = svg(f'<circle cx="128" cy="128" r="104" fill="{BLUE}"/>' + stroke("M96 92l64 64-32 30V70l32 30-64 64", "#fff", 14))

    mimes = icons.setdefault("mimetypes", {})
    mimes["text-x-generic"] = page()
    mimes["text"] = mimes["text-x-generic"]
    mimes["unknown"] = page(question(124, 140, 150, GRAY), lines=False)
    mimes["text-x-script"] = page(f'<rect x="40" y="150" width="128" height="62" rx="10" fill="{BLUE}"/>' + stroke("M78 166l-20 15 20 15M130 166l20 15-20 15M112 160l-16 42", "#fff", 9))
    mimes["application-x-shellscript"] = page(f'<rect x="40" y="148" width="128" height="64" rx="10" fill="{DARK}"/>' + stroke("M60 166l18 14-18 14M90 196h34", "#fff", 9))
    mimes["application-pdf"] = page(band(RED))
    mimes["image-x-generic"] = page(
        f'<rect x="64" y="100" width="120" height="100" rx="8" fill="#7CC4F7"/><circle cx="150" cy="128" r="14" fill="#FFD873"/><path d="M64 200l36-46 26 30 16-16 42 32v-10z" fill="{GREEN}"/>',
        lines=False,
    )
    mimes["image"] = mimes["image-x-generic"]
    mimes["video-x-generic"] = page(f'<rect x="56" y="104" width="136" height="100" rx="14" fill="{PURPLE}"/><path d="M110 128v52l42-26z" fill="#fff"/>', lines=False)
    mimes["video"] = mimes["video-x-generic"]
    mimes["audio-x-generic"] = page(f'<circle cx="124" cy="154" r="54" fill="{ORANGE}"/><path d="M110 122l38-8v52a12 10 0 1 1-8-9.4V130l-22 5v40a12 10 0 1 1-8-9.4z" fill="#fff"/>', lines=False)
    mimes["audio"] = mimes["audio-x-generic"]
    mimes["package-x-generic"] = svg(
        '<path d="M32 88l96-48 96 48v104l-96 44-96-44z" fill="#C9944F"/><path d="M32 88l96 44 96-44" fill="none" stroke="#A9783C" stroke-width="6"/><path d="M128 132v104" stroke="#A9783C" stroke-width="6"/>'
        '<path d="M80 64l96 46v38l-20 10v-38L60 74z" fill="#F0D9A8"/>'
    )
    mimes["application-x-executable"] = svg(
        f'<rect x="28" y="40" width="200" height="168" rx="16" fill="#fff" stroke="{BLUE}" stroke-width="12"/><path d="M34 80h188" stroke="{BLUE}" stroke-width="28"/>'
        f'<g transform="translate(70 92) scale(.46)">{gear(BLUE, "#fff")}</g>'
    )

    apps = icons.setdefault("apps", {})
    apps["preferences-system"] = tile("#7D8DA3", "#4F5D70", gear())
    apps["systemsettings"] = apps["preferences-system"]
    apps["vide-settings"] = apps["preferences-system"]
    apps["preferences-system-services"] = tile("#5B6B80", "#363F4C", f'<g transform="translate(14 14) scale(.62)">{gear("#fff", "#46505E")}</g><g transform="translate(112 112) scale(.4)">{gear("#9CC9F5", "#46505E")}</g>')
    apps["system-run"] = tile("#36A16A", "#1C7A48", f'<path d="M100 76v104l84-52z" fill="#fff"/>')
    apps["utilities-terminal"] = tile("#1D4E9E", "#0B2A5E", stroke("M72 92l44 36-44 36M128 168h56", "#fff", 18))
    apps["powerslop"] = apps["utilities-terminal"]
    apps["utilities-system-monitor"] = tile("#2BB673", "#178A52", f'<rect x="64" y="64" width="128" height="128" rx="10" fill="#fff" opacity=".18"/>' + stroke("M60 156h36l20-56 28 88 20-44h32", "#fff", 14))
    apps["task-mangler"] = apps["utilities-system-monitor"]
    apps["applets-screenshooter"] = tile("#E86AA0", "#B83B78", f'<rect x="62" y="70" width="132" height="104" rx="6" fill="none" stroke="#fff" stroke-width="10" stroke-dasharray="22 14"/><g transform="translate(96 96) scale(.5)">{PENCIL}</g>')
    apps["snip-and-sketchy"] = apps["applets-screenshooter"]
    apps["face-smile"] = svg('<circle cx="128" cy="128" r="104" fill="#FFCC4D"/><circle cx="94" cy="108" r="14" fill="#5A3E1B"/><circle cx="162" cy="108" r="14" fill="#5A3E1B"/>' + stroke("M78 150c26 36 74 36 100 0", "#5A3E1B", 14))
    apps["emoji-panic"] = apps["face-smile"]
    apps["system-file-manager"] = folder(extra=f'<rect x="24" y="168" width="208" height="18" fill="{BLUE}"/>')
    apps["file-exploder"] = apps["system-file-manager"]
    apps["system-software-install"] = tile("#2F8CE0", "#145FAF", '<path d="M76 100h104l-8 96a10 10 0 0 1-10 9h-68a10 10 0 0 1-10-9z" fill="#fff"/>' + stroke("M100 104V86a28 28 0 0 1 56 0v18", "#fff", 12))
    apps["window-shopping"] = apps["system-software-install"]
    apps["accessories-calculator"] = tile("#4A5260", "#2A2F38", f'<rect x="64" y="56" width="128" height="40" rx="8" fill="#D7F0E0"/>' + "".join(f'<rect x="{x}" y="{y}" width="32" height="28" rx="6" fill="{c}"/>' for (x, y, c) in [(64, 108, "#8C96A6"), (112, 108, "#8C96A6"), (160, 108, ORANGE), (64, 148, "#8C96A6"), (112, 148, "#8C96A6"), (160, 148, ORANGE)]))
    apps["internet-web-browser"] = svg(globe, globe_defs)
    apps["help-contents"] = svg(f'<circle cx="128" cy="128" r="104" fill="{BLUE}"/>' + question(128, 118, 150, "#fff"))
    apps["user-identity"] = svg(f'<circle cx="128" cy="128" r="108" fill="#DCE6F2"/><clipPath id="c"><circle cx="128" cy="128" r="104"/></clipPath><g clip-path="url(#c)">{person("#6E8BB0")}</g>')
    apps["user"] = apps["user-identity"]
    apps["preferences-desktop-remote-desktop"] = svg(computer + stroke("M84 110h88M148 88l24 22-24 22", "#fff", 12), screen)
    apps["games"] = tile("#7C5CE0", "#5236B8", '<path d="M70 104h116a30 30 0 0 1 29 37l-10 40a20 20 0 0 1-34 8l-18-21h-50l-18 21a20 20 0 0 1-34-8l-10-40a30 30 0 0 1 29-37z" fill="#fff"/>' + stroke("M88 128v28M74 142h28", "#7C5CE0", 10) + '<circle cx="164" cy="134" r="8" fill="#7C5CE0"/><circle cx="182" cy="152" r="8" fill="#7C5CE0"/>')
    apps["applications-games"] = apps["games"]

    icons["actions"] = actions()
    return icons


INDEX_THEME = """[Icon Theme]
Name=VIDE
Comment=VIDE-DE's own icons: folders, file types, our apps and toolbar glyphs
Inherits=breeze,hicolor
Directories=scalable/places,scalable/devices,scalable/mimetypes,scalable/apps,scalable/actions

[scalable/places]
Size=256
MinSize=8
MaxSize=512
Type=Scalable
Context=Places

[scalable/devices]
Size=256
MinSize=8
MaxSize=512
Type=Scalable
Context=Devices

[scalable/mimetypes]
Size=256
MinSize=8
MaxSize=512
Type=Scalable
Context=MimeTypes

[scalable/apps]
Size=256
MinSize=8
MaxSize=512
Type=Scalable
Context=Applications

[scalable/actions]
Size=256
MinSize=8
MaxSize=512
Type=Scalable
Context=Actions
"""


def main():
    if os.path.isdir(ROOT):
        shutil.rmtree(ROOT)
    n = 0
    for ctx, icons in everything().items():
        d = os.path.join(ROOT, "scalable", ctx)
        os.makedirs(d, exist_ok=True)
        for name, body in sorted(icons.items()):
            with open(os.path.join(d, f"{name}.svg"), "w") as f:
                f.write(body)
            n += 1
    with open(os.path.join(ROOT, "index.theme"), "w") as f:
        f.write(INDEX_THEME)
    # The compositor carries the pack inside itself and installs it at login (apps.rs).
    files = ["index.theme"] + sorted(
        os.path.relpath(os.path.join(dp, fn), ROOT) for dp, _, fns in os.walk(os.path.join(ROOT, "scalable")) for fn in fns
    )
    rs = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "icon_pack.rs")
    with open(rs, "w") as f:
        f.write("//! The `vide` icon theme, built into the compositor. Generated by scripts/make-icons.py; don't edit.\n\n")
        f.write("pub const FILES: &[(&str, &[u8])] = &[\n")
        for rel in files:
            f.write(f'    ("{rel}", include_bytes!("../assets/icons/vide/{rel}")),\n')
        f.write("];\n")
    print(f"wrote {n} icons to {os.path.normpath(ROOT)}")


if __name__ == "__main__":
    main()
