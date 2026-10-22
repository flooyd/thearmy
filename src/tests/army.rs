//the army: units, enemies and babies

use super::*;


// ---------- the army: units, enemies, babies ----------

//the crowd tests used to spawn units far away like this
pub(super) fn far_position(x: f32, y: f32) -> Vec2 {
    random_position_around(x, y, 15.0 * TILE_SIZE, 50.0 * TILE_SIZE)
}

#[test]
fn units_start_in_range_even_next_to_the_edge_of_the_world() {
    for (x, y) in [(RADIUS, RADIUS), (WORLD_SIZE - RADIUS, 50.0), (3000.0, WORLD_SIZE - RADIUS)] {
        for _ in 0..200 {
            for unit in starting_units(x, y) {
                assert!(unit.x >= RADIUS && unit.x <= WORLD_SIZE - RADIUS && unit.y >= RADIUS && unit.y <= WORLD_SIZE - RADIUS);
            }
        }
    }
}

#[test]
fn a_patrol_route_stays_inside_the_15_by_15_square_round_the_start() {
    let half = 7.5 * TILE_SIZE;
    for start in [Vec2::new(16000.0, 16000.0), Vec2::new(100.0, 100.0), Vec2::new(WORLD_SIZE - 30.0, 5000.0), Vec2::new(8000.0, WORLD_SIZE - 10.0)] {
        for _ in 0..200 {
            let route = patrol_route(start, PLAIN_SIZE);
            assert!((ENEMY_ROUTE_MIN_POINTS..=ENEMY_ROUTE_MAX_POINTS).contains(&(route.len() as i32)));
            for point in &route {
                assert!((point.x - start.x).abs() <= half + 0.01 && (point.y - start.y).abs() <= half + 0.01, "{point} too far from {start}");
                assert!(point.x >= PLAIN_SIZE / 2.0 && point.x <= WORLD_SIZE - PLAIN_SIZE / 2.0 && point.y >= PLAIN_SIZE / 2.0 && point.y <= WORLD_SIZE - PLAIN_SIZE / 2.0);
            }
        }
    }
}

#[test]
fn routes_are_different_each_time_and_use_the_whole_area() {
    let start = Vec2::new(16000.0, 16000.0);
    let routes: Vec<Vec<Vec2>> = (0..200).map(|_| patrol_route(start, PLAIN_SIZE)).collect();
    let points: Vec<Vec2> = routes.iter().flatten().copied().collect();
    let (low_x, high_x) = points.iter().fold((f32::MAX, f32::MIN), |(l, h), p| (l.min(p.x), h.max(p.x)));
    let (low_y, high_y) = points.iter().fold((f32::MAX, f32::MIN), |(l, h), p| (l.min(p.y), h.max(p.y)));
    println!("points span x {:.0}..{:.0}, y {:.0}..{:.0} of {:.0}..{:.0}", low_x, high_x, low_y, high_y, start.x - 480.0, start.x + 480.0);
    assert!(high_x - low_x > 0.9 * 960.0 && high_y - low_y > 0.9 * 960.0);
    assert!(routes.windows(2).any(|pair| pair[0] != pair[1]));
}

#[test]
fn an_enemy_walks_its_route_in_order_and_loops_without_leaving_the_square() {
    let start = Vec2::new(16000.0, 16000.0);
    let mut enemy = plain_enemy(start);
    enemy.speed = 100.0;
    let route = enemy.route.clone();
    let mut visited = Vec::new();
    let dt = 1.0 / 120.0;
    let mut enemies = vec![enemy];
    let mut last_target = 0;
    for _ in 0..(120 * 240) {
        patrol_free(&mut enemies, NOBODY, dt, 1.0, &mut Effects::default());
        let e = &enemies[0];
        assert!((e.position.x - start.x).abs() <= 480.0 + 0.5 && (e.position.y - start.y).abs() <= 480.0 + 0.5, "left the area at {}", e.position);
        if e.heading_for != last_target {
            visited.push(last_target);
            last_target = e.heading_for;
        }
    }
    println!("{} waypoints reached in 240 s on a route of {}", visited.len(), route.len());
    assert!(visited.len() > route.len() * 2, "did not loop");
    for (i, &point) in visited.iter().enumerate() {
        assert_eq!(point, i % route.len(), "waypoints out of order");
    }
}

