"""The 29 kinds of enemy, in the colours and sizes of ENEMY_KINDS in src/enemy.rs. Each faces up, and fills the square of its hitbox (the dashed one in the preview)."""

from dsl import sym


@sym('grunt', 'Grunt', 'the ordinary one: a beetle with pincers', 44, '#e62938')
def _(a):
    a.line([(24, -14), (38, -20), (46, -8)], 5.5, 'd', mirror=True)
    a.line([(28, 6), (44, 8), (48, 22)], 5.5, 'd', mirror=True)
    a.line([(22, 26), (34, 36), (36, 48)], 5.5, 'd', mirror=True)
    a.poly([(8, -30), (10, -46), (20, -57), (25, -49), (18, -41), (20, -28)], 'bone', mirror='pair')
    a.poly([(0, -36), (22, -32), (38, -14), (40, 18), (28, 42), (0, 48)], 'b', mirror='axis')
    a.poly([(0, -26), (9, -20), (10, 30), (0, 40)], 'l', mirror='axis', o='')
    a.poly([(22, -18), (31, -6), (31, 18), (22, 30)], 'd', mirror='pair', o='')
    a.circ(11, -27, 4.4, 'ey', mirror=True, o='ot')


@sym('scout', 'Scout', 'small, fast and frail: a wasp', 30, '#f5e61e')
def _(a):
    a.poly([(8, -6), (32, 4), (54, 30), (44, 40), (10, 22)], 'd', mirror='pair')
    a.poly([(0, -54), (10, -28), (10, 24), (0, 50)], 'b', mirror='axis')
    a.poly([(-10, -4), (10, -4), (10.6, 4), (-10.6, 4)], 'x', o='')
    a.poly([(-10.4, 10), (10.4, 10), (10, 18), (-10, 18)], 'x', o='')
    a.poly([(0, 48), (4, 58), (0, 66), (-4, 58)], 'bone', o='ot')
    a.circ(5, -34, 3.6, 'er', mirror=True, o='ot')


@sym('brute', 'Brute', 'slow and tough: a bruiser with fists', 72, '#a640f2')
def _(a):
    a.line([(36, -8), (40, -34)], 15, 'd', mirror=True)
    a.poly([(42, -36), (58, -46), (52, -28)], 'bone', mirror='pair')
    a.poly([(0, -26), (26, -30), (46, -8), (46, 24), (30, 44), (0, 48)], 'b', mirror='axis')
    a.circ(41, -40, 13, 'l', mirror=True)
    a.poly([(0, -14), (18, -16), (26, 0), (0, 6)], 'l', mirror='axis', o='')
    a.poly([(0, 14), (14, 16), (18, 34), (0, 40)], 'd', mirror='axis', o='')
    a.circ(0, -28, 12, 'd')
    a.poly([(-9, -32), (-1, -30), (-9, -27)], 'er', o='')
    a.poly([(9, -32), (1, -30), (9, -27)], 'er', o='')


@sym('titan', 'Titan', 'a giant in plate with a burning core', 130, '#f2801a')
def _(a):
    a.line([(41, -4), (45, -36)], 20, 'd', mirror=True)
    a.circ(45, -41, 15, 'b', mirror=True)
    a.circ(45, -41, 7, 'd', mirror=True, o='')
    a.poly([(0, -42), (30, -38), (48, -12), (48, 26), (30, 48), (0, 52)], 'b', mirror='axis')
    a.poly([(34, -44), (54, -40), (60, -14), (44, -8), (34, -22)], 'd', mirror='pair')
    a.poly([(50, -40), (58, -52), (58, -34)], 'bone', mirror='pair', o='ot')
    a.poly([(0, -28), (22, -26), (26, 2), (0, 8)], 'l', mirror='axis', o='ot')
    a.poly([(0, 20), (14, 22), (16, 44), (0, 48)], 'd', mirror='axis', o='')
    a.circ(0, -9, 14, 'k', o='')
    a.glow(0, -9, 26, '#ffb04a')
    a.circ(0, -9, 9.5, 'ey')
    a.circ(0, -9, 4.5, 'w', o='')
    a.poly([(0, -52), (11, -46), (9, -35), (0, -33)], 'd', mirror='axis')
    a.poly([(-7, -45), (7, -45), (5, -41), (-5, -41)], 'eo', o='')


