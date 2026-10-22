//baby pickup range and the speed that grow with the army

use super::*;


// ---------- baby pickup range and speed that grow with the army ----------

pub(super) const SCREENS: [(f32, f32); 5] = [(1280.0, 720.0), (1920.0, 1080.0), (2560.0, 1440.0), (800.0, 600.0), (3840.0, 2160.0)];

#[test]
fn the_speed_multiplier_is_1_for_the_smallest_army_and_3_at_3000_units() {
    for army in [0usize, 1, 2] {
        assert!((speed_multiplier(army) - 1.0).abs() < 0.002, "{army} units: {}", speed_multiplier(army));
    }
    assert!((speed_multiplier(3000) - 3.0).abs() < 1e-4, "{}", speed_multiplier(3000));
    println!(
        "speed multiplier: 100 units x{:.2}, 500 x{:.2}, 1000 x{:.2}, 2000 x{:.2}, 3000 x{:.2}, 20000 x{:.2}",
        speed_multiplier(100), speed_multiplier(500), speed_multiplier(1000), speed_multiplier(2000), speed_multiplier(3000), speed_multiplier(20000)
    );
}

#[test]
fn the_speed_multiplier_only_goes_up_and_stops_at_the_most() {
    let mut last = 0.0;
    for army in (0..6000).step_by(13) {
        let multiplier = speed_multiplier(army);
        assert!(multiplier >= last, "went down at {army}");
        assert!(multiplier <= MAX_SPEED_MULTIPLIER);
        last = multiplier;
    }
    assert_eq!(speed_multiplier(1_000_000), MAX_SPEED_MULTIPLIER);
}

#[test]
fn the_pickup_range_starts_at_the_normal_range_and_is_a_whole_screen_at_2000_units() {
    for (w, h) in SCREENS {
        let screen = Vec2::new(w, h);
        assert!((baby_pickup_range(1, screen) - BABY_PICKUP_RANGE).abs() < 0.01, "{w}x{h}");
        // from the middle of the screen to its corner, with the world zoomed all the way out
        let to_the_corner = screen.length() / 2.0 / MIN_ZOOM;
        let full = baby_pickup_range(PICKUP_FULL_UNITS, screen);
        println!("{w}x{h}: 1 unit {:.0}, 100 {:.0}, 500 {:.0}, 1000 {:.0}, 2000 {:.0} (corner at {:.0})",
            baby_pickup_range(1, screen), baby_pickup_range(100, screen), baby_pickup_range(500, screen), baby_pickup_range(1000, screen), full, to_the_corner);
        assert!(full >= to_the_corner * 0.999, "{w}x{h}: {full} does not reach {to_the_corner}");
        assert!(full <= to_the_corner * 1.001);
    }
}

#[test]
fn the_pickup_range_only_grows_and_stops_at_a_screen() {
    let screen = Vec2::new(1280.0, 720.0);
    let mut last = 0.0;
    for army in 1..4000 {
        let range = baby_pickup_range(army, screen);
        assert!(range >= last - 0.001, "shrank at {army}");
        last = range;
    }
    assert_eq!(baby_pickup_range(2000, screen), baby_pickup_range(100_000, screen));
    // it grows steadily rather than all at once at the end: a fair bit bigger by 100 units, nowhere near a screen
    assert!(baby_pickup_range(100, screen) > 1.15 * BABY_PICKUP_RANGE);
    assert!(baby_pickup_range(500, screen) < baby_pickup_range(2000, screen) / 5.0);
    assert!(baby_pickup_range(1000, screen) < baby_pickup_range(2000, screen) / 2.0);
}

