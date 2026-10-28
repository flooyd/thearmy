//more than one player in the one world: whose units are whose, and that each army follows, fights, heals, grows and is paid for on its
//own account, with the relics of its own player

use super::*;

pub(super) const ADA: Vec2 = Vec2::new(2000.0, 2000.0);
pub(super) const BO: Vec2 = Vec2::new(6000.0, 2000.0);

//two players a long way apart, who have joined in turn
pub(super) fn pair() -> (Players, Walls) {
    let walls = Walls::default();
    let mut players = Players::solo(Player::new(0, "Ada", ADA, &walls));
    players.join("Bo", BO, &walls).expect("room for a second player");
    (players, walls)
}

//a block of units that are a player's, with their corner at a place
pub(super) fn army_for(owner: usize, count: usize, at: Vec2) -> Vec<Unit> {
    (0..count).map(|i| unit_at(at.x + (i % 10) as f32 * 40.0, at.y + (i / 10) as f32 * 40.0).owned_by(owner)).collect()
}

//an enemy that stays where it is, that no one army can destroy in a test, and that does no harm
pub(super) fn dummy(at: Vec2) -> Enemy {
    let mut enemy = kind_at("Grunt", at);
    enemy.max_health = 1.0e9;
    enemy.health = 1.0e9;
    enemy.attack = 0.0;
    enemy.speed = 0.0;
    enemy
}

//some units of a player right on an enemy, all of them touching it
pub(super) fn on(enemy: &Enemy, owner: usize, count: usize) -> Vec<Unit> {
    (0..count).map(|i| unit_at(enemy.position.x + (i % 5) as f32, enemy.position.y + (i / 5) as f32).owned_by(owner)).collect()
}

pub(super) fn mean_distance(units: &[Unit], to: Vec2) -> f32 {
    units.iter().map(|unit| Vec2::new(unit.x, unit.y).distance(to)).sum::<f32>() / units.len() as f32
}

pub(super) fn own(units: &[Unit], owner: usize) -> Vec<Unit> {
    units.iter().filter(|unit| unit.owner as usize == owner).cloned().collect()
}

// ---------- whose units are whose ----------

#[test]
fn a_unit_belongs_to_the_first_player_until_it_is_marked_for_another() {
    assert_eq!(unit_at(0.0, 0.0).owner, 0);
    assert_eq!(new_unit(Vec2::ZERO).owner, 0);
    assert_eq!(champion_unit(0, Vec2::ZERO).owner, 0);
    assert_eq!(new_unit(Vec2::ZERO).owned_by(2).owner, 2);
}

#[test]
fn armies_are_counted_by_owner_and_a_unit_marked_past_the_last_player_counts_as_the_last() {
    let units: Vec<Unit> = [0, 0, 1, 3, 3, 3, 9].iter().map(|&owner| unit_at(0.0, 0.0).owned_by(owner)).collect();
    assert_eq!(army_sizes(&units), [2, 1, 0, 4]);
    assert_eq!(owner_index(0), 0);
    assert_eq!(owner_index(3), 3);
    assert_eq!(owner_index(200), MAX_PLAYERS - 1);
    assert_eq!(army_sizes(&[]), [0; MAX_PLAYERS]);
}

#[test]
fn sorting_by_owner_keeps_every_army_in_its_order_and_leaves_a_sorted_list_alone() {
    let mut units: Vec<Unit> = (0..12).map(|i| unit_at(i as f32, 0.0).owned_by([2, 0, 1][i % 3])).collect();
    sort_by_owner(&mut units);
    assert!(units.windows(2).all(|pair| pair[0].owner <= pair[1].owner));
    for owner in 0..3 {
        let xs: Vec<f32> = units.iter().filter(|unit| unit.owner == owner).map(|unit| unit.x).collect();
        assert_eq!(xs.len(), 4);
        assert!(xs.windows(2).all(|pair| pair[0] < pair[1]), "owner {owner}: {xs:?}");
    }
    let sorted = positions(&units);
    sort_by_owner(&mut units);
    assert_eq!(positions(&units), sorted);
}

#[test]
fn each_army_is_one_stretch_of_the_list_and_the_stretches_can_be_worked_on_apart() {
    let mut units: Vec<Unit> = [0, 0, 0, 2, 2, 3].iter().map(|&owner| unit_at(0.0, 0.0).owned_by(owner)).collect();
    //a list with three players in it: the unit marked 3 belongs to the last
    let ranges = army_ranges(&units, 3);
    assert_eq!(ranges, vec![0..3, 3..3, 3..6]);
    assert_eq!(army_ranges(&units, 1), vec![0..6]);
    assert_eq!(army_ranges(&[], 2), vec![0..0, 0..0]);
    let mut stretches = split_armies(&mut units, &ranges);
    assert_eq!(stretches.iter().map(|army| army.len()).collect::<Vec<_>>(), [3, 0, 3]);
    stretches[2][0].x = 77.0;
    drop(stretches);
    assert_eq!(units[3].x, 77.0);
    assert_eq!(units.iter().filter(|unit| unit.x == 77.0).count(), 1);
}

#[test]
fn only_a_world_with_several_players_puts_the_units_in_order_of_owner() {
    let owners = |units: &[Unit]| units.iter().map(|unit| unit.owner).collect::<Vec<_>>();
    let mut units: Vec<Unit> = [1, 0, 1, 0].iter().map(|&owner| unit_at(0.0, 0.0).owned_by(owner)).collect();
    //one player: the whole list is the army, however it is marked, and nothing moves
    assert_eq!(arrange(&mut units, 1), vec![0..4]);
    assert_eq!(owners(&units), [1, 0, 1, 0]);
    //two: the units are put in order
    assert_eq!(arrange(&mut units, 2), vec![0..2, 2..4]);
    assert_eq!(owners(&units), [0, 0, 1, 1]);
    //and nobody at all is one empty army
    assert_eq!(arrange(&mut [], 0), vec![0..0]);
}

