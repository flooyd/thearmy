//variety, scaling and respawning of enemies

use super::*;


// ---------- variety, scaling and respawning ----------

pub(super) const PLAIN_SIZE: f32 = 44.0;
pub(super) const PLAIN_HEALTH: f32 = 100.0;

//an enemy of the ordinary kind with exactly these numbers, nothing random about them
pub(super) fn plain_enemy(start: Vec2) -> Enemy {
    let mut enemy = Enemy::new(start, &ENEMY_KINDS[0], 1.0);
    enemy.position = start;
    enemy.size = PLAIN_SIZE;
    enemy.max_health = PLAIN_HEALTH;
    enemy.health = PLAIN_HEALTH;
    enemy.babies = 1;
    enemy
}

pub(super) fn some_enemies(x: f32, y: f32, army: usize) -> Vec<Enemy> {
    let mut enemies = Vec::new();
    respawn_enemies(&mut enemies, x, y, army);
    enemies
}

#[test]
fn enemies_start_15_to_50_squares_from_the_player() {
    let player = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);
    let enemies = some_enemies(player.x, player.y, 2);
    //the groups that come together (a pack of swarmers) can take it a little over
    assert!(enemies.iter().filter(|enemy| is_wild(enemy)).count() >= ENEMY_COUNT && enemies.len() < ENEMY_COUNT + 30);
    for enemy in &enemies {
        let tiles = enemy.position.distance(player) / TILE_SIZE;
        // a big one is placed that much further out, so allow for its size at the far end, and a pack for how far it spreads
        let spread = 2.0 * PACK_SPREAD / TILE_SIZE;
        assert!(tiles >= ENEMY_SPAWN_MIN_TILES - spread - 0.01 && tiles <= ENEMY_SPAWN_MAX_TILES + enemy.size / TILE_SIZE + spread, "{tiles} squares away");
        assert_eq!(enemy.health, enemy.max_health);
    }
}

#[test]
fn only_the_kinds_that_suit_the_army_turn_up_and_all_of_them_for_a_big_one() {
    //(the kinds with no weight only ever come with something else)
    let allowed = |army: usize| ENEMY_KINDS.iter().filter(|kind| army >= kind.from_army && kind.weight > 0.0).count();
    assert_eq!(allowed(2), 3);
    for army in [1, 5, 6, 24, 25, 99, 100, 5000] {
        for _ in 0..300 {
            let kind = random_kind(army);
            assert!(army >= kind.from_army, "army of {army} got a kind that needs {}", kind.from_army);
        }
    }
    let mut seen = [0usize; ENEMY_KINDS.len()];
    for _ in 0..20_000 {
        let kind = random_kind(5000);
        seen[ENEMY_KINDS.iter().position(|k| std::ptr::eq(k, kind)).unwrap()] += 1;
    }
    let total_weight: f32 = ENEMY_KINDS.iter().map(|k| k.weight).sum();
    println!("kinds drawn for a huge army: {seen:?}");
    for (kind, &count) in ENEMY_KINDS.iter().zip(&seen) {
        let expected = 20_000.0 * kind.weight / total_weight;
        if kind.weight == 0.0 {
            assert_eq!(count, 0, "{} is never drawn by itself", kind.name);
            continue;
        }
        assert!((count as f32 - expected).abs() < expected * 0.2, "{} {count} against {expected}", kind.name);
    }
}

#[test]
fn enemies_get_bigger_as_the_army_does_up_to_a_limit() {
    assert_eq!(army_scale(0), 1.0);
    assert_eq!(army_scale(SCALE_FROM_UNITS as usize), 1.0);
    assert!((army_scale(40) - 2.0).abs() < 1e-3);
    let mut last = 1.0;
    for army in (0..2000).step_by(7) {
        let scale = army_scale(army);
        assert!(scale >= last && scale <= MAX_ENEMY_SCALE);
        last = scale;
    }
    assert_eq!(army_scale(100_000_000), MAX_ENEMY_SCALE);
}

