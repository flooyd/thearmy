//the world with several players in it: each enemy goes after the player nearest to it, what the world makes is made for the fight of the
//players it is made for, and what is won goes to the player who won it

use super::*;

//two places in the middle of the world, far enough apart for each to have a world of its own
const WEST: Vec2 = Vec2::new(10000.0, 16000.0);
const EAST: Vec2 = Vec2::new(22000.0, 16000.0);

//a compact crowd of a player's units, with their corner at a place
fn crowd_for(owner: usize, count: usize, at: Vec2) -> Vec<Unit> {
    let columns = ((count as f32).sqrt().ceil() as usize).max(1);
    (0..count).map(|i| unit_at(at.x + (i % columns) as f32 * 30.0, at.y + (i / columns) as f32 * 30.0).owned_by(owner)).collect()
}

//two players, each where they are with an army of a size
struct Duo {
    players: Players,
    units: Vec<Unit>,
}

impl Duo {
    fn new(ada: (Vec2, usize), bo: (Vec2, usize)) -> Duo {
        let walls = Walls::default();
        let mut players = Players::solo(Player::new(0, "Ada", ada.0, &walls));
        players.join("Bo", bo.0, &walls).expect("room for a second player");
        let mut units = crowd_for(0, ada.1, ada.0);
        units.extend(crowd_for(1, bo.1, bo.0));
        Duo { players, units }
    }

    //the players as the enemies see them, with what is made for them made for their team
    fn prey(&self) -> Vec<Prey<'_>> {
        prey_of(&self.players, &self.units, &army_ranges(&self.units, 2), |team| team)
    }
}

//an enemy that has noticed the player from anywhere, at a steady pace
fn chaser(at: Vec2) -> Enemy {
    let mut enemy = kind_at("Grunt", at);
    enemy.aggro = 5000.0;
    enemy.speed = 100.0;
    enemy
}

fn quiet() -> Walls {
    Walls::default()
}

// ---------- who is near whom ----------

#[test]
fn a_fight_is_the_army_of_a_player_and_those_of_the_players_near_them() {
    let at = |tiles: f32| Vec2::new(5000.0 + tiles * TILE_SIZE, 5000.0);
    let (near, far) = (ALLY_TILES - 1.0, ALLY_TILES + 1.0);
    assert_eq!(team_armies(&[at(0.0)], &[100]), [100]);
    assert_eq!(team_armies(&[at(0.0), at(near)], &[100, 50]), [150, 150]);
    assert_eq!(team_armies(&[at(0.0), at(far)], &[100, 50]), [100, 50]);
    //a fight does not reach through a friend: the ones at the ends of a line of three have only the one beside them
    assert_eq!(team_armies(&[at(0.0), at(near), at(2.0 * near)], &[1, 10, 100]), [11, 111, 110]);
    //all four together are one fight
    assert_eq!(team_armies(&[at(0.0), at(1.0), at(2.0), at(3.0)], &[1, 2, 4, 8]), [15; 4]);
}

#[test]
fn the_nearest_player_to_a_place_is_the_one_whose_avatar_is_closest() {
    let duo = Duo::new((WEST, 10), (EAST, 10));
    let prey = duo.prey();
    assert_eq!(nearest_to(&prey, WEST + Vec2::new(0.0, 3000.0)), 0);
    assert_eq!(nearest_to(&prey, EAST - Vec2::new(500.0, 0.0)), 1);
    assert_eq!(nearest_to(&prey, Vec2::ZERO), 0);
    assert_eq!(nearest(&prey, EAST).position, EAST);
    assert_eq!(nearest_to(&prey[..1], EAST), 0, "with one player it is always them");
}

#[test]
fn each_player_is_seen_by_the_enemies_with_their_own_army_route_speed_and_limit() {
    let mut duo = Duo::new((WEST, 10), (EAST, 3000));
    duo.players.list[1].upgrades.army_limit = 777;
    let prey = duo.prey();
    assert_eq!((prey[0].army(), prey[1].army()), (10, 3000));
    assert_eq!((prey[0].position, prey[1].position), (WEST, EAST));
    assert_eq!((prey[0].units.len(), prey[1].units.len()), (10, 3000));
    assert_eq!(prey[0].boost, speed_multiplier(10));
    assert_eq!(prey[1].boost, speed_multiplier(3000));
    assert!(prey[1].boost > prey[0].boost * 1.2);
    assert!(std::ptr::eq(prey[0].field, &duo.players.list[0].foes.field));
    assert!(std::ptr::eq(prey[1].field, &duo.players.list[1].foes.field));
    assert_eq!(prey[0].field.target, tile_of(WEST.x, WEST.y));
    assert_eq!(prey[1].field.target, tile_of(EAST.x, EAST.y));
    assert_eq!(prey[1].cap, duo.players.list[1].upgrades.unit_cap());
    assert_eq!(prey[0].cap, duo.players.list[0].upgrades.unit_cap());
    assert_ne!(prey[0].cap, prey[1].cap, "the limit is each player's own");
}

#[test]
fn what_is_made_for_a_player_is_made_for_their_team_and_not_for_players_far_away() {
    let apart = Duo::new((WEST, 100), (EAST, 400));
    let prey = prey_of(&apart.players, &apart.units, &army_ranges(&apart.units, 2), |team| team * 2 + 1);
    assert_eq!(pressures(&prey), vec![(WEST, 201), (EAST, 801)]);
    let beside = WEST + Vec2::new(10.0 * TILE_SIZE, 0.0);
    let together = Duo::new((WEST, 100), (beside, 400));
    let prey = prey_of(&together.players, &together.units, &army_ranges(&together.units, 2), |team| team * 2 + 1);
    assert_eq!(pressures(&prey), vec![(WEST, 1001), (beside, 1001)]);
    assert_eq!((prey[0].team, prey[1].team), (500, 500));
    assert_eq!((prey[0].army(), prey[1].army()), (100, 400), "the armies are still their own");
}

// ---------- what the enemies do ----------

