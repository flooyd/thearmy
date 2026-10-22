//how the army fares walking through the wilds: a whole game's frames (without the sites and the bosses) with a player who wanders
//and fights whatever they meet, to see whether a big army wins or is worn away. the table is only run by hand:
//cargo test --release how_the_army_fares_in_the_wilds -- --ignored --nocapture

use super::boss_fights::late_run_upgrades;
use super::*;
use std::time::Instant;

pub(super) struct Wander {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) lost: usize,
    pub(super) destroyed: usize,
    pub(super) picked_up: usize,
    pub(super) lowest: usize,
    pub(super) lost_to_touch: usize,
    pub(super) lost_to_shots: usize,
    pub(super) lost_to_blasts: usize,
    pub(super) ms_a_frame: f32,
    pub(super) by_kind: HashMap<&'static str, usize>, //units killed by touch, by the kind of enemy
}

//an army of `army` units wanders for `seconds` of game time. it is settled round the player first, then the player goes in a big
//slow circle, so there are always new enemies ahead. `pressure` is how big an army the enemies are made for, as a share of the army
pub(super) fn wander(army: usize, formation: Formation, seconds: f32, pressure: f32, cap: usize, seed: u64) -> Wander {
    wander_with(Upgrades::default(), army, formation, seconds, pressure, cap, seed)
}

//the same for an army with relics
pub(super) fn wander_with(upgrades: Upgrades, army: usize, formation: Formation, seconds: f32, pressure: f32, cap: usize, seed: u64) -> Wander {
    rand::srand(seed);
    let walls = Walls::default();
    let mut player = Vec2::splat(WORLD_SIZE / 2.0);
    let mut units: Vec<Unit> = (0..army)
        .map(|_| {
            let at = random_position_around(player.x, player.y, 60.0, 700.0);
            Unit::new(at.x, at.y, 400.0)
        })
        .collect();
    let mut enemies = Vec::new();
    let mut babies = Vec::new();
    let mut shots = Vec::new();
    let mut hazards: Vec<Hazard> = Vec::new();
    let squads = Squads::new();
    let mut fx = Effects::default();
    let mut upgrades = upgrades;
    upgrades.army_limit = cap;
    let dead = |units: &[Unit]| units.iter().filter(|unit| unit.health <= 0.0).count();
    let mut routes = Routes::new(player, &walls);
    let dt = 1.0 / 60.0;
    let mut facing = Vec2::X;
    let mut velocity = Vec2::ZERO;
    let mut out = Wander { start: army, end: army, lost: 0, destroyed: 0, picked_up: 0, lowest: army, lost_to_touch: 0, lost_to_shots: 0, lost_to_blasts: 0, ms_a_frame: 0.0, by_kind: HashMap::new() };
    let pressure_for = |units: usize| ((units as f32 * pressure) as usize).max(1);
    //the army gets into its shape before there is anyone to fight
    for _ in 0..(8.0 / dt) as usize {
        let drill = Drill { formation, facing };
        let boost = speed_multiplier(units.len());
        update_units_in(&mut units, player.x, player.y, dt, &walls, &routes.army, boost, &squads, &drill);
    }
    respawn_enemies(&mut enemies, player.x, player.y, pressure_for(army));
    let frames = (seconds / dt) as usize;
    let started = Instant::now();
    for frame in 0..frames {
        let boost = speed_multiplier(units.len());
        let before = player;
        let angle = frame as f32 * dt * 0.05;
        player += Vec2::new(angle.cos(), angle.sin()) * 600.0 * dt;
        player = player.clamp(Vec2::splat(RADIUS), Vec2::splat(WORLD_SIZE - RADIUS));
        velocity = velocity * 0.9 + (player - before) / dt * 0.1;
        facing = turn_to_face(facing, velocity, dt);
        let drill = Drill { formation, facing };
        routes.update(player, &walls, false);
        update_units_in(&mut units, player.x, player.y, dt, &walls, &routes.army, boost, &squads, &drill);
        terrain_effects(&mut enemies, &walls, dt, &mut fx);
        patrol_enemies(&mut enemies, player, dt, boost, &walls, &routes.foes, &mut fx);
        enemy_abilities(&mut enemies, &units, player, dt, &mut fx);
        shoot_enemies(&mut enemies, &units, &mut shots, dt, &mut fx);
        hazards.append(&mut fx.hazards);
        let (at_first, mut then) = (dead(&units), 0);
        update_shots(&mut shots, &mut units, &walls, dt, &upgrades, &mut fx);
        then = dead(&units).max(then);
        out.lost_to_shots += then - at_first;
        update_hazards(&mut hazards, &mut units, &upgrades, dt, &mut fx);
        out.lost_to_blasts += dead(&units) - then;
        heal_units(&mut units, dt, &upgrades);
        let before_fight = dead(&units);
        let killed_before: HashMap<u32, u32> = enemies.iter().map(|enemy| (enemy.id, enemy.killed as u32)).collect();
        out.destroyed += fight(&mut enemies, &mut units, &mut babies, dt, &upgrades, &mut fx);
        out.lost_to_touch += dead(&units) - before_fight;
        for enemy in &enemies {
            let more = enemy.killed as u32 - killed_before.get(&enemy.id).copied().unwrap_or(enemy.killed as u32);
            if more > 0 {
                *out.by_kind.entry(enemy.name).or_default() += more as usize;
            }
        }
        hazards.append(&mut fx.hazards);
        tick_enemy_numbers(&mut enemies, dt, &mut fx);
        out.lost += bury_units(&mut units, &mut fx);
        fx.fallen.clear();
        fx.heal_all = 0.0;
        fx.points = 0.0;
        fx.xp = 0.0;
        respawn_enemies(&mut enemies, player.x, player.y, pressure_for(units.len()));
        let before_pickup = units.len();
        let range = baby_pickup_range(units.len(), Vec2::new(1920.0, 1080.0));
        grow_babies(&mut babies, &mut units, player.x, player.y, dt, range, upgrades.unit_cap(), &mut fx);
        out.picked_up += units.len() - before_pickup.min(units.len());
        fx.update(dt, player);
        fx.cues.clear();
        fx.recruits.clear();
        out.lowest = out.lowest.min(units.len());
        if units.is_empty() {
            break;
        }
    }
    out.end = units.len();
    out.ms_a_frame = started.elapsed().as_secs_f32() * 1000.0 / frames as f32;
    out
}

