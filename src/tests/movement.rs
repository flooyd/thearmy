//walls, flow field paths, dragging out walls, units walking, crowds and spreading

use super::*;


pub(super) fn walls_of(tiles: &[(i32, i32)]) -> Walls {
    tiles.iter().map(|&(c, r)| ((c, r), Wall::new(c, r))).collect()
}

//the 8 squares around a square
pub(super) fn ring(c: i32, r: i32) -> Vec<(i32, i32)> {
    NEIGHBORS.iter().map(|&(dc, dr)| (c + dc, r + dr)).collect()
}

//the squares on the edge of the box of squares with these corners
pub(super) fn outline(first_col: i32, first_row: i32, last_col: i32, last_row: i32) -> Vec<(i32, i32)> {
    let mut tiles = Vec::new();
    for c in first_col..=last_col {
        for r in first_row..=last_row {
            if c == first_col || c == last_col || r == first_row || r == last_row {
                tiles.push((c, r));
            }
        }
    }
    tiles
}

pub(super) fn field_for(player: Vec2, walls: &Walls) -> FlowField {
    let mut field = FlowField::new();
    field.compute(player, walls);
    field
}

//walk a unit towards a fixed player for `frames` frames of dt seconds, checking it never overlaps a wall.
//returns where it ended and the frame it got within UNIT_FOLLOW_DISTANCE of the player (if it did)
pub(super) fn simulate(walls: &Walls, start: Vec2, player: Vec2, frames: usize, dt: f32) -> (Vec2, Option<usize>) {
    let field = field_for(player, walls);
    let mut unit = Unit::new(start.x, start.y, 150.0);
    let mut reached = None;
    for frame in 0..frames {
        step_unit(&mut unit, player, dt, walls, &field, 1.0);
        push_out_of_walls(&mut unit.x, &mut unit.y, walls);
        clamp_to_world(&mut unit.x, &mut unit.y);
        assert!(
            !touches_wall(unit.x, unit.y, RADIUS - 0.05, walls),
            "unit inside a wall at ({},{}) on frame {frame}",
            unit.x,
            unit.y
        );
        if reached.is_none() && Vec2::new(unit.x, unit.y).distance(player) <= UNIT_FOLLOW_DISTANCE {
            reached = Some(frame);
        }
    }
    (Vec2::new(unit.x, unit.y), reached)
}

pub(super) fn dist_to_nearest_wall(p: Vec2, walls: &Walls) -> f32 {
    walls.values().map(|w| (p - w.nearest_point(p.x, p.y)).length()).fold(f32::MAX, f32::min)
}

#[test]
fn goes_round_a_long_wall() {
    let tiles: Vec<_> = (5..=15).map(|r| (20, r)).collect();
    let walls = walls_of(&tiles);
    let (_, reached) = simulate(&walls, tile_center((15, 10)), tile_center((25, 10)), 2500, 1.0 / 60.0);
    let frame = reached.expect("never reached the player");
    println!("round a wall: reached after {frame} frames ({:.1}s)", frame as f32 / 60.0);
}

#[test]
fn goes_into_a_cup_from_behind_via_the_opening() {
    // cup opening to the east: back wall col 20 rows 8..=12, arms along rows 8 and 12 to col 26
    let mut tiles: Vec<_> = (8..=12).map(|r| (20, r)).collect();
    tiles.extend((21..=26).map(|c| (c, 8)));
    tiles.extend((21..=26).map(|c| (c, 12)));
    let walls = walls_of(&tiles);
    let (_, reached) = simulate(&walls, tile_center((15, 10)), tile_center((23, 10)), 3000, 1.0 / 60.0);
    let frame = reached.expect("never reached the player inside the cup");
    println!("into the cup: reached after {frame} frames");
}

#[test]
fn sealed_in_player_units_press_against_the_wall_and_stop() {
    let walls = walls_of(&ring(25, 10));
    let player = tile_center((25, 10));
    for start in [(15, 10), (15, 3), (30, 3), (25, 20), (35, 10), (18, 16)] {
        let (end, reached) = simulate(&walls, tile_center(start), player, 4000, 1.0 / 60.0);
        assert!(reached.is_none(), "reached a sealed in player from {start:?}");
        let d = end.distance(player);
        let wall_gap = dist_to_nearest_wall(end, &walls);
        // pressed against the wall (circle touching it) and as close as a circle can get: the ring is 1.5 squares
        // from the player's center so the closest a unit can be is 96 + 16
        assert!((wall_gap - RADIUS).abs() < 0.6, "from {start:?}: not touching the wall, gap {wall_gap}");
        assert!(d <= 112.0 + 1.0, "from {start:?}: stopped {d} from the player, closest possible is 112");
        println!("sealed in, from {start:?}: stopped {d:.1} from the player, {wall_gap:.2} from the wall");
    }
}

#[test]
fn a_symmetric_house_does_not_send_units_the_long_way_round() {
    // every side of the house is as close as every other, so a unit that arrives at one shouldn't go to another
    let walls = walls_of(&ring(25, 10));
    let player = tile_center((25, 10));
    let (east, _) = simulate(&walls, tile_center((35, 10)), player, 4000, 1.0 / 60.0);
    assert!(east.x > player.x + 50.0 && (east.y - player.y).abs() < 2.0, "ended at {east:?}");
    let (north, _) = simulate(&walls, tile_center((25, 0)), player, 4000, 1.0 / 60.0);
    assert!(north.y < player.y - 50.0 && (north.x - player.x).abs() < 2.0, "ended at {north:?}");
}

#[test]
fn sealed_in_unit_stops_moving() {
    let walls = walls_of(&ring(25, 10));
    let player = tile_center((25, 10));
    let field = field_for(player, &walls);
    let start = tile_center((15, 4));
    let mut unit = Unit::new(start.x, start.y, 150.0);
    let mut history = Vec::new();
    for _ in 0..3000 {
        step_unit(&mut unit, player, 1.0 / 60.0, &walls, &field, 1.0);
        push_out_of_walls(&mut unit.x, &mut unit.y, &walls);
        history.push(Vec2::new(unit.x, unit.y));
    }
    let last = *history.last().unwrap();
    let moved = history[history.len() - 300..].iter().map(|p| p.distance(last)).fold(0.0, f32::max);
    assert!(moved < 0.5, "still moving by {moved} px over the last 300 frames");
    println!("sealed in: wandered {moved:.4} px over the last 5 seconds");
}

