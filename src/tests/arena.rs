//the later bosses and what is new about them: splitting, gravity, an armoured front, barriers, the citadel and its lord, and
//how a boss is shown off and its fall is felt

use super::*;

fn army_near(at: Vec2, count: usize) -> Vec<Unit> {
    (0..count).map(|i| unit_on(at + Vec2::new((i % 20) as f32 * 30.0, (i / 20) as f32 * 30.0))).collect()
}

fn boss_of(name: &str, army: usize, at: Vec2) -> Enemy {
    let def = BOSSES.iter().chain(std::iter::once(&FINAL_BOSS)).find(|def| def.name == name).expect("no such boss");
    Enemy::boss(def, army, 0, at)
}

fn step_boss(enemies: &mut Vec<Enemy>, units: &[Unit], player: Vec2, seconds: f32) -> (Effects, Vec<Shot>) {
    let (mut fx, mut shots) = (Effects::default(), Vec::new());
    for _ in 0..(seconds / 0.01) as usize {
        boss_act(enemies, units, player, 0.01, &mut shots, &mut fx);
    }
    (fx, shots)
}

#[test]
fn there_are_seven_bosses_in_turn_and_a_lord_of_the_citadel_of_its_own() {
    assert_eq!(BOSSES.len(), 7);
    assert!(BOSSES.iter().all(|def| def.name != FINAL_BOSS.name));
    let names: Vec<&str> = BOSSES.iter().map(|def| def.name).collect();
    assert_eq!(&names[4..], ["The Swarm Queen", "The Singularity", "The Bastion"]);
    assert_eq!(BOSSES.iter().filter(|def| def.splits).count(), 1);
    assert_eq!(BOSSES.iter().filter(|def| def.flank).count(), 1);
    assert!(BOSSES.iter().any(|def| def.phases.iter().any(|phase| phase.casts.iter().any(|cast| cast.pattern == Pattern::Pull))));
    //the lord is checked like the others
    assert_eq!(kind_named(FINAL_BOSS.minion).name, FINAL_BOSS.minion);
    assert!(FINAL_BOSS.phases[0].below == 1.0 && FINAL_BOSS.min_army == CITADEL_MIN_ARMY);
    for pair in FINAL_BOSS.phases.windows(2) {
        assert!(pair[1].below < pair[0].below && pair[1].below > LAST_STAND_AT);
    }
    for phase in FINAL_BOSS.phases {
        assert!(phase.casts.iter().all(|cast| cast.every >= 2.0), "{}", phase.name);
    }
    //the later ones want bigger armies than the Eclipse did
    assert!(BOSSES[4].min_army > BOSSES[3].min_army);
}

