//! the champions' bolts: what their volleys fire, flying until they hit an enemy, go off where they were thrown, or give up. the host's world flies
//! them and they hurt what they hit; a guest is told only of each volley (see `Event::Volley`), makes the same bolts from it and flies them the same
//! way, hurting nothing, as what was hit is the host's to say. there are never more than BOLT_CAP of them at once, and the enemies they can hit are
//! found in a grid made each frame, so a frame's cost is the bolts, not the bolts times the enemies

use crate::prelude::*;
use std::f32::consts::{PI, TAU};

//what a bolt is: how it flies, what it does when it gets there, and how it looks
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BoltKind {
    Round, //a sharpshooter's: fast and small, it hurts the first enemy it reaches (and may go through it, or glance off to the next)
    Shell, //a pyromancer's fireball: lobbed at a place, over everything, it goes off there and burns every enemy round it
    Ember, //a little flame a shell throws out where it goes off
    Spark, //a stormcaller's: slower, and it may bend towards the enemies near it
    Tracer, //the vanguard's: small and quick, a stream of them off the front of the army (see vanguard.rs)
}

pub(crate) const BOLT_KINDS: [BoltKind; 5] = [BoltKind::Round, BoltKind::Shell, BoltKind::Ember, BoltKind::Spark, BoltKind::Tracer];

impl BoltKind {
    //how big it is (which is also how near it has to come to an enemy to hit it), and its tail: how many fading discs, how far apart
    pub(crate) fn look(self) -> (f32, usize, f32) {
        match self {
            BoltKind::Round => (4.5, 4, 10.0),
            BoltKind::Shell => (11.0, 3, 9.0),
            BoltKind::Ember => (4.0, 2, 6.0),
            BoltKind::Spark => (3.5, 3, 7.0),
            BoltKind::Tracer => (3.0, 3, 9.0),
        }
    }

    //how hard it knocks a light enemy back, as a share of KNOCKBACK
    fn weight(self) -> f32 {
        match self {
            BoltKind::Round => 1.0,
            BoltKind::Shell => 2.0,
            BoltKind::Ember | BoltKind::Tracer => 0.4,
            BoltKind::Spark => 0.6,
        }
    }
}

//the most a bolt reaches out to hit an enemy, which the grid allows for
const BOLT_MAX_RADIUS: f32 = 4.5;

//how a volley's bolts go out
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum VolleyShape {
    Burst, //one after another along the aim, a little scattered: a stream of tracers, or shells that land round one place
    Fan,   //side by side across `scatter` radians, the middle of them along the aim
    Ring,  //all the way round, the first along the aim
}

pub(crate) const VOLLEY_SHAPES: [VolleyShape; 3] = [VolleyShape::Burst, VolleyShape::Fan, VolleyShape::Ring];

//one volley, as a champion fires it and as a guest is told of it: enough to make the same bolts on any machine
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Volley {
    pub(crate) from: Vec2,
    pub(crate) aim: Vec2, //the way it is fired (any length but nothing)
    pub(crate) kind: BoltKind,
    pub(crate) shape: VolleyShape,
    pub(crate) count: u8,
    pub(crate) scatter: f32, //radians across a fan, or how far a burst wanders off its aim; for shells, pixels round the place they land
    pub(crate) speed: f32,   //pixels a second
    pub(crate) reach: f32,   //how far a bolt flies before it gives up, or how far away a shell lands
    pub(crate) pierce: u8,   //enemies a bolt goes through before it is used up
    pub(crate) bounce: u8,   //times a bolt glances off what it hits to the nearest enemy it has not hit
    pub(crate) homing: f32,  //radians a second a bolt turns towards the nearest enemy
    pub(crate) splash: f32,  //how far round a shell its blast reaches
    pub(crate) embers: u8,   //how many embers a shell throws out where it goes off
    pub(crate) champion: u8, //which kind of champion fired it, plus one, for the colour
    pub(crate) seed: u32,    //what the scatter is made from, which is the same on every machine
}

