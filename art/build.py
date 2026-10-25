"""Builds the files from the drawings: sprites.svg (every creature as a <symbol>, coloured by CSS variables), svg/<id>.svg (each on its own, in its colour in the game),
and, with --preview <folder>, a page to look at them all in (for working on the art; it is not part of the game).

    python art/build.py                    writes art/sprites.svg and art/svg/*.svg
    python art/build.py --preview <dir>    and <dir>/preview.html
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import dsl  # noqa: E402
import enemies  # noqa: E402,F401
import bosses  # noqa: E402,F401
import marks  # noqa: E402,F401
from dsl import GLOWS, REGISTRY, STYLE, mix  # noqa: E402

SPAN = 160   #art units across the whole picture: the hitbox is the middle 100, and a horn or a crown may go out to 80
VIEW = '%d %d %d %d' % (-SPAN // 2, -SPAN // 2, SPAN, SPAN)
SCALE = SPAN / 100.0
def defs():
    """A gradient for each colour a glow has been drawn in: opaque in the middle and nothing at the edge."""
    one = ('<radialGradient id="glow-%s"><stop offset="0" stop-color="#%s" stop-opacity="0.95"/><stop offset="0.45" stop-color="#%s" stop-opacity="0.35"/>'
           '<stop offset="1" stop-color="#%s" stop-opacity="0"/></radialGradient>')
    return '<defs>%s</defs>' % ''.join(one % ((c[1:],) * 4) for c in sorted(GLOWS))


def sprites_svg():
    symbols = ''.join('<symbol id="%s" viewBox="%s">%s</symbol>\n' % (s.id, VIEW, s.svg()) for s in REGISTRY)
    return '<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0" style="position:absolute"><style>%s</style>%s\n%s</svg>\n' % (STYLE, defs(), symbols)


def standalone(s, color=None):
    color = color or s.color
    style = 'svg{--c:%s;--cd:%s;--cl:%s;--cx:%s}' % (color, mix(color, '#000000', 0.4), mix(color, '#ffffff', 0.4), mix(color, '#000000', 0.72))
    # the page's color-mix classes are for the page: here the variables are worked out
    css = STYLE.split('\n', 2)[2] if STYLE.startswith('\n.u{') else STYLE
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="%s" width="248" height="248"><title>%s</title><style>%s%s</style>%s%s</svg>\n'
            % (VIEW, s.name, style, css, defs(), s.svg()))


def picture(px, color, id, hitbox=False):
    """One creature as the page draws it, `px` across the whole picture."""
    hb = '<rect class="hb" x="%d" y="%d" width="100" height="100"/>' % ((SPAN - 100) // 2, (SPAN - 100) // 2) if hitbox else ''
    return '<svg class="u" viewBox="0 0 %d %d" width="%s" height="%s" style="--c:%s"><use href="#%s" width="%d" height="%d"/>%s</svg>' % (SPAN, SPAN, px, px, color, id, SPAN, SPAN, hb)


def preview(path, only=None):
    cards = []
    for s in REGISTRY:
        if only and s.id not in only:
            continue
        px = s.size * SCALE
        cards.append('<div class="card"><div class="art">%s</div><div class="meta"><b>%s</b> <span>%s px</span></div><div class="real">%s%s</div></div>'
                     % (picture(240, s.color, s.id, True), s.name, s.size, picture(px, s.color, s.id), picture(px * 2, s.color, s.id) if px * 2 <= 180 else ''))
    html = ('<!doctype html><html><head><meta charset="utf-8"><style>body{margin:0;background:#14141a;color:#cdd;font:13px system-ui;'
            'background-image:linear-gradient(#ffffff1a 1px,transparent 1px),linear-gradient(90deg,#ffffff1a 1px,transparent 1px);background-size:64px 64px}'
            '.wrap{display:flex;flex-wrap:wrap;gap:14px;padding:14px;align-items:flex-start}.card{width:240px;background:#0a0c12cc;padding:8px;border-radius:6px}'
            '.hb{fill:none;stroke:#fff;stroke-opacity:.18;stroke-dasharray:3 3}.real{display:flex;align-items:flex-end;gap:12px;min-height:30px}'
            '.meta span{opacity:.55}</style></head><body>%s<div class="wrap">%s</div></body></html>' % (sprites_svg(), ''.join(cards)))
    with open(path, 'w', encoding='utf-8') as f:
        f.write(html)


def main():
    with open(os.path.join(HERE, 'sprites.svg'), 'w', encoding='utf-8', newline='\n') as f:
        f.write(sprites_svg())
    folder = os.path.join(HERE, 'svg')
    os.makedirs(folder, exist_ok=True)
    for s in REGISTRY:
        with open(os.path.join(folder, s.id + '.svg'), 'w', encoding='utf-8', newline='\n') as f:
            f.write(standalone(s))
    if '--preview' in sys.argv:
        out = sys.argv[sys.argv.index('--preview') + 1]
        os.makedirs(out, exist_ok=True)
        only = sys.argv[sys.argv.index('--only') + 1].split(',') if '--only' in sys.argv else None
        preview(os.path.join(out, 'preview.html'), only)
    print('%d drawings' % len(REGISTRY))


if __name__ == '__main__':
    main()