#[test]
fn every_enemy_goes_after_the_player_nearest_to_it() {
    let duo = Duo::new((WEST, 10), (WEST + Vec2::new(4000.0, 0.0), 10));
    let mut enemies = vec![chaser(WEST + Vec2::new(1000.0, 0.0)), chaser(WEST + Vec2::new(3000.0, 0.0))];
    let started: Vec<Vec2> = enemies.iter().map(|enemy| enemy.position).collect();
    let mut fx = Effects::default();
    for _ in 0..60 {
        patrol_enemies_among(&mut enemies, &duo.prey(), 1.0 / 60.0, &quiet(), &mut fx);
    }
    assert!(enemies.iter().all(|enemy| enemy.chasing));
    assert!(enemies[0].position.x < started[0].x - 10.0, "the first went {} -> {}", started[0], enemies[0].position);
    assert!(enemies[1].position.x > started[1].x + 10.0, "the second went {} -> {}", started[1], enemies[1].position);
    //each went along the line to its own player
    assert!(enemies.iter().all(|enemy| (enemy.position.y - WEST.y).abs() < 100.0), "{:?}", enemies.iter().map(|enemy| enemy.position).collect::<Vec<_>>());
}

#[test]
fn an_enemy_changes_its_mind_when_another_player_is_nearer() {
    let mut duo = Duo::new((WEST, 10), (WEST + Vec2::new(4000.0, 0.0), 10));
    let mut enemy = chaser(WEST + Vec2::new(1000.0, 0.0));
    let mut fx = Effects::default();
    let mut x = enemy.position.x;
    let mut walk = |duo: &Duo, enemy: &mut Enemy| {
        for _ in 0..30 {
            patrol_enemies_among(std::slice::from_mut(enemy), &duo.prey(), 1.0 / 60.0, &quiet(), &mut fx);
        }
        let went = enemy.position.x - x;
        x = enemy.position.x;
        went
    };
    assert!(walk(&duo, &mut enemy) < -5.0, "towards Ada");
    //Bo walks over to the other side of it, and it is Bo it goes after
    duo.players.list[1].position = enemy.position + Vec2::new(300.0, 0.0);
    duo.players.list[1].foes = FoeRoute::new(duo.players.list[1].position, &quiet());
    assert!(walk(&duo, &mut enemy) > 5.0, "towards Bo");
}

#[test]
fn the_enemies_near_a_bigger_army_are_faster() {
    let duo = Duo::new((WEST, 5), (EAST, 3000));
    let prey = duo.prey();
    assert!(prey[1].boost > prey[0].boost * 1.2);
    drop(prey);
    let mut enemies = vec![chaser(WEST + Vec2::new(900.0, 0.0)), chaser(EAST + Vec2::new(900.0, 0.0))];
    let started: Vec<Vec2> = enemies.iter().map(|enemy| enemy.position).collect();
    let mut fx = Effects::default();
    for _ in 0..60 {
        patrol_enemies_among(&mut enemies, &duo.prey(), 1.0 / 60.0, &quiet(), &mut fx);
    }
    let moved = |i: usize| started[i].distance(enemies[i].position);
    println!("moved in a second: {:.0} near the small army, {:.0} near the big one", moved(0), moved(1));
    assert!(moved(0) > 20.0 && moved(1) > moved(0) * 1.2);
}

#[test]
fn an_ambusher_springs_when_the_player_nearest_to_it_comes_close_whichever_player_that_is() {
    let duo = Duo::new((WEST, 10), (EAST, 10));
    let close = AMBUSH_TRIGGER_TILES * TILE_SIZE / 2.0;
    let mut enemies = vec![
        kind_at("Ambusher", WEST + Vec2::new(close, 0.0)),
        kind_at("Ambusher", EAST + Vec2::new(0.0, -close)),
        //(beside the first, too far from Ada to spring for her, but a pack springs together)
        kind_at("Ambusher", WEST + Vec2::new(close + 3.0 * TILE_SIZE, 0.0)),
        //(a long way from them both)
        kind_at("Ambusher", Vec2::new(16000.0, 4000.0)),
    ];
    assert!(enemies.iter().all(|enemy| enemy.lurking()));
    let sprang = spring_ambushes_among(&mut enemies, &duo.prey(), &mut Effects::default());
    assert_eq!(sprang, 3);
    assert_eq!(enemies.iter().map(|enemy| enemy.lurking()).collect::<Vec<_>>(), [false, false, false, true]);
    //each rushes at its own player
    let Mode::Dash { dir, .. } = enemies[0].mode else { panic!("{:?}", enemies[0].mode) };
    assert!(dir.x < -0.9, "{dir}");
    let Mode::Dash { dir, .. } = enemies[1].mode else { panic!("{:?}", enemies[1].mode) };
    assert!(dir.y > 0.9, "{dir}");
    let Mode::Dash { dir, .. } = enemies[2].mode else { panic!("{:?}", enemies[2].mode) };
    assert!(dir.x < -0.9, "{dir}");
}

fn nest_at(at: Vec2) -> Enemy {
    let mut nest = nest_core(&NESTS[0], &Site::new(SiteKind::Nest(0), at), 10);
    nest.timer = 0.0;
    nest
}

#[test]
fn a_nest_sends_out_a_brood_for_the_army_of_the_player_nearest_to_it_and_none_when_no_one_is_near() {
    let duo = Duo::new((WEST, 10), (EAST, 3000));
    let (by_ada, by_bo, alone) = (nest_at(WEST + Vec2::new(20.0 * TILE_SIZE, 0.0)), nest_at(EAST + Vec2::new(20.0 * TILE_SIZE, 0.0)), nest_at(Vec2::new(16000.0, 3000.0)));
    let ids = (by_ada.id, by_bo.id, alone.id);
    let mut enemies = vec![by_ada, by_bo, alone];
    enemy_abilities_among(&mut enemies, &duo.prey(), 0.01, &mut Effects::default());
    let brood = |id: u32| enemies.iter().filter(|enemy| enemy.owner == Some(id)).count();
    let def = &NESTS[0];
    assert_eq!(brood(ids.0), def.brood + 10 / 100);
    assert_eq!(brood(ids.1), (def.brood + 3000 / 100).min((def.cap + 3000 / 40).min(NEST_MAX_BROOD)));
    assert!(brood(ids.1) > brood(ids.0) * 3);
    assert_eq!(brood(ids.2), 0, "no one is near enough to be called for");
}

