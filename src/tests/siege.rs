//enemies and the player's walls: they keep out of them, go round short ones, chew through long ones and crush with size

use super::*;
use super::fortify::*;

//an enemy that runs at the player and nothing else
pub(super) fn runner(at: Vec2) -> Enemy {
    let mut enemy = plain_enemy(at);
    enemy.speed = 100.0;
    enemy.attack = 25.0;
    enemy.aggro = WORLD_SIZE;
    enemy.chasing = true;
    enemy
}

//enemies chase a player standing still for some seconds, in the order the game does it, with the walls wearing down as they
//are chewed. returns when the first got within 50 px of the player, if it did. it checks after every frame that no enemy
//is inside a wall that stops it
pub(super) fn besiege(walls: &mut Walls, enemies: &mut Vec<Enemy>, player: Vec2, seconds: f32) -> Option<f32> {
    let mut routes = Routes::new(player, walls);
    let mut fx = Effects::default();
    let dt = 1.0 / 60.0;
    for frame in 0..(seconds / dt) as usize {
        let broke = update_walls(walls, &mut [], dt, &mut fx);
        routes.update(player, walls, broke);
        routes.finish();
        patrol_enemies(enemies, player, dt, 1.0, walls, &routes.foes, &mut fx);
        chew_walls(enemies, walls, dt, &mut fx);
        for enemy in enemies.iter() {
            let (x, y) = (enemy.position.x, enemy.position.y);
            assert!(!touches_blocking(x, y, enemy.wall_radius() - 0.5, Mover::Enemy, walls), "an enemy is inside a wall at ({x},{y}) on frame {frame}");
        }
        if enemies.iter().any(|enemy| enemy.position.distance(player) < 50.0) {
            return Some(frame as f32 * dt);
        }
    }
    None
}

fn line(col: i32, first_row: i32, last_row: i32, kind: WallKind) -> Vec<Wall> {
    (first_row..=last_row).map(|row| made(kind, Side::Friendly, col, row)).collect()
}

#[test]
fn an_enemy_goes_round_a_short_wall_without_touching_it() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let mut walls = walls_with(line(col, row - 4, row + 4, WallKind::Wall));
    let player = tile_center((col + 6, row));
    let mut enemies = vec![runner(tile_center((col - 6, row)))];
    let reached = besiege(&mut walls, &mut enemies, player, 20.0);
    assert!(reached.is_some(), "it never got there, ended at {}", enemies[0].position);
    assert!(walls.values().all(|wall| wall.health == 500.0), "it chewed a wall it could have gone round");
}

#[test]
fn an_enemy_chews_through_a_wall_that_is_too_long_to_go_round() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let mut walls = walls_with(line(col, row - 40, row + 40, WallKind::Wall));
    let player = tile_center((col + 3, row));
    let mut enemies = vec![runner(tile_center((col - 3, row)))];
    let reached = besiege(&mut walls, &mut enemies, player, 90.0);
    let reached = reached.expect("it never got through");
    println!("through a long wall in {reached:.1} s");
    //a wall of 500 health and 25 attack at 60% takes 33 seconds
    assert!(reached > 20.0, "it was through in {reached} s");
    assert!(walls.len() < 81, "something broke");
}

#[test]
fn an_enemy_cannot_get_at_a_player_walled_in_until_it_has_chewed_through() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let mut walls = walls_with(outline(col - 3, row - 3, col + 3, row + 3).into_iter().map(|(c, r)| made(WallKind::Wall, Side::Friendly, c, r)).collect());
    let player = tile_center((col, row));
    let mut enemies = vec![runner(tile_center((col - 9, row)))];
    let reached = besiege(&mut walls, &mut enemies, player, 100.0).expect("it never got in");
    println!("into the house in {reached:.1} s");
    assert!(reached > 15.0, "it was in after {reached} s");
    assert_eq!(walls.len(), 24 - 1, "exactly one wall gave way");
}

#[test]
fn the_stronger_the_enemy_the_faster_the_wall_goes() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let time_to_get_through = |attack: f32| {
        let mut walls = walls_with(outline(col - 3, row - 3, col + 3, row + 3).into_iter().map(|(c, r)| made(WallKind::Wall, Side::Friendly, c, r)).collect());
        let mut enemy = runner(tile_center((col - 9, row)));
        enemy.attack = attack;
        besiege(&mut walls, &mut vec![enemy], tile_center((col, row)), 200.0).expect("it never got in")
    };
    let weak = time_to_get_through(25.0);
    println!("attack 25: {weak:.1} s");
    let strong = time_to_get_through(100.0);
    assert!(strong < weak * 0.6, "{strong} s next to {weak} s");
}