@sym('colossus', 'Colossus', 'a walking fortress with a single burning eye', 220, '#f2339a')
def _(a):
    a.rep(4, lambda: a.poly([(28, -50), (50, -50), (50, -28), (40, -32), (32, -40)], 'd'), start=0)
    a.rep(8, lambda: a.poly([(-9, -44), (0, -63), (9, -44)], 'd'), start=22.5)
    a.circ(0, 0, 46, 'b')
    a.rep(8, lambda: a.poly([(-12, -43), (12, -43), (15, -31), (-15, -31)], 'l', o='ot'), start=22.5)
    a.ring(0, 0, 27, 5, 'x', edge=0)
    a.circ(0, 0, 21, 'k')
    a.glow(0, 0, 38, '#ff5fa0')
    a.circ(0, 0, 16, 'ek')
    a.ell(0, 0, 5, 13, 'k', o='')
    a.rep(6, lambda: a.line([(0, -24), (3, -31), (-2, -37)], 2, 'x', edge=0), start=30)


@sym('spitter', 'Spitter', 'lobs acid from a bloated sac', 56, '#9e6b40')
def _(a):
    a.line([(20, 2), (38, -4), (44, 10)], 5, 'd', mirror=True)
    a.line([(22, 24), (38, 30), (40, 44)], 5, 'd', mirror=True)
    a.circ(0, 14, 33, 'b')
    a.circ(0, 14, 21, 'eg', o='ot')
    a.circ(-7, 8, 5, 'w', o='')
    a.circ(8, 22, 3.4, 'w', o='')
    a.circ(-22, 30, 6, 'l', o='ot')
    a.circ(22, 32, 5, 'l', o='ot')
    a.poly([(0, -34), (12, -28), (16, -6), (0, -6)], 'd', mirror='axis')
    a.poly([(0, -56), (7, -52), (9, -34), (0, -34)], 'md', mirror='axis')
    a.glow(0, -57, 16, '#8dff6a')
    a.circ(0, -56, 5, 'eg', o='')
    a.circ(9, -20, 3.6, 'ey', mirror=True, o='ot')


@sym('swarmer', 'Swarmer', 'tiny, quick, and in a pack: a gnat', 20, '#99e633')
def _(a):
    a.alpha(0.6, lambda: a.ell(30, -2, 13, 25, 'l', rot=-24, mirror=True))
    a.ell(0, 4, 17, 30, 'b')
    a.poly([(-16, 4), (16, 4), (15, 12), (-15, 12)], 'x', o='')
    a.circ(0, -24, 12, 'd')
    a.circ(5, -27, 3.6, 'er', mirror=True, o='')
    a.poly([(0, 30), (4, 44), (0, 58), (-4, 44)], 'bone', o='ot')


@sym('bulwark', 'Bulwark', 'behind a shield that grows back', 78, '#4d8ccc')
def _(a):
    a.circ(30, 24, 11, 'd', mirror=True)
    a.poly([(0, -6), (36, -6), (40, 24), (30, 46), (0, 50)], 'b', mirror='axis')
    a.circ(0, 20, 15, 'd')
    a.poly([(-10, 8), (10, 8), (10, 12), (-10, 12)], 'ec', o='')
    a.poly([(0, -54), (38, -46), (52, -24), (48, -4), (26, -10), (0, -8)], 'md', mirror='axis', o='oh')
    a.poly([(0, -47), (31, -40), (43, -22), (40, -9), (24, -15), (0, -14)], 'm', mirror='axis', o='')
    a.line([(0, -46), (0, -16)], 4, 'md', edge=0)
    a.circ(0, -31, 9, 'l')
    a.circ(0, -31, 3.5, 'k', o='')
    a.circ(33, -34, 2.6, 'ml', mirror=True, o='')
    a.circ(36, -20, 2.6, 'ml', mirror=True, o='')