#[test]
fn a_big_army_gets_bigger_tougher_enemies_worth_more_babies() {
    let average = |scale: f32| {
        let enemies: Vec<Enemy> = (0..400).map(|_| Enemy::new(Vec2::new(16000.0, 16000.0), &ENEMY_KINDS[0], scale)).collect();
        let n = enemies.len() as f32;
        (
            enemies.iter().map(|e| e.size).sum::<f32>() / n,
            enemies.iter().map(|e| e.max_health).sum::<f32>() / n,
            enemies.iter().map(|e| e.babies as f32).sum::<f32>() / n,
            enemies.iter().map(|e| e.speed).sum::<f32>() / n,
        )
    };
    let (size1, health1, babies1, speed1) = average(1.0);
    let (size4, health4, babies4, speed4) = average(4.0);
    println!("scale 1: size {size1:.0}, health {health1:.0}, babies {babies1:.1}, speed {speed1:.0}; scale 4: size {size4:.0}, health {health4:.0}, babies {babies4:.1}, speed {speed4:.0}");
    assert!((size4 / size1 - 4.0).abs() < 0.4, "size grows with the scale");
    assert!((health4 / health1 - 16.0).abs() < 1.6, "health grows with its square");
    assert!((babies4 / babies1 - 4.0).abs() < 0.5, "babies grow with the scale");
    assert!((speed4 / speed1 - 1.0).abs() < 0.1, "speed does not");
}

#[test]
fn spawned_enemies_for_a_big_army_are_big_ones_too() {
    let small = some_enemies(16000.0, 16000.0, 2);
    let huge = some_enemies(16000.0, 16000.0, 2000);
    let biggest = |enemies: &[Enemy]| enemies.iter().map(|e| e.size).fold(0.0, f32::max);
    let toughest = |enemies: &[Enemy]| enemies.iter().map(|e| e.max_health).fold(0.0, f32::max);
    println!(
        "army of 2: biggest enemy {:.0} px, toughest {:.0} hp; army of 2000: biggest {:.0} px, toughest {:.0} hp",
        biggest(&small), toughest(&small), biggest(&huge), toughest(&huge)
    );
    assert!(biggest(&huge) > 5.0 * biggest(&small));
    assert!(toughest(&huge) > 50.0 * toughest(&small));
}

#[test]
fn enemies_vary_in_size_colour_health_babies_and_speed() {
    let enemies: Vec<Enemy> = (0..300).map(|_| spawn_enemy(16000.0, 16000.0, 200)).collect();
    let distinct = |values: Vec<f32>| {
        let mut values: Vec<i64> = values.iter().map(|v| (v * 10.0) as i64).collect();
        values.sort();
        values.dedup();
        values.len()
    };
    let sizes = distinct(enemies.iter().map(|e| e.size).collect());
    let healths = distinct(enemies.iter().map(|e| e.max_health).collect());
    let colours = distinct(enemies.iter().map(|e| e.color.r * 7.0 + e.color.g * 13.0 + e.color.b * 29.0).collect());
    let speeds = distinct(enemies.iter().map(|e| e.speed).collect());
    let mut baby_counts: Vec<usize> = enemies.iter().map(|e| e.babies).collect();
    baby_counts.sort();
    baby_counts.dedup();
    println!("300 enemies: {sizes} sizes, {healths} healths, {colours} colours, {speeds} speeds, babies {:?}", &baby_counts[..baby_counts.len().min(12)]);
    assert!(sizes > 100 && healths > 100 && colours > 100 && speeds > 100);
    assert!(baby_counts.len() > 8 && *baby_counts.first().unwrap() >= 1);
    // really different kinds, not just nudges: both small and big, frail and tough
    let smallest = enemies.iter().map(|e| e.size).fold(f32::MAX, f32::min);
    let biggest = enemies.iter().map(|e| e.size).fold(0.0, f32::max);
    assert!(biggest > 5.0 * smallest, "{smallest} to {biggest}");
}

#[test]
fn an_enemy_is_never_given_nothing_to_drop_or_no_health() {
    for army in [0usize, 1, 10, 100, 1000, 100_000] {
        for _ in 0..300 {
            let enemy = spawn_enemy(16000.0, 16000.0, army);
            assert!((enemy.babies >= 1 || kind_named(enemy.name).babies < 1.0) && enemy.max_health > 0.0 && enemy.size > 0.0 && enemy.speed > 0.0);
        }
    }
}