#[test]
fn a_gate_in_the_way_is_chewed_like_a_wall_and_a_gap_beside_it_is_used_instead() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    //a gate with nothing round it blocks an enemy, which goes round
    let mut walls = walls_with(line(col, row - 4, row + 4, WallKind::Gate));
    let player = tile_center((col + 6, row));
    let mut enemies = vec![runner(tile_center((col - 6, row)))];
    assert!(besiege(&mut walls, &mut enemies, player, 20.0).is_some());
    //the whole way across, it chews
    let mut walls = walls_with(line(col, row - 40, row + 40, WallKind::Gate));
    let mut enemies = vec![runner(tile_center((col - 3, row)))];
    assert!(besiege(&mut walls, &mut enemies, tile_center((col + 3, row)), 4.0).is_none());
    assert!(walls.values().any(|wall| wall.health < 500.0), "nothing was chewed");
}

#[test]
fn an_enemy_too_big_for_a_gap_chews_the_wall_beside_it() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    //a wall with a one square gap in a very long wall
    let mut tiles = line(col, row - 40, row - 1, WallKind::Wall);
    tiles.extend(line(col, row + 1, row + 40, WallKind::Wall));
    let mut walls = walls_with(tiles);
    let player = tile_center((col + 3, row));
    let mut titan = runner(tile_center((col - 3, row)));
    titan.size = 130.0;
    titan.attack = 160.0;
    let mut small = runner(tile_center((col - 3, row + 2)));
    small.size = 30.0;
    //the small one goes through the gap, the big one is wider than it
    let mut with_small = vec![small];
    assert!(besiege(&mut walls, &mut with_small, player, 10.0).is_some(), "the small one did not get through the gap");
    assert!(walls.values().all(|wall| wall.health == 500.0), "nothing should have been chewed for it");
    let mut with_titan = vec![titan];
    besiege(&mut walls, &mut with_titan, player, 30.0);
    assert!(walls.values().any(|wall| wall.health < 500.0), "the big one did not touch a wall");
}

#[test]
fn a_giant_crushes_every_wall_it_is_standing_on() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let at = tile_center((col, row));
    let walls = walls_with((-2..=2).flat_map(|dc| (-2..=2).map(move |dr| made(WallKind::Wall, Side::Friendly, col + dc, row + dr))).collect());
    let mut giant = runner(at);
    giant.size = 500.0;
    giant.attack = 400.0;
    let mut small = runner(at);
    small.size = 40.0;
    small.attack = 400.0;
    let mut fx = Effects::default();
    chew_walls(&[small], &walls, 1.0, &mut fx);
    assert!(fx.wall_hits.is_empty(), "a small one crushes nothing");
    chew_walls(&[giant], &walls, 1.0, &mut fx);
    assert_eq!(fx.wall_hits.len(), 25);
    assert!(fx.wall_hits.iter().all(|&(_, damage)| (damage - 400.0 * ENEMY_CRUSH_SHARE).abs() < 0.01));
    //and one that isn't after anyone crushes nothing
    let mut idle = runner(at);
    idle.size = 500.0;
    idle.chasing = false;
    let mut fx = Effects::default();
    chew_walls(&[idle], &walls, 1.0, &mut fx);
    assert!(fx.wall_hits.is_empty());
}

#[test]
fn an_enemy_held_up_by_a_wall_it_wants_through_presses_on_it() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let walls = walls_with(line(col, row - 40, row + 40, WallKind::Wall));
    let player = tile_center((col + 3, row));
    let mut routes = Routes::new(player, &walls);
    routes.finish();
    let mut enemies = vec![runner(tile_center((col - 1, row)))];
    let mut fx = Effects::default();
    for _ in 0..120 {
        patrol_enemies(&mut enemies, player, 1.0 / 60.0, 1.0, &walls, &routes.foes, &mut fx);
    }
    assert_eq!(enemies[0].pressing, Some((col, row)));
    assert!(enemies[0].stuck > 0.5);
    //it chews at its attack times the share, for as long as it presses
    let mut fx = Effects::default();
    chew_walls(&enemies, &walls, 0.5, &mut fx);
    assert_eq!(fx.wall_hits.len(), 1);
    assert!((fx.wall_hits[0].1 - 25.0 * ENEMY_CHEW_SHARE * 0.5).abs() < 0.01);
}

#[test]
fn an_enemy_on_its_patrol_does_not_chew_but_gives_up_on_a_waypoint_it_cannot_reach() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    //a box with the waypoint inside it
    let walls = walls_with(outline(col + 3, row - 3, col + 9, row + 3).into_iter().map(|(c, r)| made(WallKind::Wall, Side::Friendly, c, r)).collect());
    let mut enemy = plain_enemy(tile_center((col - 4, row)));
    enemy.speed = 200.0;
    enemy.route = vec![tile_center((col + 6, row)), tile_center((col - 8, row))];
    enemy.heading_for = 0;
    let mut enemies = vec![enemy];
    let mut fx = Effects::default();
    let none = FlowField::new();
    let mut gave_up_after = None;
    for frame in 0..600 {
        patrol_enemies(&mut enemies, Vec2::new(1.0, 1.0), 1.0 / 60.0, 1.0, &walls, &none, &mut fx);
        assert!(enemies[0].pressing.is_none(), "it chewed on its patrol");
        if enemies[0].heading_for == 1 && gave_up_after.is_none() {
            gave_up_after = Some(frame);
        }
    }
    let frame = gave_up_after.expect("it kept pushing at the wall");
    assert!(frame as f32 / 60.0 < 6.0);
    assert!(fx.wall_hits.is_empty());
    //and it heads off to the other waypoint
    assert!(enemies[0].position.x < tile_center((col + 3, row)).x);
}

