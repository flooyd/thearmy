//! places in the world: nests that send out enemies until they are destroyed and villages held by enemies until they are liberated

use crate::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum SiteKind {
    Nest(usize), //which of NESTS
    Village,
    Barracks, //held by soldiers, and a chance of a champion joining when it is taken
    Citadel,
}

//a site is asleep until the player comes near, when it is made for the army the player has then. one that the player leaves
//behind goes back to sleep. a destroyed nest and a liberated village stay that way
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum SiteState {
    Dormant,
    Awake,
    Cleared,
}

#[derive(Clone)]
pub(crate) struct Site {
    pub(crate) id: u32,
    pub(crate) kind: SiteKind,
    pub(crate) position: Vec2, //its middle
    pub(crate) state: SiteState,
    pub(crate) core: Option<u32>,      //the enemy a nest or citadel is built round, while it is awake
    pub(crate) tiles: Vec<(i32, i32)>, //the walls it raised
    pub(crate) clock: f32,             //seconds, a liberated village counts down to its next volunteer
}

//a kind of nest: what it sends out, how often, and how many at most
pub(crate) struct NestDef {
    pub(crate) name: &'static str,
    pub(crate) color: Color,
    pub(crate) min_army: usize, //it sleeps on until the army is this big
    pub(crate) child: &'static str,
    pub(crate) brood: usize, //sent out at once for the smallest army...
    pub(crate) every: f32,   //...this many seconds apart
    pub(crate) cap: usize,   //most it has alive
    pub(crate) ring: i32,    //the lair's walls are this many grid squares from its middle
}

pub(crate) static NESTS: [NestDef; 4] = [
    NestDef { name: "Swarm Nest", color: Color::new(0.65, 0.9, 0.25, 1.0), min_army: 4, child: "Swarmer", brood: 4, every: 5.0, cap: 16, ring: 5 },
    NestDef { name: "Warren", color: Color::new(0.8, 0.35, 0.3, 1.0), min_army: 10, child: "Grunt", brood: 2, every: 6.0, cap: 8, ring: 5 },
    NestDef { name: "Spitter Roost", color: Color::new(0.75, 0.55, 0.3, 1.0), min_army: 15, child: "Spitter", brood: 1, every: 6.0, cap: 4, ring: 6 },
    NestDef { name: "Bomb Cache", color: Color::new(0.55, 0.55, 0.65, 1.0), min_army: 30, child: "Bomber", brood: 2, every: 7.0, cap: 6, ring: 5 },
];

pub(crate) fn nest_named(name: &str) -> Option<&'static NestDef> {
    NESTS.iter().find(|def| def.name == name)
}

impl Site {
    pub(crate) fn new(kind: SiteKind, position: Vec2) -> Site {
        Site { id: NEXT_SITE_ID.fetch_add(1, Ordering::Relaxed), kind, position, state: SiteState::Dormant, core: None, tiles: Vec::new(), clock: 0.0 }
    }

