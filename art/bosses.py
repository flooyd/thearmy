"""The seven bosses and the Sovereign, in the colours of BOSSES in src/boss.rs. Each is its own drawing, not a bigger enemy: they reach the whole of the
square of their hitbox and a good way outside it, and some have a second look (--v-b) for when they are hurt."""

import math

from dsl import sym


def hexa(a, x, y, r, cls, o='ot'):
    a.poly([(x + r * math.cos(math.radians(60 * i + 30)), y + r * math.sin(math.radians(60 * i + 30))) for i in range(6)], cls, o=o)


def spikes(count, r0, r1, half, cls='bone', start=0.0, o='ot'):
    """A crown of triangles pointing out from radius r0 to r1, `half` wide at the base each side."""
    return lambda a: a.rep(count, lambda: a.poly([(-half, -r0), (0, -r1), (half, -r0)], cls, o=o), start=start)


def spiral(a, r0, r1, sweep, w0, w1, cls='b', start=0.0, o='ot', steps=14):
    """An arm that winds from radius r0 round to r1 over `sweep` degrees, w0 wide at the start and w1 at the end."""
    left, right = [], []
    for i in range(steps + 1):
        t = i / steps
        ang = math.radians(start + sweep * t)
        r, w = r0 + (r1 - r0) * t, (w0 + (w1 - w0) * t) / 2
        for sign, side in ((-1, left), (1, right)):
            rr = r + sign * w
            side.append((rr * math.sin(ang), -rr * math.cos(ang)))
    a.poly(left + right[::-1], cls, o=o)


@sym('hive_mother', 'The Hive Mother', 'the brood: a queen with a honeycomb of eggs', 150, '#c7d933')
def _(a):
    a.alpha(0.55, lambda: a.ell(34, -6, 15, 36, 'ml', rot=28, mirror=True))
    a.alpha(0.55, lambda: a.ell(26, 12, 11, 26, 'ml', rot=62, mirror=True))
    for leg in ([(10, -8), (34, -20), (46, -6)], [(11, -2), (38, 6), (50, 24)], [(10, 6), (30, 26), (36, 46)]):
        a.line(leg, 5, 'd', mirror=True)
    a.poly([(0, 6), (22, 12), (32, 30), (24, 50), (0, 62)], 'b', mirror='axis')
    a.poly([(0, 62), (5, 70), (0, 76), (-5, 70)], 'bone', o='ot')
    for y in (14, 28, 42):
        a.line([(-27 + (y - 14) * 0.2, y), (0, y + 5), (27 - (y - 14) * 0.2, y)], 3, 'x', edge=0)
    for x, y in ((-11, 22), (0, 20), (11, 22), (-6, 34), (6, 34), (0, 46)):
        hexa(a, x, y, 5.6, 'eo')
    a.glow(0, 33, 24, '#ffa83a', 0.55)
    a.circ(0, -10, 18, 'd')
    a.poly([(0, -28), (14, -20), (14, 0), (0, 8)], 'l', mirror='axis', o='')
    a.poly([(0, -50), (13, -44), (17, -28), (0, -20)], 'b', mirror='axis')
    a.poly([(7, -24), (15, -14), (10, -10)], 'bone', mirror='pair', o='ot')
    a.circ(10, -36, 7.5, 'er', mirror=True)
    a.circ(10, -37, 3, 'k', mirror=True, o='')
    a.line([(5, -48), (14, -62), (28, -64)], 3, 'd', mirror=True)
    a.circ(29, -64, 3.2, 'ey', mirror=True, o='ot')
    a.poly([(-9, -50), (-5, -64), (0, -52), (5, -64), (9, -50)], 'g')
    a.variant('b', lambda: a.pulse(lambda: a.glow(0, -34, 40, '#ff3a3a', 0.9), 0.9, 0.3))


