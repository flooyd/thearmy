"""The champions, in the colours and sizes of CHAMPIONS in src/champions.rs: each faces up, and its hitbox is the circle of its body (the size is its diameter at
level 1, RADIUS times twice its `size`). They are the player's, so they are drawn as heroes rather than beasts: armour, weapons, a light of their own."""

from dsl import sym


@sym('ch_juggernaut', 'Champion Juggernaut', 'a riveted bruiser behind riot plates', 96, '#d19959')
def _(a):
    a.line([(34, 8), (44, -14)], 15, 'd', mirror=True)
    a.circ(44, -22, 14, 'md', mirror=True)
    a.circ(44, -22, 6, 'm', mirror=True, o='')
    a.poly([(0, -34), (26, -32), (42, -16), (44, 16), (30, 40), (0, 46)], 'b', mirror='axis')
    a.poly([(0, 12), (20, 14), (24, 34), (0, 40)], 'd', mirror='axis', o='')
    a.poly([(22, -30), (48, -30), (56, -8), (46, 6), (28, -6)], 'd', mirror='pair')
    a.circ(42, -22, 2.6, 'ml', mirror=True, o='')
    a.circ(48, -6, 2.6, 'ml', mirror=True, o='')
    a.circ(30, 20, 2.6, 'ml', mirror=True, o='')
    a.circ(22, 32, 2.6, 'ml', mirror=True, o='')
    a.poly([(0, -54), (24, -50), (32, -34), (0, -28)], 'm', mirror='axis')
    a.poly([(0, -50), (16, -47), (21, -38), (0, -35)], 'ml', mirror='axis', o='')
    a.circ(0, -4, 16, 'l')
    a.poly([(0, -16), (12, -12), (13, 4), (0, 8)], 'd', mirror='axis', o='')
    a.glow(0, -12, 16, '#ffd27a', 0.8)
    a.poly([(-10, -15), (10, -15), (9, -10), (-9, -10)], 'ey', o='ot')


@sym('ch_berserker', 'Champion Berserker', 'horned, with a cleaver in each hand and red eyes', 77, '#e62433')
def _(a):
    a.line([(20, -2), (38, -18), (42, -34)], 9, 'd', mirror=True)
    a.poly([(34, -36), (52, -60), (64, -52), (50, -28)], 'ml', mirror='pair')
    a.poly([(50, -28), (64, -52), (68, -48), (56, -24)], 'w', mirror='pair', o='')
    a.line([(42, -34), (36, -28)], 6, 'gd', mirror=True, edge=1.6)
    a.poly([(0, -24), (22, -22), (34, -6), (32, 20), (18, 38), (0, 42)], 'b', mirror='axis')
    a.poly([(0, -6), (16, -8), (26, 6), (18, 22), (0, 18)], 'd', mirror='axis', o='')
    a.line([(-14, 4), (-4, 14)], 2.4, 'x', edge=0)
    a.line([(10, 10), (18, 2)], 2.4, 'x', edge=0)
    a.poly([(8, -26), (18, -42), (28, -58), (24, -40), (16, -22)], 'bone', mirror='pair')
    a.circ(0, -18, 14, 'd')
    a.glow(0, -22, 16, '#ff4a3a', 0.7)
    a.poly([(-11, -25), (-3, -21), (-11, -18)], 'er', o='')
    a.poly([(11, -25), (3, -21), (11, -18)], 'er', o='')


@sym('ch_bulwark', 'Champion Bulwark', 'a knight behind a tower shield as wide as a wall', 83, '#7399eb')
def _(a):
    a.poly([(0, -32), (24, -28), (36, -10), (34, 18), (20, 38), (0, 42)], 'm', mirror='axis')
    a.poly([(0, -22), (12, -20), (14, 30), (0, 36)], 'b', mirror='axis', o='')
    a.circ(30, -18, 12, 'ml', mirror=True)
    a.circ(0, -10, 13, 'ml')
    a.line([(0, -22), (0, 2)], 5, 'b', edge=1.6)
    a.poly([(-9, -16), (9, -16), (8, -12), (-8, -12)], 'k', o='')
    a.poly([(0, -62), (32, -58), (50, -46), (48, -34), (28, -42), (0, -44)], 'b', mirror='axis', o='oh')
    a.poly([(0, -59), (28, -55), (42, -46), (26, -50), (0, -52)], 'l', mirror='axis', o='')
    a.circ(0, -52, 7, 'g', o='ot')
    a.circ(0, -52, 3, 'gd', o='')
    a.circ(20, -52, 2.4, 'ml', mirror=True, o='')
    a.circ(38, -46, 2.4, 'ml', mirror=True, o='')