#[test]
fn a_swarm_queen_tears_in_two_twice_with_the_health_shared() {
    let units = army_near(MIDDLE, 200);
    let mut enemies = vec![boss_of("The Swarm Queen", 800, MIDDLE + Vec2::new(3000.0, 0.0))];
    let (family, total) = {
        let state = enemies[0].boss.as_ref().unwrap();
        (state.family, enemies[0].max_health)
    };
    assert_eq!(family, enemies[0].id);
    enemies[0].health = enemies[0].max_health * 0.5;
    let size = enemies[0].size;
    //she goes into her next phase first, and splits when that pause is over
    let (fx, _) = step_boss(&mut enemies, &units, MIDDLE, PHASE_SHIFT_SECONDS + 0.3);
    let queens: Vec<&Enemy> = enemies.iter().filter(|enemy| enemy.boss.is_some()).collect();
    assert_eq!(queens.len(), 2, "she splits at {} of her health", SPLIT_AT[0]);
    assert_ne!(queens[0].id, queens[1].id);
    assert!(queens.iter().all(|queen| queen.boss.as_ref().unwrap().family == family && queen.boss.as_ref().unwrap().generation == 1));
    assert!(queens.iter().all(|queen| (queen.health - total * 0.25).abs() < 1.0 && (queen.max_health - total * 0.5).abs() < 1.0), "half each");
    assert!(queens.iter().all(|queen| queen.size < size && queen.size > size * 0.7));
    assert!(queens[0].position.distance(queens[1].position) > queens[0].size * 0.5, "they stand apart");
    assert!(queens.iter().all(|queen| queen.boss.as_ref().unwrap().shifting > 0.0), "a moment to take it in");
    assert!(fx.cues.contains(&Cue::Split) && fx.announcements.iter().any(|(text, _)| text.contains("DIVIDES")));
    let babies = Enemy::boss(&BOSSES[4], 800, 0, MIDDLE).babies;
    assert!(queens.iter().map(|queen| queen.babies).sum::<usize>() >= babies, "no babies are lost");
    //the family's health bar goes on from where it was
    let share = family_health(&enemies, queens[0]);
    assert!((share - 0.5).abs() < 0.01, "{share}");
    //not again until she is hurt more
    let (fx, _) = step_boss(&mut enemies, &units, MIDDLE, 1.5);
    assert_eq!(enemies.iter().filter(|enemy| enemy.boss.is_some()).count(), 2, "{} cues", fx.cues.len());
    //hurt each half to a share of its health and it does it again
    for enemy in enemies.iter_mut().filter(|enemy| enemy.boss.is_some()) {
        enemy.health = enemy.max_health * 0.25;
    }
    step_boss(&mut enemies, &units, MIDDLE, PHASE_SHIFT_SECONDS + 0.3);
    assert_eq!(enemies.iter().filter(|enemy| enemy.boss.is_some()).count(), 4);
    //and no more after that
    for enemy in enemies.iter_mut().filter(|enemy| enemy.boss.is_some()) {
        enemy.health = enemy.max_health * 0.05;
    }
    step_boss(&mut enemies, &units, MIDDLE, 3.0);
    assert_eq!(enemies.iter().filter(|enemy| enemy.boss.is_some()).count(), 4);
    assert!(enemies.iter().filter(|enemy| enemy.boss.is_some()).all(|queen| queen.boss.as_ref().unwrap().generation == 2));
}

#[test]
fn the_director_waits_for_every_part_of_a_boss_that_split() {
    let units = army_near(MIDDLE, 100);
    let mut director = Director::new();
    director.next_boss_in = 0.0;
    director.slain = 4;
    let mut enemies = Vec::new();
    assert_eq!(director.update(&mut enemies, MIDDLE, 5000, 0.1), Some(Beat::Arrived("The Swarm Queen")));
    assert_eq!(director.boss, Some(enemies[0].id));
    enemies[0].health = enemies[0].max_health * 0.5;
    step_boss(&mut enemies, &units, MIDDLE, PHASE_SHIFT_SECONDS + 0.3);
    assert_eq!(enemies.iter().filter(|enemy| enemy.boss.is_some()).count(), 2);
    //the first one is gone, the other is not
    enemies.remove(0);
    enemies.retain(|enemy| enemy.boss.is_some());
    assert_eq!(director.update(&mut enemies, MIDDLE, 5000, 0.1), None, "one half is still alive");
    enemies.clear();
    assert!(matches!(director.update(&mut enemies, MIDDLE, 5000, 0.1), Some(Beat::Slain { name: "The Swarm Queen", .. })));
}

#[test]
fn the_director_holds_back_while_the_lord_of_the_citadel_is_alive() {
    let mut director = Director::new();
    director.next_boss_in = 0.0;
    let mut lord = boss_of("The Sovereign", 800, MIDDLE);
    lord.site = Some(7);
    let mut enemies = vec![lord];
    for _ in 0..50 {
        assert_eq!(director.update(&mut enemies, MIDDLE, 10_000, 1.0), None);
    }
    assert_eq!(director.next_boss_in, 0.0, "the countdown waits too");
    assert!(director.boss.is_none());
    enemies.clear();
    assert_eq!(director.update(&mut enemies, MIDDLE, 10_000, 0.1), Some(Beat::Arrived(BOSSES[0].name)));
}