@sym('splitter', 'Splitter', 'breaks into three: a cell with a seam', 66, '#1abf99', turns=False)
def _(a):
    a.circ(-19, 14, 26, 'b')
    a.circ(19, 14, 26, 'b')
    a.circ(0, -20, 26, 'b')
    for x, y in ((-19, 14), (19, 14), (0, -20)):
        a.circ(x, y, 9, 'x', o='')
        a.circ(x, y, 4.6, 'l', o='')
    a.arc(0, -20, 18, 300, 40, 2.4, 'l', edge=0)
    a.arc(-19, 14, 18, 200, 280, 2.4, 'l', edge=0)
    a.arc(19, 14, 18, 80, 160, 2.4, 'l', edge=0)


@sym('charger', 'Charger', 'stops, shows its lane, and rams: horned', 62, '#992e1f')
def _(a):
    a.poly([(12, -20), (26, -30), (32, -46), (44, -58), (49, -44), (42, -24), (24, -8)], 'bone', mirror='pair')
    a.poly([(0, -12), (26, -10), (40, 10), (34, 38), (0, 46)], 'b', mirror='axis')
    a.poly([(0, 2), (7, 10), (7, 34), (0, 40)], 'l', mirror='axis', o='')
    a.poly([(0, -48), (17, -34), (21, -12), (0, -6)], 'd', mirror='axis')
    a.circ(9, -26, 3.2, 'eo', mirror=True, o='')
    a.poly([(-5, -40), (0, -37), (5, -40), (0, -44)], 'x', o='')
    a.poly([(26, 34), (38, 34), (38, 48), (26, 48)], 'k', mirror='pair', o='')


@sym('bomber', 'Bomber', 'runs at the army and goes off: a lit bomb', 34, '#666675')
def _(a):
    a.circ(25, 28, 7, 'd', mirror=True)
    a.circ(0, 4, 33, 'b')
    a.poly([(-28, -8), (0, 6), (28, -8), (28, -2), (0, 14), (-28, -2)], 'ey', o='')
    a.circ(10, -14, 6.4, 'k', mirror=True, o='')
    a.circ(10, -14, 3, 'er', mirror=True, o='')
    a.poly([(-11, 20), (11, 20), (9, 27), (-9, 27)], 'bone', o='ot')
    a.line([(0, -28), (6, -42), (16, -48)], 4, 'md')
    a.glow(17, -50, 20, '#ffb020')
    a.circ(17, -50, 4.4, 'ey', o='')


@sym('healer', 'Healer', 'mends the others: a medic orb', 40, '#99ffb3', turns=False)
def _(a):
    a.glow(0, 0, 56, '#8dff6a', 0.8)
    a.rep(4, lambda: a.circ(0, -42, 7, 'eg'), start=45)
    a.circ(0, 0, 32, 'b')
    a.ring(0, 0, 26, 3, 'l', edge=0)
    a.poly([(-8, -22), (8, -22), (8, -8), (22, -8), (22, 8), (8, 8), (8, 22), (-8, 22), (-8, 8), (-22, 8), (-22, -8), (-8, -8)], 'w')


@sym('summoner', 'Summoner', 'calls up imps: a hooded caster with runes', 54, '#5940bf')
def _(a):
    a.spin(lambda: a.rep(4, lambda: a.poly([(0, -52), (6, -45), (0, -38), (-6, -45)], 'ep', o='ot'), start=45), 9)
    a.line([(20, -2), (34, -20), (37, -33)], 7, 'd', mirror=True)
    a.glow(37, -37, 18, '#d9a6ff', 0.9)
    a.circ(37, -36, 5, 'ep', mirror=True, o='')
    a.poly([(0, -22), (24, -8), (30, 22), (18, 46), (0, 50)], 'b', mirror='axis')
    a.poly([(0, 8), (11, 12), (8, 46), (0, 50)], 'd', mirror='axis', o='')
    a.circ(0, -12, 17, 'd')
    a.circ(0, -14, 10, 'k', o='')
    a.circ(4.4, -14, 2.4, 'ep', mirror=True, o='')