// ---------- the players ----------

#[test]
fn up_to_four_players_can_join_and_each_has_a_colour_of_their_own() {
    let walls = Walls::default();
    let mut players = Players::solo(Player::new(0, "Ada", ADA, &walls));
    assert_eq!(players.len(), 1);
    assert_eq!(players.join("Bo", BO, &walls), Some(1));
    assert_eq!(players.join("Cy", BO, &walls), Some(2));
    assert_eq!(players.join("Di", BO, &walls), Some(3));
    assert_eq!(MAX_PLAYERS, 4);
    assert_eq!(players.join("Ed", BO, &walls), None, "the world is full");
    assert_eq!(players.len(), 4);
    for (i, a) in players.list.iter().enumerate() {
        for b in &players.list[i + 1..] {
            let apart = (a.color.r - b.color.r).abs() + (a.color.g - b.color.g).abs() + (a.color.b - b.color.b).abs();
            assert!(apart > 0.4, "{} and {} look alike", a.name, b.name);
        }
    }
    assert_eq!(players.list[0].color, BLUE, "the first player is the blue they always were");
}

#[test]
fn a_name_has_to_be_something_that_no_one_else_is_called() {
    let walls = Walls::default();
    let mut players = Players::solo(Player::new(0, "Ada", ADA, &walls));
    for refused in ["Ada", "ada", "  ADA ", "", "   ", "!!!", "<>"] {
        assert_eq!(players.join(refused, BO, &walls), None, "{refused:?} was let in");
        assert!(!name_is_free(refused, &players.list), "{refused:?}");
    }
    assert_eq!(players.len(), 1, "a refusal changes nothing");
    assert!(name_is_free("Adam", &players.list));
    assert_eq!(players.join("Adam", BO, &walls), Some(1));
}

#[test]
fn a_name_is_kept_without_what_does_not_belong_in_one_and_no_longer_than_it_may_be() {
    assert_eq!(clean_name("  Ada!! <b>  "), "Ada b");
    assert_eq!(clean_name("Zoë O'Neil-Smith_2.0"), "Zoë O'Neil-Smith_2.0".chars().take(MAX_NAME_LENGTH).collect::<String>().trim_end());
    assert_eq!(clean_name(&"x".repeat(40)).chars().count(), MAX_NAME_LENGTH);
    //cut where a space would be left on the end, it goes too
    assert_eq!(clean_name("abcdefghijklmno pqrst"), "abcdefghijklmno");
    assert_eq!(clean_name("tab\there\nnow"), "tabherenow");
    assert_eq!(clean_name(""), "");
}

#[test]
fn a_players_place_in_the_list_is_who_their_units_belong_to_and_a_missing_player_is_the_last() {
    let (players, _) = pair();
    assert_eq!(players.get(0).name, "Ada");
    assert_eq!(players.get(1).name, "Bo");
    assert_eq!(players.get(7).name, "Bo");
    assert_eq!(players.positions(), vec![ADA, BO]);
}

#[test]
fn each_player_has_their_own_relics_choices_wall_points_and_squads() {
    let (mut players, _) = pair();
    players.list[1].upgrades.take(Relic::Whetstone);
    players.list[1].rerolls = 0;
    players.list[1].mason.earn(25.0);
    players.list[1].choice = Some(RelicChoice::new());
    assert_eq!(players.list[0].upgrades.damage, 1.0);
    assert!(players.list[1].upgrades.damage > 1.0);
    assert!(players.list[0].choice.is_none() && players.list[0].rerolls > 0);
    assert_eq!(players.list[0].mason.points, START_WALL_POINTS);
    assert_eq!(players.list[1].mason.points, START_WALL_POINTS + 25.0);
    assert!(players.list[0].squads.is_empty() && players.list[1].squads.is_empty());
    //and what asks whose relics count gets the right ones
    assert_eq!(players.of(0).damage, 1.0);
    assert_eq!(players.of(1).damage, players.list[1].upgrades.damage);
}

#[test]
fn the_relics_of_a_list_are_by_owner_and_an_owner_past_the_end_has_the_last_ones() {
    let strong = Upgrades { damage: 3.0, ..Upgrades::default() };
    let list = vec![Upgrades::default(), strong.clone()];
    assert_eq!(list.of(0).damage, 1.0);
    assert_eq!(list.of(1).damage, 3.0);
    assert_eq!(list.of(9).damage, 3.0);
    assert_eq!(list.as_slice().of(1).damage, 3.0);
    //one set of relics is everyone's
    assert_eq!(strong.of(0).damage, 3.0);
    assert_eq!(strong.of(3).damage, 3.0);
}

#[test]
fn the_armies_are_held_to_the_same_limit_when_there_is_more_than_one_player() {
    let (mut players, walls) = pair();
    players.apply_settings(300, 0.5);
    assert!(players.list.iter().all(|player| player.upgrades.unit_cap() == MULTIPLAYER_ARMY_LIMIT && player.upgrades.foe_answer == 0.5));
    assert_eq!(MULTIPLAYER_ARMY_LIMIT, 12_500);
    //alone, it is what the setting says
    let mut alone = Players::solo(Player::new(0, "Ada", ADA, &walls));
    alone.apply_settings(300, 0.25);
    assert_eq!(alone.list[0].upgrades.unit_cap(), 300);
    assert_eq!(alone.list[0].upgrades.foe_answer, 0.25);
}

#[test]
fn the_towers_fire_with_the_relics_of_the_player_who_hits_hardest() {
    let (mut players, _) = pair();
    assert_eq!(players.archer().damage, 1.0);
    players.list[1].upgrades.take(Relic::Whetstone);
    assert!(players.archer().damage > 1.0);
    players.list[0].upgrades.take(Relic::Whetstone);
    players.list[0].upgrades.take(Relic::Whetstone);
    assert_eq!(players.archer().damage, players.list[0].upgrades.damage);
}

