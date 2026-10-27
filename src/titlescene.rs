//! the picture on the title screen: the player running at the head of an army of ships, a few of what stands in their way, and a boss over it all. it is drawn with the
//! game's own art (the discs the army is drawn with, the runner, the enemies, and the boss as it looks in a game, without the circles it puts down) in a world of its own
//! that is only a backdrop: nothing in it is really a unit or an enemy, and all that is stepped is the figure's stride. which boss, which enemies and where each of them
//! stands is one of the looks in homescreens.rs, and where things stand is worked out from the size of the window each frame, with the column of words left clear

use crate::prelude::*;

const SCALE: f32 = 1.15; //how much bigger than in a game things are drawn, in a window of the size the layout is made for...
const DESIGN_HEIGHT: f32 = 720.0; //...which is this tall...
const DESIGN_WIDTH: f32 = 1100.0; //...and this wide (a window smaller than that in either way draws it all smaller)
const PARALLAX: f32 = 16.0; //pixels the nearest things move when the mouse goes from the middle of the screen to an edge
//a creature's picture reaches well past its hitbox (see PICTURE), so the enemies here are drawn smaller than the size of their kind, as this share of it
const FOE_ART: f32 = 0.7;
const BOSS_LEAN: f32 = 0.3; //radians a boss leans towards the army at the most, from upright
const GROUND_LINES_REACH: f32 = 330.0; //the lines of the ground going by only go this far from the edge the army runs from, so that none runs through the words

//one of the enemies, with where it stands
struct Foe {
    name: &'static str,
    at: Spot,
    scale: f32,
    phase: f32, //where it is in its bobbing
}

pub(crate) struct TitleScene {
    home: usize, //which of the looks it has
    runner: Runner,
    units: Vec<Unit>,
    slots: Vec<Vec2>, //where each of the units stands in relation to the player, in the world of the scene
    enemies: Vec<Enemy>,
    foes: Vec<Foe>,
    squads: Squads, //(none: nobody is sent at anything here)
}

fn kind_named(name: &str) -> &'static EnemyKind {
    ENEMY_KINDS.iter().find(|kind| kind.name == name).expect("a kind of enemy")
}