@sym('mortar', 'Mortar', 'lobs shells from far away: a cannon on treads', 58, '#8c9933')
def _(a):
    a.poly([(34, -32), (46, -34), (49, 32), (37, 38)], 'md', mirror='pair')
    a.line([(41, -22), (41, 28)], 2.4, 'k', mirror=True, edge=0)
    a.poly([(0, -34), (28, -28), (32, 0), (30, 30), (0, 38)], 'b', mirror='axis')
    a.poly([(-9, -2), (9, -2), (10, -50), (-10, -50)], 'md')
    a.poly([(-11.5, -28), (11.5, -28), (11.5, -22), (-11.5, -22)], 'm', o='ot')
    a.poly([(-13, -60), (13, -60), (10, -48), (-10, -48)], 'm')
    a.ell(0, -58, 6, 2.6, 'k', o='')
    a.glow(0, -60, 15, '#ffb04a', 0.8)
    a.circ(0, 8, 22, 'd')
    a.circ(0, 10, 8, 'l')


@sym('lancer', 'Lancer', 'fires one piercing bolt: a jouster', 52, '#d9ebff')
def _(a):
    a.poly([(8, 18), (26, 44), (14, 50), (8, 38)], 'd', mirror='pair')
    a.poly([(0, -8), (14, 10), (12, 40), (0, 48)], 'b', mirror='axis')
    a.poly([(-30, -10), (30, -10), (25, -2), (-25, -2)], 'm')
    a.poly([(0, -64), (5, -40), (5, -8), (0, -8)], 'ml', mirror='axis')
    a.line([(0, -58), (0, -12)], 2, 'ec', edge=0)
    a.ell(0, 20, 5, 9, 'ec', o='ot')
    a.poly([(-4, 48), (0, 61), (4, 48)], 'eo', o='')


@sym('grenadier', 'Grenadier', 'lobs slow grenades: a barrel-chested thrower', 60, '#e68c1a')
def _(a):
    a.poly([(0, -20), (30, -14), (40, 10), (34, 40), (0, 46)], 'b', mirror='axis')
    a.poly([(0, 20), (31, 10), (33, 18), (0, 28)], 'k', mirror='axis', o='')
    a.poly([(-14, -14), (14, -14), (16, -48), (-16, -48)], 'md')
    a.poly([(-19, -56), (19, -56), (16, -46), (-16, -46)], 'm')
    a.circ(0, -54, 8.5, 'd')
    a.circ(0, -54, 3.8, 'ey', o='')
    a.circ(24, 0, 5, 'ey', mirror=True, o='ot')
    a.circ(0, 12, 14, 'd')
    a.circ(5.4, 8, 3.6, 'ec', mirror=True, o='ot')


@sym('seeker', 'Seeker', 'its orbs bend towards the nearest unit: an eye', 36, '#eb4db8')
def _(a):
    a.poly([(18, 16), (44, 36), (28, 44), (14, 30)], 'd', mirror='pair')
    a.poly([(0, 26), (9, 52), (0, 46), (-9, 52)], 'd')
    a.line([(0, -30), (0, -46)], 3, 'd')
    a.circ(0, -49, 4.4, 'er')
    a.circ(0, 0, 31, 'b')
    a.circ(0, -2, 20, 'w')
    a.circ(0, -2, 12, 'l')
    a.circ(0, -2, 5.6, 'k', o='')
    a.circ(-3.4, -6, 2.2, 'w', o='')


@sym('ambusher', 'Ambusher', 'lies hidden, then springs: a spider', 40, '#5c4275')
def _(a):
    for leg in ([(12, -16), (34, -36), (49, -24)], [(14, -6), (40, -14), (54, 0)], [(14, 6), (40, 16), (52, 32)], [(10, 16), (28, 34), (36, 52)]):
        a.line(leg, 4.5, 'b', mirror=True)
    a.ell(0, 18, 17, 27, 'b')
    a.poly([(0, 2), (7, 13), (0, 25), (-7, 13)], 'l', o='')
    a.poly([(0, 28), (5, 36), (0, 44), (-5, 36)], 'l', o='')
    a.circ(0, -14, 14, 'd')
    a.poly([(4, -23), (8, -38), (11, -23)], 'bone', mirror='pair', o='ot')
    a.circ(5, -18, 3.6, 'er', mirror=True, o='')
    a.circ(10, -11, 2.2, 'er', mirror=True, o='')