impl Volley {
    //whether a volley a message tells of is one a champion could have fired: no more bolts than MAX_BOLTS_A_VOLLEY, flying no faster or further than
    //MAX_BOLT_SPEED and MAX_BOLT_REACH, of a champion there is
    pub(crate) fn is_believable(&self) -> bool {
        self.count <= MAX_BOLTS_A_VOLLEY
            && self.embers <= MAX_BOLTS_A_VOLLEY
            && self.pierce <= MAX_PIERCE
            && self.bounce <= MAX_BOUNCE
            && self.speed > 0.0
            && self.speed <= MAX_BOLT_SPEED
            && (0.0..=MAX_BOLT_REACH).contains(&self.reach)
            && self.scatter.abs() <= MAX_BOLT_REACH
            && (0.0..=MAX_BOLT_HOMING).contains(&self.homing)
            && (0.0..=MAX_BOLT_REACH).contains(&self.splash)
            && (self.champion as usize) <= CHAMPIONS.len()
    }
}

//what a volley's bolts do to what they hit, which only the host's world knows: the damage to each enemy each bolt hits (`ember` for the embers a
//shell throws out), before a boss's share of it (see `boss_share`), for the player who has the credit. nothing at all on a guest's machine
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(crate) struct Hurt {
    pub(crate) each: f32,
    pub(crate) ember: f32,
    pub(crate) owner: u8,
}

//the enemies one bolt can hit: one, then one more for each it can go through or glance off
const MAX_STRUCK: usize = 1 + MAX_PIERCE as usize + MAX_BOUNCE as usize;

//one bolt in flight
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Bolt {
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2,
    pub(crate) wait: f32,   //seconds before it sets off (the later rounds of a burst, the later shells)
    pub(crate) life: f32,   //seconds it has left
    pub(crate) flight: f32, //seconds it had to begin with, which a shell's arc and a tail's length are worked out from
    pub(crate) kind: BoltKind,
    pub(crate) pierce: u8,
    pub(crate) bounce: u8,
    pub(crate) homing: f32,
    pub(crate) splash: f32,
    pub(crate) embers: u8,
    pub(crate) champion: u8,
    pub(crate) seed: u32,
    pub(crate) hurt: Hurt,
    pub(crate) struck: [u32; MAX_STRUCK], //the enemies (by id) it has hit, so it hurts none of them twice
    pub(crate) hits: u8,
}

//a number from 0 to 1 made from a seed and a place in a row: the same on every machine, which the game's own random numbers are not
pub(crate) fn hash_noise(seed: u32, at: u32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E37_79B9) ^ at.wrapping_mul(0x85EB_CA6B).wrapping_add(0x27D4_EB2F);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x as f32 / u32::MAX as f32
}

static NEXT_VOLLEY: AtomicU32 = AtomicU32::new(1);

//the seed of a new volley, which is all its scatter is made from
pub(crate) fn volley_seed() -> u32 {
    NEXT_VOLLEY.fetch_add(1, Ordering::Relaxed).wrapping_mul(0x9E37_79B9)
}

//the colour of a champion's bolts, its own
//(the vanguard's tracers, which no champion fired, are a hot white-gold)
fn bolt_color(champion: u8) -> Color {
    (champion as usize).checked_sub(1).and_then(|kind| CHAMPIONS.get(kind)).map_or(Color::new(1.0, 0.88, 0.5, 1.0), |kind| kind.color)
}

//a bolt of a volley setting off from `at` along `direction`, for `life` seconds, after `wait`
fn bolt_of(volley: &Volley, hurt: Hurt, at: Vec2, direction: Vec2, speed: f32, life: f32, wait: f32) -> Bolt {
    Bolt {
        position: at,
        velocity: direction * speed,
        wait,
        life,
        flight: life,
        kind: volley.kind,
        pierce: volley.pierce.min(MAX_PIERCE),
        bounce: volley.bounce.min(MAX_BOUNCE),
        homing: volley.homing,
        splash: volley.splash,
        embers: volley.embers.min(MAX_BOLTS_A_VOLLEY),
        champion: volley.champion,
        seed: volley.seed,
        hurt,
        struck: [0; MAX_STRUCK],
        hits: 0,
    }
}

