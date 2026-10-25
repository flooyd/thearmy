"""Turns the drawings into the code the game draws them with: src/creature_art.rs, which holds a table of them and the GLSL of the shapes.

    python art/shader.py                writes src/creature_art.rs
    python art/shader.py --check        says whether it is up to date (and writes nothing)

Each drawing is a tree of nodes (see dsl.py). Here every node becomes a few lines of GLSL that work out how far the pixel is from the shape (a signed distance,
negative inside), and paint it over what is there so far: the parts are painted back to front, each with its black outline under it, as the SVG paints them.
A drawing is one function `art_N(p, aa, B, hurt, T)`: p the point in art units (the drawing faces up, and the middle of the hitbox is 0, 0), aa how many art
units are in a pixel (the soft edge of everything is that wide), B the colour of the creature, hurt 1 for the look of a boss in its last stand, T the time
in seconds (for what spins and pulses). It gives the colour of the pixel with its alpha already multiplied in.
"""

import math
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import bosses  # noqa: E402,F401
import enemies  # noqa: E402,F401
import marks  # noqa: E402,F401
from dsl import COLORS, REGISTRY  # noqa: E402

OUT = os.path.join(HERE, '..', 'src', 'creature_art.rs')
SKIP = ('ship', 'player')
OUTLINE = {'o': 2.2, 'ot': 1.4, 'oh': 3.2, '': 0.0}   #how far the black goes out from a shape: half the width of the stroke the SVG draws under it
BLACK = '#06090d'


def f(x):
    """A number as GLSL has it: always with a point or an exponent."""
    s = '%.6g' % x
    if s in ('-0', '0'):
        return '0.0'
    return s if ('.' in s or 'e' in s) else s + '.0'


def v2(x, y):
    return 'vec2(%s, %s)' % (f(x), f(y))


def rgb(hexcolor):
    return 'vec3(%s, %s, %s)' % tuple(f(int(hexcolor[i:i + 2], 16) / 255.0) for i in (1, 3, 5))


def colour(cls):
    """The GLSL for a class of colour: the creature's own, and the shades of it (the same as the SVG makes with color-mix), or one of the fixed ones."""
    return {'b': 'cb', 'd': 'cd', 'l': 'cl', 'x': 'cx'}.get(cls) or rgb(COLORS[cls])


HELPERS = '''
const vec3 INK = %s;

//the share of a pixel a shape covers, from its distance (negative inside) and how wide a pixel is
float cover(float d, float aa) {
    return clamp(0.5 - d / aa, 0.0, 1.0);
}

//the colour over what is there, both with their alpha multiplied in
void fill(inout vec4 c, float d, float aa, vec3 col) {
    float a = cover(d, aa);
    c = vec4(col * a, a) + c * (1.0 - a);
}

//the same with the black outline under it, `ow` wide outside the shape
void paint(inout vec4 c, float d, float ow, float aa, vec3 col) {
    float o = cover(d - ow, aa);
    c = vec4(INK * o, o) + c * (1.0 - o);
    fill(c, d, aa, col);
}

//the distance to a polygon, one edge at a time: d is the square of the distance so far, s the side of the line it is on (the same as the sign of the distance)
void pe(inout float d, inout float s, vec2 p, vec2 a, vec2 b, float inv) {
    vec2 e = b - a;
    vec2 w = p - a;
    vec2 q = w - e * clamp(dot(w, e) * inv, 0.0, 1.0);
    d = min(d, dot(q, q));
    bvec3 k = bvec3(p.y >= a.y, p.y < b.y, e.x * w.y > e.y * w.x);
    if (all(k) || all(not(k))) {
        s = -s;
    }
}

//an edge that is only a line the polygon is mirrored in: it is not a way out of the shape, but it is a side of it
void ps(inout float s, vec2 p, vec2 a, vec2 b) {
    vec2 e = b - a;
    vec2 w = p - a;
    bvec3 k = bvec3(p.y >= a.y, p.y < b.y, e.x * w.y > e.y * w.x);
    if (all(k) || all(not(k))) {
        s = -s;
    }
}

//the distance to a segment, from its start and its length and direction (inv is 1 over the length squared)
float sg(vec2 p, vec2 a, vec2 ba, float inv) {
    vec2 pa = p - a;
    return length(pa - ba * clamp(dot(pa, ba) * inv, 0.0, 1.0));
}

//an ellipse, near enough: exact on its axes and within a few per cent anywhere
float el(vec2 p, vec2 r) {
    float k0 = length(p / r);
    float k1 = length(p / (r * r));
    return k1 > 0.0001 ? k0 * (k0 - 1.0) / k1 : -min(r.x, r.y);
}

//an arc with round ends: p in a frame where the middle of the arc is straight up, sc the sine and cosine of half of how wide it is, r its radius
float ar(vec2 p, vec2 sc, float r) {
    p.x = abs(p.x);
    return sc.y * p.x > sc.x * p.y ? length(p - sc * r) : abs(length(p) - r);
}

//a soft light: the share of the colour that is left this far out, as the SVG's gradient has it (0.95 in the middle, 0.35 at 45%%, nothing at the edge)
void lamp(inout vec4 c, vec2 p, float r, vec3 col, float strength) {
    float t = length(p) / r;
    if (t < 1.0) {
        float a = (t < 0.45 ? mix(0.95, 0.35, t / 0.45) : mix(0.35, 0.0, (t - 0.45) / 0.55)) * strength;
        c = vec4(col * a, a) + c * (1.0 - a);
    }
}
''' % rgb(BLACK)


