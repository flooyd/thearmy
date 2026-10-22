//sending a squad of units at an enemy

use super::*;


// ---------- sending a squad of units at an enemy ----------

//an enemy that stays where it is, with the health that is wanted
pub(super) fn still_enemy(at: Vec2, health: f32) -> Enemy {
    let mut enemy = plain_enemy(at);
    enemy.speed = 0.0;
    enemy.health = health;
    enemy.max_health = health;
    enemy
}

//one frame of what the game does with units, squads and enemies. returns how many enemies were destroyed
pub(super) fn frame_of_fighting(player: Vec2, units: &mut Vec<Unit>, enemies: &mut Vec<Enemy>, babies: &mut Vec<Baby>, squads: &mut Squads, walls: &Walls, field: &FlowField, dt: f32) -> usize {
    update_squads(squads, enemies, units, walls, false);
    update_units(units, player.x, player.y, dt, walls, field, 1.0, squads);
    patrol_free(enemies, NOBODY, dt, 1.0, &mut Effects::default());
    let destroyed = fight(enemies, units, babies, dt, &Upgrades::default(), &mut Effects::default());
    bury_units(units, &mut Effects::default());
    destroyed
}

#[test]
fn a_bigger_tougher_enemy_is_sent_a_bigger_squad_and_a_hurt_one_a_smaller_one() {
    let at = Vec2::new(16000.0, 16000.0);
    let sizes: Vec<usize> = [50.0, 100.0, 400.0, 1000.0, 2000.0, 9000.0].iter().map(|&h| squad_size(&still_enemy(at, h))).collect();
    println!("squad for health 50/100/400/1000/2000/9000: {sizes:?}");
    // never fewer than the minimum, and enough to do the damage in the time
    assert_eq!(sizes[0], SQUAD_MIN_UNITS);
    assert_eq!(sizes[1], SQUAD_MIN_UNITS);
    assert_eq!(sizes[2], 2);
    assert_eq!(sizes[3], 5);
    assert_eq!(sizes[4], 10);
    assert_eq!(sizes[5], 45);
    assert!(sizes.windows(2).all(|pair| pair[0] <= pair[1]));
    // a hurt enemy needs fewer
    let mut enemy = still_enemy(at, 9000.0);
    let full = squad_size(&enemy);
    enemy.health = 2000.0;
    assert!(squad_size(&enemy) < full / 3);
    // and every kind, at every army size, is sent something between the minimum and about twice what fits in it, which is
    // as far out as the random nudges to its health and size can take it (health goes as the size squared)
    for army in [1usize, 10, 100, 1000, 4000] {
        let scale = army_scale(army);
        for kind in ENEMY_KINDS.iter().filter(|kind| army >= kind.from_army) {
            let enemy = Enemy::new(at, kind, scale);
            let size = squad_size(&enemy);
            let fit = ((enemy.size + 2.0 * RADIUS) / 32.0).powi(2);
            println!("army {army}: kind of size {:.0} with health {:.0} is sent {size} units ({fit:.0} fit in it)", kind.size, enemy.health);
            assert!(size >= SQUAD_MIN_UNITS && (size as f32) <= fit.max(SQUAD_MIN_UNITS as f32) * 2.0, "{size} units for something with room for {fit:.0}");
        }
    }
}

#[test]
fn the_squad_is_the_units_closest_to_the_enemy_not_to_the_player() {
    let player = Vec2::new(3000.0, 3000.0);
    let enemy = still_enemy(Vec2::new(5000.0, 3000.0), 1000.0); // 5 units
    // units all round the player, some of them on the enemy's side
    let mut units: Vec<Unit> = (0..60).map(|_| {
        let p = random_position_around(player.x, player.y, 50.0, 800.0);
        Unit::new(p.x, p.y, 400.0)
    }).collect();
    let mut by_distance: Vec<usize> = (0..units.len()).collect();
    by_distance.sort_by(|&a, &b| Vec2::new(units[a].x, units[a].y).distance(enemy.position).total_cmp(&Vec2::new(units[b].x, units[b].y).distance(enemy.position)));
    let mut squads = Squads::new();
    let sent = send_squad(&mut units, &mut squads, &enemy, &mut Effects::default());
    assert_eq!(sent, 5);
    let mut chosen: Vec<usize> = (0..units.len()).filter(|&i| units[i].target == Some(enemy.id)).collect();
    chosen.sort();
    let mut closest = by_distance[..5].to_vec();
    closest.sort();
    assert_eq!(chosen, closest, "not the five closest to the enemy");
    assert_eq!(units.iter().filter(|unit| unit.target.is_some()).count(), 5);
    assert_eq!(squads.len(), 1);
    assert!(squads.contains_key(&enemy.id));
    // they start heading the right way at once
    assert!(chosen.iter().all(|&i| units[i].steer_in == 0.0));
}