// ---------- walking ----------

#[test]
fn a_player_walks_at_their_speed_and_stays_in_the_world_and_out_of_the_walls() {
    let walls = Walls::default();
    let there = walk_player(ADA, Vec2::X, 300.0, 0.5, &walls);
    assert!((there - (ADA + Vec2::new(150.0, 0.0))).length() < 0.01);
    assert_eq!(walk_player(ADA, Vec2::ZERO, 300.0, 0.5, &walls), ADA);
    let edge = Vec2::new(RADIUS, RADIUS);
    assert_eq!(walk_player(edge, Vec2::new(-1.0, -1.0).normalize(), 500.0, 1.0, &walls), edge);
    let mut walled = Walls::default();
    let (column, row) = tile_of(ADA.x + 100.0, ADA.y);
    for r in row - 3..=row + 3 {
        walled.insert((column, r), Wall::new(column, r));
    }
    let blocked = walk_player(ADA, Vec2::X, 400.0, 1.0, &walled);
    assert!(blocked.x < column as f32 * TILE_SIZE, "walked into a wall: {blocked}");
}

#[test]
fn a_walking_player_is_seen_to_move_and_their_army_turns_to_face_the_way_they_go() {
    let (mut players, walls) = pair();
    let player = &mut players.list[1];
    assert_eq!(player.facing, Vec2::new(0.0, -1.0));
    for _ in 0..180 {
        let was = player.position;
        player.walk(Vec2::X, 400.0, 1.0 / 60.0, &walls);
        player.settle(was, 1.0 / 60.0);
    }
    assert!((player.position.x - (BO.x + 1200.0)).abs() < 1.0, "{}", player.position);
    assert!(player.velocity.x > 300.0 && player.velocity.y.abs() < 1.0, "{}", player.velocity);
    assert!(player.facing.x > 0.95, "{}", player.facing);
    //the other one hasn't moved, or turned
    assert_eq!(players.list[0].position, ADA);
    assert_eq!(players.list[0].facing, Vec2::new(0.0, -1.0));
}

// ---------- the armies follow their players ----------

#[test]
fn each_army_follows_its_own_player() {
    let (mut players, walls) = pair();
    let mut units = army_for(0, 40, ADA + Vec2::new(900.0, 700.0));
    units.extend(army_for(1, 40, BO - Vec2::new(1400.0, 700.0)));
    let armies = Armies::of(&mut units, &players);
    let started = (mean_distance(&own(&units, 0), ADA), mean_distance(&own(&units, 1), BO));
    let mut fx = Effects::default();
    for _ in 0..900 {
        step_armies(&mut players, &armies, &mut units, &[], &walls, false, 1.0 / 60.0, &mut fx);
    }
    let (ada, bo) = (own(&units, 0), own(&units, 1));
    assert_eq!((ada.len(), bo.len()), (40, 40));
    println!("distance to their player: {started:?} -> ({:.0}, {:.0})", mean_distance(&ada, ADA), mean_distance(&bo, BO));
    assert!(mean_distance(&ada, ADA) < 300.0 && mean_distance(&bo, BO) < 300.0);
    assert!(mean_distance(&ada, BO) > 3000.0 && mean_distance(&bo, ADA) > 3000.0, "an army went to the wrong player");
}

#[test]
fn an_army_goes_after_its_player_when_they_walk_off() {
    let (mut players, walls) = pair();
    let mut units = army_for(0, 30, ADA);
    units.extend(army_for(1, 30, BO));
    let armies = Armies::of(&mut units, &players);
    let mut fx = Effects::default();
    //they walk for two seconds, which is faster than the army can, and then stand while it catches up
    for frame in 0..900 {
        let player = &mut players.list[1];
        if frame < 120 {
            player.walk(Vec2::new(0.0, 1.0), 300.0, 1.0 / 60.0, &walls);
        }
        let at = player.position;
        //the route to them is made in full before the army uses it, so that the test is the same on any machine
        let route = &mut players.list[1].route;
        route.update(at, &walls, false, Duration::from_millis(1));
        route.finish();
        step_armies(&mut players, &armies, &mut units, &[], &walls, false, 1.0 / 60.0, &mut fx);
    }
    let (ada, bo) = (own(&units, 0), own(&units, 1));
    let walked_to = players.list[1].position;
    assert!(walked_to.y > BO.y + 500.0, "{walked_to}");
    assert!(mean_distance(&bo, walked_to) < 500.0, "{}", mean_distance(&bo, walked_to));
    assert!(mean_distance(&ada, ADA) < 400.0, "the other army stayed with its own player, {}", mean_distance(&ada, ADA));
}

#[test]
fn armies_of_different_players_do_not_push_each_other_about() {
    let (mut players, walls) = pair();
    let spot = Vec2::new(4000.0, 2000.0);
    //(with no shape to stand in: a formation gives each unit a place of its own, which would hold them apart whatever the pushing did)
    for player in players.list.iter_mut() {
        player.position = spot;
        player.formation = Formation::Free;
    }
    //two units of one player stacked on top of each other are pushed apart, and one of each player's are not
    let mut same = vec![unit_at(spot.x, spot.y), unit_at(spot.x + 1.0, spot.y)];
    let mut apart = vec![unit_at(spot.x, spot.y).owned_by(0), unit_at(spot.x + 1.0, spot.y).owned_by(1)];
    for units in [&mut same, &mut apart] {
        let armies = Armies::of(units, &players);
        let mut fx = Effects::default();
        for _ in 0..30 {
            step_armies(&mut players, &armies, units, &[], &walls, false, 1.0 / 60.0, &mut fx);
        }
    }
    println!("closest pair: same army {:.1}, two armies {:.1}", closest_pair(&same), closest_pair(&apart));
    assert!(closest_pair(&same) > closest_pair(&apart) + 1.0);
}