#[test]
fn an_enemy_moves_at_its_own_speed() {
    let mut enemies = vec![plain_enemy(Vec2::new(16000.0, 16000.0))];
    enemies[0].route = vec![Vec2::new(17000.0, 16000.0), Vec2::new(16000.0, 16000.0)];
    enemies[0].speed = 90.0;
    for _ in 0..120 {
        patrol_free(&mut enemies, NOBODY, 1.0 / 120.0, 1.0, &mut Effects::default());
    }
    assert!((enemies[0].position.x - 16090.0).abs() < 1.0 && enemies[0].position.y == 16000.0, "{}", enemies[0].position);
}


#[test]
fn a_circle_touches_an_enemy_square_on_its_sides_and_corners_only_when_close_enough() {
    let mut enemy = plain_enemy(Vec2::new(1000.0, 1000.0));
    enemy.position = Vec2::new(1000.0, 1000.0);
    let half = PLAIN_SIZE / 2.0;
    assert!(enemy.overlaps_circle(1000.0, 1000.0, RADIUS)); // inside
    assert!(enemy.overlaps_circle(1000.0 + half + RADIUS - 1.0, 1000.0, RADIUS)); // just touching a side
    assert!(!enemy.overlaps_circle(1000.0 + half + RADIUS + 1.0, 1000.0, RADIUS)); // just off a side
    // the corner: a circle diagonal from it touches only when the corner is within its radius
    let diagonal = RADIUS / 2.0f32.sqrt();
    assert!(enemy.overlaps_circle(1000.0 + half + diagonal - 1.0, 1000.0 + half + diagonal - 1.0, RADIUS));
    assert!(!enemy.overlaps_circle(1000.0 + half + diagonal + 1.0, 1000.0 + half + diagonal + 1.0, RADIUS));
}

pub(super) fn enemy_on(x: f32, y: f32) -> Enemy {
    let mut enemy = plain_enemy(Vec2::new(x, y));
    enemy.position = Vec2::new(x, y);
    enemy
}

#[test]
fn a_unit_inside_an_enemy_hurts_it_and_one_outside_does_not() {
    let mut enemies = vec![enemy_on(1000.0, 1000.0), enemy_on(5000.0, 5000.0)];
    let mut units = vec![Unit::new(1005.0, 1000.0, 300.0), Unit::new(3000.0, 3000.0, 300.0)];
    let mut babies = Vec::new();
    fight(&mut enemies, &mut units, &mut babies, 0.1, &Upgrades::default(), &mut Effects::default());
    assert!((enemies[0].health - (PLAIN_HEALTH - UNIT_DAMAGE * 0.1)).abs() < 0.01, "{}", enemies[0].health);
    assert_eq!(enemies[1].health, PLAIN_HEALTH);
    assert!(babies.is_empty());
}

#[test]
fn more_units_in_an_enemy_hurt_it_faster() {
    let mut enemies = vec![enemy_on(1000.0, 1000.0)];
    let mut units = vec![Unit::new(1000.0, 1000.0, 300.0), Unit::new(1010.0, 1000.0, 300.0), Unit::new(1000.0, 1010.0, 300.0)];
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &Upgrades::default(), &mut Effects::default());
    assert!((enemies[0].health - (PLAIN_HEALTH - 3.0 * UNIT_DAMAGE * 0.1)).abs() < 0.01);
}

#[test]
fn an_enemy_that_runs_out_of_health_is_destroyed_and_drops_a_baby_where_it_was() {
    let mut enemies = vec![enemy_on(1000.0, 1000.0), enemy_on(5000.0, 5000.0)];
    let mut units = vec![Unit::new(1000.0, 1000.0, 300.0)];
    let mut babies = Vec::new();
    let frames = (PLAIN_HEALTH / UNIT_DAMAGE * 120.0).ceil() as usize + 1;
    for frame in 0..frames {
        fight(&mut enemies, &mut units, &mut babies, 1.0 / 120.0, &Upgrades::default(), &mut Effects::default());
        if frame < frames - 3 {
            assert_eq!(enemies.len(), 2, "destroyed too early, frame {frame}");
        }
    }
    assert_eq!(enemies.len(), 1);
    assert_eq!(enemies[0].position, Vec2::new(5000.0, 5000.0));
    assert_eq!(babies.len(), 1, "exactly one baby, however long the unit stays");
    assert_eq!(babies[0].position, Vec2::new(1000.0, 1000.0));
    // and the rest of the frames do not drop more
    for _ in 0..100 {
        fight(&mut enemies, &mut units, &mut babies, 1.0 / 120.0, &Upgrades::default(), &mut Effects::default());
    }
    assert_eq!(babies.len(), 1);
}

