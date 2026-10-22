//enemies that hurt, chase and shoot, and the unit cap

use super::*;


// ---------- enemies that hurt, chase and shoot; effects; the unit cap ----------

//an enemy that stays put, with a lot of health, that hits for `attack` damage per second
pub(super) fn striker(at: Vec2, attack: f32) -> Enemy {
    let mut enemy = still_enemy(at, 1.0e6);
    enemy.attack = attack;
    enemy
}

pub(super) fn unit_on(position: Vec2) -> Unit {
    Unit::new(position.x, position.y, 150.0)
}

//`count` units spread out inside an enemy so that every one is touching it
pub(super) fn units_inside(enemy: &Enemy, count: usize) -> Vec<Unit> {
    let columns = (count as f32).sqrt().ceil().max(1.0) as usize;
    let side = enemy.size * 0.8;
    let spacing = side / columns as f32;
    (0..count)
        .map(|i| {
            let (col, row) = ((i % columns) as f32, (i / columns) as f32);
            unit_on(enemy.position - Vec2::splat(side / 2.0) + Vec2::new(col, row) * spacing + Vec2::splat(spacing / 2.0))
        })
        .collect()
}

pub(super) fn the_spitter() -> &'static EnemyKind {
    ENEMY_KINDS.iter().find(|kind| kind.range > 0.0).expect("a kind that shoots")
}

#[test]
fn an_enemy_hurts_the_units_inside_it_one_after_another() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemies = vec![striker(at, 150.0)];
    let mut units: Vec<Unit> = (0..5).map(|_| unit_on(at)).collect();
    let mut fx = Effects::default();
    fight(&mut enemies, &mut units, &mut Vec::new(), 1.0, &Upgrades::default(), &mut fx);
    //150 damage: the first unit is used up, the second has half its health left, the rest are untouched
    assert_eq!(units[0].health, 0.0);
    assert!((units[1].health - 50.0).abs() < 0.01);
    assert!(units[2..].iter().all(|unit| unit.health == UNIT_HEALTH));
    assert_eq!(enemies[0].killed, 1);
    //and they hurt it back, five of them at UNIT_DAMAGE a second
    assert!((1.0e6 - enemies[0].health - 5.0 * UNIT_DAMAGE).abs() < 1.0);
}

#[test]
fn units_outside_an_enemy_are_not_hurt_by_it() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemies = vec![striker(at, 1.0e6)];
    let far = at + Vec2::new(enemies[0].size / 2.0 + RADIUS + 5.0, 0.0);
    let mut units = vec![unit_on(far)];
    fight(&mut enemies, &mut units, &mut Vec::new(), 1.0, &Upgrades::default(), &mut Effects::default());
    assert_eq!(units[0].health, UNIT_HEALTH);
    assert_eq!(enemies[0].health, 1.0e6);
}

#[test]
fn the_damage_an_enemy_does_in_a_second_does_not_depend_on_the_frame_rate() {
    let at = Vec2::new(16000.0, 16000.0);
    let lost_in_a_second = |dt: f32| {
        let mut enemies = vec![striker(at, 130.0)];
        let mut units: Vec<Unit> = (0..20).map(|_| unit_on(at)).collect();
        let frames = (1.0 / dt).round() as usize;
        for _ in 0..frames {
            fight(&mut enemies, &mut units, &mut Vec::new(), dt, &Upgrades::default(), &mut Effects::default());
        }
        units.iter().map(|unit| UNIT_HEALTH - unit.health).sum::<f32>()
    };
    for dt in [1.0 / 30.0, 1.0 / 120.0, 1.0 / 240.0] {
        let lost = lost_in_a_second(dt);
        assert!((lost - 130.0).abs() < 1.0, "lost {lost} at dt {dt}");
    }
}

#[test]
fn a_unit_with_no_health_left_does_not_fight_or_get_hit_again() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemies = vec![striker(at, 50.0)];
    let mut dead = unit_on(at);
    dead.health = 0.0;
    let mut units = vec![dead, unit_on(at)];
    fight(&mut enemies, &mut units, &mut Vec::new(), 1.0, &Upgrades::default(), &mut Effects::default());
    //only the living one hurt it, and took the damage
    assert!((1.0e6 - enemies[0].health - UNIT_DAMAGE).abs() < 1.0);
    assert!((units[1].health - 50.0).abs() < 0.01);
    assert_eq!(enemies[0].killed, 0);
}