@sym('iron_warden', 'The Iron Warden', 'the wall: a knight in riveted plate, shield-first', 200, '#7399d9')
def _(a):
    a.poly([(30, -30), (56, -34), (62, -4), (50, 8), (32, 0)], 'm', mirror='pair', o='oh')
    a.poly([(34, -24), (52, -28), (56, -6), (48, 2), (36, -2)], 'ml', mirror='pair', o='')
    a.line([(46, 4), (48, 38)], 14, 'md', mirror=True)
    a.poly([(40, 36), (58, 36), (60, 56), (38, 56)], 'm', mirror='pair')
    a.poly([(0, -34), (30, -30), (38, 6), (30, 42), (0, 52)], 'b', mirror='axis', o='oh')
    a.poly([(0, 40), (16, 40), (16, 60), (0, 60)], 'md', mirror='axis')
    a.poly([(0, -22), (24, -18), (29, 4), (0, 10)], 'l', mirror='axis', o='')
    a.poly([(0, 10), (26, 4), (24, 36), (0, 46)], 'd', mirror='axis', o='')
    a.line([(0, -22), (0, 46)], 3, 'x', edge=0)
    a.circ(0, 4, 17, 'k')
    a.ring(0, 4, 17, 3, 'm', edge=0)
    a.glow(0, 4, 24, '#69f0ff', 0.8)
    a.circ(0, 4, 9.5, 'ec', o='')
    a.circ(0, 4, 4, 'w', o='')
    a.poly([(0, -64), (13, -58), (17, -38), (11, -26), (0, -24)], 'm', mirror='axis', o='oh')
    a.poly([(0, -57), (9, -53), (11, -40), (0, -38)], 'ml', mirror='axis', o='')
    a.poly([(-12, -46), (12, -46), (11, -41), (-11, -41)], 'k', o='')
    a.poly([(-9, -45), (9, -45), (8, -42.4), (-8, -42.4)], 'ec', o='')
    a.poly([(0, -78), (4, -62), (0, -58), (-4, -62)], 'er', o='ot')
    for x, y in ((24, -26), (34, -12), (36, 22), (24, 38), (52, -20), (8, -50)):
        a.circ(x, y, 2.4, 'ml', mirror=True, o='')
    a.variant('b', lambda: a.line([(10, -20), (18, -6), (14, 10), (24, 24)], 2.4, 'ey', mirror=True, edge=0))


@sym('warlord_grakk', 'Warlord Grakk', 'the charge: a horned warlord with a cleaver in each fist', 130, '#d94d26')
def _(a):
    a.poly([(8, -34), (20, -44), (26, -60), (38, -70), (42, -58), (36, -44), (30, -26), (14, -22)], 'bone', mirror='pair', o='oh')
    a.line([(40, -2), (52, 28)], 15, 'd', mirror=True)
    a.poly([(46, 24), (66, 32), (70, 62), (54, 66), (44, 50)], 'm', mirror='pair', o='oh')
    a.poly([(50, 32), (62, 36), (64, 56), (54, 58)], 'ml', mirror='pair', o='')
    a.poly([(0, -32), (30, -28), (40, -2), (34, 28), (22, 44), (0, 48)], 'b', mirror='axis', o='oh')
    a.circ(36, -14, 16, 'd', mirror=True)
    a.poly([(30, -26), (44, -42), (46, -26)], 'bone', mirror='pair', o='ot')
    a.poly([(46, -16), (62, -22), (52, -6)], 'bone', mirror='pair', o='ot')
    a.poly([(28, -4), (40, -2), (38, 6)], 'bone', mirror='pair', o='ot')
    a.poly([(0, -18), (22, -10), (22, 2), (0, -4)], 'k', mirror='axis', o='')
    a.poly([(0, 2), (18, 8), (18, 16), (0, 10)], 'k', mirror='axis', o='')
    a.poly([(0, 24), (22, 20), (22, 32), (0, 36)], 'gd', mirror='axis')
    a.circ(0, 28, 8, 'bone')
    a.circ(-2.6, 27, 1.9, 'k', o='')
    a.circ(2.6, 27, 1.9, 'k', o='')
    a.circ(0, -32, 17, 'd')
    a.poly([(0, -48), (9, -44), (10, -26), (0, -24)], 'l', mirror='axis', o='')
    a.poly([(-14, -38), (-3, -32), (-11, -29)], 'k', o='')
    a.poly([(14, -38), (3, -32), (11, -29)], 'k', o='')
    a.circ(8, -34, 2.6, 'ey', mirror=True, o='')
    a.poly([(6, -24), (9, -44), (12, -24)], 'bone', mirror='pair', o='ot')
    a.variant('b', lambda: a.pulse(lambda: a.glow(0, -33, 36, '#ff4a3a', 0.9), 0.7, 0.35))


@sym('eclipse', 'The Eclipse', 'the dark sun: a black disc with a burning rim', 220, '#9954f2', turns=False)
def _(a):
    a.spin(lambda: a.rep(20, lambda: a.poly([(-3.4, -52), (0, -66), (3.4, -52)], 'ep', o='ot')), 42)
    a.spin(lambda: a.rep(10, lambda: a.poly([(-5, -48), (0, -58), (5, -48)], 'l', o='ot'), start=9), 31, back=True)
    a.glow(0, 0, 60, '#9954f2', 0.85)
    a.circ(0, 0, 47, 'b')
    a.ring(0, 0, 47, 2.6, 'w', edge=0)
    a.pulse(lambda: a.glow(0, 0, 54, '#ffffff', 0.5), 3.6, 0.25)
    a.circ(7, -6, 41, 'k')
    a.arc(7, -6, 36, 200, 340, 1.6, 'x', edge=0)
    a.spin(lambda: a.arc(7, -6, 30, 20, 130, 2.4, 'ep', edge=0), 14, cx=7, cy=-6)
    a.spin(lambda: a.arc(7, -6, 24, 200, 290, 2, 'b', edge=0), 9, cx=7, cy=-6, back=True)
    a.glow(7, -6, 24, '#d9a6ff', 0.9)
    a.ell(7, -6, 8, 17, 'ep', o='ot')
    a.ell(7, -6, 2.6, 15, 'k', o='')
    a.variant('b', lambda: a.pulse(lambda: a.ring(0, 0, 55, 4, 'w', edge=0), 1.2, 0.2))