@sym('imp', 'Imp', 'what summoners make: a bat with horns', 22, '#cc4de6')
def _(a):
    a.poly([(6, -4), (22, -26), (46, -32), (40, -14), (56, 0), (38, 2), (34, 20), (20, 8), (6, 12)], 'd', mirror='pair')
    a.line([(8, 0), (44, -28)], 2, 'x', edge=0, mirror=True)
    a.line([(0, 30), (8, 42), (2, 52)], 3, 'd')
    a.poly([(-4, 50), (8, 46), (4, 62), (-2, 58)], 'd', o='ot')
    a.ell(0, 10, 12, 22, 'b')
    a.poly([(6, -26), (12, -44), (15, -26)], 'bone', mirror='pair', o='ot')
    a.circ(0, -16, 14, 'b')
    a.circ(5.4, -18, 3.4, 'ey', mirror=True, o='')


@sym('splitling', 'Splitling', 'what a splitter breaks into: twin cells', 34, '#59e6bf', turns=False)
def _(a):
    a.circ(-16, 8, 29, 'b')
    a.circ(16, 8, 29, 'b')
    a.circ(-16, 12, 8, 'x', o='')
    a.circ(16, 12, 8, 'x', o='')
    a.circ(-16, 12, 3.6, 'l', o='')
    a.circ(16, 12, 3.6, 'l', o='')
    a.circ(0, -22, 5.2, 'w', o='ot')
    a.circ(-9, -14, 4.2, 'w', o='ot')
    a.circ(9, -14, 4.2, 'w', o='ot')


@sym('drone', 'Drone', 'hovers and shoots: a quad-rotor', 28, '#80ccff')
def _(a):
    a.line([(0, 0), (30, -30)], 6, 'md', mirror=True)
    a.line([(0, 0), (30, 30)], 6, 'md', mirror=True)
    for sx in (-1, 1):
        for sy in (-1, 1):
            a.alpha(0.6, lambda sx=sx, sy=sy: a.circ(30 * sx, 30 * sy, 17, 'ml', o='ot'))
            a.spin(lambda sx=sx, sy=sy: a.line([(30 * sx - 14, 30 * sy), (30 * sx + 14, 30 * sy)], 3, 'md', edge=0), 0.5 + 0.07 * sx * sy, cx=30 * sx, cy=30 * sy)
    a.circ(0, 0, 19, 'b')
    a.circ(0, -4, 9.5, 'k')
    a.glow(0, -4, 16, '#69f0ff')
    a.circ(0, -4, 4.8, 'ec', o='')
    a.poly([(-3, -32), (3, -32), (3, -20), (-3, -20)], 'md', o='ot')


@sym('shard', 'Shard', 'what a frozen enemy shatters into: ice', 26, '#bfedff')
def _(a):
    a.poly([(8, 8), (30, -22), (44, 6), (26, 40)], 'd', mirror='pair')
    a.poly([(0, -58), (16, -14), (0, 52)], 'b', mirror='axis')
    a.poly([(0, -58), (16, -14), (0, -8)], 'w', o='', mirror='pair')
    a.poly([(-3, -26), (0, -34), (3, -26), (0, -18)], 'w', o='')
    a.poly([(12, 4), (14, -6), (16, 4), (14, 14)], 'w', o='')


@sym('nest', 'Nest', 'lairs are built round it: a maw in a mound of eggs', 96, '#cc404d', turns=False)
def _(a):
    a.rep(8, lambda: a.poly([(-9, -40), (0, -61), (9, -40)], 'd'), start=22.5)
    a.circ(0, 0, 46, 'd')
    a.rep(8, lambda: a.ell(0, -35, 11.5, 15, 'b'))
    a.rep(8, lambda: a.circ(0, -37, 4, 'l', o=''))
    a.circ(0, 0, 25, 'k')
    a.rep(10, lambda: a.poly([(-5, -23), (5, -23), (0, -10)], 'bone', o='ot'), start=18)
    a.glow(0, 0, 22, '#ff3a3a')
    a.circ(0, 0, 7, 'er', o='')