@sym('ch_medic', 'Champion Medic', 'a white hover drone with a green cross, ringed in soft light', 64, '#66f299', turns=False)
def _(a):
    a.glow(0, 0, 62, '#8dff6a', 0.35)
    a.pulse(lambda: a.ring(0, 0, 40, 2.5, 'l', edge=0), seconds=1.6, low=0.2)
    a.rep(4, lambda: a.line([(0, -14), (0, -34)], 7, 'md'), start=45)
    a.rep(4, lambda: a.circ(0, -40, 13, 'x'), start=45)
    a.rep(4, lambda: a.place(0, -40, lambda: a.spin(lambda: a.line([(-11, 0), (11, 0)], 3, 'ml', edge=0), 0.35)), start=45)
    a.circ(0, 0, 25, 'w')
    a.ring(0, 0, 19, 2, 'ml', edge=0)
    a.poly([(-6, -16), (6, -16), (6, -6), (16, -6), (16, 6), (6, 6), (6, 16), (-6, 16), (-6, 6), (-16, 6), (-16, -6), (-6, -6)], 'b', o='ot')
    a.circ(0, -22, 3.4, 'ec', o='ot')


@sym('ch_drummer', 'Champion Drummer', 'a war-drum carrier, the beat going out ahead in rings', 64, '#f2b847')
def _(a):
    a.pulse(lambda: a.arc(0, -20, 46, -40, 40, 3, 'l', edge=0), seconds=0.7, low=0.15)
    a.pulse(lambda: a.arc(0, -20, 58, -32, 32, 2.4, 'l', edge=0), seconds=0.7, low=0.0)
    a.poly([(0, -6), (22, -4), (32, 14), (26, 34), (0, 40)], 'd', mirror='axis')
    a.poly([(0, 4), (12, 6), (14, 30), (0, 34)], 'b', mirror='axis', o='')
    a.line([(22, 0), (24, -20), (16, -36)], 6, 'd', mirror=True)
    a.circ(0, -22, 26, 'b')
    a.place(0, -22, lambda: a.rep(10, lambda: a.line([(0, -19), (0, -25)], 3, 'gd', edge=0), start=18))
    a.ring(0, -22, 19, 3, 'gd', edge=1.2)
    a.circ(0, -22, 16, 'bone', o='')
    a.circ(0, -22, 6, 'bd', o='')
    a.line([(-24, 6), (0, -2), (24, 6)], 3.4, 'x', edge=1.2)
    a.line([(-22, -6), (-6, -18)], 3.6, 'bone', edge=1.6)
    a.circ(-5, -19, 4, 'w', o='ot')
    a.line([(22, -6), (34, -30)], 3.6, 'bone', edge=1.6)
    a.circ(35, -32, 4, 'w', o='ot')
    a.circ(0, 16, 10, 'l')
    a.ring(0, 16, 10, 2.4, 'gd', edge=1.2)


@sym('ch_pyromancer', 'Champion Pyromancer', 'a hooded mage with a burning staff', 67, '#ff8014')
def _(a):
    a.poly([(0, -18), (22, -14), (34, 10), (30, 42), (0, 48)], 'd', mirror='axis')
    a.poly([(0, 22), (24, 24), (30, 42), (0, 48)], 'b', mirror='axis', o='')
    a.line([(26, 14), (32, -48)], 5, 'gd')
    a.circ(26, 0, 6.5, 'l')
    a.poly([(0, -36), (16, -28), (22, -8), (12, 6), (0, 8)], 'b', mirror='axis')
    a.ell(0, -16, 10, 9, 'k', o='')
    a.circ(4, -18, 2.2, 'eo', mirror=True, o='')
    a.poly([(32, -48), (38, -54), (36, -46), (28, -46), (26, -54)], 'g', o='ot')
    a.glow(32, -64, 26, '#ffb04a', 0.9)
    a.pulse(lambda: a.poly([(32, -82), (40, -64), (36, -52), (28, -52), (24, -64)], 'eo', o='ot'), seconds=0.5, low=0.7)
    a.poly([(32, -72), (36, -62), (32, -55), (28, -62)], 'ey', o='')
    a.spin(lambda: a.circ(18, -64, 2.4, 'ey', o=''), 1.4, cx=32, cy=-64)


