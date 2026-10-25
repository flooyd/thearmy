"""What is drawn on top of an enemy (an elite's mark, a captain's crown, ice, a shield), in the colours of Modifier::color in src/enemy.rs, and the army's
own ship and the player, drawn the same way so the mock-ups can put them in the same field. These are overlays: drawn after the creature, with the
middle of them left clear."""

from dsl import sym


def corner(a, fn):
    """fn() once at each of the four corners of the hitbox, the one in the top left drawn and the others turned to it."""
    a.rep(4, fn, start=0)


@sym('mark_swift', 'Swift', 'elite: much faster. streaks behind it and chevrons at its shoulders', 100, '#66ffff')
def _(a):
    for x, h in ((-30, 22), (-10, 30), (10, 30), (30, 22)):
        a.poly([(x - 3.6, 56), (x + 3.6, 56), (x, 56 + h)], 'b', o='ot')
    a.alpha(0.9, lambda: a.line([(-60, -30), (-52, -42), (-44, -30)], 4.4, 'b', mirror=True, edge=1.6))
    a.alpha(0.9, lambda: a.line([(-60, -8), (-52, -20), (-44, -8)], 4.4, 'b', mirror=True, edge=1.6))
    a.pulse(lambda: a.glow(0, 62, 36, '#66ffff', 0.8), 0.7, 0.3)


@sym('mark_armored', 'Armored', 'elite: shrugs off half the damage. plates riveted over its edges', 100, '#bfc7d9', turns=False)
def _(a):
    corner(a, lambda: (a.poly([(-58, -58), (-22, -58), (-22, -48), (-48, -48), (-48, -22), (-58, -22)], 'm', o='oh'),
                       a.poly([(-54, -54), (-26, -54), (-26, -51), (-51, -51), (-51, -26), (-54, -26)], 'ml', o=''),
                       a.circ(-53, -53, 2.6, 'k', o='')))
    for s in (1, -1):
        a.poly([(-12, -57 * s), (12, -57 * s), (9, -50 * s), (-9, -50 * s)], 'm', o='ot')
        a.poly([(-57 * s, -12), (-57 * s, 12), (-50 * s, 9), (-50 * s, -9)], 'm', o='ot')


@sym('mark_volatile', 'Volatile', 'elite: goes off when it is destroyed. cracked, with a light inside', 100, '#ff8c1a')
def _(a):
    a.pulse(lambda: a.glow(0, 0, 54, '#ff8c1a', 0.75), 0.5, 0.35)
    a.alpha(0.95, lambda: a.line([(-22, -50), (-12, -30), (-26, -14), (-10, 4), (-20, 30), (-8, 50)], 3.4, 'eo', edge=1.4))
    a.alpha(0.95, lambda: a.line([(20, -50), (10, -34), (24, -16), (8, 6), (22, 26), (14, 50)], 3.4, 'eo', edge=1.4))
    a.line([(-12, -30), (0, -22), (-10, 4)], 2.4, 'ey', edge=0)
    a.poly([(-3, -64), (0, -74), (3, -64), (0, -60)], 'ey', o='ot')
    a.circ(-34, -58, 2.4, 'ey', o='')
    a.circ(36, -56, 2, 'eo', o='')
    a.circ(46, 60, 2.2, 'ey', o='')


@sym('mark_frenzied', 'Frenzied', 'elite: hits twice as hard and runs faster. a ring of flames and red eyes', 100, '#ff404d')
def _(a):
    a.pulse(lambda: a.glow(0, 0, 66, '#ff404d', 0.7), 0.45, 0.3)
    a.rep(10, lambda: a.poly([(-7, -52), (-2, -70), (2, -58), (7, -52)], 'b', o='ot'), start=18)
    a.rep(10, lambda: a.poly([(-3.4, -52), (0, -62), (3.4, -52)], 'eo', o=''), start=18)
    a.circ(12, -22, 4.6, 'er', mirror=True, o='ot')
    a.poly([(6, -34), (22, -26), (8, -27)], 'k', mirror='pair', o='')