#[test]
fn units_walk_to_the_closest_point_of_a_big_house_the_player_is_off_centre_in() {
    // an 11x11 house, the player is 2 squares in from the west wall. its closest outside point is level with the player
    // against the west wall, whichever side the unit arrives at
    let walls = walls_of(&outline(20, 10, 30, 20));
    let player = tile_center((22, 15));
    let wall_x = 20.0 * TILE_SIZE; // west face of the house
    let closest = player.distance(Vec2::new(wall_x, player.y)) + RADIUS;
    for start in [(10, 15), (10, 5), (25, 5), (36, 5), (36, 15), (36, 25), (25, 26), (10, 26), (21, 8), (31, 12)] {
        let (end, reached) = simulate(&walls, tile_center(start), player, 8000, 1.0 / 60.0);
        assert!(reached.is_none());
        let d = end.distance(player);
        println!("big house, from {start:?}: stopped {d:.1} from the player, closest possible is {closest:.1}");
        assert!(d <= closest + 3.0, "from {start:?}: stopped {d} away, closest possible is {closest}");
        assert!(end.x < wall_x, "from {start:?}: ended up at {end:?}, not against the west wall");
    }
}

#[test]
fn a_unit_walled_in_gets_as_close_as_its_walls_allow() {
    // a one square cell with the unit in it, the player is to the east, so it should end up against the east wall
    let walls = walls_of(&ring(10, 10));
    let player = tile_center((20, 10));
    let (end, reached) = simulate(&walls, tile_center((10, 10)), player, 3000, 1.0 / 60.0);
    assert!(reached.is_none());
    let east_inner_face = 11.0 * TILE_SIZE;
    assert!((end.x - (east_inner_face - RADIUS)).abs() < 1.0, "ended at {end:?}");
    assert!((end.y - player.y).abs() < 2.0, "ended at {end:?}");
}

#[test]
fn a_unit_in_a_dead_end_pocket_does_not_stay_in_it() {
    // a wall between unit and player that can be walked round: the unit mustn't treat the near side as cut off
    let tiles: Vec<_> = (3..=17).map(|r| (20, r)).collect();
    let walls = walls_of(&tiles);
    let (_, reached) = simulate(&walls, tile_center((15, 10)), tile_center((25, 10)), 4000, 1.0 / 60.0);
    assert!(reached.is_some());
}

#[test]
fn several_units_sealed_out_do_not_get_into_the_wall() {
    // five units all pressing at the same house, with the same spreading and pushing the game does
    let walls = walls_of(&ring(25, 10));
    let player = tile_center((25, 10));
    let field = field_for(player, &walls);
    let starts = [(15, 10), (16, 10), (15, 11), (16, 9), (17, 10)];
    let mut units: Vec<Unit> =
        starts.iter().map(|&t| Unit::new(tile_center(t).x, tile_center(t).y, 150.0)).collect();
    for _ in 0..4000 {
        move_units(&mut units, player.x, player.y, 1.0 / 60.0, &walls, &field, 1.0, &Squads::new());
        spread_units(&mut units);
        for e in units.iter_mut() {
            push_out_of_walls(&mut e.x, &mut e.y, &walls);
            clamp_to_world(&mut e.x, &mut e.y);
        }
    }
    for e in &units {
        assert!(!touches_wall(e.x, e.y, RADIUS - 0.6, &walls), "unit inside a wall at ({},{})", e.x, e.y);
        let d = Vec2::new(e.x, e.y).distance(player);
        println!("crowd at a sealed house: unit at {d:.1} from the player");
        assert!(d > 16.0);
    }
}

#[test]
fn a_doorway_is_found() {
    let mut tiles = ring(25, 10);
    tiles.retain(|&t| t != (26, 10)); // door on the east side, the unit starts on the west
    let walls = walls_of(&tiles);
    let (_, reached) = simulate(&walls, tile_center((15, 10)), tile_center((25, 10)), 3000, 1.0 / 60.0);
    let frame = reached.expect("did not find the doorway");
    println!("doorway: reached after {frame} frames");
}

#[test]
fn closing_the_doorway_makes_units_stop_at_the_wall() {
    // the same house with the door, then the door is walled up and the field is rebuilt
    let mut tiles = ring(25, 10);
    tiles.retain(|&t| t != (26, 10));
    let mut walls = walls_of(&tiles);
    let player = tile_center((25, 10));
    let mut field = field_for(player, &walls);
    let mut unit = Unit::new(tile_center((15, 10)).x, tile_center((15, 10)).y, 150.0);
    for _ in 0..200 {
        step_unit(&mut unit, player, 1.0 / 60.0, &walls, &field, 1.0);
    }
    walls.insert((26, 10), Wall::new(26, 10));
    field.compute(player, &walls);
    for _ in 0..4000 {
        step_unit(&mut unit, player, 1.0 / 60.0, &walls, &field, 1.0);
        push_out_of_walls(&mut unit.x, &mut unit.y, &walls);
    }
    let d = Vec2::new(unit.x, unit.y).distance(player);
    assert!(d > 100.0 && d < 125.0, "stopped {d} away");
}

#[test]
fn a_long_detour_still_beats_giving_up_at_a_wall() {
    // a wall that spans most of the field: the way round is long but exists, so the unit must not give up at it
    let tiles: Vec<_> = (0..=60).map(|r| (40, r)).collect();
    let walls = walls_of(&tiles); // rows 0..=60 at col 40, the way round is below row 60
    let (_, reached) = simulate(&walls, tile_center((35, 30)), tile_center((45, 30)), 6000, 1.0 / 60.0);
    assert!(reached.is_some(), "gave up instead of taking the long way round");
}