#[test]
fn a_mortar_lobs_its_shell_at_the_army_of_the_player_nearest_to_it_for_the_size_of_that_army() {
    let duo = Duo::new((WEST, 10), (EAST, 3000));
    let mut small = kind_at("Mortar", WEST + Vec2::new(-100.0, 0.0));
    let mut big = kind_at("Mortar", EAST + Vec2::new(-100.0, 800.0));
    (small.timer, big.timer) = (0.0, 0.0);
    let mut enemies = vec![small, big];
    let mut fx = Effects::default();
    enemy_abilities_among(&mut enemies, &duo.prey(), 0.01, &mut fx);
    assert_eq!(fx.hazards.len(), 2);
    let at_ada = fx.hazards.iter().find(|hazard| hazard.position.distance(WEST) < 1000.0).expect("a shell for Ada's army");
    let at_bo = fx.hazards.iter().find(|hazard| hazard.position.distance(EAST) < 3000.0).expect("a shell for Bo's army");
    println!("victims: {} of 10, {} of 3000", at_ada.victims, at_bo.victims);
    assert!(at_bo.victims > at_ada.victims * 3);
}

#[test]
fn a_boss_turns_to_face_the_player_nearest_to_it() {
    let duo = Duo::new((WEST, 10), (WEST + Vec2::new(6000.0, 0.0), 10));
    let faces = |at: Vec2| {
        let mut boss = Enemy::boss(&BOSSES[0], 300, 0, at);
        boss.boss.as_mut().unwrap().facing = Vec2::Y;
        let mut enemies = vec![boss];
        for _ in 0..10 {
            boss_act_among(&mut enemies, &duo.prey(), 0.01, &mut Vec::new(), &mut Effects::default());
        }
        enemies[0].boss.as_ref().unwrap().facing
    };
    assert!(faces(WEST + Vec2::new(1200.0, 0.0)).x < -0.05, "it turns to Ada, to the west");
    assert!(faces(WEST + Vec2::new(4800.0, 0.0)).x > 0.05, "it turns to Bo, to the east");
}

#[test]
fn the_pulse_of_a_boss_changing_phase_is_for_the_army_it_is_beside() {
    let duo = Duo::new((WEST, 10), (EAST, 3000));
    let victims = |at: Vec2| {
        let mut boss = Enemy::boss(&BOSSES[0], 300, 0, at);
        boss.boss.as_mut().unwrap().generation = SPLIT_AT.len() as u32;
        boss.health = boss.max_health * (BOSSES[0].phases[1].below - 0.01);
        let mut enemies = vec![boss];
        let mut fx = Effects::default();
        boss_act_among(&mut enemies, &duo.prey(), 0.01, &mut Vec::new(), &mut fx);
        assert_eq!(enemies[0].boss.as_ref().unwrap().phase, 1);
        fx.hazards.iter().map(|hazard| hazard.victims).max().expect("a pulse is coming")
    };
    let (by_ada, by_bo) = (victims(WEST + Vec2::new(1200.0, 0.0)), victims(EAST + Vec2::new(1200.0, 0.0)));
    println!("victims of the pulse: {by_ada} of 10, {by_bo} of 3000");
    assert!(by_bo > by_ada);
}

// ---------- the wild ----------

fn wild_by_each_player(enemies: &[Enemy]) -> (Vec<&Enemy>, Vec<&Enemy>) {
    enemies.iter().filter(|enemy| is_wild(enemy)).partition(|enemy| enemy.position.x < (WEST.x + EAST.x) / 2.0)
}

#[test]
fn each_player_has_the_wild_enemies_round_them_that_one_alone_would() {
    let mut enemies = Vec::new();
    respawn_enemies_among(&mut enemies, &[(WEST, 50), (EAST, 50)]);
    let leash = ENEMY_LEASH_TILES * TILE_SIZE;
    let round = |at: Vec2| enemies.iter().filter(|enemy| is_wild(enemy) && enemy.position.distance(at) <= leash).count();
    println!("{} wild enemies round Ada and {} round Bo, of {}", round(WEST), round(EAST), enemies.len());
    assert!(round(WEST) >= ENEMY_COUNT && round(EAST) >= ENEMY_COUNT);
    assert!(enemies.len() > ENEMY_COUNT + 50, "{} is hardly more than one player has", enemies.len());
    //(the ones that lie in wait are a share of all of them, however many players there are)
    let lurking = enemies.iter().filter(|enemy| enemy.lurking()).count();
    assert!(lurking <= ENEMY_COUNT / MAX_LURKERS_SHARE * 2 + 8, "{lurking} lying in wait");
}

#[test]
fn players_beside_each_other_have_the_same_wild_enemies_and_not_twice_as_many() {
    let mut alone = Vec::new();
    respawn_enemies_among(&mut alone, &[(WEST, 50)]);
    let mut together = Vec::new();
    respawn_enemies_among(&mut together, &[(WEST, 50), (WEST + Vec2::new(1500.0, 0.0), 50)]);
    println!("{} for one player, {} for two together", alone.len(), together.len());
    assert!(together.len() <= ENEMY_COUNT + 30, "{} is too many", together.len());
    let leash = ENEMY_LEASH_TILES * TILE_SIZE;
    for at in [WEST, WEST + Vec2::new(1500.0, 0.0)] {
        assert!(together.iter().filter(|enemy| enemy.position.distance(at) <= leash).count() >= ENEMY_COUNT);
    }
}

#[test]
fn the_wild_enemies_are_made_for_the_army_of_the_player_they_turn_up_round() {
    let mut enemies = Vec::new();
    respawn_enemies_among(&mut enemies, &[(WEST, 5), (EAST, 5000)]);
    let (west, east) = wild_by_each_player(&enemies);
    let mean = |list: &[&Enemy]| list.iter().map(|enemy| enemy.max_health).sum::<f32>() / list.len() as f32;
    println!("mean health: {:.0} for an army of 5, {:.0} for one of 5000", mean(&west), mean(&east));
    assert!(mean(&east) > mean(&west) * 1.5);
}

