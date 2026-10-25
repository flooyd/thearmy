//! the shapes a warning on the ground can have (`Shape`): a circle, a ring with a safe middle, a lane or beam, a cone and a box that sweeps. what is inside
//! each, how far outside a point is and the quickest way out, and how each is drawn: the countdown fill (which grows out from the middle of a circle, closes
//! in on a ring, runs along a lane or a box and opens out across a cone), a hatching of its own for each kind of a boss's attack so that colour is not the
//! only way to tell them apart, the outline on the map, an arrow at the edge of the screen for a big one that is off it, and the ticking that quickens as the
//! worst one the army is in comes due. hazards put down together (a fan of beams) share a `group` and are warned of as one

use crate::prelude::*;
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Shape {
    Circle,                                      //everything within the radius of the position
    Ring { inner: f32 },                         //between `inner` and the radius: the middle is safe, stand close
    Line { dir: Vec2, length: f32, width: f32 }, //a lane or a beam from the position out along `dir` (a unit vector)
    Cone { dir: Vec2, arc: f32 },                //out to the radius and `arc` radians across, round `dir`: get behind it or to its side
    Rect { dir: Vec2, length: f32, width: f32 }, //a box round the position, `length` along `dir` and `width` across, that sweeps along `dir`
}

impl Shape {
    //how far from the position the shape reaches at most: its radius for a circle, a ring and a cone, and for a lane or a box what its size makes it
    pub(crate) fn reach(self, radius: f32) -> f32 {
        match self {
            Shape::Line { length, width, .. } => length.hypot(width / 2.0),
            Shape::Rect { length, width, .. } => (length / 2.0).hypot(width / 2.0),
            _ => radius,
        }
    }

    //the word for it on the screen, after its colour, and what to do about it: none for a circle, which is named by its colour alone
    pub(crate) fn words(self) -> Option<(&'static str, &'static str)> {
        match self {
            Shape::Circle => None,
            Shape::Ring { .. } => Some(("RING", "STAND CLOSE")),
            Shape::Line { .. } => Some(("BEAM", "STEP ASIDE")),
            Shape::Cone { .. } => Some(("CONE", "GET BEHIND IT")),
            Shape::Rect { .. } => Some(("LANE", "STEP ASIDE")),
        }
    }

    //what to do about one, in a few words, when it is not one of a boss's (which say for themselves)
    pub(crate) fn advice(self) -> &'static str {
        match self {
            Shape::Circle => "walk clear",
            Shape::Ring { .. } => "stand close to its middle",
            Shape::Line { .. } | Shape::Rect { .. } => "step out of the lane",
            Shape::Cone { .. } => "get behind it or to its side",
        }
    }
}

//whether a point is inside a shape at `position` reaching `radius`
pub(crate) fn inside(position: Vec2, radius: f32, shape: Shape, at: Vec2) -> bool {
    let d = at - position;
    match shape {
        Shape::Circle => d.length_squared() <= radius * radius,
        Shape::Ring { inner } => {
            let far = d.length_squared();
            far <= radius * radius && far >= inner * inner
        }
        Shape::Line { dir, length, width } => {
            let along = d.dot(dir);
            along >= 0.0 && along <= length && dir.perp_dot(d).abs() <= width / 2.0
        }
        Shape::Rect { dir, length, width } => d.dot(dir).abs() <= length / 2.0 && dir.perp_dot(d).abs() <= width / 2.0,
        Shape::Cone { dir, arc } => {
            let far = d.length_squared();
            far <= radius * radius && (far == 0.0 || d.dot(dir) >= far.sqrt() * (arc / 2.0).cos())
        }
    }
}