#[test]
fn the_champions_in_an_army_help_that_army_alone() {
    let (mut players, walls) = pair();
    players.list[0].position = BO;
    let mut units = vec![champion_unit(champion_named("Warlord").unwrap(), BO).owned_by(0), unit_at(BO.x + 60.0, BO.y).owned_by(0), unit_at(BO.x - 60.0, BO.y).owned_by(1)];
    let armies = Armies::of(&mut units, &players);
    step_armies(&mut players, &armies, &mut units, &[], &walls, false, 1.0 / 60.0, &mut Effects::default());
    let banner = match CHAMPIONS[champion_named("Warlord").unwrap()].power {
        Power::Banner { might, .. } => might,
        _ => panic!("the Warlord carries a banner"),
    };
    assert!((units[1].might - banner).abs() < 0.001, "its own army's unit hits harder: {}", units[1].might);
    assert_eq!(units[2].might, 1.0, "another player's does not");
}

#[test]
fn an_army_is_as_fast_as_its_own_size_and_relics_and_champions_make_it() {
    let (mut players, _) = pair();
    players.list[1].upgrades.take(Relic::FleetFeet);
    let mut units = army_for(0, 400, ADA);
    units.extend(army_for(1, 100, BO));
    units.push(champion_unit(champion_named("Pathfinder").unwrap(), BO).owned_by(1));
    let armies = Armies::of(&mut units, &players);
    assert_eq!((armies.size(0), armies.size(1)), (400, 101));
    assert_eq!(armies.perks[0].speed, 1.0);
    assert!(armies.perks[1].speed > 1.0);
    let ada = players.list[0].upgrades.army_boost(400);
    assert_eq!(armies.boosts[0], ada);
    assert!(armies.perked[1].speed > players.list[1].upgrades.speed, "the Pathfinder adds to its own army's speed");
    assert_eq!(armies.perked[0].speed, 1.0);
    assert_eq!(armies.boosts[1], armies.perked[1].army_boost(101));
    assert_eq!(armies.perked[0].pickup, 1.0);
    assert!(armies.perked[1].pickup > 1.0);
}

// ---------- squads ----------

#[test]
fn a_squad_is_its_players_and_no_one_elses_units_are_sent_with_it() {
    let (mut players, walls) = pair();
    let mut units = army_for(0, 20, ADA + Vec2::new(300.0, 0.0));
    units.extend(army_for(1, 20, ADA + Vec2::new(300.0, 100.0)));
    let enemy = dummy(ADA + Vec2::new(1000.0, 0.0));
    let armies = Armies::of(&mut units, &players);
    let sent = {
        let mut stretches = split_armies(&mut units, &armies.ranges);
        send_squad(&mut stretches[0], &mut players.list[0].squads, &enemy, &mut Effects::default())
    };
    assert!(sent > 0);
    assert!(own(&units, 0).iter().filter(|unit| unit.target == Some(enemy.id)).count() == sent);
    assert!(own(&units, 1).iter().all(|unit| unit.target.is_none()), "the other player's units were sent");
    //a squad goes on while the enemy does, and the other player has none
    let enemies = vec![enemy.clone()];
    let mut fx = Effects::default();
    for _ in 0..10 {
        step_armies(&mut players, &armies, &mut units, &enemies, &walls, false, 1.0 / 60.0, &mut fx);
    }
    assert_eq!(players.list[0].squads.len(), 1);
    assert!(players.list[1].squads.is_empty());
    assert_eq!(own(&units, 0).iter().filter(|unit| unit.target.is_some()).count(), sent);
    //once the enemy is gone it is disbanded, and its units go back to following their player
    for _ in 0..10 {
        step_armies(&mut players, &armies, &mut units, &[], &walls, false, 1.0 / 60.0, &mut fx);
    }
    assert!(players.list[0].squads.is_empty());
    assert!(units.iter().all(|unit| unit.target.is_none()));
}

#[test]
fn orders_from_a_squad_that_belongs_to_another_player_are_not_followed() {
    let (mut players, walls) = pair();
    let enemy = dummy(ADA + Vec2::new(1000.0, 0.0));
    let mut units = army_for(0, 10, ADA);
    units.extend(army_for(1, 10, BO));
    let armies = Armies::of(&mut units, &players);
    {
        let mut stretches = split_armies(&mut units, &armies.ranges);
        send_squad(&mut stretches[0], &mut players.list[0].squads, &enemy, &mut Effects::default());
    }
    //a unit of the other army that has the same orders has no squad to take them from
    units[10].target = Some(enemy.id);
    step_armies(&mut players, &armies, &mut units, &[enemy.clone()], &walls, false, 1.0 / 60.0, &mut Effects::default());
    assert_eq!(units[10].target, None);
    assert!(units[..10].iter().any(|unit| unit.target == Some(enemy.id)));
}

#[test]
fn recalling_a_players_squads_leaves_the_other_players_alone() {
    let (mut players, _) = pair();
    let enemy = dummy(ADA + Vec2::new(1000.0, 0.0));
    let mut units = army_for(0, 10, ADA);
    units.extend(army_for(1, 10, ADA));
    let armies = Armies::of(&mut units, &players);
    let mut stretches = split_armies(&mut units, &armies.ranges);
    let mut fx = Effects::default();
    let (first, second) = stretches.split_at_mut(1);
    let (sent_a, sent_b) = (
        send_squad(&mut first[0], &mut players.list[0].squads, &enemy, &mut fx),
        send_squad(&mut second[0], &mut players.list[1].squads, &enemy, &mut fx),
    );
    assert!(sent_a > 0 && sent_b > 0);
    let recalled = recall_squads(&mut stretches[0], &mut players.list[0].squads, ADA, &mut fx);
    assert_eq!(recalled, sent_a);
    assert!(players.list[0].squads.is_empty());
    assert_eq!(players.list[1].squads.len(), 1);
    assert_eq!(stretches[1].iter().filter(|unit| unit.target.is_some()).count(), sent_b);
}