    //the army it takes to wake it
    pub(crate) fn min_army(&self) -> usize {
        match self.kind {
            SiteKind::Nest(i) => NESTS[i].min_army,
            SiteKind::Village => 3,
            SiteKind::Barracks => BARRACKS_MIN_ARMY,
            SiteKind::Citadel => CITADEL_MIN_ARMY,
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        match self.kind {
            SiteKind::Nest(i) => NESTS[i].name,
            SiteKind::Village => "Village",
            SiteKind::Barracks => "Barracks",
            SiteKind::Citadel => "The Citadel",
        }
    }
}

pub(crate) static NEXT_SITE_ID: AtomicU32 = AtomicU32::new(0);

//where the sites are: spread over the world, far enough from each other, the edge and the start, a citadel in one corner
pub(crate) fn generate_sites() -> Vec<Site> {
    let corner_gap = (SITE_EDGE_TILES + CITADEL_RING as f32 + 4.0) * TILE_SIZE;
    //the citadel is in a corner, the one that comes up
    let corner = Vec2::new(if rand::gen_range(0, 2) == 0 { corner_gap } else { WORLD_SIZE - corner_gap }, if rand::gen_range(0, 2) == 0 { corner_gap } else { WORLD_SIZE - corner_gap });
    let mut sites = vec![Site::new(SiteKind::Citadel, corner)];
    let wanted: Vec<SiteKind> = (0..NEST_COUNT).map(|k| SiteKind::Nest(k % NESTS.len())).chain((0..VILLAGE_COUNT).map(|_| SiteKind::Village)).chain((0..BARRACKS_COUNT).map(|_| SiteKind::Barracks)).collect();
    place_sites(&mut sites, wanted);
    sites
}

//put sites of these kinds in the world, each somewhere that is far enough from the sites already there, the edge and the start.
//one that can't find room after a few tries is left out
pub(crate) fn place_sites(sites: &mut Vec<Site>, wanted: Vec<SiteKind>) {
    let centre = Vec2::splat(WORLD_SIZE / 2.0);
    let edge = SITE_EDGE_TILES * TILE_SIZE;
    for kind in wanted {
        for _ in 0..200 {
            let position = Vec2::new(rand::gen_range(edge, WORLD_SIZE - edge), rand::gen_range(edge, WORLD_SIZE - edge));
            let far_enough = position.distance(centre) >= SITE_START_CLEAR_TILES * TILE_SIZE && sites.iter().all(|site| site.position.distance(position) >= SITE_SEPARATION_TILES * TILE_SIZE);
            if far_enough {
                sites.push(Site::new(kind, position));
                break;
            }
        }
    }
}

//the squares of the walls of a lair, a square ring `half` squares from the middle with two gaps of two squares in it
pub(crate) fn lair_tiles(middle: (i32, i32), half: i32, gap_sides: [i32; 2], gap_offsets: [i32; 2]) -> Vec<(i32, i32)> {
    let mut gaps: HashSet<(i32, i32)> = HashSet::new();
    for (side, offset) in gap_sides.into_iter().zip(gap_offsets) {
        for k in 0..2 {
            let along = offset + k;
            gaps.insert(match side {
                0 => (middle.0 + along, middle.1 - half),
                1 => (middle.0 + along, middle.1 + half),
                2 => (middle.0 - half, middle.1 + along),
                _ => (middle.0 + half, middle.1 + along),
            });
        }
    }
    let mut tiles = Vec::new();
    for col in (middle.0 - half)..=(middle.0 + half) {
        for row in (middle.1 - half)..=(middle.1 + half) {
            let on_ring = col == middle.0 - half || col == middle.0 + half || row == middle.1 - half || row == middle.1 + half;
            if on_ring && !gaps.contains(&(col, row)) {
                tiles.push((col, row));
            }
        }
    }
    tiles
}

//raise the hostile walls of a lair, on the squares that have nothing on them
pub(crate) fn raise_lair(site: &mut Site, half: i32, army: usize, walls: &mut Walls) {
    let middle = tile_of(site.position.x, site.position.y);
    let first = rand::gen_range(0, 4);
    let second = (first + rand::gen_range(1, 4)) % 4;
    let reach = (half - 3).max(1);
    let tiles = lair_tiles(middle, half, [first, second], [rand::gen_range(-reach, reach), rand::gen_range(-reach, reach)]);
    let health = WALL_HEALTH * LAIR_WALL_HEALTH * army_scale(army);
    for tile in tiles {
        if let std::collections::hash_map::Entry::Vacant(spot) = walls.entry(tile) {
            spot.insert(Wall::made(tile.0, tile.1, WallKind::Wall, Side::Hostile, health));
            site.tiles.push(tile);
        }
    }
}

//the core of a nest: a big square that doesn't move
pub(crate) fn nest_core(def: &'static NestDef, site: &Site, army: usize) -> Enemy {
    let mut core = enemy_for(site.position, kind_named("Nest"), army);
    core.name = def.name;
    core.color = def.color;
    core.site = Some(site.id);
    core.timer = def.every * 0.5;
    core
}

//the enemies that hold a village
pub(crate) fn village_guards(site: &Site, army: usize) -> Vec<Enemy> {
    garrison(site, army, (3 + army / 60).min(16), TILE_SIZE * 5.0)
}

//the soldiers that hold a barracks, one of them a captain: an elite of some kind
pub(crate) fn barracks_guards(site: &Site, army: usize) -> Vec<Enemy> {
    let mut soldiers = garrison(site, army, (BARRACKS_GUARDS + army / BARRACKS_GUARDS_PER).min(BARRACKS_MAX_GUARDS), TILE_SIZE * (BARRACKS_RING as f32 - 2.5));
    if let Some(captain) = soldiers.first_mut() {
        captain.make_elite(MODIFIERS[rand::gen_range(0, MODIFIERS.len())]);
    }
    soldiers
}

//enemies of kinds that don't come in packs or mend others, standing round a site up to `reach` pixels from its middle. they
//belong to it, and it is held until every one of them is destroyed
fn garrison(site: &Site, army: usize, count: usize, reach: f32) -> Vec<Enemy> {
    (0..count)
        .map(|_| {
            let kind = (0..20).map(|_| random_kind(army)).find(|kind| kind.pack == 0 && kind.behavior != Behavior::Healer).unwrap_or(&ENEMY_KINDS[0]);
            let around = site.position + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(TILE_SIZE, reach);
            let mut guard = enemy_for(around, kind, army);
            guard.site = Some(site.id);
            guard
        })
        .collect()
}

//whether taking a barracks sends a champion, for a roll between 0 and 1
pub(crate) fn barracks_champion_dropped(roll: f32) -> bool {
    roll < BARRACKS_CHAMPION_CHANCE
}

//the enemies of a site that are gone from the world, with whatever they made
pub(crate) fn remove_site_enemies(enemies: &mut Vec<Enemy>, site: &Site) {
    let owners: HashSet<u32> = enemies.iter().filter(|enemy| enemy.site == Some(site.id)).map(|enemy| enemy.id).collect();
    enemies.retain(|enemy| enemy.site != Some(site.id) && !enemy.owner.is_some_and(|owner| owners.contains(&owner)));
}

//the hostile walls of a site come down
pub(crate) fn take_down_walls(site: &mut Site, walls: &mut Walls, fx: &mut Effects, with_a_bang: bool) -> bool {
    let mut any = false;
    for tile in site.tiles.drain(..) {
        if walls.get(&tile).is_some_and(|wall| wall.side == Side::Hostile) {
            let wall = walls.remove(&tile).unwrap();
            if with_a_bang {
                fx.burst(wall.center(), wall.color, 6, 200.0);
            }
            any = true;
        }
    }
    any
}

//what the sites did in a frame, for main to announce
#[derive(Default, Debug)]
pub(crate) struct SiteNews {
    pub(crate) nests: usize,        //destroyed
    pub(crate) villages: usize,     //liberated
    pub(crate) barracks: usize,     //taken...
    pub(crate) barracks_champions: usize, //...and how many sent a champion (who is put in `fx.recruits`)
    pub(crate) citadel: bool,       //the citadel's lord fell
    pub(crate) citadel_rearmed: bool, //and is back: the citadel has waited out CITADEL_RESPAWN_SECONDS
    pub(crate) walls_changed: bool, //walls went up or came down
}

//wake the sites the player has come to, put the ones they have left back to sleep, and see whether a nest has been destroyed or
//a village liberated. a liberated village mends the units near it and now and then sends a volunteer. `boss_busy` keeps the
//citadel shut while a boss is already alive. `citadels` is how many times the lord of the citadel has fallen, the next one is
//that much stronger. a citadel that has fallen counts down to being armed again, wherever the player is
pub(crate) fn update_sites(
    sites: &mut [Site],
    enemies: &mut Vec<Enemy>,
    walls: &mut Walls,
    babies: &mut Vec<Baby>,
    units: &mut [Unit],
    player: Vec2,
    dt: f32,
    boss_busy: bool,
    cap: usize,
    citadels: u32,
    fx: &mut Effects,
) -> SiteNews {
    let army = units.len();
    let mut news = SiteNews::default();
    for site in sites.iter_mut() {
        let distance = site.position.distance(player);
        match site.state {
            SiteState::Dormant => {
                let ready = army >= site.min_army() && !(site.kind == SiteKind::Citadel && boss_busy);
                if distance <= AWAKEN_TILES * TILE_SIZE && ready {
                    awaken(site, enemies, walls, army, citadels, fx);
                    news.walls_changed |= !site.tiles.is_empty();
                }
            }
            SiteState::Awake => {
                if distance > SLEEP_TILES * TILE_SIZE {
                    remove_site_enemies(enemies, site);
                    news.walls_changed |= take_down_walls(site, walls, fx, false);
                    site.core = None;
                    site.state = SiteState::Dormant;
                    continue;
                }
                let held = match site.kind {
                    SiteKind::Village | SiteKind::Barracks => enemies.iter().any(|enemy| enemy.site == Some(site.id)),
                    _ => site.core.is_some_and(|core| enemies.iter().any(|enemy| enemy.id == core)),
                };
                if held {
                    continue;
                }
                site.state = SiteState::Cleared;
                site.core = None;
                news.walls_changed |= take_down_walls(site, walls, fx, true);
                match site.kind {
                    SiteKind::Nest(_) => {
                        news.nests += 1;
                        fx.points += NEST_REWARD_POINTS;
                        spread_babies(babies, site.position, NEST_REWARD_BABIES + army / NEST_BABIES_PER, army, cap, TILE_SIZE * 3.0);
                        fx.announce(format!("{} DESTROYED", site.name().to_uppercase()), Color::new(1.0, 0.55, 0.4, 1.0));
                        fx.add_shake(0.4);
                    }
                    SiteKind::Village => {
                        news.villages += 1;
                        fx.points += VILLAGE_REWARD_POINTS;
                        spread_babies(babies, site.position, VILLAGE_VILLAGERS + army / VILLAGERS_PER, army, cap, TILE_SIZE * 2.5);
                        site.clock = VILLAGE_TRICKLE_SECONDS;
                        fx.announce("VILLAGE LIBERATED".to_owned(), CYAN);
                        fx.cue(Cue::Liberate);
                    }
                    SiteKind::Barracks => {
                        news.barracks += 1;
                        fx.points += BARRACKS_REWARD_POINTS;
                        spread_babies(babies, site.position, BARRACKS_RECRUITS + army / BARRACKS_RECRUITS_PER, army, cap, TILE_SIZE * 3.0);
                        let champion = barracks_champion_dropped(rand::gen_range(0.0, 1.0));
                        news.barracks_champions += champion as usize;
                        if champion {
                            fx.recruits.push((site.position, Odds::Barracks));
                        }
                        let message = if champion {
                            "BARRACKS TAKEN: A CHAMPION JOINS!"
                        } else {
                            "BARRACKS TAKEN"
                        };
                        fx.announce(message.to_owned(), BARRACKS_COLOR);
                        fx.add_shake(0.3);
                        fx.cue(Cue::Liberate);
                    }
                    SiteKind::Citadel => {
                        news.citadel = true;
                        site.clock = CITADEL_RESPAWN_SECONDS;
                    }
                }
            }
            SiteState::Cleared => {
                if site.kind == SiteKind::Citadel {
                    site.clock -= dt;
                    if site.clock <= 0.0 {
                        site.clock = 0.0;
                        site.state = SiteState::Dormant;
                        news.citadel_rearmed = true;
                        fx.announce("THE CITADEL IS ARMED AGAIN".to_owned(), Color::new(1.0, 0.3, 0.3, 1.0));
                        fx.cue(Cue::Warning);
                    }
                    continue;
                }
                if site.kind != SiteKind::Village || distance > AWAKEN_TILES * TILE_SIZE {
                    continue;
                }
                //a liberated village mends the units round it and sends volunteers
                let reach = VILLAGE_HEAL_TILES * TILE_SIZE;
                for unit in units.iter_mut().filter(|unit| unit.health < unit.max_health() && Vec2::new(unit.x, unit.y).distance(site.position) <= reach) {
                    unit.health = (unit.health + VILLAGE_HEAL_RATE * dt).min(unit.max_health());
                }
                site.clock -= dt;
                if site.clock <= 0.0 {
                    site.clock = VILLAGE_TRICKLE_SECONDS;
                    let waiting = babies.iter().filter(|baby| baby.position.distance(site.position) <= TILE_SIZE * 4.0).count();
                    if waiting < 3 {
                        spread_babies(babies, site.position, 1, army, cap, TILE_SIZE * 2.0);
                    }
                }
            }
        }
    }
    news
}

//wake a site up: a nest gets its core and its walls, a village its guards
pub(crate) fn awaken(site: &mut Site, enemies: &mut Vec<Enemy>, walls: &mut Walls, army: usize, citadels: u32, fx: &mut Effects) {
    site.state = SiteState::Awake;
    match site.kind {
        SiteKind::Nest(i) => {
            let def = &NESTS[i];
            let core = nest_core(def, site, army);
            site.core = Some(core.id);
            //the walls stand clear of the core, however big it has grown
            let half = def.ring.max((core.size / 2.0 / TILE_SIZE).ceil() as i32 + 3);
            enemies.push(core);
            raise_lair(site, half, army, walls);
        }
        SiteKind::Village => enemies.extend(village_guards(site, army)),
        SiteKind::Barracks => {
            enemies.extend(barracks_guards(site, army));
            raise_lair(site, BARRACKS_RING, army, walls);
        }
        SiteKind::Citadel => awaken_citadel(site, enemies, walls, army, citadels, fx),
    }
}

//babies scattered round a point, as many as there is room in the army for when it can be `cap` big
pub(crate) fn spread_babies(babies: &mut Vec<Baby>, at: Vec2, count: usize, army: usize, cap: usize, spread: f32) {
    let room = cap.saturating_sub(army + babies.len());
    for _ in 0..count.min(room) {
        let position = (at + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(0.0, spread)).clamp(Vec2::splat(BABY_RADIUS), Vec2::splat(WORLD_SIZE - BABY_RADIUS));
        babies.push(Baby { position, age: 0.0 });
    }
}

//the citadel: a ring of walls with the lord of it inside, which stays where it is until someone comes close. it has fallen
//`citadels` times before, and is that much stronger
pub(crate) fn awaken_citadel(site: &mut Site, enemies: &mut Vec<Enemy>, walls: &mut Walls, army: usize, citadels: u32, fx: &mut Effects) {
    site.state = SiteState::Awake;
    let mut lord = Enemy::boss(&FINAL_BOSS, army, citadels, site.position);
    lord.site = Some(site.id);
    lord.chasing = false;
    lord.aggro = (CITADEL_RING as f32 + 8.0) * TILE_SIZE;
    site.core = Some(lord.id);
    //the walls stand clear of it, however big it has grown
    let half = CITADEL_RING.max((lord.size / 2.0 / TILE_SIZE).ceil() as i32 + 6);
    enemies.push(lord);
    raise_lair(site, half, army, walls);
    fx.announce(if citadels == 0 { "THE CITADEL STIRS" } else { "THE CITADEL STIRS, AND THE SOVEREIGN IS STRONGER" }.to_owned(), Color::new(1.0, 0.3, 0.3, 1.0));
    fx.cue(Cue::Warning);
}

//the houses of a village: where they stand round its middle and how big they are, the same every time for the same village
pub(crate) fn village_houses(site: &Site) -> Vec<Rect> {
    let mut seed = site.id.wrapping_mul(2654435761).wrapping_add(12345);
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 10_000) as f32 / 10_000.0
    };
    (0..7)
        .map(|k| {
            let angle = k as f32 / 7.0 * std::f32::consts::TAU + next() * 0.6;
            let distance = TILE_SIZE * (1.8 + next() * 2.4);
            let side = TILE_SIZE * (0.9 + next() * 0.7);
            let at = site.position + Vec2::from_angle(angle) * distance;
            Rect::new(at.x - side / 2.0, at.y - side / 2.0, side, side)
        })
        .collect()
}

