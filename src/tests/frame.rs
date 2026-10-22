//how long a whole frame takes with a big army, a world full of enemies and sites, and the player on the move. only run by hand:
//cargo test --release where_a_whole_frame_goes -- --ignored --nocapture

use super::*;
use std::time::Instant;

macro_rules! timed {
    ($total:expr, $body:expr) => {{
        let started = Instant::now();
        let result = $body;
        $total += started.elapsed().as_secs_f64();
        result
    }};
}

#[test]
#[ignore]
fn where_a_whole_frame_goes() {
    let formation = std::env::var("FRAME_FORMATION").ok().and_then(|name| Formation::from_name(&name)).unwrap_or(Formation::Free);
    for army in [300usize, 1500, 4000, 13000, 20000] {
        rand::srand(5);
        let mut walls = starting_house();
        let mut sites = generate_sites();
        let mut player = Vec2::splat(WORLD_SIZE / 2.0);
        let mut units: Vec<Unit> = (0..army)
            .map(|_| {
                let at = random_position_around(player.x, player.y, 60.0, 700.0);
                Unit::new(at.x, at.y, 400.0)
            })
            .collect();
        let mut enemies = Vec::new();
        respawn_enemies(&mut enemies, player.x, player.y, units.len());
        let mut babies = Vec::new();
        let mut shots = Vec::new();
        let mut hazards: Vec<Hazard> = Vec::new();
        let mut squads = Squads::new();
        let mut fx = Effects::default();
        let upgrades = Upgrades::default();
        let mut routes = Routes::new(player, &walls);
        let dt = 1.0 / 120.0;
        let boost = speed_multiplier(units.len());
        let mut facing = Vec2::X;
        let mut particles = Vec::new();
        //[routes, army, enemies, combat, sites, the rest, what is made ready to draw]
        let mut totals = [0.0f64; 7];
        let frames = 600;
        for frame in 0..frames {
            //the player walks a circle, so the routes have to be made again every few frames
            let angle = frame as f32 * 0.004;
            player += Vec2::new(angle.cos(), angle.sin()) * 800.0 * boost * dt;
            player = player.clamp(Vec2::splat(RADIUS), Vec2::splat(WORLD_SIZE - RADIUS));
            facing = turn_to_face(facing, Vec2::new(angle.cos(), angle.sin()) * 800.0, dt);
            let drill = Drill { formation, facing };
            timed!(totals[0], routes.update(player, &walls, false));
            timed!(totals[1], {
                update_squads(&mut squads, &enemies, &mut units, &walls, false);
                update_units_in(&mut units, player.x, player.y, dt, &walls, &routes.army, boost, &squads, &drill);
            });
            timed!(totals[2], {
                terrain_effects(&mut enemies, &walls, dt, &mut fx);
                patrol_enemies(&mut enemies, player, dt, boost, &walls, &routes.foes, &mut fx);
                chew_walls(&enemies, &walls, dt, &mut fx);
                enemy_abilities(&mut enemies, &units, player, dt, &mut fx);
                boss_act(&mut enemies, &units, player, dt, &mut shots, &mut fx);
            });
            timed!(totals[3], {
                shoot_enemies(&mut enemies, &units, &mut shots, dt, &mut fx);
                fire_towers(&mut walls, &mut enemies, &upgrades, dt, &mut fx);
                chew_hostile_walls(&units, &walls, &upgrades, dt, &mut fx);
                hazards.append(&mut fx.hazards);
                update_shots(&mut shots, &mut units, &walls, dt, &upgrades, &mut fx);
                update_hazards(&mut hazards, &mut units, &upgrades, dt, &mut fx);
                heal_units(&mut units, dt, &upgrades);
                fight(&mut enemies, &mut units, &mut babies, dt, &upgrades, &mut fx);
                tick_enemy_numbers(&mut enemies, dt, &mut fx);
                bury_units(&mut units, &mut fx);
            });
            timed!(totals[4], {
                update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, player, dt, false, MAX_UNITS, 0, &mut fx);
            });
            timed!(totals[5], {
                fx.fallen.clear();
                fx.heal_all = 0.0;
                fx.points = 0.0;
                respawn_enemies(&mut enemies, player.x, player.y, units.len());
                grow_babies(&mut babies, &mut units, player.x, player.y, dt, 200.0, MAX_UNITS, &mut fx);
                fx.update(dt, player);
                fx.cues.clear();
            });
            //what the drawing is handed: every unit as a disc (the whole army is in view, as if the camera were pulled right back),
            //the trails that are left behind, and the particles as discs
            timed!(totals[6], {
                let view = Rect::new(player.x - 8000.0, player.y - 8000.0, 16000.0, 16000.0);
                let discs = unit_discs(&units, view);
                std::hint::black_box(&discs);
                for unit in &units {
                    emit_trail(&mut particles, Vec2::new(unit.x, unit.y), unit.velocity, unit.speed * boost, dt, unit_color(unit));
                }
                update_particles(&mut particles, dt);
                std::hint::black_box(particle_discs(&particles));
            });
            //the army is kept at its size, so the numbers can be compared
            while units.len() < army {
                let at = random_position_around(player.x, player.y, 60.0, 700.0);
                units.push(Unit::new(at.x, at.y, 400.0));
            }
        }
        let ms = |total: f64| total * 1000.0 / frames as f64;
        let all: f64 = totals.iter().sum();
        println!(
            "{army} units {}, {} enemies: {:.2} ms a frame (routes {:.2}, army {:.2}, enemies {:.2}, combat {:.2}, sites {:.2}, rest {:.2}, ready to draw {:.2})",
            formation.name(), enemies.len(), ms(all), ms(totals[0]), ms(totals[1]), ms(totals[2]), ms(totals[3]), ms(totals[4]), ms(totals[5]), ms(totals[6])
        );
    }
}