@sym('mark_captain', 'Captain', 'leads a pincer horde: a crown, gold on the shoulders and a banner', 100, '#ffcc33', turns=False)
def _(a):
    a.line([(-44, -50), (-44, -80)], 3.2, 'gd')
    a.poly([(-43, -80), (-18, -73), (-43, -64)], 'b', o='ot')
    a.poly([(-27, -50), (-26, -64), (-17, -58), (-9, -72), (0, -58), (9, -72), (17, -58), (26, -64), (27, -50)], 'g', o='oh')
    a.poly([(-25, -50), (25, -50), (25, -55), (-25, -55)], 'gd', o='')
    a.circ(0, -62, 3.4, 'er', o='ot')
    a.circ(-14, -57, 2.2, 'ec', o='')
    a.circ(14, -57, 2.2, 'ec', o='')
    a.circ(52, -34, 11, 'g', mirror=True, o='oh')
    a.circ(52, -34, 5, 'gd', mirror=True, o='')
    a.rep(4, lambda: a.poly([(-52, -52), (-38, -52), (-52, -38)], 'g', o='ot'), start=0)


@sym('mark_frozen', 'Frozen', 'held solid by a Freezing relic: a crust of ice with crystals through it', 100, '#bfedff', turns=False)
def _(a):
    a.alpha(0.5, lambda: a.poly([(-46, -50), (46, -50), (52, -2), (46, 50), (-46, 50), (-52, -2)], 'b', o='ot'))
    a.alpha(0.8, lambda: a.line([(-34, 38), (34, -38)], 5, 'w', edge=0))
    a.poly([(-34, -44), (-26, -70), (-20, -46)], 'b', o='ot')
    a.poly([(22, -46), (32, -66), (40, -42)], 'l', o='ot')
    a.poly([(-52, 20), (-72, 30), (-52, 38)], 'l', o='ot')
    a.poly([(52, 8), (74, 4), (52, 26)], 'b', o='ot')
    a.poly([(-44, 46), (-48, 70), (-30, 48)], 'b', o='ot')
    a.poly([(30, 46), (40, 68), (46, 44)], 'l', o='ot')
    a.poly([(-6, 50), (0, 62), (8, 50)], 'w', o='ot')


@sym('mark_shield', 'Shield', 'a boss shield, or a Bulwark\'s: a bubble of linked cells that wears away', 100, '#66d9ff', turns=False)
def _(a):
    a.alpha(0.2, lambda: a.circ(0, 0, 66, 'b', o=''))
    a.ring(0, 0, 66, 4, 'b', edge=0)
    a.rep(6, lambda: a.arc(0, 0, 58, -22, 22, 3, 'l', edge=0), start=0)
    a.rep(6, lambda: a.line([(-8, -70), (0, -62), (8, -70)], 2.4, 'w', edge=0), start=30)
    a.arc(0, 0, 66, 292, 350, 4.4, 'w', edge=0)
    a.arc(0, 0, 66, 140, 172, 3, 'w', edge=0)


@sym('ship', 'Ship', 'a unit of the army: the dart', 28, '#4d8ccc', note='drawn by the shader already; here so the mock-ups can put one beside the enemies')
def _(a):
    a.poly([(0, 14), (6, 28), (0, 40)], 'eo', o='ot')
    a.poly([(0, -48), (34, 38), (0, 22)], 'b', mirror='axis', o='o')
    a.ell(0, -10, 5.2, 11.6, 'k', o='')
    a.ell(-1.4, -13, 1.2, 3.2, 'ec', o='')


@sym('player', 'The player', 'the stick figure the player is, front on', 20, '#e8eef7', note='drawn by runner.rs already; here so the mock-ups can put one in the field')
def _(a):
    a.ell(0, 52, 22, 6, 'k', o='')
    a.line([(0, -4), (-14, 22), (-18, 48)], 7, 'b', mirror=True)
    a.line([(0, -26), (0, 14)], 9, 'b')
    a.line([(0, -20), (-22, -4), (-24, 18)], 6.4, 'b', mirror=True)
    a.circ(0, -40, 15, 'b')
    a.circ(5, -42, 2.6, 'k', mirror=True, o='')


@sym('mark_chilled', 'Chilled', 'chill building on it: frost creeping in from the corners and a thin cold rim', 100, '#bfedff', turns=False)
def _(a):
    a.alpha(0.6, lambda: a.ring(0, 0, 54, 3, 'l', edge=0))
    corner(a, lambda: a.poly([(-60, -60), (-34, -56), (-44, -46), (-56, -34)], 'w', o='ot'))
    corner(a, lambda: a.line([(-48, -48), (-36, -40)], 2.2, 'l', edge=0))


