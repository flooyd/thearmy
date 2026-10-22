//! hordes: now and then a crowd of enemies is sent at the army from one direction. it is announced a few seconds before, with a
//! banner, an arrow at the edge of the screen and a ping on the minimap where they will come from, and then they are there, all
//! running at the player together. break one, by destroying most of it, and it leaves a chest where the last of it fell (see
//! `chests.rs`). where the horde is in all this is kept with the run

use crate::prelude::*;

pub(crate) const HORDE_SITE: u32 = u32::MAX; //what a horde's enemies have for a site, so the game can tell them from the wild ones: no site has this number
pub(crate) const HORDE_COLOR: Color = Color::new(1.0, 0.6, 0.2, 1.0);

//a kind of horde: which enemies it is made of and how many
pub(crate) struct HordeDef {
    pub(crate) name: &'static str,
    pub(crate) kinds: &'static [(&'static str, f32)], //the kinds of enemy in it, and how much of it each is
    pub(crate) base: usize,                          //how many there are for the smallest army...
    pub(crate) per_units: usize,                     //...and one more for every this many units in the army...
    pub(crate) most: usize,                          //...up to this many
    pub(crate) from_army: usize,                     //it only comes once the army is this big
    pub(crate) modifier: Modifier,                   //every one of them is an elite of this kind, unless it is None
    pub(crate) prize: u32,                           //the tier of the chest it leaves when it is broken: 1 bronze, 2 silver, 3 gold
}

pub(crate) static HORDES: [HordeDef; 11] = [
    HordeDef { name: "Swarm Tide", kinds: &[("Swarmer", 0.7), ("Scout", 0.3)], base: 24, per_units: 70, most: 80, from_army: 0, modifier: Modifier::None, prize: 1 },
    HordeDef { name: "Raiding Party", kinds: &[("Grunt", 0.5), ("Scout", 0.3), ("Splitter", 0.2)], base: 12, per_units: 150, most: 40, from_army: 20, modifier: Modifier::None, prize: 1 },
    HordeDef { name: "Bomber Run", kinds: &[("Bomber", 0.65), ("Scout", 0.35)], base: 8, per_units: 250, most: 22, from_army: 40, modifier: Modifier::None, prize: 2 },
    //all splitters: every one that falls is three more
    HordeDef { name: "Fission Mob", kinds: &[("Splitter", 1.0)], base: 6, per_units: 300, most: 24, from_army: 40, modifier: Modifier::None, prize: 1 },
    //shooters that stop at a distance and fire, behind a line of shielded ones
    HordeDef { name: "Firing Line", kinds: &[("Spitter", 0.6), ("Mortar", 0.15), ("Bulwark", 0.25)], base: 6, per_units: 400, most: 16, from_army: 60, modifier: Modifier::None, prize: 2 },
    HordeDef { name: "Blood Frenzy", kinds: &[("Grunt", 0.5), ("Scout", 0.5)], base: 10, per_units: 200, most: 36, from_army: 60, modifier: Modifier::Frenzied, prize: 2 },
    HordeDef { name: "Wildfire", kinds: &[("Grunt", 0.55), ("Bomber", 0.45)], base: 8, per_units: 300, most: 26, from_army: 80, modifier: Modifier::Volatile, prize: 2 },
    HordeDef { name: "Iron Column", kinds: &[("Brute", 0.4), ("Bulwark", 0.3), ("Charger", 0.3)], base: 8, per_units: 500, most: 24, from_army: 80, modifier: Modifier::None, prize: 2 },
    //nearly all chargers, closing in and then dashing one after another
    HordeDef { name: "Stampede", kinds: &[("Charger", 0.85), ("Brute", 0.15)], base: 5, per_units: 600, most: 14, from_army: 100, modifier: Modifier::None, prize: 2 },
    HordeDef { name: "Titan March", kinds: &[("Titan", 0.6), ("Brute", 0.4)], base: 3, per_units: 2000, most: 9, from_army: 250, modifier: Modifier::None, prize: 3 },
    HordeDef { name: "Armoured Host", kinds: &[("Brute", 0.5), ("Bulwark", 0.3), ("Titan", 0.2)], base: 6, per_units: 700, most: 20, from_army: 400, modifier: Modifier::Armored, prize: 3 },
];