//the bolts of a volley, put in flight (as many as there is room for under BOLT_CAP). `hurt` is what they do, nothing on a guest's machine. the
//same volley makes the same bolts wherever it is launched
pub(crate) fn launch_volley(bolts: &mut Vec<Bolt>, volley: &Volley, hurt: Hurt) {
    let aim = volley.aim.try_normalize().unwrap_or(Vec2::X);
    let count = volley.count.min(MAX_BOLTS_A_VOLLEY) as usize;
    for i in 0..count {
        if bolts.len() >= BOLT_CAP {
            return;
        }
        let wobble = hash_noise(volley.seed, i as u32) - 0.5;
        let bolt = match (volley.kind, volley.shape) {
            //shells land round the place they were thrown at, the first on it, the rest scattered and a little after
            (BoltKind::Shell, _) => {
                let mut landing = volley.from + aim * volley.reach;
                if i > 0 {
                    let angle = hash_noise(volley.seed, 100 + i as u32) * TAU;
                    landing += Vec2::from_angle(angle) * volley.scatter * (0.5 + 0.5 * hash_noise(volley.seed, 200 + i as u32));
                }
                let distance = landing.distance(volley.from);
                let flight = (distance / volley.speed).max(SHELL_FLIGHT);
                let direction = (landing - volley.from).try_normalize().unwrap_or(aim);
                bolt_of(volley, hurt, volley.from, direction, distance / flight, flight, i as f32 * SHELL_GAP)
            }
            (_, VolleyShape::Burst) => {
                let direction = Vec2::from_angle(wobble * volley.scatter).rotate(aim);
                bolt_of(volley, hurt, volley.from, direction, volley.speed, volley.reach / volley.speed, i as f32 * BURST_GAP)
            }
            (_, VolleyShape::Fan) => {
                let across = if count == 1 { 0.0 } else { -volley.scatter / 2.0 + volley.scatter * i as f32 / (count - 1) as f32 };
                let direction = Vec2::from_angle(across + wobble * 0.02).rotate(aim);
                bolt_of(volley, hurt, volley.from, direction, volley.speed, volley.reach / volley.speed, 0.0)
            }
            (_, VolleyShape::Ring) => {
                let direction = Vec2::from_angle(TAU * i as f32 / count as f32 + wobble * 0.05).rotate(aim);
                bolt_of(volley, hurt, volley.from, direction, volley.speed, volley.reach / volley.speed, 0.0)
            }
        };
        bolts.push(bolt);
    }
}

//on a guest's machine, the bolts of the volleys the host has told of, which hurt nothing
pub(crate) fn launch_told(bolts: &mut Vec<Bolt>, events: &[Event]) {
    for event in events {
        if let Event::Volley(volley) = event {
            launch_volley(bolts, volley, Hurt::default());
        }
    }
}

//the enemies a bolt can hit, by the square of BOLT_CELL they reach into, over the part of the world the bolts are in and as far round it as one
//can fly, turn or glance off in a frame: each is in every square it overlaps, so a bolt only has to look in its own
struct EnemyGrid {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    starts: Vec<u32>, //where each square's enemies begin in `items`, square by square, and at the end where the last ends
    items: Vec<u32>,  //the enemies' places in the list
}

//how far round the bolts the grid reaches, and the most squares it has a side however far apart they are
const GRID_MARGIN: f32 = RICOCHET_RANGE + HOMING_REACH + 2.0 * BOLT_CELL;
const GRID_MOST: i32 = 512;

fn cell_of(at: Vec2) -> (i32, i32) {
    ((at.x / BOLT_CELL).floor() as i32, (at.y / BOLT_CELL).floor() as i32)
}