#[test]
fn units_already_sent_somewhere_are_not_taken_and_a_second_click_sends_the_next_closest() {
    let enemy = still_enemy(Vec2::new(5000.0, 3000.0), 600.0); // 3 units
    let other = still_enemy(Vec2::new(900.0, 900.0), 600.0);
    // ten units in a line towards the enemy, the closest to it first
    let mut units: Vec<Unit> = (0..10).map(|i| Unit::new(4800.0 - 100.0 * i as f32, 3000.0, 400.0)).collect();
    let mut squads = Squads::new();
    // the two closest are busy with another enemy
    units[0].target = Some(other.id);
    units[1].target = Some(other.id);
    assert_eq!(send_squad(&mut units, &mut squads, &enemy, &mut Effects::default()), 3);
    let on_it: Vec<usize> = (0..10).filter(|&i| units[i].target == Some(enemy.id)).collect();
    assert_eq!(on_it, vec![2, 3, 4], "the closest free ones");
    assert_eq!(units[0].target, Some(other.id));
    // clicking it again sends three more
    assert_eq!(send_squad(&mut units, &mut squads, &enemy, &mut Effects::default()), 3);
    let on_it: Vec<usize> = (0..10).filter(|&i| units[i].target == Some(enemy.id)).collect();
    assert_eq!(on_it, vec![2, 3, 4, 5, 6, 7]);
    assert_eq!(squads.len(), 1, "joined the squad it already has");
    // and with the rest, which is fewer than wanted, it sends what there is
    assert_eq!(send_squad(&mut units, &mut squads, &enemy, &mut Effects::default()), 2);
    assert_eq!(units.iter().filter(|unit| unit.target.is_none()).count(), 0);
    // then there is nobody left to send
    assert_eq!(send_squad(&mut units, &mut squads, &enemy, &mut Effects::default()), 0);
    let mut empty = Squads::new();
    assert_eq!(send_squad(&mut units, &mut empty, &enemy, &mut Effects::default()), 0);
    assert!(empty.is_empty(), "made a squad with nobody in it");
    assert_eq!(send_squad(&mut Vec::new(), &mut empty, &enemy, &mut Effects::default()), 0);
}

#[test]
fn a_squad_is_never_bigger_than_the_army() {
    let enemy = still_enemy(Vec2::new(5000.0, 3000.0), 9000.0); // 45 units
    let mut units: Vec<Unit> = (0..7).map(|i| Unit::new(4000.0 + 10.0 * i as f32, 3000.0, 400.0)).collect();
    let mut squads = Squads::new();
    assert_eq!(send_squad(&mut units, &mut squads, &enemy, &mut Effects::default()), 7);
    assert!(units.iter().all(|unit| unit.target == Some(enemy.id)));
}