#[test]
fn a_gravity_well_drags_the_army_and_less_the_player_and_is_held_off_by_walls() {
    let units_at = MIDDLE + Vec2::new(600.0, 0.0);
    let units = army_near(units_at, 40);
    let mut boss = boss_of("The Singularity", 1000, MIDDLE);
    let mut state = boss.boss.clone().unwrap();
    let (mut fx, mut born, mut shots) = (Effects::default(), Vec::new(), Vec::new());
    cast_pattern(&mut boss, &mut state, Pattern::Pull, &units, units_at, 0, &mut born, &mut shots, &mut fx);
    assert_eq!(fx.pulls.len(), 1);
    assert!(fx.cues.contains(&Cue::Pull));
    let pull = &fx.pulls[0];
    assert_eq!(pull.position, boss.position);
    assert!(pull.radius > 1000.0 && pull.left >= 2.0);
    //it goes off where they end up
    let blast = fx.hazards.iter().find(|hazard| hazard.position == boss.position).expect("a blast at the middle");
    assert!((blast.warn - pull.left).abs() < 1e-4 && blast.damage >= UNIT_HEALTH && blast.radius > boss.size * 0.5);
    //the units are dragged towards the middle, at PULL_SPEED
    let mut pulls = std::mem::take(&mut fx.pulls);
    let mut units = units;
    let before: Vec<f32> = units.iter().map(|unit| Vec2::new(unit.x, unit.y).distance(boss.position)).collect();
    let player = MIDDLE + Vec2::new(0.0, 700.0);
    let moved_player = update_pulls(&mut pulls, &mut units, player, &Walls::default(), 0.1);
    for (unit, before) in units.iter().zip(&before) {
        let after = Vec2::new(unit.x, unit.y).distance(boss.position);
        assert!((before - after - PULL_SPEED * 0.1).abs() < 0.5, "{before} -> {after}");
    }
    let player_pull = player.distance(boss.position) - moved_player.distance(boss.position);
    assert!((player_pull - PULL_SPEED * PULL_PLAYER_SHARE * 0.1).abs() < 0.5, "{player_pull}");
    //out of its reach nothing moves
    let mut far = vec![unit_on(boss.position + Vec2::new(pull_radius(&pulls) + 500.0, 0.0))];
    let far_at = Vec2::new(far[0].x, far[0].y);
    update_pulls(&mut pulls, &mut far, far_at, &Walls::default(), 0.1);
    assert_eq!(Vec2::new(far[0].x, far[0].y), far_at);
    //a wall holds a unit
    let (col, row) = tile_of(MIDDLE.x + 300.0, MIDDLE.y);
    let mut walls = Walls::default();
    for r in (row - 6)..=(row + 6) {
        walls.insert((col, r), made(WallKind::Wall, Side::Friendly, col, r));
    }
    let mut behind = vec![unit_on(Vec2::new(col as f32 * TILE_SIZE + TILE_SIZE + 40.0, row as f32 * TILE_SIZE + 30.0))];
    for _ in 0..60 {
        update_pulls(&mut pulls, &mut behind, Vec2::new(-1e5, -1e5), &walls, 0.05);
        if pulls.is_empty() {
            pulls.push(Pull { position: boss.position, radius: 3000.0, left: 5.0, total: 5.0 });
        }
    }
    assert!(behind[0].x > (col + 1) as f32 * TILE_SIZE, "the unit is held at the wall: {}", behind[0].x);
    //and it wears off
    let mut pulls = vec![Pull { position: boss.position, radius: 500.0, left: 0.5, total: 0.5 }];
    update_pulls(&mut pulls, &mut Vec::new(), player, &Walls::default(), 0.6);
    assert!(pulls.is_empty());
}

fn pull_radius(pulls: &[Pull]) -> f32 {
    pulls[0].radius
}