#[test]
fn a_wild_enemy_is_let_go_only_when_it_is_far_behind_every_player() {
    let mut enemies = vec![kind_at("Grunt", WEST + Vec2::new(100.0, 0.0)), kind_at("Grunt", EAST - Vec2::new(100.0, 0.0)), kind_at("Grunt", Vec2::new(16000.0, 2000.0))];
    let ids: Vec<u32> = enemies.iter().map(|enemy| enemy.id).collect();
    respawn_enemies_among(&mut enemies, &[(WEST, 50), (EAST, 50)]);
    assert!(enemies.iter().any(|enemy| enemy.id == ids[0]), "the one by Ada was kept");
    assert!(enemies.iter().any(|enemy| enemy.id == ids[1]), "the one by Bo was kept");
    assert!(!enemies.iter().any(|enemy| enemy.id == ids[2]), "the one a long way from both was let go");
    //only the wild ones are let go: a boss, a horde and the garrison of a site stay where they are
    let mut kept = vec![Enemy::boss(&BOSSES[0], 100, 0, Vec2::new(16000.0, 2000.0)), kind_at("Grunt", Vec2::new(16000.0, 2100.0)), kind_at("Grunt", Vec2::new(16000.0, 2200.0))];
    join_horde(&mut kept[1]);
    kept[2].site = Some(7);
    respawn_enemies_among(&mut kept, &[(WEST, 50), (EAST, 50)]);
    assert!(kept.iter().any(|enemy| enemy.boss.is_some()));
    assert!(kept.iter().any(|enemy| in_a_horde(enemy)));
    assert!(kept.iter().any(|enemy| enemy.site == Some(7)));
}

// ---------- hordes ----------

fn due() -> Hordes {
    let mut hordes = Hordes::new();
    hordes.next_in = 0.0;
    hordes
}

#[test]
fn a_horde_is_sent_at_one_of_the_players_whose_fight_is_big_enough_for_it() {
    let duo = Duo::new((WEST, HORDE_MIN_ARMY - 1), (EAST, 200));
    for _ in 0..20 {
        let mut hordes = due();
        let beat = hordes.update_among(&mut Vec::new(), &duo.prey(), false, 0.016);
        assert!(matches!(beat, Some(HordeBeat::Warned { .. })));
        let coming = hordes.coming.expect("it was announced");
        let off = (coming.at.distance(EAST) - HORDE_DISTANCE_TILES * TILE_SIZE).abs();
        assert!(off < 1.0, "it comes from {} which is not {HORDE_DISTANCE_TILES} tiles from Bo", coming.at);
    }
}

#[test]
fn every_player_with_an_army_big_enough_is_sent_hordes_in_turn() {
    let duo = Duo::new((WEST, 200), (EAST, 200));
    let reach = HORDE_DISTANCE_TILES * TILE_SIZE;
    let mut sent_at = [0, 0];
    for _ in 0..40 {
        let mut hordes = due();
        hordes.update_among(&mut Vec::new(), &duo.prey(), false, 0.016);
        let at = hordes.coming.expect("it was announced").at;
        sent_at[((at.distance(EAST) - reach).abs() < 1.0) as usize] += 1;
    }
    println!("sent at Ada {} times, at Bo {}", sent_at[0], sent_at[1]);
    assert!(sent_at[0] > 0 && sent_at[1] > 0);
}

#[test]
fn no_horde_comes_while_no_one_is_worth_it_or_a_boss_is_about_and_two_small_armies_together_are_worth_it() {
    let small = Duo::new((WEST, 10), (EAST, 10));
    let mut hordes = due();
    hordes.next_in = 30.0;
    assert!(hordes.update_among(&mut Vec::new(), &small.prey(), false, 5.0).is_none());
    assert_eq!(hordes.next_in, 30.0, "the countdown waits");
    //a boss about
    let big = Duo::new((WEST, 200), (EAST, 200));
    let mut hordes = due();
    assert!(hordes.update_among(&mut Vec::new(), &big.prey(), true, 0.016).is_none());
    assert!(hordes.coming.is_none());
    //two armies that are small, side by side, are a fight big enough for one
    let beside = Duo::new((WEST, HORDE_MIN_ARMY / 2 + 1), (WEST + Vec2::new(1000.0, 0.0), HORDE_MIN_ARMY / 2 + 1));
    let mut hordes = due();
    assert!(matches!(hordes.update_among(&mut Vec::new(), &beside.prey(), false, 0.016), Some(HordeBeat::Warned { .. })));
}

#[test]
fn a_horde_that_arrives_is_made_for_the_fight_of_the_player_nearest_to_where_it_comes_from() {
    let duo = Duo::new((WEST, 10), (EAST, 3000));
    let def = horde_named("Swarm Tide").unwrap();
    let arrive = |at: Vec2| {
        let mut hordes = Hordes::new();
        hordes.coming = Some(Coming { def, at, left: 0.0, other: None });
        match hordes.update_among(&mut Vec::new(), &duo.prey(), false, 0.016) {
            Some(HordeBeat::Arrived { count, .. }) => count,
            _ => panic!("it did not arrive"),
        }
    };
    assert_eq!(arrive(WEST + Vec2::new(0.0, 2900.0)), def.count(10));
    assert_eq!(arrive(EAST - Vec2::new(0.0, 2900.0)), def.count(3000));
    assert!(def.count(3000) > def.count(10) + 10);
    //players side by side are one fight, whichever of them it comes at
    let together = Duo::new((WEST, 10), (WEST + Vec2::new(1000.0, 0.0), 3000));
    let mut hordes = Hordes::new();
    hordes.coming = Some(Coming { def, at: WEST - Vec2::new(0.0, 2900.0), left: 0.0, other: None });
    assert!(matches!(hordes.update_among(&mut Vec::new(), &together.prey(), false, 0.016), Some(HordeBeat::Arrived { count, .. }) if count == def.count(3010)));
}

#[test]
fn the_other_side_of_a_pincer_comes_from_across_the_player_it_is_sent_at() {
    let duo = Duo::new((WEST, 300), (EAST, 300));
    let def = horde_named("Flanking Raid").unwrap();
    let mut hordes = Hordes::new();
    hordes.coming = Some(Coming { def, at: EAST + Vec2::new(2900.0, 0.0), left: 0.0, other: None });
    let mut enemies = Vec::new();
    hordes.update_among(&mut enemies, &duo.prey(), false, 0.016);
    let captains: Vec<&Enemy> = enemies.iter().filter(|enemy| enemy.is_captain()).collect();
    assert_eq!(captains.len(), 2);
    assert!(captains.iter().any(|captain| captain.position.x > EAST.x + 2000.0), "one comes from where it was announced");
    assert!(captains.iter().any(|captain| captain.position.x < EAST.x - 2000.0), "the other from across Bo");
}