impl EnemyGrid {
    //the ones that are there to be hit, alive and not lying in wait, sorted into their squares in two goes: counted, then put in
    fn of(enemies: &[Enemy], bolts: &[Bolt]) -> EnemyGrid {
        let (low, high) = bolts.iter().fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(low, high), bolt| (low.min(bolt.position), high.max(bolt.position)));
        let (left, top) = cell_of(low - Vec2::splat(GRID_MARGIN));
        let (right, bottom) = cell_of(high + Vec2::splat(GRID_MARGIN));
        let (width, height) = ((right - left + 1).clamp(1, GRID_MOST), (bottom - top + 1).clamp(1, GRID_MOST));
        let mut grid = EnemyGrid { left, top, width, height, starts: vec![0; (width * height + 1) as usize], items: Vec::new() };
        let spans: Vec<(u32, (i32, i32), (i32, i32))> = enemies
            .iter()
            .enumerate()
            .filter(|(_, enemy)| enemy.health > 0.0 && !enemy.lurking())
            .filter_map(|(i, enemy)| {
                let reach = Vec2::splat(enemy.size / 2.0 + BOLT_MAX_RADIUS);
                let ((from_col, from_row), (to_col, to_row)) = (cell_of(enemy.position - reach), cell_of(enemy.position + reach));
                let (from_col, from_row) = (from_col.max(left), from_row.max(top));
                let (to_col, to_row) = (to_col.min(left + width - 1), to_row.min(top + height - 1));
                (from_col <= to_col && from_row <= to_row).then_some((i as u32, (from_col, from_row), (to_col, to_row)))
            })
            .collect();
        for &(_, (from_col, from_row), (to_col, to_row)) in &spans {
            for row in from_row..=to_row {
                for col in from_col..=to_col {
                    let at = grid.index(col, row);
                    grid.starts[at + 1] += 1;
                }
            }
        }
        for at in 1..grid.starts.len() {
            grid.starts[at] += grid.starts[at - 1];
        }
        grid.items = vec![0; grid.starts[grid.starts.len() - 1] as usize];
        let mut next = grid.starts.clone();
        for &(i, (from_col, from_row), (to_col, to_row)) in &spans {
            for row in from_row..=to_row {
                for col in from_col..=to_col {
                    let at = grid.index(col, row);
                    grid.items[next[at] as usize] = i;
                    next[at] += 1;
                }
            }
        }
        grid
    }

    fn index(&self, col: i32, row: i32) -> usize {
        ((row - self.top) * self.width + (col - self.left)) as usize
    }

    fn in_cell(&self, col: i32, row: i32) -> &[u32] {
        if col < self.left || row < self.top || col >= self.left + self.width || row >= self.top + self.height {
            return &[];
        }
        let at = self.index(col, row);
        &self.items[self.starts[at] as usize..self.starts[at + 1] as usize]
    }

    //the enemy a bolt of `radius` at a place is touching, if any, leaving out the ones in `skip` (by id)
    fn touching(&self, enemies: &[Enemy], at: Vec2, radius: f32, skip: &[u32]) -> Option<usize> {
        let (col, row) = cell_of(at);
        self.in_cell(col, row).iter().map(|&i| i as usize).find(|&i| {
            let enemy = &enemies[i];
            enemy.health > 0.0 && !skip.contains(&enemy.id) && enemy.position.distance(at) <= enemy.size / 2.0 + radius
        })
    }

    //the nearest enemy whose edge is within `reach` of a place, leaving out the ones in `skip`. it looks a ring of squares at a time, outwards,
    //and stops once the next ring can hold nothing nearer than what it has found
    fn nearest(&self, enemies: &[Enemy], at: Vec2, reach: f32, skip: &[u32]) -> Option<usize> {
        let (col, row) = cell_of(at);
        let rings = (reach / BOLT_CELL).ceil() as i32 + 1;
        let mut best: Option<(f32, usize)> = None;
        for ring in 0..=rings {
            if best.is_some_and(|(gap, _)| gap <= (ring - 1) as f32 * BOLT_CELL - BOLT_MAX_RADIUS) {
                break;
            }
            for down in -ring..=ring {
                for across in -ring..=ring {
                    if down.abs().max(across.abs()) != ring {
                        continue;
                    }
                    for &i in self.in_cell(col + across, row + down) {
                        let enemy = &enemies[i as usize];
                        if enemy.health <= 0.0 || skip.contains(&enemy.id) {
                            continue;
                        }
                        let gap = enemy.position.distance(at) - enemy.size / 2.0;
                        if gap <= reach && best.is_none_or(|(nearest, _)| gap < nearest) {
                            best = Some((gap, i as usize));
                        }
                    }
                }
            }
        }
        best.map(|(_, i)| i)
    }
}

//a velocity turned towards a way, by no more than `most` radians, at the same speed
fn turned_towards(velocity: Vec2, towards: Vec2, most: f32) -> Vec2 {
    let now = velocity.y.atan2(velocity.x);
    let turn = (towards.y.atan2(towards.x) - now + PI).rem_euclid(TAU) - PI;
    Vec2::from_angle(now + turn.clamp(-most, most)) * velocity.length()
}

//a light enemy is knocked back by what hits it, kept inside the world
fn knock(enemy: &mut Enemy, push: Vec2) {
    if enemy.boss.is_none() && enemy.size < KNOCKBACK_SIZE {
        let edge = Vec2::splat(enemy.size / 2.0);
        enemy.position = (enemy.position + push).clamp(edge, Vec2::splat(WORLD_SIZE) - edge);
    }
}

