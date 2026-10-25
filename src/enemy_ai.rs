//! what enemies do each frame: patrol, chase, charge, heal, summon and lob shells

use crate::prelude::*;

//random points to patrol between, all inside the ENEMY_PATROL_TILES square around where the enemy started
pub(crate) fn patrol_route(start: Vec2, size: f32) -> Vec<Vec2> {
    let half_area = ENEMY_PATROL_TILES * TILE_SIZE / 2.0;
    let edge = size / 2.0;
    let count = rand::gen_range(ENEMY_ROUTE_MIN_POINTS, ENEMY_ROUTE_MAX_POINTS + 1);
    (0..count)
        .map(|_| {
            //the world's edge cuts the area short when the enemy started near it
            let x = rand::gen_range(start.x - half_area, start.x + half_area).clamp(edge, WORLD_SIZE - edge);
            let y = rand::gen_range(start.y - half_area, start.y + half_area).clamp(edge, WORLD_SIZE - edge);
            Vec2::new(x, y)
        })
        .collect()
}

pub(crate) const HEALER_STAY: f32 = 150.0; //pixels, a healer goes to the nearest enemy that is not one and stays this close to it

//the same for one player and a speed that is a number: what the tests that have just the one player and an enemy use
#[cfg(test)]
pub(crate) fn patrol_enemies(enemies: &mut [Enemy], player: Vec2, dt: f32, speed_multiplier: f32, walls: &Walls, field: &FlowField, fx: &mut Effects) {
    let prey = [Prey { position: player, units: &[], team: 0, pressure: 0, boost: speed_multiplier, cap: DEFAULT_ARMY_LIMIT, field }];
    patrol_enemies_among(enemies, &prey, dt, walls, fx);
}

