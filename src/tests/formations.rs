//formations: where each unit stands in a ring, a wedge and a line, and the army walking to its places and staying in them

use super::*;

fn slots(count: usize) -> Vec<u32> {
    rand::srand(5);
    (0..count).map(|_| rand::rand()).collect()
}

fn area_for(army: usize) -> f32 {
    formation_area(army)
}

#[test]
fn the_formations_have_names_that_read_back_and_come_round_in_turn() {
    for formation in FORMATIONS {
        assert_eq!(Formation::from_name(formation.name()), Some(formation));
        assert!(!formation.blurb().is_empty());
    }
    assert_eq!(Formation::from_name("Phalanx"), None);
    let mut formation = Formation::Free;
    for expected in [Formation::Ring, Formation::Wedge, Formation::Line, Formation::Free] {
        formation = formation.next();
        assert_eq!(formation, expected);
    }
}

#[test]
fn a_unit_has_the_same_place_every_time_and_the_shape_turns_with_the_way_it_faces() {
    for formation in [Formation::Ring, Formation::Wedge, Formation::Line] {
        for slot in slots(50) {
            let ahead = formation_offset(formation, slot, 3000, Vec2::X);
            assert_eq!(ahead, formation_offset(formation, slot, 3000, Vec2::X), "{formation:?}");
            //facing a quarter turn round puts the unit a quarter turn round
            let down = formation_offset(formation, slot, 3000, Vec2::Y);
            assert!((down - Vec2::new(-ahead.y, ahead.x)).length() < 1e-2, "{formation:?} {ahead} {down}");
        }
    }
    assert_eq!(formation_offset(Formation::Free, 123, 3000, Vec2::X), Vec2::ZERO, "no shape, no place");
}

#[test]
fn a_ring_is_a_ring_with_a_hole_for_the_player_and_the_units_are_spread_evenly_over_it() {
    let army = 4000;
    let area = area_for(army);
    let outer = (area / (std::f32::consts::PI * (1.0 - FORMATION_RING_HOLE * FORMATION_RING_HOLE))).sqrt();
    let inner = (outer * FORMATION_RING_HOLE).max(FORMATION_MIN_HOLE);
    let outer = (inner * inner + area / std::f32::consts::PI).sqrt();
    let radii: Vec<f32> = slots(6000).into_iter().map(|slot| formation_offset(Formation::Ring, slot, army, Vec2::X).length()).collect();
    assert!(radii.iter().all(|&radius| radius >= inner - 0.01 && radius <= outer + 0.01), "between {inner} and {outer}");
    assert!(inner >= FORMATION_MIN_HOLE);
    //half the area is inside the radius that cuts it in two, so about half the units are
    let half = (inner * inner + (outer * outer - inner * inner) / 2.0).sqrt();
    let inside = radii.iter().filter(|&&radius| radius <= half).count() as f32 / radii.len() as f32;
    assert!((inside - 0.5).abs() < 0.04, "{inside}");
    //a small army still leaves the player room
    let small = formation_offset(Formation::Ring, 0, 3, Vec2::X).length();
    assert!(small >= FORMATION_MIN_HOLE - 0.01, "{small}");
}