//0 to 1, eased: how far something that arrives `delay` seconds after the screen came up and takes `length` seconds to get there has come
fn arrival(time: f32, delay: f32, length: f32) -> f32 {
    let k = ((time - delay) / length).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

//how many times as big as the design everything is drawn in a window of a size
pub(crate) fn design_scale(screen: Vec2) -> f32 {
    (screen.y / DESIGN_HEIGHT).min(screen.x / DESIGN_WIDTH)
}

//where the screen is, and what that makes of the design: how much bigger or smaller everything is, and the camera that looks at it
struct Frame {
    screen: Vec2,
    scale: f32,
    zoom: f32,
    home: &'static Homescreen,
    army: Vec2, //where the player is, see `army_spot`
}

impl Frame {
    fn new(screen: Vec2, home: &'static Homescreen) -> Frame {
        let scale = design_scale(screen);
        let mut frame = Frame { screen, scale, zoom: scale * SCALE, home, army: Vec2::ZERO };
        frame.army = frame.army_spot();
        frame
    }

    //where a place on the screen, in pixels, is in the world of the scene, which has the middle of the screen as its middle
    fn world(&self, pixel: Vec2) -> Vec2 {
        (pixel - self.screen / 2.0) / self.zoom
    }

    //the camera for a layer of the scene, which is moved `shift` pixels over (the nearer layers more than the far ones, as the mouse moves)
    fn camera(&self, shift: Vec2) -> Camera2D {
        follow_camera(-shift.x / self.zoom, -shift.y / self.zoom, self.zoom)
    }

    //where a spot of the design is in the window
    fn spot(&self, spot: Spot) -> Vec2 {
        spot.at(self.screen, self.scale)
    }

    //where the words are, which the picture keeps off: the letters of the title, the line under it, and the prompts under that, each as wide as the widest of its lines
    //(they are sized by the height of the window, so these are too). an enemy that would stand in them in a window of an odd shape is left out
    fn words(&self) -> [Rect; 3] {
        words_of(self.home, self.screen, self.scale)
    }

    //the square an enemy takes up when it stands where it does
    #[cfg(test)]
    fn foe_box(&self, foe: &Placed) -> Rect {
        let (middle, half) = (self.spot(foe.at), kind_named(foe.name).size * foe.scale * self.zoom / 2.0);
        Rect::new(middle.x - half, middle.y - half, half * 2.0, half * 2.0)
    }

    //how far from the player the army reaches, in pixels
    fn army_reach(&self) -> f32 {
        let army = &self.home.army;
        (army.outer.1 + RADIUS * 1.5) * self.zoom * army.zoom
    }

    //where the player is, in the middle of the army
    fn army_middle(&self) -> Vec2 {
        self.army
    }

    //how far from the middle of the player the top of their head is, which is about as far as their feet, in pixels
    fn player_half(&self) -> f32 {
        let army = &self.home.army;
        RADIUS * 2.0 * self.zoom * army.zoom * army.hero
    }

    //where the player is in a window: where the look puts them, and in a window that is not the shape the picture is made for, the nearest place to that where the army is
    //off the words and the button in the corner. the sides of the army may run off the screen, as they do coming in, but not the player (a head cut off by the top of
    //the window is not much of a picture), so it is somewhere in the window that leaves them whole
    fn army_spot(&self) -> Vec2 {
        let reach = self.army_reach();
        let (w, h) = (self.screen.x, self.screen.y);
        let body = self.player_half();
        let (left, right) = ((reach * 0.5).max(body * 1.1).min(w / 2.0), (w - (reach * 0.5).max(body * 1.1)).max(w / 2.0));
        let (top, bottom) = ((reach * 0.55).max(body * 1.15).min(h / 2.0), (h - (reach * 0.3).max(body * 1.15)).max(h / 2.0));
        let spot = self.spot(self.home.army.at);
        let wanted = Vec2::new(spot.x.clamp(left, right), spot.y.clamp(top, bottom));
        let mut keep_off: Vec<Rect> = self.words().to_vec();
        keep_off.push(menu_button_of(self.screen));
        //how far the army would reach into the words and the button from a place, all in all (nothing if it is clear of them)
        let overlap = |at: Vec2| -> f32 {
            keep_off
                .iter()
                .map(|rect| {
                    let nearest = Vec2::new(at.x.clamp(rect.x, rect.right()), at.y.clamp(rect.y, rect.bottom()));
                    (reach + 6.0 - nearest.distance(at)).max(0.0)
                })
                .sum()
        };
        //a place that is clear is better than any that is not, and the nearer to where it is wanted the better of the clear ones
        let cost = |at: Vec2| {
            let over = overlap(at);
            at.distance(wanted) + if over > 0.0 { 1.0e6 + over * 100.0 } else { 0.0 }
        };
        let mut best = (cost(wanted), wanted);
        let mut consider = |at: Vec2| {
            let at = Vec2::new(at.x.clamp(left, right), at.y.clamp(top, bottom));
            let worth = cost(at);
            if worth < best.0 {
                best = (worth, at);
            }
        };
        //(just clear of each of them, to each side, and then all over the window)
        for rect in &keep_off {
            consider(Vec2::new(rect.x - reach - 6.0, wanted.y));
            consider(Vec2::new(rect.right() + reach + 6.0, wanted.y));
            consider(Vec2::new(wanted.x, rect.y - reach - 6.0));
            consider(Vec2::new(wanted.x, rect.bottom() + reach + 6.0));
        }
        let step = (reach / 12.0).max(8.0);
        let mut y = top;
        while y <= bottom {
            let mut x = left;
            while x <= right {
                consider(Vec2::new(x, y));
                x += step;
            }
            y += step;
        }
        best.1
    }

    //whether a ship that stands `slot` from the player (in the world of the scene) is clear of the words, as the army rests
    fn ship_is_clear(&self, slot: Vec2) -> bool {
        let army = &self.home.army;
        let (place, half) = (self.army_middle() + slot * self.zoom * army.zoom, RADIUS * 1.5 * self.zoom * army.zoom);
        let ship = Rect::new(place.x - half, place.y - half, half * 2.0, half * 2.0);
        !self.words().iter().any(|words| words.overlaps(&ship))
    }
}

//where the words of a look are in a window of a size, when the design is drawn `scale` times as big: see `Frame::words`
pub(crate) fn words_of(home: &Homescreen, screen: Vec2, scale: f32) -> [Rect; 3] {
    let (words, h) = (&home.words, screen.y);
    let (center, width) = (words.center(screen, scale), words.width(screen));
    let rect = |wide: f32, top: f32, bottom: f32| {
        let wide = wide.min(screen.x);
        Rect::new(center - wide / 2.0, top * h, wide, (bottom - top) * h)
    };
    let (title_y, prompts_y, size) = (words.title_y, words.prompts_y, words.size);
    [
        rect(width, title_y - 0.17 * size, title_y + 0.05 * size),
        rect(0.82 * h * size, title_y + 0.115 * size, title_y + 0.24 * size),
        rect(0.74 * h * size, prompts_y - 0.02 * size, prompts_y + 0.18 * size),
    ]
}

//where the button in the bottom left corner is, which opens the menu: as the screen draws it, near enough
pub(crate) fn menu_button_of(screen: Vec2) -> Rect {
    let size = (screen.y * 0.034).round().max(15.0);
    Rect::new(24.0, screen.y - size * 2.4 - 24.0, size * 5.2 + 48.0, size * 2.4)
}

//where the ships stand round the player: a ring close in, a ring further out, and a string of them behind
fn ship_slots(army: &Army) -> Vec<Vec2> {
    let ring = |(count, radius): (usize, f32), turn: f32| (0..count).map(move |k| Vec2::from_angle(turn + std::f32::consts::TAU * k as f32 / count as f32) * radius);
    let behind = if army.faces_left { 1.0 } else { -1.0 };
    let tail = (0..army.tail).map(|k| Vec2::new(behind * (army.outer.1 + 40.0 + k as f32 * 19.0), (k as f32 * 2.1).sin() * 38.0 + (k as f32 * 0.9).cos() * 12.0));
    ring(army.inner, 0.3).chain(ring(army.outer, 0.0)).chain(tail).collect()
}

//the boss as it is in a game (the creature it is, with an aura that pulses), with a glow behind it that the title is read over. it is looking at the army, but it stays on
//its feet: turned all the way it would be upside down, so it only leans towards them. with `simple` it is the plain square it was, a big one in its colour with two eyes
//that look at the army
fn draw_boss(frame: &Frame, time: f32, shift: Vec2, simple: bool, army: Vec2) {
    let home = frame.home;
    let rest = frame.spot(home.boss_at);
    //it comes in from above, slowly, or up from below, and then breathes
    let come = 1.0 - arrival(time, 0.15, 1.8);
    let from = if rest.y < frame.screen.y / 2.0 { -1.0 } else { 1.0 };
    let at = Vec2::new(rest.x, rest.y + (time * 0.8).sin() * 6.0 * frame.scale + from * come * 420.0 * frame.scale);
    let middle = frame.world(at);
    let picture = home.boss_size * (1.0 + 0.012 * (time * 1.9).sin());
    //(its picture reaches past its hitbox, which is the size of the square it was)
    let size = if simple { picture * 0.8 } else { picture / PICTURE };
    let camera = frame.camera(shift);
    set_camera(&camera);
    let color = boss_color(home.boss);
    let haze = if home.theme.light { 0.02 } else { 0.016 };
    let glow: Vec<Disc> = (0..16).map(|k| Disc::new(middle.x, middle.y, picture * 0.7 * (0.62 + 0.085 * k as f32), Color::new(color.r, color.g, color.b, haze))).collect();
    draw_discs(Layer::Marks, &glow, &camera);
    let half = size / 2.0;
    let pulse = 0.5 + 0.5 * (time * 3.0).sin();
    let width = (size * 0.02).clamp(6.0, 30.0);
    let aura = Color::new(color.r, color.g, color.b, 0.3 + 0.35 * pulse);
    let toward = (frame.world(army) - middle).normalize_or_zero();
    if simple {
        draw_rectangle_lines(middle.x - half - width, middle.y - half - width, size + width * 2.0, size + width * 2.0, width, aura);
        draw_rectangle(middle.x - half, middle.y - half, size, size, color);
        let eye = size * 0.14;
        for side in [-1.0, 1.0] {
            let eye_at = Vec2::new(middle.x + side * size * 0.22, middle.y - size * 0.1);
            draw_rectangle(eye_at.x - eye / 2.0, eye_at.y - eye / 2.0, eye, eye, WHITE);
            let pupil = eye_at + toward * eye * 0.22;
            draw_rectangle(pupil.x - eye / 4.0, pupil.y - eye / 4.0, eye / 2.0, eye / 2.0, BLACK);
        }
        return;
    }
    draw_circle_lines(middle.x, middle.y, half * 1.45, width, aura);
    //(a little more as they are further round, and swaying)
    let lean = toward.x.atan2(-toward.y).clamp(-BOSS_LEAN, BOSS_LEAN) * (0.5 + 0.5 * (time * 0.9).sin());
    if let Some(shape) = shape_named(home.boss) {
        draw_creatures(&[Creature::new(shape, middle, size, Vec2::new(lean.sin(), -lean.cos()), color)], &camera);
    }
}

impl TitleScene {
    //the scene in one of the looks of the title screen (a number that is past the last goes round to the first)
    pub(crate) fn new(home: usize) -> TitleScene {
        let home = home % HOMESCREENS.len();
        let look = &HOMESCREENS[home];
        let slots = ship_slots(&look.army);
        let nose = if look.army.faces_left { -Vec2::X } else { Vec2::X };
        let units = slots
            .iter()
            .map(|_| {
                let mut unit = new_unit(Vec2::ZERO);
                unit.age = 99.0;
                unit.nose = nose;
                unit
            })
            .collect();
        //the farthest up the screen are drawn first, so the nearer cover them
        let reference = Vec2::new(DESIGN_WIDTH, DESIGN_HEIGHT);
        let mut made: Vec<(Enemy, Foe)> = look
            .foes
            .iter()
            .enumerate()
            .map(|(k, placed)| {
                let mut enemy = Enemy::new(Vec2::splat(WORLD_SIZE / 2.0), kind_named(placed.name), 1.0);
                //(the same size every time, the number the kind says: it was only nudged so that no two in a game are the same)
                enemy.size = kind_named(placed.name).size * placed.scale * FOE_ART;
                enemy.chasing = placed.chasing;
                enemy.health = enemy.max_health * placed.health;
                (enemy, Foe { name: placed.name, at: placed.at, scale: placed.scale, phase: k as f32 * 1.9 })
            })
            .collect();
        made.sort_by(|a, b| a.1.at.at(reference, 1.0).y.total_cmp(&b.1.at.at(reference, 1.0).y));
        let (enemies, foes): (Vec<Enemy>, Vec<Foe>) = made.into_iter().unzip();
        let mut runner = Runner::new();
        if look.army.faces_left {
            runner.side = -1.0;
            runner.flip = -1.0;
        }
        TitleScene { home, runner, units, slots, enemies, foes, squads: Squads::new() }
    }

    //the look it is in
    pub(crate) fn home(&self) -> &'static Homescreen {
        &HOMESCREENS[self.home]
    }

    pub(crate) fn index(&self) -> usize {
        self.home
    }

    //a frame later: the figure runs
    pub(crate) fn update(&mut self, dt: f32) {
        let way = if self.home().army.faces_left { -1.0 } else { 1.0 };
        self.runner.step(Vec2::new(PLAYER_SPEED * way, 0.0), dt);
    }

    //the picture, over the backdrop and under the words. `time` is how many seconds the screen has been up, `mouse` where the pointer is (the nearer things
    //move a little as it does), `shape` what the units are drawn as, as in a game, and `simple` whether the enemies are plain squares, as in a game
    pub(crate) fn draw(&mut self, time: f32, mouse: Vec2, shape: UnitShape, simple: bool) {
        let frame = Frame::new(Vec2::new(screen_width(), screen_height()), self.home());
        let look = ((mouse - frame.screen / 2.0) / (frame.screen / 2.0)).clamp(Vec2::splat(-1.0), Vec2::splat(1.0));
        let shift = |depth: f32| -look * PARALLAX * depth * frame.scale;
        draw_boss(&frame, time, shift(0.35), simple, frame.army_middle());
        self.draw_foes(&frame, time, shift(0.7), simple);
        self.draw_army(&frame, time, shift(1.0), shape);
        set_default_camera();
    }

    fn draw_foes(&mut self, frame: &Frame, time: f32, shift: Vec2, simple: bool) {
        let words = frame.words();
        for (k, (enemy, foe)) in self.enemies.iter_mut().zip(&self.foes).enumerate() {
            //(drawn smaller as creatures than as squares, see FOE_ART)
            enemy.size = kind_named(foe.name).size * foe.scale * if simple { 1.0 } else { FOE_ART };
            //(one that would be in the words is not drawn)
            let rest = frame.spot(foe.at);
            let half = kind_named(foe.name).size * foe.scale * frame.zoom / 2.0;
            let bounds = Rect::new(rest.x - half, rest.y - half, half * 2.0, half * 2.0);
            if words.iter().any(|words| words.overlaps(&bounds)) {
                enemy.position = Vec2::splat(-1.0e6);
                continue;
            }
            //they come in from the side they are on a little after the army has, and then stand their ground, leaning in and out
            let come = 1.0 - arrival(time, 0.5 + 0.12 * k as f32, 1.5);
            let lean = (time * 1.6 + foe.phase).sin();
            let from = foe.at.side(frame.screen, frame.scale).x;
            let at = rest + Vec2::new(lean * 7.0 * frame.scale + from * come * 520.0 * frame.scale, (time * 2.3 + foe.phase * 1.3).cos() * 4.0 * frame.scale);
            enemy.position = frame.world(at);
        }
        let camera = frame.camera(shift);
        set_camera(&camera);
        if simple {
            draw_enemies_plain(&self.enemies, &self.squads, time, 1.0);
        } else {
            //(they all look at the army)
            let army = frame.world(frame.army_middle());
            draw_enemy_art(&self.enemies, &self.squads, time, 1.0, &camera, &mut |enemy| army - enemy.position);
        }
    }

    fn draw_army(&mut self, frame: &Frame, time: f32, shift: Vec2, shape: UnitShape) {
        let army = frame.home.army;
        let theme = &frame.home.theme;
        let way = if army.faces_left { -1.0 } else { 1.0 };
        //the army runs in from the side it runs from, and then runs on the spot while the ground goes by
        let come = 1.0 - arrival(time, 0.0, 1.4);
        let rest = frame.army_middle();
        let at = Vec2::new(rest.x - way * come * 520.0 * frame.scale, rest.y + (time * 3.1).sin() * 2.5 * frame.scale);
        let middle = frame.world(at);
        let camera = frame.camera(shift);
        //the army is drawn as big as the look says, about the middle of it staying where it is on the screen
        let target = middle - (middle - camera.target) / army.zoom;
        let big = follow_camera(target.x, target.y, frame.zoom * army.zoom);
        set_camera(&camera);
        //the lines of the ground going by, which stay in the strip by the edge the army runs from
        let reach = GROUND_LINES_REACH * frame.scale;
        let words = frame.words();
        let line = Color::new(theme.accent.r, theme.accent.g, theme.accent.b, if theme.light { 0.3 } else { 0.14 });
        for k in 0..9 {
            let row = at.y + ((k as f32 * 37.0) % 140.0 - 70.0) * frame.scale;
            let speed = (340.0 + (k % 4) as f32 * 90.0) * frame.scale;
            let length = (60.0 + (k % 3) as f32 * 45.0) * frame.scale;
            let start = (k as f32 * 211.0 - time * speed).rem_euclid(reach.max(0.0) + length) - length;
            //(measured from the edge the army runs from, and where the words are, a line that goes by in their rows stops short of them)
            let near = start.max(-40.0);
            let mut far = (start + length).min(reach);
            for rect in words.iter().filter(|rect| row + 2.0 >= rect.y && row - 2.0 <= rect.bottom()) {
                far = far.min(if way > 0.0 { rect.x - 8.0 } else { frame.screen.x - rect.right() - 8.0 });
            }
            if far > near {
                let across = |edge: f32| if way > 0.0 { edge } else { frame.screen.x - edge };
                let (from, to) = (frame.world(Vec2::new(across(near), row)), frame.world(Vec2::new(across(far), row)));
                draw_line(from.x, from.y, to.x, to.y, 1.5, line);
            }
        }
        set_camera(&big);
        //the glow behind it
        let blue = PLAYER_COLORS[0];
        let haze = if theme.light { Color::new(blue.r, blue.g, blue.b, 0.012) } else { Color::new(theme.accent.r, theme.accent.g, theme.accent.b, 0.011) };
        let glow: Vec<Disc> = (0..14).map(|k| Disc::new(middle.x, middle.y, 55.0 + 11.0 * k as f32, haze)).collect();
        draw_discs(Layer::Marks, &glow, &big);
        for (k, (unit, slot)) in self.units.iter_mut().zip(&self.slots).enumerate() {
            let phase = k as f32 * 1.37;
            let sway = Vec2::new((time * 2.1 + phase).sin() * 3.0, (time * 2.7 + phase * 1.7).cos() * 3.0);
            //(a ship that would be in the words, in a window of an odd shape, is not drawn)
            let place = if frame.ship_is_clear(*slot) { middle + *slot + sway } else { Vec2::splat(-1.0e6) };
            unit.x = place.x;
            unit.y = place.y;
            unit.nose = Vec2::from_angle(0.16 * (time * 1.7 + phase).sin() + if army.faces_left { std::f32::consts::PI } else { 0.0 });
        }
        //(what is drawn is what is on the screen, with some to spare for the nearer things moving with the mouse)
        let seen = camera_view(&big);
        let view = Rect::new(seen.x - 100.0, seen.y - 100.0, seen.w + 200.0, seen.h + 200.0);
        draw_discs(Layer::Army, &unit_discs_in(&self.units, view, None, shape, 1.0), &big);
        //the player is drawn bigger than the ships, about the same place on the screen
        let hero = frame.zoom * army.zoom * army.hero;
        let target = middle - (middle - big.target) / army.hero;
        set_camera(&follow_camera(target.x, target.y, hero));
        draw_runner(&self.runner, middle, blue, 1.0);
    }
}