#[test]
fn a_destroyed_enemy_is_replaced_somewhere_else_and_never_runs_out() {
    let player = Vec2::new(16000.0, 16000.0);
    let mut enemies = some_enemies(player.x, player.y, 3);
    // a crowd of units sits on the first enemy every frame and destroys one after another
    let mut destroyed = 0;
    let mut babies = Vec::new();
    for _ in 0..400 {
        let target = enemies[0].position;
        let mut units: Vec<Unit> = (0..40).map(|i| Unit::new(target.x + (i % 4) as f32, target.y + (i / 4) as f32, 300.0)).collect();
        let killed = fight(&mut enemies, &mut units, &mut babies, 1.0 / 60.0, &Upgrades::default(), &mut Effects::default());
        destroyed += killed;
        let before = enemies.len();
        respawn_enemies(&mut enemies, player.x, player.y, 3);
        let wild = enemies.iter().filter(|enemy| is_wild(enemy)).count();
        assert!(wild >= ENEMY_COUNT && wild < ENEMY_COUNT + 60, "{wild} wild enemies");
        assert!(enemies.len() >= before);
        //the first of a group is in the ring, the rest are up to PACK_SPREAD from it, and a pack was spread round the point it was shifted from
        let spread = 2.0 * PACK_SPREAD / TILE_SIZE;
        for fresh in &enemies[before..] {
            let tiles = fresh.position.distance(player) / TILE_SIZE;
            assert!(tiles >= ENEMY_SPAWN_MIN_TILES - spread - 0.01 && tiles <= ENEMY_SPAWN_MAX_TILES + fresh.size / TILE_SIZE + spread);
            assert_eq!(fresh.health, fresh.max_health);
        }
        let _ = killed;
    }
    println!("destroyed {destroyed} enemies and there are still {}", enemies.len());
    assert!(destroyed > 100, "only {destroyed} destroyed");
    assert!(enemies.iter().filter(|enemy| is_wild(enemy)).count() >= ENEMY_COUNT);
}

#[test]
fn replacements_appear_in_random_places_all_round_the_player() {
    let player = Vec2::new(16000.0, 16000.0);
    let mut spots = Vec::new();
    for _ in 0..200 {
        let mut enemies = Vec::new();
        respawn_enemies(&mut enemies, player.x, player.y, 3);
        spots.push(enemies[0].position);
    }
    let mean = spots.iter().fold(Vec2::ZERO, |a, b| a + *b) / spots.len() as f32;
    let spread = spots.iter().map(|p| p.distance(mean)).sum::<f32>() / spots.len() as f32;
    println!("200 spawn spots average {spread:.0} px from their middle {mean}");
    assert!(spread > 1000.0);
    for (dx, dy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        assert!(spots.iter().filter(|p| (p.x - player.x) * dx > 0.0 && (p.y - player.y) * dy > 0.0).count() > 20);
    }
}

#[test]
fn a_big_enemy_does_not_spawn_on_top_of_the_player() {
    let player = Vec2::new(16000.0, 16000.0);
    for army in [100usize, 1000, 10_000] {
        for _ in 0..200 {
            let enemy = spawn_enemy(player.x, player.y, army);
            assert!(!enemy.overlaps_circle(player.x, player.y, 1.0), "army of {army}: enemy of {:.0} px lands on the player", enemy.size);
        }
    }
}

#[test]
fn a_huge_enemy_near_the_edge_of_the_world_stays_inside_it() {
    for _ in 0..100 {
        let enemy = Enemy::new(Vec2::new(10.0, WORLD_SIZE - 5.0), &ENEMY_KINDS[4], MAX_ENEMY_SCALE);
        let half = enemy.size / 2.0;
        assert!(enemy.position.x >= half - 0.01 && enemy.position.y <= WORLD_SIZE - half + 0.01);
        for point in &enemy.route {
            assert!(point.x >= half - 0.01 && point.x <= WORLD_SIZE - half + 0.01 && point.y >= half - 0.01 && point.y <= WORLD_SIZE - half + 0.01);
        }
    }
}

