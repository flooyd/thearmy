//! fighting: damage by touch, deaths, shots, hazards and the babies enemies drop

use crate::prelude::*;

//every unit inside an enemy hurts it, and it hurts the units inside it: its attack, a number of damage per second, is
//spent on one unit after another until it is used up, so units die one at a time rather than all being worn down. a
//bomber that is touched lights its fuse. an enemy that runs out of health is destroyed (see resolve_deaths). the units that
//run out of health are left for bury_units. returns how many enemies were destroyed.
//the units are everyone's: each does as much as its own player's relics make it, takes as much as their relics let through, and
//the player whose units did the most to an enemy has the credit for it
pub(crate) fn fight<R: Relics + ?Sized>(enemies: &mut Vec<Enemy>, units: &mut [Unit], babies: &mut Vec<Baby>, dt: f32, relics: &R, fx: &mut Effects) -> usize {
    let grid = UnitGrid::new(units);
    //what each player's relics make of a unit's touch, worked out once as the power in it is not cheap
    let hits: [f32; MAX_PLAYERS] = std::array::from_fn(|owner| relics.of(owner as u8).hit());
    let mut touching: Vec<usize> = Vec::new();
    for enemy in enemies.iter_mut() {
        touching.clear();
        let reach = Vec2::splat(enemy.size / 2.0 + RADIUS * MAX_CHAMPION_SIZE);
        grid.near(enemy.position - reach, enemy.position + reach, &mut touching);
        touching.retain(|&i| {
            let unit = &units[i];
            unit.health > 0.0 && unit.garrison.is_none() && enemy.overlaps_circle(unit.x, unit.y, unit.radius())
        });
        //an ambusher that a unit stumbles on springs on it, and is not fought until it has
        if enemy.lurking() {
            if let Some(&first) = touching.first() {
                enemy.spring(Vec2::new(units[first].x, units[first].y), fx);
            }
            continue;
        }
        //a champion counts as many units, and a boss with an armoured front takes little from the units in front of it
        let mut strength = [0.0f32; MAX_PLAYERS];
        if enemy.boss.as_ref().is_some_and(|state| state.def.flank) {
            for &i in &touching {
                strength[owner_index(units[i].owner)] += flank_factor(enemy, Vec2::new(units[i].x, units[i].y)) * units[i].might;
            }
        } else {
            for &i in &touching {
                strength[owner_index(units[i].owner)] += units[i].might;
            }
        }
        let mut damage = 0.0;
        for (owner, &each) in strength.iter().enumerate() {
            damage += each * UNIT_DAMAGE * hits[owner] * dt;
        }
        let dealt = enemy.take_damage(damage);
        enemy.hurt += dealt;
        //the credit is the army that did the most of it, which is not always the one with the most units on it
        let share = |owner: usize| strength[owner] * hits[owner];
        if let Some(top) = (0..MAX_PLAYERS).filter(|&owner| share(owner) > 0.0).max_by(|&a, &b| share(a).total_cmp(&share(b))) {
            enemy.credit = top as u8;
        }
        let leading = relics.of(enemy.credit);
        //hurt by units, it fights back at where they are, and the enemies near it come at them too
        if dealt > 0.0 && !touching.is_empty() && enemy.can_be_provoked() {
            let nearest = touching.iter().take(8).fold(Vec2::ZERO, |sum, &i| sum + Vec2::new(units[i].x, units[i].y)) / touching.len().min(8) as f32;
            if enemy.provoke(nearest) {
                fx.alarms.push((enemy.position, nearest));
            }
        }
        //a Freezing relic: an enemy being fought now and then freezes solid
        enemy.frozen = (enemy.frozen - dt).max(0.0);
        if leading.freeze > 0.0 && enemy.frozen <= 0.0 && enemy.can_freeze() && !touching.is_empty() && rand::gen_range(0.0, 1.0) < leading.freeze_chance(dt) {
            enemy.frozen = FREEZE_SECONDS;
            fx.burst(enemy.position, ICE, 8, 160.0);
            fx.cue_at(Cue::Frost, enemy.position);
        }
        if enemy.behavior == Behavior::Bomber && enemy.mode == Mode::Roam && !touching.is_empty() {
            enemy.mode = Mode::Fuse { left: BOMB_FUSE };
            enemy.flash = 0.0;
            fx.cue_at(Cue::Alert, enemy.position);
        }
        let mut budget = enemy.attack_now() * dt;
        let spent_from = budget;
        let mut took = [0.0f32; MAX_PLAYERS]; //how much of it each player's units took
        //the grid finds them in no order, and the first ones in the army are the first to be hit. an attack only gets through so many,
        //so they are put in order a batch at a time (the first few of thousands, then more if it is not used up) rather than all of them
        let mut done = 0;
        let mut batch = (budget / UNIT_HEALTH).ceil().min(touching.len() as f32) as usize + 16; //(no more than there are, so an attack out of all reason, as a damaged save can give, is not an overflow)
        while budget > 0.0 && done < touching.len() {
            let end = (done + batch).min(touching.len());
            let rest = &mut touching[done..];
            if end - done < rest.len() {
                rest.select_nth_unstable(end - done - 1);
            }
            rest[..end - done].sort_unstable();
            for &i in &touching[done..end] {
                if budget <= 0.0 {
                    break;
                }
                //a tougher unit takes less from the same hit, so the budget goes further. a champion, or a unit a champion is
                //looking after, takes less again, which hurt works out
                let unit = &mut units[i];
                let toughness = relics.of(unit.owner).toughness;
                let hit = budget.min((unit.health + unit.guard) / (toughness * unit.ward));
                unit.hurt(hit * toughness);
                budget -= hit;
                took[owner_index(unit.owner)] += hit;
                if unit.health <= 0.0 {
                    enemy.killed += 1;
                }
            }
            done = end;
            batch = batch.saturating_mul(4);
        }
        //thorns: it takes back some of what it did, as much as the relics of the units that took it say
        if budget < spent_from {
            let total: f32 = took.iter().sum();
            let back: f32 = took.iter().enumerate().filter(|(_, share)| **share > 0.0).map(|(owner, share)| (spent_from - budget) * (share / total) * relics.of(owner as u8).thorns).sum();
            if back > 0.0 {
                enemy.hurt += enemy.take_damage(back);
            }
        }
    }
    let sizes = army_sizes(units);
    resolve_deaths_among(enemies, babies, &sizes, relics, fx)
}

