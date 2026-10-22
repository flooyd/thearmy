//! what the player's walls do besides stand there: towers that units man and shoot from, spikes that hurt and tar that slows

use crate::prelude::*;

//let the units that were inside a tower out round it, the tower having been destroyed or deleted
pub(crate) fn release_garrison(units: &mut [Unit], tile: (i32, i32), at: Vec2) {
    for unit in units.iter_mut().filter(|unit| unit.garrison == Some(tile)) {
        let out = at + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(TILE_SIZE * 0.6, TILE_SIZE * 1.2);
        unit.garrison = None;
        unit.x = out.x;
        unit.y = out.y;
        unit.steer_in = 0.0;
    }
}

//the key G on a tower: if it is manned the units come out, otherwise the free units closest to it, within
//TOWER_REACH_TILES, go inside up to TOWER_CAPACITY. returns how many went in or came out
pub(crate) fn man_tower(walls: &mut Walls, units: &mut [Unit], tile: (i32, i32), fx: &mut Effects) -> usize {
    let Some(wall) = walls.get_mut(&tile).filter(|wall| wall.kind == WallKind::Tower && wall.side == Side::Friendly) else {
        return 0;
    };
    let at = wall.center();
    if wall.garrison > 0 {
        let out = wall.garrison;
        wall.garrison = 0;
        release_garrison(units, tile, at);
        fx.popup(at, TILE_SIZE, 0.0, format!("{out} out"), SQUAD_COLOR);
        fx.cue(Cue::Dispatch);
        return out;
    }
    let reach = TOWER_REACH_TILES * TILE_SIZE;
    let mut near: Vec<(f32, usize)> = units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.garrison.is_none() && unit.target.is_none() && unit.champion == 0)
        .map(|(i, unit)| (Vec2::new(unit.x, unit.y).distance(at), i))
        .filter(|&(distance, _)| distance <= reach)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.truncate(TOWER_CAPACITY);
    for &(_, i) in &near {
        units[i].garrison = Some(tile);
        units[i].x = at.x;
        units[i].y = at.y;
    }
    wall.garrison = near.len();
    if near.is_empty() {
        fx.popup(at, TILE_SIZE, 0.0, "no units close enough".to_owned(), Color::new(1.0, 0.55, 0.4, 1.0));
    } else {
        fx.popup(at, TILE_SIZE, 0.0, format!("{} inside", near.len()), SQUAD_COLOR);
        fx.burst(at, SQUAD_COLOR, 10, 160.0);
        fx.cue(Cue::Dispatch);
    }
    near.len()
}

//every manned tower shoots the nearest enemy in range when its timer is up, for what the units inside would do to it in
//that time at half their usual rate, and shows a streak from the tower to it
pub(crate) fn fire_towers(walls: &mut Walls, enemies: &mut [Enemy], upgrades: &Upgrades, dt: f32, fx: &mut Effects) {
    for wall in walls.values_mut().filter(|wall| wall.kind == WallKind::Tower && wall.garrison > 0) {
        wall.fire_in -= dt;
        if wall.fire_in > 0.0 {
            continue;
        }
        let at = wall.center();
        let target = enemies
            .iter_mut()
            .filter(|enemy| enemy.health > 0.0 && !enemy.lurking() && enemy.position.distance(at) <= TOWER_RANGE + enemy.size / 2.0)
            .min_by(|a, b| a.position.distance_squared(at).total_cmp(&b.position.distance_squared(at)));
        let Some(enemy) = target else {
            //nothing to shoot at, look again soon
            wall.fire_in = 0.2;
            continue;
        };
        wall.fire_in = TOWER_INTERVAL;
        let damage = wall.garrison as f32 * TOWER_DAMAGE * TOWER_INTERVAL * upgrades.hit() * upgrades.archery;
        let dealt = enemy.take_damage(damage);
        enemy.hurt += dealt;
        fx.beams.push(Beam { from: at, to: enemy.position, color: SQUAD_COLOR, age: 0.0 });
        fx.cue_at(Cue::Arrow, at);
    }
}

//what the floor does to the enemies standing on it, by the square their middle is on: spikes hurt them and wear down, tar
//slows them (`slow` is the share of their speed they keep)
pub(crate) fn terrain_effects(enemies: &mut [Enemy], walls: &Walls, dt: f32, fx: &mut Effects) {
    for enemy in enemies.iter_mut() {
        enemy.slow = 1.0;
        let tile = tile_of(enemy.position.x, enemy.position.y);
        let Some(wall) = walls.get(&tile).filter(|wall| wall.side == Side::Friendly) else {
            continue;
        };
        match wall.kind {
            WallKind::Tar => enemy.slow = TAR_SLOW,
            WallKind::Spikes => {
                let share = SPIKE_DAMAGE_SHARE * if enemy.boss.is_some() { SPIKE_BOSS_FACTOR } else { 1.0 };
                let dealt = enemy.take_damage(enemy.max_health * share * dt);
                enemy.hurt += dealt;
                fx.wall_hits.push((tile, SPIKE_WEAR * dt));
            }
            _ => {}
        }
    }
}

//hostile walls that units are pressed against wear down: every unit touching one does UNIT_CHEW_SHARE of its damage to it
pub(crate) fn chew_hostile_walls(units: &[Unit], walls: &Walls, upgrades: &Upgrades, dt: f32, fx: &mut Effects) {
    if !walls.values().any(|wall| wall.side == Side::Hostile) {
        return;
    }
    let each = UNIT_DAMAGE * UNIT_CHEW_SHARE * upgrades.damage * dt;
    let mut taken: HashMap<(i32, i32), f32> = HashMap::new();
    for unit in units.iter().filter(|unit| unit.garrison.is_none()) {
        let (col, row) = tile_of(unit.x, unit.y);
        for wall_col in (col - 1)..=(col + 1) {
            for wall_row in (row - 1)..=(row + 1) {
                let Some(wall) = walls.get(&(wall_col, wall_row)).filter(|wall| wall.side == Side::Hostile && wall.blocks(Mover::Ally)) else {
                    continue;
                };
                //a unit resting against a wall is exactly RADIUS away, so the reach is a little more
                if wall.overlaps_circle(unit.x, unit.y, RADIUS + 3.0) {
                    *taken.entry((wall_col, wall_row)).or_default() += each;
                }
            }
        }
    }
    fx.wall_hits.extend(taken);
}

//a streak of light for a moment where something was shot
pub(crate) struct Beam {
    pub(crate) from: Vec2,
    pub(crate) to: Vec2,
    pub(crate) color: Color,
    pub(crate) age: f32, //seconds
}

pub(crate) const BEAM_LIFE: f32 = 0.18; //seconds a streak lasts

pub(crate) fn draw_beams(beams: &[Beam]) {
    for beam in beams {
        let fade = 1.0 - beam.age / BEAM_LIFE;
        draw_line(beam.from.x, beam.from.y, beam.to.x, beam.to.y, 6.0 * fade + 1.0, Color::new(beam.color.r, beam.color.g, beam.color.b, 0.8 * fade));
    }
}
