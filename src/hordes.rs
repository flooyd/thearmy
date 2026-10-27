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
    pub(crate) captain: Option<&'static str>,        //the kind of enemy that leads it, as a captain. one that has a captain comes from two sides, a captain at the head of each
}

pub(crate) static HORDES: [HordeDef; 13] = [
    HordeDef { name: "Swarm Tide", kinds: &[("Swarmer", 0.7), ("Scout", 0.3)], base: 24, per_units: 70, most: 80, from_army: 0, modifier: Modifier::None, prize: 1, captain: None },
    HordeDef { name: "Raiding Party", kinds: &[("Grunt", 0.5), ("Scout", 0.3), ("Splitter", 0.2)], base: 12, per_units: 150, most: 40, from_army: 20, modifier: Modifier::None, prize: 1, captain: None },
    HordeDef { name: "Bomber Run", kinds: &[("Bomber", 0.65), ("Scout", 0.35)], base: 8, per_units: 250, most: 22, from_army: 40, modifier: Modifier::None, prize: 2, captain: None },
    //all splitters: every one that falls is three more
    HordeDef { name: "Fission Mob", kinds: &[("Splitter", 1.0)], base: 6, per_units: 300, most: 24, from_army: 40, modifier: Modifier::None, prize: 1, captain: None },
    //shooters that stop at a distance and fire, behind a line of shielded ones
    HordeDef { name: "Firing Line", kinds: &[("Spitter", 0.6), ("Mortar", 0.15), ("Bulwark", 0.25)], base: 6, per_units: 400, most: 16, from_army: 60, modifier: Modifier::None, prize: 2, captain: None },
    HordeDef { name: "Blood Frenzy", kinds: &[("Grunt", 0.5), ("Scout", 0.5)], base: 10, per_units: 200, most: 36, from_army: 60, modifier: Modifier::Frenzied, prize: 2, captain: None },
    HordeDef { name: "Wildfire", kinds: &[("Grunt", 0.55), ("Bomber", 0.45)], base: 8, per_units: 300, most: 26, from_army: 80, modifier: Modifier::Volatile, prize: 2, captain: None },
    HordeDef { name: "Iron Column", kinds: &[("Brute", 0.4), ("Bulwark", 0.3), ("Charger", 0.3)], base: 8, per_units: 500, most: 24, from_army: 80, modifier: Modifier::None, prize: 2, captain: None },
    //nearly all chargers, closing in and then dashing one after another
    HordeDef { name: "Stampede", kinds: &[("Charger", 0.85), ("Brute", 0.15)], base: 5, per_units: 600, most: 14, from_army: 100, modifier: Modifier::None, prize: 2, captain: None },
    HordeDef { name: "Titan March", kinds: &[("Titan", 0.6), ("Brute", 0.4)], base: 3, per_units: 2000, most: 9, from_army: 250, modifier: Modifier::None, prize: 3, captain: None },
    HordeDef { name: "Armoured Host", kinds: &[("Brute", 0.5), ("Bulwark", 0.3), ("Titan", 0.2)], base: 6, per_units: 700, most: 20, from_army: 400, modifier: Modifier::Armored, prize: 3, captain: None },
    //two crowds, one from each side of the army, each behind a captain as big as a boss: break both to open the chest
    HordeDef { name: "Flanking Raid", kinds: &[("Grunt", 0.4), ("Scout", 0.25), ("Charger", 0.2), ("Spitter", 0.15)], base: 10, per_units: 150, most: 36, from_army: 100, modifier: Modifier::None, prize: 2, captain: Some("Titan") },
    HordeDef { name: "Pincer Host", kinds: &[("Brute", 0.35), ("Bulwark", 0.25), ("Charger", 0.15), ("Spitter", 0.15), ("Mortar", 0.1)], base: 12, per_units: 400, most: 40, from_army: 250, modifier: Modifier::None, prize: 3, captain: Some("Titan") },
];

impl HordeDef {
    //how many enemies there are in it for an army this big, not counting the captains
    pub(crate) fn count(&self, army: usize) -> usize {
        (self.base + army / self.per_units.max(1)).min(self.most)
    }

