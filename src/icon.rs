//! the picture in the corner of the window and on the task bar: the player with some of the units round them, in the shapes and colours they have in the game (the
//! figure is `Runner`'s own pose and the ships are drawn from the corners `discs` draws them from). it is drawn when the game starts, from a few shapes and a few
//! samples to a pixel, rather than kept as a file to carry beside the game. the window system asks for it in three sizes

use crate::prelude::*;
use macroquad::miniquad::conf::Icon;
use std::f32::consts::FRAC_PI_2;

const DESIGN: f32 = 64.0; //it is laid out on a square this many units across, and drawn from that at each size
const CORNER: f32 = 14.0; //how round the corners of the tile are
const FIGURE_SCALE: f32 = 1.2; //the figure is this many times as big as it is in the game (where a unit's radius is 16)
const FIGURE_HIP: Vec2 = Vec2::new(32.0, 35.5);
const LIMB_BOLD: f32 = 1.3; //the limbs are this much thicker than the game's, which are thin for a picture this small
const GLOW: f32 = 29.0; //how far the light behind the army reaches
//the ships round the player: where, how big (a radius) and how far they lean from straight up
const SHIPS: [(Vec2, f32, f32); 4] = [
    (Vec2::new(12.0, 19.0), 9.0, -0.22),
    (Vec2::new(52.0, 19.0), 9.0, 0.22),
    (Vec2::new(10.0, 44.0), 7.5, -0.12),
    (Vec2::new(54.0, 44.0), 7.5, 0.12),
];

#[derive(Clone, Copy)]
enum Shape {
    Circle(Vec2, f32),
    Stroke(Vec2, Vec2, f32), //a line with round ends, this far out to each side of it
    Triangle(Vec2, Vec2, Vec2),
    Tile(f32, f32), //a square with round corners, in the middle of the picture: how far it reaches from the middle, and how round the corners are
}

