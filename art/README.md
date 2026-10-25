# Enemy, boss and champion art

Drawings for the 23 enemies, the 8 bosses (the 7 and the Sovereign), the overlays that go on top of them (elite marks, a captain's crown, ice, a shield) and the
champions (`champions.py`, named "Champion <name>"; the first seven so far), made to be drawn the way the ships are: flat colours, a black outline round every
part, nothing the instanced shader can't also do. The game draws them through `src/creatures.rs` from `src/creature_art.rs`, which `shader.py` writes:

    python art/shader.py                     writes src/creature_art.rs from the drawings (never edit that file by hand)
    python art/shader.py --check             says whether it is up to date
    python art/glpreview.py <page.html>      a page that draws them with the game's own shaders in WebGL (--only "Grunt,Champion Medic" for some), and says if one
                                             would not compile; a headless browser can screenshot it:
    msedge --headless=new --use-angle=swiftshader --enable-unsafe-swiftshader --window-size=1600,1000 --screenshot=<shot.png> file:///<page.html>

    python art/build.py                      writes sprites.svg (every drawing as a <symbol>) and svg/<id>.svg (each on its own, in its colour in the game)
    python art/build.py --preview <dir>      and <dir>/preview.html, a page to look at them in (--only a,b,c for some)
    python art/mockups.py <dir>              the artboards of the Design canvas the mock-ups were viewed in

The drawings are in `enemies.py`, `bosses.py` and `marks.py`, written with the little kit in `dsl.py`.

## Units

Every drawing faces up (-y) and is on a square that reaches 50 from the middle: that square is the enemy's hitbox (`size` pixels along a side). Horns, crowns
and spikes may go out of it, to 80. So a drawing is `size * 1.6` pixels across.

Colours are the enemy's own: `b` is its colour (the instance colour in the game), `d`, `l` and `x` are that darker, lighter and nearly black, and the rest (eyes,
steel, bone, gold) are fixed. One drawing is any colour; that is where most of the variety is meant to come from.

## What the shader needs

The kit only has what a signed distance function has: polygons (a half drawn and mirrored with `abs(x)`, or a pair), circles, ellipses, capsules (a stroked line
with round ends), rings and arcs, a rotated repeat (`rep`), and a soft glow. A part's outline is its distance field grown by a few units, drawn first. So each
creature is a branch of the fragment shader like the ship's, picked by a new shape id on the `Disc` beside `nose`, with the parts as constants pasted in by a
function like `art_constants()`. Things worth knowing before doing it:

- parts are drawn back to front, so a drawing is a list of `(sdf, colour)` to composite in order; the biggest are the bosses, at about 60 parts each
- the spinning and pulsing parts (`spin`, `pulse`) are a time uniform and a rotation of the point, nothing more
- like a unit, a creature smaller than a few pixels should fall back to a plain square or circle of its colour; at that size the outline is all there is
- `variant` parts (`--v-b`) are the hurt look of a boss, for its last stand
- the ship and the player are here only so the mock-ups can put them in the same picture: the game draws those already