#[test]
fn a_wedge_has_its_point_at_the_player_and_a_line_is_a_wall_in_front_of_them() {
    let army = 2000;
    let area = area_for(army);
    let all: Vec<Vec2> = slots(5000).into_iter().map(|slot| formation_offset(Formation::Wedge, slot, army, Vec2::X)).collect();
    let length = (area / FORMATION_WEDGE_WIDTH).sqrt();
    assert!(all.iter().all(|at| at.x <= FORMATION_LEAD + 0.01 && at.x >= FORMATION_LEAD - length - 0.01));
    //it widens towards the back, and never wider than its width allows
    assert!(all.iter().all(|at| at.y.abs() <= (FORMATION_LEAD - at.x) * FORMATION_WEDGE_WIDTH + 0.5), "inside the two edges of the wedge");
    let front: f32 = all.iter().filter(|at| at.x > FORMATION_LEAD - length * 0.25).map(|at| at.y.abs()).fold(0.0, f32::max);
    let back: f32 = all.iter().filter(|at| at.x < FORMATION_LEAD - length * 0.75).map(|at| at.y.abs()).fold(0.0, f32::max);
    assert!(back > front * 2.0, "{front} against {back}");

    let depth = (area / FORMATION_LINE_RATIO).sqrt();
    let wall: Vec<Vec2> = slots(5000).into_iter().map(|slot| formation_offset(Formation::Line, slot, army, Vec2::X)).collect();
    assert!(wall.iter().all(|at| at.x >= FORMATION_LEAD - 0.01 && at.x <= FORMATION_LEAD + depth + 0.01), "in front of the player");
    let width = wall.iter().map(|at| at.y.abs()).fold(0.0, f32::max) * 2.0;
    assert!((width / depth - FORMATION_LINE_RATIO).abs() < 0.3, "{width} wide and {depth} deep");
}

#[test]
fn the_shape_grows_with_the_army_and_units_stay_where_they_were_in_it() {
    //(a unit's place is a share of the shape, so more units make it bigger and leave nobody out of place)
    let small = formation_offset(Formation::Ring, 40_000, 500, Vec2::X).length();
    let big = formation_offset(Formation::Ring, 40_000, 13_000, Vec2::X).length();
    assert!(big > small * 2.0, "{small} against {big}");
    //each unit has the room it was meant to have for an army up to the size that enemies stop growing at, and past it they are packed
    //tighter, the shape growing slower than the army, but still growing
    let per_unit = |army: usize| area_for(army) / army as f32;
    assert!((per_unit(400) - FORMATION_SPACING * FORMATION_SPACING * 0.87).abs() < 1e-3);
    assert!((per_unit(1000) - per_unit(100)).abs() < 1e-3);
    assert!(per_unit(13_000) < per_unit(1000) * 0.5 && area_for(13_000) > area_for(1000) * 2.0 && area_for(20_000) > area_for(13_000));
}

#[test]
fn a_champion_stays_close_to_the_player_in_any_formation_and_free_has_no_places_at_all() {
    let player = Vec2::new(5000.0, 5000.0);
    let champion = champion_unit(champion_named("Juggernaut").unwrap(), player);
    let ordinary = unit_at(player.x, player.y);
    for formation in [Formation::Ring, Formation::Wedge, Formation::Line] {
        let drill = Drill { formation, facing: Vec2::X };
        let guard = drill.goal(&champion, player, 8000).unwrap();
        assert!(guard.distance(player) <= FORMATION_GUARD + 0.01, "{formation:?}");
        assert!(drill.goal(&ordinary, player, 8000).is_some());
    }
    assert!(Drill { formation: Formation::Free, facing: Vec2::X }.goal(&ordinary, player, 8000).is_none());
}

#[test]
fn the_shape_turns_smoothly_to_the_way_the_player_goes_and_stays_put_when_they_stop() {
    let mut facing = Vec2::new(0.0, -1.0);
    //standing still, or too slowly, leaves it alone
    assert_eq!(turn_to_face(facing, Vec2::ZERO, 1.0 / 60.0), facing);
    assert_eq!(turn_to_face(facing, Vec2::new(FORMATION_TURN_MIN_SPEED * 0.5, 0.0), 1.0 / 60.0), facing);
    //going east it turns towards east, a bit each frame, and always by the short way
    let mut last = facing.angle_between(Vec2::X).abs();
    for _ in 0..60 {
        facing = turn_to_face(facing, Vec2::new(800.0, 0.0), 1.0 / 60.0);
        let now = facing.angle_between(Vec2::X).abs();
        assert!(now < last + 1e-4 && (facing.length() - 1.0).abs() < 1e-4);
        last = now;
    }
    assert!(last < 0.1, "a second later it faces the way they go: {last}");
    //directly behind is no problem either
    let turned = turn_to_face(Vec2::X, Vec2::new(-800.0, 0.0), 1.0 / 60.0);
    assert!((turned.length() - 1.0).abs() < 1e-4 && turned.x < 1.0);
}

