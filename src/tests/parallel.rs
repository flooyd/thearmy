//the army's work shared out over the processor's cores gives exactly what one core going through it in order would

use super::*;

#[test]
fn sharing_a_list_out_does_every_item_once_and_says_where_each_chunk_starts() {
    for len in [0usize, 1, 7, PARALLEL_FROM - 1, PARALLEL_FROM, PARALLEL_FROM + 1, 4001, 10_007] {
        let mut items: Vec<usize> = vec![usize::MAX; len];
        share_out(&mut items, |first, chunk| {
            for (offset, item) in chunk.iter_mut().enumerate() {
                //an item written twice, or never, or given the wrong place, shows up below
                assert_eq!(*item, usize::MAX);
                *item = first + offset;
            }
        });
        assert!(items.iter().enumerate().all(|(i, &item)| item == i), "{len} items");
    }
}

#[test]
fn a_short_list_is_done_in_one_go_on_the_thread_that_asked() {
    let asked = std::thread::current().id();
    let mut items = vec![0u8; PARALLEL_FROM - 1];
    let mut calls = 0;
    let calls_ref = std::sync::Mutex::new(&mut calls);
    share_out(&mut items, |first, chunk| {
        assert_eq!((first, chunk.len()), (0, PARALLEL_FROM - 1));
        assert_eq!(std::thread::current().id(), asked);
        **calls_ref.lock().unwrap() += 1;
    });
    assert_eq!(calls, 1);
}

//a big army round the player and the walls of the starting house, some in towers
fn an_army_round_the_player(count: usize, player: Vec2) -> Vec<Unit> {
    (0..count)
        .map(|i| {
            let at = random_position_around(player.x, player.y, 40.0, 500.0);
            let mut unit = Unit::new(at.x, at.y, rand::gen_range(UNIT_MIN_SPEED, UNIT_MAX_SPEED));
            if i % 97 == 0 {
                unit.garrison = Some((3, 3));
            }
            unit
        })
        .collect()
}

#[test]
fn moving_a_big_army_over_the_cores_is_exactly_what_moving_it_in_order_gives() {
    let player = Vec2::splat(WORLD_SIZE / 2.0);
    let walls = starting_house();
    let field = field_for(player, &walls);
    let count = PARALLEL_FROM * 3;
    let mut shared = an_army_round_the_player(count, player);
    let mut in_order = shared.clone();
    //a few hundred of them are sent at an enemy, the same ones in both
    let enemy = still_enemy(player + Vec2::new(900.0, 100.0), 300.0 * UNIT_DAMAGE * SQUAD_KILL_SECONDS);
    let mut squads = Squads::new();
    let sent = send_squad(&mut shared, &mut squads, &enemy, &mut Effects::default());
    assert!(sent > 100, "{sent}");
    for (unit, same) in shared.iter().zip(in_order.iter_mut()) {
        same.target = unit.target;
        same.steer_in = unit.steer_in;
    }
    let dt = 1.0 / 120.0;
    for frame in 0..40 {
        //the player walks about, so the units keep working out where to head
        let player = player + Vec2::new((frame as f32 * 0.2).cos(), (frame as f32 * 0.2).sin()) * 150.0;
        update_units(&mut shared, player.x, player.y, dt, &walls, &field, 2.0, &squads);
        //the same, a unit at a time
        let started: Vec<Vec2> = in_order.iter().map(|unit| Vec2::new(unit.x, unit.y)).collect();
        for unit in in_order.iter_mut().filter(|unit| unit.garrison.is_none()) {
            match unit.target.and_then(|enemy| squads.get(&enemy)) {
                Some(squad) => step_unit(unit, squad.position, dt, &walls, &squad.field, 2.0),
                None => {
                    unit.target = None;
                    step_unit(unit, player, dt, &walls, &field, 2.0);
                }
            }
        }
        spread_units(&mut in_order);
        for (unit, &from) in in_order.iter_mut().zip(&started) {
            if unit.garrison.is_some() {
                continue;
            }
            let settled = move_clear_of_walls(from, Vec2::new(unit.x, unit.y), &walls);
            unit.x = settled.x;
            unit.y = settled.y;
            clamp_to_world(&mut unit.x, &mut unit.y);
            unit.velocity = smoothed_velocity(unit.velocity, from, Vec2::new(unit.x, unit.y), dt);
        }
        for (i, (a, b)) in shared.iter().zip(&in_order).enumerate() {
            assert!(a.x == b.x && a.y == b.y && a.velocity == b.velocity && a.target == b.target, "frame {frame}, unit {i}: ({}, {}) against ({}, {})", a.x, a.y, b.x, b.y);
        }
    }
}

#[test]
fn a_big_army_is_moved_in_a_small_share_of_a_frame() {
    let player = Vec2::splat(WORLD_SIZE / 2.0);
    let walls = starting_house();
    let field = field_for(player, &walls);
    let mut units = an_army_round_the_player(DEFAULT_ARMY_LIMIT, player);
    let squads = Squads::new();
    //the first frames start the threads
    for _ in 0..5 {
        update_units(&mut units, player.x, player.y, 1.0 / 120.0, &walls, &field, 2.0, &squads);
    }
    let started = Instant::now();
    let frames = 60;
    for _ in 0..frames {
        update_units(&mut units, player.x, player.y, 1.0 / 120.0, &walls, &field, 2.0, &squads);
    }
    let per_frame = started.elapsed().as_secs_f32() / frames as f32 * 1000.0;
    eprintln!("moving {} units: {per_frame:.2} ms a frame, with {} workers", units.len(), worker_count());
    //about 1.3 ms on its own, other tests running at the same time make it several times that
    assert!(per_frame < 8.0, "{per_frame} ms a frame");
}