//how far a point is outside a shape (0 inside it): exact for all but a cone, where it is near enough
pub(crate) fn outside_by(position: Vec2, radius: f32, shape: Shape, at: Vec2) -> f32 {
    let d = at - position;
    match shape {
        Shape::Circle => (d.length() - radius).max(0.0),
        Shape::Ring { inner } => {
            let far = d.length();
            (far - radius).max(inner - far).max(0.0)
        }
        Shape::Line { dir, length, width } => {
            let along = d.dot(dir);
            (along - along.clamp(0.0, length)).abs().hypot((dir.perp_dot(d).abs() - width / 2.0).max(0.0))
        }
        Shape::Rect { dir, length, width } => (d.dot(dir).abs() - length / 2.0).max(0.0).hypot((dir.perp_dot(d).abs() - width / 2.0).max(0.0)),
        Shape::Cone { dir, arc } => {
            let far = d.length();
            let radial = (far - radius).max(0.0);
            let off = if far > 0.0 { dir.angle_between(d).abs() - arc / 2.0 } else { 0.0 };
            if off <= 0.0 {
                radial
            } else if off < PI / 2.0 {
                (far * off.sin()).max(radial)
            } else {
                far
            }
        }
    }
}

//the quickest way out of a shape from a point in it, as a unit vector: out from the middle of a circle, in to the middle of a ring (or out, if its edge is
//nearer), sideways out of a lane or a box (or past an end, if that is nearer), and out of the nearer side of a cone (or past its end)
#[cfg(test)]
pub(crate) fn way_out(position: Vec2, radius: f32, shape: Shape, at: Vec2) -> Vec2 {
    let d = at - position;
    let away = d.try_normalize().unwrap_or(Vec2::X);
    match shape {
        Shape::Circle => away,
        Shape::Ring { inner } => {
            let far = d.length();
            if far - inner < radius - far { -away } else { away }
        }
        Shape::Line { dir, length, width } | Shape::Rect { dir, length, width } => {
            let across = dir.perp_dot(d);
            let side = if across >= 0.0 { dir.perp() } else { -dir.perp() };
            let to_side = width / 2.0 - across.abs();
            let along = d.dot(dir);
            //(how far to the end it is nearer, and which way that is)
            let (to_end, end) = match shape {
                Shape::Line { .. } => {
                    if along < length / 2.0 { (along, -dir) } else { (length - along, dir) }
                }
                _ => (length / 2.0 - along.abs(), if along >= 0.0 { dir } else { -dir }),
            };
            if to_end < to_side { end } else { side }
        }
        Shape::Cone { dir, arc } => {
            let far = d.length();
            let angle = dir.angle_between(d);
            let to_side = far * (arc / 2.0 - angle.abs()).clamp(0.0, PI / 2.0).sin();
            if radius - far < to_side {
                away
            } else {
                away.perp() * if angle >= 0.0 { 1.0 } else { -1.0 }
            }
        }
    }
}

//the middle of a shape, where an arrow at the edge of the screen points
pub(crate) fn shape_middle(position: Vec2, shape: Shape) -> Vec2 {
    match shape {
        Shape::Line { dir, length, .. } => position + dir * length / 2.0,
        _ => position,
    }
}

//the corners of a lane, a box or a cone (its arc in steps) in order round it; a circle and a ring have none. `filled` is the share of it the countdown has
//filled: a lane fills from where it starts, a box from its back edge along `dir`, and a cone out from its point
pub(crate) fn shape_outline(position: Vec2, radius: f32, shape: Shape, filled: f32) -> Vec<Vec2> {
    let filled = filled.clamp(0.0, 1.0);
    match shape {
        Shape::Line { dir, length, width } => {
            let (side, end) = (dir.perp() * width / 2.0, position + dir * length * filled);
            vec![position + side, end + side, end - side, position - side]
        }
        Shape::Rect { dir, length, width } => {
            let (back, side) = (position - dir * length / 2.0, dir.perp() * width / 2.0);
            let front = back + dir * length * filled;
            vec![back - side, front - side, front + side, back + side]
        }
        Shape::Cone { dir, arc } => {
            let steps = ((arc / 0.12).ceil() as usize).clamp(4, 64);
            let first = dir.y.atan2(dir.x) - arc / 2.0;
            let reach = radius * filled;
            std::iter::once(position).chain((0..=steps).map(|k| position + Vec2::from_angle(first + arc * k as f32 / steps as f32) * reach)).collect()
        }
        _ => Vec::new(),
    }
}