#[test]
fn a_destroyed_enemy_drops_babies_shakes_the_screen_and_a_big_one_shakes_it_more() {
    let at = Vec2::new(16000.0, 16000.0);
    let destroy = |size: f32| {
        let mut enemy = still_enemy(at, 10.0);
        enemy.size = size;
        enemy.babies = 3;
        let mut enemies = vec![enemy];
        let mut units = vec![unit_on(at)];
        let mut babies = Vec::new();
        let mut fx = Effects::default();
        let destroyed = fight(&mut enemies, &mut units, &mut babies, 0.2, &Upgrades::default(), &mut fx);
        assert_eq!(destroyed, 1);
        assert!(enemies.is_empty());
        assert_eq!(babies.len(), 3);
        assert!(fx.cues.contains(&Cue::Thud));
        assert!(!fx.bursts.is_empty());
        fx.shake
    };
    let small = destroy(40.0);
    let big = destroy(400.0);
    assert!(small > 0.0 && big > small, "{small} vs {big}");
    assert!(big <= 1.0);
}

#[test]
fn burying_removes_the_units_with_no_health_and_counts_them() {
    let mut units: Vec<Unit> = (0..6).map(|i| unit_on(Vec2::new(100.0 + 50.0 * i as f32, 100.0))).collect();
    units[1].health = 0.0;
    units[4].health = -3.0;
    let mut fx = Effects::default();
    assert_eq!(bury_units(&mut units, &mut fx), 2);
    assert_eq!(units.len(), 4);
    assert!(units.iter().all(|unit| unit.health > 0.0));
    assert!(fx.cues.contains(&Cue::Death));
    assert_eq!(fx.bursts.len(), 2);
    //nobody died, nothing to hear
    let mut quiet = Effects::default();
    assert_eq!(bury_units(&mut units, &mut quiet), 0);
    assert!(quiet.cues.is_empty());
}

#[test]
fn hurt_units_heal_slowly_and_never_past_full() {
    let mut units = vec![unit_on(Vec2::ZERO), unit_on(Vec2::ZERO)];
    units[0].health = 20.0;
    heal_units(&mut units, 1.0, &Upgrades::default());
    assert!((units[0].health - (20.0 + UNIT_HEAL_RATE)).abs() < 0.001);
    assert_eq!(units[1].health, UNIT_HEALTH);
    heal_units(&mut units, 1000.0, &Upgrades::default());
    assert_eq!(units[0].health, UNIT_HEALTH);
}

#[test]
fn a_squad_with_nobody_left_alive_is_disbanded() {
    let at = Vec2::new(16000.0, 16000.0);
    let enemies = vec![still_enemy(at, 1.0e6)];
    let mut squads = Squads::new();
    let mut units: Vec<Unit> = (0..3).map(|i| unit_on(at - Vec2::new(300.0 + 20.0 * i as f32, 0.0))).collect();
    send_squad(&mut units, &mut squads, &enemies[0], &mut Effects::default());
    assert_eq!(squads.len(), 1);
    for unit in &mut units {
        unit.health = 0.0;
    }
    bury_units(&mut units, &mut Effects::default());
    update_squads(&mut squads, &enemies, &mut units, &Walls::default(), false);
    assert!(squads.is_empty());
}