@sym('swarm_queen', 'The Swarm Queen', 'the brood-sister: a moth-queen with eye spots', 120, '#eb59a6')
def _(a):
    a.poly([(6, 4), (32, 8), (56, 34), (46, 54), (24, 52), (8, 34)], 'd', mirror='pair', o='oh')
    a.poly([(8, -8), (30, -42), (58, -50), (62, -18), (46, 0), (22, 14)], 'b', mirror='pair', o='oh')
    a.line([(10, -4), (34, -30)], 2.2, 'x', mirror=True, edge=0)
    a.line([(12, 4), (46, -14)], 2.2, 'x', mirror=True, edge=0)
    a.line([(10, 14), (48, 38)], 2.2, 'x', mirror=True, edge=0)
    a.circ(40, -26, 10, 'l', mirror=True, o='ot')
    a.circ(40, -26, 5.8, 'k', mirror=True, o='')
    a.circ(40, -26, 2.8, 'ek', mirror=True, o='')
    a.circ(34, 38, 7, 'l', mirror=True, o='ot')
    a.circ(34, 38, 3, 'k', mirror=True, o='')
    a.ell(0, 12, 10, 36, 'd')
    for y in (4, 16, 28, 40):
        a.line([(-9, y), (9, y)], 3, 'x', edge=0)
    a.circ(0, -12, 12.5, 'b')
    a.circ(0, -30, 11, 'l')
    a.circ(5, -32, 4.4, 'k', mirror=True, o='')
    a.circ(5, -32, 2.2, 'er', mirror=True, o='')
    a.line([(3, -38), (10, -54), (22, -60)], 2.6, 'd', mirror=True)
    a.circ(23, -60, 3.4, 'ey', mirror=True, o='ot')
    a.poly([(-7, -39), (-5, -50), (-2, -41), (0, -52), (2, -41), (5, -50), (7, -39)], 'g')
    a.variant('b', lambda: a.pulse(lambda: a.glow(0, 0, 58, '#ff8ac4', 0.9), 0.8, 0.3))


@sym('singularity', 'The Singularity', 'the well: a black hole, its disc dragged round into arms', 170, '#4d59e6', turns=False)
def _(a):
    a.glow(0, 0, 66, '#4d59e6', 0.9)
    a.spin(lambda: a.rep(3, lambda: spiral(a, 24, 62, 170, 3, 15, 'b')), 16)
    a.spin(lambda: a.rep(3, lambda: spiral(a, 26, 56, 150, 1.4, 6.4, 'l', start=22, o=''), start=0), 16)
    a.spin(lambda: a.rep(6, lambda: spiral(a, 28, 50, 90, 1, 3.4, 'ec', start=60, o=''), start=0), 11, back=True)
    a.circ(0, 0, 27, 'k')
    a.ring(0, 0, 27, 2.6, 'ey', edge=0)
    a.pulse(lambda: a.ring(0, 0, 30, 6, 'ec', edge=0), 2.4, 0.15)
    a.circ(0, 0, 24, 'k', o='')
    a.spin(lambda: a.rep(2, lambda: a.arc(0, 0, 15, 0, 120, 2.2, 'x', edge=0)), 7, back=True)
    a.circ(0, 0, 4.2, 'w', o='')
    a.variant('b', lambda: a.pulse(lambda: a.glow(0, 0, 54, '#ffffff', 0.9), 1.4, 0.2))


