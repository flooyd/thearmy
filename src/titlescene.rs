//! the picture on the title screen: the player running at the head of an army of ships, a few of what stands in their way, and the Sovereign over it all. it is
//! drawn with the game's own art (the discs the army is drawn with, the runner, the enemies, and the boss as it looks in a game, without the circles it puts down) in a
//! world of its own that is only a backdrop: nothing in it is really a unit or an enemy, and all that is stepped is the figure's stride. where things stand is worked
//! out from the size of the window each frame, with the middle of the screen left to the title

use crate::prelude::*;

const SCALE: f32 = 1.15; //how much bigger than in a game things are drawn, in a window of the size the layout is made for...
const DESIGN_HEIGHT: f32 = 720.0; //...which is this tall...
const DESIGN_WIDTH: f32 = 1100.0; //...and this wide (a window smaller than that in either way draws it all smaller)
const PARALLAX: f32 = 16.0; //pixels the nearest things move when the mouse goes from the middle of the screen to an edge
const SOVEREIGN_SIZE: f32 = 195.0; //pixels along each side, in the world of the scene
const SOVEREIGN_FROM_RIGHT: f32 = 150.0; //where the middle of it is, in pixels from the right edge and the top at the design size
const SOVEREIGN_FROM_TOP: f32 = 98.0;
const ARMY_FROM_LEFT: f32 = 195.0; //and the player in the middle of the army, from the left edge and the bottom
const ARMY_FROM_BOTTOM: f32 = 190.0;
const HERO_SCALE: f32 = 1.7; //the player is drawn this many times bigger than in a game, so that it can be found in the crowd
const GROUND_LINES_REACH: f32 = 330.0; //the lines of the ground going by only go this far from the left edge, so that none runs through the words
const RING_INNER: (usize, f32) = (6, 36.0); //how many ships are in the inner ring round the player, and how far out it is (in the world of the scene)
const RING_OUTER: (usize, f32) = (13, 70.0);
const TAIL_SHIPS: usize = 9; //ships strung out behind them, coming in from the edge of the screen

//one of the enemies, with where it stands: in pixels from the right edge and the bottom of a window of the design size
struct Foe {
    name: &'static str,
    from_right: f32,
    from_bottom: f32,
    phase: f32, //where it is in its bobbing
}

pub(crate) struct TitleScene {
    runner: Runner,
    units: Vec<Unit>,
    slots: Vec<Vec2>, //where each of the units stands in relation to the player, in the world of the scene
    enemies: Vec<Enemy>,
    foes: Vec<Foe>,
    squads: Squads, //(none: nobody is sent at anything here)
}

//the enemies on the title screen: what they are, where they stand, whether they are running at the army, and how much of their health they have left
const FOES: [(&str, f32, f32, bool, f32); 6] = [
    ("Bulwark", 110.0, 235.0, false, 1.0),
    ("Splitter", 215.0, 255.0, false, 1.0),
    ("Brute", 230.0, 150.0, true, 0.6),
    ("Grunt", 130.0, 105.0, true, 1.0),
    ("Grunt", 235.0, 62.0, true, 0.7),
    ("Scout", 55.0, 160.0, true, 1.0),
];

fn kind_named(name: &str) -> &'static EnemyKind {
    ENEMY_KINDS.iter().find(|kind| kind.name == name).expect("a kind of enemy")
}