#[test]
fn diagonal_pinholes_are_not_squeezed_through() {
    // a staircase wall (every square diagonal to the next) with no orthogonal gaps, long enough to go round
    let tiles: Vec<_> = (0..=14).map(|i| (10 + i, 10 + i)).collect();
    let walls = walls_of(&tiles);
    let (_, reached) = simulate(&walls, tile_center((14, 18)), tile_center((18, 12)), 6000, 1.0 / 60.0);
    assert!(reached.is_some(), "did not get round the staircase wall");
}

#[test]
fn corner_cutting_is_refused() {
    let walls = walls_of(&[(11, 10)]);
    let f = field_for(tile_center((20, 20)), &walls);
    // moving diagonally from (10,10) to (11,11) would cut the corner of the wall at (11,10)
    assert!(!f.can_step(10, 10, 1, 1));
    assert!(f.can_step(10, 12, 1, 1));
}

#[test]
fn field_follows_world_edges() {
    let walls = walls_of(&[]);
    for target in [(0, 0), (WORLD_TILES - 1, WORLD_TILES - 1), (0, WORLD_TILES - 1), (250, 0)] {
        let f = field_for(tile_center(target), &walls);
        assert_eq!(f.cost[f.index(target.0, target.1).unwrap()], 0);
        // a unit a few squares away walks onto the target
        let mut tile = (
            target.0 + if target.0 == 0 { 5 } else { -5 },
            target.1 + if target.1 == 0 { 5 } else { -5 },
        );
        for _ in 0..20 {
            match f.next_tile(tile) {
                Some(next) => tile = next,
                None => break,
            }
        }
        assert_eq!(tile, target, "did not walk to {target:?}");
    }
    // far outside the field there is no route
    let f = field_for(tile_center((250, 250)), &walls);
    assert_eq!(f.next_tile((250 + FIELD_RADIUS + 5, 250)), None);
}

#[test]
fn every_open_square_has_a_cost_and_walls_have_none() {
    let mut tiles = outline(240, 240, 260, 260);
    tiles.extend(outline(246, 246, 254, 254)); // a house inside a house
    let walls = walls_of(&tiles);
    let f = field_for(tile_center((250, 250)), &walls);
    for (i, &cost) in f.cost.iter().enumerate() {
        assert_eq!(cost == u32::MAX, f.wall[i], "square {i}: cost {cost}, wall {}", f.wall[i]);
    }
}

#[test]
fn frame_rate_does_not_change_how_far_units_walk() {
    let walls = walls_of(&[]);
    let player = tile_center((60, 10));
    let field = field_for(player, &walls);
    let at = |dt: f32, frames: usize| {
        let mut e = Unit::new(200.0, 640.0, 150.0);
        for _ in 0..frames {
            step_unit(&mut e, player, dt, &walls, &field, 1.0);
        }
        e.x - 200.0
    };
    let a = at(1.0 / 60.0, 60);
    let b = at(1.0 / 120.0, 120);
    let c = at(1.0 / 240.0, 240);
    println!("distance walked in 1 s: 60fps {a:.2}, 120fps {b:.2}, 240fps {c:.2}");
    assert!((a - 150.0).abs() < 0.5 && (b - 150.0).abs() < 0.5 && (c - 150.0).abs() < 0.5);
}

#[test]
fn walls_cannot_be_placed_on_a_unit() {
    let mut walls = Walls::default();
    let units = [Unit::new(330.0, 330.0, 150.0)]; // inside tile (5,5)
    assert!(!place_wall(&mut walls, WallKind::Wall, WALL_HEALTH, 5, 5, 16000.0, 16000.0, &units, &[]));
    assert!(place_wall(&mut walls, WallKind::Wall, WALL_HEALTH, 9, 9, 16000.0, 16000.0, &units, &[]));
    assert_eq!(walls.len(), 1);
}

#[test]
fn a_circle_whose_center_is_inside_a_wall_is_pushed_out() {
    let walls = walls_of(&[(10, 10)]); // 640..704
    for start in [(672.0, 672.0), (650.0, 672.0), (672.0, 698.0), (700.0, 645.0)] {
        let (mut x, mut y) = start;
        push_out_of_walls(&mut x, &mut y, &walls);
        assert!(!touches_wall(x, y, RADIUS - 0.01, &walls), "{start:?} -> ({x},{y}) still overlapping");
    }
}

#[test]
fn building_the_field_in_slices_gives_the_same_field() {
    let mut walls = Walls::default();
    let mut seed = 99u32;
    for _ in 0..1500 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let c = 200 + (seed >> 16) as i32 % 100;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let r = 200 + (seed >> 16) as i32 % 100;
        walls.insert((c, r), Wall::new(c, r));
    }
    // a house round the player so there is an area that can't reach the player to work out too
    for tile in outline(244, 244, 256, 256) {
        walls.insert(tile, Wall::new(tile.0, tile.1));
    }
    walls.remove(&(250, 250));
    let player = Vec2::new(250.0 * TILE_SIZE + 20.0, 250.0 * TILE_SIZE + 40.0);
    let whole = field_for(player, &walls);

    let mut sliced = FlowField::new();
    sliced.start(player, &walls);
    assert!(sliced.is_building());
    let (mut slices, mut longest) = (0, Duration::ZERO);
    loop {
        let t = Instant::now();
        let done = sliced.work(FIELD_BUDGET);
        longest = longest.max(t.elapsed());
        slices += 1;
        if done {
            break;
        }
    }
    assert!(!sliced.is_building());
    assert_eq!(whole.cost, sliced.cost);
    assert_eq!(whole.wall, sliced.wall);
    println!(
        "building in slices of {:?}: {slices} slices, longest slice {:.2} ms (debug build)",
        FIELD_BUDGET,
        longest.as_secs_f64() * 1000.0
    );
}