//points spread over a shape, each with how far round it is to cover: where a hazard going off throws its sparks and leaves its scorch marks
pub(crate) fn spots(position: Vec2, radius: f32, shape: Shape) -> Vec<(Vec2, f32)> {
    match shape {
        Shape::Circle => vec![(position, radius)],
        Shape::Ring { inner } => {
            let (middle, thick) = ((inner + radius) / 2.0, (radius - inner) / 2.0);
            let count = ((TAU * middle / (thick * 2.0).max(1.0)).ceil() as usize).clamp(4, 12);
            (0..count).map(|k| (position + Vec2::from_angle(TAU * k as f32 / count as f32) * middle, thick)).collect()
        }
        Shape::Line { dir, length, width } | Shape::Rect { dir, length, width } => {
            let start = if matches!(shape, Shape::Line { .. }) { position } else { position - dir * length / 2.0 };
            let count = ((length / width.max(1.0)).ceil() as usize).clamp(1, 12);
            (0..count).map(|k| (start + dir * length * (k as f32 + 0.5) / count as f32, (width / 2.0).max(length / count as f32 / 2.0))).collect()
        }
        Shape::Cone { dir, arc } => (0..4)
            .map(|k| {
                let out = (k as f32 + 0.5) / 4.0;
                (position + dir * radius * out, (radius * out * (arc / 2.0).min(PI / 2.0).sin()).max(radius / 8.0))
            })
            .collect(),
    }
}

//the hatching each kind of a boss's attack is drawn with, so that it can be told from the others by more than its colour: lines at an angle in degrees (to
//the world for a circle and a ring, to the way it faces for the rest), crossed or not, and how many pixels apart on the screen. the other hazards have none
pub(crate) fn hatch_of(circle: Option<BossCircle>) -> Option<(f32, bool, f32)> {
    Some(match circle? {
        BossCircle::Slam => (45.0, false, 22.0),
        BossCircle::Meteor => (45.0, true, 26.0),
        BossCircle::Nova => (135.0, false, 22.0),
        BossCircle::Pull => (0.0, false, 20.0),
        BossCircle::Shift => (90.0, false, 22.0),
        BossCircle::Split => (0.0, true, 26.0),
        BossCircle::Piston => (90.0, false, 16.0),
        BossCircle::Beam => (90.0, false, 12.0),
        BossCircle::Cleave => (0.0, false, 18.0),
        BossCircle::GravityRing => (45.0, true, 34.0),
        BossCircle::Acid => (135.0, true, 24.0),
        BossCircle::Rush => (0.0, false, 14.0),
        BossCircle::Whirlwind => (45.0, false, 30.0),
        BossCircle::Leap => (135.0, false, 32.0),
        BossCircle::Twin => (90.0, true, 28.0),
        BossCircle::Barrage => (90.0, false, 26.0),
        BossCircle::Sweep => (0.0, true, 18.0),
        BossCircle::Judgement => (45.0, true, 18.0),
    })
}

//the stretches of a line through `from` along `along` that are inside a shape, as (from, to) pairs
pub(crate) fn clip_line(position: Vec2, radius: f32, shape: Shape, from: Vec2, along: Vec2) -> Vec<(Vec2, Vec2)> {
    let chord = |r: f32| -> Option<(f32, f32)> {
        //(where the line crosses a circle of radius r round the position)
        let d = from - position;
        let (b, c) = (d.dot(along), d.length_squared() - r * r);
        let root = b * b - c;
        (root > 0.0).then(|| (-b - root.sqrt(), -b + root.sqrt()))
    };
    let at = |t: f32| from + along * t;
    match shape {
        Shape::Circle => chord(radius).map(|(a, b)| vec![(at(a), at(b))]).unwrap_or_default(),
        Shape::Ring { inner } => match (chord(radius), chord(inner)) {
            (Some((a, b)), Some((c, d))) => vec![(at(a), at(c)), (at(d), at(b))],
            (Some((a, b)), None) => vec![(at(a), at(b))],
            _ => Vec::new(),
        },
        Shape::Cone { arc, .. } if arc > PI => Vec::new(),
        _ => {
            //a convex polygon: the line kept on the inside of each edge in turn
            let corners = shape_outline(position, radius, shape, 1.0);
            let middle = corners.iter().copied().sum::<Vec2>() / corners.len() as f32;
            let (mut low, mut high) = (-1.0e9f32, 1.0e9f32);
            for k in 0..corners.len() {
                let (p, q) = (corners[k], corners[(k + 1) % corners.len()]);
                let mut normal = (q - p).perp();
                if normal.dot(middle - p) < 0.0 {
                    normal = -normal;
                }
                let (start, rate) = (normal.dot(from - p), normal.dot(along));
                if rate.abs() < 1e-6 {
                    if start < 0.0 {
                        return Vec::new();
                    }
                } else if rate > 0.0 {
                    low = low.max(-start / rate);
                } else {
                    high = high.min(-start / rate);
                }
            }
            if low < high { vec![(at(low), at(high))] } else { Vec::new() }
        }
    }
}