class Gen:
    def __init__(self):
        self.out = []
        self.n = 0

    def var(self, base):
        self.n += 1
        return '%s%d' % (base, self.n)

    def line(self, indent, text):
        self.out.append('    ' * indent + text)

    def nodes(self, nodes, p, c, ind):
        for node in nodes:
            getattr(self, 'do_' + node[0])(node, p, c, ind)

    # ---- the shapes

    def painted(self, c, ind, d, ow, col):
        if ow:
            self.line(ind, 'paint(%s, %s, %s, aa, %s);' % (c, d, f(ow), col))
        else:
            self.line(ind, 'fill(%s, %s, aa, %s);' % (c, d, col))

    def reach(self, p, pts, extra, ind):
        """The opening of an `if` that skips a shape whose bounding circle is nowhere near this pixel, and the number of braces to close."""
        xs, ys = [q[0] for q in pts], [q[1] for q in pts]
        cx, cy = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
        r = max(math.hypot(x - cx, y - cy) for x, y in pts) + extra
        self.line(ind, 'if (length(%s - %s) < %s + 2.0 * aa) {' % (p, v2(cx, cy), f(r + 1.0)))

    def do_poly(self, node, p, c, ind):
        _, cls, o, pts, mirror = node
        pts = [tuple(q) for q in pts]
        ow = OUTLINE[o or '']
        axis_closed = False
        if mirror == 'axis':
            if pts[0][0] == 0 and pts[-1][0] == 0 and all(x >= 0 for x, _ in pts):
                polys, fold, axis_closed = [pts], True, True
            else:
                full = pts + [(-x, y) for x, y in (pts[-2:0:-1] if pts[0][0] == 0 and pts[-1][0] == 0 else pts[::-1])]
                polys, fold = [full], False
        elif mirror == 'pair':
            if all(x >= 0 for x, _ in pts):
                polys, fold = [pts], True
            else:
                polys, fold = [pts, [(-x, y) for x, y in pts]], False
        else:
            polys, fold = [pts], False
        self.line(ind, '{')
        q = self.var('q')
        self.line(ind + 1, 'vec2 %s = %s;' % (q, 'vec2(abs(%s.x), %s.y)' % (p, p) if fold else p))
        allpts = [pt for poly in polys for pt in poly]
        self.reach(q, allpts, ow, ind + 1)
        self.line(ind + 2, 'float d = 1.0e9;')
        ds = []
        for k, poly in enumerate(polys):
            s, dd = 's%d' % k, 'd%d' % k
            self.line(ind + 2, 'float %s = 1.0; float %s = dot(%s - %s, %s - %s);' % (s, dd, q, v2(*poly[0]), q, v2(*poly[0])))
            for i in range(len(poly)):
                a, b = poly[i], poly[i - 1]
                if a == b:
                    continue
                if axis_closed and i == 0:
                    self.line(ind + 2, 'ps(%s, %s, %s, %s);' % (s, q, v2(*a), v2(*b)))
                else:
                    e = (b[0] - a[0], b[1] - a[1])
                    self.line(ind + 2, 'pe(%s, %s, %s, %s, %s, %s);' % (dd, s, q, v2(*a), v2(*b), f(1.0 / (e[0] ** 2 + e[1] ** 2))))
            ds.append('%s * sqrt(%s)' % (s, dd))
        self.line(ind + 2, 'd = %s;' % (ds[0] if len(ds) == 1 else 'min(%s, %s)' % tuple(ds)))
        self.painted(c, ind + 2, 'd', ow, colour(cls))
        self.line(ind + 1, '}')
        self.line(ind, '}')

    def do_circ(self, node, p, c, ind):
        _, cls, o, x, y, r, mirror = node
        q = 'vec2(abs(%s.x), %s.y)' % (p, p) if mirror and x >= 0 else p
        d = 'length(%s - %s) - %s' % (q, v2(x, y), f(r))
        if mirror and x < 0:
            d = 'min(length(%s - %s), length(%s - %s)) - %s' % (p, v2(x, y), p, v2(-x, y), f(r))
        self.painted_expr(c, ind, d, OUTLINE[o or ''], colour(cls))

    def painted_expr(self, c, ind, d, ow, col):
        self.line(ind, '{')
        self.line(ind + 1, 'float d = %s;' % d)
        self.painted(c, ind + 1, 'd', ow, col)
        self.line(ind, '}')

    def ellipse_distance(self, p, x, y, rx, ry, rot):
        """The GLSL for the distance to an ellipse centred at x, y, turned `rot` degrees (clockwise, as the SVG turns it)."""
        a = math.radians(rot)
        co, si = math.cos(a), math.sin(a)
        if rot:
            q = 'vec2(dot(%s - %s, %s), dot(%s - %s, %s))' % (p, v2(x, y), v2(co, si), p, v2(x, y), v2(-si, co))
        else:
            q = '%s - %s' % (p, v2(x, y))
        return 'el(%s, %s)' % (q, v2(rx, ry))

    def do_ell(self, node, p, c, ind):
        _, cls, o, x, y, rx, ry, rot, mirror = node
        if mirror:
            # (the mirror image is turned the other way)
            d = 'min(%s, %s)' % (self.ellipse_distance(p, x, y, rx, ry, rot), self.ellipse_distance(p, -x, y, rx, ry, -rot))
        else:
            d = self.ellipse_distance(p, x, y, rx, ry, rot)
        self.painted_expr(c, ind, d, OUTLINE[o or ''], colour(cls))

    def do_line(self, node, p, c, ind):
        _, cls, pts, w, mirror, edge = node
        pts = [tuple(q) for q in pts]
        fold = mirror and all(x >= 0 for x, _ in pts)
        sides = [pts] if (not mirror or fold) else [pts, [(-x, y) for x, y in pts]]
        self.line(ind, '{')
        q = self.var('q')
        self.line(ind + 1, 'vec2 %s = %s;' % (q, 'vec2(abs(%s.x), %s.y)' % (p, p) if fold else p))
        self.reach(q, [pt for side in sides for pt in side], w / 2 + edge, ind + 1)
        terms = []
        for side in sides:
            for a, b in zip(side, side[1:]):
                ba = (b[0] - a[0], b[1] - a[1])
                if ba == (0, 0):
                    continue
                terms.append('sg(%s, %s, %s, %s)' % (q, v2(*a), v2(*ba), f(1.0 / (ba[0] ** 2 + ba[1] ** 2))))
        self.line(ind + 2, 'float d = %s - %s;' % (self.fold_min(terms), f(w / 2)))
        self.painted(c, ind + 2, 'd', edge, colour(cls))
        self.line(ind + 1, '}')
        self.line(ind, '}')

    def fold_min(self, terms):
        out = terms[0]
        for t in terms[1:]:
            out = 'min(%s, %s)' % (out, t)
        return out

    def do_ring(self, node, p, c, ind):
        _, cls, x, y, r, w, edge = node
        self.painted_expr(c, ind, 'abs(length(%s - %s) - %s) - %s' % (p, v2(x, y), f(r), f(w / 2)), edge, colour(cls))

    def do_arc(self, node, p, c, ind):
        _, cls, x, y, r, a0, a1, w, edge = node
        span = (a1 - a0) % 360
        mid = math.radians(a0 + span / 2)
        half = math.radians(span / 2)
        # the arc's own frame: its middle is straight up (+y), so the point is turned by -mid (0 is up on the drawing, clockwise)
        along = (math.sin(mid), -math.cos(mid))      # where the middle of the arc is, from its centre
        across = (math.cos(mid), math.sin(mid))      # a quarter turn on from that
        self.line(ind, '{')
        q = self.var('q')
        self.line(ind + 1, 'vec2 %s = %s - %s;' % (q, p, v2(x, y)))
        self.line(ind + 1, 'if (length(%s) < %s + 2.0 * aa) {' % (q, f(r + w / 2 + edge + 1.0)))
        self.line(ind + 2, 'float d = ar(vec2(dot(%s, %s), dot(%s, %s)), %s, %s) - %s;' % (q, v2(*across), q, v2(*along), v2(math.sin(half), math.cos(half)), f(r), f(w / 2)))
        self.painted(c, ind + 2, 'd', edge, colour(cls))
        self.line(ind + 1, '}')
        self.line(ind, '}')

    def do_glow(self, node, p, c, ind):
        _, color, x, y, r, a = node
        self.line(ind, 'lamp(%s, %s - %s, %s, %s, %s);' % (c, p, v2(x, y), f(r), rgb(color), f(a)))

    # ---- the groups

    def turned(self, p, ind, degrees, about=(0.0, 0.0)):
        """A new point: p, as it is when what is drawn in it has been turned `degrees` about a point (the drawn thing turns by +, so the point goes the other way)."""
        a = math.radians(degrees)
        co, si = math.cos(a), math.sin(a)
        q = self.var('p')
        self.line(ind, 'vec2 %s = vec2(dot(%s - %s, %s), dot(%s - %s, %s)) + %s;' % (q, p, v2(*about), v2(co, si), p, v2(*about), v2(-si, co), v2(*about)))
        return q

    def do_rot(self, node, p, c, ind):
        _, degrees, kids = node
        self.line(ind, '{')
        q = self.turned(p, ind + 1, degrees)
        self.nodes(kids, q, c, ind + 1)
        self.line(ind, '}')

    def do_place(self, node, p, c, ind):
        _, x, y, rot, kids = node
        # what is drawn in it is turned `rot` degrees about its own middle and put at (x, y): so the point is measured from (x, y) and turned back
        a = math.radians(rot)
        co, si = math.cos(a), math.sin(a)
        q = self.var('p')
        self.line(ind, '{')
        self.line(ind + 1, 'vec2 %s = vec2(dot(%s - %s, %s), dot(%s - %s, %s));' % (q, p, v2(x, y), v2(co, si), p, v2(x, y), v2(-si, co)))
        self.nodes(kids, q, c, ind + 1)
        self.line(ind, '}')

    def do_spin(self, node, p, c, ind):
        _, seconds, cx, cy, back, kids = node
        a = self.var('a')
        self.line(ind, '{')
        self.line(ind + 1, 'float %s = %s * T * %s;' % (a, f(-6.2831853 / seconds if back else 6.2831853 / seconds), '1.0'))
        q = self.var('p')
        self.line(ind + 1, 'vec2 %s = vec2(dot(%s - %s, vec2(cos(%s), sin(%s))), dot(%s - %s, vec2(-sin(%s), cos(%s)))) + %s;' % (q, p, v2(cx, cy), a, a, p, v2(cx, cy), a, a, v2(cx, cy)))
        self.nodes(kids, q, c, ind + 1)
        self.line(ind, '}')

    def group(self, kids, p, c, ind, opacity):
        g = self.var('g')
        self.line(ind, '{')
        self.line(ind + 1, 'vec4 %s = vec4(0.0);' % g)
        self.nodes(kids, p, g, ind + 1)
        self.line(ind + 1, 'float k = %s;' % opacity)
        self.line(ind + 1, '%s = %s * k + %s * (1.0 - %s.a * k);' % (c, g, c, g))
        self.line(ind, '}')

    def do_pulse(self, node, p, c, ind):
        _, seconds, low, kids = node
        self.group(kids, p, c, ind, 'mix(%s, 1.0, abs(2.0 * fract(T / %s) - 1.0))' % (f(low), f(seconds)))

    def do_alpha(self, node, p, c, ind):
        _, a, kids = node
        self.group(kids, p, c, ind, f(a))

    def do_variant(self, node, p, c, ind):
        _, name, kids = node
        assert name == 'b', 'the shader has two looks, the usual one and the hurt one'
        self.line(ind, 'if (hurt > 0.5) {')
        self.nodes(kids, p, c, ind + 1)
        self.line(ind, '}')