#[test]
fn a_unit_running_through_an_enemy_does_damage_for_the_time_it_is_inside() {
    for fps in [60.0f32, 120.0, 240.0] {
        let dt = 1.0 / fps;
        let mut enemies = vec![enemy_on(2000.0, 2000.0)];
        let mut unit = Unit::new(1800.0, 2000.0, 400.0);
        let mut babies = Vec::new();
        // straight through at 400 px/s, nothing in the way. the circle is inside the square while it is within half
        // the square plus its radius on each side of the middle
        for _ in 0..(fps * 1.0) as usize {
            unit.x += 400.0 * dt;
            fight(&mut enemies, std::slice::from_mut(&mut unit), &mut babies, dt, &Upgrades::default(), &mut Effects::default());
        }
        let dealt = PLAIN_HEALTH - enemies[0].health;
        let expected = UNIT_DAMAGE * (PLAIN_SIZE + 2.0 * RADIUS) / 400.0;
        println!("{fps} fps: dealt {dealt:.1}, about {expected:.1} expected");
        assert!((dealt - expected).abs() < expected * 0.12, "{dealt} vs {expected}");
    }
}

#[test]
fn a_baby_grows_into_a_unit_when_the_player_comes_within_range_and_not_before() {
    let mut babies = vec![Baby { position: Vec2::new(3000.0, 3000.0), age: BABY_PICKUP_DELAY }];
    let mut units = Vec::new();
    grow_babies(&mut babies, &mut units, 3000.0 + BABY_PICKUP_RANGE + 1.0, 3000.0, 0.0, BABY_PICKUP_RANGE, MAX_UNITS, &mut Effects::default());
    assert_eq!((babies.len(), units.len()), (1, 0), "grew when the player was out of range");
    grow_babies(&mut babies, &mut units, 3000.0 + BABY_PICKUP_RANGE - 1.0, 3000.0, 0.0, BABY_PICKUP_RANGE, MAX_UNITS, &mut Effects::default());
    assert_eq!((babies.len(), units.len()), (0, 1));
    assert_eq!((units[0].x, units[0].y), (3000.0, 3000.0));
    assert!(units[0].speed >= UNIT_MIN_SPEED && units[0].speed <= UNIT_MAX_SPEED);
    // nothing left to grow
    grow_babies(&mut babies, &mut units, 3000.0, 3000.0, 0.0, BABY_PICKUP_RANGE, MAX_UNITS, &mut Effects::default());
    assert_eq!(units.len(), 1);
}

#[test]
fn only_the_babies_in_range_grow() {
    let mut babies = vec![
        Baby { position: Vec2::new(1000.0, 1000.0), age: BABY_PICKUP_DELAY },
        Baby { position: Vec2::new(1100.0, 1000.0), age: BABY_PICKUP_DELAY },
        Baby { position: Vec2::new(5000.0, 5000.0), age: BABY_PICKUP_DELAY },
    ];
    let mut units = Vec::new();
    grow_babies(&mut babies, &mut units, 1050.0, 1000.0, 0.0, BABY_PICKUP_RANGE, MAX_UNITS, &mut Effects::default());
    assert_eq!(units.len(), 2);
    assert_eq!(babies.len(), 1);
    assert_eq!(babies[0].position, Vec2::new(5000.0, 5000.0));
}