#[test]
#[ignore]
fn how_the_army_fares_in_the_wilds() {
    let armies: Vec<usize> = std::env::var("WILDS_ARMIES").map(|list| list.split(',').filter_map(|n| n.trim().parse().ok()).collect()).unwrap_or_else(|_| vec![1000, 4000, 8000, 13000, 20000]);
    let seeds = 3u64;
    for army in armies {
        for formation in wilds_formations() {
            let runs: Vec<Wander> = (0..seeds).map(|seed| wander(army, formation, 60.0, 1.0, 30000, 21 + seed)).collect();
            let mean = |f: &dyn Fn(&Wander) -> f32| runs.iter().map(f).sum::<f32>() / seeds as f32;
            println!(
                "{army:6} units {:6}: {:+6.1}% (runs {}), lost {:5.0} = touch {:5.0} + shots {:4.0} + blasts {:5.0}, {:4.0} enemies destroyed, {:5.0} picked up, {:.1} ms a frame",
                formation.name(),
                mean(&|w| (w.end as f32 / w.start as f32 - 1.0) * 100.0),
                runs.iter().map(|w| format!("{:+.0}%", (w.end as f32 / w.start as f32 - 1.0) * 100.0)).collect::<Vec<_>>().join(" "),
                mean(&|w| w.lost as f32),
                mean(&|w| w.lost_to_touch as f32),
                mean(&|w| w.lost_to_shots as f32),
                mean(&|w| w.lost_to_blasts as f32),
                mean(&|w| w.destroyed as f32),
                mean(&|w| w.picked_up as f32),
                mean(&|w| w.ms_a_frame)
            );
            let mut kinds: HashMap<&str, usize> = HashMap::new();
            for run in &runs {
                for (name, count) in &run.by_kind {
                    *kinds.entry(name).or_default() += count;
                }
            }
            let mut kinds: Vec<_> = kinds.into_iter().collect();
            kinds.sort_by_key(|(_, count)| Reverse(*count));
            println!("         killed by: {}", kinds.iter().take(7).map(|(name, count)| format!("{name} {}", count / seeds as usize)).collect::<Vec<_>>().join(", "));
        }
    }
}

//the formations to try (WILDS_FORMATIONS=Ring,Line for some of them)
fn wilds_formations() -> Vec<Formation> {
    match std::env::var("WILDS_FORMATIONS") {
        Ok(list) => list.split(',').filter_map(|name| Formation::from_name(name.trim())).collect(),
        Err(_) => FORMATIONS.to_vec(),
    }
}

//a late run (the relics of one, and 35 bosses slain) against each difficulty. WILDS_ARMIES picks the armies
#[test]
#[ignore]
fn the_wilds_for_a_late_run() {
    let armies: Vec<usize> = std::env::var("WILDS_ARMIES").map(|list| list.split(',').filter_map(|n| n.trim().parse().ok()).collect()).unwrap_or_else(|_| vec![5000, 20000]);
    for army in armies {
        for difficulty in DIFFICULTIES {
            let mut upgrades = late_run_upgrades();
            upgrades.foe_answer = difficulty.relic_answer();
            let share = (1.0 + ENEMY_ROUND_GROWTH * 35.0) * difficulty.pressure();
            let run = wander_with(upgrades.clone(), army, Formation::Ring, 60.0, share, 50000, 31);
            println!(
                "{army:6} units {:<7} (enemies for an army x{share:>5.1}, damage x{:>5.1}): {:+6.1}%, lost {:6} = touch {:6} + shots {:5} + blasts {:5}, {:4} enemies destroyed, {:5} picked up, lowest {}",
                difficulty.name(),
                upgrades.hit(),
                (run.end as f32 / run.start as f32 - 1.0) * 100.0,
                run.lost,
                run.lost_to_touch,
                run.lost_to_shots,
                run.lost_to_blasts,
                run.destroyed,
                run.picked_up,
                run.lowest
            );
        }
    }
}