#[test]
fn timing_of_the_field_in_a_debug_build() {
    // 4000 scattered walls inside the field, the worst case for a recompute, with a house round the player
    let mut walls = Walls::default();
    let mut seed = 12345u32;
    for _ in 0..4000 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let c = 190 + (seed >> 16) as i32 % 120;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let r = 190 + (seed >> 16) as i32 % 120;
        walls.insert((c, r), Wall::new(c, r));
    }
    for sealed in [false, true] {
        if sealed {
            for tile in outline(247, 247, 253, 253) {
                walls.insert(tile, Wall::new(tile.0, tile.1));
            }
        }
        let mut f = FlowField::new();
        let t = std::time::Instant::now();
        let n = 20;
        for i in 0..n {
            f.compute(tile_center((250 + i % 2, 250)), &walls);
        }
        let per = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
        println!(
            "flow field recompute with {} walls, player sealed in: {sealed}: {per:.2} ms each (debug build)",
            walls.len()
        );
    }
    let f = field_for(tile_center((250, 250)), &walls);
    let unit = Vec2::new(250.0 * 64.0 - 1500.0, 250.0 * 64.0);
    let t = std::time::Instant::now();
    let n = 200;
    for _ in 0..n {
        std::hint::black_box(unit_steer(unit, tile_center((250, 250)), &walls, &f));
    }
    let per = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    println!("unit_steer: {per:.3} ms per unit per frame (debug build)");
}

// ---- dragging ----

//every square on the way from one square to another along a row or column, one at a time like a slow mouse
pub(super) fn walk(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let (dc, dr) = ((to.0 - from.0).signum(), (to.1 - from.1).signum());
    let mut path = vec![from];
    while *path.last().unwrap() != to {
        let (c, r) = *path.last().unwrap();
        path.push((c + dc, r + dr));
    }
    path
}

//the squares a drag covers when the mouse is pressed on the first square and then visits the rest in turn
pub(super) fn drag_over(path: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut drag = Drag::new(path[0]);
    let mut covered = Vec::new();
    for &square in path {
        covered.extend(drag.reach(square));
    }
    covered.sort();
    covered.dedup();
    covered
}

pub(super) fn sorted(mut tiles: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    tiles.sort();
    tiles.dedup();
    tiles
}

#[test]
fn a_whole_square_can_be_dragged_without_letting_go() {
    let mut path = walk((0, 0), (9, 0));
    path.extend(walk((9, 0), (9, 9)));
    path.extend(walk((9, 9), (0, 9)));
    path.extend(walk((0, 9), (0, 1)));
    assert_eq!(drag_over(&path), sorted(outline(0, 0, 9, 9)));
}

#[test]
fn a_fast_mouse_that_skips_squares_still_makes_a_square() {
    // the mouse only lands on the corners and a square or two between
    let path = [(0, 0), (4, 0), (9, 0), (9, 5), (9, 9), (3, 9), (0, 9), (0, 1)];
    assert_eq!(drag_over(&path), sorted(outline(0, 0, 9, 9)));
}

#[test]
fn turning_the_other_way_round_works_too() {
    let mut path = walk((5, 5), (5, 8));
    path.extend(walk((5, 8), (9, 8)));
    path.extend(walk((9, 8), (9, 3)));
    let mut expected = Vec::new();
    for r in 5..=8 {
        expected.push((5, r));
    }
    for c in 5..=9 {
        expected.push((c, 8));
    }
    for r in 3..=8 {
        expected.push((9, r));
    }
    assert_eq!(drag_over(&path), sorted(expected));
}

#[test]
fn diagonal_movement_builds_nothing() {
    assert_eq!(drag_over(&[(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)]), vec![(0, 0)]);
    assert_eq!(drag_over(&[(5, 5), (4, 6), (3, 7), (2, 8)]), vec![(5, 5)]);
}

#[test]
fn a_slow_diagonal_does_not_build_a_staircase() {
    // a mouse going diagonally slowly lands on the squares either side of the diagonal as well
    let path = [(0, 0), (1, 0), (1, 1), (2, 1), (2, 2), (3, 2), (3, 3), (4, 3), (4, 4)];
    assert_eq!(drag_over(&path), vec![(0, 0), (1, 0)]);
    let other_way = [(0, 0), (0, 1), (1, 1), (1, 2), (2, 2), (2, 3)];
    assert_eq!(drag_over(&other_way), vec![(0, 0), (0, 1)]);
}

#[test]
fn drifting_off_the_line_and_back_still_builds_the_line() {
    let path = [(0, 0), (1, 0), (2, 1), (3, 1), (4, 0), (5, 0)];
    let expected: Vec<_> = (0..=5).map(|c| (c, 0)).collect();
    assert_eq!(drag_over(&path), expected);
}

#[test]
fn a_turn_needs_a_few_squares_first() {
    // one square along and straight down is treated like the start of a diagonal
    assert_eq!(drag_over(&[(0, 0), (1, 0), (1, 1), (1, 2)]), vec![(0, 0), (1, 0)]);
    // but two squares is a deliberate side
    assert_eq!(
        drag_over(&[(0, 0), (1, 0), (2, 0), (2, 1), (2, 2)]),
        vec![(0, 0), (1, 0), (2, 0), (2, 1), (2, 2)]
    );
    // the first side doesn't have to be two squares if it is vertical first then along, the rule is the same
    assert_eq!(drag_over(&[(0, 0), (0, 1), (1, 1), (2, 1)]), vec![(0, 0), (0, 1)]);
}

#[test]
fn staying_on_a_square_or_going_back_does_not_upset_a_turn() {
    let path = [(0, 0), (3, 0), (3, 0), (3, 0), (1, 0), (1, 0), (1, 4), (1, 4)];
    let mut expected: Vec<_> = (0..=3).map(|c| (c, 0)).collect();
    expected.extend((1..=4).map(|r| (1, r)));
    assert_eq!(drag_over(&path), sorted(expected));
}

#[test]
fn a_drag_that_never_moves_places_one_square() {
    assert_eq!(drag_over(&[(7, 7), (7, 7), (7, 7)]), vec![(7, 7)]);
}

// ---- sliding along walls ----

//press a circle into a wall at an angle for `frames` frames, like holding two keys, and see how far it gets along it
pub(super) fn slide(walls: &Walls, start: Vec2, step: Vec2, frames: usize) -> Vec2 {
    let (mut x, mut y) = (start.x, start.y);
    for _ in 0..frames {
        x += step.x;
        y += step.y;
        push_out_of_walls(&mut x, &mut y, walls);
    }
    Vec2::new(x, y)
}