#[test]
fn an_enemy_that_notices_the_player_runs_at_them_and_gives_up_when_they_get_far() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemy = Enemy::new(at, &ENEMY_KINDS[0], 1.0);
    enemy.position = at;
    enemy.speed = 100.0;
    let aggro = enemy.aggro;
    assert!(aggro > 0.0);
    let mut enemies = vec![enemy];
    let mut fx = Effects::default();

    //out of sight: it patrols
    let player = at + Vec2::new(aggro + 10.0, 0.0);
    patrol_free(&mut enemies, player, 0.1, 1.0, &mut fx);
    assert!(!enemies[0].chasing);
    assert!(fx.cues.is_empty());

    //in sight: it notices, once, with an exclamation mark
    let player = at + Vec2::new(aggro - 30.0, 0.0);
    let before = enemies[0].position.distance(player);
    patrol_free(&mut enemies, player, 0.1, 1.0, &mut fx);
    assert!(enemies[0].chasing);
    assert_eq!(fx.cues, vec![Cue::Alert]);
    assert_eq!(fx.popups.len(), 1);
    assert_eq!(fx.popups[0].text, "!");
    let gained = before - enemies[0].position.distance(player);
    assert!((gained - 100.0 * CHASE_SPEED * 0.1).abs() < 0.01, "ran {gained}");
    patrol_free(&mut enemies, player, 0.1, 1.0, &mut fx);
    assert_eq!(fx.cues.len(), 1, "it doesn't notice again every frame");

    //it keeps chasing a bit past where it noticed them, and stops when it gets close
    let player = at + Vec2::new(aggro * (CHASE_LEASH - 0.05), 0.0);
    patrol_free(&mut enemies, player, 0.1, 1.0, &mut fx);
    assert!(enemies[0].chasing);
    for _ in 0..400 {
        patrol_free(&mut enemies, player, 0.1, 1.0, &mut fx);
    }
    assert!((enemies[0].position.distance(player) - CHASE_STOP).abs() < 0.5);

    //they get away
    let player = enemies[0].position + Vec2::new(aggro * CHASE_LEASH + 5.0, 0.0);
    patrol_free(&mut enemies, player, 0.1, 1.0, &mut fx);
    assert!(!enemies[0].chasing);
}

#[test]
fn a_fast_enemy_chasing_does_not_run_past_the_player() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemy = Enemy::new(at, &ENEMY_KINDS[0], 1.0);
    enemy.position = at;
    enemy.speed = 1000.0;
    let mut enemies = vec![enemy];
    let player = at + Vec2::new(200.0, 0.0);
    for _ in 0..10 {
        patrol_free(&mut enemies, player, MAX_FRAME_TIME, 3.0, &mut Effects::default());
        assert!(enemies[0].position.x <= player.x - CHASE_STOP + 0.01);
    }
}

#[test]
fn an_enemy_with_no_aggro_never_chases_however_close_the_player_is() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemy = Enemy::new(at, the_spitter(), 1.0);
    enemy.position = at;
    assert_eq!(enemy.aggro, 0.0);
    let mut enemies = vec![enemy];
    patrol_free(&mut enemies, at + Vec2::new(5.0, 0.0), 0.1, 1.0, &mut Effects::default());
    assert!(!enemies[0].chasing);
}

#[test]
fn every_kind_can_hurt_and_only_shooters_and_mortars_have_a_range() {
    for kind in ENEMY_KINDS.iter() {
        assert!(kind.attack > 0.0, "{} has no attack", kind.name);
        assert_eq!(kind.range > 0.0, matches!(kind.behavior, Behavior::Shooter | Behavior::Mortar), "{}", kind.name);
    }
    assert!(ENEMY_KINDS.iter().filter(|kind| kind.behavior == Behavior::Shooter).count() >= 1);
    assert_eq!(the_spitter().behavior, Behavior::Shooter);
}

#[test]
fn a_shooter_fires_a_volley_at_the_nearest_unit_in_range() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemy = Enemy::new(at, the_spitter(), 3.0);
    enemy.position = at;
    enemy.shot_in = 0.0;
    assert_eq!(enemy.volley, 3);
    let mut enemies = vec![enemy];
    let units = vec![unit_on(at + Vec2::new(300.0, 0.0)), unit_on(at + Vec2::new(0.0, 500.0)), unit_on(at + Vec2::new(2000.0, 0.0))];
    let mut shots = Vec::new();
    let mut fx = Effects::default();
    shoot_enemies(&mut enemies, &units, &mut shots, 1.0 / 60.0, &mut fx);
    assert_eq!(shots.len(), 3);
    //the middle one goes straight at the nearest unit, the others to each side
    assert!((shots[1].velocity - Vec2::new(SHOT_SPEED, 0.0)).length() < 0.01);
    assert!(shots[0].velocity.y < 0.0 && shots[2].velocity.y > 0.0 || shots[0].velocity.y > 0.0 && shots[2].velocity.y < 0.0);
    assert!(shots.iter().all(|shot| (shot.velocity.length() - SHOT_SPEED).abs() < 0.01));
    assert_eq!(fx.cues, vec![Cue::Shoot]);
    //it has to wait before the next
    assert!(enemies[0].shot_in > 0.5);
    shoot_enemies(&mut enemies, &units, &mut shots, 1.0 / 60.0, &mut fx);
    assert_eq!(shots.len(), 3);
}