#[test]
fn tar_slows_a_walking_enemy_down() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let tar = walls_with((0..20).map(|k| made(WallKind::Tar, Side::Friendly, col + k, row)).collect());
    let player = tile_center((col + 19, row));
    let distance_in = |walls: &Walls| {
        let mut enemies = vec![runner(tile_center((col, row)))];
        let mut fx = Effects::default();
        let none = FlowField::new();
        for _ in 0..60 {
            terrain_effects(&mut enemies, walls, 1.0 / 60.0, &mut fx);
            patrol_enemies(&mut enemies, player, 1.0 / 60.0, 1.0, walls, &none, &mut fx);
        }
        enemies[0].position.x - tile_center((col, row)).x
    };
    let slow = distance_in(&tar);
    let quick = distance_in(&Walls::default());
    assert!((slow / quick - TAR_SLOW).abs() < 0.05, "{slow} next to {quick}");
}

#[test]
fn a_charger_that_dashes_into_a_wall_hits_it_hard_and_is_dazed() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let walls = walls_with(line(col, row - 3, row + 3, WallKind::Wall));
    let mut charger = kind_at("Charger", tile_center((col - 2, row)));
    charger.mode = Mode::Dash { dir: Vec2::X, left: CHARGE_TIME };
    let mut fx = Effects::default();
    let mut frames = 0;
    while matches!(charger.mode, Mode::Dash { .. }) && frames < 120 {
        charger.carry_on(1.0 / 60.0, 1.0, &walls, &mut fx);
        frames += 1;
    }
    assert!(frames < 40, "it kept dashing for {frames} frames");
    match charger.mode {
        Mode::Recover { left } => assert!(left > CHARGE_RECOVER, "dazed for {left}"),
        mode => panic!("it is {mode:?}"),
    }
    assert!(fx.wall_hits.iter().any(|&((c, _), damage)| c == col && damage >= charger.attack * CHARGE_DAMAGE * CHARGE_CRASH_SHARE - 0.01));
    assert!(charger.position.x < col as f32 * TILE_SIZE);
    //on open ground it dashes the whole way and is only winded
    let mut free = kind_at("Charger", tile_center((col - 20, row)));
    free.mode = Mode::Dash { dir: Vec2::X, left: CHARGE_TIME };
    let mut fx = Effects::default();
    while matches!(free.mode, Mode::Dash { .. }) {
        free.carry_on(1.0 / 60.0, 1.0, &Walls::default(), &mut fx);
    }
    assert!(matches!(free.mode, Mode::Recover { left } if (left - CHARGE_RECOVER).abs() < 0.001));
    assert!(fx.wall_hits.is_empty());
}

#[test]
fn enemies_walk_as_before_when_there_are_no_walls_near() {
    let at = FAR;
    let mut enemies = vec![runner(at)];
    let player = at + Vec2::new(900.0, 0.0);
    let mut fx = Effects::default();
    for _ in 0..60 {
        patrol_enemies(&mut enemies, player, 1.0 / 60.0, 1.0, &Walls::default(), &FlowField::new(), &mut fx);
    }
    assert!((enemies[0].position.x - at.x - 100.0 * CHASE_SPEED).abs() < 1.0, "{}", enemies[0].position.x - at.x);
    assert_eq!(enemies[0].position.y, at.y);
}

#[test]
fn many_enemies_round_many_walls_cost_little_time() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let mut tiles = Vec::new();
    for k in 0..30 {
        tiles.extend(line(col - 20 + k * 2, row - 20, row + 20, WallKind::Wall));
    }
    let mut walls = walls_with(tiles);
    let player = tile_center((col, row));
    let mut enemies: Vec<Enemy> = (0..150).map(|k| runner(tile_center((col - 25 + k % 50, row - 22 + k / 50)))).collect();
    let mut routes = Routes::new(player, &walls);
    routes.finish();
    let mut fx = Effects::default();
    let started = Instant::now();
    for _ in 0..60 {
        patrol_enemies(&mut enemies, player, 1.0 / 60.0, 1.0, &walls, &routes.foes, &mut fx);
        chew_walls(&enemies, &walls, 1.0 / 60.0, &mut fx);
        update_walls(&mut walls, &mut [], 1.0 / 60.0, &mut fx);
    }
    let per_frame = started.elapsed().as_secs_f32() / 60.0 * 1000.0;
    println!("150 enemies among 1200 walls: {per_frame:.2} ms a frame");
    assert!(per_frame < 6.0, "{per_frame} ms a frame");
}