#[test]
fn a_horde_is_let_go_when_it_is_far_behind_every_player_and_not_before() {
    let duo = Duo::new((WEST, 50), (EAST, 50));
    let member = |at: Vec2| {
        let mut enemy = kind_at("Grunt", at);
        join_horde(&mut enemy);
        enemy
    };
    let mut enemies = vec![member(WEST + Vec2::new(500.0, 0.0)), member(EAST - Vec2::new(500.0, 0.0)), member(Vec2::new(16000.0, 2000.0))];
    let ids: Vec<u32> = enemies.iter().map(|enemy| enemy.id).collect();
    Hordes::new().update_among(&mut enemies, &duo.prey(), false, 0.016);
    assert_eq!(enemies.iter().map(|enemy| enemy.id).collect::<Vec<_>>(), &ids[..2]);
}

// ---------- bosses ----------

#[test]
fn the_countdown_to_a_boss_waits_until_a_fight_is_big_enough_for_it() {
    let def = &BOSSES[0];
    let mut director = Director::new();
    director.next_boss_in = 50.0;
    let small = Duo::new((WEST, def.min_army - 1), (EAST, def.min_army - 1));
    assert_eq!(director.update_among(&mut Vec::new(), &small.prey(), 10.0), None);
    assert_eq!(director.next_boss_in, 50.0);
    //one big enough, and it counts down
    let big = Duo::new((WEST, 10), (EAST, def.min_army));
    director.update_among(&mut Vec::new(), &big.prey(), 10.0);
    assert_eq!(director.next_boss_in, 40.0);
}

#[test]
fn a_boss_comes_at_a_player_whose_fight_is_big_enough_and_is_made_for_that_fight() {
    let def = &BOSSES[0];
    let duo = Duo::new((WEST, 10), (EAST, def.min_army + 40));
    for _ in 0..10 {
        let mut director = Director::new();
        director.next_boss_in = 0.0;
        let mut enemies = Vec::new();
        assert_eq!(director.update_among(&mut enemies, &duo.prey(), 0.016), Some(Beat::Arrived(def.name)));
        let boss = &enemies[0];
        let apart = boss.position.distance(EAST);
        assert!(apart >= BOSS_SPAWN_MIN_TILES * TILE_SIZE - 1.0 && apart <= BOSS_SPAWN_MAX_TILES * TILE_SIZE + 1.0, "{apart} from Bo");
        assert_eq!(boss.max_health, Enemy::boss(def, def.min_army + 40, 0, EAST).max_health);
    }
}

#[test]
fn every_player_with_a_fight_big_enough_gets_bosses_in_turn_and_two_small_armies_together_get_one() {
    let def = &BOSSES[0];
    let both = Duo::new((WEST, def.min_army), (EAST, def.min_army));
    let mut at = [0, 0];
    for _ in 0..40 {
        let mut director = Director::new();
        director.next_boss_in = 0.0;
        let mut enemies = Vec::new();
        director.update_among(&mut enemies, &both.prey(), 0.016);
        at[(enemies[0].position.distance(EAST) < enemies[0].position.distance(WEST)) as usize] += 1;
    }
    println!("sent at Ada {} times, at Bo {}", at[0], at[1]);
    assert!(at[0] > 0 && at[1] > 0);
    //half an army each is not enough, side by side it is
    let half = def.min_army / 2;
    let mut director = Director::new();
    director.next_boss_in = 0.0;
    let apart = Duo::new((WEST, half), (EAST, half));
    assert_eq!(director.update_among(&mut Vec::new(), &apart.prey(), 0.016), None);
    let beside = Duo::new((WEST, half), (WEST + Vec2::new(1000.0, 0.0), half));
    assert_eq!(director.update_among(&mut Vec::new(), &beside.prey(), 0.016), Some(Beat::Arrived(def.name)));
}

// ---------- chests ----------

//players at two places, with no army but what the test puts down
fn watchers(ada: Vec2, bo: Vec2) -> Duo {
    Duo::new((ada, 0), (bo, 0))
}

#[test]
fn a_chest_is_opened_by_the_player_whose_avatar_is_nearest_of_those_who_have_reached_it() {
    let chest_at = MIDDLE;
    let inside = CHEST_REACH * 0.6;
    //Ada at it, Bo too, but Bo is nearer
    let duo = watchers(chest_at + Vec2::new(inside, 0.0), chest_at + Vec2::new(inside / 2.0, 0.0));
    let mut chests = vec![Chest::new(chest_at, 1, 10.0)];
    let opened = take_reached_among(&mut chests, &duo.prey(), &[], 0.016);
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].1, 1);
    assert!(chests.is_empty());
    //and the other way round
    let duo = watchers(chest_at + Vec2::new(inside / 2.0, 0.0), chest_at + Vec2::new(inside, 0.0));
    let mut chests = vec![Chest::new(chest_at, 1, 10.0)];
    assert_eq!(take_reached_among(&mut chests, &duo.prey(), &[], 0.016)[0].1, 0);
}

#[test]
fn a_chest_that_only_the_units_of_a_player_have_reached_is_that_players() {
    let mut duo = watchers(MIDDLE + Vec2::new(5000.0, 0.0), MIDDLE - Vec2::new(5000.0, 0.0));
    duo.units = vec![unit_at(MIDDLE.x + 20.0, MIDDLE.y).owned_by(1)];
    let mut chests = vec![Chest::new(MIDDLE, 1, 10.0)];
    let opened = take_reached_among(&mut chests, &duo.prey(), &[], 0.016);
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].1, 1);
    //and a chest no one is at is left where it is
    let mut chests = vec![Chest::new(MIDDLE + Vec2::new(0.0, 3000.0), 1, 10.0)];
    assert!(take_reached_among(&mut chests, &duo.prey(), &[], 0.016).is_empty());
    assert_eq!(chests.len(), 1);
}

