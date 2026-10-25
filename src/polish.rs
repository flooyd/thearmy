//! the finishing touches: kill streaks, the red edge of a failing army, marks left on the ground and pieces of what broke

use crate::prelude::*;

// ---------- kill streaks ----------

//the streak sizes that are marked when they are reached, and what each is called
pub(crate) const COMBO_STEPS: [(usize, &str); 6] = [(10, "COMBO"), (25, "RAMPAGE"), (50, "MASSACRE"), (100, "CARNAGE"), (200, "ANNIHILATION"), (400, "APOCALYPSE")];

//enemies destroyed one after another, each within COMBO_WINDOW seconds of the last
#[derive(Default)]
pub(crate) struct Combo {
    pub(crate) count: usize,
    pub(crate) left: f32,  //seconds until it is over
    pub(crate) flash: f32, //1 when it has just gone up, fading, for the number to jump
}

impl Combo {
    //more enemies destroyed. returns the step it has just reached, if it has
    pub(crate) fn add(&mut self, kills: usize) -> Option<(usize, &'static str)> {
        if kills == 0 {
            return None;
        }
        let before = self.count;
        self.count += kills;
        self.left = COMBO_WINDOW;
        self.flash = 1.0;
        COMBO_STEPS.iter().copied().rev().find(|&(size, _)| before < size && self.count >= size)
    }

    pub(crate) fn update(&mut self, dt: f32) {
        self.flash = (self.flash - dt * 5.0).max(0.0);
        if self.count > 0 {
            self.left -= dt;
            if self.left <= 0.0 {
                self.count = 0;
            }
        }
    }

    //what the streak is called now: nothing until it is worth a mention
    pub(crate) fn name(&self) -> Option<&'static str> {
        COMBO_STEPS.iter().rev().find(|&&(size, _)| self.count >= size).map(|&(_, name)| name)
    }
}

//the streak, big, at the top middle of the screen under the banners: the number, its name, and a bar running out
pub(crate) fn draw_combo(combo: &Combo) {
    if combo.count < COMBO_SHOWN_FROM {
        return;
    }
    let middle = screen_width() / 2.0;
    let top = 74.0;
    let heat = (combo.count as f32 / 200.0).clamp(0.0, 1.0);
    //from white through gold to red as it grows
    let color = if heat < 0.5 { lerp_color(INK, TROPHY, heat * 2.0) } else { lerp_color(TROPHY, ALERT, heat * 2.0 - 1.0) };
    let size = 44.0 + combo.flash * 10.0;
    let text = format!("x{}", combo.count);
    draw_label_shadowed(&text, middle - text_width(&text, size, Face::Header) / 2.0, top + size * 0.8, size, color, Face::Header);
    let name = combo.name().unwrap_or("KILL STREAK");
    draw_label_shadowed(name, middle - text_width(name, 18.0, Face::Bold) / 2.0, top + size * 0.8 + 24.0, 18.0, INK, Face::Bold);
    let width = 180.0;
    draw_meter(middle - width / 2.0, top + size * 0.8 + 34.0, width, 4.0, combo.left / COMBO_WINDOW, color);
}

pub(crate) fn lerp_color(from: Color, to: Color, amount: f32) -> Color {
    let t = amount.clamp(0.0, 1.0);
    Color::new(from.r + (to.r - from.r) * t, from.g + (to.g - from.g) * t, from.b + (to.b - from.b) * t, from.a + (to.a - from.a) * t)
}

// ---------- a failing army ----------

//how close the army is to being gone, 0 when it is fine to 1 when it is nearly all lost, judged against the most it has been.
//an army that never got anywhere is never in peril, a run has to be lost from somewhere
pub(crate) fn peril(units: usize, peak: usize) -> f32 {
    if peak < PERIL_MIN_PEAK {
        return 0.0;
    }
    let line = peak as f32 * PERIL_FROM;
    if units as f32 >= line {
        return 0.0;
    }
    (1.0 - units as f32 / line).clamp(0.0, 1.0)
}

//a red glow round the edge of the screen: `strength` is the peril and `beat` the heart, 1 as it thumps and fading
pub(crate) fn draw_vignette(strength: f32, beat: f32) {
    if strength <= 0.0 {
        return;
    }
    let (w, h) = (screen_width(), screen_height());
    let depth = w.min(h) * (0.1 + 0.15 * strength);
    let alpha = (0.3 + 0.3 * strength) * (0.55 + 0.45 * beat);
    let edge = Color::new(0.85, 0.04, 0.07, alpha);
    let clear = Color::new(0.85, 0.04, 0.07, 0.0);
    let corner = |x: f32, y: f32, color: Color| Vertex::new(x, y, 0.0, 0.0, 0.0, color);
    //the four corners of the screen, then the four of a rectangle inside it
    let vertices = vec![
        corner(0.0, 0.0, edge),
        corner(w, 0.0, edge),
        corner(w, h, edge),
        corner(0.0, h, edge),
        corner(depth, depth, clear),
        corner(w - depth, depth, clear),
        corner(w - depth, h - depth, clear),
        corner(depth, h - depth, clear),
    ];
    //a strip along each side
    let indices: Vec<u16> = vec![0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7];
    draw_mesh(&Mesh { vertices, indices, texture: None });
}

