//! fighting: damage by touch, deaths, shots, hazards and the babies enemies drop

use crate::prelude::*;

//every unit inside an enemy hurts it, and it hurts the units inside it: its attack, a number of damage per second, is
//spent on one unit after another until it is used up, so units die one at a time rather than all being worn down. a
//bomber that is touched lights its fuse. an enemy that runs out of health is destroyed (see resolve_deaths). the units that
//run out of health are left for bury_units. returns how many enemies were destroyed
pub(crate) fn fight(enemies: &mut Vec<Enemy>, units: &mut [Unit], babies: &mut Vec<Baby>, dt: f32, upgrades: &Upgrades, fx: &mut Effects) -> usize {
    let grid = UnitGrid::new(units);
    let mut touching: Vec<usize> = Vec::new();
    for enemy in enemies.iter_mut() {
        touching.clear();
        let reach = Vec2::splat(enemy.size / 2.0 + RADIUS * MAX_CHAMPION_SIZE);
        grid.near(enemy.position - reach, enemy.position + reach, &mut touching);
        touching.retain(|&i| {
            let unit = &units[i];
            unit.health > 0.0 && unit.garrison.is_none() && enemy.overlaps_circle(unit.x, unit.y, unit.radius())
        });
        //a champion counts as many units, and a boss with an armoured front takes little from the units in front of it
        let strength: f32 = if enemy.boss.as_ref().is_some_and(|state| state.def.flank) {
            touching.iter().map(|&i| flank_factor(enemy, Vec2::new(units[i].x, units[i].y)) * units[i].might).sum()
        } else {
            touching.iter().map(|&i| units[i].might).sum()
        };
        let dealt = enemy.take_damage(strength * UNIT_DAMAGE * upgrades.damage * dt);
        enemy.hurt += dealt;
        //a Freezing relic: an enemy being fought now and then freezes solid
        enemy.frozen = (enemy.frozen - dt).max(0.0);
        if upgrades.freeze > 0.0 && enemy.frozen <= 0.0 && enemy.can_freeze() && !touching.is_empty() && rand::gen_range(0.0, 1.0) < upgrades.freeze_chance(dt) {
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
        //the grid finds them in no order, and the first ones in the army are the first to be hit. an attack only gets through so many,
        //so they are put in order a batch at a time (the first few of thousands, then more if it is not used up) rather than all of them
        let mut done = 0;
        let mut batch = (budget / UNIT_HEALTH).ceil() as usize + 16;
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
                let hit = budget.min((unit.health + unit.guard) / (upgrades.toughness * unit.ward));
                unit.hurt(hit * upgrades.toughness);
                budget -= hit;
                if unit.health <= 0.0 {
                    enemy.killed += 1;
                }
            }
            done = end;
            batch *= 4;
        }
        //thorns: it takes back some of what it did
        if upgrades.thorns > 0.0 && budget < spent_from {
            enemy.hurt += enemy.take_damage((spent_from - budget) * upgrades.thorns);
        }
    }
    resolve_deaths(enemies, babies, units.len(), upgrades, fx)
}