#[test]
fn an_armoured_front_takes_a_quarter_from_the_units_before_it_and_all_from_the_ones_behind() {
    let mut boss = boss_of("The Bastion", 1500, MIDDLE);
    boss.boss.as_mut().unwrap().facing = Vec2::X;
    let front = MIDDLE + Vec2::new(60.0, 10.0);
    let back = MIDDLE - Vec2::new(60.0, 0.0);
    let side = MIDDLE + Vec2::new(0.0, 60.0);
    assert_eq!(flank_factor(&boss, front), FLANK_FRONT);
    assert_eq!(flank_factor(&boss, back), 1.0);
    assert_eq!(flank_factor(&boss, side), 1.0, "the side is outside the {FLANK_ARC} radians of the front");
    assert_eq!(flank_factor(&boss, MIDDLE + Vec2::new(30.0, 55.0)), FLANK_FRONT, "a bit forward of the side is still the front");
    assert_eq!(flank_factor(&boss, MIDDLE + Vec2::new(-30.0, 55.0)), 1.0);
    //bosses without one take it all from anywhere
    let plain = boss_of("The Hive Mother", 500, MIDDLE);
    assert_eq!(flank_factor(&plain, front), 1.0);
    //in a fight: the same units do four times as much from behind
    let damage_with = |at: Vec2| {
        let mut enemies = vec![boss_of("The Bastion", 1500, MIDDLE)];
        enemies[0].boss.as_mut().unwrap().facing = Vec2::X;
        let mut units: Vec<Unit> = (0..50).map(|_| unit_on(at)).collect();
        let (mut babies, mut fx) = (Vec::new(), Effects::default());
        let before = enemies[0].health;
        fight(&mut enemies, &mut units, &mut babies, 0.1, &Upgrades::default(), &mut fx);
        before - enemies[0].health
    };
    let in_front = damage_with(front);
    let behind = damage_with(back);
    assert!(in_front > 0.0 && (behind / in_front - 1.0 / FLANK_FRONT).abs() < 0.01, "{in_front} against {behind}");
}

#[test]
fn a_boss_turns_to_face_the_player_a_little_slower_than_they_can_run_round_it() {
    let units = army_near(MIDDLE, 10);
    let mut enemies = vec![boss_of("The Bastion", 1500, MIDDLE + Vec2::new(5000.0, 0.0))];
    enemies[0].boss.as_mut().unwrap().facing = Vec2::X;
    let player = enemies[0].position + Vec2::new(0.0, -2000.0); //straight up
    step_boss(&mut enemies, &units, player, 0.5);
    let facing = enemies[0].boss.as_ref().unwrap().facing;
    let turned = Vec2::X.dot(facing).acos();
    assert!((turned - FLANK_TURN * 0.5).abs() < 0.05, "turned {turned}");
    assert!((facing.length() - 1.0).abs() < 1e-3);
    step_boss(&mut enemies, &units, player, 3.0);
    let facing = enemies[0].boss.as_ref().unwrap().facing;
    assert!(facing.dot(Vec2::new(0.0, -1.0)) > 0.999, "it ends up looking at the player: {facing}");
    //and when the player goes round to the other side it turns right round
    let player = enemies[0].position + Vec2::new(0.0, 2000.0);
    step_boss(&mut enemies, &units, player, 4.0);
    assert!(enemies[0].boss.as_ref().unwrap().facing.dot(Vec2::new(0.0, 1.0)) > 0.999);
}

#[test]
fn fortify_raises_barriers_with_gaps_and_holds_the_boss_inside_until_they_are_down() {
    let units = army_near(MIDDLE + Vec2::new(900.0, 0.0), 60);
    let mut boss = boss_of("The Bastion", 1500, Vec2::new(20_000.0, 20_000.0));
    let mut state = boss.boss.clone().unwrap();
    let (mut fx, mut born, mut shots) = (Effects::default(), Vec::new(), Vec::new());
    cast_pattern(&mut boss, &mut state, Pattern::Fortify, &units, MIDDLE, 0, &mut born, &mut shots, &mut fx);
    assert!(!fx.raise.is_empty());
    assert!(fx.raise.iter().all(|wall| wall.kind == WallKind::Barrier && wall.side == Side::Hostile && wall.life == BARRIER_SECONDS));
    let middle = tile_of(boss.position.x, boss.position.y);
    let half = fx.raise.iter().map(|wall| (wall.col - middle.0).abs().max((wall.row - middle.1).abs())).max().unwrap();
    assert!(half as f32 * TILE_SIZE > boss.size / 2.0, "the ring is clear of the boss");
    assert_eq!(fx.raise.len() as i32, 8 * half - 4, "a ring with two gaps of two squares");
    assert!(state.anchor_left > 0.0);
    //they go up where there is room and not on what is there already
    let mut walls = Walls::default();
    let taken = (fx.raise[0].col, fx.raise[0].row);
    walls.insert(taken, made(WallKind::Wall, Side::Friendly, taken.0, taken.1));
    let count = fx.raise.len();
    assert!(raise_walls(&mut walls, &mut fx.raise));
    assert_eq!(walls.len(), count);
    assert_eq!(walls[&taken].side, Side::Friendly);
    assert!(fx.raise.is_empty());
    assert!(!raise_walls(&mut walls, &mut fx.raise));
    //the ring is something the army has to get through, the boss is not stopped by it
    assert!(!walls.values().filter(|wall| wall.side == Side::Hostile).any(|wall| wall.blocks(Mover::Enemy)));
    assert!(walls.values().filter(|wall| wall.side == Side::Hostile).all(|wall| wall.blocks(Mover::Ally)));
    //while it is held it stays where it is, however far the player is
    state.timers.clear();
    boss.boss = Some(state);
    boss.mode = Mode::Roam;
    let mut enemies = vec![boss];
    let at = enemies[0].position;
    let player = at + Vec2::new(5000.0, 0.0);
    let mut fx = Effects::default();
    patrol_free(&mut enemies, player, 1.0, 1.0, &mut fx);
    assert_eq!(enemies[0].position, at);
    //then it gets on with it
    step_boss(&mut enemies, &units, player, BARRIER_SECONDS);
    patrol_free(&mut enemies, player, 1.0, 1.0, &mut fx);
    assert!(enemies[0].position.distance(at) > 1.0);
}