#[test]
fn at_2000_units_every_baby_on_a_screen_zoomed_all_the_way_out_is_picked_up() {
    for (w, h) in SCREENS {
        let screen = Vec2::new(w, h);
        let player = Vec2::new(16000.0, 16000.0);
        // what is on screen at the smallest zoom, the camera is on the player
        let (half_w, half_h) = (w / MIN_ZOOM / 2.0, h / MIN_ZOOM / 2.0);
        let mut babies: Vec<Baby> = Vec::new();
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (0.0, 1.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 0.0)] {
            // right at the corners and edges of the screen
            babies.push(Baby { position: player + Vec2::new(dx * half_w * 0.999, dy * half_h * 0.999), age: BABY_PICKUP_DELAY });
        }
        let total = babies.len();
        let mut units = Vec::new();
        grow_babies(&mut babies, &mut units, player.x, player.y, 0.0, baby_pickup_range(2000, screen), MAX_UNITS, &mut Effects::default());
        assert_eq!((babies.len(), units.len()), (0, total), "{w}x{h}: a baby on screen was left");
        // the same screen with a small army leaves them all
        let mut babies: Vec<Baby> = (0..total).map(|i| Baby { position: player + Vec2::new(half_w * 0.5, i as f32), age: BABY_PICKUP_DELAY }).collect();
        let mut units = Vec::new();
        grow_babies(&mut babies, &mut units, player.x, player.y, 0.0, baby_pickup_range(20, screen), MAX_UNITS, &mut Effects::default());
        assert_eq!(units.len(), 0, "{w}x{h}: a small army reached half a screen away");
    }
}

//one unit chasing a player standing still in the open, for a second, and how far it got
pub(super) fn distance_covered_in_a_second(speed: f32, boost: f32, fps: f32) -> f32 {
    let walls = walls_of(&[]);
    let player = Vec2::new(30000.0, 3000.0);
    let field = field_for(player, &walls);
    let mut units = vec![Unit::new(3000.0, 3000.0, speed)];
    for _ in 0..fps as usize {
        update_units(&mut units, player.x, player.y, 1.0 / fps, &walls, &field, boost, &Squads::new());
    }
    units[0].x - 3000.0
}

#[test]
fn a_unit_goes_as_much_faster_as_the_multiplier_says_at_any_frame_rate() {
    for fps in [30.0, 60.0, 120.0, 240.0] {
        let normal = distance_covered_in_a_second(400.0, 1.0, fps);
        let boosted = distance_covered_in_a_second(400.0, 3.0, fps);
        let slight = distance_covered_in_a_second(400.0, 1.5, fps);
        println!("{fps} fps: x1 {normal:.0} px, x1.5 {slight:.0} px, x3 {boosted:.0} px");
        assert!((normal - 400.0).abs() < 2.0, "{normal}");
        assert!((slight / normal - 1.5).abs() < 0.01);
        assert!((boosted / normal - 3.0).abs() < 0.01);
    }
}

#[test]
fn a_fast_unit_still_stops_the_follow_distance_from_the_player_and_not_inside_them() {
    let walls = walls_of(&[]);
    let player = Vec2::new(3000.0, 3000.0);
    let field = field_for(player, &walls);
    // top speed 550 x3, with a frame long enough that one step is 80 px
    for (fps, boost) in [(120.0, 1.0), (60.0, 3.0), (20.0, 3.0)] {
        let mut units = vec![
            Unit::new(2000.0, 3000.0, 550.0),
            Unit::new(3000.0, 4500.0, 250.0),
            Unit::new(3700.0, 3500.0, 550.0),
            Unit::new(3030.0 + UNIT_FOLLOW_DISTANCE + 20.0, 3000.0, 550.0),
        ];
        for _ in 0..(fps * 8.0) as usize {
            update_units(&mut units, player.x, player.y, 1.0 / fps, &walls, &field, boost, &Squads::new());
        }
        for unit in &units {
            let distance = Vec2::new(unit.x, unit.y).distance(player);
            assert!(distance > 2.0 * RADIUS && distance <= UNIT_FOLLOW_DISTANCE + 0.01 && distance >= UNIT_FOLLOW_DISTANCE - 3.0, "{fps} fps x{boost}: stopped {distance} from the player");
        }
    }
}

pub(super) fn run_boosted_crowd(count: usize, speed: f32, fps: f32, boost: f32, seconds: f32) -> (usize, usize) {
    let walls = starting_house();
    let player = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);
    let field = field_for(player, &walls);
    let mut units: Vec<Unit> = (0..count)
        .map(|_| {
            let p = far_position(player.x, player.y);
            Unit::new(p.x, p.y, speed)
        })
        .collect();
    let mut worst_inside = 0;
    for _ in 0..(seconds * fps) as usize {
        update_units(&mut units, player.x, player.y, 1.0 / fps, &walls, &field, boost, &Squads::new());
        worst_inside = worst_inside.max(units.iter().filter(|e| in_the_house(e)).count());
    }
    (worst_inside, units.iter().filter(|e| touches_wall(e.x, e.y, RADIUS - 2.0, &walls)).count())
}