// ---------- babies ----------

fn picker(at: Vec2, range: f32, army: usize, cap: usize) -> Picker {
    Picker { position: at, range, army, cap }
}

fn old_enough(at: Vec2) -> Baby {
    Baby { position: at, age: BABY_PICKUP_DELAY }
}

#[test]
fn a_baby_grows_for_the_nearest_player_who_can_reach_it() {
    let mut units = Vec::new();
    let mut babies = vec![old_enough(Vec2::new(300.0, 0.0))];
    let mut pickers = [picker(Vec2::ZERO, 500.0, 0, 100), picker(Vec2::new(400.0, 0.0), 500.0, 0, 100)];
    grow_babies_among(&mut babies, &mut units, &mut pickers, 0.01, &mut Effects::default());
    assert!(babies.is_empty());
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].owner, 1);
    assert_eq!((pickers[0].army, pickers[1].army), (0, 1));
}

#[test]
fn a_baby_goes_to_the_next_player_when_the_nearest_has_no_room_and_waits_when_no_one_has() {
    let mut units = Vec::new();
    let mut babies = vec![old_enough(Vec2::new(300.0, 0.0)), old_enough(Vec2::new(300.0, 10.0))];
    let mut pickers = [picker(Vec2::ZERO, 500.0, 0, 100), picker(Vec2::new(400.0, 0.0), 500.0, 5, 5)];
    grow_babies_among(&mut babies, &mut units, &mut pickers, 0.01, &mut Effects::default());
    assert_eq!(units.iter().map(|unit| unit.owner).collect::<Vec<_>>(), [0, 0], "the nearer one is full");
    //both full, and the babies stay
    let mut babies = vec![old_enough(Vec2::new(300.0, 0.0))];
    let mut pickers = [picker(Vec2::ZERO, 500.0, 5, 5), picker(Vec2::new(400.0, 0.0), 500.0, 5, 5)];
    grow_babies_among(&mut babies, &mut units, &mut pickers, 0.01, &mut Effects::default());
    assert_eq!((babies.len(), units.len()), (1, 2));
    //and one that is out of everyone's reach
    let mut babies = vec![old_enough(Vec2::new(3000.0, 0.0))];
    let mut pickers = [picker(Vec2::ZERO, 500.0, 0, 5), picker(Vec2::new(400.0, 0.0), 500.0, 0, 5)];
    grow_babies_among(&mut babies, &mut units, &mut pickers, 0.01, &mut Effects::default());
    assert_eq!((babies.len(), units.len()), (1, 2));
}

#[test]
fn a_baby_waits_a_moment_and_is_gone_after_its_life_whoever_is_near() {
    let mut units = Vec::new();
    let mut fx = Effects::default();
    let mut babies = vec![Baby { position: Vec2::new(10.0, 0.0), age: 0.0 }];
    let mut pickers = [picker(Vec2::ZERO, 500.0, 0, 100)];
    grow_babies_among(&mut babies, &mut units, &mut pickers, BABY_PICKUP_DELAY / 2.0, &mut fx);
    assert_eq!((babies.len(), units.len()), (1, 0), "it has only just dropped");
    //too far, so it waits until it times out
    let mut babies = vec![Baby { position: Vec2::new(5000.0, 0.0), age: 0.0 }];
    for _ in 0..(BABY_LIFE as usize - 1) {
        grow_babies_among(&mut babies, &mut units, &mut pickers, 1.0, &mut fx);
    }
    assert_eq!(babies.len(), 1);
    grow_babies_among(&mut babies, &mut units, &mut pickers, 1.5, &mut fx);
    assert!(babies.is_empty());
    assert!(units.is_empty());
}

#[test]
fn babies_become_units_of_the_player_whose_army_is_near_and_the_list_is_put_in_order() {
    let (players, _) = pair();
    let mut units = army_for(0, 5, ADA);
    units.extend(army_for(1, 5, BO));
    let armies = Armies::of(&mut units, &players);
    let mut babies = vec![old_enough(ADA + Vec2::new(0.0, 50.0)), old_enough(BO + Vec2::new(0.0, 50.0)), old_enough(BO + Vec2::new(10.0, 50.0)), old_enough(Vec2::new(4000.0, 20000.0))];
    let mut fx = Effects::default();
    let reaches = grow_armies(&players, &armies, &mut units, &mut babies, &[Vec2::new(1920.0, 1080.0); 2], 0.01, &mut fx);
    assert_eq!(reaches.len(), 2);
    assert_eq!(babies.len(), 1, "the one in the middle of nowhere is still there");
    assert_eq!(fx.grown[..2], [1, 2], "each army's own count, for whoever is watching");
    assert_eq!(army_sizes(&units)[..2], [6, 7]);
    //the new units are at the end of the list, and arranging it puts them with their army
    let ranges = arrange(&mut units, 2);
    assert_eq!(ranges, vec![0..6, 6..13]);
    assert!(units[..6].iter().all(|unit| unit.owner == 0) && units[6..].iter().all(|unit| unit.owner == 1));
}

#[test]
fn the_bigger_the_army_the_further_its_players_reach_for_babies() {
    let (players, _) = pair();
    let mut units = army_for(0, 5, ADA);
    units.extend(army_for(1, 400, BO));
    let armies = Armies::of(&mut units, &players);
    let reaches = grow_armies(&players, &armies, &mut units, &mut Vec::new(), &[Vec2::new(1920.0, 1080.0); 2], 0.01, &mut Effects::default());
    assert!(reaches[1] > reaches[0] && reaches[0] > 0.0, "{reaches:?}");
}