//remove every enemy that has run out of health, with one army to drop babies for
#[cfg(test)]
pub(crate) fn resolve_deaths<R: Relics + ?Sized>(enemies: &mut Vec<Enemy>, babies: &mut Vec<Baby>, army: usize, relics: &R, fx: &mut Effects) -> usize {
    resolve_deaths_among(enemies, babies, &[army], relics, fx)
}

//remove every enemy that has run out of health. one that was made by another is just gone, a bomber that set itself off
//too, anything else drops its baby units (more with the bounty relic of the player with the credit for it) and counts as destroyed. a
//splitter breaks into smaller ones, a frozen one shatters, a volatile one goes off, and whatever an enemy made dies with it. what it was
//worth goes to the player who has the credit, as their army's `spoils`. `armies` is how big each player's army is, by player.
//returns how many were destroyed
pub(crate) fn resolve_deaths_among<R: Relics + ?Sized>(enemies: &mut Vec<Enemy>, babies: &mut Vec<Baby>, armies: &[usize], relics: &R, fx: &mut Effects) -> usize {
    let mut destroyed = 0;
    let mut gone: Vec<u32> = Vec::new();
    let mut born: Vec<Enemy> = Vec::new();
    let mut captains: Vec<(Vec2, f32, Color)> = Vec::new(); //where the captains that fell were, how far their rally reached, and their colour
    enemies.retain_mut(|enemy| {
        if enemy.health > 0.0 {
            return true;
        }
        enemy.show_numbers(fx);
        gone.push(enemy.id);
        if enemy.is_captain() {
            captains.push((enemy.position, rally_reach(enemy), enemy.modifier.color()));
        }
        if enemy.blew_up {
            fx.burst(enemy.position, enemy.color, 14, 260.0);
            return false;
        }
        if enemy.is_minion() {
            fx.burst(enemy.position, enemy.color, (enemy.size / 4.0).clamp(6.0, 40.0) as usize, 220.0);
            fx.cue_at(Cue::Death, enemy.position);
            return false;
        }
        let upgrades = relics.of(enemy.credit);
        let army = armies[(enemy.credit as usize).min(armies.len() - 1)];
        drop_babies(enemy, babies, army, upgrades.bounty, upgrades.unit_cap());
        let spoils = &mut fx.spoils[owner_index(enemy.credit)];
        spoils.points += kill_points(enemy);
        spoils.xp += kill_xp(enemy);
        spoils.heal_all += upgrades.bloodlust * if enemy.boss.is_some() { 10.0 } else { 1.0 };
        let boss = enemy.boss.is_some();
        fx.add_shake(if boss { 1.0 } else { (enemy.size / 600.0).clamp(0.06, 0.7) });
        fx.burst(enemy.position, enemy.color, (enemy.size / 4.0).clamp(14.0, if boss { 400.0 } else { 160.0 }) as usize, 320.0);
        fx.burst(enemy.position, GREEN, enemy.babies.min(40), 200.0);
        fx.debris(enemy.position, enemy.color, enemy.size);
        fx.mark(enemy.position, (enemy.size * 0.7).clamp(22.0, 300.0), enemy.color, MarkKind::Splat);
        fx.cue_at(Cue::Thud, enemy.position);
        if enemy.modifier != Modifier::None && rand::gen_range(0.0, 1.0) < CHAMPION_ELITE_CHANCE {
            fx.recruits.push((enemy.position, Odds::Elite, owner_index(enemy.credit)));
        }
        if boss {
            fx.cue_at(Cue::Explosion, enemy.position);
            fx.mark(enemy.position, enemy.size * 0.9, enemy.color, MarkKind::Scorch);
        }
        if enemy.behavior == Behavior::Splitter {
            break_off(enemy, kind_named("Splitling"), 3, army, &mut born);
        }
        //a frozen one is brittle: it breaks into as many shards as it has size for, which are new enemies
        if enemy.shatters() {
            fx.burst(enemy.position, ICE, 28, 380.0);
            fx.cue_at(Cue::Frost, enemy.position);
            let pieces = ((enemy.size / SHATTER_PIECE_SIZE).round() as usize).clamp(SHATTER_MIN_PIECES, SHATTER_MAX_PIECES);
            break_off(enemy, kind_named("Shard"), pieces, army, &mut born);
        }
        if enemy.modifier == Modifier::Volatile {
            fx.hazards.push(Hazard::new(enemy.position, enemy.size * 0.9 + 90.0, 0.4, UNIT_HEALTH, victims_of(army, 0.015), enemy.color, Cue::Explosion));
        }
        destroyed += 1;
        false
    });
    if !gone.is_empty() {
        enemies.retain(|enemy| {
            let orphan = enemy.owner.is_some_and(|owner| gone.contains(&owner));
            if orphan {
                fx.burst(enemy.position, enemy.color, (enemy.size / 4.0).clamp(6.0, 40.0) as usize, 220.0);
            }
            !orphan
        });
    }
    enemies.extend(born);
    //the crowd a captain was rallying breaks when it falls
    for (at, reach, color) in captains {
        let routed = rout_followers(enemies, at, reach);
        fx.ring(at, reach, color);
        fx.add_punch(0.3);
        if routed > 0 {
            fx.popup(at, reach * 0.1, 0.0, "ROUTED!".to_owned(), color);
            fx.announce(format!("CAPTAIN FALLEN: {routed} ROUTED"), color);
        } else {
            fx.popup(at, reach * 0.1, 0.0, "CAPTAIN FALLEN".to_owned(), color);
        }
    }
    destroyed
}