//a shape's hatching: lines `gap` pixels apart on the screen across all of it, drifting slowly so it is alive. at most HATCH_MOST_LINES each way
fn draw_hatching(position: Vec2, radius: f32, shape: Shape, (angle, crossed, gap): (f32, bool, f32), zoom: f32, time: f32, color: Color) {
    let facing = match shape {
        Shape::Line { dir, .. } | Shape::Rect { dir, .. } | Shape::Cone { dir, .. } => dir.y.atan2(dir.x),
        _ => 0.0,
    };
    let middle = match shape {
        Shape::Line { .. } | Shape::Cone { .. } => {
            let corners = shape_outline(position, radius, shape, 1.0);
            corners.iter().copied().sum::<Vec2>() / corners.len().max(1) as f32
        }
        _ => position,
    };
    let reach = shape.reach(radius) + radius * 0.01;
    let gap = (gap / zoom.max(0.01)).max(reach * 2.0 / HATCH_MOST_LINES);
    let thick = 2.0 / zoom.max(0.01);
    for turn in if crossed { [0.0, 90.0].as_slice() } else { [0.0].as_slice() } {
        let along = Vec2::from_angle(facing + (angle + turn).to_radians());
        let across = along.perp();
        let drift = (time * 0.35).fract() * gap;
        let mut offset = -reach + drift;
        while offset <= reach {
            for (a, b) in clip_line(position, radius, shape, middle + across * offset, along) {
                draw_line(a.x, a.y, b.x, b.y, thick, color);
            }
            offset += gap;
        }
    }
}

//a band round a point, from `inner` out to `outer`
fn fill_band(at: Vec2, inner: f32, outer: f32, sides: usize, color: Color) {
    for k in 0..sides {
        let (a, b) = (Vec2::from_angle(TAU * k as f32 / sides as f32), Vec2::from_angle(TAU * (k + 1) as f32 / sides as f32));
        draw_triangle(at + a * inner, at + a * outer, at + b * outer, color);
        draw_triangle(at + a * inner, at + b * outer, at + b * inner, color);
    }
}

//a convex polygon filled: a fan from its first corner
fn fill_polygon(points: &[Vec2], color: Color) {
    for k in 1..points.len().saturating_sub(1) {
        draw_triangle(points[0], points[k], points[k + 1], color);
    }
}

fn edge_polygon(points: &[Vec2], thick: f32, color: Color) {
    for k in 0..points.len() {
        let (a, b) = (points[k], points[(k + 1) % points.len()]);
        draw_line(a.x, a.y, b.x, b.y, thick, color);
    }
}

//a shape filled, `filled` of it (the countdown): see `outline` for which way each fills. a circle fills out from its middle, a ring in from its edge
fn fill_shape(position: Vec2, radius: f32, shape: Shape, filled: f32, color: Color) {
    let sides = (radius.sqrt() * 3.0).clamp(24.0, 96.0) as u8;
    match shape {
        Shape::Circle => draw_poly(position.x, position.y, sides, radius * filled.clamp(0.0, 1.0), 0.0, color),
        Shape::Ring { inner } => fill_band(position, radius - (radius - inner) * filled.clamp(0.0, 1.0), radius, sides as usize, color),
        _ => fill_polygon(&shape_outline(position, radius, shape, filled), color),
    }
}