@sym('ch_sharpshooter', 'Champion Sharpshooter', 'a long rifle, a glinting scope and a scarf in the wind', 64, '#bff24d')
def _(a):
    a.poly([(-2, -4), (-14, 12), (-24, 44), (-14, 46), (-6, 16)], 'b')
    a.poly([(0, -20), (18, -16), (26, 4), (22, 30), (0, 38)], 'x', mirror='axis')
    a.poly([(0, -10), (12, -8), (16, 24), (0, 30)], 'd', mirror='axis', o='')
    a.line([(-16, -6), (-4, -30)], 6, 'd')
    a.line([(18, -2), (10, -14)], 6, 'd')
    a.line([(8, 22), (8, 4)], 9, 'gd')
    a.line([(8, 4), (8, -78)], 5, 'md')
    a.line([(8, -76), (8, -82)], 7, 'x', edge=1.6)
    a.line([(8, -22), (8, -40)], 8, 'x')
    a.circ(8, -42, 3, 'w', o='')
    a.glow(8, -42, 9, '#ffffff', 0.8)
    a.circ(0, -10, 11, 'l')
    a.ell(0, -10, 15, 6, 'x', o='ot')
    a.poly([(-6, -4), (6, -4), (8, 4), (-8, 4)], 'b', o='ot')


@sym('ch_pathfinder', 'Champion Pathfinder', 'a winged scout with a compass rose on its back', 64, '#33d9cc')
def _(a):
    a.poly([(8, -6), (40, -24), (58, -16), (50, -4), (58, 6), (40, 12), (14, 12)], 'b', mirror='pair')
    a.line([(18, -2), (44, -16)], 2.2, 'l', mirror=True, edge=0)
    a.line([(18, 6), (46, 2)], 2.2, 'l', mirror=True, edge=0)
    a.poly([(0, -24), (12, -20), (16, 6), (10, 30), (0, 36)], 'd', mirror='axis')
    a.place(0, 12, lambda: a.poly([(0, -13), (3, -3), (13, 0), (3, 3), (0, 13), (-3, 3), (-13, 0), (-3, -3)], 'g', o='ot'))
    a.circ(0, 12, 3, 'gd', o='')
    a.circ(0, -16, 10, 'l')
    a.poly([(-9, -21), (9, -21), (8, -15), (-8, -15)], 'x', o='')
    a.circ(4, -18, 2.4, 'ec', mirror=True, o='')


@sym('ch_quartermaster', 'Champion Quartermaster', 'a pack mule pulling a cart of crates, a coin glinting on top', 67, '#ccb880')
def _(a):
    a.line([(-8, -22), (-12, -6)], 2.4, 'x', edge=0)
    a.line([(8, -22), (12, -6)], 2.4, 'x', edge=0)
    a.ell(0, -34, 9, 15, 'm')
    a.circ(0, -50, 7, 'm')
    a.poly([(4, -55), (8, -64), (9, -54)], 'md', mirror='pair', o='ot')
    a.poly([(28, -6), (34, -6), (34, 8), (28, 8)], 'x', mirror='pair', o='')
    a.poly([(28, 22), (34, 22), (34, 36), (28, 36)], 'x', mirror='pair', o='')
    a.poly([(-26, -10), (26, -10), (26, 40), (-26, 40)], 'gd')
    a.poly([(-22, -6), (-2, -6), (-2, 14), (-22, 14)], 'b', o='ot')
    a.poly([(2, -6), (22, -6), (22, 14), (2, 14)], 'd', o='ot')
    a.poly([(-22, 18), (22, 18), (22, 36), (-22, 36)], 'b', o='ot')
    a.line([(-22, 27), (22, 27)], 1.6, 'gd', edge=0)
    a.glow(12, 4, 14, '#ffe45e', 0.8)
    a.pulse(lambda: a.circ(12, 4, 5, 'g', o='ot'), seconds=1.2, low=0.5)
    a.poly([(12, -4), (13, 2), (19, 4), (13, 6), (12, 12), (11, 6), (5, 4), (11, 2)], 'w', o='')


@sym('ch_warlord', 'Champion Warlord', 'a caped commander with a crown and a banner', 80, '#d959f2')
def _(a):
    a.poly([(0, -10), (30, -6), (40, 30), (26, 46), (0, 40)], 'd', mirror='axis')
    a.poly([(0, 20), (22, 22), (26, 40), (0, 36)], 'x', mirror='axis', o='')
    a.poly([(0, -30), (20, -26), (28, -6), (22, 14), (0, 18)], 'b', mirror='axis')
    a.circ(24, -20, 10, 'g', mirror=True)
    a.circ(24, -20, 4, 'gd', mirror=True, o='')
    a.line([(30, 16), (36, -62)], 4, 'gd', edge=1.6)
    a.poly([(36, -62), (62, -56), (54, -48), (62, -40), (36, -44)], 'b')
    a.circ(46, -51, 4, 'g', o='ot')
    a.circ(0, -18, 11, 'l')
    a.poly([(0, -34), (5, -40), (8, -32), (13, -40), (15, -28), (0, -26)], 'g', mirror='axis', o='ot')
    a.circ(0, -31, 2.4, 'er', o='')