//`count` enemies of a smaller kind that break off one that has been destroyed, where it was. what a horde's enemy breaks into is still
//part of the horde, so it isn't broken until they are gone too
fn break_off(enemy: &Enemy, kind: &'static EnemyKind, count: usize, army: usize, born: &mut Vec<Enemy>) {
    for _ in 0..count {
        let at = enemy.position + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(0.0, enemy.size * 0.3);
        let mut piece = Enemy::new(at, kind, minion_scale(enemy.scale));
        piece.empower(army_might(army));
        if in_a_horde(enemy) {
            join_horde(&mut piece);
        }
        born.push(piece);
    }
}

//what a shot does when it reaches a unit
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum ShotKind {
    Plain,    //hurts the first unit it reaches and is used up
    Piercing, //goes through the units in its way, hurting each, until it has been through as many as it can
    Splash,   //goes off where it lands (or on the first unit it reaches), hurting the units round that place
    Homing,   //bends towards the nearest unit as it flies, and flies for longer
}

//something an enemy that shoots has fired, which flies until it hits a unit or a wall, or gives up
pub(crate) struct Shot {
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2, //pixels per second
    pub(crate) age: f32,       //seconds
    pub(crate) damage: f32,
    pub(crate) radius: f32,
    pub(crate) color: Color,
    pub(crate) kind: ShotKind,
    pub(crate) life: f32,         //seconds it flies for, a splash shot goes off when it is up
    pub(crate) pierce: usize,     //units a piercing shot can still go through
    pub(crate) struck: Vec<usize>, //the ones it has gone through, so it does not hurt one twice
    pub(crate) splash: f32,       //pixels a splash shot reaches where it goes off
    pub(crate) victims: usize,    //and the most units it hurts
}

impl Shot {
    pub(crate) fn new(position: Vec2, velocity: Vec2) -> Shot {
        Shot { position, velocity, age: 0.0, damage: SHOT_DAMAGE, radius: SHOT_RADIUS, color: ORANGE, kind: ShotKind::Plain, life: SHOT_LIFE, pierce: 0, struck: Vec::new(), splash: 0.0, victims: 0 }
    }

    //the bigger, harder hitting kind a boss fires
    pub(crate) fn heavy(position: Vec2, velocity: Vec2, color: Color) -> Shot {
        Shot { damage: SHOT_DAMAGE * 1.2, radius: SHOT_RADIUS * 1.6, color, ..Shot::new(position, velocity) }
    }