//remove every enemy that has run out of health. one that was made by another is just gone, a bomber that set itself off
//too, anything else drops its baby units (more with upgrades.bounty) and counts as destroyed. a splitter breaks into
//smaller ones, a frozen one shatters, a volatile one goes off, and whatever an enemy made dies with it. returns how many were destroyed
pub(crate) fn resolve_deaths(enemies: &mut Vec<Enemy>, babies: &mut Vec<Baby>, army: usize, upgrades: &Upgrades, fx: &mut Effects) -> usize {
    let mut destroyed = 0;
    let mut gone: Vec<u32> = Vec::new();
    let mut born: Vec<Enemy> = Vec::new();
    enemies.retain_mut(|enemy| {
        if enemy.health > 0.0 {
            return true;
        }
        enemy.show_numbers(fx);
        gone.push(enemy.id);
        if enemy.blew_up {
            fx.burst(enemy.position, enemy.color, 14, 260.0);
            return false;
        }
        if enemy.is_minion() {
            fx.burst(enemy.position, enemy.color, (enemy.size / 4.0).clamp(6.0, 40.0) as usize, 220.0);
            fx.cue_at(Cue::Death, enemy.position);
            return false;
        }
        drop_babies(enemy, babies, army, upgrades.bounty, upgrades.unit_cap());
        fx.points += kill_points(enemy);
        fx.xp += kill_xp(enemy);
        fx.heal_all += upgrades.bloodlust * if enemy.boss.is_some() { 10.0 } else { 1.0 };
        let boss = enemy.boss.is_some();
        fx.add_shake(if boss { 1.0 } else { (enemy.size / 600.0).clamp(0.06, 0.7) });
        fx.burst(enemy.position, enemy.color, (enemy.size / 4.0).clamp(14.0, if boss { 400.0 } else { 160.0 }) as usize, 320.0);
        fx.burst(enemy.position, GREEN, enemy.babies.min(40), 200.0);
        fx.debris(enemy.position, enemy.color, enemy.size);
        fx.mark(enemy.position, (enemy.size * 0.7).clamp(22.0, 300.0), enemy.color, MarkKind::Splat);
        fx.cue_at(Cue::Thud, enemy.position);
        if enemy.modifier != Modifier::None && rand::gen_range(0.0, 1.0) < CHAMPION_ELITE_CHANCE {
            fx.recruits.push((enemy.position, Odds::Elite));
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

//something an enemy that shoots has fired, which flies straight until it hits a unit or a wall, or gives up
pub(crate) struct Shot {
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2, //pixels per second
    pub(crate) age: f32,       //seconds
    pub(crate) damage: f32,
    pub(crate) radius: f32,
    pub(crate) color: Color,
}

impl Shot {
    pub(crate) fn new(position: Vec2, velocity: Vec2) -> Shot {
        Shot { position, velocity, age: 0.0, damage: SHOT_DAMAGE, radius: SHOT_RADIUS, color: ORANGE }
    }

    //the bigger, harder hitting kind a boss fires
    pub(crate) fn heavy(position: Vec2, velocity: Vec2, color: Color) -> Shot {
        Shot { position, velocity, age: 0.0, damage: SHOT_DAMAGE * 1.2, radius: SHOT_RADIUS * 1.6, color }
    }
}

//an enemy that shoots fires a volley at the nearest unit within its range whenever it is ready
pub(crate) fn shoot_enemies(enemies: &mut [Enemy], units: &[Unit], shots: &mut Vec<Shot>, dt: f32, fx: &mut Effects) {
    for enemy in enemies.iter_mut().filter(|enemy| enemy.behavior == Behavior::Shooter && enemy.range > 0.0 && enemy.frozen <= 0.0) {
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
        enemy.shot_in = SHOT_INTERVAL * rand::gen_range(0.8, 1.2);
        let aim = (target - enemy.position).normalize_or_zero();
        let first = -(enemy.volley as f32 - 1.0) / 2.0;
        for k in 0..enemy.volley {
            let direction = aim.rotate(Vec2::from_angle((first + k as f32) * SHOT_SPREAD));
            shots.push(Shot::new(enemy.position + direction * enemy.size / 2.0, direction * SHOT_SPEED));
        }
        fx.cue_at(Cue::Shoot, enemy.position);
    }
}

//fly the shots. one that touches a unit hurts it and is used up, and so is one that hits a wall or has flown too long.
//the units are sorted into a grid first, there can be a lot of shots and a lot of units
pub(crate) fn update_shots(shots: &mut Vec<Shot>, units: &mut [Unit], walls: &Walls, dt: f32, upgrades: &Upgrades, fx: &mut Effects) {
    if shots.is_empty() {
        return;
    }
    const CELL: f32 = 64.0;
    let mut grid: TileMap<Vec<usize>> = TileMap::default();
    for (i, unit) in units.iter().enumerate() {
        grid.entry(((unit.x / CELL) as i32, (unit.y / CELL) as i32)).or_default().push(i);
    }
    shots.retain_mut(|shot| {
        shot.age += dt;
        if shot.age > SHOT_LIFE {
            return false;
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
                fx.wall_hits.push(((col, row), shot.damage * SHOT_WALL_SHARE));
                fx.burst(shot.position, shot.color, 5, 120.0);
                return false;
            }
            let cell = ((shot.position.x / CELL) as i32, (shot.position.y / CELL) as i32);
            for cell_x in cell.0 - 1..=cell.0 + 1 {
                for cell_y in cell.1 - 1..=cell.1 + 1 {
                    let Some(in_cell) = grid.get(&(cell_x, cell_y)) else {
                        continue;
                    };
                    for &i in in_cell {
                        let unit = &mut units[i];
                        if unit.health > 0.0 && unit.garrison.is_none() && Vec2::new(unit.x, unit.y).distance(shot.position) < reach {
                            unit.hurt(shot.damage * upgrades.toughness);
                            fx.burst(shot.position, shot.color, 6, 160.0);
                            return false;
                        }
                    }
                }
            }
        }
        true
    });
}

//a shot is a small bright circle with a glow round it
pub(crate) fn draw_shots(shots: &[Shot]) {
    for shot in shots {
        draw_circle(shot.position.x, shot.position.y, shot.radius * 2.2, Color::new(shot.color.r, shot.color.g * 0.8, shot.color.b, 0.25));
        draw_circle(shot.position.x, shot.position.y, shot.radius, shot.color);
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
}

impl Hazard {
    pub(crate) fn new(position: Vec2, radius: f32, warn: f32, damage: f32, victims: usize, color: Color, boom: Cue) -> Hazard {
        Hazard { position, radius, armed: true, warn, warn_total: warn.max(0.001), damage, victims, linger: 0.0, burn: 0.0, flash: 0.0, color, boom, wall_damage: damage * BLAST_WALL_FACTOR }
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
}

//count the hazards down, set off the ones that are due and burn the units in the ones that are burning
pub(crate) fn update_hazards(hazards: &mut Vec<Hazard>, units: &mut [Unit], upgrades: &Upgrades, dt: f32, fx: &mut Effects) {
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
                        unit.hurt(hazard.damage * upgrades.toughness);
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
                unit.hurt(worst * upgrades.toughness * dt);
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
            draw_poly(x, y, sides, radius, 0.0, Color::new(color.r, color.g, color.b, 0.10 + 0.06 * pulse));
            draw_poly(x, y, sides, radius * filled, 0.0, Color::new(color.r, color.g, color.b, 0.22));
            draw_poly_lines(x, y, sides, radius, 0.0, (radius * 0.02).clamp(3.0, 10.0), danger_color(0.55 + 0.45 * pulse));
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