//walk every enemy along its route, at its own speed times the multiplier the size of the army it is near gives. one that has noticed a
//player (they came within its aggro distance; the one it goes after is the one nearest to it) runs at them instead, faster, until they
//get too far away. a charger stops and shows where it will dash instead, a healer goes to the nearest other enemy, and anything in the
//middle of a dash or a cast carries on with that. none of them can walk through the player's walls: the ones that chase follow the
//route the field of the player they are after gives them round the walls, and chew through the ones that are in the way, and the others
//slide along a wall until they give up on the waypoint they were heading for
pub(crate) fn patrol_enemies_among(enemies: &mut [Enemy], prey: &[Prey], dt: f32, walls: &Walls, fx: &mut Effects) {
    //a healer goes to the others, so where they are is needed while one of them is borrowed
    let others: Vec<Vec2> = if enemies.iter().any(|enemy| enemy.behavior == Behavior::Healer) {
        enemies.iter().filter(|enemy| enemy.behavior != Behavior::Healer).map(|enemy| enemy.position).collect()
    } else {
        Vec::new()
    };
    let no_route = FlowField::new();
    //the tank champions of each player: an enemy chasing a player that comes near one goes for it instead (see `Champion::taunts`)
    let taunts: Vec<Vec<Vec2>> = prey
        .iter()
        .map(|hunted| hunted.units.iter().filter(|unit| unit.garrison.is_none() && unit.health > 0.0 && unit.champion_kind().is_some_and(|kind| kind.taunts())).map(|unit| Vec2::new(unit.x, unit.y)).collect())
        .collect();
    for enemy in enemies {
        let which = nearest_to(prey, enemy.position);
        let hunted = &prey[which];
        let (player, speed_multiplier, field) = (hunted.position, hunted.boost, hunted.field);
        enemy.pressing = None;
        enemy.provoked = (enemy.provoked - dt).max(0.0);
        if enemy.routed() {
            enemy.rout = (enemy.rout - dt).max(0.0);
            //(it looks for its way again as soon as it turns back)
            if !enemy.routed() {
                enemy.steer_in = 0.0;
            }
        }
        //a frozen one is held where it is
        if enemy.frozen > 0.0 {
            continue;
        }
        //a nest stays where it is, and so does an ambusher until it springs
        if enemy.behavior == Behavior::Nest || enemy.lurking() {
            continue;
        }
        //one whose captain has fallen runs from the player, whatever else it was doing
        if enemy.routed() {
            let goal = enemy.position + (enemy.position - player).normalize_or_zero() * 600.0;
            let speed = enemy.speed * CHASE_SPEED * speed_multiplier * enemy.pace() * ROUT_SPEED;
            walk(enemy, goal, 0.0, speed, dt, false, walls, &no_route);
            continue;
        }
        if enemy.carry_on(dt, speed_multiplier, walls, fx) {
            continue;
        }
        if enemy.behavior == Behavior::Charger {
            let to_player = player - enemy.position;
            if to_player.length() <= enemy.aggro {
                enemy.mode = Mode::Windup { dir: to_player.normalize_or_zero(), left: CHARGE_WINDUP };
                fx.popup(enemy.position, enemy.size / 2.0, 0.0, "!".to_owned(), RED);
                fx.cue_at(Cue::Alert, enemy.position);
                continue;
            }
        } else if enemy.aggro > 0.0 {
            let distance = player.distance(enemy.position);
            if !enemy.chasing && distance <= enemy.aggro {
                enemy.chasing = true;
                fx.popup(enemy.position, enemy.size / 2.0, 0.0, "!".to_owned(), RED);
                fx.cue_at(Cue::Alert, enemy.position);
            } else if enemy.chasing && distance > enemy.aggro * CHASE_LEASH {
                enemy.chasing = false;
            }
        }
        //one that units have hurt fights them where they are instead of walking off along its route. the ones that call up help
        //(summoners) or mend (healers) stay where they are
        if enemy.provoked > 0.0 && !enemy.chasing && enemy.can_be_provoked() && !matches!(enemy.behavior, Behavior::Healer | Behavior::Summoner) {
            let goal = enemy.grudge;
            let stop = if enemy.range > 0.0 && matches!(enemy.behavior, Behavior::Shooter | Behavior::Mortar) { enemy.range * CHASE_STANDOFF } else { CHASE_STOP };
            let speed = enemy.speed * CHASE_SPEED * speed_multiplier * enemy.pace();
            walk(enemy, goal, stop, speed, dt, true, walls, &no_route);
            continue;
        }
        if enemy.chasing {
            let speed = enemy.speed * CHASE_SPEED * speed_multiplier * enemy.pace();
            //a boss that keeps its distance holds its ground inside its barriers, and otherwise stays out of the player's reach
            let (keeps_away, anchored) = enemy.boss.as_ref().map_or((0.0, false), |state| (state.def.keeps_away, state.anchor_left > 0.0));
            if anchored {
                continue;
            }
            if keeps_away > 0.0 {
                if player.distance(enemy.position) < keeps_away * 0.8 {
                    let goal = enemy.position + (enemy.position - player).normalize_or_zero() * 600.0;
                    walk(enemy, goal, 0.0, speed, dt, false, walls, &no_route);
                } else {
                    walk(enemy, player, keeps_away, speed, dt, true, walls, field);
                }
                continue;
            }
            //one that shoots or lobs shells (only a horde sends them running) stops at a distance and fires from there
            let stop = if enemy.range > 0.0 && matches!(enemy.behavior, Behavior::Shooter | Behavior::Mortar) { enemy.range * CHASE_STANDOFF } else { CHASE_STOP };
            //a tank champion near it draws it off the player (a boss is not so easily led)
            let at = enemy.position;
            let taunted = if enemy.boss.is_some() { None } else { taunts[which].iter().copied().filter(|spot| spot.distance_squared(at) <= TAUNT_RANGE * TAUNT_RANGE).min_by(|a, b| a.distance_squared(at).total_cmp(&b.distance_squared(at))) };
            match taunted {
                Some(champion) => walk(enemy, champion, stop, speed, dt, true, walls, &no_route),
                None => walk(enemy, player, stop, speed, dt, true, walls, field),
            }
            continue;
        }
        if enemy.behavior == Behavior::Healer {
            let position = enemy.position;
            let nearest = others.iter().copied().min_by(|a, b| a.distance_squared(position).total_cmp(&b.distance_squared(position)));
            if let Some(nearest) = nearest {
                let speed = enemy.speed * CHASE_SPEED * speed_multiplier * enemy.pace();
                walk(enemy, nearest, HEALER_STAY, speed, dt, false, walls, &no_route);
                continue;
            }
        }
        let target = enemy.route[enemy.heading_for];
        let speed = enemy.speed * speed_multiplier * enemy.pace();
        walk(enemy, target, 0.0, speed, dt, false, walls, &no_route);
        //arrived, or held up by a wall for too long to be going to get there
        if enemy.position.distance(target) < 1.0 || enemy.stuck > ROUTE_GIVE_UP {
            enemy.heading_for = (enemy.heading_for + 1) % enemy.route.len();
            enemy.stuck = 0.0;
        }
    }
}