#[test]
fn a_player_with_a_full_army_picks_up_no_more_babies_and_the_others_still_do() {
    let (mut players, _) = pair();
    players.apply_settings(10, 0.0);
    players.list[1].position = ADA;
    //two players: the limit is the multiplayer one, so a small army is not full. make one full with a small cap by hand
    players.list[0].upgrades.army_limit = 5;
    let mut units = army_for(0, 5, ADA);
    units.extend(army_for(1, 5, ADA));
    let armies = Armies::of(&mut units, &players);
    let mut babies = vec![old_enough(ADA), old_enough(ADA + Vec2::new(5.0, 0.0))];
    grow_armies(&players, &armies, &mut units, &mut babies, &[Vec2::new(1920.0, 1080.0); 2], 0.01, &mut Effects::default());
    assert!(babies.is_empty());
    assert_eq!(army_sizes(&units)[..2], [5, 7], "both babies went to the player with room");
}

// ---------- fights ----------

#[test]
fn each_army_does_the_damage_its_own_relics_say_and_the_enemy_remembers_who_did_it() {
    let (mut players, _) = pair();
    players.list[1].upgrades.take(Relic::Whetstone);
    players.list[1].upgrades.take(Relic::Whetstone);
    let mut enemies = vec![dummy(ADA), dummy(BO)];
    let mut units = on(&enemies[0], 0, 10);
    units.extend(on(&enemies[1], 1, 10));
    let mut fx = Effects::default();
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &players, &mut fx);
    let (plain, sharp) = (enemies[0].hurt, enemies[1].hurt);
    assert!((plain - 10.0 * UNIT_DAMAGE * 0.1).abs() < 0.01, "{plain}");
    assert!((sharp - 10.0 * UNIT_DAMAGE * 0.1 * players.list[1].upgrades.hit()).abs() < 0.01, "{sharp}");
    assert!(sharp > plain * 1.5);
    assert_eq!((enemies[0].credit, enemies[1].credit), (0, 1));
}

#[test]
fn an_enemy_two_armies_are_fighting_gives_the_credit_to_the_one_doing_more() {
    let (mut players, _) = pair();
    let mut enemies = vec![dummy(ADA)];
    let mut units = on(&enemies[0], 0, 4);
    units.extend(on(&enemies[0], 1, 9));
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &players, &mut Effects::default());
    assert_eq!(enemies[0].credit, 1);
    assert!((enemies[0].hurt - 13.0 * UNIT_DAMAGE * 0.1).abs() < 0.01, "everyone's damage counts: {}", enemies[0].hurt);
    //the same few units win it with a relic that makes them hit harder
    players.list[0].upgrades.take(Relic::Whetstone);
    players.list[0].upgrades.take(Relic::Whetstone);
    players.list[0].upgrades.take(Relic::Whetstone);
    players.list[0].upgrades.take(Relic::Whetstone);
    players.list[0].upgrades.take(Relic::Whetstone);
    let mut enemies = vec![dummy(ADA)];
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &players, &mut Effects::default());
    assert_eq!(enemies[0].credit, 0, "{} against {}", players.list[0].upgrades.hit() * 4.0, 9.0);
}

#[test]
fn a_tougher_army_takes_less_from_the_same_blow_and_the_other_takes_all_of_it() {
    let (mut players, _) = pair();
    players.list[1].upgrades.take(Relic::IronResolve);
    let mut enemies = vec![dummy(ADA), dummy(BO)];
    for enemy in &mut enemies {
        enemy.attack = 40.0;
    }
    let mut units = on(&enemies[0], 0, 1);
    units.extend(on(&enemies[1], 1, 1));
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.5, &players, &mut Effects::default());
    let (lost_0, lost_1) = (UNIT_HEALTH - units[0].health, UNIT_HEALTH - units[1].health);
    println!("lost {lost_0} and {lost_1}");
    assert!((lost_0 - enemies[0].attack_now() * 0.5).abs() < 0.01);
    assert!((lost_1 - lost_0 * players.list[1].upgrades.toughness).abs() < 0.01);
    assert!(lost_1 < lost_0);
}

#[test]
fn thorns_hurt_the_enemy_for_the_player_who_has_them_alone() {
    let (mut players, _) = pair();
    players.list[1].upgrades.take(Relic::Thorns);
    let mut enemies = vec![dummy(ADA), dummy(BO)];
    for enemy in &mut enemies {
        enemy.attack = 40.0;
    }
    let mut units = on(&enemies[0], 0, 3);
    units.extend(on(&enemies[1], 1, 3));
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.5, &players, &mut Effects::default());
    let from_units = 3.0 * UNIT_DAMAGE * 0.5;
    assert!((enemies[0].hurt - from_units).abs() < 0.01, "{}", enemies[0].hurt);
    let thorns = enemies[1].hurt - from_units;
    assert!((thorns - enemies[1].attack_now() * 0.5 * players.list[1].upgrades.thorns).abs() < 0.01, "{thorns}");
}