#[test]
fn walls_cannot_be_placed_on_a_baby() {
    let mut walls = Walls::default();
    let babies = [Baby { position: Vec2::new(330.0, 330.0), age: BABY_PICKUP_DELAY }]; // inside tile (5,5)
    assert!(!place_wall(&mut walls, WallKind::Wall, WALL_HEALTH, 5, 5, 16000.0, 16000.0, &[], &babies));
    assert!(place_wall(&mut walls, WallKind::Wall, WALL_HEALTH, 9, 9, 16000.0, 16000.0, &[], &babies));
}

#[test]
fn a_whole_fight_works_start_to_finish() {
    // no walls. the player runs at the nearest enemy and the units follow, wearing it down. it is destroyed, drops a
    // baby, and the player walking over to it makes a new unit
    let walls = Walls::default();
    let mut player = Vec2::new(16000.0, 16000.0);
    let mut units = starting_units(player.x, player.y);
    let started_with = units.len();
    let mut enemies = vec![enemy_on(16000.0 + 6.0 * TILE_SIZE, 16000.0)];
    enemies[0].route = vec![Vec2::new(16000.0 + 6.0 * TILE_SIZE, 16000.0), Vec2::new(16000.0 + 6.0 * TILE_SIZE, 16000.0 + 200.0)];
    let mut babies: Vec<Baby> = Vec::new();
    let dt = 1.0 / 120.0;
    let mut field = FlowField::new();
    let mut killed_at = None;
    let mut grown_at = None;
    for frame in 0..(120 * 60) {
        if frame % 12 == 0 {
            field.compute(player, &walls);
        }
        // the player heads for the enemy, or for the baby once there is one
        let goal = if let Some(baby) = babies.first() { baby.position } else if let Some(enemy) = enemies.first() { enemy.position } else { player };
        let direction = (goal - player).normalize_or_zero();
        player += direction * 600.0 * dt;
        update_units(&mut units, player.x, player.y, dt, &walls, &field, 1.0, &Squads::new());
        patrol_free(&mut enemies, NOBODY, dt, 1.0, &mut Effects::default());
        fight(&mut enemies, &mut units, &mut babies, dt, &Upgrades::default(), &mut Effects::default());
        if enemies.is_empty() && killed_at.is_none() {
            killed_at = Some(frame);
            assert_eq!(babies.len(), 1);
        }
        let before = units.len();
        grow_babies(&mut babies, &mut units, player.x, player.y, dt, BABY_PICKUP_RANGE, MAX_UNITS, &mut Effects::default());
        if units.len() > before && grown_at.is_none() {
            grown_at = Some(frame);
        }
    }
    println!("started with {started_with} units, killed the enemy at {:.1} s, new unit at {:?}, ended with {}", killed_at.unwrap_or(0) as f32 / 120.0, grown_at.map(|f| f as f32 / 120.0), units.len());
    assert!(killed_at.is_some(), "the enemy was never destroyed");
    assert!(grown_at.is_some(), "the baby never grew");
    assert_eq!(units.len(), started_with + 1);
    assert!(babies.is_empty());
}

#[test]
fn units_and_the_player_are_not_stopped_by_enemies() {
    // an enemy sitting right on the path makes no difference to a unit chasing the player past it
    let walls = Walls::default();
    let player = Vec2::new(3000.0, 3000.0);
    let field = field_for(player, &walls);
    let mut with = vec![Unit::new(2000.0, 3000.0, 300.0)];
    let mut without = vec![Unit::new(2000.0, 3000.0, 300.0)];
    let mut enemies = vec![enemy_on(2500.0, 3000.0)];
    enemies[0].route = vec![Vec2::new(2500.0, 3000.0)];
    for _ in 0..(120 * 3) {
        update_units(&mut with, player.x, player.y, 1.0 / 120.0, &walls, &field, 1.0, &Squads::new());
        patrol_free(&mut enemies, NOBODY, 1.0 / 120.0, 1.0, &mut Effects::default());
        fight(&mut enemies, &mut with, &mut Vec::new(), 1.0 / 120.0, &Upgrades::default(), &mut Effects::default());
        update_units(&mut without, player.x, player.y, 1.0 / 120.0, &walls, &field, 1.0, &Squads::new());
    }
    // the enemy was in the way for a while and the unit went straight through it
    assert!(with[0].x > 2500.0 + PLAIN_SIZE);
    assert!((with[0].x - without[0].x).abs() < 0.5 && (with[0].y - without[0].y).abs() < 0.5);
}