@sym('mark_burning', 'Burning', 'alight: flames licking up round its edges and embers going up', 100, '#ff8c1a', turns=False)
def _(a):
    a.pulse(lambda: a.glow(0, 0, 62, '#ff8c1a', 0.6), 0.4, 0.4)
    a.rep(8, lambda: a.poly([(-8, -46), (-2, -70), (2, -58), (8, -46)], 'b', o='ot'), start=10)
    a.rep(8, lambda: a.poly([(-4, -46), (0, -60), (4, -46)], 'ey', o=''), start=10)
    a.spin(lambda: a.circ(30, -60, 3, 'ey', o=''), 1.5)
    a.spin(lambda: a.circ(-42, -52, 2.4, 'eo', o=''), 1.1, back=True)


@sym('mark_shocked', 'Shocked', 'charged: sparks crawling over it', 100, '#d9a6ff', turns=False)
def _(a):
    a.pulse(lambda: a.glow(0, 0, 56, '#d9a6ff', 0.5), 0.25, 0.2)
    a.spin(lambda: a.rep(3, lambda: a.line([(-6, -56), (6, -48), (-4, -40), (8, -32)], 3, 'w', edge=1.2)), 0.8)


@sym('mark_poisoned', 'Poisoned', 'poisoned: green bubbles rising off it', 100, '#8dff6a', turns=False)
def _(a):
    a.pulse(lambda: a.glow(0, 0, 52, '#8dff6a', 0.45), 1.0, 0.4)
    for x, y, r in ((-30, -52, 6), (20, -60, 4.5), (42, -30, 5), (-50, -12, 4)):
        a.pulse(lambda x=x, y=y, r=r: a.circ(x, y, r, 'b', o='ot'), 0.9, 0.2)

@sym('mark_slowed', 'Slowed', 'held back by tar or an aura: sluggish ripples spreading slowly round its feet', 100, '#9fb3d9', turns=False)
def _(a):
    a.pulse(lambda: a.alpha(0.85, lambda: a.ring(0, 0, 50, 4, 'w', edge=0)), 2.4, 0.3)
    a.pulse(lambda: a.alpha(0.6, lambda: a.ring(0, 0, 62, 3, 'w', edge=0)), 3.1, 0.2)
    a.rep(3, lambda: a.line([(-10, 66), (0, 72), (10, 66)], 2.4, 'l', edge=0))


def crack(a, points, width=3.4):
    """A crack across a boss: dark, with light showing in it."""
    a.line(points, width, 'k', edge=0)
    a.line(points, width * 0.4, 'l', edge=0)


@sym('mark_cracked_1', 'Cracked I', 'a boss in its second phase: a few cracks across it', 100, '#ffd27a', turns=False)
def _(a):
    crack(a, [(-8, -34), (-16, -18), (-8, -6), (-22, 10)])
    crack(a, [(14, 8), (24, 18), (18, 32)])


@sym('mark_cracked_2', 'Cracked II', 'a boss in its third phase: cracked all over', 100, '#ffd27a', turns=False)
def _(a):
    crack(a, [(-8, -34), (-16, -18), (-8, -6), (-22, 10), (-30, 14)])
    crack(a, [(14, 8), (24, 18), (18, 32), (26, 40)])
    crack(a, [(20, -30), (10, -20), (16, -8)])
    crack(a, [(-6, 22), (4, 30), (-2, 42)])


@sym('mark_cracked_3', 'Cracked III', 'a boss near its end: split open, light pouring out of it', 100, '#ffd27a', turns=False)
def _(a):
    a.pulse(lambda: a.glow(0, 0, 40, '#ffd27a', 0.35), 0.5, 0.3)
    crack(a, [(-8, -40), (-16, -18), (-6, -4), (-24, 10), (-36, 16)], 4.4)
    crack(a, [(10, 4), (24, 18), (18, 32), (28, 44)], 4.4)
    crack(a, [(22, -34), (10, -20), (18, -6), (34, -2)], 4.0)
    crack(a, [(-6, 20), (4, 30), (-4, 44)], 4.0)
    crack(a, [(-6, -4), (10, 4)], 4.0)
    crack(a, [(-34, -26), (-20, -30)], 3.0)
    crack(a, [(30, 22), (40, 30)], 3.0)