//0 to 1, eased: how far something that arrives `delay` seconds after the screen came up and takes `length` seconds to get there has come
fn arrival(time: f32, delay: f32, length: f32) -> f32 {
    let k = ((time - delay) / length).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

//where the screen is, and what that makes of the design: how much bigger or smaller everything is, and the camera that looks at it
struct Frame {
    screen: Vec2,
    scale: f32,
    zoom: f32,
}

impl Frame {
    fn new(screen: Vec2) -> Frame {
        let scale = (screen.y / DESIGN_HEIGHT).min(screen.x / DESIGN_WIDTH);
        Frame { screen, scale, zoom: scale * SCALE }
    }

    //where a place on the screen, in pixels, is in the world of the scene, which has the middle of the screen as its middle
    fn world(&self, pixel: Vec2) -> Vec2 {
        (pixel - self.screen / 2.0) / self.zoom
    }

    //the camera for a layer of the scene, which is moved `shift` pixels over (the nearer layers more than the far ones, as the mouse moves)
    fn camera(&self, shift: Vec2) -> Camera2D {
        follow_camera(-shift.x / self.zoom, -shift.y / self.zoom, self.zoom)
    }

    //a place a number of pixels in from the right edge (at the design size) and up from the bottom
    fn right_bottom(&self, from_right: f32, from_bottom: f32) -> Vec2 {
        Vec2::new(self.screen.x - from_right * self.scale, self.screen.y - from_bottom * self.scale)
    }

    //where the words are, which the picture keeps off: the line under the title and the prompts under that, as wide as the widest of them (they are sized by the
    //height of the window, so this is too). an enemy that would stand in it in a window of an odd shape is left out
    fn text_area(&self) -> Rect {
        Rect::new(self.screen.x / 2.0 - 0.41 * self.screen.y, 0.575 * self.screen.y, 0.82 * self.screen.y, 0.35 * self.screen.y)
    }

    //the square an enemy of a kind takes up when it stands where it does
    fn foe_box(&self, name: &str, from_right: f32, from_bottom: f32) -> Rect {
        let (middle, half) = (self.right_bottom(from_right, from_bottom), kind_named(name).size * self.zoom / 2.0);
        Rect::new(middle.x - half, middle.y - half, half * 2.0, half * 2.0)
    }

    //where the prompts are (from the first to below the last, as wide as the longest of them), which the ships keep off
    fn prompts(&self) -> Rect {
        Rect::new(self.screen.x / 2.0 - 0.31 * self.screen.y, 0.74 * self.screen.y, 0.62 * self.screen.y, 0.2 * self.screen.y)
    }

    //how far from the player the army reaches, in pixels
    fn army_reach(&self) -> f32 {
        (RING_OUTER.1 + RADIUS * 1.5) * self.zoom
    }

    //where the player is, in the middle of the army: down by the left edge, and in a window that is not as wide as the picture is made for, moved to keep the army
    //off the prompts (to the left of them if there is room there, and if not, up in the gap over them) and above the quit button
    fn army_middle(&self) -> Vec2 {
        let (reach, prompts) = (self.army_reach(), self.prompts());
        let mut middle = Vec2::new(ARMY_FROM_LEFT * self.scale, self.screen.y - ARMY_FROM_BOTTOM * self.scale);
        let left_of_prompts = prompts.x - reach - 6.0;
        if left_of_prompts >= reach * 0.6 {
            middle.x = middle.x.min(left_of_prompts);
        } else {
            middle.y = middle.y.min(prompts.y - reach - 6.0);
        }
        let quit_top = self.screen.y * 0.915 - 28.0;
        middle.y = middle.y.min(quit_top - reach);
        middle
    }

    //whether a ship that stands `slot` from the player (in the world of the scene) is clear of the prompts, as the army rests
    fn ship_is_clear(&self, slot: Vec2) -> bool {
        let (place, half) = (self.army_middle() + slot * self.zoom, RADIUS * 1.5 * self.zoom);
        !self.prompts().overlaps(&Rect::new(place.x - half, place.y - half, half * 2.0, half * 2.0))
    }
}

//where the ships stand round the player: a ring close in, a ring further out, and a string of them behind
fn ship_slots() -> Vec<Vec2> {
    let ring = |(count, radius): (usize, f32), turn: f32| (0..count).map(move |k| Vec2::from_angle(turn + std::f32::consts::TAU * k as f32 / count as f32) * radius);
    let tail = (0..TAIL_SHIPS).map(|k| Vec2::new(-(RING_OUTER.1 + 40.0 + k as f32 * 19.0), (k as f32 * 2.1).sin() * 38.0 + (k as f32 * 0.9).cos() * 12.0));
    ring(RING_INNER, 0.3).chain(ring(RING_OUTER, 0.0)).chain(tail).collect()
}

//the Sovereign as it is in a game (a big square in its colour with two eyes and an aura that pulses), looking at the army, with a glow behind it that the title is
//read over
fn draw_sovereign(frame: &Frame, time: f32, shift: Vec2) {
    //it comes down from above, slowly, and then breathes
    let come = 1.0 - arrival(time, 0.15, 1.8);
    let at = Vec2::new(frame.screen.x - SOVEREIGN_FROM_RIGHT * frame.scale, SOVEREIGN_FROM_TOP * frame.scale + (time * 0.8).sin() * 6.0 * frame.scale - come * 420.0 * frame.scale);
    let middle = frame.world(at);
    let size = SOVEREIGN_SIZE * (1.0 + 0.012 * (time * 1.9).sin());
    let camera = frame.camera(shift);
    set_camera(&camera);
    let color = FINAL_BOSS.color;
    let glow: Vec<Disc> = (0..16).map(|k| Disc::new(middle.x, middle.y, size * (0.62 + 0.085 * k as f32), Color::new(color.r, color.g, color.b, 0.016))).collect();
    draw_discs(Layer::Marks, &glow, &camera);
    let half = size / 2.0;
    let pulse = 0.5 + 0.5 * (time * 3.0).sin();
    let width = (size * 0.02).clamp(6.0, 30.0);
    draw_rectangle_lines(middle.x - half - width, middle.y - half - width, size + width * 2.0, size + width * 2.0, width, Color::new(color.r, color.g, color.b, 0.3 + 0.35 * pulse));
    draw_rectangle(middle.x - half, middle.y - half, size, size, color);
    //the eyes are on the army, which is down and to the left of it
    let eye = size * 0.14;
    let toward = (frame.world(frame.army_middle()) - middle).normalize_or_zero();
    for side in [-1.0, 1.0] {
        let eye_at = Vec2::new(middle.x + side * size * 0.22, middle.y - size * 0.1);
        draw_rectangle(eye_at.x - eye / 2.0, eye_at.y - eye / 2.0, eye, eye, WHITE);
        let pupil = eye_at + toward * eye * 0.22;
        draw_rectangle(pupil.x - eye / 4.0, pupil.y - eye / 4.0, eye / 2.0, eye / 2.0, BLACK);
    }
}

impl TitleScene {
    pub(crate) fn new() -> TitleScene {
        let slots = ship_slots();
        let units = slots
            .iter()
            .map(|_| {
                let mut unit = new_unit(Vec2::ZERO);
                unit.age = 99.0;
                unit.nose = Vec2::X;
                unit
            })
            .collect();
        //the farthest up the screen are drawn first, so the nearer cover them
        let mut made: Vec<(Enemy, Foe)> = FOES
            .iter()
            .enumerate()
            .map(|(k, &(name, from_right, from_bottom, chasing, health))| {
                let mut enemy = Enemy::new(Vec2::splat(WORLD_SIZE / 2.0), kind_named(name), 1.0);
                //(the same size every time, the number the kind says: it was only nudged so that no two in a game are the same)
                enemy.size = kind_named(name).size;
                enemy.chasing = chasing;
                enemy.health = enemy.max_health * health;
                (enemy, Foe { name, from_right, from_bottom, phase: k as f32 * 1.9 })
            })
            .collect();
        made.sort_by(|a, b| b.1.from_bottom.total_cmp(&a.1.from_bottom));
        let (enemies, foes): (Vec<Enemy>, Vec<Foe>) = made.into_iter().unzip();
        TitleScene { runner: Runner::new(), units, slots, enemies, foes, squads: Squads::new() }
    }

    //a frame later: the figure runs
    pub(crate) fn update(&mut self, dt: f32) {
        self.runner.step(Vec2::new(PLAYER_SPEED, 0.0), dt);
    }

    //the picture, over the backdrop and under the words. `time` is how many seconds the screen has been up, `mouse` where the pointer is (the nearer things
    //move a little as it does) and `shape` what the units are drawn as, as in a game
    pub(crate) fn draw(&mut self, time: f32, mouse: Vec2, shape: UnitShape) {
        let frame = Frame::new(Vec2::new(screen_width(), screen_height()));
        let look = ((mouse - frame.screen / 2.0) / (frame.screen / 2.0)).clamp(Vec2::splat(-1.0), Vec2::splat(1.0));
        let shift = |depth: f32| -look * PARALLAX * depth * frame.scale;
        draw_sovereign(&frame, time, shift(0.35));
        self.draw_foes(&frame, time, shift(0.7));
        self.draw_army(&frame, time, shift(1.0), shape);
        set_default_camera();
    }

    fn draw_foes(&mut self, frame: &Frame, time: f32, shift: Vec2) {
        for (k, (enemy, foe)) in self.enemies.iter_mut().zip(&self.foes).enumerate() {
            //(one that would be in the words is not drawn)
            if frame.text_area().overlaps(&frame.foe_box(foe.name, foe.from_right, foe.from_bottom)) {
                enemy.position = Vec2::splat(-1.0e6);
                continue;
            }
            //they come in from the right a little after the army has, and then stand their ground, leaning in and out
            let come = 1.0 - arrival(time, 0.5 + 0.12 * k as f32, 1.5);
            let lean = (time * 1.6 + foe.phase).sin();
            let at = frame.right_bottom(foe.from_right, foe.from_bottom) + Vec2::new(lean * 7.0 * frame.scale + come * 520.0 * frame.scale, (time * 2.3 + foe.phase * 1.3).cos() * 4.0 * frame.scale);
            enemy.position = frame.world(at);
        }
        set_camera(&frame.camera(shift));
        draw_enemies(&self.enemies, &self.squads, time, 1.0);
    }

    fn draw_army(&mut self, frame: &Frame, time: f32, shift: Vec2, shape: UnitShape) {
        //the army runs in from the left, and then runs on the spot while the ground goes by
        let come = 1.0 - arrival(time, 0.0, 1.4);
        let rest = frame.army_middle();
        let at = Vec2::new(rest.x - come * 520.0 * frame.scale, rest.y + (time * 3.1).sin() * 2.5 * frame.scale);
        let middle = frame.world(at);
        let camera = frame.camera(shift);
        set_camera(&camera);
        //the glow behind it
        let blue = PLAYER_COLORS[0];
        let glow: Vec<Disc> = (0..14).map(|k| Disc::new(middle.x, middle.y, 55.0 + 11.0 * k as f32, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.011))).collect();
        draw_discs(Layer::Marks, &glow, &camera);
        //the lines of the ground going by, which stay in the strip by the left edge
        let reach = GROUND_LINES_REACH * frame.scale;
        for k in 0..9 {
            let row = at.y + ((k as f32 * 37.0) % 140.0 - 70.0) * frame.scale;
            let speed = (340.0 + (k % 4) as f32 * 90.0) * frame.scale;
            let length = (60.0 + (k % 3) as f32 * 45.0) * frame.scale;
            //(and where the prompts are, a line that goes by in their rows stops short of them)
            let reach = if row + 2.0 < frame.prompts().y { reach } else { reach.min(frame.prompts().x - 8.0) };
            let start = (k as f32 * 211.0 - time * speed).rem_euclid(reach.max(0.0) + length) - length;
            let (from, to) = (frame.world(Vec2::new(start.max(-40.0), row)), frame.world(Vec2::new((start + length).min(reach), row)));
            if to.x > from.x {
                draw_line(from.x, from.y, to.x, to.y, 1.5, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.14));
            }
        }
        for (k, (unit, slot)) in self.units.iter_mut().zip(&self.slots).enumerate() {
            let phase = k as f32 * 1.37;
            let sway = Vec2::new((time * 2.1 + phase).sin() * 3.0, (time * 2.7 + phase * 1.7).cos() * 3.0);
            //(a ship that would be in the words, in a window of an odd shape, is not drawn)
            let place = if frame.ship_is_clear(*slot) { middle + *slot + sway } else { Vec2::splat(-1.0e6) };
            unit.x = place.x;
            unit.y = place.y;
            unit.nose = Vec2::from_angle(0.16 * (time * 1.7 + phase).sin());
        }
        //(what is drawn is what is on the screen, with some to spare for the nearer things moving with the mouse)
        let (top_left, bottom_right) = (frame.world(Vec2::ZERO), frame.world(frame.screen));
        let view = Rect::new(top_left.x - 100.0, top_left.y - 100.0, bottom_right.x - top_left.x + 200.0, bottom_right.y - top_left.y + 200.0);
        draw_discs(Layer::Army, &unit_discs_in(&self.units, view, None, shape), &camera);
        //the player is drawn bigger than the ships, about the same place on the screen
        let hero = frame.zoom * HERO_SCALE;
        let target = middle - (middle - camera.target) / HERO_SCALE;
        set_camera(&follow_camera(target.x, target.y, hero));
        draw_runner(&self.runner, middle, blue);
    }
}

