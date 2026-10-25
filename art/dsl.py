"""A tiny drawing kit for the enemy and boss art.

Every call makes one SVG element out of the few shapes a signed-distance shader can also draw (a polygon, a circle, an ellipse, a stroked line, a ring, a
glow), so the same drawing can later be pasted into the instanced shader (see README.md). Everything is drawn in art units on a square that reaches 50 from the
middle (which is the enemy's hitbox: `size` pixels along each side), facing up (-y), and may stick out of it a little (a horn, a spike) up to 62.

Colours are classes, so one drawing is any colour: `b` is the colour of the enemy (what the instance's tint is in the game), `d` and `l` are that darker and
lighter, `x` is nearly black, and the others are the fixed colours of eyes, steel, bone and so on. A part is drawn with a black outline round the outside of it,
as the ships and the stick figure are.
"""

import math


def n(x):
    s = '%.1f' % x
    if s.endswith('.0'):
        s = s[:-2]
    return '0' if s == '-0' else s


def pts_d(pts, close=True):
    d = 'M' + ' '.join('%s %s' % (n(x), n(y)) for x, y in pts[:1])
    d += 'L' + ' '.join('%s %s' % (n(x), n(y)) for x, y in pts[1:])
    return d + ('Z' if close else '')


def flip(pts):
    return [(-x, y) for x, y in pts]


#the fixed colours (the first four are what the colour of an enemy is made into)
COLORS = {
    'b': 'var(--c)',
    'd': 'var(--cd)',
    'l': 'var(--cl)',
    'x': 'var(--cx)',
    'k': '#06090d',
    'm': '#8a97aa',    #steel
    'md': '#566174',
    'ml': '#cfd8e6',
    'bone': '#eadfc2',
    'bd': '#b9a982',
    'ey': '#ffe45e',   #eyes and lights
    'er': '#ff4a3a',
    'ec': '#69f0ff',
    'eg': '#8dff6a',
    'eo': '#ffa83a',
    'ep': '#d9a6ff',
    'ek': '#ff8ac4',
    'w': '#f4f7ff',
    'g': '#f2c14e',    #gold
    'gd': '#b8872a',
}

STYLE = """
.u{--cd:color-mix(in srgb,var(--c) 60%,#000);--cl:color-mix(in srgb,var(--c) 60%,#fff);--cx:color-mix(in srgb,var(--c) 28%,#000)}
.o{stroke:#06090d;stroke-width:4.4;stroke-linejoin:round;paint-order:stroke}
.ot{stroke:#06090d;stroke-width:2.8;stroke-linejoin:round;paint-order:stroke}
.oh{stroke:#06090d;stroke-width:6.4;stroke-linejoin:round;paint-order:stroke}
.sl{fill:none;stroke-linecap:round;stroke-linejoin:round}
.v-a{display:var(--v-a,inline)}.v-b{display:var(--v-b,none)}.v-c{display:var(--v-c,none)}
""" + '\n'.join('.%s{fill:%s}.s-%s{stroke:%s}' % (k, v, k, v) for k, v in COLORS.items())