impl HordeDef {
    //how many enemies there are in it for an army this big
    pub(crate) fn count(&self, army: usize) -> usize {
        (self.base + army / self.per_units.max(1)).min(self.most)
    }
}

pub(crate) fn horde_named(name: &str) -> Option<&'static HordeDef> {
    HORDES.iter().find(|def| def.name == name)
}

//the horde that will turn up: what it is, where it will appear and how long until it does
#[derive(Clone, Copy)]
pub(crate) struct Coming {
    pub(crate) def: &'static HordeDef,
    pub(crate) at: Vec2,
    pub(crate) left: f32,
}

//a horde that is here and being fought, kept so the chest can be given when it is broken
#[derive(Clone, Copy)]
pub(crate) struct Fight {
    pub(crate) def: &'static HordeDef,
    pub(crate) total: usize, //how many came
    pub(crate) lost: usize,  //how many were let go of, left far behind
    pub(crate) worth: f32,   //the wall points all of them were worth
    pub(crate) last_at: Vec2, //where one of them was last seen
}

//what the hordes did in a frame, for the game to announce
pub(crate) enum HordeBeat {
    Warned { name: &'static str, from: &'static str },
    Arrived { name: &'static str, count: usize },
    Broken { name: &'static str, tier: u32 },
}

#[derive(Clone)]
pub(crate) struct Hordes {
    pub(crate) next_in: f32, //seconds until the next one is announced
    pub(crate) coming: Option<Coming>,
    pub(crate) fight: Option<Fight>,
    pub(crate) chests: Vec<Chest>, //what broken hordes have left, waiting to be opened
}

//the compass point a direction is, with north up the screen
pub(crate) fn compass(direction: Vec2) -> &'static str {
    const POINTS: [&str; 8] = ["EAST", "SOUTH-EAST", "SOUTH", "SOUTH-WEST", "WEST", "NORTH-WEST", "NORTH", "NORTH-EAST"];
    let eighth = (direction.to_angle().rem_euclid(std::f32::consts::TAU) / (std::f32::consts::TAU / 8.0)).round() as usize % 8;
    POINTS[eighth]
}

//the kind of horde for an army, picked by chance out of those that come for it
pub(crate) fn random_horde(army: usize) -> &'static HordeDef {
    let open: Vec<&HordeDef> = HORDES.iter().filter(|def| army >= def.from_army).collect();
    open[rand::gen_range(0, open.len())]
}

//where a horde appears: HORDE_DISTANCE_TILES from the player in a random direction, as long as that is in the world
pub(crate) fn horde_spot(player: Vec2) -> Option<Vec2> {
    for _ in 0..30 {
        let direction = Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU));
        let at = player + direction * HORDE_DISTANCE_TILES * TILE_SIZE;
        let edge = HORDE_EDGE_TILES * TILE_SIZE;
        if at.x > edge && at.y > edge && at.x < WORLD_SIZE - edge && at.y < WORLD_SIZE - edge {
            return Some(at);
        }
    }
    None
}

//the enemies of a horde, as made for an army (`pressure` is the size of army they are made for), crowded about a point and all
//running at the player. each is no one's: it is no part of the wild enemies that are kept up
pub(crate) fn make_horde(def: &'static HordeDef, at: Vec2, army: usize, pressure: usize) -> Vec<Enemy> {
    let count = def.count(army);
    let total: f32 = def.kinds.iter().map(|(_, share)| share).sum();
    let spread = HORDE_SPREAD + (count as f32).sqrt() * HORDE_SPREAD_PER_ROOT;
    let mut horde = Vec::with_capacity(count);
    for _ in 0..count {
        let mut pick = rand::gen_range(0.0, total);
        let mut name = def.kinds[0].0;
        for &(kind, share) in def.kinds {
            if pick < share {
                name = kind;
                break;
            }
            pick -= share;
        }
        let kind = kind_named(name);
        let place = at + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * spread * rand::gen_range(0.0, 1.0f32).sqrt();
        let mut enemy = enemy_for(place, kind, pressure);
        if def.modifier != Modifier::None {
            enemy.make_elite(def.modifier);
        } else if rand::gen_range(0.0, 1.0) < elite_chance(pressure) * 0.5 {
            enemy.make_elite(MODIFIERS[rand::gen_range(0, MODIFIERS.len())]);
        }
        join_horde(&mut enemy);
        horde.push(enemy);
    }
    horde
}