#[test]
fn sliding_along_a_straight_wall_does_not_catch_where_the_squares_join() {
    // 11 squares in a row, pressed into at 45 degrees from every side. 200 frames of 2.5 along the wall is 500
    let vertical = walls_of(&(10..=20).map(|r| (20, r)).collect::<Vec<_>>()); // x 1280..1344, y 640..1344
    let horizontal = walls_of(&(10..=20).map(|c| (c, 20)).collect::<Vec<_>>()); // y 1280..1344, x 640..1344
    let cases = [
        ("west face, going down", &vertical, Vec2::new(1264.0, 700.0), Vec2::new(2.5, 2.5)),
        ("west face, going up", &vertical, Vec2::new(1264.0, 1300.0), Vec2::new(2.5, -2.5)),
        ("east face, going down", &vertical, Vec2::new(1360.0, 700.0), Vec2::new(-2.5, 2.5)),
        ("north face, going right", &horizontal, Vec2::new(700.0, 1264.0), Vec2::new(2.5, 2.5)),
        ("north face, going left", &horizontal, Vec2::new(1300.0, 1264.0), Vec2::new(-2.5, 2.5)),
        ("south face, going right", &horizontal, Vec2::new(700.0, 1360.0), Vec2::new(2.5, -2.5)),
    ];
    for (name, walls, start, step) in cases {
        let end = slide(walls, start, step, 200);
        let travelled = (end - start).abs();
        let along_the_wall = if name.contains("west") || name.contains("east") { travelled.y } else { travelled.x };
        println!("{name}: slid {along_the_wall:.1} of 500");
        assert!(along_the_wall > 495.0, "{name}: caught after {along_the_wall}");
    }
}

#[test]
fn the_start_is_a_ten_by_ten_house_with_the_player_in_the_middle() {
    let walls = starting_house();
    assert_eq!(walls.len(), 10 * 4 - 4);
    let cols = walls.keys().map(|t| t.0);
    let rows = walls.keys().map(|t| t.1);
    let (first_col, last_col) = (cols.clone().min().unwrap(), cols.max().unwrap());
    let (first_row, last_row) = (rows.clone().min().unwrap(), rows.max().unwrap());
    assert_eq!((last_col - first_col + 1, last_row - first_row + 1), (10, 10));
    let middle_x = (first_col + last_col + 1) as f32 * TILE_SIZE / 2.0;
    let middle_y = (first_row + last_row + 1) as f32 * TILE_SIZE / 2.0;
    assert_eq!((middle_x, middle_y), (WORLD_SIZE / 2.0, WORLD_SIZE / 2.0));
    // hollow, with nothing in the way of the player
    assert!(!walls.contains_key(&(first_col + 1, first_row + 1)));
    assert!(!touches_wall(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0, RADIUS, &walls));
    // sealed: no open square inside reaches the outside
    let f = field_for(Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0), &walls);
    let outside = f.index(first_col - 1, first_row).unwrap();
    assert!(f.cost[outside] >= CUT_OFF_STEP, "the house has a gap");
}

#[test]
fn units_from_the_random_spawns_end_up_against_the_starting_house() {
    let walls = starting_house();
    let player = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);
    let field = field_for(player, &walls);
    for seed in 1..=6u64 {
        rand::srand(seed);
        let mut units: Vec<Unit> = (0..5)
            .map(|_| {
                let p = far_position(player.x, player.y);
                Unit::new(p.x, p.y, 150.0)
            })
            .collect();
        for _ in 0..(60 * 100) {
            move_units(&mut units, player.x, player.y, 1.0 / 60.0, &walls, &field, 1.0, &Squads::new());
            spread_units(&mut units);
            for e in units.iter_mut() {
                push_out_of_walls(&mut e.x, &mut e.y, &walls);
                clamp_to_world(&mut e.x, &mut e.y);
            }
        }
        let distances: Vec<f32> = units.iter().map(|e| Vec2::new(e.x, e.y).distance(player)).collect();
        println!("seed {seed}: units stopped {distances:.0?} from the player");
        for (e, d) in units.iter().zip(&distances) {
            assert!(!touches_wall(e.x, e.y, RADIUS - 0.6, &walls));
            // the wall is 5 squares from the player, so 320 + 16 is as close as it goes, with up to 5 units in a row
            // on one side sharing that
            assert!(*d >= 335.0 && *d < 350.0, "seed {seed}: unit stopped {d} from the player");
        }
    }
}

// ---- big crowds ----

//whether a centre is within the outside edge of the starting house
pub(super) fn in_the_house(e: &Unit) -> bool {
    let (low, high) = (245.0 * TILE_SIZE, 255.0 * TILE_SIZE);
    e.x > low && e.x < high && e.y > low && e.y < high
}

pub(super) fn run_crowd(count: usize, speed: f32, fps: f32, seconds: f32, seed: u64) -> (usize, usize, f32) {
    let walls = starting_house();
    let player = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);
    let field = field_for(player, &walls);
    rand::srand(seed);
    let mut units: Vec<Unit> = (0..count)
        .map(|_| {
            let p = far_position(player.x, player.y);
            Unit::new(p.x, p.y, speed)
        })
        .collect();
    let mut worst_inside = 0;
    let mut ever_inside = vec![false; count];
    for _ in 0..(seconds * fps) as usize {
        update_units(&mut units, player.x, player.y, 1.0 / fps, &walls, &field, 1.0, &Squads::new());
        let inside = units.iter().filter(|e| in_the_house(e)).count();
        worst_inside = worst_inside.max(inside);
        for (i, e) in units.iter().enumerate() {
            ever_inside[i] |= in_the_house(e);
        }
    }
    let in_a_wall = units.iter().filter(|e| touches_wall(e.x, e.y, RADIUS - 2.0, &walls)).count();
    (worst_inside, in_a_wall, ever_inside.iter().filter(|&&b| b).count() as f32)
}

#[test]
fn a_big_crowd_does_not_get_into_the_starting_house() {
    for (count, speed, fps) in [(500, 400.0, 120.0), (500, 400.0, 60.0), (100, 400.0, 120.0), (500, 150.0, 60.0)] {
        let t = Instant::now();
        let (worst_inside, in_a_wall, ever) = run_crowd(count, speed, fps, 40.0, 7);
        println!(
            "{count} units at {speed} px/s, {fps} fps: at worst {worst_inside} inside the house at once, {ever} got in at some point, {in_a_wall} left overlapping a wall ({:.1}s to run)",
            t.elapsed().as_secs_f32()
        );
        assert_eq!(worst_inside, 0, "{count} units at {speed} px/s {fps} fps got into the house");
    }
}