class Sym:
    """One creature: the SVG elements of it, and the same drawing as a tree of nodes (see shader.py), built by the calls below."""

    def __init__(self, id, name, role, size, color, note='', turns=True):
        self.id, self.name, self.role, self.size, self.color, self.note, self.turns = id, name, role, size, color, note, turns
        self.stack = [[]]
        self.nodes = [[]]

    def add(self, s, node=None):
        self.stack[-1].append(s)
        if node:
            self.nodes[-1].append(node)

    def capture(self, fn):
        """What fn() draws, as (svg elements, nodes)."""
        self.stack.append([])
        self.nodes.append([])
        fn()
        return self.stack.pop(), self.nodes.pop()

    def svg(self):
        return ''.join(self.stack[0])

    def tree(self):
        return self.nodes[0]

    #--- shapes
    def poly(self, pts, cls='b', mirror=None, o='o'):
        """A closed polygon. mirror='axis': the points run from the axis (x = 0) down the right side to the axis again, and the left side is made from them.
        mirror='pair': a second one at the other side."""
        c = cls + (' ' + o if o else '')
        node = ('poly', cls, o, list(pts), mirror)
        if mirror == 'axis':
            if pts[0][0] == 0 and pts[-1][0] == 0:
                pts = pts + flip(pts[-2:0:-1])
            else:
                pts = pts + flip(pts[::-1])
            self.add('<path class="%s" d="%s"/>' % (c, pts_d(pts)), node)
        else:
            self.add('<path class="%s" d="%s"/>' % (c, pts_d(pts)), node)
            if mirror == 'pair':
                self.add('<path class="%s" d="%s"/>' % (c, pts_d(flip(pts))))

    def circ(self, x, y, r, cls='b', mirror=False, o='o'):
        c = cls + (' ' + o if o else '')
        self.add('<circle class="%s" cx="%s" cy="%s" r="%s"/>' % (c, n(x), n(y), n(r)), ('circ', cls, o, x, y, r, mirror))
        if mirror:
            self.add('<circle class="%s" cx="%s" cy="%s" r="%s"/>' % (c, n(-x), n(y), n(r)))

    def ell(self, x, y, rx, ry, cls='b', rot=0, mirror=False, o='o'):
        c = cls + (' ' + o if o else '')
        t = ' transform="rotate(%s %s %s)"' % (n(rot), n(x), n(y)) if rot else ''
        self.add('<ellipse class="%s" cx="%s" cy="%s" rx="%s" ry="%s"%s/>' % (c, n(x), n(y), n(rx), n(ry), t), ('ell', cls, o, x, y, rx, ry, rot, mirror))
        if mirror:
            t = ' transform="rotate(%s %s %s)"' % (n(-rot), n(-x), n(y)) if rot else ''
            self.add('<ellipse class="%s" cx="%s" cy="%s" rx="%s" ry="%s"%s/>' % (c, n(-x), n(y), n(rx), n(ry), t))

    def line(self, pts, w, cls='d', mirror=False, edge=2.2):
        """A line through the points with round ends and joints, `w` wide, with its black edge `edge` wide outside it."""
        node = ('line', cls, list(pts), w, mirror, edge)
        for k, p in enumerate([pts, flip(pts)] if mirror else [pts]):
            d = pts_d(p, close=False)
            if edge:
                self.add('<path class="sl s-k" stroke-width="%s" d="%s"/>' % (n(w + 2 * edge), d), node if k == 0 else None)
                self.add('<path class="sl s-%s" stroke-width="%s" d="%s"/>' % (cls, n(w), d))
            else:
                self.add('<path class="sl s-%s" stroke-width="%s" d="%s"/>' % (cls, n(w), d), node if k == 0 else None)

    def ring(self, x, y, r, w, cls='d', edge=2.0):
        node = ('ring', cls, x, y, r, w, edge)
        if edge:
            self.add('<circle class="sl s-k" cx="%s" cy="%s" r="%s" stroke-width="%s"/>' % (n(x), n(y), n(r), n(w + 2 * edge)), node)
            self.add('<circle class="sl s-%s" cx="%s" cy="%s" r="%s" stroke-width="%s"/>' % (cls, n(x), n(y), n(r), n(w)))
        else:
            self.add('<circle class="sl s-%s" cx="%s" cy="%s" r="%s" stroke-width="%s"/>' % (cls, n(x), n(y), n(r), n(w)), node)

    def arc(self, x, y, r, a0, a1, w, cls='d', edge=2.0):
        """Part of a ring, from a0 to a1 degrees (0 is up, clockwise)."""
        def at(a):
            t = math.radians(a - 90)
            return x + r * math.cos(t), y + r * math.sin(t)
        (sx, sy), (ex, ey) = at(a0), at(a1)
        d = 'M%s %sA%s %s 0 %d 1 %s %s' % (n(sx), n(sy), n(r), n(r), 1 if (a1 - a0) % 360 > 180 else 0, n(ex), n(ey))
        node = ('arc', cls, x, y, r, a0, a1, w, edge)
        if edge:
            self.add('<path class="sl s-k" stroke-width="%s" d="%s"/>' % (n(w + 2 * edge), d), node)
            self.add('<path class="sl s-%s" stroke-width="%s" d="%s"/>' % (cls, n(w), d))
        else:
            self.add('<path class="sl s-%s" stroke-width="%s" d="%s"/>' % (cls, n(w), d), node)

    def glow(self, x, y, r, color='#ffd27a', a=1.0):
        """A soft light: a disc that fades to nothing at its edge (a radial gradient of this colour, which the file's <defs> has one of for each colour used)."""
        GLOWS.add(color)
        self.add('<circle cx="%s" cy="%s" r="%s" fill="url(#glow-%s)" opacity="%s"/>' % (n(x), n(y), n(r), color[1:], n(a)), ('glow', color, x, y, r, a))

    def raw(self, s):
        self.add(s)

    #--- groups
    def rep(self, count, fn, start=0.0, step=None):
        """fn() `count` times, each turned another step (the turn all the way round, divided by count, unless step is given) round the middle."""
        step = 360.0 / count if step is None else step
        for i in range(count):
            svg, nodes = self.capture(fn)
            self.add('<g transform="rotate(%s)">%s</g>' % (n(start + step * i), ''.join(svg)), ('rot', start + step * i, nodes))

    def place(self, x, y, fn, rot=0, scale=1):
        """fn() moved to (x, y), and turned `rot` degrees about there and made `scale` times as big."""
        assert scale == 1, 'the shader moves points, and a scale would have to scale the distances too'
        svg, nodes = self.capture(fn)
        self.add('<g transform="translate(%s %s) rotate(%s) scale(%s)">%s</g>' % (n(x), n(y), n(rot), n(scale), ''.join(svg)), ('place', x, y, rot, nodes))

    def spin(self, fn, seconds, cx=0, cy=0, back=False):
        svg, nodes = self.capture(fn)
        a, b = (360, 0) if back else (0, 360)
        self.add('<g><animateTransform attributeName="transform" type="rotate" from="%d %s %s" to="%d %s %s" dur="%ss" repeatCount="indefinite"/>%s</g>'
                 % (a, n(cx), n(cy), b, n(cx), n(cy), n(seconds), ''.join(svg)), ('spin', seconds, cx, cy, back, nodes))

    def pulse(self, fn, seconds=2.0, low=0.45):
        svg, nodes = self.capture(fn)
        self.add('<g><animate attributeName="opacity" values="1;%s;1" dur="%ss" repeatCount="indefinite"/>%s</g>' % (n(low), n(seconds), ''.join(svg)), ('pulse', seconds, low, nodes))

    def variant(self, name, fn):
        svg, nodes = self.capture(fn)
        self.add('<g class="v-%s">%s</g>' % (name, ''.join(svg)), ('variant', name, nodes))

    def alpha(self, a, fn):
        svg, nodes = self.capture(fn)
        self.add('<g opacity="%s">%s</g>' % (n(a), ''.join(svg)), ('alpha', a, nodes))


REGISTRY = []
GLOWS = set()   #the colours of every glow drawn, for the gradients


def sym(id, name, role, size, color, note='', turns=True):
    """Register the drawing function below it as a creature. `turns`: whether it is drawn turned the way it is facing (not for the ones that are round, or never move)."""
    def wrap(fn):
        s = Sym(id, name, role, size, color, note, turns)
        fn(s)
        REGISTRY.append(s)
        return fn
    return wrap


def mix(hexcolor, other, share):
    """The colour `share` of the way from hexcolor to other (both as #rrggbb), as #rrggbb: what the page's color-mix works out, for the files that stand alone."""
    a = [int(hexcolor[i:i + 2], 16) for i in (1, 3, 5)]
    b = [int(other[i:i + 2], 16) for i in (1, 3, 5)]
    return '#%02x%02x%02x' % tuple(round(a[i] + (b[i] - a[i]) * share) for i in range(3))