    //a bolt that goes through up to `pierce` units, hurting each
    pub(crate) fn lance(position: Vec2, velocity: Vec2, pierce: usize, color: Color) -> Shot {
        Shot { damage: SHOT_DAMAGE * LANCE_DAMAGE, radius: SHOT_RADIUS * 0.85, color, kind: ShotKind::Piercing, pierce: pierce.max(1), ..Shot::new(position, velocity) }
    }

    //a grenade that goes off `life` seconds from now, or sooner on a unit, hurting up to `victims` units within `splash` pixels
    pub(crate) fn grenade(position: Vec2, velocity: Vec2, life: f32, splash: f32, victims: usize, color: Color) -> Shot {
        Shot { damage: SHOT_DAMAGE * GRENADE_DAMAGE, radius: SHOT_RADIUS * 1.5, color, kind: ShotKind::Splash, life, splash, victims, ..Shot::new(position, velocity) }
    }

    //an orb that bends towards the nearest unit
    pub(crate) fn seeker(position: Vec2, velocity: Vec2, color: Color) -> Shot {
        Shot { damage: SHOT_DAMAGE * SEEKER_DAMAGE, radius: SHOT_RADIUS * 1.1, color, kind: ShotKind::Homing, life: SEEKER_LIFE, ..Shot::new(position, velocity) }
    }
}

//an enemy that shoots fires at the nearest unit within its range whenever it is ready: a volley of ordinary shots, a bolt that goes through
//the units in its way, grenades that go off where they land, or orbs that home in on the army
pub(crate) fn shoot_enemies(enemies: &mut [Enemy], units: &[Unit], shots: &mut Vec<Shot>, dt: f32, fx: &mut Effects) {
    let army = units.len();
    for enemy in enemies.iter_mut().filter(|enemy| enemy.behavior == Behavior::Shooter && enemy.range > 0.0 && enemy.frozen <= 0.0 && !enemy.routed()) {
        enemy.shot_in -= dt;
        if enemy.shot_in > 0.0 {
            continue;
        }
        let nearest = units
            .iter()
            .map(|unit| Vec2::new(unit.x, unit.y))
            .filter(|position| position.distance(enemy.position) <= enemy.range)
            .min_by(|a, b| a.distance_squared(enemy.position).total_cmp(&b.distance_squared(enemy.position)));
        let Some(target) = nearest else {
            //nothing to shoot at, look again soon
            enemy.shot_in = 0.25;
            continue;
        };
        let aim = (target - enemy.position).normalize_or_zero();
        let from = |direction: Vec2| enemy.position + direction * enemy.size / 2.0;
        match kind_named(enemy.name).shot {
            ShotKind::Plain => {
                enemy.shot_in = SHOT_INTERVAL * rand::gen_range(0.8, 1.2);
                let first = -(enemy.volley as f32 - 1.0) / 2.0;
                for k in 0..enemy.volley {
                    let direction = aim.rotate(Vec2::from_angle((first + k as f32) * SHOT_SPREAD));
                    shots.push(Shot::new(from(direction), direction * SHOT_SPEED));
                }
            }
            //a bolt takes longer to ready, and goes through more of a bigger army
            ShotKind::Piercing => {
                enemy.shot_in = SHOT_INTERVAL * 1.7 * rand::gen_range(0.8, 1.2);
                let bolts = (enemy.volley / 3).max(1);
                let pierce = (LANCE_PIERCE + army / 400).min(LANCE_PIERCE_MAX);
                let first = -(bolts as f32 - 1.0) / 2.0;
                for k in 0..bolts {
                    let direction = aim.rotate(Vec2::from_angle((first + k as f32) * SHOT_SPREAD * 1.5));
                    shots.push(Shot::lance(from(direction), direction * LANCE_SPEED, pierce, enemy.color));
                }
            }
            //the grenades come down on and round the nearest unit, each going off where it was aimed
            ShotKind::Splash => {
                enemy.shot_in = SHOT_INTERVAL * 1.8 * rand::gen_range(0.8, 1.2);
                let grenades = (enemy.volley / 3).max(1);
                let splash = GRENADE_RADIUS * blast_scale(enemy.scale);
                let victims = victims_of(army, GRENADE_VICTIMS);
                for k in 0..grenades {
                    let scatter = if k == 0 { Vec2::ZERO } else { Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(0.0, splash * 1.5) };
                    let goal = target + scatter;
                    let direction = (goal - enemy.position).normalize_or_zero();
                    let flight = (goal.distance(enemy.position) - enemy.size / 2.0).max(0.0) / GRENADE_SPEED;
                    shots.push(Shot::grenade(from(direction), direction * GRENADE_SPEED, flight, splash, victims, enemy.color));
                }
            }
            ShotKind::Homing => {
                enemy.shot_in = SHOT_INTERVAL * 1.6 * rand::gen_range(0.8, 1.2);
                let orbs = (enemy.volley / 2).clamp(1, 5);
                let first = -(orbs as f32 - 1.0) / 2.0;
                for k in 0..orbs {
                    let direction = aim.rotate(Vec2::from_angle((first + k as f32) * SHOT_SPREAD * 2.5));
                    shots.push(Shot::seeker(from(direction), direction * SEEKER_SPEED, enemy.color));
                }
            }
        }
        fx.cue_at(Cue::Shoot, enemy.position);
    }
}