//the reach of a captain's rally: RALLY_TILES from its edge
pub(crate) fn rally_reach(captain: &Enemy) -> f32 {
    RALLY_TILES * TILE_SIZE + captain.size / 2.0
}

//the crowd of a horde that a living captain is leading is rallied while it is near the captain: faster and harder hitting. who is
//rallied is worked out again every frame, so a captain's death ends it, and one that is running away is not. returns how many are
pub(crate) fn rally_followers(enemies: &mut [Enemy]) -> usize {
    let captains: Vec<(Vec2, f32)> = enemies.iter().filter(|enemy| enemy.is_captain() && enemy.health > 0.0).map(|captain| (captain.position, rally_reach(captain))).collect();
    let mut rallied = 0;
    for enemy in enemies.iter_mut() {
        enemy.rallied = !captains.is_empty()
            && in_a_horde(enemy)
            && !enemy.is_captain()
            && enemy.boss.is_none()
            && !enemy.routed()
            && captains.iter().any(|&(at, reach)| at.distance_squared(enemy.position) <= reach * reach);
        rallied += enemy.rallied as usize;
    }
    rallied
}

//a captain has fallen at a place: the crowd of its horde that was inside its rally, `reach` pixels round it, breaks and runs. returns
//how many ran
pub(crate) fn rout_followers(enemies: &mut [Enemy], at: Vec2, reach: f32) -> usize {
    let mut routed = 0;
    for enemy in enemies.iter_mut().filter(|enemy| in_a_horde(enemy) && !enemy.is_captain() && enemy.boss.is_none() && enemy.health > 0.0 && enemy.position.distance_squared(at) <= reach * reach) {
        enemy.break_and_run();
        routed += 1;
    }
    routed
}

//one step of an enemy towards a goal, stopping `stop` short of it, at a speed in pixels a second. it works out where to head
//every STEERING_INTERVAL, or when it gets where it was heading
pub(crate) fn walk(enemy: &mut Enemy, goal: Vec2, stop: f32, speed: f32, dt: f32, chew: bool, walls: &Walls, field: &FlowField) {
    let distance = enemy.position.distance(goal);
    if distance <= stop {
        enemy.stuck = 0.0;
        return;
    }
    let step_length = speed * dt;
    enemy.steer_in -= dt;
    let arrived = matches!(enemy.steer, Steer::Point(point) if enemy.position.distance(point) <= step_length);
    if enemy.steer_in <= 0.0 || arrived {
        //a square gets through gaps its circle fits, and it can't be steered by a circle bigger than the gap it is for
        let clearance = (enemy.wall_radius() - 1.0).max(4.0);
        enemy.steer = steer_along(enemy.position, goal, clearance, Mover::Enemy, walls, field);
        enemy.steer_in = STEERING_INTERVAL;
    }
    let (target, reach) = match enemy.steer {
        Steer::Player => (goal, distance - stop),
        Steer::Point(point) => (point, enemy.position.distance(point)),
    };
    let direction = (target - enemy.position).normalize_or_zero();
    enemy.advance(direction * step_length.min(reach.max(0.0)), dt, chew, walls);
}