//an army in a formation, round a player who stands still, for some seconds
fn drill_for(formation: Formation, count: usize, seconds: f32) -> (Vec<Unit>, Vec2, Drill) {
    rand::srand(11);
    let player = Vec2::new(8000.0, 8000.0);
    let walls = Walls::default();
    let field = field_for(player, &walls);
    let drill = Drill { formation, facing: Vec2::new(1.0, 0.0) };
    let mut units: Vec<Unit> = (0..count).map(|i| unit_at(player.x + (i % 40) as f32 * 3.0 - 60.0, player.y + (i / 40) as f32 * 3.0 - 40.0)).collect();
    units.push(champion_unit(champion_named("Medic").unwrap(), player));
    let squads = Squads::new();
    for _ in 0..(seconds * 60.0) as usize {
        update_units_in(&mut units, player.x, player.y, 1.0 / 60.0, &walls, &field, 1.0, &squads, &drill);
    }
    (units, player, drill)
}

#[test]
fn an_army_in_a_formation_walks_to_its_places_and_stays_there() {
    for formation in [Formation::Ring, Formation::Wedge, Formation::Line] {
        let (mut units, player, drill) = drill_for(formation, 600, 12.0);
        let army = units.len();
        let near = units.iter().filter(|unit| drill.goal(unit, player, army).is_some_and(|goal| goal.distance(Vec2::new(unit.x, unit.y)) < 8.0)).count();
        assert!(near as f32 > army as f32 * 0.95, "{formation:?}: {near} of {army} are in their places");
        //and once there they hardly move
        let before: Vec<Vec2> = units.iter().map(|unit| Vec2::new(unit.x, unit.y)).collect();
        let walls = Walls::default();
        let field = field_for(player, &walls);
        let squads = Squads::new();
        for _ in 0..30 {
            update_units_in(&mut units, player.x, player.y, 1.0 / 60.0, &walls, &field, 1.0, &squads, &drill);
        }
        let moved = units.iter().zip(&before).map(|(unit, was)| Vec2::new(unit.x, unit.y).distance(*was)).fold(0.0, f32::max);
        assert!(moved < 4.0, "{formation:?}: someone still moved {moved} px in half a second");
    }
}

#[test]
fn an_army_in_a_ring_is_no_more_bunched_up_than_places_picked_at_random_would_be() {
    //(the places are picked at random over the shape, so a few are close, but nothing like the heap a free army makes of itself)
    let (units, _, _) = drill_for(Formation::Ring, 2000, 12.0);
    let stacked = units.iter().enumerate().filter(|(i, unit)| units.iter().enumerate().any(|(j, other)| *i != j && (other.x - unit.x).hypot(other.y - unit.y) < 3.0)).count();
    let share = stacked as f32 / units.len() as f32;
    assert!(share < 0.25, "{:.0}% of the units are within 3 px of another", share * 100.0);
}

#[test]
fn units_far_behind_their_place_hurry_to_catch_up() {
    let player = Vec2::new(8000.0, 8000.0);
    let walls = Walls::default();
    let field = field_for(player, &walls);
    let squads = Squads::new();
    let drill = Drill { formation: Formation::Ring, facing: Vec2::X };
    let travelled = |far: f32| {
        let mut units = vec![unit_at(player.x - far, player.y)];
        let start = Vec2::new(units[0].x, units[0].y);
        for _ in 0..30 {
            update_units_in(&mut units, player.x, player.y, 1.0 / 60.0, &walls, &field, 1.0, &squads, &drill);
        }
        Vec2::new(units[0].x, units[0].y).distance(start)
    };
    assert!(travelled(1500.0) > travelled(150.0) * 1.15, "{} against {}", travelled(1500.0), travelled(150.0));
}