#[test]
fn a_click_is_on_the_enemy_under_it_with_a_margin_and_the_nearest_when_they_overlap() {
    let a = still_enemy(Vec2::new(1000.0, 1000.0), 100.0); // 44 across
    let b = still_enemy(Vec2::new(1030.0, 1000.0), 100.0);
    let enemies = vec![a, b];
    // inside
    assert_eq!(enemy_at(Vec2::new(1000.0, 1000.0), &enemies, 0.0), Some(0));
    assert_eq!(enemy_at(Vec2::new(1021.0, 1021.0), &enemies, 0.0), Some(1), "the corner of a, inside b and nearer to b's middle");
    // the one nearer wherever both are under the point
    assert_eq!(enemy_at(Vec2::new(1010.0, 1000.0), &enemies, 0.0), Some(0));
    assert_eq!(enemy_at(Vec2::new(1020.0, 1000.0), &enemies, 0.0), Some(1));
    // just outside, which only counts with the margin
    let outside = Vec2::new(1000.0 - 22.0 - 5.0, 1000.0);
    assert_eq!(enemy_at(outside, &enemies, 0.0), None);
    assert_eq!(enemy_at(outside, &enemies, 6.0), Some(0));
    assert_eq!(enemy_at(outside, &enemies, 4.0), None);
    // on a diagonal the margin is a square all round, like the enemy
    assert_eq!(enemy_at(Vec2::new(1000.0 - 22.0 - 5.0, 1000.0 - 22.0 - 5.0), &enemies, 6.0), Some(0));
    // far from them
    assert_eq!(enemy_at(Vec2::new(3000.0, 3000.0), &enemies, 12.0), None);
    assert_eq!(enemy_at(Vec2::new(1000.0, 1000.0), &[], 12.0), None);
    // zoomed all the way out the margin is 120 world pixels, so a 4 pixel enemy on screen is easy to hit
    let margin = ENEMY_CLICK_MARGIN / MIN_ZOOM;
    assert_eq!(enemy_at(Vec2::new(1000.0 + 22.0 + 100.0, 1000.0), &[enemies.into_iter().next().unwrap()], margin), Some(0));
}

#[test]
fn a_squad_walks_to_its_enemy_destroys_it_and_goes_back_to_the_player() {
    let walls = walls_of(&[]);
    let player = Vec2::new(3000.0, 3000.0);
    let field = field_for(player, &walls);
    let mut units: Vec<Unit> = (0..30).map(|_| {
        let p = random_position_around(player.x, player.y, 60.0, 200.0);
        Unit::new(p.x, p.y, 400.0)
    }).collect();
    let mut enemy = still_enemy(Vec2::new(4500.0, 3000.0), 600.0);
    enemy.babies = 2;
    let id = enemy.id;
    let mut enemies = vec![enemy];
    let mut squads = Squads::new();
    let mut babies = Vec::new();
    let mut closest: Vec<usize> = (0..units.len()).collect();
    closest.sort_by(|&a, &b| Vec2::new(units[a].x, units[a].y).distance(enemies[0].position).total_cmp(&Vec2::new(units[b].x, units[b].y).distance(enemies[0].position)));
    closest.truncate(3);
    closest.sort();
    assert_eq!(send_squad(&mut units, &mut squads, &enemies[0], &mut Effects::default()), 3);
    let sent: Vec<usize> = (0..units.len()).filter(|&i| units[i].target == Some(id)).collect();
    assert_eq!(sent, closest);

    let mut destroyed_at = None;
    let mut disbanded_at = None;
    for frame in 0..(60 * 30) {
        let destroyed = frame_of_fighting(player, &mut units, &mut enemies, &mut babies, &mut squads, &walls, &field, 1.0 / 60.0);
        if destroyed > 0 && destroyed_at.is_none() {
            destroyed_at = Some(frame as f32 / 60.0);
        }
        if destroyed_at.is_some() && squads.is_empty() && disbanded_at.is_none() {
            disbanded_at = Some(frame as f32 / 60.0);
        }
        // nobody but the squad ever leaves the player
        if destroyed_at.is_none() {
            for (i, unit) in units.iter().enumerate() {
                if !sent.contains(&i) {
                    assert!(Vec2::new(unit.x, unit.y).distance(player) < 400.0, "unit {i} that was not sent wandered off");
                }
            }
        }
    }
    let destroyed_at = destroyed_at.expect("the enemy was never destroyed");
    println!("destroyed after {destroyed_at:.1} s, squad disbanded at {:.2} s", disbanded_at.unwrap());
    // 1300 px to go at 400 px/s is 3.2 s, then 3 units do 600 in 2 s
    assert!(destroyed_at > 3.0 && destroyed_at < 8.0, "destroyed at {destroyed_at}");
    assert_eq!(babies.len(), 2);
    assert!(squads.is_empty());
    assert!(units.iter().all(|unit| unit.target.is_none()));
    // and the three that were sent are back with the player
    for &i in &sent {
        let distance = Vec2::new(units[i].x, units[i].y).distance(player);
        assert!(distance < 250.0, "sent unit {i} is {distance} from the player");
    }
}