//what the world shows of the sites that are on screen: a village's houses, dim while it is held and lit once it is free, the
//scorched ground a nest was or is built on, and the outline of the citadel
pub(crate) fn draw_sites(sites: &[Site], view: Rect, time: f32) {
    let reach = TILE_SIZE * (CITADEL_RING as f32 + 6.0);
    let view = Rect::new(view.x - reach, view.y - reach, view.w + reach * 2.0, view.h + reach * 2.0);
    for site in sites.iter().filter(|site| view.contains(site.position)) {
        let (x, y) = (site.position.x, site.position.y);
        match site.kind {
            SiteKind::Nest(i) => {
                let color = NESTS[i].color;
                let radius = TILE_SIZE * NESTS[i].ring as f32;
                let strength = if site.state == SiteState::Cleared { 0.06 } else { 0.14 };
                draw_circle(x, y, radius, Color::new(color.r * 0.4, color.g * 0.3, color.b * 0.3, strength + 0.5));
                draw_circle_lines(x, y, radius, 3.0, Color::new(color.r, color.g, color.b, strength + 0.15));
                if site.state == SiteState::Dormant {
                    //something is under there
                    draw_circle(x, y, TILE_SIZE * 0.9, Color::new(0.05, 0.0, 0.0, 0.9));
                    draw_circle_lines(x, y, TILE_SIZE * 0.9, 4.0, Color::new(color.r, color.g, color.b, 0.5));
                }
            }
            SiteKind::Village => {
                let free = site.state == SiteState::Cleared;
                for house in village_houses(site) {
                    let (body, roof) = if free {
                        (Color::new(0.62, 0.5, 0.36, 1.0), Color::new(0.8, 0.32, 0.25, 1.0))
                    } else {
                        (Color::new(0.3, 0.26, 0.24, 1.0), Color::new(0.2, 0.17, 0.16, 1.0))
                    };
                    draw_rectangle(house.x, house.y, house.w, house.h, body);
                    draw_rectangle(house.x, house.y, house.w, house.h * 0.4, roof);
                    if free {
                        let glow = 0.7 + 0.3 * (time * 2.0 + house.x).sin();
                        draw_rectangle(house.x + house.w * 0.4, house.y + house.h * 0.55, house.w * 0.2, house.h * 0.25, Color::new(CYAN.r, CYAN.g, CYAN.b, glow));
                    }
                }
                if free {
                    //a flag over the middle of it
                    draw_line(x, y, x, y - TILE_SIZE * 2.0, 4.0, WHITE);
                    draw_triangle(Vec2::new(x, y - TILE_SIZE * 2.0), Vec2::new(x + TILE_SIZE * 0.9, y - TILE_SIZE * 1.7), Vec2::new(x, y - TILE_SIZE * 1.4), CYAN);
                    draw_circle_lines(x, y, VILLAGE_HEAL_TILES * TILE_SIZE, 2.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.15));
                }
            }
            SiteKind::Barracks => draw_barracks(site, time),
            SiteKind::Citadel => {
                let half = TILE_SIZE * CITADEL_RING as f32;
                let color = if site.state == SiteState::Cleared { Color::new(0.5, 0.5, 0.55, 0.3) } else { Color::new(0.9, 0.2, 0.25, 0.4 + 0.2 * (time * 2.0).sin()) };
                draw_rectangle_lines(x - half, y - half, half * 2.0, half * 2.0, 8.0, color);
            }
        }
    }
}