//a shell going off where it landed: every enemy in its blast is hurt, and it shows (only on the host's machine, which says so to the others)
fn blast(bolt: &Bolt, enemies: &mut [Enemy], fx: Option<&mut Effects>) {
    let Some(fx) = fx else {
        return;
    };
    let (at, color) = (bolt.position, bolt_color(bolt.champion));
    for enemy in enemies.iter_mut().filter(|enemy| in_reach(enemy, at, bolt.splash)) {
        let damage = bolt.hurt.each * boss_share(enemy);
        strike(enemy, damage, bolt.hurt.owner);
        let away = (enemy.position - at).try_normalize().unwrap_or(Vec2::X);
        knock(enemy, away * KNOCKBACK * BoltKind::Shell.weight());
    }
    fx.ring(at, bolt.splash, color);
    fx.burst(at, color, 36, 340.0);
    fx.mark(at, bolt.splash * 0.8, color, MarkKind::Scorch);
    fx.add_shake(0.1);
    fx.cue_at(Cue::Explosion, at);
}

//the embers a shell throws out where it went off, in a ring: the same on every machine
fn embers_of(shell: &Bolt) -> Vec<Bolt> {
    let count = shell.embers as usize;
    let turn = hash_noise(shell.seed, 1000) * TAU;
    (0..count)
        .map(|k| {
            let direction = Vec2::from_angle(turn + TAU * k as f32 / count as f32);
            Bolt {
                velocity: direction * EMBER_SPEED,
                wait: 0.0,
                life: EMBER_REACH / EMBER_SPEED,
                flight: EMBER_REACH / EMBER_SPEED,
                kind: BoltKind::Ember,
                pierce: 0,
                bounce: 0,
                homing: 0.0,
                splash: 0.0,
                embers: 0,
                hurt: Hurt { each: shell.hurt.ember, ember: 0.0, owner: shell.hurt.owner },
                struck: [0; MAX_STRUCK],
                hits: 0,
                ..*shell
            }
        })
        .collect()
}

//a frame of the bolts' flight. with `fx`, this is the host's world: they hurt what they hit, and a few puffs show where. without (on a guest's
//machine) they fly the same way and stop where they would have, and what they hit is the host's to say. a bolt goes a little at a time so it
//can't jump over an enemy, and one that leaves the world is gone
pub(crate) fn fly_bolts(bolts: &mut Vec<Bolt>, enemies: &mut [Enemy], dt: f32, mut fx: Option<&mut Effects>) {
    if bolts.is_empty() || dt <= 0.0 {
        return;
    }
    let grid = EnemyGrid::of(enemies, bolts);
    let mut thrown: Vec<Bolt> = Vec::new();
    let (mut puffs, mut crits, mut ticked) = (0, 0, false);
    bolts.retain_mut(|bolt| {
        if bolt.wait > 0.0 {
            bolt.wait -= dt;
            return true;
        }
        let left = bolt.life;
        bolt.life -= dt;
        //a shell flies over everything to where it was thrown, and goes off there
        if bolt.kind == BoltKind::Shell {
            bolt.position += bolt.velocity * dt.min(left.max(0.0));
            if bolt.life <= 0.0 {
                blast(bolt, enemies, fx.as_deref_mut());
                thrown.extend(embers_of(bolt));
                return false;
            }
            return true;
        }
        if bolt.life <= 0.0 {
            return false;
        }
        let struck = bolt.hits as usize;
        if bolt.homing > 0.0 {
            if let Some(i) = grid.nearest(enemies, bolt.position, HOMING_REACH, &bolt.struck[..struck]) {
                bolt.velocity = turned_towards(bolt.velocity, enemies[i].position - bolt.position, bolt.homing * dt);
            }
        }
        let (radius, _, _) = bolt.kind.look();
        let steps = (bolt.velocity.length() * dt / BOLT_STEP).ceil().max(1.0) as usize;
        for _ in 0..steps {
            bolt.position += bolt.velocity * dt / steps as f32;
            if !(0.0..WORLD_SIZE).contains(&bolt.position.x) || !(0.0..WORLD_SIZE).contains(&bolt.position.y) {
                return false;
            }
            let Some(i) = grid.touching(enemies, bolt.position, radius, &bolt.struck[..bolt.hits as usize]) else {
                continue;
            };
            enemies[i].struck = HIT_FLASH;
            if let Some(fx) = fx.as_deref_mut() {
                //a champion's bolt now and then hits for BOLT_CRIT times as much, which shows, sounds, and holds a world of one still for a moment
                let crit = bolt.champion != 0 && hash_noise(bolt.seed, 7000 + bolt.hits as u32) < BOLT_CRIT_CHANCE;
                let enemy = &mut enemies[i];
                let damage = bolt.hurt.each * boss_share(enemy) * if crit { BOLT_CRIT } else { 1.0 };
                strike(enemy, damage, bolt.hurt.owner);
                knock(enemy, bolt.velocity.normalize_or_zero() * KNOCKBACK * bolt.kind.weight());
                let color = bolt_color(bolt.champion);
                if puffs < BOLT_PUFFS {
                    fx.burst(bolt.position, color, 4, 150.0);
                    puffs += 1;
                }
                if crit && crits < BOLT_CRIT_POPUPS {
                    fx.popup(enemy.position, enemy.size / 2.0 + 12.0, 14.0, format!("{}!", compact_number(damage)), lighten(color, 0.3));
                    fx.burst(bolt.position, WHITE, 10, 260.0);
                    fx.cue_at(Cue::Crit, bolt.position);
                    fx.hit_stop = HIT_STOP;
                    crits += 1;
                } else if !ticked {
                    fx.cue_at(Cue::Hit, bolt.position);
                    ticked = true;
                }
            }
            if (bolt.hits as usize) < MAX_STRUCK {
                bolt.struck[bolt.hits as usize] = enemies[i].id;
                bolt.hits += 1;
            }
            //it goes on through, or glances off to the nearest it has not hit, or is used up
            if bolt.pierce > 0 {
                bolt.pierce -= 1;
                continue;
            }
            if bolt.bounce > 0 {
                if let Some(next) = grid.nearest(enemies, bolt.position, RICOCHET_RANGE, &bolt.struck[..bolt.hits as usize]) {
                    let speed = bolt.velocity.length();
                    let way = enemies[next].position - bolt.position;
                    bolt.velocity = way.try_normalize().unwrap_or(Vec2::X) * speed;
                    bolt.life = bolt.life.max(way.length() / speed.max(1.0) + 0.1);
                    bolt.bounce -= 1;
                    continue;
                }
            }
            return false;
        }
        true
    });
    for ember in thrown {
        if bolts.len() >= BOLT_CAP {
            break;
        }
        bolts.push(ember);
    }
}