//enemies held up by the player's walls wear them down, by their attack a second (a dash hits harder), and giants crush
//every wall they are standing on. the damage goes to `fx` for update_walls to apply
pub(crate) fn chew_walls(enemies: &[Enemy], walls: &Walls, dt: f32, fx: &mut Effects) {
    for enemy in enemies {
        if let Some(tile) = enemy.pressing {
            if walls.get(&tile).is_some_and(|wall| wall.side == Side::Friendly) {
                fx.wall_hits.push((tile, enemy.attack_now() * ENEMY_CHEW_SHARE * dt));
            }
        }
        if enemy.size / 2.0 > ENEMY_WALL_RADIUS_MAX && (enemy.chasing || enemy.boss.is_some()) {
            let (col, row) = tile_of(enemy.position.x, enemy.position.y);
            let body = Rect::new(enemy.position.x - enemy.size / 2.0, enemy.position.y - enemy.size / 2.0, enemy.size, enemy.size);
            for wall_col in (col - CRUSH_TILES)..=(col + CRUSH_TILES) {
                for wall_row in (row - CRUSH_TILES)..=(row + CRUSH_TILES) {
                    if let Some(wall) = walls.get(&(wall_col, wall_row)).filter(|wall| wall.blocks(Mover::Enemy) && wall.side == Side::Friendly) {
                        if wall.rect().overlaps(&body) {
                            fx.wall_hits.push(((wall_col, wall_row), enemy.attack_now() * ENEMY_CRUSH_SHARE * dt));
                        }
                    }
                }
            }
        }
    }
}

//the same for one player, whose army is `units`: what the tests that have just the one player use
#[cfg(test)]
pub(crate) fn enemy_abilities(enemies: &mut Vec<Enemy>, units: &[Unit], player: Vec2, dt: f32, fx: &mut Effects) {
    let field = FlowField::new();
    let prey = [Prey { position: player, units, team: units.len(), pressure: units.len(), boost: 1.0, cap: DEFAULT_ARMY_LIMIT, field: &field }];
    enemy_abilities_among(enemies, &prey, dt, fx);
}