//a barracks: a parade ground with a long hall on it, a door, training dummies and a flag. dark and red while soldiers hold it,
//and lit in the accent colour once it is taken
fn draw_barracks(site: &Site, time: f32) {
    let (x, y) = (site.position.x, site.position.y);
    let free = site.state == SiteState::Cleared;
    let t = TILE_SIZE;
    let (ground, wall, roof, trim) = if free {
        (Color::new(0.16, 0.2, 0.27, 0.6), Color::new(0.66, 0.7, 0.78, 1.0), Color::new(0.3, 0.45, 0.7, 1.0), CYAN)
    } else {
        (Color::new(0.17, 0.11, 0.12, 0.65), Color::new(0.36, 0.3, 0.32, 1.0), Color::new(0.5, 0.15, 0.19, 1.0), Color::new(0.85, 0.3, 0.3, 1.0))
    };
    let yard = t * (BARRACKS_RING as f32 - 1.5);
    draw_rectangle(x - yard, y - yard, yard * 2.0, yard * 2.0, ground);
    draw_rectangle_lines(x - yard, y - yard, yard * 2.0, yard * 2.0, 3.0, Color::new(trim.r, trim.g, trim.b, 0.4));
    //training dummies in the corners of the yard
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let at = Vec2::new(x + dx * t * 3.4, y + dy * t * 3.3);
        draw_circle(at.x, at.y, t * 0.3, wall);
        draw_circle_lines(at.x, at.y, t * 0.3, 3.0, trim);
        draw_line(at.x - t * 0.2, at.y, at.x + t * 0.2, at.y, 3.0, trim);
    }
    //the hall, with a roof, a ridge and battlements along its back
    let hall = Rect::new(x - t * 2.6, y - t * 1.5, t * 5.2, t * 2.6);
    for k in 0..9 {
        draw_rectangle(hall.x + t * 0.1 + k as f32 * t * 0.6, hall.y - t * 0.2, t * 0.38, t * 0.25, wall);
    }
    draw_rectangle(hall.x, hall.y, hall.w, hall.h, wall);
    draw_rectangle(hall.x + t * 0.25, hall.y + t * 0.25, hall.w - t * 0.5, hall.h - t * 0.9, roof);
    draw_line(hall.x + t * 0.25, hall.y + hall.h * 0.4, hall.right() - t * 0.25, hall.y + hall.h * 0.4, 3.0, Color::new(0.0, 0.0, 0.0, 0.35));
    //the door, in the middle of the front, and the emblem on the roof
    draw_rectangle(x - t * 0.45, hall.bottom() - t * 0.6, t * 0.9, t * 0.6, Color::new(0.04, 0.03, 0.05, 1.0));
    draw_rectangle_lines(x - t * 0.45, hall.bottom() - t * 0.6, t * 0.9, t * 0.6, 3.0, trim);
    let emblem = BARRACKS_COLOR;
    draw_circle(x, hall.y + hall.h * 0.4, t * 0.5, emblem);
    draw_circle_lines(x, hall.y + hall.h * 0.4, t * 0.5, 3.0, Color::new(0.0, 0.0, 0.0, 0.5));
    //the flag over a corner of it
    let pole = Vec2::new(hall.right() - t * 0.3, hall.y - t * 0.1);
    let wave = (time * 3.0).sin() * t * 0.12;
    draw_line(pole.x, pole.y, pole.x, pole.y - t * 1.9, 4.0, WHITE);
    draw_triangle(Vec2::new(pole.x, pole.y - t * 1.9), Vec2::new(pole.x + t * 1.0, pole.y - t * 1.6 + wave), Vec2::new(pole.x, pole.y - t * 1.3), trim);
}