fn edge_shape(position: Vec2, radius: f32, shape: Shape, thick: f32, color: Color) {
    let sides = (radius.sqrt() * 3.0).clamp(24.0, 96.0) as u8;
    match shape {
        Shape::Circle => draw_poly_lines(position.x, position.y, sides, radius, 0.0, thick, color),
        Shape::Ring { inner } => {
            draw_poly_lines(position.x, position.y, sides, radius, 0.0, thick, color);
            draw_poly_lines(position.x, position.y, sides, inner, 0.0, thick, color);
        }
        _ => edge_polygon(&shape_outline(position, radius, shape, 1.0), thick, color),
    }
}

//hazards on the ground: one that has not gone off is its shape, faint, with the countdown filling it and an edge that pulses, in red or in the colour of its
//kind with that kind's hatching; one that has gone off is a flash, and one that is burning flickers orange. `zoom` keeps the hatching and edges a size on the
//screen
pub(crate) fn draw_hazards(hazards: &[Hazard], time: f32, zoom: f32) {
    for hazard in hazards {
        let (at, radius, shape) = (hazard.position, hazard.radius, hazard.shape);
        //a mine that nothing has come near: a faint ring and a blinking light, which the army sees when it is close
        if hazard.mine {
            let blink = 0.5 + 0.5 * (time * 4.0 + at.x * 0.013).sin();
            let tint = hazard.color;
            draw_poly_lines(at.x, at.y, 20, radius, 0.0, 1.5 / zoom.max(0.01), Color::new(tint.r, tint.g, tint.b, 0.2));
            draw_circle(at.x, at.y, (radius * 0.12).max(4.0), Color::new(1.0, 0.25, 0.2, 0.25 + 0.5 * blink));
            continue;
        }
        if hazard.armed {
            let filled = 1.0 - (hazard.warn / hazard.warn_total).clamp(0.0, 1.0);
            let pulse = 0.5 + 0.5 * (time * 12.0).sin();
            //one of a boss's is drawn in the colour of its kind, and the rest are red
            let edge = match hazard.circle {
                Some(circle) => {
                    let tint = circle.color();
                    Color::new(tint.r, tint.g, tint.b, 0.55 + 0.45 * pulse)
                }
                None => danger_color(0.55 + 0.45 * pulse),
            };
            let fill = hazard.tint();
            fill_shape(at, radius, shape, 1.0, Color::new(fill.r, fill.g, fill.b, 0.10 + 0.06 * pulse));
            fill_shape(at, radius, shape, filled, Color::new(fill.r, fill.g, fill.b, 0.22));
            if let Some(hatch) = hatch_of(hazard.circle) {
                draw_hatching(at, radius, shape, hatch, zoom, time, Color::new(fill.r, fill.g, fill.b, 0.26 + 0.1 * pulse));
            }
            let size = match shape {
                Shape::Line { width, .. } | Shape::Rect { width, .. } => width,
                _ => radius,
            };
            edge_shape(at, radius, shape, (size * 0.02).clamp(3.0, 10.0).max(2.0 / zoom.max(0.01)), edge);
            continue;
        }
        if hazard.flash > 0.0 {
            let fade = hazard.flash / HAZARD_FLASH;
            fill_shape(at, radius, shape, 1.0, Color::new(1.0, 0.95, 0.8, 0.55 * fade));
        }
        if hazard.linger > 0.0 {
            let flicker = 0.5 + 0.5 * (time * 9.0 + at.x * 0.01).sin();
            let fade = hazard.linger.min(1.0);
            //(in its own colour: a fire's orange, a curse's purple)
            let tint = hazard.color;
            fill_shape(at, radius, shape, 1.0, Color::new(tint.r, tint.g * 0.9, tint.b, (0.16 + 0.1 * flicker) * fade));
            edge_shape(at, radius, shape, 4.0, Color::new((tint.r + 0.1).min(1.0), (tint.g + 0.1).min(1.0), (tint.b + 0.05).min(1.0), (0.5 + 0.3 * flicker) * fade));
        }
    }
}