//the things enemies do on a timer: shields grow back, a bomber's fuse burns down, a healer mends the enemies near it, a
//summoner calls up imps while a player is close, a mortar lobs a shell at the army of the player nearest to it. what depends on how
//big an army is (the blasts, the broods of a nest) is for the army of the player nearest to the enemy
pub(crate) fn enemy_abilities_among(enemies: &mut Vec<Enemy>, prey: &[Prey], dt: f32, fx: &mut Effects) {
    let mut owned: HashMap<u32, usize> = HashMap::new();
    for owner in enemies.iter().filter_map(|enemy| enemy.owner) {
        *owned.entry(owner).or_default() += 1;
    }
    let mut born: Vec<Enemy> = Vec::new();
    let mut pulses: Vec<Vec2> = Vec::new();
    for enemy in enemies.iter_mut() {
        let hunted = nearest(prey, enemy.position);
        let (player, units, army) = (hunted.position, hunted.units, hunted.army());
        enemy.flash = (enemy.flash + dt).min(10.0);
        if enemy.max_shield > 0.0 {
            enemy.shield_idle += dt;
            if enemy.shield_idle >= SHIELD_REGEN_DELAY && enemy.shield < enemy.max_shield {
                enemy.shield = (enemy.shield + enemy.max_shield * SHIELD_REGEN_RATE * dt).min(enemy.max_shield);
            }
        }
        if enemy.frozen > 0.0 || enemy.routed() {
            continue;
        }
        match enemy.behavior {
            Behavior::Bomber => {
                if let Mode::Fuse { left } = enemy.mode {
                    if left - dt > 0.0 {
                        enemy.mode = Mode::Fuse { left: left - dt };
                    } else {
                        //it goes off, and is gone
                        let radius = BLAST_RADIUS * blast_scale(enemy.scale) + enemy.size / 2.0;
                        fx.hazards.push(Hazard::new(enemy.position, radius, 0.0, UNIT_HEALTH, victims_of(army, 0.007), enemy.color, Cue::Explosion).vs_walls(enemy.scale));
                        enemy.health = 0.0;
                        enemy.blew_up = true;
                    }
                }
            }
            Behavior::Healer => {
                enemy.timer -= dt;
                if enemy.timer <= 0.0 {
                    enemy.timer = HEAL_INTERVAL;
                    enemy.flash = 0.0;
                    pulses.push(enemy.position);
                }
            }
            Behavior::Summoner => {
                enemy.timer -= dt;
                if enemy.timer <= 0.0 {
                    //(it calls for the player, or for whoever is hurting it)
                    if enemy.provoked <= 0.0 && player.distance(enemy.position) > SUMMON_RANGE_TILES * TILE_SIZE + enemy.size / 2.0 {
                        //nobody to call them for, look again soon
                        enemy.timer = 0.5;
                        continue;
                    }
                    enemy.timer = SUMMON_INTERVAL * rand::gen_range(0.85, 1.15);
                    let alive = owned.get(&enemy.id).copied().unwrap_or(0);
                    let count = SUMMON_COUNT.min(MAX_MINIONS.saturating_sub(alive));
                    for _ in 0..count {
                        let around = enemy.position + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * (enemy.size / 2.0 + 20.0);
                        born.push(Enemy::minion(around, kind_named("Imp"), enemy.scale, enemy.id));
                    }
                    if count > 0 {
                        enemy.flash = 0.0;
                        fx.burst(enemy.position, enemy.color, 14, 220.0);
                        fx.cue_at(Cue::Summon, enemy.position);
                    }
                }
            }
            Behavior::Mortar => {
                enemy.timer -= dt;
                if enemy.timer <= 0.0 {
                    //the first unit in range, from a random place in the list so it isn't always the same one. one that units are
                    //hurting lobs its shells at them
                    let start = if units.is_empty() { 0 } else { rand::gen_range(0, units.len()) };
                    let target = if enemy.provoked > 0.0 && enemy.grudge.distance(enemy.position) <= enemy.range {
                        Some(enemy.grudge)
                    } else {
                        (0..units.len())
                            .map(|k| &units[(start + k) % units.len()])
                            .map(|unit| Vec2::new(unit.x, unit.y))
                            .find(|at| at.distance(enemy.position) <= enemy.range)
                    };
                    let Some(target) = target else {
                        enemy.timer = 0.4;
                        continue;
                    };
                    enemy.timer = MORTAR_INTERVAL * rand::gen_range(0.8, 1.2);
                    enemy.flash = 0.0;
                    let radius = BLAST_RADIUS * blast_scale(enemy.scale);
                    fx.hazards.push(Hazard::new(target, radius, SHELL_WARN, UNIT_HEALTH, victims_of(army, 0.01), enemy.color, Cue::Explosion).vs_walls(enemy.scale));
                    fx.cue_at(Cue::Shoot, enemy.position);
                }
            }
            Behavior::Nest => {
                enemy.timer -= dt;
                if enemy.timer <= 0.0 {
                    let Some(def) = nest_named(enemy.name) else {
                        enemy.timer = 5.0;
                        continue;
                    };
                    if player.distance(enemy.position) > NEST_ACTIVE_TILES * TILE_SIZE {
                        enemy.timer = 0.5;
                        continue;
                    }
                    enemy.timer = def.every * rand::gen_range(0.85, 1.15);
                    let alive = owned.get(&enemy.id).copied().unwrap_or(0);
                    let cap = (def.cap + army / 40).min(NEST_MAX_BROOD);
                    let count = (def.brood + army / 100).min(cap.saturating_sub(alive));
                    for _ in 0..count {
                        let around = enemy.position + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * (enemy.size / 2.0 + 24.0);
                        born.push(Enemy::minion(around, kind_named(def.child), enemy.scale, enemy.id));
                    }
                    if count > 0 {
                        enemy.flash = 0.0;
                        fx.burst(enemy.position, enemy.color, 10, 200.0);
                        fx.cue_at(Cue::Summon, enemy.position);
                    }
                }
            }
            _ => {}
        }
    }
    for at in pulses {
        let mut healed = 0;
        for enemy in enemies.iter_mut().filter(|enemy| enemy.boss.is_none() && enemy.health > 0.0 && enemy.health < enemy.max_health && enemy.position.distance(at) <= HEAL_RANGE) {
            enemy.health = (enemy.health + enemy.max_health * HEAL_SHARE).min(enemy.max_health);
            fx.burst(enemy.position, GREEN, 4, 120.0);
            healed += 1;
        }
        if healed > 0 {
            fx.cue_at(Cue::Heal, at);
        }
    }
    enemies.extend(born);
}