// ---------- marks on the ground ----------

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum MarkKind {
    Scorch, //burnt ground where something blew up
    Splat,  //what is left of an enemy
}

//a mark on the ground, which fades away over MARK_LIFE seconds
#[derive(Clone)]
pub(crate) struct Mark {
    pub(crate) position: Vec2,
    pub(crate) radius: f32,
    pub(crate) color: Color, //of what made it
    pub(crate) kind: MarkKind,
    pub(crate) age: f32,
}

//age the marks on the ground, take on the new ones and drop the oldest past MAX_MARKS
pub(crate) fn update_marks(ground: &mut Vec<Mark>, fresh: &mut Vec<Mark>, dt: f32) {
    for mark in ground.iter_mut() {
        mark.age += dt;
    }
    ground.retain(|mark| mark.age < MARK_LIFE);
    ground.append(fresh);
    if ground.len() > MAX_MARKS {
        let extra = ground.len() - MAX_MARKS;
        ground.drain(..extra);
    }
}

//a number from 0 to 1 that is always the same for the same two inputs, so a mark is lumpy in the same way every frame
fn scatter(seed: u32, which: u32) -> f32 {
    let mut value = seed ^ which.wrapping_mul(0x9E37_79B9);
    value = (value ^ (value >> 15)).wrapping_mul(0x2C1B_3C6D);
    value = (value ^ (value >> 12)).wrapping_mul(0x297A_2D39);
    ((value ^ (value >> 15)) >> 8) as f32 / (1u32 << 24) as f32
}

//the marks in view as circles, a few overlapping ones to each so they are not perfectly round
pub(crate) fn mark_discs(marks: &[Mark], view: Rect) -> Vec<Disc> {
    let mut discs = Vec::new();
    for mark in marks {
        let reach = mark.radius * 1.3;
        if !view.overlaps(&Rect::new(mark.position.x - reach, mark.position.y - reach, reach * 2.0, reach * 2.0)) {
            continue;
        }
        let faded = ((MARK_LIFE - mark.age) / MARK_FADE).clamp(0.0, 1.0);
        let seed = (mark.position.x as i32 as u32).wrapping_mul(73_856_093) ^ (mark.position.y as i32 as u32).wrapping_mul(19_349_663);
        let blob = |discs: &mut Vec<Disc>, which: u32, distance: f32, size: f32, color: Color| {
            let angle = scatter(seed, which) * std::f32::consts::TAU;
            let away = mark.radius * distance * (0.6 + 0.4 * scatter(seed, which + 100));
            discs.push(Disc::new(mark.position.x + angle.cos() * away, mark.position.y + angle.sin() * away, mark.radius * size, color));
        };
        match mark.kind {
            MarkKind::Scorch => {
                //scorched earth: a dull brown that is darker the nearer the middle, which shows on the dark floor
                let soot = |shade: f32, alpha: f32| Color::new(0.2 * shade, 0.11 * shade, 0.06 * shade, alpha * faded);
                discs.push(Disc::new(mark.position.x, mark.position.y, mark.radius * 0.85, soot(1.0, 0.4)));
                blob(&mut discs, 1, 0.3, 0.55, soot(0.6, 0.45));
                blob(&mut discs, 2, 0.35, 0.45, soot(0.6, 0.45));
                discs.push(Disc::new(mark.position.x, mark.position.y, mark.radius * 0.4, soot(0.15, 0.7)));
                for which in 3..7 {
                    blob(&mut discs, which, 0.95, 0.14, soot(0.8, 0.4));
                }
                //still glowing for a moment
                let glow = (1.0 - mark.age / MARK_GLOW).clamp(0.0, 1.0);
                if glow > 0.0 {
                    discs.push(Disc::new(mark.position.x, mark.position.y, mark.radius * 0.5, Color::new(1.0, 0.45, 0.1, 0.4 * glow * faded)));
                }
            }
            MarkKind::Splat => {
                let stain = |alpha: f32| Color::new(mark.color.r * 0.6, mark.color.g * 0.6, mark.color.b * 0.6, alpha * faded);
                discs.push(Disc::new(mark.position.x, mark.position.y, mark.radius * 0.6, stain(0.4)));
                for which in 1..5 {
                    blob(&mut discs, which, 0.8, 0.28, stain(0.35));
                }
                for which in 5..9 {
                    blob(&mut discs, which, 1.25, 0.1, stain(0.32));
                }
            }
        }
    }
    discs
}