#[test]
fn a_huge_step_does_not_carry_a_circle_through_a_wall() {
    // the west wall of the starting house is x 15680..15744, 64 thick. moving 90 px at once from outside would put the
    // centre well past the middle of it
    let walls = starting_house();
    let from = Vec2::new(15660.0 + SHIFT, 15900.0 + SHIFT);
    for distance in [20.0, 40.0, 60.0, 90.0, 140.0] {
        let end = move_clear_of_walls(from, from + Vec2::new(distance, 0.0), &walls);
        assert!(end.x <= 15680.0 + SHIFT - RADIUS + 0.01, "a step of {distance} ended at x {}", end.x);
        assert!(!touches_wall(end.x, end.y, RADIUS - 0.01, &walls));
    }
    // and diagonally at the corner of the house, and into the seam between two squares of the wall
    let corner = move_clear_of_walls(Vec2::new(15660.0 + SHIFT, 15660.0 + SHIFT), Vec2::new(15740.0 + SHIFT, 15740.0 + SHIFT), &walls);
    assert!(corner.x < 15680.0 + SHIFT || corner.y < 15680.0 + SHIFT, "went through the corner: {corner:?}");
    assert!(!touches_wall(corner.x, corner.y, RADIUS - 0.01, &walls));
}

#[test]
fn the_player_at_full_speed_cannot_get_into_the_house_either() {
    let walls = starting_house();
    // a long hitch at 800 px/s: 0.05 s is 40 px
    let mut p = Vec2::new(15600.0 + SHIFT, 16000.0 + SHIFT);
    for _ in 0..200 {
        p = move_clear_of_walls(p, p + Vec2::new(40.0, 0.0), &walls);
    }
    assert!(p.x <= 15680.0 + SHIFT - RADIUS + 0.01, "got to x {}", p.x);
}

#[test]
fn a_centre_inside_a_wall_next_to_a_seam_leaves_the_way_it_came() {
    // inside the west wall at (245,249), 17 px in from the west face and only 13 px from the seam with (245,248)
    let walls = starting_house();
    let (mut x, mut y) = (15697.0 + SHIFT, 15949.0 + SHIFT);
    push_out_of_walls(&mut x, &mut y, &walls);
    assert!(!touches_wall(x, y, RADIUS - 0.01, &walls), "still in the wall at ({x},{y})");
    assert!(x < 15680.0 + SHIFT, "left through the inside, at ({x},{y})");
}

#[test]
fn a_centre_deep_in_a_wall_with_walls_on_every_side_still_gets_out_somewhere() {
    let mut tiles = Vec::new();
    for c in 9..=11 {
        for r in 9..=11 {
            tiles.push((c, r));
        }
    }
    let walls = walls_of(&tiles);
    let (mut x, mut y) = (10.5 * TILE_SIZE, 10.5 * TILE_SIZE);
    push_out_of_walls(&mut x, &mut y, &walls);
    // the middle square of a solid block has no way out in one push, it must at least have moved
    assert!((x - 10.5 * TILE_SIZE).abs() + (y - 10.5 * TILE_SIZE).abs() > 0.0);
}

#[test]
fn stress_crowds_beyond_what_the_game_uses() {
    for (count, speed, fps, seconds, seed) in [
        (2000, 800.0, 30.0, 30.0, 1),
        (500, 400.0, 20.0, 30.0, 2),
        (1000, 1000.0, 120.0, 30.0, 3),
        (500, 400.0, 240.0, 20.0, 4),
        (500, 400.0, 120.0, 100.0, 5),
    ] {
        let t = Instant::now();
        let (worst_inside, in_a_wall, ever) = run_crowd(count, speed, fps, seconds, seed);
        println!(
            "{count} units at {speed} px/s, {fps} fps for {seconds}s: at worst {worst_inside} inside the house, {ever} ever, {in_a_wall} overlapping a wall at the end ({:.1}s to run)",
            t.elapsed().as_secs_f32()
        );
        assert_eq!(worst_inside, 0);
    }
}

#[test]
fn where_the_frame_time_goes() {
    let walls = starting_house();
    let player = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);
    let field = field_for(player, &walls);
    for count in [500, 1000, 2000, 4000] {
        rand::srand(11);
        let mut units: Vec<Unit> = (0..count)
            .map(|_| {
                let p = far_position(player.x, player.y);
                Unit::new(p.x, p.y, 400.0)
            })
            .collect();
        // two states: while walking in, and piled up against the house
        for (state, seconds) in [("walking in", 3.0), ("piled up", 40.0)] {
            let frames = (seconds * 120.0) as usize;
            let start_frame = if state == "walking in" { 0 } else { 1 };
            let _ = start_frame;
            for _ in 0..frames {
                update_units(&mut units, player.x, player.y, 1.0 / 120.0, &walls, &field, 1.0, &Squads::new());
            }
            let n = 30;
            let (mut t_move, mut t_spread, mut t_settle) = (0.0, 0.0, 0.0);
            for _ in 0..n {
                let started: Vec<Vec2> = units.iter().map(|e| Vec2::new(e.x, e.y)).collect();
                let t = Instant::now();
                move_units(&mut units, player.x, player.y, 1.0 / 120.0, &walls, &field, 1.0, &Squads::new());
                t_move += t.elapsed().as_secs_f64();
                let t = Instant::now();
                spread_units(&mut units);
                t_spread += t.elapsed().as_secs_f64();
                let t = Instant::now();
                for (e, &from) in units.iter_mut().zip(&started) {
                    let s = move_clear_of_walls(from, Vec2::new(e.x, e.y), &walls);
                    e.x = s.x;
                    e.y = s.y;
                }
                t_settle += t.elapsed().as_secs_f64();
            }
            let ms = |t: f64| t * 1000.0 / n as f64;
            println!(
                "{count} units {state}: move {:.2} ms, spread {:.2} ms, wall settling {:.2} ms",
                ms(t_move), ms(t_spread), ms(t_settle)
            );
        }
    }
}