#[cfg(test)]
impl TitleScene {
    //the things in a look and where they would stand at a size of window, as boxes in pixels: the boss, the army with the tail left out (it comes in from off the
    //screen), and each of the enemies. for the tests of where they are
    pub(crate) fn boxes(home: usize, screen: Vec2) -> Vec<(&'static str, Rect)> {
        let look = &HOMESCREENS[home];
        let frame = Frame::new(screen, look);
        let around = |middle: Vec2, half: f32| Rect::new(middle.x - half, middle.y - half, half * 2.0, half * 2.0);
        let mut boxes = vec![("the boss", around(frame.spot(look.boss_at), look.boss_size * frame.scale / 2.0))];
        //(the army is a round thing, so this is the square inside its circle, which is what its corners could be)
        boxes.push(("the army", around(frame.army_middle(), frame.army_reach() * 0.707)));
        for placed in look.foes {
            boxes.push((placed.name, frame.foe_box(placed)));
        }
        boxes
    }

    //how many units and enemies there are in it
    pub(crate) fn counts(&self) -> (usize, usize) {
        (self.units.len(), self.enemies.len())
    }

    //where the words of a look are in a window of a size
    pub(crate) fn words_in(home: usize, screen: Vec2) -> [Rect; 3] {
        Frame::new(screen, &HOMESCREENS[home]).words()
    }

    //how far over the middle of the player the top of their head is in a window of a size (and about as far under it are their feet), in pixels
    pub(crate) fn head_room_of(home: usize, screen: Vec2) -> f32 {
        Frame::new(screen, &HOMESCREENS[home]).player_half()
    }

    //where the player stands in a window of a size, and how many of the ships are drawn round them there
    pub(crate) fn player_and_ships_shown(home: usize, screen: Vec2) -> (Vec2, usize) {
        let frame = Frame::new(screen, &HOMESCREENS[home]);
        (frame.army_middle(), ship_slots(&frame.home.army).iter().filter(|slot| frame.ship_is_clear(**slot)).count())
    }
}