    //how many sides it comes from: a captain leads each
    pub(crate) fn sides(&self) -> usize {
        if self.captain.is_some() { 2 } else { 1 }
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
    pub(crate) other: Option<Vec2>, //where the second side comes from, for a horde that comes from two
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
    Warned { name: &'static str, from: String },
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

//whether a horde can appear at a place: it is in the world and not too near the edge of it
fn room_for_horde(at: Vec2) -> bool {
    let edge = HORDE_EDGE_TILES * TILE_SIZE;
    at.x > edge && at.y > edge && at.x < WORLD_SIZE - edge && at.y < WORLD_SIZE - edge
}

//where a horde appears: HORDE_DISTANCE_TILES from the player in a random direction, as long as that is in the world
pub(crate) fn horde_spot(player: Vec2) -> Option<Vec2> {
    for _ in 0..30 {
        let direction = Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU));
        let at = player + direction * HORDE_DISTANCE_TILES * TILE_SIZE;
        if room_for_horde(at) {
            return Some(at);
        }
    }
    None
}

//where the two sides of a pincer appear: both HORDE_DISTANCE_TILES from the player, the second more or less straight across from the
//first, as long as both are in the world
pub(crate) fn horde_spots(player: Vec2) -> Option<(Vec2, Vec2)> {
    for _ in 0..60 {
        let angle = rand::gen_range(0.0, std::f32::consts::TAU);
        let across = angle + std::f32::consts::PI + rand::gen_range(-PINCER_SKEW, PINCER_SKEW);
        let first = player + Vec2::from_angle(angle) * HORDE_DISTANCE_TILES * TILE_SIZE;
        let second = player + Vec2::from_angle(across) * HORDE_DISTANCE_TILES * TILE_SIZE;
        if room_for_horde(first) && room_for_horde(second) {
            return Some((first, second));
        }
    }
    None
}

//the enemies of a horde, as made for an army (`pressure` is the size of army they are made for), crowded about a point and all
//running at the player. each is no one's: it is no part of the wild enemies that are kept up. this is one side of it: a horde with a
//captain also has the captains and the other side, which `make_pincer` makes
pub(crate) fn make_horde(def: &'static HordeDef, at: Vec2, army: usize, pressure: usize) -> Vec<Enemy> {
    make_side(def, at, def.count(army), pressure, 0.0)
}

//both sides of a horde that has a captain: the enemies shared between two crowds, and a captain, as big as a boss, at the head of each. the
//crowds start round their captains, and not inside them where they could not be seen
pub(crate) fn make_pincer(def: &'static HordeDef, first: Vec2, second: Vec2, army: usize, pressure: usize) -> Vec<Enemy> {
    let captains: Vec<Enemy> = def
        .captain
        .into_iter()
        .flat_map(|name| {
            [first, second].map(|at| {
                let mut captain = enemy_for(at, kind_named(name), pressure);
                captain.make_captain(pressure);
                join_horde(&mut captain);
                captain
            })
        })
        .collect();
    let clear = captains.first().map_or(0.0, |captain| captain.size / 2.0);
    let count = def.count(army);
    let mut horde = make_side(def, first, count.div_ceil(2), pressure, clear);
    horde.extend(make_side(def, second, count / 2, pressure, clear));
    horde.extend(captains);
    horde
}

//where the other side of a pincer comes from when it is not known: straight across the player from the first, in the world
fn across(player: Vec2, at: Vec2) -> Vec2 {
    let edge = HORDE_EDGE_TILES * TILE_SIZE;
    (player * 2.0 - at).clamp(Vec2::splat(edge), Vec2::splat(WORLD_SIZE - edge))
}

//`count` enemies of a horde, crowded about a point, none nearer to it than `clear` pixels
fn make_side(def: &'static HordeDef, at: Vec2, count: usize, pressure: usize, clear: f32) -> Vec<Enemy> {
    let total: f32 = def.kinds.iter().map(|(_, share)| share).sum();
    let spread = HORDE_SPREAD + (count as f32).sqrt() * HORDE_SPREAD_PER_ROOT;
    let crossfire = ruled_by(Mutator::Crossfire);
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
        //(with Crossfire, on the Insane ladder, some of them are snipers and lancers, whatever the horde)
        if crossfire && rand::gen_range(0.0, 1.0) < CROSSFIRE_LANES {
            name = if rand::gen_range(0.0, 1.0) < 0.5 { "Sniper" } else { "Lancer" };
        }
        let kind = kind_named(name);
        let place = at + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * (clear + spread * rand::gen_range(0.0, 1.0f32).sqrt());
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

    //the same for one player, whose army is `army` big: what the tests that have just the one player use
    #[cfg(test)]
    pub(crate) fn update(&mut self, enemies: &mut Vec<Enemy>, player: Vec2, army: usize, pressure: usize, boss_busy: bool, dt: f32) -> Option<HordeBeat> {
        let field = FlowField::new();
        let prey = [Prey { position: player, units: &[], team: army, pressure, boost: 1.0, cap: DEFAULT_ARMY_LIMIT, field: &field }];
        self.update_among(enemies, &prey, boss_busy, dt)
    }

    //count down to the next horde, announce it, and bring it when the time is up. a horde is sent at one of the players (the ones whose fight
    //is big enough to have anything to fear from a crowd), made for the fight they are in, and runs at whoever is nearest when it
    //gets there. nothing comes while a boss is about. the ones left far behind every player are let go, and a horde that is gone,
    //mostly destroyed, leaves its chest
    pub(crate) fn update_among(&mut self, enemies: &mut Vec<Enemy>, prey: &[Prey], boss_busy: bool, dt: f32) -> Option<HordeBeat> {
        let leash = ENEMY_LEASH_TILES * TILE_SIZE;
        let before = enemies.len();
        enemies.retain(|enemy| !in_a_horde(enemy) || prey.iter().any(|player| enemy.position.distance_squared(player.position) <= leash * leash));
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
            //(it is for the player it was sent at, who is the nearest to where it comes from)
            let target = nearest(prey, coming.at);
            let horde = match coming.def.captain {
                Some(_) => make_pincer(coming.def, coming.at, coming.other.unwrap_or_else(|| across(target.position, coming.at)), target.team, target.pressure),
                None => make_horde(coming.def, coming.at, target.team, target.pressure),
            };
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
        let worth_it: Vec<&Prey> = prey.iter().filter(|player| player.team >= HORDE_MIN_ARMY).collect();
        if boss_busy || worth_it.is_empty() {
            return None;
        }
        self.next_in -= dt;
        if self.next_in > 0.0 {
            return None;
        }
        let target = worth_it[rand::gen_range(0, worth_it.len())];
        let player = target.position;
        let Some(mut at) = horde_spot(player) else {
            self.next_in = 3.0;
            return None;
        };
        let def = random_horde(target.team);
        //one with a captain comes from two sides, which have to fit in the world together
        let mut other = None;
        if def.sides() == 2 {
            let Some((first, second)) = horde_spots(player) else {
                self.next_in = 3.0;
                return None;
            };
            (at, other) = (first, Some(second));
        }
        self.coming = Some(Coming { def, at, left: HORDE_WARNING_SECONDS, other });
        let from = match other {
            Some(second) => format!("{} AND THE {}", compass(at - player), compass(second - player)),
            None => compass(at - player).to_owned(),
        };
        Some(HordeBeat::Warned { name: def.name, from })
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

    //every place to point the player at: both sides of a horde that is still coming, otherwise the one `heading` gives
    pub(crate) fn headings(&self, enemies: &[Enemy], player: Vec2) -> Vec<Vec2> {
        match &self.coming {
            Some(coming) => [Some(coming.at), coming.other].into_iter().flatten().collect(),
            None => self.heading(enemies, player).into_iter().collect(),
        }
    }

    //the line for the HUD: the horde that is coming or being fought, or the chest it left, if there is one of those
    pub(crate) fn status(&self, enemies: &[Enemy]) -> Option<(String, Color)> {
        if let Some(fight) = &self.fight {
            let alive = enemies.iter().filter(|enemy| in_a_horde(enemy)).count();
            if alive > 0 {
                let captains = enemies.iter().filter(|enemy| in_a_horde(enemy) && enemy.is_captain()).count();
                let led = match captains {
                    0 => String::new(),
                    1 => ", 1 captain".to_owned(),
                    many => format!(", {many} captains"),
                };
                return Some((format!("HORDE: {} ({alive} left{led})", fight.def.name), HORDE_COLOR));
            }
        }
        if let Some(coming) = &self.coming {
            return Some((format!("HORDE: {} in {}s", coming.def.name, coming.left.ceil() as i32), HORDE_COLOR));
        }
        //a chest that is being held and is contested says so, over the others: it is what needs doing
        if let Some(stalled) = self.chests.iter().find(|chest| chest.stalled) {
            return Some((format!("{} CHEST: CONTESTED, CLEAR THE ENEMIES AROUND IT", stalled.name()), danger_color(1.0)));
        }
        //a chest that is being held says so, over the others
        if let Some(held) =self.chests.iter().filter(|chest| chest.held > 0.0).max_by(|a, b| a.held.total_cmp(&b.held)) {
            return Some((format!("{} CHEST: HOLD IT {}s", held.name(), (held.hold() - held.held).ceil().max(1.0) as i32), held.color()));
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

//an arrow at the edge of the screen to each captain that is off it
pub(crate) fn draw_captain_arrows(enemies: &[Enemy], camera: Vec2, zoom: f32) {
    for captain in enemies.iter().filter(|enemy| enemy.is_captain()) {
        let label = format!("CAPTAIN {}", (captain.position.distance(camera) / TILE_SIZE) as i32);
        draw_edge_arrow(captain.position, camera, zoom, captain.modifier.color(), &label);
    }
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
//a point that is on something drawn over the game, moved along the edge of the screen to a little outside it, whichever way is the shorter
pub(crate) fn slid_clear_of(spot: Vec2, covered: Rect, screen: Vec2) -> Vec2 {
    if !covered.contains(spot) {
        return spot;
    }
    let gap = 20.0;
    [Vec2::new(covered.x - gap, spot.y), Vec2::new(covered.right() + gap, spot.y), Vec2::new(spot.x, covered.y - gap), Vec2::new(spot.x, covered.bottom() + gap)]
        .into_iter()
        .filter(|at| at.x >= 0.0 && at.y >= 0.0 && at.x <= screen.x && at.y <= screen.y)
        .min_by(|a, b| a.distance(spot).total_cmp(&b.distance(spot)))
        .unwrap_or(spot)
}

//whether something at a place is off the screen far enough for an arrow at the edge to point at it (`player` being where the middle of the screen is)
fn is_off_screen(at: Vec2, player: Vec2, zoom: f32, screen: Vec2) -> bool {
    let offset = (at - player) * zoom;
    let half = screen / 2.0;
    offset.x.abs() >= half.x - 40.0 || offset.y.abs() >= half.y - 40.0
}

//where the arrow for something that is off the screen goes, and the way it points: at the edge, towards it. one that would land on the panel with the army
//in it, the minimap, the list of players or the roster of champions slides along the edge, to the nearer side of it. None for something that is on the screen
pub(crate) fn edge_spot(at: Vec2, player: Vec2, zoom: f32, screen: Vec2) -> Option<(Vec2, Vec2)> {
    if !is_off_screen(at, player, zoom, screen) {
        return None;
    }
    let half = screen / 2.0;
    let margin = 46.0;
    let direction = ((at - player) * zoom).normalize_or_zero();
    let along = ((half.x - margin) / direction.x.abs().max(0.0001)).min((half.y - margin) / direction.y.abs().max(0.0001));
    let mut spot = half + direction * along;
    for covered in [Rect::new(0.0, 0.0, HUD_PANEL_RIGHT, HUD_PANEL_BOTTOM), Minimap::new(screen, MapView::Near, Vec2::ZERO).rect].into_iter().chain(team_panel(screen)).chain(roster_panel(screen)) {
        spot = slid_clear_of(spot, covered, screen);
    }
    Some((spot, direction))
}

pub(crate) fn draw_edge_arrow(at: Vec2, player: Vec2, zoom: f32, color: Color, label: &str) {
    let Some((spot, direction)) = edge_spot(at, player, zoom, Vec2::new(screen_width(), screen_height())) else { return };
    let side = direction.perp();
    let pulse = 0.65 + 0.35 * (get_time() as f32 * 7.0).sin();
    let pulsing = Color::new(color.r, color.g, color.b, pulse);
    draw_triangle(spot + direction * 22.0, spot - direction * 12.0 + side * 16.0, spot - direction * 12.0 - side * 16.0, Color::new(0.0, 0.0, 0.0, 0.6));
    draw_triangle(spot + direction * 20.0, spot - direction * 11.0 + side * 14.0, spot - direction * 11.0 - side * 14.0, pulsing);
    let width = text_width(label, 18.0, Face::Bold);
    //(behind the arrow, far enough back that a long label pointing left or right is clear of it)
    let label_at = (spot - direction * (38.0 + width / 2.0 * direction.x.abs()) - Vec2::new(width / 2.0, -7.0)).max(Vec2::splat(4.0)).min(Vec2::new(screen_width() - width - 4.0, screen_height() - 4.0));
    let light = Color::new((color.r + 1.0) / 2.0, (color.g + 1.0) / 2.0, (color.b + 1.0) / 2.0, 1.0);
    draw_label_shadowed(label, label_at.x, label_at.y, 18.0, light, Face::Bold);
}