#[test]
fn a_boss_that_keeps_its_distance_backs_off_when_the_player_comes_close_and_follows_when_they_go() {
    let mut fx = Effects::default();
    let keep = BOSSES[6].keeps_away;
    let at = |distance: f32| {
        let mut enemies = vec![boss_of("The Bastion", 1500, MIDDLE)];
        let player = MIDDLE + Vec2::new(distance, 0.0);
        patrol_free(&mut enemies, player, 1.0, 1.0, &mut Effects::default());
        enemies[0].position.x - MIDDLE.x
    };
    assert!(at(keep * 2.0) > 10.0, "it comes closer when it is far");
    assert!(at(keep * 0.5) < -10.0, "it backs off when the player is close");
    assert!(at(keep * 0.9).abs() < 1.0 || at(keep * 0.9) > 0.0, "in between it holds or edges closer");
    let _ = &mut fx;
}

#[test]
fn a_volley_is_a_fan_of_heavy_shots_at_the_army() {
    let units = army_near(MIDDLE, 400);
    let mut boss = boss_of("The Bastion", 1500, MIDDLE - Vec2::new(1500.0, 0.0));
    let mut state = boss.boss.clone().unwrap();
    let (mut fx, mut born, mut shots) = (Effects::default(), Vec::new(), Vec::new());
    cast_pattern(&mut boss, &mut state, Pattern::Volley, &units, MIDDLE, 0, &mut born, &mut shots, &mut fx);
    assert_eq!(shots.len(), 5 + 400 / 400);
    assert!(shots.iter().all(|shot| shot.damage > SHOT_DAMAGE && shot.radius > SHOT_RADIUS && (shot.velocity.length() - 460.0).abs() < 0.1));
    //a fan: all of them point roughly the same way, towards the army
    let aim = (MIDDLE + Vec2::new(300.0, 100.0) - boss.position).normalize();
    assert!(shots.iter().all(|shot| shot.velocity.normalize().dot(aim) > 0.75));
    let spread = shots.iter().map(|shot| shot.velocity.normalize().perp_dot(shots[0].velocity.normalize()).abs()).fold(0.0, f32::max);
    assert!(spread > 0.1, "not all on one line: {spread}");
    assert!(fx.cues.contains(&Cue::Shoot));
}

#[test]
fn shots_and_blasts_from_the_later_bosses_hurt_walls_more_the_bigger_the_army() {
    let units = army_near(MIDDLE, 100);
    let hurt = |army: usize| {
        let mut boss = boss_of("The Eclipse", army, MIDDLE);
        let mut state = boss.boss.clone().unwrap();
        let (mut fx, mut born, mut shots) = (Effects::default(), Vec::new(), Vec::new());
        cast_pattern(&mut boss, &mut state, Pattern::Slam, &units, MIDDLE, 0, &mut born, &mut shots, &mut fx);
        fx.hazards[0].wall_damage
    };
    assert!(hurt(6000) > hurt(600) * 1.2);
}