#[test]
fn a_shooter_with_nobody_in_range_holds_its_fire() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemy = Enemy::new(at, the_spitter(), 1.0);
    enemy.position = at;
    enemy.shot_in = 0.0;
    let range = enemy.range;
    let mut enemies = vec![enemy];
    let units = vec![unit_on(at + Vec2::new(range + 50.0, 0.0))];
    let mut shots = Vec::new();
    let mut fx = Effects::default();
    shoot_enemies(&mut enemies, &units, &mut shots, 1.0 / 60.0, &mut fx);
    assert!(shots.is_empty());
    assert!(fx.cues.is_empty());
    //and looks again soon, not after a whole interval
    assert!(enemies[0].shot_in <= 0.25);
}

#[test]
fn enemies_that_do_not_shoot_fire_nothing() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemy = plain_enemy(at);
    enemy.shot_in = 0.0;
    let mut enemies = vec![enemy];
    let units = vec![unit_on(at + Vec2::new(100.0, 0.0))];
    let mut shots = Vec::new();
    shoot_enemies(&mut enemies, &units, &mut shots, 1.0, &mut Effects::default());
    assert!(shots.is_empty());
}

#[test]
fn a_volley_grows_with_the_enemy_but_never_past_the_most() {
    for scale in [1.0, 2.5, 4.0, MAX_ENEMY_SCALE] {
        let enemy = Enemy::new(Vec2::new(16000.0, 16000.0), the_spitter(), scale);
        assert!(enemy.volley >= 1 && enemy.volley <= MAX_VOLLEY, "{} at {scale}", enemy.volley);
    }
    assert!(Enemy::new(Vec2::new(16000.0, 16000.0), the_spitter(), 4.0).volley > Enemy::new(Vec2::new(16000.0, 16000.0), the_spitter(), 1.0).volley);
}

#[test]
fn a_shot_hurts_the_one_unit_it_hits_and_is_used_up() {
    let start = Vec2::new(1000.0, 1000.0);
    let mut shots = vec![Shot::new(start, Vec2::new(SHOT_SPEED, 0.0))];
    //two units in the same spot: only one takes it
    let mut units = vec![unit_on(start + Vec2::new(120.0, 0.0)), unit_on(start + Vec2::new(120.0, 0.0))];
    let mut fx = Effects::default();
    for _ in 0..60 {
        update_shots(&mut shots, &mut units, &Walls::default(), 1.0 / 60.0, &Upgrades::default(), &mut fx);
    }
    assert!(shots.is_empty());
    let total: f32 = units.iter().map(|unit| UNIT_HEALTH - unit.health).sum();
    assert!((total - SHOT_DAMAGE).abs() < 0.01, "took {total}");
    assert!(!fx.bursts.is_empty());
}

#[test]
fn a_shot_cannot_skip_over_a_unit_in_a_long_frame() {
    //1560 px/s at a 50 ms frame is 78 px a frame, more than three times the reach of a unit
    for offset in [150.0, 163.0, 190.0, 211.0, 250.0] {
        let start = Vec2::new(1000.0, 1000.0);
        let mut shots = vec![Shot::new(start, Vec2::new(SHOT_SPEED * 3.0, 0.0))];
        let mut units = vec![unit_on(start + Vec2::new(offset, 0.0))];
        for _ in 0..3 {
            update_shots(&mut shots, &mut units, &Walls::default(), MAX_FRAME_TIME, &Upgrades::default(), &mut Effects::default());
        }
        assert!(units[0].health < UNIT_HEALTH, "missed a unit {offset} px away");
    }
}

