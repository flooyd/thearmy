"""The babies the destroyed enemies drop: little stick-figure toddlers in the bold-outline style of the runner, in the colour of what the army picks
up (the game tints them with pickup_color, so the colour-blind setting applies), in three poses, each baby getting one by where it lies. units.rs
(`baby_figures`) draws them rocking from foot to foot, popping up as they drop and rocking harder and fading at the end of their time; where one is too
small on the screen to make out, or with simple shapes, the game draws the plain disc instead."""

from dsl import sym


def toddler(a, arms):
    """A toddler seen from the front, facing up the page: stubby legs and feet, its arms (drawn by `arms`), a round body and a big round head."""
    a.line([(-3, 12), (-12, 33)], 8, 'b', edge=2.2)
    a.line([(3, 12), (12, 33)], 8, 'b', edge=2.2)
    a.ell(-13, 36, 6.5, 4, 'b')
    a.ell(13, 36, 6.5, 4, 'b')
    arms()
    a.poly([(0, -10), (11, -6), (13, 12), (0, 18)], 'b', mirror='axis')
    a.circ(0, -27, 17, 'b')
    a.circ(-6, -28, 2.6, 'k', o='')
    a.circ(6, -28, 2.6, 'k', o='')
    a.ell(0, -19.5, 3.6, 1.8, 'x', o='')
    a.circ(-11, -22, 3, 'l', o='')
    a.circ(11, -22, 3, 'l', o='')
    a.line([(0, -44), (4, -50)], 2.6, 'b', edge=1.4)


@sym('baby_1', 'Baby I', 'a toddler with its arms up, wanting to be picked up', 29, '#4ce64c')
def _(a):
    toddler(a, lambda: (a.line([(-7, -4), (-21, -22)], 6, 'b'), a.line([(7, -4), (21, -22)], 6, 'b')))


@sym('baby_2', 'Baby II', 'a toddler with its arms out, keeping its balance', 29, '#4ce64c')
def _(a):
    toddler(a, lambda: (a.line([(-8, -2), (-25, 2)], 6, 'b'), a.line([(8, -2), (25, 2)], 6, 'b')))


@sym('baby_3', 'Baby III', 'a toddler hugging a little rattle', 29, '#4ce64c')
def _(a):
    toddler(a, lambda: (a.line([(-8, -2), (-12, 10), (-3, 8)], 6, 'b'), a.line([(8, -2), (12, 10), (3, 8)], 6, 'b'), a.circ(0, 6, 5, 'ey', o='ot')))