#[test]
fn a_lone_baby_drops_in_the_middle_and_a_bunch_are_scattered_over_the_enemy() {
    let mut babies = Vec::new();
    drop_babies(&plain_enemy(Vec2::new(5000.0, 5000.0)), &mut babies, 0, 1.0, MAX_UNITS);
    assert_eq!(babies.len(), 1);
    assert_eq!(babies[0].position, Vec2::new(5000.0, 5000.0));

    let mut big = plain_enemy(Vec2::new(5000.0, 5000.0));
    big.size = 300.0;
    big.babies = 40;
    let mut babies = Vec::new();
    drop_babies(&big, &mut babies, 0, 1.0, MAX_UNITS);
    assert_eq!(babies.len(), 40);
    assert!(babies.iter().all(|b| (b.position.x - 5000.0).abs() <= 150.0 && (b.position.y - 5000.0).abs() <= 150.0 && b.age == 0.0));
    let spread = babies.iter().map(|b| b.position.distance(Vec2::new(5000.0, 5000.0))).fold(0.0, f32::max);
    assert!(spread > 80.0, "all piled up in the middle, {spread}");
}

#[test]
fn babies_dropped_at_the_edge_of_the_world_stay_inside_it() {
    let mut enemy = plain_enemy(Vec2::new(30.0, 30.0));
    enemy.size = 400.0;
    enemy.babies = 100;
    let mut babies = Vec::new();
    drop_babies(&enemy, &mut babies, 0, 1.0, MAX_UNITS);
    assert!(babies.iter().all(|b| b.position.x >= 0.0 && b.position.y >= 0.0 && b.position.x <= WORLD_SIZE && b.position.y <= WORLD_SIZE));
}

#[test]
fn a_destroyed_enemy_drops_all_its_babies_once() {
    let mut enemies = vec![plain_enemy(Vec2::new(1000.0, 1000.0))];
    enemies[0].babies = 7;
    let mut units = vec![Unit::new(1000.0, 1000.0, 300.0)];
    let mut babies = Vec::new();
    let mut destroyed = 0;
    for _ in 0..(120 * 3) {
        destroyed += fight(&mut enemies, &mut units, &mut babies, 1.0 / 120.0, &Upgrades::default(), &mut Effects::default());
    }
    assert_eq!((destroyed, enemies.len(), babies.len()), (1, 0, 7));
}

#[test]
fn a_tougher_enemy_takes_longer_to_destroy_and_a_bigger_one_has_more_units_inside_it() {
    let seconds_to_destroy = |health: f32, size: f32, units_in_a_row: usize| {
        let mut enemy = plain_enemy(Vec2::new(5000.0, 5000.0));
        enemy.max_health = health;
        enemy.health = health;
        enemy.size = size;
        // this is about how fast the units hurt it, not about it hurting them back
        enemy.attack = 0.0;
        let mut units: Vec<Unit> = (0..units_in_a_row)
            .map(|i| Unit::new(5000.0 - size / 2.0 + (i as f32 + 0.5) * (size / units_in_a_row as f32), 5000.0, 300.0))
            .collect();
        let mut enemies = vec![enemy];
        let mut frames = 0;
        while !enemies.is_empty() && frames < 120 * 600 {
            fight(&mut enemies, &mut units, &mut Vec::new(), 1.0 / 120.0, &Upgrades::default(), &mut Effects::default());
            frames += 1;
        }
        frames as f32 / 120.0
    };
    let plain = seconds_to_destroy(100.0, 44.0, 1);
    let tough = seconds_to_destroy(400.0, 44.0, 1);
    let tough_with_crowd = seconds_to_destroy(400.0, 400.0, 20);
    println!("100 hp, 1 unit: {plain:.2} s; 400 hp, 1 unit: {tough:.2} s; 400 hp with 20 units inside: {tough_with_crowd:.2} s");
    assert!((tough / plain - 4.0).abs() < 0.2);
    assert!(tough_with_crowd < tough / 10.0);
}