def function(sym):
    gen = Gen()
    gen.line(0, '//%s' % sym.name)
    gen.line(0, 'vec4 art(vec2 p, float aa, vec3 B, float hurt, float T) {')
    gen.line(1, 'vec4 c = vec4(0.0);')
    gen.line(1, 'vec3 cb = B;')
    gen.line(1, 'vec3 cd = B * 0.6;')
    gen.line(1, 'vec3 cl = mix(B, vec3(1.0), 0.4);')
    gen.line(1, 'vec3 cx = B * 0.28;')
    gen.nodes(sym.tree(), 'p', 'c', 1)
    gen.line(1, 'return c;')
    gen.line(0, '}')
    return '\n'.join(gen.out)


def source():
    syms = [s for s in REGISTRY if s.id not in SKIP]
    table = ''.join('    Art {\n        name: "%s",\n        turns: %s,\n        mark: %s,\n        size: %s,\n        glsl: r#"%s"#,\n    },\n'
                    % (s.name.replace('"', '\\"'), 'true' if s.turns else 'false', 'true' if s.id.startswith('mark_') else 'false', f(s.size), function(s)) for s in syms)
    return ('//! the enemies, the bosses and the marks put on them, as the graphics card draws them. generated by art/shader.py from the drawings in art/*.py: do not edit,\n'
            '//! change the drawing and run `python art/shader.py`\n\n'
            '//what one of them is: its name in the game, whether it is drawn turned the way it is facing, whether it is a mark (drawn over a creature, not one of them), what size it\n'
            '//is drawn for, and its drawing, as a GLSL function `vec4 art(vec2 p, float aa, vec3 B, float hurt, float T)` that is put after HELPERS\n'
            'pub(crate) struct Art {\n    pub(crate) name: &\'static str,\n    pub(crate) turns: bool,\n    pub(crate) mark: bool,\n    pub(crate) size: f32,\n    pub(crate) glsl: &\'static str,\n}\n\n'
            '//what every drawing uses: how far a pixel is from each kind of shape, and how a shape is painted\n'
            'pub(crate) const HELPERS: &str = r#"%s"#;\n\n'
            '//the shape number of one is its place here, counting from 1 (0 is no shape)\n'
            'pub(crate) static ARTS: [Art; %d] = [\n%s];\n' % (HELPERS, len(syms), table))


def main():
    text = source()
    if '--check' in sys.argv:
        with open(OUT, encoding='utf-8') as fh:
            same = fh.read() == text
        print('up to date' if same else 'out of date')
        sys.exit(0 if same else 1)
    with open(OUT, 'w', encoding='utf-8', newline='\n') as fh:
        fh.write(text)
    print('%d drawings, %d KB' % (len([s for s in REGISTRY if s.id not in SKIP]), len(text) // 1024))


if __name__ == '__main__':
    main()