// ---- the grid for spreading, and the steering throttle ----

//the spreading the game used before the grid: every unit against every other
pub(super) fn spread_every_pair(units: &mut Vec<Unit>) {
    let min_distance = 32.0;
    for i in 0..units.len() {
        for j in (i + 1)..units.len() {
            let dx = units[j].x - units[i].x;
            let dy = units[j].y - units[i].y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance < min_distance && distance != 0.0 {
                let overlap = (min_distance - distance) / 2.0;
                let (nx, ny) = (dx / distance, dy / distance);
                units[i].x -= nx * overlap;
                units[i].y -= ny * overlap;
                units[j].x += nx * overlap;
                units[j].y += ny * overlap;
            }
        }
    }
}

pub(super) fn unit_at(x: f32, y: f32) -> Unit {
    Unit::new(x, y, 100.0)
}

pub(super) fn positions(units: &[Unit]) -> Vec<(f32, f32)> {
    units.iter().map(|e| (e.x, e.y)).collect()
}

pub(super) fn closest_pair(units: &[Unit]) -> f32 {
    let mut best = f32::MAX;
    for i in 0..units.len() {
        for j in (i + 1)..units.len() {
            best = best.min(Vec2::new(units[i].x, units[i].y).distance(Vec2::new(units[j].x, units[j].y)));
        }
    }
    best
}

#[test]
fn the_grid_gives_the_same_result_as_comparing_every_pair_when_pairs_are_apart() {
    // 300 separate overlapping pairs, far from each other and spread over a big area, so the order of the pushes can't matter
    let mut grid: Vec<Unit> = Vec::new();
    for k in 0..300 {
        let (bx, by) = ((k % 20) as f32 * 500.0 + 100.0, (k / 20) as f32 * 500.0 + 100.0);
        let gap = 3.0 + (k % 25) as f32; // 3..27 px apart, all overlapping
        let angle = k as f32 * 0.7;
        grid.push(unit_at(bx, by));
        grid.push(unit_at(bx + angle.cos() * gap, by + angle.sin() * gap));
    }
    let mut every_pair: Vec<Unit> = grid.iter().map(|e| unit_at(e.x, e.y)).collect();
    spread_units(&mut grid);
    spread_every_pair(&mut every_pair);
    for (a, b) in positions(&grid).into_iter().zip(positions(&every_pair)) {
        assert!((a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3, "{a:?} against {b:?}");
    }
}

#[test]
fn the_grid_finds_every_overlapping_pair_across_cell_edges() {
    // one overlapping pair at a time, straddling an edge or corner of the grid, whose cells are 32 wide and start at the
    // lowest unit (the anchor at 0,0, which is too far away to be pushed itself)
    let cases = [
        (31.0, 100.0, 33.0, 100.0),   // across a column edge
        (100.0, 31.5, 100.0, 33.5),   // across a row edge
        (95.9, 63.9, 96.1, 64.1),     // across a corner
        (31.9, 99.9, 32.1, 96.2),     // across a column edge, a little diagonal
        (200.0, 200.0, 210.0, 205.0), // not near an edge at all
    ];
    for (ax, ay, bx, by) in cases {
        let mut grid = vec![unit_at(0.0, 0.0), unit_at(ax, ay), unit_at(bx, by)];
        let mut every_pair: Vec<Unit> = grid.iter().map(|e| unit_at(e.x, e.y)).collect();
        spread_units(&mut grid);
        spread_every_pair(&mut every_pair);
        assert!(Vec2::new(grid[1].x, grid[1].y).distance(Vec2::new(grid[2].x, grid[2].y)) > 31.99, "{ax},{ay} not pushed apart");
        for (a, b) in positions(&grid).into_iter().zip(positions(&every_pair)) {
            assert!((a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3, "{a:?} against {b:?}");
        }
    }
}

#[test]
fn units_far_apart_are_left_alone() {
    let mut units: Vec<Unit> = (0..50).map(|i| unit_at(i as f32 * 40.0, (i % 7) as f32 * 41.0)).collect();
    let before = positions(&units);
    spread_units(&mut units);
    assert_eq!(positions(&units), before);
}

#[test]
fn a_dense_pile_spreads_out_as_well_as_it_did_before() {
    // 400 units in a 400 x 400 px square, too crowded to fit without moving, spread for 300 frames each way
    let mut seed = 5u32;
    let mut next = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / 16_777_216.0
    };
    let start: Vec<(f32, f32)> = (0..400).map(|_| (1000.0 + next() * 400.0, 1000.0 + next() * 400.0)).collect();
    let mut grid: Vec<Unit> = start.iter().map(|&(x, y)| unit_at(x, y)).collect();
    let mut every_pair: Vec<Unit> = start.iter().map(|&(x, y)| unit_at(x, y)).collect();
    for _ in 0..300 {
        spread_units(&mut grid);
        spread_every_pair(&mut every_pair);
    }
    let (g, e) = (closest_pair(&grid), closest_pair(&every_pair));
    println!("closest pair after 300 passes: grid {g:.2} px, every pair {e:.2} px");
    assert!(g > 30.0 && g > e - 1.5, "grid left a pair {g} apart, every-pair {e}");
}

#[test]
fn units_exactly_on_top_of_each_other_get_apart() {
    let mut units: Vec<Unit> = (0..12).map(|_| unit_at(500.0, 500.0)).collect();
    for _ in 0..400 {
        spread_units(&mut units);
    }
    let closest = closest_pair(&units);
    assert!(closest > 29.0, "still {closest} apart");
}

#[test]
fn spreading_handles_none_one_and_far_apart() {
    let mut none: Vec<Unit> = Vec::new();
    spread_units(&mut none);
    let mut one = vec![unit_at(5.0, 5.0)];
    spread_units(&mut one);
    assert_eq!((one[0].x, one[0].y), (5.0, 5.0));
    let mut far = vec![unit_at(16.0, 16.0), unit_at(WORLD_SIZE - 16.0, WORLD_SIZE - 16.0)];
    spread_units(&mut far);
    assert_eq!(positions(&far), vec![(16.0, 16.0), (WORLD_SIZE - 16.0, WORLD_SIZE - 16.0)]);
}

#[test]
fn spreading_time_for_4000_units() {
    let player = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);
    rand::srand(3);
    let mut grid: Vec<Unit> = (0..4000)
        .map(|_| {
            let p = far_position(player.x, player.y);
            unit_at(p.x, p.y)
        })
        .collect();
    let mut every_pair: Vec<Unit> = grid.iter().map(|e| unit_at(e.x, e.y)).collect();
    let n = 20;
    let t = Instant::now();
    for _ in 0..n {
        spread_units(&mut grid);
    }
    let grid_ms = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let t = Instant::now();
    for _ in 0..n {
        spread_every_pair(&mut every_pair);
    }
    let pair_ms = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    println!("spreading 4000 units scattered over the field: grid {grid_ms:.2} ms, every pair {pair_ms:.2} ms");
}