#[test]
fn units_sent_at_an_enemy_walk_right_into_it_while_followers_stop_short() {
    let walls = walls_of(&[]);
    let player = Vec2::new(3000.0, 3000.0);
    let field = field_for(player, &walls);
    for (fps, boost) in [(120.0, 1.0), (60.0, 3.0), (20.0, 3.0)] {
        let enemy = still_enemy(Vec2::new(3000.0, 4000.0), 100000.0);
        let mut units = vec![Unit::new(2000.0, 3000.0, 550.0), Unit::new(4000.0, 3000.0, 550.0)];
        let mut squads = Squads::new();
        units[0].target = Some(enemy.id);
        squads.insert(enemy.id, Squad { position: enemy.position, field: FlowField::new(), next_field: FlowField::new() });
        for _ in 0..(fps * 8.0) as usize {
            update_units(&mut units, player.x, player.y, 1.0 / fps, &walls, &field, boost, &squads);
        }
        let to_enemy = Vec2::new(units[0].x, units[0].y).distance(enemy.position);
        let to_player = Vec2::new(units[1].x, units[1].y).distance(player);
        println!("{fps} fps x{boost}: sent unit {to_enemy:.2} from the enemy's middle, follower {to_player:.2} from the player");
        assert!(to_enemy < 0.5, "{to_enemy}");
        assert!(to_player > UNIT_FOLLOW_DISTANCE - 1.0 && to_player <= UNIT_FOLLOW_DISTANCE + 0.01);
    }
}

#[test]
fn a_squad_finds_its_way_round_a_wall_to_an_enemy() {
    // a long wall with its ends 20 squares up and down from the middle, the army on one side and the enemy on the other
    let tiles: Vec<(i32, i32)> = (30..=70).map(|row| (50, row)).collect();
    let walls = walls_of(&tiles);
    let player = tile_center((45, 50));
    let field = field_for(player, &walls);
    let enemy_at_tile = tile_center((55, 50));
    let make = || {
        let units: Vec<Unit> = (0..8).map(|i| Unit::new(player.x - 40.0 - 10.0 * i as f32, player.y + 20.0 * i as f32 - 70.0, 400.0)).collect();
        (units, vec![still_enemy(enemy_at_tile, 1200.0)]) // 6 units
    };

    let (mut units, mut enemies) = make();
    let mut squads = Squads::new();
    let mut babies = Vec::new();
    assert_eq!(send_squad(&mut units, &mut squads, &enemies[0], &mut Effects::default()), 6);
    let mut destroyed_at = None;
    for frame in 0..(60 * 60) {
        let destroyed = frame_of_fighting(player, &mut units, &mut enemies, &mut babies, &mut squads, &walls, &field, 1.0 / 60.0);
        for unit in &units {
            assert!(!touches_wall(unit.x, unit.y, RADIUS - 0.05, &walls), "a unit is in the wall on frame {frame}");
        }
        if destroyed > 0 {
            destroyed_at = Some(frame as f32 / 60.0);
            break;
        }
    }
    println!("went round the wall and destroyed the enemy after {:?} s", destroyed_at);
    assert!(destroyed_at.is_some(), "the squad never got round the wall");

    // without the route to lead them they walk straight at it and stay on the wrong side of the wall
    let (mut units, mut enemies) = make();
    let mut squads = Squads::new();
    send_squad(&mut units, &mut squads, &enemies[0], &mut Effects::default());
    for _ in 0..(60 * 40) {
        update_units(&mut units, player.x, player.y, 1.0 / 60.0, &walls, &field, 1.0, &squads);
        fight(&mut enemies, &mut units, &mut babies, 1.0 / 60.0, &Upgrades::default(), &mut Effects::default());
    }
    assert!(!enemies.is_empty(), "the enemy was reached through the wall");
}