#[test]
fn a_chest_that_has_to_be_held_is_held_by_whoever_is_there_and_not_while_enemies_contest_it() {
    let duo = watchers(MIDDLE + Vec2::new(10.0, 0.0), MIDDLE - Vec2::new(5000.0, 0.0));
    let mut chests = vec![Chest::new(MIDDLE, 2, 10.0)];
    assert!(chest_hold(&chests) >= 2.0);
    //an awake enemy beside it, and the hold stands still
    let enemy = dummy(MIDDLE + Vec2::new(0.0, 200.0));
    assert!(take_reached_among(&mut chests, &duo.prey(), std::slice::from_ref(&enemy), 1.0).is_empty());
    assert_eq!((chests[0].held, chests[0].contested), (0.0, true));
    //it fills when they are gone, and opens for the one who held it
    assert!(take_reached_among(&mut chests, &duo.prey(), &[], 1.0).is_empty());
    assert_eq!(chests[0].held, 1.0);
    let opened = take_reached_among(&mut chests, &duo.prey(), &[], 1.0);
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].1, 0);
}

fn chest_hold(chests: &[Chest]) -> f32 {
    chests[0].hold()
}

#[test]
fn what_is_in_a_chest_goes_to_the_player_who_opened_it() {
    let chest = Chest::new(MIDDLE, 3, 10.0);
    let loot = chest.loot(100);
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    let relic = open_chest(1, &chest, 100, MULTIPLAYER_ARMY_LIMIT, &mut babies, &mut fx);
    assert_eq!(relic, loot.relic);
    assert_eq!((fx.spoils[0].points, fx.spoils[1].points), (0.0, loot.points));
    assert_eq!(babies.len(), loot.recruits);
    assert_eq!(fx.recruits.len(), loot.champion as usize);
    assert!(fx.recruits.iter().all(|(at, odds, owner)| *at == MIDDLE && *odds == Odds::Horde(3) && *owner == 1));
    //the recruits only fill the room their army has
    let mut crowded = Vec::new();
    open_chest(0, &chest, 100, 103, &mut crowded, &mut Effects::default());
    assert!(crowded.len() <= 3, "{} recruits for 3 places", crowded.len());
}

// ---------- sites ----------

//a site, and what a frame of the sites needs round it
struct Lair {
    sites: Vec<Site>,
    enemies: Vec<Enemy>,
    walls: Walls,
    babies: Vec<Baby>,
    fx: Effects,
}

impl Lair {
    fn new(kind: SiteKind) -> Lair {
        Lair { sites: vec![Site::new(kind, MIDDLE)], enemies: Vec::new(), walls: Walls::default(), babies: Vec::new(), fx: Effects::default() }
    }

    fn step(&mut self, duo: &Duo) -> SiteNews {
        update_sites_among(&mut self.sites, &mut self.enemies, &mut self.walls, &mut self.babies, &duo.prey(), 0.016, false, 0, &mut self.fx)
    }

    fn state(&self) -> SiteState {
        self.sites[0].state
    }
}

//a place this many grid squares south of the site (north, if the number is below nought)
fn from_site(tiles: f32) -> Vec2 {
    MIDDLE + Vec2::new(0.0, tiles * TILE_SIZE)
}

#[test]
fn a_site_wakes_for_the_fight_of_the_player_nearest_to_it_and_not_for_one_far_off() {
    let (near, far) = (from_site(20.0), from_site(-(SLEEP_TILES + 10.0)));
    //Ada is there with an army too small for it and Bo has a big one far away: it is Ada's fight, and it stays asleep
    let mut lair = Lair::new(SiteKind::Nest(1)); //the warren wants ten
    lair.step(&Duo::new((near, 5), (far, 500)));
    assert_eq!(lair.state(), SiteState::Dormant);
    //neither is near
    lair.step(&Duo::new((far, 500), (far + Vec2::new(500.0, 0.0), 500)));
    assert_eq!(lair.state(), SiteState::Dormant);
    //Bo comes up beside Ada: together they are enough
    lair.step(&Duo::new((near, 5), (near + Vec2::new(500.0, 0.0), 5)));
    assert_eq!(lair.state(), SiteState::Awake);
    assert!(lair.enemies.iter().any(|enemy| enemy.behavior == Behavior::Nest));
}

#[test]
fn a_site_that_is_awake_sleeps_again_only_when_every_player_has_gone() {
    let (near, far) = (from_site(20.0), from_site(-(SLEEP_TILES + 10.0)));
    let mut lair = Lair::new(SiteKind::Nest(0));
    lair.step(&Duo::new((near, 20), (far, 20)));
    assert_eq!(lair.state(), SiteState::Awake);
    //Ada goes, Bo is still there
    lair.step(&Duo::new((far, 20), (near, 20)));
    assert_eq!(lair.state(), SiteState::Awake);
    assert!(!lair.enemies.is_empty());
    //and now they both have
    lair.step(&Duo::new((far, 20), (far + Vec2::new(0.0, -500.0), 20)));
    assert_eq!(lair.state(), SiteState::Dormant);
    assert!(lair.enemies.is_empty());
}

#[test]
fn the_garrison_of_a_village_is_as_big_as_the_fight_that_wakes_it() {
    let (near, far) = (from_site(20.0), from_site(-(SLEEP_TILES + 10.0)));
    let garrison = |duo: Duo| {
        let mut lair = Lair::new(SiteKind::Village);
        lair.step(&duo);
        assert_eq!(lair.state(), SiteState::Awake);
        lair.enemies.len()
    };
    let alone = garrison(Duo::new((near, 100), (far, 500)));
    let together = garrison(Duo::new((near, 100), (near + Vec2::new(500.0, 0.0), 500)));
    println!("{alone} guards for one army of 100, {together} for 100 beside 500");
    assert!(together > alone);
}

#[test]
fn what_a_cleared_site_gives_goes_to_the_player_nearest_to_it() {
    for (kind, points) in [(SiteKind::Village, VILLAGE_REWARD_POINTS), (SiteKind::Nest(0), NEST_REWARD_POINTS)] {
        for (ada_at, bo_at, earner) in [(20.0, 30.0, 0), (30.0, 20.0, 1)] {
            let mut lair = Lair::new(kind);
            let duo = Duo::new((from_site(ada_at), 20), (from_site(bo_at) + Vec2::new(300.0, 0.0), 20));
            lair.step(&duo);
            assert_eq!(lair.state(), SiteState::Awake);
            //everything that held it is beaten
            lair.enemies.clear();
            lair.step(&duo);
            assert_eq!(lair.state(), SiteState::Cleared);
            assert_eq!(lair.fx.spoils[earner].points, points, "{kind:?}: the player at {ada_at} / {bo_at} tiles");
            assert_eq!(lair.fx.spoils[1 - earner].points, 0.0);
            assert!(!lair.babies.is_empty(), "recruits");
        }
    }
}

