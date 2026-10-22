//the shortcuts that make a frame cheaper must not change what happens in it

use super::*;

#[test]
fn the_parts_of_the_world_with_walls_near_them_are_marked_and_the_rest_are_not() {
    let near = NearWalls::new(&Walls::default());
    assert!(!near.may_be_near(Vec2::new(5000.0, 5000.0)), "no walls, nothing is near one");
    assert!(near.may_be_near(Vec2::new(-10.0, 5000.0)) && near.may_be_near(Vec2::new(5000.0, WORLD_SIZE + 2000.0)), "outside the world is not worth guessing at");
    let walls = walls_of(&[(100, 100), (3, 900)]);
    let near = NearWalls::new(&walls);
    let at = |col: f32, row: f32| Vec2::new(col * TILE_SIZE, row * TILE_SIZE);
    //anything within eight grid squares of a wall is marked, whichever side of it
    for (col, row) in [(100.5, 100.5), (93.0, 100.0), (107.9, 100.0), (100.0, 92.1), (100.0, 108.9), (95.0, 105.0), (3.5, 900.5), (0.1, 905.0)] {
        assert!(near.may_be_near(at(col, row)), "({col}, {row})");
    }
    //and there are places that are not
    for (col, row) in [(100.0, 125.0), (130.0, 100.0), (60.0, 60.0), (3.0, 850.0), (500.0, 500.0)] {
        assert!(!near.may_be_near(at(col, row)), "({col}, {row})");
    }
    //a wall on the edge of the world doesn't go off the edge of what is marked
    let corner = NearWalls::new(&walls_of(&[(0, 0), (WORLD_TILES - 1, WORLD_TILES - 1)]));
    assert!(corner.may_be_near(at(2.0, 2.0)) && corner.may_be_near(at(WORLD_TILES as f32 - 3.0, WORLD_TILES as f32 - 3.0)) && !corner.may_be_near(at(500.0, 500.0)));
}

#[test]
fn units_are_settled_the_same_with_the_shortcut_for_the_ones_far_from_walls_as_without() {
    rand::srand(2);
    let mut walls = Walls::default();
    for (c, r) in outline(120, 120, 140, 140).into_iter().chain(outline(300, 80, 306, 86)) {
        walls.insert((c, r), Wall::new(c, r));
    }
    let origin = Vec2::new(120.0 * TILE_SIZE, 120.0 * TILE_SIZE);
    //units all over the place, some in the walls' way, some far from any, each having moved a bit and some a lot
    let mut units: Vec<Unit> = Vec::new();
    let mut started: Vec<Vec2> = Vec::new();
    for i in 0..3000 {
        let from = origin + Vec2::new(rand::gen_range(-400.0, 3000.0), rand::gen_range(-400.0, 3000.0)) * if i % 3 == 0 { 1.0 } else { 6.0 };
        let step = Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(0.0, if i % 10 == 0 { 300.0 } else { 40.0 });
        let mut unit = unit_at(from.x + step.x, from.y + step.y);
        unit.garrison = (i % 97 == 0).then_some((130, 130));
        units.push(unit);
        started.push(from);
    }
    let mut expected = units.clone();
    for (unit, &from) in expected.iter_mut().zip(&started) {
        if unit.garrison.is_some() {
            continue;
        }
        let settled = move_clear_of_walls(from, Vec2::new(unit.x, unit.y), &walls);
        unit.x = settled.x;
        unit.y = settled.y;
        clamp_to_world(&mut unit.x, &mut unit.y);
    }
    settle_units(&mut units, &started, 1.0 / 60.0, &walls);
    let worst = units.iter().zip(&expected).map(|(a, b)| (a.x - b.x).abs().max((a.y - b.y).abs())).fold(0.0, f32::max);
    assert!(worst < 0.05, "a unit ended {worst} px from where it should");
    assert!(units.iter().zip(&started).any(|(unit, from)| Vec2::new(unit.x, unit.y).distance(*from) > 1.0), "they did move");
}

#[test]
fn an_army_in_a_formation_is_moved_and_settled_in_one_go_with_the_same_result_as_in_two() {
    rand::srand(9);
    let player = Vec2::new(16000.0, 16000.0);
    let walls = walls_of(&outline(240, 240, 262, 262));
    let field = field_for(player, &walls);
    let squads = Squads::new();
    let mut army: Vec<Unit> = (0..2500).map(|_| unit_at(player.x + rand::gen_range(-900.0, 900.0), player.y + rand::gen_range(-900.0, 900.0))).collect();
    army[5].garrison = Some((250, 250));
    for formation in [Formation::Ring, Formation::Wedge, Formation::Line] {
        let drill = Drill { formation, facing: Vec2::new(0.6, 0.8) };
        let mut together = army.clone();
        let mut apart = army.clone();
        for _ in 0..20 {
            update_units_in(&mut together, player.x, player.y, 1.0 / 60.0, &walls, &field, 1.5, &squads, &drill);
            let started: Vec<Vec2> = apart.iter().map(|unit| Vec2::new(unit.x, unit.y)).collect();
            move_units_in(&mut apart, player.x, player.y, 1.0 / 60.0, &walls, &field, 1.5, &squads, &drill);
            settle_units(&mut apart, &started, 1.0 / 60.0, &walls);
        }
        let worst = together.iter().zip(&apart).map(|(a, b)| (a.x - b.x).abs().max((a.y - b.y).abs()).max((a.velocity - b.velocity).length())).fold(0.0, f32::max);
        assert!(worst < 1e-3, "{formation:?}: {worst}");
        assert!(together.iter().zip(&army).any(|(a, b)| a.x != b.x), "{formation:?} did move");
    }
}
