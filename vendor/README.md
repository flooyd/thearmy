# vendor

`miniquad` is the graphics library under macroquad, copied from crates.io (0.4.11, MIT or Apache-2.0, see its LICENSE files)
with one change, in `src/graphics/gl.rs`: `delete_texture` does not free the texture at once but when the frame is committed.

Why: macroquad draws text from glyph atlases, and when an atlas fills up it makes a bigger texture and deletes the old one. miniquad
remembers which texture is bound, and did not forget a deleted one, so when the game had drawn part of a frame (it does, to put the
circles and ships in between) and the atlas that was bound then grew, the new texture was given the deleted one's name, was never bound
or filled, and every glyph drawn from it came out as a black block. Text drawn earlier in the same frame, still waiting to be drawn,
was run with the wrong texture as well.

To check a newer miniquad, take this directory out of `[patch.crates-io]` in `Cargo.toml` and see whether the change is in it.