//make an enemy one of the horde: running at the player from the start. a charger keeps its own reach, so it runs until it is
//close and then dashes, rather than dashing from where it stands
pub(crate) fn join_horde(enemy: &mut Enemy) {
    enemy.site = Some(HORDE_SITE);
    enemy.chasing = true;
    if enemy.behavior != Behavior::Charger {
        enemy.aggro = enemy.aggro.max(HORDE_AGGRO_TILES * TILE_SIZE);
    }
}

pub(crate) fn in_a_horde(enemy: &Enemy) -> bool {
    enemy.site == Some(HORDE_SITE)
}

impl Hordes {
    pub(crate) fn new() -> Hordes {
        Hordes { next_in: HORDE_FIRST_SECONDS, coming: None, fight: None, chests: Vec::new() }
    }

    //count down to the next horde, announce it, and bring it when the time is up. nothing comes while a boss is about, or while the
    //army is too small to have anything to fear from a crowd. the ones left far behind are let go, and a horde that is gone,
    //mostly destroyed, leaves its chest
    pub(crate) fn update(&mut self, enemies: &mut Vec<Enemy>, player: Vec2, army: usize, pressure: usize, boss_busy: bool, dt: f32) -> Option<HordeBeat> {
        let leash = ENEMY_LEASH_TILES * TILE_SIZE;
        let before = enemies.len();
        enemies.retain(|enemy| !in_a_horde(enemy) || enemy.position.distance_squared(player) <= leash * leash);
        let let_go = before - enemies.len();
        for chest in &mut self.chests {
            chest.left -= dt;
        }
        self.chests.retain(|chest| chest.left > 0.0);
        if let Some(fight) = self.fight.as_mut() {
            fight.lost += let_go;
            match enemies.iter().find(|enemy| in_a_horde(enemy)) {
                Some(member) => fight.last_at = member.position,
                None => {
                    let fight = *fight;
                    self.fight = None;
                    let destroyed = fight.total - fight.lost.min(fight.total);
                    if destroyed as f32 >= fight.total as f32 * HORDE_BROKEN_SHARE {
                        self.chests.push(Chest::new(fight.last_at, fight.def.prize, fight.worth));
                        return Some(HordeBeat::Broken { name: fight.def.name, tier: fight.def.prize });
                    }
                }
            }
        }
        if let Some(coming) = self.coming.as_mut() {
            coming.left -= dt;
            if coming.left > 0.0 {
                return None;
            }
            let coming = *coming;
            self.coming = None;
            self.next_in = rand::gen_range(HORDE_GAP_SECONDS.0, HORDE_GAP_SECONDS.1);
            let horde = make_horde(coming.def, coming.at, army, pressure);
            let count = horde.len();
            let worth: f32 = horde.iter().map(kill_points).sum();
            enemies.extend(horde);
            //one that arrives while another is still being fought joins it, and the better of the two decides the chest
            match self.fight.as_mut() {
                Some(fight) => {
                    fight.total += count;
                    fight.worth += worth;
                    if coming.def.prize >= fight.def.prize {
                        fight.def = coming.def;
                    }
                }
                None => self.fight = Some(Fight { def: coming.def, total: count, lost: 0, worth, last_at: coming.at }),
            }
            return Some(HordeBeat::Arrived { name: coming.def.name, count });
        }
        if boss_busy || army < HORDE_MIN_ARMY {
            return None;
        }
        self.next_in -= dt;
        if self.next_in > 0.0 {
            return None;
        }
        let Some(at) = horde_spot(player) else {
            self.next_in = 3.0;
            return None;
        };
        let def = random_horde(army);
        self.coming = Some(Coming { def, at, left: HORDE_WARNING_SECONDS });
        Some(HordeBeat::Warned { name: def.name, from: compass(at - player) })
    }

    //where to point the player at: the place a horde is about to come from, or the middle of the one that is coming, if it is still far
    pub(crate) fn heading(&self, enemies: &[Enemy], player: Vec2) -> Option<Vec2> {
        if let Some(coming) = &self.coming {
            return Some(coming.at);
        }
        let near = HORDE_ARRIVED_TILES * TILE_SIZE;
        let members: Vec<Vec2> = enemies.iter().filter(|enemy| in_a_horde(enemy)).map(|enemy| enemy.position).collect();
        if members.is_empty() {
            return None;
        }
        let middle = members.iter().copied().sum::<Vec2>() / members.len() as f32;
        (middle.distance(player) > near).then_some(middle)
    }