@sym('mark_vampiric', 'Vampiric', 'an elite that heals from the harm it does: fangs and drops of blood', 100, '#d9143a', turns=False)
def _(a):
    a.poly([(-14, -58), (-6, -58), (-10, -40)], 'w', o='ot')
    a.poly([(6, -58), (14, -58), (10, -40)], 'w', o='ot')
    for x, y in ((-36, -44), (34, -36), (-44, 14), (40, 20)):
        a.pulse(lambda x=x, y=y: a.ell(x, y, 3.4, 5, 'b', o='ot'), 1.1, 0.2)


@sym('mark_thorned', 'Thorned', 'an elite that hurts what hurts it: thorns all round its edge', 100, '#80cc40', turns=False)
def _(a):
    a.rep(12, lambda: a.poly([(-5, -50), (0, -66), (5, -50)], 'b', o='ot'), start=15)


@sym('mark_teleporting', 'Teleporting', 'an elite that jumps at the army: a turning swirl and sparkles', 100, '#bf80ff', turns=False)
def _(a):
    a.spin(lambda: a.rep(3, lambda: a.arc(0, 0, 58, 0, 80, 3.4, 'b', edge=1.2)), 1.6)
    a.pulse(lambda: a.circ(40, -40, 3, 'w', o=''), 0.5, 0.0)
    a.pulse(lambda: a.circ(-44, 30, 2.6, 'w', o=''), 0.7, 0.0)


@sym('mark_magnetic', 'Magnetic', 'an elite that drags units in: a horseshoe magnet and its field', 100, '#8cb3f2', turns=False)
def _(a):
    a.arc(0, -52, 12, 180, 360, 7, 'm', edge=1.6)
    a.poly([(-15, -52), (-9, -52), (-9, -44), (-15, -44)], 'er', o='ot')
    a.poly([(9, -52), (15, -52), (15, -44), (9, -44)], 'b', o='ot')
    a.pulse(lambda: a.ring(0, 0, 62, 2, 'b', edge=0), 0.8, 0.2)


@sym('mark_blazing', 'Blazing', 'an elite with a burning aura: a ring of fire round it', 100, '#ff801a', turns=False)
def _(a):
    a.pulse(lambda: a.glow(0, 0, 70, '#ff801a', 0.5), 0.6, 0.5)
    a.spin(lambda: a.rep(10, lambda: a.poly([(-6, -54), (0, -72), (6, -54)], 'eo', o='ot')), 4.0)


@sym('mark_frigid', 'Frigid', 'an elite with a freezing aura: ice crystals turning round it', 100, '#b3ebff', turns=False)
def _(a):
    a.alpha(0.5, lambda: a.ring(0, 0, 62, 2.4, 'b', edge=0))
    a.spin(lambda: a.rep(6, lambda: a.poly([(0, -72), (5, -62), (0, -52), (-5, -62)], 'w', o='ot')), 6.0, back=True)


@sym('mark_conjuring', 'Conjuring', 'an elite that calls up imps: a ring of runes', 100, '#d966f2', turns=False)
def _(a):
    a.spin(lambda: a.rep(8, lambda: a.poly([(-3, -60), (3, -60), (0, -54)], 'b', o='')), 5.0)
    a.alpha(0.6, lambda: a.ring(0, 0, 57, 1.6, 'b', edge=0))


@sym('mark_regenerating', 'Regenerating', 'an elite that mends itself: green crosses rising off it', 100, '#66ff80', turns=False)
def _(a):
    for x, y in ((-38, -46), (36, -54), (44, 6)):
        a.pulse(lambda x=x, y=y: a.place(x, y, lambda: a.poly([(-2.5, -8), (2.5, -8), (2.5, -2.5), (8, -2.5), (8, 2.5), (2.5, 2.5), (2.5, 8), (-2.5, 8), (-2.5, 2.5), (-8, 2.5), (-8, -2.5), (-2.5, -2.5)], 'b', o='ot')), 1.2, 0.2)