#[test]
fn a_wall_stops_a_shot_and_protects_the_unit_behind_it() {
    let start = Vec2::new(100.0, 100.0);
    //column 4 is x 256 to 320
    let walls = walls_of(&[(4, 1)]);
    let mut shots = vec![Shot::new(start, Vec2::new(SHOT_SPEED, 0.0))];
    let mut units = vec![unit_on(Vec2::new(380.0, 100.0))];
    let mut fx = Effects::default();
    for _ in 0..120 {
        update_shots(&mut shots, &mut units, &walls, 1.0 / 60.0, &Upgrades::default(), &mut fx);
    }
    assert!(shots.is_empty());
    assert_eq!(units[0].health, UNIT_HEALTH);
    assert!(!fx.bursts.is_empty(), "it makes a spark where it hits");
}

#[test]
fn a_shot_that_hits_nothing_gives_up_after_its_life() {
    let mut shots = vec![Shot::new(Vec2::new(100.0, 16000.0), Vec2::new(SHOT_SPEED, 0.0))];
    let mut units = Vec::new();
    let frames_alive = (SHOT_LIFE * 60.0) as usize - 3;
    for _ in 0..frames_alive {
        update_shots(&mut shots, &mut units, &Walls::default(), 1.0 / 60.0, &Upgrades::default(), &mut Effects::default());
    }
    assert_eq!(shots.len(), 1);
    for _ in 0..8 {
        update_shots(&mut shots, &mut units, &Walls::default(), 1.0 / 60.0, &Upgrades::default(), &mut Effects::default());
    }
    assert!(shots.is_empty());
}

#[test]
fn a_shot_leaving_the_world_is_removed() {
    let mut shots = vec![Shot::new(Vec2::new(WORLD_SIZE - 5.0, 100.0), Vec2::new(SHOT_SPEED, 0.0))];
    update_shots(&mut shots, &mut Vec::new(), &Walls::default(), 0.1, &Upgrades::default(), &mut Effects::default());
    assert!(shots.is_empty());
}

#[test]
fn babies_stop_growing_at_the_unit_cap() {
    let player = Vec2::new(16000.0, 16000.0);
    let mut units: Vec<Unit> = (0..DEFAULT_ARMY_LIMIT - 2).map(|_| unit_on(player)).collect();
    let mut babies: Vec<Baby> = (0..5).map(|i| Baby { position: player + Vec2::new(10.0 * i as f32, 0.0), age: 5.0 }).collect();
    let mut fx = Effects::default();
    grow_babies(&mut babies, &mut units, player.x, player.y, 1.0 / 60.0, 160.0, DEFAULT_ARMY_LIMIT, &mut fx);
    assert_eq!(units.len(), DEFAULT_ARMY_LIMIT);
    //the other three wait where they are, for room
    assert_eq!(babies.len(), 3);
    grow_babies(&mut babies, &mut units, player.x, player.y, 1.0 / 60.0, 160.0, DEFAULT_ARMY_LIMIT, &mut fx);
    assert_eq!(units.len(), DEFAULT_ARMY_LIMIT);
    assert_eq!(babies.len(), 3);
    //and when units die they grow into the room
    units.truncate(DEFAULT_ARMY_LIMIT - 1);
    grow_babies(&mut babies, &mut units, player.x, player.y, 1.0 / 60.0, 160.0, DEFAULT_ARMY_LIMIT, &mut fx);
    assert_eq!(units.len(), DEFAULT_ARMY_LIMIT);
    assert_eq!(babies.len(), 2);
}

#[test]
fn an_enemy_drops_no_more_babies_than_the_army_has_room_for() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemy = plain_enemy(at);
    enemy.babies = 20;
    let mut babies = Vec::new();
    drop_babies(&enemy, &mut babies, DEFAULT_ARMY_LIMIT - 5, 1.0, DEFAULT_ARMY_LIMIT);
    assert_eq!(babies.len(), 5);
    //babies already waiting count against the room
    drop_babies(&enemy, &mut babies, DEFAULT_ARMY_LIMIT - 5, 1.0, DEFAULT_ARMY_LIMIT);
    assert_eq!(babies.len(), 5);
    let mut none = Vec::new();
    drop_babies(&enemy, &mut none, DEFAULT_ARMY_LIMIT, 1.0, DEFAULT_ARMY_LIMIT);
    assert!(none.is_empty());
    let mut all = Vec::new();
    drop_babies(&enemy, &mut all, 100, 1.0, DEFAULT_ARMY_LIMIT);
    assert_eq!(all.len(), 20);
}