#[test]
fn a_freed_village_mends_the_units_of_every_army_round_it_while_a_player_is_near() {
    let mut village = Site::new(SiteKind::Village, MIDDLE);
    village.state = SiteState::Cleared;
    let sites = [village];
    let hurt = |at: Vec2, owner: usize| {
        let mut unit = unit_at(at.x, at.y).owned_by(owner);
        unit.health = 1.0;
        unit
    };
    let mut units = vec![hurt(MIDDLE + Vec2::new(100.0, 0.0), 0), hurt(MIDDLE + Vec2::new(0.0, 150.0), 1), hurt(from_site(20.0), 0)];
    //nobody is near enough to see it
    mend_at_villages(&sites, &mut units, &[from_site(-(AWAKEN_TILES + 10.0))], 1.0);
    assert!(units.iter().all(|unit| unit.health == 1.0));
    //one of two is
    mend_at_villages(&sites, &mut units, &[from_site(-(AWAKEN_TILES + 10.0)), from_site(10.0)], 1.0);
    assert!(units[0].health > 1.0 && units[1].health > 1.0, "both armies were mended: {} and {}", units[0].health, units[1].health);
    assert_eq!(units[2].health, 1.0, "the one out of its reach was not");
}

// ---------- what is won ----------

#[test]
fn the_world_pays_the_player_it_names_and_nobody_past_the_last() {
    let mut fx = Effects::default();
    fx.earn(0, 5.0);
    fx.earn(1, 2.0);
    fx.earn(1, 3.0);
    fx.earn(9, 1.0);
    assert_eq!((fx.spoils[0].points, fx.spoils[1].points, fx.spoils[2].points), (5.0, 5.0, 0.0));
    assert_eq!(fx.spoils[MAX_PLAYERS - 1].points, 1.0, "a player past the end is paid as the last");
}

#[test]
fn an_elite_that_leaves_a_champion_leaves_it_for_the_player_with_the_credit() {
    let (players, _) = pair();
    let elites = |credit: u8| {
        let mut enemies: Vec<Enemy> = (0..2000)
            .map(|i| {
                let mut enemy = kind_at("Grunt", MIDDLE + Vec2::new(i as f32, 0.0));
                enemy.make_elite(Modifier::Swift);
                enemy.health = 0.0;
                enemy.credit = credit;
                enemy
            })
            .collect();
        let mut fx = Effects::default();
        resolve_deaths_among(&mut enemies, &mut Vec::new(), &[50, 50], &players, &mut fx);
        fx
    };
    for credit in [0u8, 1] {
        let fx = elites(credit);
        assert!(!fx.recruits.is_empty());
        assert!(fx.recruits.iter().all(|(_, odds, owner)| *odds == Odds::Elite && *owner == credit as usize));
        assert!(fx.spoils[credit as usize].points > 0.0 && fx.spoils[1 - credit as usize].points == 0.0);
    }
}

#[test]
fn each_champion_won_joins_the_army_of_the_player_who_won_it() {
    let (players, _) = pair();
    let mut units = army_for(0, 5, ADA);
    units.extend(army_for(1, 5, BO));
    let mut fx = Effects::default();
    fx.recruits.push((ADA, Odds::Citadel, 0));
    fx.recruits.push((BO, Odds::Citadel, 1));
    fx.recruits.push((BO, Odds::Citadel, 1));
    award_recruits(&mut units, &mut Vec::new(), &players, &mut fx);
    assert!(fx.recruits.is_empty());
    //(a champion of a kind the army already has is a level for it, so what is counted is the levels)
    let won = |owner: usize| own(&units, owner).iter().filter(|unit| unit.champion != 0).map(|unit| unit.level as usize).sum::<usize>();
    assert_eq!((won(0), won(1)), (1, 2));
    for unit in units.iter().filter(|unit| unit.champion != 0) {
        let home = if unit.owner == 0 { ADA } else { BO };
        assert!(Vec2::new(unit.x, unit.y).distance(home) < 600.0, "a champion of player {} turned up at {}, {}", unit.owner, unit.x, unit.y);
    }
}

#[test]
fn a_relic_choice_won_while_choosing_waits_its_turn_and_only_for_the_player_who_won_it() {
    let (mut players, _) = pair();
    let player = &mut players.list[0];
    for tier in [1, 2, 3] {
        player.offer(RelicChoice::from_chest(tier));
    }
    assert_eq!((player.choice.as_ref().map(|choice| choice.chest), player.owed.len()), (Some(1), 2));
    player.chosen();
    assert_eq!((player.choice.as_ref().map(|choice| choice.chest), player.owed.len()), (Some(2), 1));
    player.chosen();
    assert_eq!((player.choice.as_ref().map(|choice| choice.chest), player.owed.len()), (Some(3), 0));
    player.chosen();
    assert!(player.choice.is_none());
    player.chosen();
    assert!(player.choice.is_none(), "nothing to come up");
    assert!(players.list[1].choice.is_none() && players.list[1].owed.is_empty());
}

#[test]
fn a_boss_slain_is_a_win_for_every_player() {
    let (mut players, _) = pair();
    let mut fx = Effects::default();
    let rerolls = (players.list[0].rerolls, players.list[1].rerolls);
    //the first always leaves a champion, and gives no reroll
    players.share_boss(1, 2, &mut fx);
    assert!(players.list.iter().all(|player| player.choice.is_some() && player.owed.is_empty()));
    assert_eq!((players.list[0].rerolls, players.list[1].rerolls), rerolls);
    for (i, player) in players.list.iter().enumerate() {
        assert!(fx.recruits.iter().any(|(at, odds, owner)| *owner == i && *at == player.position && *odds == Odds::Boss(2)), "no champion for {}", player.name);
    }
    assert_eq!(fx.recruits.len(), 2);
    //every third gives a reroll, and a choice that comes while one is up is kept for later
    players.share_boss(3, 0, &mut fx);
    assert_eq!((players.list[0].rerolls, players.list[1].rerolls), (rerolls.0 + 1, rerolls.1 + 1));
    assert!(players.list.iter().all(|player| player.owed.len() == 1));
}