@sym('bastion', 'The Bastion', 'the gun line: an octagonal fort with four turrets, armoured at the front', 150, '#66bfb3', turns=False)
def _(a):
    a.poly([(0, -50), (20, -50), (50, -20), (50, 20), (20, 50), (0, 50)], 'd', mirror='axis', o='oh')
    a.poly([(0, -42), (17, -42), (42, -17), (42, 17), (17, 42), (0, 42)], 'b', mirror='axis', o='')
    for t in (-30, -10, 10, 30):
        a.poly([(t - 4, -54), (t + 4, -54), (t + 4, -46), (t - 4, -46)], 'l', o='ot')
    for t in (-30, -10, 10, 30):
        a.poly([(t - 4, 54), (t + 4, 54), (t + 4, 46), (t - 4, 46)], 'l', o='ot')
    a.arc(0, 0, 54, -58, 58, 9, 'm', edge=2.6)
    a.arc(0, 0, 54, -50, 50, 3, 'ml', edge=0)
    for sx in (-1, 1):
        for sy in (-1, 1):
            x, y = 32 * sx, 32 * sy
            a.line([(x, y), (x, y - 26)], 8.4, 'md')
            a.line([(x, y - 22), (x, y - 28)], 5.2, 'ml', edge=0)
            a.circ(x, y, 12, 'd')
            a.circ(x, y, 7, 'm')
            a.circ(x, y, 3, 'k', o='')
    a.rep(4, lambda: a.poly([(-5, -26), (5, -26), (5, -18), (-5, -18)], 'l', o='ot'), start=45)
    a.circ(0, 0, 21, 'l')
    a.circ(0, 0, 14, 'm')
    a.ring(0, 0, 14, 2.4, 'md', edge=0)
    a.line([(0, 0), (0, -34)], 9.6, 'md')
    a.line([(0, -30), (0, -38)], 6.4, 'ml', edge=0)
    a.glow(0, 0, 14, '#69f0ff', 0.9)
    a.circ(0, 0, 5, 'ec', o='')
    a.rep(2, lambda: a.poly([(-3, 44), (3, 44), (3, 50), (-3, 50)], 'ec', o=''), start=0)
    a.variant('b', lambda: a.pulse(lambda: a.ring(0, 0, 58, 3, 'ec', edge=0), 1.0, 0.2))


@sym('sovereign', 'The Sovereign', 'the last: a crowned tyrant in red and gold, blades for wings', 240, '#eb334d')
def _(a):
    a.spin(lambda: a.rep(12, lambda: a.poly([(-2.4, -50), (0, -66), (2.4, -50)], 'g', o='ot'), start=15), 60)
    for ang, length in ((38, 62), (62, 70), (86, 72), (110, 64), (134, 52)):
        for side in (1, -1):
            a.place(22 * side, -12, lambda length=length: (a.poly([(-8.5, -12), (0, -length), (8.5, -12)], 'm', o='oh'), a.poly([(0, -14), (0, -length + 7), (4.4, -14)], 'ml', o='')), rot=ang * side)
    a.poly([(0, -16), (46, -28), (66, 18), (46, 60), (0, 60)], 'x', mirror='axis', o='oh')
    a.poly([(0, -16), (46, -28), (66, 18), (46, 60), (0, 60)], 'x', mirror='axis', o='')
    a.line([(26, -18), (50, 8), (40, 52)], 3, 'gd', mirror=True, edge=0)
    a.poly([(0, -30), (28, -28), (34, 4), (24, 42), (0, 54)], 'b', mirror='axis', o='oh')
    a.poly([(0, -24), (20, -22), (24, 4), (0, 10)], 'l', mirror='axis', o='')
    a.line([(0, 6), (0, 52)], 3.4, 'g', edge=0)
    a.line([(-24, 4), (0, 12), (24, 4)], 3, 'g', edge=0)
    a.glow(0, 4, 30, '#ff4a3a', 0.9)
    a.poly([(0, -8), (10, 4), (0, 16), (-10, 4)], 'er', o='o')
    a.poly([(0, -5), (5, 4), (0, 7)], 'w', mirror='pair', o='')
    a.circ(34, -26, 16, 'g', mirror=True, o='oh')
    a.circ(34, -26, 9, 'er', mirror=True, o='ot')
    a.poly([(22, -36), (34, -52), (46, -36)], 'g', mirror='pair', o='ot')
    a.line([(52, -8), (56, 52)], 3.4, 'g')
    a.line([(52, -8), (56, 52)], 1.2, 'gd', edge=0)
    a.circ(52, -14, 7.4, 'ep', o='ot')
    a.glow(52, -14, 20, '#d9a6ff', 0.9)
    a.poly([(0, -50), (11, -44), (13, -30), (0, -22)], 'bone', mirror='axis', o='oh')
    a.poly([(-12, -39), (-3, -36), (-8, -33)], 'k', o='')
    a.poly([(12, -39), (3, -36), (8, -33)], 'k', o='')
    a.circ(8, -38, 2.2, 'er', mirror=True, o='')
    a.poly([(-12, -52), (-14, -70), (-7, -62), (0, -78), (7, -62), (14, -70), (12, -52)], 'g', o='oh')
    a.circ(0, -62, 3.6, 'er', o='ot')
    a.circ(-8, -58, 2, 'ec', o='')
    a.circ(8, -58, 2, 'ec', o='')
    a.spin(lambda: a.rep(3, lambda: a.circ(0, -58, 2.4, 'ey', o=''), start=0), 6, cx=0, cy=0)
    a.variant('b', lambda: a.pulse(lambda: a.glow(0, -24, 52, '#ffe45e', 0.9), 1.1, 0.2))