@sym('executioner', 'Executioner', 'winds up a cleave that cuts down what is in front of it: a hooded brute with a great axe', 70, '#8c1f26')
def _(a):
    a.line([(26, 10), (42, 20), (44, 36)], 6, 'd', mirror=True)
    a.poly([(0, -30), (26, -24), (36, 0), (32, 30), (0, 42)], 'b', mirror='axis')
    a.poly([(0, -42), (16, -36), (18, -18), (0, -12)], 'x', mirror='axis')
    a.circ(7, -26, 3, 'er', mirror=True, o='')
    a.line([(30, -6), (30, -54)], 4, 'bd')
    a.poly([(30, -54), (48, -60), (50, -38), (30, -42)], 'm')
    a.line([(48, -60), (50, -38)], 2.4, 'ml', edge=0)


@sym('sniper', 'Sniper', 'shows a long line at the army and fires down it: a gaunt marksman with a long rifle', 46, '#738c73')
def _(a):
    a.line([(18, 14), (30, 26), (30, 42)], 4.5, 'd', mirror=True)
    a.poly([(0, -22), (18, -16), (22, 10), (16, 34), (0, 38)], 'b', mirror='axis')
    a.circ(0, -18, 12, 'd')
    a.poly([(-3, -12), (3, -12), (3, -66), (-3, -66)], 'md')
    a.poly([(-6, -30), (6, -30), (6, -44), (-6, -44)], 'm', o='ot')
    a.pulse(lambda: a.glow(0, -66, 10, '#ff4a3a', 0.8), 0.8, 0.3)
    a.circ(0, -20, 3.2, 'er', o='')


@sym('hexer', 'Hexer', 'curses the ground so the units in it take more harm: a robed witch with a turning ring of runes', 48, '#8c33bf')
def _(a):
    a.spin(lambda: a.rep(6, lambda: a.poly([(-4, -48), (4, -48), (0, -40)], 'ep', o='')), 6.0)
    a.poly([(0, -28), (18, -18), (26, 30), (0, 40)], 'b', mirror='axis')
    a.poly([(0, -42), (14, -32), (12, -16), (0, -12)], 'x', mirror='axis')
    a.circ(6, -24, 3, 'ep', mirror=True, o='')
    a.pulse(lambda: a.glow(0, 6, 16, '#d9a6ff', 0.7), 1.2, 0.3)
    a.circ(0, 6, 6, 'ep', o='ot')


@sym('blinker', 'Blinker', 'flickers, and is behind the army in a moment: a shard of a creature with an afterimage', 40, '#7366ff')
def _(a):
    a.alpha(0.35, lambda: a.poly([(0, -24), (22, 12), (12, 46), (0, 38)], 'l', mirror='axis'))
    a.poly([(0, -46), (24, -6), (14, 30), (0, 20)], 'b', mirror='axis')
    a.poly([(0, -32), (10, -10), (0, 2)], 'l', mirror='axis', o='')
    a.circ(7, -16, 3, 'ec', mirror=True, o='')
    a.pulse(lambda: a.glow(0, -8, 18, '#69f0ff', 0.6), 0.4, 0.2)


@sym('sapper', 'Sapper', 'lays mines where the army walks: a crouched digger with a pack of mines', 44, '#99804d')
def _(a):
    a.line([(20, 8), (34, 16), (38, 30)], 5, 'd', mirror=True)
    a.poly([(0, -24), (20, -18), (26, 8), (20, 30), (0, 36)], 'b', mirror='axis')
    a.circ(0, 16, 14, 'md')
    a.circ(-6, 12, 4, 'm', o='ot')
    a.circ(6, 20, 4, 'm', o='ot')
    a.circ(0, -22, 10, 'd')
    a.circ(0, -44, 9, 'md')
    a.pulse(lambda: a.circ(0, -44, 3.4, 'er', o=''), 0.6, 0.1)


@sym('overseer', 'Overseer', 'marks a champion, and a heavy blow lands where it was: a floating eye that watches', 60, '#267380')
def _(a):
    a.rep(4, lambda: a.line([(0, 28), (0, 48)], 4, 'd'), start=45)
    a.circ(0, 0, 34, 'b')
    a.circ(0, -4, 22, 'w', o='ot')
    a.circ(0, -6, 11, 'g', o='')
    a.circ(0, -6, 5, 'k', o='')
    a.pulse(lambda: a.glow(0, -6, 26, '#f2c14e', 0.5), 1.4, 0.3)