//a hazard on the map: the outline of its shape, at `scale` map pixels to a world one
pub(crate) fn draw_hazard_on_map(map: &Minimap, hazard: &Hazard, color: Color) {
    let at = map.to_screen(hazard.position);
    let scale = map.scale();
    match hazard.shape {
        Shape::Circle => draw_circle_lines(at.x, at.y, (hazard.radius * scale).max(2.0), 1.5, color),
        Shape::Ring { inner } => {
            draw_circle_lines(at.x, at.y, (hazard.radius * scale).max(2.0), 1.5, color);
            draw_circle_lines(at.x, at.y, (inner * scale).max(1.0), 1.0, color);
        }
        shape => {
            let corners: Vec<Vec2> = shape_outline(hazard.position, hazard.radius, shape, 1.0).into_iter().map(|corner| map.to_screen(corner)).collect();
            edge_polygon(&corners, 1.5, color);
        }
    }
}

//the big warnings off the screen that the player should know of, as where to point, the colour and what to say: one of a boss's, or any as big as
//EDGE_MARK_SIZE, that will kill what it hits, that is not on the screen at all but is near enough to matter, at most EDGE_MARKS_MOST of them, one for each
//group (the meteors, which fall on the army and are small, are left out)
pub(crate) fn edge_marks(hazards: &[Hazard], player: Vec2, zoom: f32, screen: Vec2) -> Vec<(Vec2, Color, String)> {
    let half = screen / 2.0 / zoom.max(0.0001);
    let mut marks: Vec<(Vec2, Color, String)> = Vec::new();
    let mut groups: Vec<u32> = Vec::new();
    for hazard in hazards.iter().filter(|hazard| {
        hazard.armed && !hazard.mine && hazard.damage >= UNIT_HEALTH && hazard.warn_total >= BLAST_WARNING_MIN_SECONDS && hazard.circle != Some(BossCircle::Meteor) && (hazard.circle.is_some() || hazard.radius >= EDGE_MARK_SIZE)
    }) {
        let reach = hazard.radius;
        let offset = hazard.position - player;
        let unseen = offset.x.abs() > half.x + reach || offset.y.abs() > half.y + reach;
        let near = outside_by(hazard.position, hazard.radius, hazard.shape, player) <= half.length() * EDGE_MARK_SCREENS;
        if !unseen || !near || (hazard.group != 0 && groups.contains(&hazard.group)) {
            continue;
        }
        groups.push(hazard.group);
        let name = hazard.circle.map_or("BLAST", BossCircle::name);
        marks.push((hazard.middle(), hazard.tint(), format!("{name} {:.1}s", hazard.warn.max(0.0))));
        if marks.len() >= EDGE_MARKS_MOST {
            break;
        }
    }
    marks
}

pub(crate) fn draw_hazard_arrows(hazards: &[Hazard], player: Vec2, zoom: f32) {
    for (at, color, label) in edge_marks(hazards, player, zoom, Vec2::new(screen_width(), screen_height())) {
        draw_edge_arrow(at, player, zoom, color, &label);
    }
}

//seconds between one tick and the next of the warning the army is in, `seconds` before it goes off: TICK_SLOWEST while there is TICK_FROM or more, and
//quicker as it comes due, down to TICK_FASTEST
pub(crate) fn tick_gap(seconds: f32) -> f32 {
    (TICK_SLOWEST * seconds.max(0.0) / TICK_FROM).clamp(TICK_FASTEST, TICK_SLOWEST)
}

//the warning's headline: what to get out of, named by its colour and, when it is not a circle, its shape and what to do
pub(crate) fn warning_title(warning: &BlastWarning) -> String {
    let colour = warning.circle.map_or("RED", BossCircle::colour_name);
    let thing = match warning.circle.and_then(BossCircle::title_words).or(warning.shape.words()) {
        Some((word, what)) => format!("{colour} {word}: {what}"),
        None => colour.to_string(),
    };
    if warning.player_inside { format!("GET OUT OF THE {thing}!") } else { format!("THE ARMY IS IN THE {thing}!") }
}