#[test]
fn a_squad_builds_its_route_and_builds_it_again_when_walls_change() {
    let mut walls = walls_of(&[]);
    let enemies = vec![still_enemy(Vec2::new(16000.0, 16000.0), 100.0)];
    let mut units = vec![Unit::new(15000.0, 16000.0, 400.0)];
    let mut squads = Squads::new();
    send_squad(&mut units, &mut squads, &enemies[0], &mut Effects::default());
    // not built yet
    assert_eq!(squads[&enemies[0].id].field.size, 0);
    for _ in 0..300 {
        update_squads(&mut squads, &enemies, &mut units, &walls, false);
    }
    let squad = &squads[&enemies[0].id];
    assert!(squad.field.size > 0 && !squad.field.is_building() && !squad.next_field.is_building());
    assert_eq!(squad.field.target, tile_of(16000.0, 16000.0));
    // a wall is built, which the route has to know about
    let (col, row) = (240, 250);
    walls.insert((col, row), Wall::new(col, row));
    update_squads(&mut squads, &enemies, &mut units, &walls, true);
    for _ in 0..300 {
        update_squads(&mut squads, &enemies, &mut units, &walls, false);
    }
    let squad = &squads[&enemies[0].id];
    let i = squad.field.index(col, row).unwrap();
    assert!(squad.field.wall[i], "the new route doesn't have the wall in it");
    assert!(!squad.field.is_building());
}

#[test]
fn a_squad_works_out_a_new_route_only_once_its_enemy_has_moved_a_few_squares() {
    let walls = walls_of(&[]);
    let mut enemies = vec![still_enemy(Vec2::new(16000.0, 16000.0), 100.0)];
    let mut units = vec![Unit::new(15000.0, 16000.0, 400.0)];
    let mut squads = Squads::new();
    send_squad(&mut units, &mut squads, &enemies[0], &mut Effects::default());
    let id = enemies[0].id;
    for _ in 0..300 {
        update_squads(&mut squads, &enemies, &mut units, &walls, false);
    }
    let first = tile_of(16000.0, 16000.0);
    assert_eq!(squads[&id].field.target, first);
    // two squares on, the route still leads where it did, but the squad knows where the enemy is
    enemies[0].position.x += 2.0 * TILE_SIZE;
    for _ in 0..300 {
        update_squads(&mut squads, &enemies, &mut units, &walls, false);
    }
    assert_eq!(squads[&id].field.target, first);
    assert_eq!(squads[&id].position, enemies[0].position);
    // three squares on it is time
    enemies[0].position.x += TILE_SIZE;
    for _ in 0..300 {
        update_squads(&mut squads, &enemies, &mut units, &walls, false);
    }
    assert_eq!(squads[&id].field.target, tile_of(enemies[0].position.x, enemies[0].position.y));
}

#[test]
fn when_an_enemy_is_destroyed_only_its_squad_is_disbanded() {
    let walls = walls_of(&[]);
    let mut enemies = vec![still_enemy(Vec2::new(5000.0, 3000.0), 100.0), still_enemy(Vec2::new(3000.0, 6000.0), 100.0)];
    let mut units: Vec<Unit> = (0..8).map(|i| Unit::new(3000.0 + 20.0 * i as f32, 3000.0, 400.0)).collect();
    let mut squads = Squads::new();
    let (first, second) = (enemies[0].id, enemies[1].id);
    send_squad(&mut units, &mut squads, &enemies[0], &mut Effects::default());
    // the squad for the second takes the next closest to it
    send_squad(&mut units, &mut squads, &enemies[1], &mut Effects::default());
    assert_eq!(squads.len(), 2);
    let on_first = units.iter().filter(|unit| unit.target == Some(first)).count();
    let on_second = units.iter().filter(|unit| unit.target == Some(second)).count();
    assert_eq!((on_first, on_second), (2, 2));
    enemies.remove(0);
    update_squads(&mut squads, &enemies, &mut units, &walls, false);
    assert_eq!(squads.len(), 1);
    assert!(squads.contains_key(&second));
    assert_eq!(units.iter().filter(|unit| unit.target == Some(first)).count(), 0);
    assert_eq!(units.iter().filter(|unit| unit.target == Some(second)).count(), 2);
    // the ones let go head for the player straight away
    assert!(units.iter().filter(|unit| unit.target.is_none()).all(|unit| unit.steer_in <= STEERING_INTERVAL));
    enemies.clear();
    update_squads(&mut squads, &enemies, &mut units, &walls, false);
    assert!(squads.is_empty());
    assert!(units.iter().all(|unit| unit.target.is_none()));
}