#[test]
fn a_crowd_at_triple_speed_still_does_not_get_into_the_house() {
    // the quickest unit is 550 px/s x3 = 1650, at the slowest frame rate that is still run: 82 px a frame
    for (count, fps) in [(500, 120.0), (500, 60.0), (200, 20.0)] {
        let (worst_inside, in_a_wall) = run_boosted_crowd(count, 550.0, fps, 3.0, 30.0);
        println!("{count} units at x3 and {fps} fps: at worst {worst_inside} inside the house, {in_a_wall} left in a wall");
        assert_eq!(worst_inside, 0, "{count} units at x3 {fps} fps got into the house");
        assert_eq!(in_a_wall, 0);
    }
}

#[test]
fn the_player_at_triple_speed_cannot_get_into_the_house_either() {
    let walls = starting_house();
    // 800 x3 = 2400 px/s, a long hitch of 0.05 s is 120 px in one step, and the same from the inside
    let mut p = Vec2::new(15400.0 + SHIFT, 16000.0 + SHIFT);
    for _ in 0..200 {
        p = move_clear_of_walls(p, p + Vec2::new(120.0, 0.0), &walls);
    }
    assert!(p.x <= 15680.0 + SHIFT - RADIUS + 0.01, "from outside, got to x {}", p.x);
    let mut p = Vec2::new(15800.0 + SHIFT, 16000.0 + SHIFT);
    for _ in 0..200 {
        p = move_clear_of_walls(p, p + Vec2::new(-120.0, 0.0), &walls);
    }
    assert!(p.x >= 15744.0 + SHIFT + RADIUS - 0.01, "from the inside, got to x {}", p.x);
    for direction in [Vec2::new(0.0, 1.0), Vec2::new(0.0, -1.0), Vec2::new(0.7, 0.7).normalize(), Vec2::new(-0.7, 0.7).normalize()] {
        let mut p = Vec2::new(16000.0 + SHIFT, 16000.0 + SHIFT);
        for _ in 0..300 {
            p = move_clear_of_walls(p, p + direction * 120.0, &walls);
        }
        assert!(!touches_wall(p.x, p.y, RADIUS - 0.01, &walls));
        assert!(p.x > 15744.0 + SHIFT && p.x < 16256.0 + SHIFT && p.y > 15744.0 + SHIFT && p.y < 16256.0 + SHIFT, "left the house: {p}");
    }
}

#[test]
fn enemies_walk_as_much_faster_as_the_multiplier_says() {
    for boost in [1.0f32, 2.0, 3.0] {
        let mut enemies = vec![plain_enemy(Vec2::new(16000.0, 16000.0))];
        enemies[0].route = vec![Vec2::new(20000.0, 16000.0), Vec2::new(16000.0, 16000.0)];
        enemies[0].speed = 90.0;
        for _ in 0..120 {
            patrol_free(&mut enemies, NOBODY, 1.0 / 120.0, boost, &mut Effects::default());
        }
        let walked = enemies[0].position.x - 16000.0;
        assert!((walked - 90.0 * boost).abs() < 0.5, "x{boost}: walked {walked}");
    }
}

#[test]
fn a_fast_enemy_does_not_overshoot_its_waypoints_or_leave_its_square() {
    let start = Vec2::new(16000.0, 16000.0);
    let mut enemy = plain_enemy(start);
    enemy.speed = 240.0;
    let route = enemy.route.clone();
    let mut enemies = vec![enemy];
    let mut reached = 0;
    let mut heading_for = 0;
    for _ in 0..(20 * 120) {
        // x3 on a runner, in long 20 fps frames: 36 px a step
        patrol_free(&mut enemies, NOBODY, 1.0 / 20.0, 3.0, &mut Effects::default());
        let e = &enemies[0];
        assert!((e.position.x - start.x).abs() <= 480.0 + 0.5 && (e.position.y - start.y).abs() <= 480.0 + 0.5, "left the area at {}", e.position);
        if e.heading_for != heading_for {
            assert_eq!(e.heading_for, (heading_for + 1) % route.len(), "skipped a waypoint");
            heading_for = e.heading_for;
            reached += 1;
        }
    }
    assert!(reached > route.len() * 3, "only reached {reached} waypoints");
}