@sym('ch_frostweaver', 'Champion Frostweaver', 'a crystal sorceress under a turning snowflake', 67, '#99dbff')
def _(a):
    a.glow(0, 0, 56, '#69f0ff', 0.4)
    a.place(0, -14, lambda: a.spin(lambda: a.rep(6, lambda: a.line([(0, -17), (0, -40)], 3, 'w', edge=1.4)), 12.0))
    a.place(0, -14, lambda: a.spin(lambda: a.rep(6, lambda: a.line([(-7, -36), (0, -29), (7, -36)], 2.4, 'l', edge=1.2)), 12.0))
    a.poly([(0, -20), (18, -16), (30, 10), (26, 40), (0, 46)], 'b', mirror='axis')
    a.poly([(0, -8), (10, -6), (12, 34), (0, 40)], 'l', mirror='axis', o='')
    a.line([(-24, 10), (-28, -40)], 3.4, 'ml', edge=1.6)
    a.glow(-28, -48, 16, '#69f0ff', 0.9)
    a.poly([(-28, -60), (-22, -48), (-28, -38), (-34, -48)], 'ec', o='ot')
    a.circ(0, -14, 11, 'w')
    a.poly([(0, -36), (4, -27), (9, -32), (8, -21), (0, -19)], 'ec', mirror='axis', o='ot')
    a.circ(4, -14, 2, 'ec', mirror=True, o='')


@sym('ch_stormcaller', 'Champion Stormcaller', 'a cloaked caller with a storm held in its hands', 70, '#9e80ff')
def _(a):
    a.poly([(0, -24), (24, -18), (38, 12), (30, 42), (0, 48)], 'd', mirror='axis')
    a.poly([(30, 42), (36, 30), (26, 34), (30, 22), (20, 30)], 'b', mirror='pair', o='')
    a.line([(18, -10), (12, -34)], 6, 'd', mirror=True)
    a.circ(0, -14, 12, 'b')
    a.ell(0, -16, 8, 7, 'k', o='')
    a.circ(3, -17, 1.8, 'ec', mirror=True, o='')
    a.glow(0, -48, 30, '#d9a6ff', 0.9)
    a.circ(0, -48, 13, 'x')
    a.circ(0, -48, 8, 'ep', o='')
    a.place(0, -48, lambda: a.spin(lambda: a.rep(3, lambda: a.line([(0, -6), (4, -11), (-2, -14), (3, -20)], 2, 'w', edge=0)), 1.1))
    a.circ(0, -48, 3, 'w', o='')


@sym('ch_earthshaker', 'Champion Earthshaker', 'a stone golem with a hammer, fire in its cracks', 90, '#b85733')
def _(a):
    a.line([(34, -6), (44, -56)], 7, 'gd', edge=2)
    a.poly([(28, -58), (58, -66), (62, -50), (34, -42)], 'm')
    a.poly([(34, -55), (56, -61), (58, -54), (36, -48)], 'ml', o='')
    a.poly([(0, -30), (30, -28), (44, -6), (42, 24), (26, 42), (0, 44)], 'b', mirror='axis')
    a.poly([(0, 6), (20, 8), (24, 30), (0, 36)], 'd', mirror='axis', o='')
    a.line([(-24, -10), (-12, 0), (-18, 14)], 2.4, 'eo', edge=0)
    a.line([(16, 18), (6, 26), (10, 36)], 2.4, 'eo', edge=0)
    a.glow(0, 4, 22, '#ffa83a', 0.5)
    a.circ(36, -18, 12, 'd', mirror=True)
    a.circ(-30, -16, 3, 'x', o='')
    a.poly([(0, -36), (11, -32), (11, -20), (0, -17)], 'd', mirror='axis')
    a.circ(5, -27, 2.6, 'eo', mirror=True, o='')