// ---------- the citadel ----------

fn citadel_scene(army: usize) -> (Vec<Site>, Vec<Enemy>, Walls, Vec<Baby>, Vec<Unit>, Effects) {
    let at = Vec2::new(5000.0, 5000.0);
    (vec![Site::new(SiteKind::Citadel, at)], Vec::new(), Walls::default(), Vec::new(), army_near(at + Vec2::new(0.0, 2000.0), army), Effects::default())
}

#[test]
fn the_citadel_waits_for_a_big_army_and_wakes_with_its_lord_inside_its_walls() {
    let (mut sites, mut enemies, mut walls, mut babies, mut units, mut fx) = citadel_scene(CITADEL_MIN_ARMY - 1);
    let near = sites[0].position + Vec2::new(0.0, TILE_SIZE * 30.0);
    update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, near, 0.016, false, DEFAULT_ARMY_LIMIT, 0, &mut fx);
    assert_eq!(sites[0].state, SiteState::Dormant, "{} units are not enough", units.len());
    let (mut sites, mut enemies, mut walls, mut babies, mut units, mut fx) = citadel_scene(CITADEL_MIN_ARMY);
    let near = sites[0].position + Vec2::new(0.0, TILE_SIZE * 30.0);
    let news = update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, near, 0.016, false, DEFAULT_ARMY_LIMIT, 0, &mut fx);
    assert_eq!(sites[0].state, SiteState::Awake);
    assert!(news.walls_changed && !news.citadel);
    assert_eq!(enemies.len(), 1);
    let lord = &enemies[0];
    assert_eq!(lord.name, FINAL_BOSS.name);
    assert!(lord.boss.is_some() && lord.site == Some(sites[0].id) && !lord.chasing && lord.position == sites[0].position);
    assert!(lord.aggro > 0.0 && lord.aggro < WORLD_SIZE, "it notices the player when they come close, not from across the world");
    assert!(walls.values().all(|wall| wall.side == Side::Hostile) && !walls.is_empty());
    let middle = tile_of(lord.position.x, lord.position.y);
    let half = walls.keys().map(|&(col, row)| (col - middle.0).abs().max((row - middle.1).abs())).max().unwrap();
    assert!(half as f32 * TILE_SIZE > lord.size / 2.0, "the walls stand clear of it");
    assert!(fx.announcements.iter().any(|(text, _)| text.contains("CITADEL")));
}

#[test]
fn the_lord_of_the_citadel_stays_put_and_quiet_until_the_player_comes_close() {
    let (mut sites, mut enemies, mut walls, mut babies, mut units, mut fx) = citadel_scene(CITADEL_MIN_ARMY);
    let near = sites[0].position + Vec2::new(0.0, TILE_SIZE * 40.0);
    update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, near, 0.016, false, DEFAULT_ARMY_LIMIT, 0, &mut fx);
    let at = enemies[0].position;
    let (mut shots, mut fx) = (Vec::new(), Effects::default());
    for _ in 0..3000 {
        patrol_free(&mut enemies, near, 0.01, 1.0, &mut fx);
        boss_act(&mut enemies, &units, near, 0.01, &mut shots, &mut fx);
    }
    assert_eq!(enemies[0].position, at, "it doesn't leave");
    assert_eq!(enemies.len(), 1, "it calls no one");
    assert!(fx.hazards.is_empty() && shots.is_empty(), "and attacks no one");
    //someone comes to the gate
    let close = at + Vec2::new(0.0, enemies[0].aggro * 0.5);
    for _ in 0..200 {
        patrol_free(&mut enemies, close, 0.01, 1.0, &mut fx);
    }
    assert!(enemies[0].chasing);
    for _ in 0..3000 {
        boss_act(&mut enemies, &units, close, 0.01, &mut shots, &mut fx);
        if !fx.hazards.is_empty() || !shots.is_empty() || enemies.len() > 1 {
            return;
        }
    }
    panic!("it never attacked");
}