//the same for one player: what the tests that have just the one player use
#[cfg(test)]
pub(crate) fn spring_ambushes(enemies: &mut [Enemy], player: Vec2, fx: &mut Effects) -> usize {
    let field = FlowField::new();
    let prey = [Prey { position: player, units: &[], team: 0, pressure: 0, boost: 1.0, cap: DEFAULT_ARMY_LIMIT, field: &field }];
    spring_ambushes_among(enemies, &prey, fx)
}

//ambushers lying in wait spring when a player comes close, and the ones beside them spring too (so do the ones beside an ambusher
//that has only just sprung, whatever set it off: a unit that stumbled on it). a spring is a rush at the player nearest to it, and the
//one popup and sound that says so. returns how many sprang
pub(crate) fn spring_ambushes_among(enemies: &mut [Enemy], prey: &[Prey], fx: &mut Effects) -> usize {
    if !enemies.iter().any(|enemy| enemy.behavior == Behavior::Ambusher) {
        return 0;
    }
    //where the ones that have just sprung are
    let mut sprung: Vec<Vec2> = enemies.iter().filter(|enemy| enemy.behavior == Behavior::Ambusher && !enemy.lurking() && enemy.flash < 0.5).map(|enemy| enemy.position).collect();
    let trigger = AMBUSH_TRIGGER_TILES * TILE_SIZE;
    let together = AMBUSH_ALARM_TILES * TILE_SIZE;
    let mut count = 0;
    for enemy in enemies.iter_mut().filter(|enemy| enemy.lurking()) {
        let player = nearest(prey, enemy.position).position;
        if player.distance(enemy.position) <= trigger + enemy.size / 2.0 {
            enemy.spring(player, fx);
            sprung.push(enemy.position);
            count += 1;
        }
    }
    //the rest of a pack goes with the first of it
    for enemy in enemies.iter_mut().filter(|enemy| enemy.lurking()) {
        let player = nearest(prey, enemy.position).position;
        if sprung.iter().any(|at| at.distance_squared(enemy.position) <= together * together) {
            enemy.spring(player, fx);
            count += 1;
        }
    }
    if count > 0 {
        fx.add_shake(0.15);
    }
    count
}

//enemies that were hurt by units come to fight them, and the enemies near them that haven't been hurt come too: each alarm is where
//the one that was hurt is, and where the units that hurt it are. returns how many came
pub(crate) fn call_for_help(enemies: &mut [Enemy], alarms: Vec<(Vec2, Vec2)>, fx: &mut Effects) -> usize {
    let reach = RETALIATE_ALARM_TILES * TILE_SIZE;
    let orange = Color::new(1.0, 0.62, 0.15, 1.0);
    let mut called = 0;
    for (from, at) in alarms.into_iter().take(RETALIATE_MAX_ALARMS) {
        let before = called;
        for enemy in enemies.iter_mut().filter(|enemy| enemy.provoked <= 0.0 && !enemy.chasing && enemy.can_be_provoked() && enemy.position.distance_squared(from) <= reach * reach) {
            enemy.provoke(at);
            called += 1;
            if called <= 6 {
                fx.popup(enemy.position, enemy.size / 2.0, 0.0, "!".to_owned(), orange);
            }
        }
        if called > before {
            fx.ring(from, reach, orange);
            fx.cue_at(Cue::Alert, from);
        }
    }
    called
}