const SHOT_CELL: f32 = 64.0; //units are sorted into squares this big for the shots to look through

//the units, sorted into squares (by their index) for the shots
fn unit_squares(units: &[Unit]) -> TileMap<Vec<usize>> {
    let mut grid: TileMap<Vec<usize>> = TileMap::default();
    for (i, unit) in units.iter().enumerate() {
        grid.entry(((unit.x / SHOT_CELL) as i32, (unit.y / SHOT_CELL) as i32)).or_default().push(i);
    }
    grid
}

//the living units that are within `reach` of a point, found through the squares they were sorted into
fn units_near(grid: &TileMap<Vec<usize>>, units: &[Unit], at: Vec2, reach: f32, found: &mut Vec<usize>) {
    let cells = (reach / SHOT_CELL).ceil() as i32 + 1;
    let middle = ((at.x / SHOT_CELL) as i32, (at.y / SHOT_CELL) as i32);
    for cell_x in middle.0 - cells..=middle.0 + cells {
        for cell_y in middle.1 - cells..=middle.1 + cells {
            let Some(in_cell) = grid.get(&(cell_x, cell_y)) else {
                continue;
            };
            for &i in in_cell {
                let unit = &units[i];
                if unit.health > 0.0 && unit.garrison.is_none() && Vec2::new(unit.x, unit.y).distance(at) <= reach {
                    found.push(i);
                }
            }
        }
    }
}

//a splash shot goes off where it is: up to `victims` of the units within its reach (the closest ones) are hurt, the walls there too,
//and it leaves a scorch on the ground
fn detonate<R: Relics + ?Sized>(shot: &Shot, grid: &TileMap<Vec<usize>>, units: &mut [Unit], relics: &R, fx: &mut Effects) {
    let mut near = Vec::new();
    units_near(grid, units, shot.position, shot.splash, &mut near);
    if shot.victims > 0 && near.len() > shot.victims {
        near.select_nth_unstable_by(shot.victims - 1, |&a, &b| {
            let (a, b) = (Vec2::new(units[a].x, units[a].y), Vec2::new(units[b].x, units[b].y));
            a.distance_squared(shot.position).total_cmp(&b.distance_squared(shot.position))
        });
        near.truncate(shot.victims);
    }
    for &i in &near {
        let toughness = relics.of(units[i].owner).toughness;
        units[i].hurt(shot.damage * toughness);
    }
    fx.blasts.push(Blast { position: shot.position, radius: shot.splash, damage: shot.damage * BLAST_WALL_FACTOR });
    fx.burst(shot.position, shot.color, (shot.splash / 5.0).clamp(10.0, 60.0) as usize, shot.splash * 2.0);
    fx.ring(shot.position, shot.splash, shot.color);
    fx.mark(shot.position, shot.splash * 0.7, shot.color, MarkKind::Scorch);
    fx.add_shake(0.04);
    fx.cue_at(Cue::Explosion, shot.position);
}

//where the nearest living unit within `reach` of a point is
fn nearest_unit(grid: &TileMap<Vec<usize>>, units: &[Unit], at: Vec2, reach: f32) -> Option<Vec2> {
    let mut near = Vec::new();
    units_near(grid, units, at, reach, &mut near);
    near.into_iter().map(|i| Vec2::new(units[i].x, units[i].y)).min_by(|a, b| a.distance_squared(at).total_cmp(&b.distance_squared(at)))
}

//turn a velocity towards a point by at most `turn` radians, keeping its speed
fn bend_towards(velocity: Vec2, from: Vec2, to: Vec2, turn: f32) -> Vec2 {
    let wanted = (to - from).normalize_or_zero();
    let heading = velocity.normalize_or_zero();
    if wanted == Vec2::ZERO || heading == Vec2::ZERO {
        return velocity;
    }
    let angle = heading.perp_dot(wanted).atan2(heading.dot(wanted)).clamp(-turn, turn);
    velocity.rotate(Vec2::from_angle(angle))
}