    //the line for the HUD: the horde that is coming or being fought, or the chest it left, if there is one of those
    pub(crate) fn status(&self, enemies: &[Enemy]) -> Option<(String, Color)> {
        if let Some(fight) = &self.fight {
            let alive = enemies.iter().filter(|enemy| in_a_horde(enemy)).count();
            if alive > 0 {
                return Some((format!("HORDE: {} ({alive} left)", fight.def.name), HORDE_COLOR));
            }
        }
        if let Some(coming) = &self.coming {
            return Some((format!("HORDE: {} in {}s", coming.def.name, coming.left.ceil() as i32), HORDE_COLOR));
        }
        let soonest = self.chests.iter().min_by(|a, b| a.left.total_cmp(&b.left))?;
        Some((format!("{} CHEST: {}s left", soonest.name(), soonest.left.ceil() as i32), soonest.color()))
    }
}

//a mark on the minimap where a horde will come from, rings that spread out and fade, and one that stays
pub(crate) fn draw_horde_ping(map: &Minimap, at: Vec2, time: f32) {
    let rect = map.rect;
    let point = map.to_screen(at).max(rect.point() + Vec2::splat(6.0)).min(rect.point() + rect.size() - Vec2::splat(6.0));
    for k in 0..3 {
        let phase = (time * 0.9 + k as f32 / 3.0).fract();
        draw_circle_lines(point.x, point.y, 4.0 + phase * 26.0, 2.0, Color::new(1.0, 0.55, 0.1, 1.0 - phase));
    }
    draw_circle(point.x, point.y, 4.5, Color::new(1.0, 0.55, 0.1, 1.0));
    draw_circle_lines(point.x, point.y, 4.5, 1.5, WHITE);
}

//an arrow at the edge of the screen pointing at where the horde is or will be, with how many grid squares away that is
pub(crate) fn draw_horde_arrow(at: Vec2, player: Vec2, zoom: f32, seconds: Option<f32>) {
    let label = match seconds {
        Some(seconds) => format!("HORDE {:.0}s", seconds.ceil()),
        None => format!("HORDE {}", (at.distance(player) / TILE_SIZE) as i32),
    };
    draw_edge_arrow(at, player, zoom, Color::new(1.0, 0.55, 0.1, 1.0), &label);
}

//an arrow in a colour at the edge of the screen pointing at a place that is off it, with a label beside it. `player` is where the
//camera is
pub(crate) fn draw_edge_arrow(at: Vec2, player: Vec2, zoom: f32, color: Color, label: &str) {
    let half = Vec2::new(screen_width(), screen_height()) / 2.0;
    let offset = (at - player) * zoom;
    if offset.x.abs() < half.x - 40.0 && offset.y.abs() < half.y - 40.0 {
        return;
    }
    let margin = 46.0;
    let direction = offset.normalize_or_zero();
    let along = ((half.x - margin) / direction.x.abs().max(0.0001)).min((half.y - margin) / direction.y.abs().max(0.0001));
    let spot = half + direction * along;
    let side = direction.perp();
    let pulse = 0.65 + 0.35 * (get_time() as f32 * 7.0).sin();
    let pulsing = Color::new(color.r, color.g, color.b, pulse);
    draw_triangle(spot + direction * 22.0, spot - direction * 12.0 + side * 16.0, spot - direction * 12.0 - side * 16.0, Color::new(0.0, 0.0, 0.0, 0.6));
    draw_triangle(spot + direction * 20.0, spot - direction * 11.0 + side * 14.0, spot - direction * 11.0 - side * 14.0, pulsing);
    let width = text_width(label, 18.0, Face::Bold);
    let label_at = (spot - direction * 38.0 - Vec2::new(width / 2.0, -7.0)).max(Vec2::splat(4.0)).min(Vec2::new(screen_width() - width - 4.0, screen_height() - 4.0));
    let light = Color::new((color.r + 1.0) / 2.0, (color.g + 1.0) / 2.0, (color.b + 1.0) / 2.0, 1.0);
    draw_label_shadowed(label, label_at.x, label_at.y, 18.0, light, Face::Bold);
}