#[test]
fn a_following_unit_stops_a_little_clear_of_the_player() {
    let walls = Walls::default();
    let player = Vec2::new(3000.0, 3000.0);
    let field = field_for(player, &walls);
    let mut units = vec![Unit::new(2400.0, 3000.0, 400.0), Unit::new(3000.0, 3600.0, 250.0), Unit::new(3500.0, 3400.0, 550.0)];
    for _ in 0..(120 * 6) {
        update_units(&mut units, player.x, player.y, 1.0 / 120.0, &walls, &field, 1.0, &Squads::new());
    }
    for unit in &units {
        let distance = Vec2::new(unit.x, unit.y).distance(player);
        println!("settled {distance:.1} px from the player");
        // not touching the player (which would be 2 radii), and not far off either
        assert!(distance > 2.0 * RADIUS && distance < UNIT_FOLLOW_DISTANCE + 5.0, "{distance}");
    }
}

#[test]
fn a_new_baby_waits_a_moment_before_it_can_grow_even_with_the_player_on_it() {
    let mut babies: Vec<Baby> = Vec::new();
    let mut enemies = vec![enemy_on(1000.0, 1000.0)];
    let mut units = vec![Unit::new(1000.0, 1000.0, 300.0)];
    while !enemies.is_empty() {
        fight(&mut enemies, &mut units, &mut babies, 1.0 / 120.0, &Upgrades::default(), &mut Effects::default());
    }
    assert_eq!(babies.len(), 1);
    assert_eq!(babies[0].age, 0.0);
    let mut grown = Vec::new();
    let frames = (BABY_PICKUP_DELAY * 120.0) as usize;
    for frame in 0..frames - 2 {
        grow_babies(&mut babies, &mut grown, 1000.0, 1000.0, 1.0 / 120.0, BABY_PICKUP_RANGE, MAX_UNITS, &mut Effects::default());
        assert_eq!((babies.len(), grown.len()), (1, 0), "grew after only {frame} frames");
    }
    for _ in 0..6 {
        grow_babies(&mut babies, &mut grown, 1000.0, 1000.0, 1.0 / 120.0, BABY_PICKUP_RANGE, MAX_UNITS, &mut Effects::default());
    }
    assert_eq!((babies.len(), grown.len()), (0, 1));
}

#[test]
fn a_baby_that_is_waiting_still_waits_for_the_player_to_come() {
    let mut babies = vec![Baby { position: Vec2::new(1000.0, 1000.0), age: 0.0 }];
    let mut units = Vec::new();
    for _ in 0..(120 * 5) {
        grow_babies(&mut babies, &mut units, 4000.0, 4000.0, 1.0 / 120.0, BABY_PICKUP_RANGE, MAX_UNITS, &mut Effects::default());
    }
    assert_eq!((babies.len(), units.len()), (1, 0));
    grow_babies(&mut babies, &mut units, 1100.0, 1000.0, 1.0 / 120.0, BABY_PICKUP_RANGE, MAX_UNITS, &mut Effects::default());
    assert_eq!((babies.len(), units.len()), (0, 1));
}

#[test]
fn damage_check_time_with_a_lot_of_units_and_enemies() {
    // the user stress tests with thousands of units, and every enemy is checked against every unit
    for (unit_count, enemy_count) in [(100, 50), (2000, 50), (4000, 50), (4000, 500)] {
        let mut units: Vec<Unit> = (0..unit_count).map(|_| { let p = far_position(16000.0, 16000.0); Unit::new(p.x, p.y, 300.0) }).collect();
        let mut enemies: Vec<Enemy> = (0..enemy_count).map(|_| spawn_enemy(16000.0, 16000.0, 10)).collect();
        let mut babies = Vec::new();
        let started = std::time::Instant::now();
        for _ in 0..100 {
            fight(&mut enemies, &mut units, &mut babies, 1.0 / 120.0, &Upgrades::default(), &mut Effects::default());
        }
        println!("{unit_count} units x {enemy_count} enemies: {:.3} ms per frame", started.elapsed().as_secs_f64() * 10.0);
    }
}