//the sites as marks on the minimap: a hollow mark while one sleeps, a bright one when it is awake, a faded one once it is done
pub(crate) fn draw_site_marks(map: &Minimap, sites: &[Site], time: f32) {
    let rect = map.rect;
    for site in sites {
        let point = map.to_screen(site.position);
        let size = match site.kind {
            SiteKind::Citadel => 9.0,
            _ => 6.0,
        };
        if point.x < rect.x + size || point.x > rect.right() - size || point.y < rect.y + size || point.y > rect.bottom() - size {
            continue;
        }
        let fade = match site.state {
            SiteState::Dormant => 0.45,
            SiteState::Awake => 1.0,
            SiteState::Cleared => 0.3,
        };
        match site.kind {
            SiteKind::Nest(i) => {
                let color = NESTS[i].color;
                let color = Color::new(color.r, color.g, color.b, fade);
                if site.state == SiteState::Cleared {
                    draw_line(point.x - size / 2.0, point.y - size / 2.0, point.x + size / 2.0, point.y + size / 2.0, 1.5, color);
                    draw_line(point.x - size / 2.0, point.y + size / 2.0, point.x + size / 2.0, point.y - size / 2.0, 1.5, color);
                } else {
                    //a diamond
                    let h = size / 2.0 + 1.0;
                    let (a, b, c, d) = (point + Vec2::new(0.0, -h), point + Vec2::new(h, 0.0), point + Vec2::new(0.0, h), point + Vec2::new(-h, 0.0));
                    for (from, to) in [(a, b), (b, c), (c, d), (d, a)] {
                        draw_line(from.x, from.y, to.x, to.y, 1.5, color);
                    }
                    if site.state == SiteState::Awake {
                        draw_circle(point.x, point.y, 1.5, color);
                    }
                }
            }
            SiteKind::Village => {
                //a little house, cyan once it is free
                let color = if site.state == SiteState::Cleared { Color::new(CYAN.r, CYAN.g, CYAN.b, 0.9) } else { Color::new(0.85, 0.75, 0.55, fade) };
                draw_rectangle(point.x - 2.5, point.y - 1.0, 5.0, 4.0, color);
                draw_triangle(point + Vec2::new(-3.5, -1.0), point + Vec2::new(3.5, -1.0), point + Vec2::new(0.0, -4.5), color);
            }
            SiteKind::Barracks => {
                //a little castle, cyan once it is taken
                let emblem = BARRACKS_COLOR;
                let color = if site.state == SiteState::Cleared { Color::new(CYAN.r, CYAN.g, CYAN.b, 0.9) } else { Color::new(emblem.r, emblem.g, emblem.b, fade.max(0.6)) };
                draw_rectangle(point.x - 3.5, point.y - 1.0, 7.0, 5.0, color);
                for dx in [-3.5, -0.6, 2.5] {
                    draw_rectangle(point.x + dx, point.y - 3.5, 1.1, 2.5, color);
                }
            }
            SiteKind::Citadel => {
                let pulse = 0.6 + 0.4 * (time * 3.0).sin();
                let color = if site.state == SiteState::Cleared { Color::new(0.6, 0.6, 0.65, 0.5) } else { Color::new(1.0, 0.25, 0.3, fade.max(0.7) * pulse) };
                draw_rectangle_lines(point.x - size / 2.0, point.y - size / 2.0, size, size, 2.0, color);
                draw_rectangle(point.x - 2.0, point.y - 2.0, 4.0, 4.0, color);
            }
        }
    }
}