#[test]
fn what_a_kill_is_worth_goes_to_the_player_who_got_it_with_their_own_relics() {
    let (mut players, _) = pair();
    players.list[1].upgrades.take(Relic::Bloodlust);
    players.list[1].upgrades.take(Relic::Bounty);
    let mut enemies = vec![dummy(BO)];
    enemies[0].health = 1.0;
    enemies[0].babies = 10;
    let mut units = on(&enemies[0], 1, 4);
    units.extend(army_for(0, 3, ADA));
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    let destroyed = fight(&mut enemies, &mut units, &mut babies, 0.1, &players, &mut fx);
    assert_eq!(destroyed, 1);
    assert!(fx.spoils[1].points > 0.0 && fx.spoils[1].xp > 0.0);
    assert_eq!(fx.spoils[1].heal_all, players.list[1].upgrades.bloodlust);
    assert_eq!(fx.spoils[0], Spoils::default(), "the player who wasn't there got nothing");
    assert_eq!(fx.spoils[2], Spoils::default());
    //the same kill by the first player is a smaller one
    let mut other = vec![dummy(ADA)];
    other[0].health = 1.0;
    other[0].babies = 10;
    let mut units = on(&other[0], 0, 4);
    let mut fx = Effects::default();
    let mut plain_babies = Vec::new();
    fight(&mut other, &mut units, &mut plain_babies, 0.1, &players, &mut fx);
    assert_eq!(fx.spoils[0].heal_all, 0.0);
    assert!(fx.spoils[0].points > 0.0 && fx.spoils[1].points == 0.0);
    assert!(babies.len() > plain_babies.len(), "{} babies against {}", babies.len(), plain_babies.len());
}

#[test]
fn a_martyr_dying_in_an_enemy_is_the_martyrs_players_and_only_if_they_have_the_relic() {
    let (mut players, _) = pair();
    players.list[1].upgrades.take(Relic::Martyr);
    let mut enemies = vec![dummy(ADA)];
    let before = enemies[0].health;
    martyr_blasts(&mut enemies, &[(ADA, 0)], &players);
    assert_eq!(enemies[0].health, before, "no relic, no blast");
    martyr_blasts(&mut enemies, &[(ADA, 1)], &players);
    assert!(enemies[0].health < before);
    assert_eq!(enemies[0].credit, 1);
}

#[test]
fn a_unit_that_dies_is_noted_with_the_player_it_fought_for() {
    let mut units = vec![unit_at(10.0, 10.0).owned_by(1), unit_at(20.0, 20.0).owned_by(0), unit_at(30.0, 30.0).owned_by(1)];
    units[0].health = 0.0;
    units[2].health = 0.0;
    let mut fx = Effects::default();
    assert_eq!(bury_units(&mut units, &mut fx), 2);
    assert_eq!(fx.fallen, vec![(Vec2::new(10.0, 10.0), 1), (Vec2::new(30.0, 30.0), 1)]);
    assert_eq!(units.len(), 1);
}

#[test]
fn one_players_relics_through_the_list_of_players_fight_exactly_as_the_same_relics_alone() {
    let mut up = Upgrades::default();
    for relic in [Relic::Whetstone, Relic::IronResolve, Relic::Thorns, Relic::Whetstone] {
        up.take(relic);
    }
    up.foe_answer = 0.4;
    let walls = Walls::default();
    let mut solo = Players::solo(Player::new(0, "Ada", ADA, &walls));
    solo.list[0].upgrades = up.clone();
    let run = |relics: &dyn Relics| {
        let mut enemies = vec![dummy(ADA), dummy(ADA + Vec2::new(5.0, 0.0))];
        enemies[0].attack = 90.0;
        let mut units = on(&enemies[0], 0, 17);
        for _ in 0..30 {
            fight(&mut enemies, &mut units, &mut Vec::new(), 1.0 / 60.0, relics, &mut Effects::default());
        }
        (enemies.iter().map(|enemy| enemy.health.to_bits()).collect::<Vec<_>>(), units.iter().map(|unit| unit.health.to_bits()).collect::<Vec<_>>())
    };
    assert_eq!(run(&up), run(&solo));
}

// ---------- looking after an army ----------

#[test]
fn hurt_units_heal_as_fast_as_their_own_players_relics_say() {
    let (mut players, _) = pair();
    players.list[1].upgrades.take(Relic::Regeneration);
    let mut units = army_for(0, 5, ADA);
    units.extend(army_for(1, 5, BO));
    for unit in &mut units {
        unit.health = 10.0;
    }
    heal_armies(&players, &mut units, 1.0);
    let (slow, quick) = (units[0].health, units[5].health);
    assert!((slow - (10.0 + UNIT_HEAL_RATE)).abs() < 0.01, "{slow}");
    assert!((quick - (10.0 + UNIT_HEAL_RATE * players.list[1].upgrades.healing)).abs() < 0.01, "{quick}");
    assert!(quick > slow);
}

#[test]
fn the_health_a_kill_gives_and_the_experience_it_teaches_go_to_the_army_that_got_it() {
    let (players, _) = pair();
    let mut units = army_for(0, 4, ADA);
    units.extend(army_for(1, 4, BO));
    units.push(champion_unit(champion_named("Juggernaut").unwrap(), ADA).owned_by(0));
    units.push(champion_unit(champion_named("Juggernaut").unwrap(), BO).owned_by(1));
    for unit in &mut units {
        unit.health = 40.0;
    }
    let mut fx = Effects::default();
    fx.spoils[1].heal_all = 25.0;
    fx.spoils[1].xp = 30.0;
    refresh_armies(&players, &mut units, &mut fx);
    train_armies(&players, &mut units, &mut fx);
    let healed = |units: &[Unit], owner: u8| units.iter().filter(|unit| unit.owner == owner && unit.champion == 0).map(|unit| unit.health).collect::<Vec<_>>();
    assert!(healed(&units, 0).iter().all(|&health| health == 40.0));
    assert!(healed(&units, 1).iter().all(|&health| health == 65.0));
    let level_and_xp = |owner: u8| units.iter().find(|unit| unit.owner == owner && unit.champion != 0).map(|unit| (unit.level, unit.xp)).unwrap();
    assert_eq!(level_and_xp(0), (units.iter().find(|unit| unit.owner == 0 && unit.champion != 0).unwrap().level, 0.0));
    let (level_1, xp_1) = level_and_xp(1);
    let (level_0, _) = level_and_xp(0);
    assert!(level_1 > level_0 || xp_1 > 0.0, "the second player's champion learned from it");
    assert_eq!((fx.spoils[1].heal_all, fx.spoils[1].xp), (0.0, 0.0), "it is used up");
}