impl Shape {
    fn covers(&self, point: Vec2) -> bool {
        match *self {
            Shape::Circle(at, radius) => point.distance_squared(at) <= radius * radius,
            Shape::Stroke(from, to, radius) => {
                let along = to - from;
                let share = if along == Vec2::ZERO { 0.0 } else { ((point - from).dot(along) / along.length_squared()).clamp(0.0, 1.0) };
                point.distance_squared(from + along * share) <= radius * radius
            }
            Shape::Triangle(a, b, c) => {
                let side = |from: Vec2, to: Vec2| (to - from).perp_dot(point - from);
                let (x, y, z) = (side(a, b), side(b, c), side(c, a));
                (x >= 0.0 && y >= 0.0 && z >= 0.0) || (x <= 0.0 && y <= 0.0 && z <= 0.0)
            }
            Shape::Tile(reach, corner) => {
                let past = (point - Vec2::splat(DESIGN / 2.0)).abs() - Vec2::splat(reach - corner);
                past.max(Vec2::ZERO).length() <= corner
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Fill {
    Flat(Color),
    Down(Color, Color),      //from the top of the picture to the bottom
    Glow(Vec2, f32, Color), //this colour in the middle, fading to nothing this far out
}

impl Fill {
    fn color(&self, point: Vec2) -> Color {
        match *self {
            Fill::Flat(color) => color,
            Fill::Down(top, bottom) => lerp_color(top, bottom, (point.y / DESIGN).clamp(0.0, 1.0)),
            Fill::Glow(at, reach, color) => Color::new(color.r, color.g, color.b, color.a * (1.0 - point.distance(at) / reach).clamp(0.0, 1.0).powi(2)),
        }
    }
}

type Layer = (Shape, Fill);

//one ship, as `draw_ship` draws one the slow way: the flame, the black edge, the hull and the glass
fn push_ship(layers: &mut Vec<Layer>, bold: f32, at: Vec2, radius: f32, lean: f32, color: Color) {
    let nose = Vec2::from_angle(lean - FRAC_PI_2);
    let place = |art: Vec2| at + art_to_world(art, nose) * radius;
    let [front, right, back, left] = dart_corners();
    let middle = (front + back) / 2.0;
    let swell = |corner: Vec2| middle + (corner - middle) * (1.0 + 0.12 * bold);
    let mut triangle = |a: Vec2, b: Vec2, c: Vec2, fill: Color| layers.push((Shape::Triangle(place(a), place(b), place(c)), Fill::Flat(fill)));
    triangle(FLAME_BASE, Vec2::new(-FLAME_BASE.x, FLAME_BASE.y), FLAME_TIP, Color::new(FLAME[0], FLAME[1], FLAME[2], 1.0));
    triangle(swell(front), swell(right), swell(back), BLACK);
    triangle(swell(front), swell(back), swell(left), BLACK);
    triangle(front, right, back, color);
    triangle(front, back, left, color);
    layers.push((Shape::Circle(place(COCKPIT_AT), COCKPIT_SIZE.x * radius / SHIP_SPAN * 1.4 * bold), Fill::Flat(Color::new(GLASS[0], GLASS[1], GLASS[2], 1.0))));
}

//the player, as `draw_runner` draws them: the black edge round every part, and then the part
fn push_figure(layers: &mut Vec<Layer>, bold: f32, color: Color) {
    //(standing: with the arms by the sides it is a figure that can still be made out at 16 pixels, which one that is running, with its arms across it, is not)
    let runner = Runner::new();
    let place = |point: Vec2| FIGURE_HIP + point * FIGURE_SCALE;
    let thick = FIGURE_SCALE * bold * LIMB_BOLD;
    for part in runner.pose() {
        match part {
            RunnerPart::Shadow { .. } => {}
            RunnerPart::Limb { points, tone, width } => {
                for (radius, fill) in [((width / 2.0 + OUTLINE) * thick, EDGE), (width / 2.0 * thick, tone_color(tone, color))] {
                    for pair in points.windows(2).filter(|pair| pair[0] != pair[1]) {
                        layers.push((Shape::Stroke(place(pair[0]), place(pair[1]), radius), Fill::Flat(fill)));
                    }
                }
            }
            RunnerPart::Head { at, tone, eyes } => {
                //(the head grows less than the limbs do when it is small, or it is most of the figure)
                let round = FIGURE_SCALE * 1.12 * (1.0 + (bold - 1.0) * 0.6);
                layers.push((Shape::Circle(place(at), (HEAD_RADIUS + OUTLINE) * round), Fill::Flat(EDGE)));
                layers.push((Shape::Circle(place(at), HEAD_RADIUS * round), Fill::Flat(tone_color(tone, color))));
                for eye in eyes.into_iter().flatten() {
                    layers.push((Shape::Circle(place(eye), EYE_RADIUS * round), Fill::Flat(EDGE)));
                }
            }
        }
    }
}

//everything in the picture at a size, from the back to the front
fn scene(size: usize) -> Vec<Layer> {
    //(a small picture needs its lines bolder than a big one to be made out at all)
    let bold = 1.0 + (DESIGN / size as f32 - 1.0) * 0.2;
    let blue = PLAYER_COLORS[0];
    let middle = Vec2::splat(DESIGN / 2.0);
    let rim = 2.0 * bold;
    let mut layers: Vec<Layer> = vec![
        (Shape::Tile(DESIGN / 2.0, CORNER), Fill::Flat(Color::new(CYAN.r, CYAN.g, CYAN.b, 0.7))),
        (Shape::Tile(DESIGN / 2.0 - rim, CORNER - rim), Fill::Down(Color::new(0.05, 0.11, 0.2, 1.0), Color::new(0.015, 0.035, 0.08, 1.0))),
        (Shape::Circle(middle, GLOW), Fill::Glow(middle, GLOW, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.25))),
    ];
    for (at, radius, lean) in SHIPS {
        push_ship(&mut layers, bold, at, radius, lean, blue);
    }
    push_figure(&mut layers, bold, blue);
    layers
}

//what is seen at a point: the layers laid one over the next (as red, green and blue already multiplied by the alpha, and then the alpha)
fn sample(layers: &[Layer], point: Vec2) -> [f32; 4] {
    let mut seen = [0.0f32; 4];
    for (shape, fill) in layers {
        if shape.covers(point) {
            let over = fill.color(point);
            let keep = 1.0 - over.a;
            seen = [over.r * over.a + seen[0] * keep, over.g * over.a + seen[1] * keep, over.b * over.a + seen[2] * keep, over.a + seen[3] * keep];
        }
    }
    seen
}

//the picture at a size across, as rows of red, green, blue and alpha bytes. each pixel is the average of a grid of points in it, which is what smooths the edges
fn render(size: usize) -> Vec<u8> {
    let layers = scene(size);
    let grid = if size <= 16 { 8 } else if size <= 32 { 6 } else { 4 };
    let share = 1.0 / (grid * grid) as f32;
    let mut bytes = Vec::with_capacity(size * size * 4);
    for row in 0..size {
        for column in 0..size {
            let mut total = [0.0f32; 4];
            for down in 0..grid {
                for across in 0..grid {
                    let in_pixel = Vec2::new(column as f32 + (across as f32 + 0.5) / grid as f32, row as f32 + (down as f32 + 0.5) / grid as f32);
                    let seen = sample(&layers, in_pixel * (DESIGN / size as f32));
                    for (sum, value) in total.iter_mut().zip(seen) {
                        *sum += value * share;
                    }
                }
            }
            let alpha = total[3];
            let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
            let colour = |k: usize| if alpha > 0.0 { byte(total[k] / alpha) } else { 0 };
            bytes.extend_from_slice(&[colour(0), colour(1), colour(2), byte(alpha)]);
        }
    }
    bytes
}

fn bytes_of<const N: usize>(size: usize) -> [u8; N] {
    render(size).try_into().unwrap_or_else(|_| panic!("a picture {size} pixels across is not {N} bytes"))
}

//the icon of the window, which the task bar and the corner of the window show
pub(crate) fn window_icon() -> Icon {
    Icon { small: bytes_of(16), medium: bytes_of(32), big: bytes_of(64) }
}

//the picture at a size, for the tests to look at
#[cfg(test)]
pub(crate) fn picture(size: usize) -> Vec<u8> {
    render(size)
}