// ---------- pieces of what broke ----------

//debris to be thrown from where something was destroyed
pub(crate) struct ShardBurst {
    pub(crate) position: Vec2,
    pub(crate) color: Color,
    pub(crate) size: f32, //of what broke
}

//a flying chip of an enemy, spinning as it goes and fading
pub(crate) struct Shard {
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2,
    pub(crate) angle: f32,
    pub(crate) spin: f32, //radians a second
    pub(crate) age: f32,
    pub(crate) life: f32,
    pub(crate) size: f32,
    pub(crate) color: Color,
}

//turn the debris asked for into chips: more of them and bigger ones from something big
pub(crate) fn throw_shards(shards: &mut Vec<Shard>, bursts: &mut Vec<ShardBurst>) {
    for burst in bursts.drain(..) {
        let count = (burst.size / 12.0).clamp(3.0, 16.0) as usize;
        let force = 0.6 + (burst.size / 220.0).min(1.0) * 0.7;
        for _ in 0..count {
            if shards.len() >= MAX_SHARDS {
                return;
            }
            let direction = Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU));
            shards.push(Shard {
                position: burst.position + direction * burst.size * rand::gen_range(0.0, 0.3),
                velocity: direction * rand::gen_range(140.0, 460.0) * force,
                angle: rand::gen_range(0.0, std::f32::consts::TAU),
                spin: rand::gen_range(-14.0, 14.0),
                age: 0.0,
                life: rand::gen_range(0.55, 1.1),
                size: (burst.size * rand::gen_range(0.1, 0.22)).clamp(4.0, 46.0),
                color: burst.color,
            });
        }
    }
}

pub(crate) fn update_shards(shards: &mut Vec<Shard>, dt: f32) {
    let slow_down = (-SHARD_DRAG * dt).exp();
    shards.retain_mut(|shard| {
        shard.age += dt;
        shard.position += shard.velocity * dt;
        shard.velocity *= slow_down;
        shard.angle += shard.spin * dt;
        shard.age < shard.life
    });
}

//the chips in view, as little tumbling rectangles of the colour of what broke
pub(crate) fn draw_shards(shards: &[Shard], view: Rect) {
    for shard in shards.iter().filter(|shard| view.contains(shard.position)) {
        let left = 1.0 - shard.age / shard.life;
        let size = shard.size * (0.4 + 0.6 * left);
        let color = Color::new(shard.color.r, shard.color.g, shard.color.b, left.min(1.0) * 0.9);
        draw_rectangle_ex(shard.position.x, shard.position.y, size, size * 0.6, DrawRectangleParams { offset: Vec2::splat(0.5), rotation: shard.angle, color });
    }
}

// ---------- the pieces of a shatter ----------

//what is to be thrown from an enemy that shattered (or a boss that burst, or cracked): the same pieces on every machine, from the seed
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShatterBurst {
    pub(crate) at: Vec2,
    pub(crate) size: f32,
    pub(crate) color: Color,
    pub(crate) seed: u32,
    pub(crate) frost: bool, //it was frozen, and its pieces are ice
}

//a piece of a shattered enemy: a cell of a pattern cut over its square, flying out from where it was, turning, slowing and fading
pub(crate) struct Fragment {
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2,
    pub(crate) angle: f32,
    pub(crate) spin: f32, //radians a second
    pub(crate) age: f32,
    pub(crate) life: f32,
    pub(crate) corners: Vec<Vec2>, //round its own middle
    pub(crate) color: Color,
    pub(crate) glint: bool, //a piece of ice that catches the light now and then
}

//a polygon cut by a line, keeping what is on the side away from `normal` (where (p - at) . normal is not more than 0)
fn cut(polygon: &[Vec2], at: Vec2, normal: Vec2) -> Vec<Vec2> {
    let mut kept = Vec::with_capacity(polygon.len() + 1);
    for k in 0..polygon.len() {
        let (a, b) = (polygon[k], polygon[(k + 1) % polygon.len()]);
        let (da, db) = ((a - at).dot(normal), (b - at).dot(normal));
        if da <= 0.0 {
            kept.push(a);
        }
        if (da <= 0.0) != (db <= 0.0) {
            kept.push(a + (b - a) * (da / (da - db)));
        }
    }
    kept
}