#[test]
fn when_the_lord_falls_the_citadel_is_cleared_and_the_walls_come_down() {
    let (mut sites, mut enemies, mut walls, mut babies, mut units, mut fx) = citadel_scene(CITADEL_MIN_ARMY);
    let near = sites[0].position + Vec2::new(0.0, TILE_SIZE * 30.0);
    update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, near, 0.016, false, DEFAULT_ARMY_LIMIT, 0, &mut fx);
    enemies.clear();
    let news = update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, near, 0.016, false, DEFAULT_ARMY_LIMIT, 0, &mut fx);
    assert!(news.citadel && news.walls_changed);
    assert_eq!(sites[0].state, SiteState::Cleared);
    assert!(walls.is_empty());
    let news = update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, near, 0.016, false, DEFAULT_ARMY_LIMIT, 0, &mut fx);
    assert!(!news.citadel, "the victory is only once");
}

#[test]
fn a_citadel_the_player_runs_from_resets_and_comes_back_at_full_strength() {
    let (mut sites, mut enemies, mut walls, mut babies, mut units, mut fx) = citadel_scene(CITADEL_MIN_ARMY);
    let near = sites[0].position + Vec2::new(0.0, TILE_SIZE * 30.0);
    update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, near, 0.016, false, DEFAULT_ARMY_LIMIT, 0, &mut fx);
    enemies[0].health = 5.0;
    let far = sites[0].position + Vec2::new(0.0, TILE_SIZE * (SLEEP_TILES + 20.0));
    let news = update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, far, 0.016, false, DEFAULT_ARMY_LIMIT, 0, &mut fx);
    assert_eq!(sites[0].state, SiteState::Dormant);
    assert!(enemies.is_empty() && walls.is_empty() && !news.citadel);
    update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, near, 0.016, false, DEFAULT_ARMY_LIMIT, 0, &mut fx);
    assert_eq!(enemies[0].health, enemies[0].max_health);
}

// ---------- the entrance and the fall ----------

#[test]
fn a_boss_is_shown_off_by_taking_the_camera_to_it_and_back() {
    let boss = boss_of("The Eclipse", 800, MIDDLE);
    let mut intro = Intro::new(&boss);
    assert_eq!((intro.name, intro.at), ("The Eclipse", boss.position));
    assert_eq!(intro.ease(), 0.0);
    let mut highest: f32 = 0.0;
    let mut last = 0.0;
    let mut rose = true;
    while !intro.over() {
        intro.age += 0.01;
        let ease = intro.ease();
        assert!((0.0..=1.0).contains(&ease));
        highest = highest.max(ease);
        //it goes up and then comes down, once
        if ease < last - 1e-6 {
            rose = false;
        } else if !rose && ease > last + 1e-6 {
            panic!("it went up again");
        }
        last = ease;
    }
    assert!(highest > 0.99, "it gets there: {highest}");
    assert!(intro.ease() < 0.01, "and it comes back");
}

#[test]
fn the_world_freezes_when_a_boss_falls_and_then_crawls_back_to_speed() {
    let mut still = Stillness::after_a_boss();
    assert_eq!(still.pace(), 0.0);
    let mut seconds = 0.0;
    let mut paces = Vec::new();
    while seconds < HIT_STOP_SECONDS + SLOWMO_SECONDS + 0.5 {
        paces.push(still.pace());
        still.tick(0.01);
        seconds += 0.01;
    }
    assert_eq!(paces[0], 0.0);
    assert!(paces.contains(&SLOWMO_PACE));
    assert_eq!(*paces.last().unwrap(), 1.0);
    let frozen = paces.iter().filter(|&&pace| pace == 0.0).count() as f32 * 0.01;
    let slow = paces.iter().filter(|&&pace| pace == SLOWMO_PACE).count() as f32 * 0.01;
    assert!((frozen - HIT_STOP_SECONDS).abs() < 0.03 && (slow - SLOWMO_SECONDS).abs() < 0.03, "{frozen} s frozen, {slow} s slow");
    assert_eq!(Stillness::default().pace(), 1.0);
}