//the army's part of a frame cut into its stages: working out where to head and stepping, shoving apart, and pushing out of walls
#[test]
#[ignore]
fn where_the_army_stage_goes() {
    for army in [1500usize, 4000, 6000] {
        rand::srand(5);
        let walls = starting_house();
        let mut player = Vec2::splat(WORLD_SIZE / 2.0);
        let mut units: Vec<Unit> = (0..army)
            .map(|_| {
                let at = random_position_around(player.x, player.y, 60.0, 700.0);
                Unit::new(at.x, at.y, 400.0)
            })
            .collect();
        let squads = Squads::new();
        let mut routes = Routes::new(player, &walls);
        let dt = 1.0 / 120.0;
        let boost = speed_multiplier(units.len());
        //[collecting where they started, move_units, spread_units, settle_units]
        let mut totals = [0.0f64; 4];
        let frames = 600;
        for frame in 0..frames {
            let angle = frame as f32 * 0.004;
            player += Vec2::new(angle.cos(), angle.sin()) * 800.0 * boost * dt;
            player = player.clamp(Vec2::splat(RADIUS), Vec2::splat(WORLD_SIZE - RADIUS));
            routes.update(player, &walls, false);
            let started: Vec<Vec2> = timed!(totals[0], units.iter().map(|unit| Vec2::new(unit.x, unit.y)).collect());
            timed!(totals[1], move_units(&mut units, player.x, player.y, dt, &walls, &routes.army, boost, &squads));
            timed!(totals[2], spread_units(&mut units));
            timed!(totals[3], settle_units(&mut units, &started, dt, &walls));
        }
        let ms = |total: f64| total * 1000.0 / frames as f64;
        let all: f64 = totals.iter().sum();
        println!("{army} units: {:.2} ms a frame (collect {:.2}, move {:.2}, spread {:.2}, settle {:.2})", ms(all), ms(totals[0]), ms(totals[1]), ms(totals[2]), ms(totals[3]));
    }
}