//fly the shots. an ordinary one that touches a unit hurts it and is used up, and so is one that hits a wall or has flown too long. a
//piercing one goes through the units it meets, hurting each, until it has been through as many as it can. a splash one goes off on the
//first unit it meets, or where it was aimed, and hurts the units round that. a homing one bends towards the nearest unit as it flies.
//the units are sorted into a grid first, there can be a lot of shots and a lot of units
pub(crate) fn update_shots<R: Relics + ?Sized>(shots: &mut Vec<Shot>, units: &mut [Unit], walls: &Walls, dt: f32, relics: &R, fx: &mut Effects) {
    if shots.is_empty() {
        return;
    }
    let grid = unit_squares(units);
    shots.retain_mut(|shot| {
        shot.age += dt;
        if shot.age > shot.life {
            //a grenade that has flown as far as it was aimed goes off, any other shot is just gone
            if shot.kind == ShotKind::Splash {
                detonate(shot, &grid, units, relics, fx);
            }
            return false;
        }
        if shot.kind == ShotKind::Homing {
            if let Some(target) = nearest_unit(&grid, units, shot.position, SEEKER_REACH) {
                shot.velocity = bend_towards(shot.velocity, shot.position, target, SEEKER_TURN * dt);
            }
        }
        let reach = RADIUS + shot.radius;
        //a shot goes a little at a time so it can't jump over a unit or a wall
        let steps = (shot.velocity.length() * dt / reach).ceil().max(1.0) as i32;
        for _ in 0..steps {
            shot.position += shot.velocity * dt / steps as f32;
            let (col, row) = tile_of(shot.position.x, shot.position.y);
            if !(0..WORLD_TILES).contains(&col) || !(0..WORLD_TILES).contains(&row) {
                return false;
            }
            //the player's walls stop a shot and take its damage, a lair's let it through
            if walls.get(&(col, row)).is_some_and(|wall| wall.stops_shots()) {
                if shot.kind == ShotKind::Splash {
                    detonate(shot, &grid, units, relics, fx);
                } else {
                    fx.wall_hits.push(((col, row), shot.damage * SHOT_WALL_SHARE));
                    fx.burst(shot.position, shot.color, 5, 120.0);
                }
                return false;
            }
            let cell = ((shot.position.x / SHOT_CELL) as i32, (shot.position.y / SHOT_CELL) as i32);
            for cell_x in cell.0 - 1..=cell.0 + 1 {
                for cell_y in cell.1 - 1..=cell.1 + 1 {
                    let Some(in_cell) = grid.get(&(cell_x, cell_y)) else {
                        continue;
                    };
                    for &i in in_cell {
                        let unit = &mut units[i];
                        if unit.health <= 0.0 || unit.garrison.is_some() || Vec2::new(unit.x, unit.y).distance(shot.position) >= reach {
                            continue;
                        }
                        match shot.kind {
                            ShotKind::Splash => {
                                detonate(shot, &grid, units, relics, fx);
                                return false;
                            }
                            ShotKind::Piercing => {
                                if shot.struck.contains(&i) {
                                    continue;
                                }
                                unit.hurt(shot.damage * relics.of(unit.owner).toughness);
                                fx.burst(shot.position, shot.color, 4, 140.0);
                                shot.struck.push(i);
                                shot.pierce = shot.pierce.saturating_sub(1);
                                if shot.pierce == 0 {
                                    return false;
                                }
                            }
                            ShotKind::Plain | ShotKind::Homing => {
                                unit.hurt(shot.damage * relics.of(unit.owner).toughness);
                                fx.burst(shot.position, shot.color, 6, 160.0);
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    });
}

//an ordinary shot is a small bright circle with a glow round it. a piercing one is a streak, a grenade a big slow ball with a ring on the
//ground where it will land, and a homing one an orb with a tail
pub(crate) fn draw_shots(shots: &[Shot]) {
    let time = get_time() as f32;
    for shot in shots {
        let (x, y) = (shot.position.x, shot.position.y);
        let glow = Color::new(shot.color.r, shot.color.g * 0.8, shot.color.b, 0.25);
        match shot.kind {
            ShotKind::Plain => {
                draw_circle(x, y, shot.radius * 2.2, glow);
                draw_circle(x, y, shot.radius, shot.color);
            }
            ShotKind::Piercing => {
                let along = shot.velocity.normalize_or_zero();
                let tail = Vec2::new(x, y) - along * shot.radius * 9.0;
                draw_line(tail.x, tail.y, x, y, shot.radius * 2.6, glow);
                draw_line(tail.x, tail.y, x, y, shot.radius * 1.1, lighten(shot.color, 0.6));
                draw_circle(x, y, shot.radius, WHITE);
            }
            ShotKind::Splash => {
                //where it will land, so the army can be somewhere else
                let landing = Vec2::new(x, y) + shot.velocity * (shot.life - shot.age).max(0.0);
                let pulse = 0.5 + 0.5 * (time * 10.0).sin();
                draw_circle(landing.x, landing.y, shot.splash, Color::new(shot.color.r, shot.color.g, shot.color.b, 0.07 + 0.05 * pulse));
                draw_circle_lines(landing.x, landing.y, shot.splash, 2.5, danger_color(0.4 + 0.4 * pulse));
                draw_circle(x, y, shot.radius * 1.8, glow);
                draw_circle(x, y, shot.radius, shot.color);
                draw_circle_lines(x, y, shot.radius, 2.0, WHITE);
            }
            ShotKind::Homing => {
                let along = shot.velocity.normalize_or_zero();
                let tail = Vec2::new(x, y) - along * shot.radius * 5.0;
                let pulse = 0.7 + 0.3 * (time * 14.0 + x * 0.05).sin();
                draw_line(tail.x, tail.y, x, y, shot.radius * 1.4, glow);
                draw_circle(x, y, shot.radius * 2.0 * pulse, glow);
                draw_circle(x, y, shot.radius, shot.color);
                draw_circle(x, y, shot.radius * 0.45, WHITE);
            }
        }
    }
}

pub(crate) const HAZARD_FLASH: f32 = 0.25; //seconds a hazard that has gone off stays lit

//a place that is about to hurt the units in it, shown on the ground until it does: a shell that is landing, a bomb, a
//boss's stomp. it goes off once, hurting up to `victims` of the units inside it, and may go on burning afterwards
pub(crate) struct Hazard {
    pub(crate) position: Vec2,
    pub(crate) radius: f32,
    pub(crate) armed: bool,       //it has not gone off yet
    pub(crate) warn: f32,         //seconds until it does
    pub(crate) warn_total: f32,
    pub(crate) damage: f32,       //to each unit in it when it goes off
    pub(crate) victims: usize,    //the most units it hurts when it goes off
    pub(crate) linger: f32,       //seconds it burns for afterwards
    pub(crate) burn: f32,         //damage a second to every unit in it while it burns
    pub(crate) flash: f32,        //seconds left of the flash when it has gone off
    pub(crate) color: Color,
    pub(crate) boom: Cue,         //the sound it makes when it goes off
    pub(crate) wall_damage: f32,  //to each of the player's walls in it when it goes off
    pub(crate) circle: Option<BossCircle>, //which of a boss's circles it is, if it is one, which sets its colour
}

impl Hazard {
    pub(crate) fn new(position: Vec2, radius: f32, warn: f32, damage: f32, victims: usize, color: Color, boom: Cue) -> Hazard {
        Hazard { position, radius, armed: true, warn, warn_total: warn.max(0.001), damage, victims, linger: 0.0, burn: 0.0, flash: 0.0, color, boom, wall_damage: damage * BLAST_WALL_FACTOR, circle: None }
    }

    //it hurts walls this many times as much as usual: a bigger enemy's blast breaks a wall that was built for a bigger army
    pub(crate) fn vs_walls(mut self, factor: f32) -> Hazard {
        self.wall_damage *= factor;
        self
    }

    //it burns for `seconds` after it goes off, doing `per_second` damage to every unit in it
    pub(crate) fn burning(mut self, seconds: f32, per_second: f32) -> Hazard {
        self.linger = seconds;
        self.burn = per_second;
        self
    }

    //it is one of a boss's circles, drawn and named as that kind
    pub(crate) fn circle(mut self, circle: BossCircle) -> Hazard {
        self.circle = Some(circle);
        self
    }
}

//count the hazards down, set off the ones that are due and burn the units in the ones that are burning
pub(crate) fn update_hazards<R: Relics + ?Sized>(hazards: &mut Vec<Hazard>, units: &mut [Unit], relics: &R, dt: f32, fx: &mut Effects) {
    let mut burning: Vec<(Vec2, f32, f32)> = Vec::new(); //where, how far it reaches and how hard it burns
    hazards.retain_mut(|hazard| {
        if hazard.armed {
            hazard.warn -= dt;
            if hazard.warn > 0.0 {
                return true;
            }
            hazard.armed = false;
            hazard.flash = HAZARD_FLASH;
            if hazard.wall_damage > 0.0 {
                fx.blasts.push(Blast { position: hazard.position, radius: hazard.radius, damage: hazard.wall_damage });
            }
            let (mut hurt, mut killed) = (0, 0);
            if hazard.damage > 0.0 {
                for unit in units.iter_mut() {
                    if hurt >= hazard.victims {
                        break;
                    }
                    if unit.health > 0.0 && unit.garrison.is_none() && Vec2::new(unit.x, unit.y).distance(hazard.position) <= hazard.radius {
                        unit.hurt(hazard.damage * relics.of(unit.owner).toughness);
                        hurt += 1;
                        killed += (unit.health <= 0.0) as usize;
                    }
                }
            }
            fx.burst(hazard.position, hazard.color, (hazard.radius / 5.0).clamp(12.0, 80.0) as usize, hazard.radius * 2.0);
            fx.add_shake((hazard.radius / 1500.0).clamp(0.05, 0.6));
            fx.mark(hazard.position, hazard.radius * 0.85, hazard.color, MarkKind::Scorch);
            fx.cue_at(hazard.boom, hazard.position);
            if killed > 0 {
                fx.popup(hazard.position, 0.0, 0.0, format!("-{killed} units"), Color::new(1.0, 0.35, 0.35, 1.0));
            }
            return hazard.linger > 0.0 || hazard.flash > 0.0;
        }
        hazard.flash = (hazard.flash - dt).max(0.0);
        if hazard.linger > 0.0 {
            hazard.linger -= dt;
            if hazard.burn > 0.0 {
                burning.push((hazard.position, hazard.radius, hazard.burn));
            }
        }
        hazard.linger > 0.0 || hazard.flash > 0.0
    });
    //fire doesn't add up: a unit standing in several burning places takes the worst of them, so a pile of them on one army can't
    //melt it in a moment
    if !burning.is_empty() {
        for unit in units.iter_mut().filter(|unit| unit.health > 0.0 && unit.garrison.is_none()) {
            let at = Vec2::new(unit.x, unit.y);
            let worst = burning.iter().filter(|(position, radius, _)| at.distance(*position) <= *radius).map(|(_, _, burn)| *burn).fold(0.0, f32::max);
            if worst > 0.0 {
                unit.hurt(worst * relics.of(unit.owner).toughness * dt);
            }
        }
    }
}

//hazards on the ground: one that has not gone off is a red disc that fills up as the time runs out, one that has is a
//flash, and one that is burning flickers orange
pub(crate) fn draw_hazards(hazards: &[Hazard], time: f32) {
    for hazard in hazards {
        let (x, y, radius) = (hazard.position.x, hazard.position.y, hazard.radius);
        let sides = (radius.sqrt() * 3.0).clamp(24.0, 96.0) as u8;
        let color = hazard.color;
        if hazard.armed {
            let filled = 1.0 - (hazard.warn / hazard.warn_total).clamp(0.0, 1.0);
            let pulse = 0.5 + 0.5 * (time * 12.0).sin();
            //one of a boss's circles is drawn in the colour of its kind, and the rest are red
            let edge = match hazard.circle {
                Some(circle) => {
                    let tint = circle.color();
                    Color::new(tint.r, tint.g, tint.b, 0.55 + 0.45 * pulse)
                }
                None => danger_color(0.55 + 0.45 * pulse),
            };
            let fill = hazard.circle.map_or(color, BossCircle::color);
            draw_poly(x, y, sides, radius, 0.0, Color::new(fill.r, fill.g, fill.b, 0.10 + 0.06 * pulse));
            draw_poly(x, y, sides, radius * filled, 0.0, Color::new(fill.r, fill.g, fill.b, 0.22));
            draw_poly_lines(x, y, sides, radius, 0.0, (radius * 0.02).clamp(3.0, 10.0), edge);
            continue;
        }
        if hazard.flash > 0.0 {
            let fade = hazard.flash / HAZARD_FLASH;
            draw_poly(x, y, sides, radius, 0.0, Color::new(1.0, 0.95, 0.8, 0.55 * fade));
        }
        if hazard.linger > 0.0 {
            let flicker = 0.5 + 0.5 * (time * 9.0 + x * 0.01).sin();
            let fade = hazard.linger.min(1.0);
            draw_poly(x, y, sides, radius, 0.0, Color::new(1.0, 0.45, 0.1, (0.16 + 0.1 * flicker) * fade));
            draw_poly_lines(x, y, sides, radius, 0.0, 4.0, Color::new(1.0, 0.6, 0.15, (0.5 + 0.3 * flicker) * fade));
        }
    }
}

//a lone baby drops in the middle of the enemy that was destroyed, more are scattered over the square it filled. the
//enemy drops as many as it has times the bounty, or as many as there is room for in an army that can be `cap` big: a baby
//that could never grow isn't dropped
pub(crate) fn drop_babies(enemy: &Enemy, babies: &mut Vec<Baby>, army: usize, bounty: f32, cap: usize) {
    let half = enemy.size / 2.0;
    let count = ((enemy.babies as f32 * bounty).round() as usize).min(cap.saturating_sub(army + babies.len()));
    for _ in 0..count {
        let offset = if count == 1 { Vec2::ZERO } else { Vec2::new(rand::gen_range(-half, half), rand::gen_range(-half, half)) };
        let position = (enemy.position + offset).clamp(Vec2::splat(BABY_RADIUS), Vec2::splat(WORLD_SIZE - BABY_RADIUS));
        babies.push(Baby { position, age: 0.0 });
    }
}


//the wall points an enemy is worth when it is destroyed: a bit for each, more for the ones that drop babies, a lot for a boss
pub(crate) fn kill_points(enemy: &Enemy) -> f32 {
    POINTS_PER_KILL + enemy.babies as f32 * POINTS_PER_BABY + if enemy.boss.is_some() { BOSS_POINTS } else { 0.0 }
}