#[test]
fn wall_points_go_to_the_player_who_earned_them_by_their_kills_or_from_the_world() {
    let (mut players, _) = pair();
    let mut units = army_for(0, 3, ADA);
    units.extend(army_for(1, 3, BO));
    let armies = Armies::of(&mut units, &players);
    let mut fx = Effects::default();
    fx.spoils[1].points = 10.0;
    fx.earn(0, 7.0);
    earn_wall_points(&mut players, &armies, 1.0 / 60.0, &mut fx);
    assert!((players.list[0].mason.points - (START_WALL_POINTS + 7.0)).abs() < 0.001, "{}", players.list[0].mason.points);
    assert!((players.list[1].mason.points - (START_WALL_POINTS + 10.0)).abs() < 0.001, "{}", players.list[1].mason.points);
    assert_eq!((fx.spoils[0].points, fx.spoils[1].points), (0.0, 0.0));
}

#[test]
fn a_quartermaster_makes_wall_points_for_its_own_player() {
    let (mut players, _) = pair();
    let mut units = army_for(0, 3, ADA);
    units.extend(army_for(1, 3, BO));
    units.push(champion_unit(champion_named("Quartermaster").unwrap(), BO).owned_by(1));
    let armies = Armies::of(&mut units, &players);
    for _ in 0..60 {
        earn_wall_points(&mut players, &armies, 1.0 / 60.0, &mut Effects::default());
    }
    assert_eq!(players.list[0].mason.points, START_WALL_POINTS);
    assert!(players.list[1].mason.points > START_WALL_POINTS + 1.0, "{}", players.list[1].mason.points);
}

// ---------- champions ----------

#[test]
fn a_champion_won_joins_the_army_of_the_winner_and_never_merges_into_another_players() {
    for _ in 0..100 {
        let mut units = vec![champion_unit(champion_named("Juggernaut").unwrap(), ADA).owned_by(0)];
        let level = units[0].level;
        let won = award_champion_for(1, &mut units, &mut Vec::new(), BO, Odds::Elite, MULTIPLAYER_ARMY_LIMIT, &mut Effects::default());
        assert!(won.is_some());
        assert_eq!(units.len(), 2, "it joined and did not merge");
        assert_eq!(units[0].level, level);
        assert_eq!(units[1].owner, 1);
    }
}

#[test]
fn a_full_roster_is_one_players_and_not_everyones() {
    let mut units: Vec<Unit> = (0..MAX_CHAMPIONS).map(|kind| champion_unit(kind, ADA).owned_by(0)).collect();
    let mut babies = Vec::new();
    let won = award_champion_for(1, &mut units, &mut babies, BO, Odds::Elite, MULTIPLAYER_ARMY_LIMIT, &mut Effects::default());
    assert!(won.is_some() && units.len() == MAX_CHAMPIONS + 1 && babies.is_empty(), "the second player has room");
    assert_eq!(units.last().map(|unit| unit.owner), Some(1));
    let won = award_champion_for(0, &mut units, &mut babies, ADA, Odds::Elite, MULTIPLAYER_ARMY_LIMIT, &mut Effects::default());
    assert!(won.is_none() || units.len() == MAX_CHAMPIONS + 1, "the first has none to spare");
}

// ---------- saving ----------

#[test]
fn a_saved_game_has_one_army_so_the_marks_are_not_kept_and_everything_else_is() {
    let mut world = new_run();
    world.units.clear();
    world.units.extend(army_for(0, 30, Vec2::splat(15000.0)));
    world.units.extend(army_for(1, 30, Vec2::splat(15500.0)));
    let loaded = Snapshot::from_text(&world.to_text()).expect("the save reads back");
    assert_eq!(loaded.units.len(), 60);
    assert!(loaded.units.iter().all(|unit| unit.owner == 0));
    assert_eq!(loaded.body(), Snapshot::from_text(&loaded.to_text()).unwrap().body());
}

// ---------- the cost of it ----------

#[test]
#[ignore]
fn four_full_armies_cost_a_frame_how_long() {
    let walls = Walls::default();
    let mut players = Players::solo(Player::new(0, "A", Vec2::new(8000.0, 8000.0), &walls));
    for (name, at) in [("B", Vec2::new(24000.0, 8000.0)), ("C", Vec2::new(8000.0, 24000.0)), ("D", Vec2::new(24000.0, 24000.0))] {
        players.join(name, at, &walls).expect("room");
    }
    players.apply_settings(DEFAULT_ARMY_LIMIT, 0.0);
    let mut units = Vec::new();
    for (owner, player) in players.list.iter().enumerate() {
        units.extend((0..MULTIPLAYER_ARMY_LIMIT).map(|_| {
            let around = random_position_around(player.position.x, player.position.y, 50.0, 1500.0);
            unit_at(around.x, around.y).owned_by(owner)
        }));
    }
    let mut fx = Effects::default();
    let mut worst = Duration::ZERO;
    let mut total = Duration::ZERO;
    let frames = 120;
    for _ in 0..frames {
        let started = Instant::now();
        let armies = Armies::of(&mut units, &players);
        step_armies(&mut players, &armies, &mut units, &[], &walls, false, 1.0 / 120.0, &mut fx);
        heal_armies(&players, &mut units, 1.0 / 120.0);
        let spent = started.elapsed();
        worst = worst.max(spent);
        total += spent;
    }
    println!("{} units: {:.2} ms a frame on average, {:.2} ms at worst", units.len(), total.as_secs_f64() * 1000.0 / frames as f64, worst.as_secs_f64() * 1000.0);
}