#[test]
fn the_citadel_falling_sends_every_player_a_champion_and_a_relic_only_where_there_is_no_victory_screen() {
    let (mut players, _) = pair();
    let mut fx = Effects::default();
    players.share_citadel(&mut fx);
    assert_eq!(fx.recruits.len(), 2);
    for (i, player) in players.list.iter().enumerate() {
        assert!(fx.recruits.iter().any(|(at, odds, owner)| *owner == i && *at == player.position && *odds == Odds::Citadel));
        assert!(player.choice.as_ref().is_some_and(|choice| choice.rare_only), "{} has no rare choice", player.name);
    }
    //one player has the screen for it, and the choice comes from there
    let walls = Walls::default();
    let mut alone = Players::solo(Player::new(0, "Ada", ADA, &walls));
    let mut fx = Effects::default();
    alone.share_citadel(&mut fx);
    assert_eq!(fx.recruits.len(), 1);
    assert!(alone.list[0].choice.is_none());
}

#[test]
fn the_world_stands_still_for_a_menu_or_a_choice_only_when_there_is_one_player_to_wait_for() {
    assert!(world_stands_still(1, true));
    assert!(!world_stands_still(1, false));
    assert!(!world_stands_still(2, true));
    assert!(!world_stands_still(4, true));
    assert!(!world_stands_still(2, false));
}

// ---------- a whole world ----------

#[test]
fn two_armies_in_one_world_fight_the_enemies_that_come_to_them_and_each_is_paid_for_its_own() {
    rand::srand(11);
    let walls = Walls::default();
    let mut players = Players::solo(Player::new(0, "Ada", WEST, &walls));
    players.join("Bo", EAST, &walls).expect("room");
    players.apply_settings(DEFAULT_ARMY_LIMIT, 0.0);
    let mut units = crowd_for(0, 80, WEST);
    units.extend(crowd_for(1, 80, EAST));
    //a few enemies come at Ada and more at Bo, from all sides
    let mut enemies: Vec<Enemy> = Vec::new();
    for (player, count) in [(WEST, 4), (EAST, 12)] {
        for i in 0..count {
            let mut enemy = chaser(player + Vec2::from_angle(i as f32 * 0.9) * 1200.0);
            enemy.speed = 400.0;
            enemies.push(enemy);
        }
    }
    let started = enemies.len();
    let (mut babies, dt) = (Vec::new(), 1.0 / 60.0);
    let mut earned = [0.0f32; 2];
    for _ in 0..1800 {
        let mut fx = Effects::default();
        let armies = Armies::of(&mut units, &players);
        step_armies(&mut players, &armies, &mut units, &enemies, &walls, false, dt, &mut fx);
        {
            let prey = armies.prey(&players, &units, |team| team.max(1));
            spring_ambushes_among(&mut enemies, &prey, &mut fx);
            patrol_enemies_among(&mut enemies, &prey, dt, &walls, &mut fx);
            enemy_abilities_among(&mut enemies, &prey, dt, &mut fx);
        }
        fight(&mut enemies, &mut units, &mut babies, dt, &players, &mut fx);
        bury_units(&mut units, &mut fx);
        for (i, gained) in earned.iter_mut().enumerate() {
            *gained += fx.spoils[i].points;
        }
        grow_armies(&players, &Armies::of(&mut units, &players), &mut units, &mut babies, &[Vec2::new(1920.0, 1080.0); 4], dt, &mut fx);
        earn_wall_points(&mut players, &armies, dt, &mut fx);
    }
    let (ada, bo) = (own(&units, 0), own(&units, 1));
    println!("{} enemies of {started} left; armies of {} and {}; earned {:.0} and {:.0}", enemies.len(), ada.len(), bo.len(), earned[0], earned[1]);
    assert!(enemies.len() <= started / 4, "{} of {started} are left", enemies.len());
    assert!(ada.len() > 20 && bo.len() > 20, "the armies were not wiped out");
    assert!(earned[0] > 0.0 && earned[1] > earned[0], "Bo fought three times as many and should have earned more: {earned:?}");
    //the armies stayed with their players
    assert!(mean_distance(&ada, WEST) < 1500.0 && mean_distance(&bo, EAST) < 1500.0);
    assert!(players.list[0].mason.points > START_WALL_POINTS && players.list[1].mason.points > players.list[0].mason.points);
}

// ---------- the cost of it ----------

#[test]
#[ignore]
fn four_players_and_the_enemies_round_them_cost_the_enemy_side_of_a_frame_how_long() {
    let walls = Walls::default();
    let mut players = Players::solo(Player::new(0, "A", Vec2::new(8000.0, 8000.0), &walls));
    for (name, at) in [("B", Vec2::new(24000.0, 8000.0)), ("C", Vec2::new(8000.0, 24000.0)), ("D", Vec2::new(24000.0, 24000.0))] {
        players.join(name, at, &walls).expect("room");
    }
    players.apply_settings(DEFAULT_ARMY_LIMIT, 0.0);
    let mut units = Vec::new();
    for (owner, player) in players.list.iter().enumerate() {
        units.extend(crowd_for(owner, 3000, player.position));
    }
    let mut enemies = Vec::new();
    let ranges = arrange(&mut units, players.len());
    respawn_enemies_among(&mut enemies, &pressures(&prey_of(&players, &units, &ranges, |team| team)));
    let mut fx = Effects::default();
    let mut shots = Vec::new();
    let (frames, dt) = (120, 1.0 / 120.0);
    let started = Instant::now();
    for _ in 0..frames {
        let ranges = arrange(&mut units, players.len());
        let prey = prey_of(&players, &units, &ranges, |team| team);
        spring_ambushes_among(&mut enemies, &prey, &mut fx);
        patrol_enemies_among(&mut enemies, &prey, dt, &walls, &mut fx);
        enemy_abilities_among(&mut enemies, &prey, dt, &mut fx);
        boss_act_among(&mut enemies, &prey, dt, &mut shots, &mut fx);
        respawn_enemies_among(&mut enemies, &pressures(&prey));
        fx = Effects::default();
    }
    println!("{} enemies round four armies of 3000: {:.2} ms a frame", enemies.len(), started.elapsed().as_secs_f64() * 1000.0 / frames as f64);
}