#[test]
fn the_halves_of_a_queen_do_not_attack_at_the_same_moment() {
    let units = army_near(MIDDLE, 200);
    let mut enemies = vec![boss_of("The Swarm Queen", 800, MIDDLE + Vec2::new(3000.0, 0.0))];
    enemies[0].health = enemies[0].max_health * 0.5;
    step_boss(&mut enemies, &units, MIDDLE, PHASE_SHIFT_SECONDS + 0.3);
    let queens: Vec<&Enemy> = enemies.iter().filter(|enemy| enemy.boss.is_some()).collect();
    assert_eq!(queens.len(), 2);
    let (a, b) = (&queens[0].boss.as_ref().unwrap().timers, &queens[1].boss.as_ref().unwrap().timers);
    assert_eq!(a.len(), b.len());
    for (first, second) in a.iter().zip(b) {
        assert!((first - second).abs() > 1.0, "{a:?} against {b:?}");
    }
}

type CitadelWorld = (Vec<Site>, Vec<Enemy>, Walls, Vec<Baby>, Vec<Unit>, Effects);

//one frame of the sites in a citadel scene, the lord having fallen `falls` times before
fn citadel_step(world: &mut CitadelWorld, player: Vec2, dt: f32, falls: u32) -> SiteNews {
    let (sites, enemies, walls, babies, units, fx) = world;
    update_sites(sites, enemies, walls, babies, units, player, dt, false, DEFAULT_ARMY_LIMIT, falls, fx)
}

#[test]
fn the_lord_of_the_citadel_is_back_four_minutes_after_it_falls_and_stronger_each_time() {
    let mut world = citadel_scene(CITADEL_MIN_ARMY);
    let near = world.0[0].position + Vec2::new(0.0, TILE_SIZE * 30.0);
    let far = world.0[0].position + Vec2::new(0.0, TILE_SIZE * (SLEEP_TILES + 20.0));
    citadel_step(&mut world, near, 0.016, 0);
    let first = world.1[0].clone();
    assert_eq!(first.boss.as_ref().unwrap().tier, 0);
    world.1.clear();
    assert!(citadel_step(&mut world, near, 0.016, 0).citadel, "the first victory");
    assert_eq!((world.0[0].state, world.0[0].clock), (SiteState::Cleared, CITADEL_RESPAWN_SECONDS));
    assert_eq!(CITADEL_RESPAWN_SECONDS, 240.0, "four minutes");
    //the timer runs wherever the player is, and until it is out the citadel is quiet
    let mut seconds = 0.0;
    while seconds < CITADEL_RESPAWN_SECONDS - 5.0 {
        let news = citadel_step(&mut world, if seconds < 100.0 { near } else { far }, 1.0, 1);
        assert!(!news.citadel_rearmed && !news.citadel && world.1.is_empty());
        seconds += 1.0;
    }
    assert_eq!(world.0[0].state, SiteState::Cleared);
    let mut rearmed = false;
    for _ in 0..10 {
        rearmed |= citadel_step(&mut world, far, 1.0, 1).citadel_rearmed;
    }
    assert!(rearmed && world.0[0].state == SiteState::Dormant);
    assert!(world.5.announcements.iter().any(|(text, _)| text.contains("ARMED AGAIN")));
    //when the player comes back it wakes, tougher and quicker than before
    citadel_step(&mut world, near, 0.016, 1);
    assert_eq!(world.0[0].state, SiteState::Awake);
    let second = world.1[0].clone();
    assert_eq!(second.boss.as_ref().unwrap().tier, 1);
    assert!((second.max_health / first.max_health - (1.0 + BOSS_TIER_HEALTH)).abs() < 0.01, "{} against {}", second.max_health, first.max_health);
    assert!(second.attack > first.attack * 1.4 && second.speed > first.speed);
    assert!(world.5.announcements.iter().any(|(text, _)| text.contains("STRONGER")));
    //and it falls to a victory again
    world.1.clear();
    let news = citadel_step(&mut world, near, 0.016, 1);
    assert!(news.citadel && world.0[0].clock == CITADEL_RESPAWN_SECONDS, "a victory every time");
}

#[test]
fn a_citadel_that_was_already_cleared_in_an_older_save_arms_again_at_once() {
    let mut world = citadel_scene(CITADEL_MIN_ARMY);
    world.0[0].state = SiteState::Cleared;
    world.0[0].clock = 0.0;
    let away = world.0[0].position + Vec2::new(0.0, TILE_SIZE * 80.0);
    let news = citadel_step(&mut world, away, 0.016, 0);
    assert!(news.citadel_rearmed && !news.citadel && world.0[0].state == SiteState::Dormant);
}