#[test]
fn the_starting_army_and_the_milestones_fit_under_the_unit_cap() {
    assert!(MIN_ARMY_LIMIT <= DEFAULT_ARMY_LIMIT && DEFAULT_ARMY_LIMIT <= MAX_ARMY_LIMIT);
    assert_eq!((MAX_ARMY_LIMIT - MIN_ARMY_LIMIT) % ARMY_LIMIT_STEP, 0, "the slider moves in whole steps from one end to the other");
    assert_eq!((DEFAULT_ARMY_LIMIT - MIN_ARMY_LIMIT) % ARMY_LIMIT_STEP, 0);
    //the smallest limit still holds the starting army
    assert!(START_UNITS_MAX as usize <= MIN_ARMY_LIMIT);
    assert!(MILESTONES.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(*MILESTONES.last().unwrap(), MAX_ARMY_LIMIT);
    //the full speed and the full pickup range come before the army can't grow any more, whatever the limit is
    assert!(SPEED_BOOST_UNITS <= MIN_ARMY_LIMIT && PICKUP_FULL_UNITS <= MIN_ARMY_LIMIT);
}

#[test]
fn the_army_is_congratulated_at_round_sizes_below_the_limit_and_then_at_the_limit() {
    let limit = DEFAULT_ARMY_LIMIT;
    assert_eq!(next_milestone(0, limit), 100);
    assert_eq!(next_milestone(99, limit), 100);
    assert_eq!(next_milestone(100, limit), 250);
    assert_eq!(next_milestone(2000, limit), 3000);
    //each is further on than the army that got there, and no round size below the limit is skipped
    let mut size = 0;
    let mut seen = Vec::new();
    loop {
        size = next_milestone(size, limit);
        if size == usize::MAX {
            break;
        }
        assert!(seen.last().is_none_or(|&last| size > last));
        seen.push(size);
    }
    let round: Vec<usize> = MILESTONES.iter().copied().filter(|&milestone| milestone < limit).collect();
    assert_eq!(&seen[..seen.len() - 1], &round[..]);
    assert_eq!(seen.last(), Some(&limit), "and the last is the limit itself, which is full strength");
    //an army at or past the limit has none to come
    assert_eq!(next_milestone(limit, limit), usize::MAX);
    assert_eq!(next_milestone(limit + 500, limit), usize::MAX);
    //a limit that is itself a round size comes once, and a bigger one has more sizes below it
    assert_eq!(next_milestone(7500, 10000), 10000);
    assert_eq!(next_milestone(7500, 50000), 10000);
    assert_eq!(next_milestone(40000, 50000), 50000);
    assert_eq!(next_milestone(0, MIN_ARMY_LIMIT), 100);
    assert_eq!(next_milestone(3000, MIN_ARMY_LIMIT), MIN_ARMY_LIMIT);
}

//what happens when a squad the game would send fights an enemy of each kind, the army being `army` units
//as (the squad the enemy wants, the squad sent, units lost, seconds it took to destroy it or f32::MAX if it wasn't)
pub(super) fn lost_destroying(kind: &EnemyKind, army: usize) -> (usize, usize, usize, f32) {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemy = Enemy::new(at, kind, army_scale(army));
    enemy.position = at;
    enemy.speed = 0.0;
    let wanted = squad_size(&enemy);
    let squad = wanted.min(army);
    let mut units = units_inside(&enemy, squad);
    let id = enemy.id;
    let mut enemies = vec![enemy];
    let mut seconds = 0.0;
    //(a splitter leaves smaller ones behind, it is the one that was sent at that counts)
    let alive = |enemies: &[Enemy]| enemies.iter().any(|enemy| enemy.id == id);
    while alive(&enemies) && !units.is_empty() && seconds < 30.0 {
        fight(&mut enemies, &mut units, &mut Vec::new(), 1.0 / 60.0, &Upgrades::default(), &mut Effects::default());
        bury_units(&mut units, &mut Effects::default());
        seconds += 1.0 / 60.0;
    }
    (wanted, squad, squad - units.len(), if alive(&enemies) { f32::MAX } else { seconds })
}

#[test]
fn a_squad_the_size_the_game_sends_wins_with_losses_that_are_a_minority() {
    //an army of 100 and one of 1000: every kind but the one that outguns the army is beaten in a few seconds, and the
    //squad loses fewer than half its units doing it
    for army in [100, 1000] {
        for kind in ENEMY_KINDS.iter() {
            //repeat it, enemies are nudged at random
            for _ in 0..5 {
                let (wanted, squad, lost, seconds) = lost_destroying(kind, army);
                if wanted > army {
                    //too big a job for the whole army, the player has to do something else
                    continue;
                }
                assert!(seconds < 12.0, "{} took {seconds} s with an army of {army} (squad {squad})", kind.name);
                assert!(lost * 2 <= squad + 2, "{} cost {lost} of {squad} with an army of {army}", kind.name);
            }
        }
    }
}

#[test]
fn a_far_too_small_squad_loses_to_a_tough_enemy() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemy = Enemy::new(at, ENEMY_KINDS.iter().max_by(|a, b| a.attack.total_cmp(&b.attack)).unwrap(), 4.0);
    enemy.position = at;
    enemy.speed = 0.0;
    let mut units = units_inside(&enemy, 6);
    let mut enemies = vec![enemy];
    for _ in 0..(10 * 60) {
        fight(&mut enemies, &mut units, &mut Vec::new(), 1.0 / 60.0, &Upgrades::default(), &mut Effects::default());
        bury_units(&mut units, &mut Effects::default());
    }
    assert!(units.is_empty(), "{} survived", units.len());
    assert_eq!(enemies.len(), 1);
}

