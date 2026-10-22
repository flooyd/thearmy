//an army crammed onto one spot

use super::*;

// ---------- an army crammed onto one spot ----------

#[test]
fn spreading_a_huge_army_crammed_into_a_small_space_stays_cheap_and_still_pushes_it_apart() {
    let centre = Vec2::new(16000.0, 16000.0);
    let mut units: Vec<Unit> = (0..DEFAULT_ARMY_LIMIT.min(6000)).map(|_| {
        let at = random_position_around(centre.x, centre.y, 0.0, 150.0);
        unit_on(at)
    }).collect();
    let mean_distance = |units: &[Unit]| units.iter().map(|unit| Vec2::new(unit.x, unit.y).distance(centre)).sum::<f32>() / units.len() as f32;
    let before = mean_distance(&units);
    let started = Instant::now();
    let frames = 30;
    for _ in 0..frames {
        spread_units(&mut units);
    }
    let per_frame = started.elapsed().as_secs_f32() / frames as f32 * 1000.0;
    eprintln!("spreading {} units crammed into a 150 px circle: {per_frame:.2} ms a frame", units.len());
    //with every unit compared with hundreds of others it took 20 ms and more
    //(other tests run at the same time and can triple this, the old way of doing it took 20 ms and more)
    assert!(per_frame < 10.0, "{per_frame} ms a frame");
    assert!(mean_distance(&units) > before * 1.05, "the crowd was not pushed outwards: {before} -> {}", mean_distance(&units));
}

#[test]
fn two_units_on_top_of_each_other_are_still_pushed_apart_exactly() {
    let mut units = vec![unit_on(Vec2::new(500.0, 500.0)), unit_on(Vec2::new(510.0, 500.0))];
    spread_units(&mut units);
    let distance = Vec2::new(units[0].x, units[0].y).distance(Vec2::new(units[1].x, units[1].y));
    assert!((distance - 32.0).abs() < 0.01, "{distance}");
}

#[test]
fn the_unit_grid_finds_every_unit_an_enemy_touches_wherever_and_however_big_it_is() {
    rand::srand(11);
    let mut units: Vec<Unit> = (0..3000).map(|_| unit_on(random_position_around(16000.0, 16000.0, 0.0, 900.0))).collect();
    //a couple of strays a long way from the rest stretch the grid out
    units.push(unit_on(Vec2::new(2000.0, 30000.0)));
    units.push(unit_on(Vec2::new(40000.0, 1500.0)));
    let grid = UnitGrid::new(&units);
    let mut found = Vec::new();
    let mut touched = 0;
    for k in 0..600 {
        let at = match k % 3 {
            0 => random_position_around(16000.0, 16000.0, 0.0, 1100.0),
            1 => Vec2::new(rand::gen_range(0.0, WORLD_SIZE), rand::gen_range(0.0, WORLD_SIZE)),
            _ => {
                let unit = &units[rand::gen_range(0, units.len())];
                Vec2::new(unit.x, unit.y)
            }
        };
        //from a speck to something as big as the biggest boss
        let size = [4.0, 60.0, 300.0, 2500.0][k % 4] * rand::gen_range(0.5, 1.5);
        let mut enemy = kind_at("Brute", at);
        enemy.size = size;
        let reach = Vec2::splat(enemy.size / 2.0 + RADIUS);
        found.clear();
        grid.near(enemy.position - reach, enemy.position + reach, &mut found);
        let candidates = found.len();
        found.sort_unstable();
        found.dedup();
        assert_eq!(found.len(), candidates, "a unit was found twice");
        found.retain(|&i| enemy.overlaps_circle(units[i].x, units[i].y, RADIUS));
        let brute: Vec<usize> = (0..units.len()).filter(|&i| enemy.overlaps_circle(units[i].x, units[i].y, RADIUS)).collect();
        assert_eq!(found, brute, "enemy of {size} at {at}");
        touched += brute.len();
    }
    assert!(touched > 1000, "the test should have something to find: {touched}");
    //no units is no cells, and nothing to find
    let empty = UnitGrid::new(&[]);
    empty.near(Vec2::splat(-1.0e6), Vec2::splat(1.0e6), &mut found);
    found.clear();
    empty.near(Vec2::ZERO, Vec2::splat(500.0), &mut found);
    assert!(found.is_empty());
}