//walk a unit at a player for `frames` frames, optionally working out the steering every frame like the game used to
pub(super) fn chase(walls: &Walls, start: Vec2, player: Vec2, every_frame: bool, frames: usize, dt: f32) -> Option<usize> {
    let field = field_for(player, walls);
    let mut unit = Unit::new(start.x, start.y, 150.0);
    for frame in 0..frames {
        if every_frame {
            unit.steer_in = 0.0;
        }
        step_unit(&mut unit, player, dt, walls, &field, 1.0);
        push_out_of_walls(&mut unit.x, &mut unit.y, walls);
        if Vec2::new(unit.x, unit.y).distance(player) <= UNIT_FOLLOW_DISTANCE {
            return Some(frame);
        }
    }
    None
}

#[test]
fn steering_every_tenth_of_a_second_gets_round_walls_about_as_fast_as_every_frame() {
    let long_wall = walls_of(&(5..=15).map(|r| (20, r)).collect::<Vec<_>>());
    let mut cup: Vec<_> = (8..=12).map(|r| (20, r)).collect();
    cup.extend((21..=26).map(|c| (c, 8)));
    cup.extend((21..=26).map(|c| (c, 12)));
    let cup = walls_of(&cup);
    let stairs = walls_of(&(0..=14).map(|i| (10 + i, 10 + i)).collect::<Vec<_>>());
    let cases = [
        ("long wall", &long_wall, (15, 10), (25, 10)),
        ("cup", &cup, (15, 10), (23, 10)),
        ("staircase", &stairs, (14, 18), (18, 12)),
    ];
    for fps in [60.0, 120.0, 240.0] {
        for (name, walls, from, to) in cases {
            let (start, player) = (tile_center(from), tile_center(to));
            let each = chase(walls, start, player, true, 8000, 1.0 / fps).expect("never arrived steering every frame");
            let tenth = chase(walls, start, player, false, 8000, 1.0 / fps).expect("never arrived steering every 0.1 s");
            println!("{name} at {fps} fps: every frame {each} frames, every 0.1 s {tenth} frames");
            assert!(tenth as f32 <= each as f32 * 1.1 + 0.2 * fps, "{name} at {fps}: {tenth} against {each}");
        }
    }
}

#[test]
fn a_unit_keeps_its_heading_between_steering_updates_and_then_changes_it() {
    let walls = Walls::default();
    let player = Vec2::new(5000.0, 1000.0);
    let field = field_for(player, &walls);
    let mut unit = Unit::new(1000.0, 1000.0, 100.0);
    unit.steer = Steer::Point(Vec2::new(1000.0, 1500.0)); // heading south
    unit.steer_in = 0.05;
    step_unit(&mut unit, player, 0.01, &walls, &field, 1.0);
    assert!(unit.y > 1000.0 && unit.x == 1000.0, "should keep heading south, at ({}, {})", unit.x, unit.y);
    for _ in 0..5 {
        step_unit(&mut unit, player, 0.01, &walls, &field, 1.0);
    }
    // the update was due after 0.05 s, the player is far to the east and the way is clear
    let before = unit.x;
    step_unit(&mut unit, player, 0.01, &walls, &field, 1.0);
    assert!(unit.x > before, "should have turned towards the player");
}

#[test]
fn units_start_their_steering_timers_at_different_times() {
    let units: Vec<Unit> = (0..200).map(|_| unit_at(0.0, 0.0)).collect();
    let timers: Vec<f32> = units.iter().map(|e| e.steer_in).collect();
    assert!(timers.iter().all(|&t| (0.0..STEERING_INTERVAL).contains(&t)));
    let mut buckets = [0; 10];
    for t in timers {
        buckets[(t / STEERING_INTERVAL * 10.0) as usize] += 1;
    }
    assert!(buckets.iter().all(|&b| b > 5), "uneven start times {buckets:?}");
}

#[test]
fn a_unit_chasing_a_moving_player_in_the_open_keeps_up_just_as_well() {
    // the player runs in a circle at 300 px/s, the unit is faster; how close does it get
    let walls = Walls::default();
    for every_frame in [true, false] {
        let mut player = Vec2::new(3000.0, 3000.0);
        let mut field = FlowField::new();
        field.compute(player, &walls);
        let mut unit = Unit::new(2000.0, 3000.0, 400.0);
        let dt = 1.0 / 120.0;
        let mut closest = f32::MAX;
        for frame in 0..(120 * 20) {
            let angle = frame as f32 * dt * 0.5;
            player = Vec2::new(3000.0 + 600.0 * angle.cos(), 3000.0 + 600.0 * angle.sin());
            if frame % 24 == 0 {
                field.compute(player, &walls);
            }
            if every_frame {
                unit.steer_in = 0.0;
            }
            step_unit(&mut unit, player, dt, &walls, &field, 1.0);
            if frame > 120 * 10 {
                closest = closest.min(Vec2::new(unit.x, unit.y).distance(player));
            }
        }
        println!("steering every frame: {every_frame}: closest approach in the last 10 s {closest:.1} px");
        assert!(closest <= UNIT_FOLLOW_DISTANCE + 1.0, "never caught up, closest {closest}");
    }
}