//the cells a square `size` across breaks into: a Voronoi pattern over it, a cell for each of size / FRAGMENT_SIZE points (FEWEST to MOST_FRAGMENTS) put down from
//the seed, each as where its middle is and its corners round that. the same for the same seed on every machine
pub(crate) fn shatter_cells(size: f32, seed: u32) -> Vec<(Vec2, Vec<Vec2>)> {
    if size <= 0.0 {
        return Vec::new();
    }
    let count = ((size / FRAGMENT_SIZE) as usize).clamp(FEWEST_FRAGMENTS, MOST_FRAGMENTS);
    let half = size / 2.0;
    let sites: Vec<Vec2> = (0..count).map(|i| Vec2::new(hash_noise(seed, 2 * i as u32) - 0.5, hash_noise(seed, 2 * i as u32 + 1) - 0.5) * size).collect();
    sites
        .iter()
        .enumerate()
        .filter_map(|(i, &site)| {
            let mut cell = vec![Vec2::new(-half, -half), Vec2::new(half, -half), Vec2::new(half, half), Vec2::new(-half, half)];
            for (j, &other) in sites.iter().enumerate() {
                if i != j && other != site {
                    cell = cut(&cell, (site + other) / 2.0, other - site);
                }
            }
            if cell.len() < 3 {
                return None;
            }
            let middle = cell.iter().copied().sum::<Vec2>() / cell.len() as f32;
            Some((middle, cell.into_iter().map(|corner| corner - middle).collect()))
        })
        .collect()
}

//the pieces of a shattered enemy thrown out from where it was, as many as there is room for under MAX_FRAGMENTS: a frozen one's ice-white, glinting
pub(crate) fn throw_fragments(fragments: &mut Vec<Fragment>, at: Vec2, size: f32, color: Color, seed: u32, frost: bool) {
    let tint = if frost { lerp_color(color, ICE, 0.5) } else { color };
    let force = 0.7 + (size / 200.0).min(1.2);
    for (k, (middle, corners)) in shatter_cells(size, seed).into_iter().enumerate() {
        if fragments.len() >= MAX_FRAGMENTS {
            return;
        }
        let noise = |salt: u32| hash_noise(seed ^ 0x5BD1_E995, k as u32 * 8 + salt);
        let out = middle.try_normalize().unwrap_or_else(|| Vec2::from_angle(noise(0) * std::f32::consts::TAU));
        fragments.push(Fragment {
            position: at + middle,
            velocity: (out * (180.0 + 320.0 * noise(1)) + Vec2::from_angle(noise(2) * std::f32::consts::TAU) * 60.0) * force,
            angle: 0.0,
            spin: (noise(3) - 0.5) * 14.0,
            age: 0.0,
            life: FRAGMENT_LIFE * (0.75 + 0.5 * noise(4)),
            corners,
            color: tint,
            glint: frost && noise(5) < 0.35,
        });
    }
}

pub(crate) fn update_fragments(fragments: &mut Vec<Fragment>, dt: f32) {
    let slow_down = (-FRAGMENT_DRAG * dt).exp();
    fragments.retain_mut(|piece| {
        piece.age += dt;
        piece.position += piece.velocity * dt;
        piece.velocity *= slow_down;
        piece.angle += piece.spin * dt;
        piece.age < piece.life
    });
}

//the pieces in view: each its polygon in its colour with a black edge, turning and fading at the end of its flight, and the ice ones glinting. `zoom` keeps
//the edge a pixel and a half wide
pub(crate) fn draw_fragments(fragments: &[Fragment], view: Rect, zoom: f32) {
    let pixel = 1.0 / zoom.max(0.01);
    for piece in fragments.iter().filter(|piece| view.contains(piece.position)) {
        let alpha = ((1.0 - piece.age / piece.life) * 2.0).clamp(0.0, 1.0);
        let (sin, cos) = piece.angle.sin_cos();
        let points: Vec<Vec2> = piece.corners.iter().map(|corner| piece.position + Vec2::new(corner.x * cos - corner.y * sin, corner.x * sin + corner.y * cos)).collect();
        let fill = Color::new(piece.color.r, piece.color.g, piece.color.b, alpha);
        let edge = Color::new(0.02, 0.03, 0.05, alpha);
        for k in 1..points.len() - 1 {
            draw_triangle(points[0], points[k], points[k + 1], fill);
        }
        for k in 0..points.len() {
            let (a, b) = (points[k], points[(k + 1) % points.len()]);
            draw_line(a.x, a.y, b.x, b.y, 1.5 * pixel, edge);
        }
        if piece.glint && (piece.age * 9.0 + piece.angle).sin() > 0.75 {
            draw_poly(piece.position.x, piece.position.y, 4, 2.6 * pixel, 45.0, Color::new(1.0, 1.0, 1.0, alpha));
        }
    }
}