//the bolts in view, as discs: each a bright core with a glow round it and a short tail of fading discs behind, in its champion's colour. a shell is
//drawn up off the ground on its arc, with its shadow under it where it really is. zoomed out past BOLT_DETAIL_ZOOM there is the core alone, a little
//bigger so it is still seen
pub(crate) fn bolt_discs(bolts: &[Bolt], view: Rect, zoom: f32) -> Vec<Disc> {
    let detail = zoom >= BOLT_DETAIL_ZOOM;
    let seen = Rect::new(view.x - 80.0, view.y - 80.0, view.w + 160.0, view.h + 160.0);
    let mut discs = Vec::with_capacity(bolts.len() * 6);
    for bolt in bolts.iter().filter(|bolt| bolt.wait <= 0.0 && seen.contains(bolt.position)) {
        let color = bolt_color(bolt.champion);
        let (radius, tail, gap) = bolt.kind.look();
        let speed = bolt.velocity.length();
        let along = bolt.velocity / speed.max(0.001);
        let gone = (bolt.flight - bolt.life).max(0.0) * speed;
        let mut at = bolt.position;
        if !detail {
            discs.push(Disc::new(at.x, at.y, radius * 1.6, lighten(color, 0.4)));
            continue;
        }
        if bolt.kind == BoltKind::Shell {
            discs.push(Disc::new(at.x, at.y, radius * 1.1, Color::new(0.0, 0.0, 0.0, 0.35)));
            let t = (1.0 - bolt.life / bolt.flight.max(0.001)).clamp(0.0, 1.0);
            at.y -= speed * bolt.flight * SHELL_ARC * 4.0 * t * (1.0 - t);
        }
        discs.push(Disc::new(at.x, at.y, radius * 2.4, Color::new(color.r, color.g, color.b, 0.22)));
        for k in (1..=tail).filter(|&k| gap * k as f32 <= gone) {
            let share = 1.0 - k as f32 / (tail + 1) as f32;
            let spot = at - along * gap * k as f32;
            discs.push(Disc::new(spot.x, spot.y, radius * (0.4 + 0.5 * share), Color::new(color.r, color.g, color.b, 0.55 * share)));
        }
        discs.push(Disc::new(at.x, at.y, radius, lighten(color, 0.6)));
    }
    discs
}

pub(crate) fn draw_bolts(bolts: &[Bolt], view: Rect, zoom: f32, camera: &Camera2D) {
    draw_discs(Layer::Bolts, &bolt_discs(bolts, view, zoom), camera);
}