@sym('ch_phoenix', 'Champion Phoenix', 'a firebird, wings spread', 77, '#ffd140')
def _(a):
    a.glow(0, 0, 60, '#ffb04a', 0.45)
    a.poly([(0, 16), (12, 40), (6, 60), (0, 50)], 'eo', mirror='axis')
    a.poly([(6, 18), (22, 44), (16, 54), (6, 36)], 'er', mirror='pair')
    a.poly([(6, -8), (30, -24), (60, -30), (50, -16), (62, -8), (44, 2), (52, 12), (28, 10), (8, 8)], 'b', mirror='pair')
    a.poly([(40, -22), (60, -30), (50, -16), (62, -8), (44, 2), (36, -8)], 'eo', mirror='pair', o='')
    a.pulse(lambda: a.poly([(52, -24), (60, -30), (56, -20)], 'ey', mirror='pair', o=''), seconds=0.6, low=0.3)
    a.ell(0, 0, 11, 24, 'b')
    a.ell(0, 4, 5, 14, 'l', o='')
    a.circ(0, -26, 9, 'b')
    a.poly([(0, -44), (4, -33), (-4, -33)], 'eo', o='ot')
    a.poly([(0, -30), (6, -42), (2, -26)], 'er', mirror='pair', o='')
    a.circ(4, -28, 1.8, 'k', mirror=True, o='')


@sym('ch_emberlord', 'Champion Emberlord', 'a horned lord of fire with a molten heart, flames on its shoulders', 70, '#ff591a')
def _(a):
    a.pulse(lambda: a.glow(0, 0, 48, '#ff801a', 0.5), 0.7, 0.4)
    a.poly([(0, -30), (22, -22), (34, 2), (28, 30), (0, 40)], 'b', mirror='axis')
    a.poly([(0, -22), (14, -16), (20, 6), (0, 14)], 'd', mirror='axis', o='')
    a.circ(0, 4, 11, 'eo', o='ot')
    a.pulse(lambda: a.circ(0, 4, 6, 'ey', o=''), 0.4, 0.3)
    a.poly([(8, -32), (16, -54), (18, -34)], 'bone', mirror='pair')
    a.circ(0, -34, 13, 'd')
    a.circ(5, -36, 2.6, 'ey', mirror=True, o='')
    a.rep(2, lambda: a.place(30, -14, lambda: a.pulse(lambda: a.poly([(-7, 0), (0, -22), (7, 0)], 'eo', o='ot'), 0.5, 0.4)), start=0, step=180)


@sym('ch_chronomancer', 'Champion Chronomancer', 'a robed sage under a great clock face whose hands turn', 67, '#f2d973')
def _(a):
    a.alpha(0.5, lambda: a.ring(0, 0, 46, 3, 'l', edge=0))
    a.rep(12, lambda: a.line([(0, -46), (0, -40)], 2.4, 'l', edge=0))
    a.poly([(0, -20), (20, -10), (26, 30), (0, 38)], 'b', mirror='axis')
    a.circ(0, -24, 12, 'd')
    a.circ(0, -4, 16, 'w', o='ot')
    a.spin(lambda: a.line([(0, -4), (0, -16)], 2.6, 'k', edge=0), 6.0)
    a.spin(lambda: a.line([(0, -4), (9, -4)], 2.2, 'k', edge=0), 60.0)
    a.circ(0, -4, 2.4, 'g', o='')
    a.circ(5, -26, 2.2, 'ey', mirror=True, o='')


@sym('ch_venomancer', 'Champion Venomancer', 'a hooded plague-brewer with a bubbling green flask', 64, '#73e640')
def _(a):
    a.pulse(lambda: a.glow(0, 0, 44, '#8dff6a', 0.4), 1.2, 0.3)
    a.poly([(0, -24), (20, -14), (26, 32), (0, 38)], 'b', mirror='axis')
    a.poly([(0, -44), (16, -34), (16, -14), (0, -10)], 'x', mirror='axis')
    a.circ(6, -26, 2.6, 'eg', mirror=True, o='')
    a.circ(0, 12, 11, 'eg', o='ot')
    a.poly([(-4, -2), (4, -2), (4, 4), (-4, 4)], 'md', o='ot')
    for x, y, r in ((-6, -10, 3), (4, -16, 2.4), (-1, -22, 2)):
        a.pulse(lambda x=x, y=y, r=r: a.circ(x, y, r, 'eg', o='ot'), 0.8, 0.1)


@sym('ch_gravitron', 'Champion Gravitron', 'a dark core with rings of force turning round it', 74, '#8c73f2')
def _(a):
    a.pulse(lambda: a.glow(0, 0, 50, '#8c73f2', 0.45), 1.0, 0.4)
    a.spin(lambda: a.ell(0, 0, 44, 16, 'l', rot=20, o='ot'), 3.0)
    a.spin(lambda: a.ell(0, 0, 44, 16, 'l', rot=-40, o='ot'), 4.5, back=True)
    a.circ(0, 0, 24, 'b')
    a.circ(0, 0, 15, 'x', o='')
    a.pulse(lambda: a.circ(0, 0, 7, 'ep', o=''), 0.6, 0.2)