#[test]
fn fighting_thousands_of_units_costs_little_of_a_frame() {
    let mut rng_x = 0.0f32;
    let mut enemies: Vec<Enemy> = (0..ENEMY_COUNT)
        .map(|i| {
            let mut enemy = still_enemy(Vec2::new(8000.0 + 400.0 * i as f32 % 12000.0, 12000.0 + 700.0 * (i / 20) as f32), 1.0e6);
            enemy.attack = 200.0;
            enemy
        })
        .collect();
    let mut units: Vec<Unit> = (0..DEFAULT_ARMY_LIMIT)
        .map(|i| {
            rng_x += 7.3;
            let enemy = &enemies[i % ENEMY_COUNT];
            unit_on(enemy.position + Vec2::new((rng_x % 40.0) - 20.0, ((i / ENEMY_COUNT) as f32 * 3.0) % 40.0 - 20.0))
        })
        .collect();
    let mut shots: Vec<Shot> = (0..800)
        .map(|i| Shot::new(Vec2::new(8000.0 + (i % 40) as f32 * 300.0, 12000.0 + (i / 40) as f32 * 100.0), Vec2::new(SHOT_SPEED, 40.0)))
        .collect();
    let mut babies = Vec::new();
    let frames = 120;
    let started = Instant::now();
    for _ in 0..frames {
        let mut fx = Effects::default();
        shoot_enemies(&mut enemies, &units, &mut shots, 1.0 / 120.0, &mut fx);
        update_shots(&mut shots, &mut units, &Walls::default(), 1.0 / 120.0, &Upgrades::default(), &mut fx);
        heal_units(&mut units, 1.0 / 120.0, &Upgrades::default());
        fight(&mut enemies, &mut units, &mut babies, 1.0 / 120.0, &Upgrades::default(), &mut fx);
        tick_enemy_numbers(&mut enemies, 1.0 / 120.0, &mut fx);
        bury_units(&mut units, &mut fx);
        //units that die in the test are put back so every frame is as busy as the first
        while units.len() < DEFAULT_ARMY_LIMIT {
            let enemy = &enemies[units.len() % ENEMY_COUNT];
            units.push(unit_on(enemy.position));
        }
        for shot in shots.iter_mut() {
            shot.age = 0.0;
        }
        while shots.len() < 800 {
            shots.push(Shot::new(Vec2::new(8000.0, 12000.0), Vec2::new(SHOT_SPEED, 40.0)));
        }
    }
    let per_frame = started.elapsed().as_secs_f32() / frames as f32 * 1000.0;
    eprintln!("fight, shots and burying with {} units: {per_frame:.2} ms a frame", DEFAULT_ARMY_LIMIT);
    assert!(per_frame < 6.0, "{per_frame} ms a frame");
}