#[test]
fn a_unit_whose_squad_is_gone_follows_the_player_again_even_without_the_squad_being_updated() {
    // the enemy can be destroyed in the same frame the units move
    let walls = walls_of(&[]);
    let player = Vec2::new(3000.0, 3000.0);
    let field = field_for(player, &walls);
    let mut units = vec![Unit::new(2000.0, 3000.0, 400.0)];
    units[0].target = Some(987654);
    let squads = Squads::new();
    for _ in 0..600 {
        update_units(&mut units, player.x, player.y, 1.0 / 60.0, &walls, &field, 1.0, &squads);
    }
    assert!(units[0].target.is_none());
    let distance = Vec2::new(units[0].x, units[0].y).distance(player);
    assert!(distance > UNIT_FOLLOW_DISTANCE - 1.0 && distance <= UNIT_FOLLOW_DISTANCE + 0.01, "{distance}");
}

#[test]
fn every_enemy_has_its_own_id() {
    let mut ids: Vec<u32> = (0..3000).map(|i| spawn_enemy(16000.0, 16000.0, i % 500).id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 3000);
}

#[test]
fn a_lot_of_squads_do_not_cost_much_of_a_frame() {
    let walls = starting_house();
    let player = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);
    let field = field_for(player, &walls);
    rand::srand(5);
    let mut enemies: Vec<Enemy> = (0..ENEMY_COUNT).map(|_| spawn_enemy(player.x, player.y, 3000)).collect();
    let mut units: Vec<Unit> = (0..3000).map(|_| {
        let p = random_position_around(player.x, player.y, 400.0, 2000.0);
        Unit::new(p.x, p.y, 400.0)
    }).collect();
    let mut squads = Squads::new();
    for i in 0..20 {
        // 20 enemies each sent a handful
        let mut enemy = still_enemy(enemies[i].position, 600.0);
        enemy.speed = enemies[i].speed;
        enemy.route = enemies[i].route.clone();
        enemies[i] = enemy;
        send_squad(&mut units, &mut squads, &enemies[i], &mut Effects::default());
    }
    assert_eq!(squads.len(), 20);
    let mut babies = Vec::new();
    let mut squad_time = Duration::ZERO;
    let mut worst = Duration::ZERO;
    let frames = 600;
    for _ in 0..frames {
        let t = Instant::now();
        update_squads(&mut squads, &enemies, &mut units, &walls, false);
        let spent = t.elapsed();
        squad_time += spent;
        worst = worst.max(spent);
        update_units(&mut units, player.x, player.y, 1.0 / 60.0, &walls, &field, 3.0, &squads);
        patrol_free(&mut enemies, NOBODY, 1.0 / 60.0, 3.0, &mut Effects::default());
        fight(&mut enemies, &mut units, &mut babies, 1.0 / 60.0, &Upgrades::default(), &mut Effects::default());
    }
    println!("20 squads over {frames} frames: update_squads {:.3} ms a frame on average, {:.2} ms at worst, {} squads left", squad_time.as_secs_f32() * 1000.0 / frames as f32, worst.as_secs_f32() * 1000.0, squads.len());
    // the routes are given a time limit between them
    assert!(worst < Duration::from_millis(12), "{worst:?}");
}

//a player so far from everything that no enemy notices them
pub(super) const NOBODY: Vec2 = Vec2::new(-1.0e6, -1.0e6);