#[cfg(test)]
impl TitleScene {
    //the things in the scene and where they would stand at a size of window, as boxes in pixels: the Sovereign, the army with the tail left out (it comes in from off
    //the screen), and each of the enemies. for the tests of where they are
    pub(crate) fn boxes(screen: Vec2) -> Vec<(&'static str, Rect)> {
        let frame = Frame::new(screen);
        let around = |middle: Vec2, half: f32| Rect::new(middle.x - half, middle.y - half, half * 2.0, half * 2.0);
        let mut boxes = vec![("the Sovereign", around(Vec2::new(screen.x - SOVEREIGN_FROM_RIGHT * frame.scale, SOVEREIGN_FROM_TOP * frame.scale), SOVEREIGN_SIZE * frame.zoom / 2.0))];
        //(the army is a round thing, so this is the square inside its circle, which is what its corners could be)
        boxes.push(("the army", around(frame.army_middle(), frame.army_reach() * 0.707)));
        for &(name, from_right, from_bottom, _, _) in &FOES {
            boxes.push((name, frame.foe_box(name, from_right, from_bottom)));
        }
        boxes
    }

    //how many units and enemies there are in it
    pub(crate) fn counts(&self) -> (usize, usize) {
        (self.units.len(), self.enemies.len())
    }

    //where the words are in a window of a size
    pub(crate) fn text_area_of(screen: Vec2) -> Rect {
        Frame::new(screen).text_area()
    }

    pub(crate) fn prompts_of(screen: Vec2) -> Rect {
        Frame::new(screen).prompts()
    }

    //where the player stands in a window of a size, and how many of the ships are drawn round them there
    pub(crate) fn player_and_ships_shown(screen: Vec2) -> (Vec2, usize) {
        let frame = Frame::new(screen);
        (frame.army_middle(), ship_slots().iter().filter(|slot| frame.ship_is_clear(**slot)).count())
    }
}
