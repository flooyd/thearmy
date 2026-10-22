//a captain rallies the crowd round it, and when it falls the crowd it was rallying breaks and runs

use super::*;

const CENTRE: Vec2 = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);

fn captain_at(at: Vec2) -> Enemy {
    let mut captain = kind_at("Titan", at);
    captain.make_captain(600);
    join_horde(&mut captain);
    captain
}

fn follower_at(at: Vec2) -> Enemy {
    let mut follower = kind_at("Grunt", at);
    join_horde(&mut follower);
    follower
}

//a spot this many pixels from a captain's edge, going east
fn beside(captain: &Enemy, gap: f32) -> Vec2 {
    captain.position + Vec2::new(captain.size / 2.0 + gap, 0.0)
}

#[test]
fn the_crowd_near_a_living_captain_is_rallied_and_nobody_else_is() {
    rand::srand(21);
    let captain = captain_at(CENTRE);
    let reach = rally_reach(&captain);
    assert_eq!(reach, RALLY_TILES * TILE_SIZE + captain.size / 2.0);
    let near = follower_at(beside(&captain, 100.0));
    let edge = follower_at(CENTRE + Vec2::new(reach - 5.0, 0.0));
    let far = follower_at(CENTRE + Vec2::new(reach + 50.0, 0.0));
    let mut wild = kind_at("Grunt", beside(&captain, 100.0));
    wild.chasing = true;
    let mut enemies = vec![captain, near, edge, far, wild];
    assert_eq!(rally_followers(&mut enemies), 2);
    let flags: Vec<bool> = enemies.iter().map(|enemy| enemy.rallied).collect();
    assert_eq!(flags, vec![false, true, true, false, false], "(the captain, two near it, one too far, and a wild one that is nobody's crowd)");
    //it is worked out again every frame: the crowd that moves out of reach is not rallied any more
    enemies[1].position = CENTRE + Vec2::new(reach + 200.0, 0.0);
    assert_eq!(rally_followers(&mut enemies), 1);
    assert!(!enemies[1].rallied && enemies[2].rallied);
}

#[test]
fn a_captain_that_is_down_or_gone_rallies_no_one_and_a_second_captain_rallies_its_own() {
    rand::srand(22);
    let first = captain_at(CENTRE);
    let second = captain_at(CENTRE + Vec2::new(9000.0, 0.0));
    let by_first = follower_at(beside(&first, 50.0));
    let by_second = follower_at(beside(&second, 50.0));
    let mut enemies = vec![first, second, by_first, by_second];
    assert_eq!(rally_followers(&mut enemies), 2);
    //one is dead (it is taken out of the world a moment later): its crowd is not rallied by it any more
    enemies[0].health = 0.0;
    assert_eq!(rally_followers(&mut enemies), 1);
    assert!(!enemies[2].rallied && enemies[3].rallied);
    enemies.remove(1);
    assert_eq!(rally_followers(&mut enemies), 0, "no captain at all");
    assert!(enemies.iter().all(|enemy| !enemy.rallied));
}

#[test]
fn a_boss_a_captain_and_a_runner_are_not_rallied_by_one() {
    rand::srand(23);
    let captain = captain_at(CENTRE);
    let other_captain = captain_at(beside(&captain, 40.0));
    let mut boss = Enemy::boss(&BOSSES[0], 600, 0, beside(&captain, 80.0));
    join_horde(&mut boss);
    let mut runner = follower_at(beside(&captain, 60.0));
    runner.break_and_run();
    let mut enemies = vec![captain, other_captain, boss, runner];
    assert_eq!(rally_followers(&mut enemies), 0);
}

#[test]
fn a_rallied_enemy_hits_harder_and_runs_faster_and_a_dash_gets_both() {
    rand::srand(24);
    let plain = follower_at(CENTRE);
    let mut rallied = plain.clone();
    rallied.rallied = true;
    assert_eq!(plain.attack_now(), plain.attack);
    assert!((rallied.attack_now() / plain.attack_now() - RALLY_ATTACK).abs() < 1.0e-4);
    rallied.mode = Mode::Dash { dir: Vec2::X, left: 0.2 };
    assert!((rallied.attack_now() - plain.attack * RALLY_ATTACK * CHARGE_DAMAGE).abs() < 1.0e-3);
    rallied.mode = Mode::Roam;
    assert!((rallied.pace() / plain.pace() - RALLY_SPEED).abs() < 1.0e-5);
    //tar and a rally together
    let mut stuck = rallied.clone();
    stuck.slow = TAR_SLOW;
    assert!((stuck.pace() - TAR_SLOW * RALLY_SPEED).abs() < 1.0e-5);
    //running at a player a long way off, it covers more ground in the same time
    let player = CENTRE + Vec2::new(4000.0, 0.0);
    let (mut a, mut b) = (vec![plain], vec![rallied]);
    let mut fx = Effects::default();
    for _ in 0..10 {
        patrol_free(&mut a, player, 0.1, 1.0, &mut fx);
        patrol_free(&mut b, player, 0.1, 1.0, &mut fx);
    }
    let (went_a, went_b) = (a[0].position.x - CENTRE.x, b[0].position.x - CENTRE.x);
    assert!(went_a > 50.0, "it ran: {went_a}");
    assert!((went_b / went_a - RALLY_SPEED).abs() < 0.01, "{went_b} against {went_a}");
}

#[test]
fn a_captain_falling_routs_the_crowd_it_was_rallying_and_only_that() {
    rand::srand(25);
    let mut captain = captain_at(CENTRE);
    let reach = rally_reach(&captain);
    let inside = [follower_at(beside(&captain, 60.0)), follower_at(CENTRE + Vec2::new(0.0, -reach + 20.0)), follower_at(CENTRE - Vec2::new(reach - 20.0, 0.0))];
    let outside = follower_at(CENTRE + Vec2::new(reach + 80.0, 0.0));
    let mut wild = kind_at("Grunt", beside(&captain, 60.0));
    wild.chasing = true;
    let mut boss = Enemy::boss(&BOSSES[0], 600, 0, beside(&captain, 200.0));
    join_horde(&mut boss);
    captain.health = 0.0;
    let mut enemies = vec![captain, outside, wild, boss];
    enemies.extend(inside);
    let mut fx = Effects::default();
    let destroyed = resolve_deaths(&mut enemies, &mut Vec::new(), 600, &Upgrades::default(), &mut fx);
    assert_eq!(destroyed, 1);
    assert_eq!(enemies.len(), 6, "the captain is gone");
    let routed: Vec<bool> = enemies.iter().map(|enemy| enemy.routed()).collect();
    assert_eq!(routed, vec![false, false, false, true, true, true], "(the one too far, a wild one, a boss, then the three it was rallying)");
    assert!(enemies[3..].iter().all(|enemy| enemy.rout == ROUT_SECONDS && !enemy.rallied));
    assert!(fx.announcements.iter().any(|(text, _)| text == "CAPTAIN FALLEN: 3 ROUTED"), "{:?}", fx.announcements.iter().map(|(text, _)| text).collect::<Vec<_>>());
    assert!(fx.popups.iter().any(|popup| popup.text == "ROUTED!"));
    assert_eq!(fx.rings.len(), 1);
}

#[test]
fn a_captain_that_falls_with_no_one_round_it_just_falls() {
    rand::srand(26);
    let mut captain = captain_at(CENTRE);
    captain.health = 0.0;
    let mut enemies = vec![captain, follower_at(CENTRE + Vec2::new(9000.0, 0.0))];
    let mut fx = Effects::default();
    resolve_deaths(&mut enemies, &mut Vec::new(), 600, &Upgrades::default(), &mut fx);
    assert!(!enemies[0].routed());
    assert!(fx.announcements.is_empty(), "no one to say anything about");
    assert!(fx.popups.iter().any(|popup| popup.text == "CAPTAIN FALLEN"));
}

#[test]
fn an_ordinary_enemy_falling_routs_no_one() {
    rand::srand(27);
    let mut fallen = follower_at(CENTRE);
    fallen.health = 0.0;
    let mut elite = follower_at(CENTRE + Vec2::new(30.0, 0.0));
    elite.make_elite(Modifier::Frenzied);
    elite.health = 0.0;
    let mut enemies = vec![fallen, elite, follower_at(CENTRE + Vec2::new(60.0, 0.0))];
    resolve_deaths(&mut enemies, &mut Vec::new(), 600, &Upgrades::default(), &mut Effects::default());
    assert!(enemies.iter().all(|enemy| !enemy.routed()));
}

#[test]
fn a_pincer_that_loses_a_captain_routs_that_side_and_the_other_side_fights_on() {
    rand::srand(28);
    let reach = HORDE_DISTANCE_TILES * TILE_SIZE;
    let mut hordes = Hordes::new();
    let mut enemies = Vec::new();
    hordes.coming = Some(Coming { def: horde_named("Pincer Host").unwrap(), at: CENTRE + Vec2::new(reach, 0.0), left: 0.5, other: Some(CENTRE - Vec2::new(reach, 0.0)) });
    hordes.update(&mut enemies, CENTRE, 600, 600, false, 1.0);
    let followers = enemies.iter().filter(|enemy| !enemy.is_captain()).count();
    assert_eq!(rally_followers(&mut enemies), followers, "everyone starts round a captain, so everyone is rallied");
    //the east captain is brought down
    let east = enemies.iter().position(|enemy| enemy.is_captain() && enemy.position.x > CENTRE.x).unwrap();
    enemies[east].health = 0.0;
    resolve_deaths(&mut enemies, &mut Vec::new(), 600, &Upgrades::default(), &mut Effects::default());
    let (eastern, western): (Vec<&Enemy>, Vec<&Enemy>) = enemies.iter().filter(|enemy| !enemy.is_captain()).partition(|enemy| enemy.position.x > CENTRE.x);
    assert!(!eastern.is_empty() && !western.is_empty());
    assert!(eastern.iter().all(|enemy| enemy.routed()), "all of its side ran");
    assert!(western.iter().all(|enemy| !enemy.routed()), "the other side did not");
    let western = western.len();
    assert_eq!(rally_followers(&mut enemies), western, "and only the other side is rallied now");
}

#[test]
fn a_routed_enemy_runs_from_the_player_does_nothing_else_and_then_turns_back() {
    rand::srand(29);
    let player = CENTRE;
    let mut enemies = vec![follower_at(player + Vec2::new(500.0, 0.0))];
    let mut fx = Effects::default();
    patrol_free(&mut enemies, player, 0.1, 1.0, &mut fx);
    let chasing_at = enemies[0].position.x;
    assert!(chasing_at < player.x + 500.0, "it comes at the player");
    //its captain falls: away it goes
    enemies[0].break_and_run();
    assert_eq!(enemies[0].attack_now(), 0.0);
    let start = enemies[0].position;
    for _ in 0..20 {
        patrol_free(&mut enemies, player, 0.1, 1.0, &mut fx);
    }
    assert!(enemies[0].position.distance(player) > start.distance(player) + 100.0, "it ran: {} from {}", enemies[0].position.distance(player), start.distance(player));
    assert!((enemies[0].rout - (ROUT_SECONDS - 2.0)).abs() < 0.01);
    //(a little slower than it chased)
    assert!(enemies[0].position.distance(start) < enemies[0].speed * CHASE_SPEED * 2.0 * ROUT_SPEED * 1.01 + 1.0);
    for _ in 0..30 {
        patrol_free(&mut enemies, player, 0.1, 1.0, &mut fx);
    }
    assert!(!enemies[0].routed(), "the rout is over");
    assert!(enemies[0].attack_now() > 0.0);
    let spent = enemies[0].position;
    for _ in 0..10 {
        patrol_free(&mut enemies, player, 0.1, 1.0, &mut fx);
    }
    assert!(enemies[0].position.distance(player) < spent.distance(player), "and it turns back at the army");
}

#[test]
fn a_routed_charger_gives_up_its_dash_and_a_routed_shooter_holds_its_fire() {
    rand::srand(30);
    let mut charger = kind_at("Charger", CENTRE + Vec2::new(400.0, 0.0));
    join_horde(&mut charger);
    charger.mode = Mode::Dash { dir: -Vec2::X, left: 0.3 };
    charger.break_and_run();
    assert_eq!(charger.mode, Mode::Roam);
    let mut windup = kind_at("Charger", CENTRE);
    windup.mode = Mode::Windup { dir: Vec2::X, left: 0.3 };
    windup.break_and_run();
    assert_eq!(windup.mode, Mode::Roam);
    //a bomber with a lit fuse keeps it
    let mut bomber = kind_at("Bomber", CENTRE);
    bomber.mode = Mode::Fuse { left: 1.0 };
    bomber.break_and_run();
    assert!(matches!(bomber.mode, Mode::Fuse { .. }));

    let mut spitter = kind_at("Spitter", CENTRE + Vec2::new(200.0, 0.0));
    join_horde(&mut spitter);
    spitter.shot_in = 0.0;
    let units = vec![unit_at(CENTRE.x, CENTRE.y)];
    let mut shots = Vec::new();
    let mut enemies = vec![spitter];
    enemies[0].break_and_run();
    shoot_enemies(&mut enemies, &units, &mut shots, 0.5, &mut Effects::default());
    assert!(shots.is_empty(), "routed, it does not fire");
    enemies[0].rout = 0.0;
    shoot_enemies(&mut enemies, &units, &mut shots, 0.5, &mut Effects::default());
    assert!(!shots.is_empty(), "and when it has turned back it does");
}

#[test]
fn a_routed_enemy_hurts_no_unit_but_is_hurt_by_them() {
    rand::srand(31);
    let mut enemy = kind_at("Brute", CENTRE);
    join_horde(&mut enemy);
    enemy.max_health = 1.0e6;
    enemy.health = enemy.max_health;
    enemy.break_and_run();
    let mut units = units_inside(&enemy, 20);
    let health: Vec<f32> = units.iter().map(|unit| unit.health).collect();
    let mut enemies = vec![enemy];
    let before = enemies[0].health;
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.5, &Upgrades::default(), &mut Effects::default());
    assert_eq!(units.iter().map(|unit| unit.health).collect::<Vec<_>>(), health, "not a scratch on them");
    assert!(enemies[0].health < before, "but it can still be cut down while it runs");
}

#[test]
fn the_tooltip_says_when_an_enemy_is_rallied_or_routed_and_neither_is_kept_in_a_save() {
    rand::srand(32);
    let mut enemy = follower_at(CENTRE);
    assert!(!traits(&enemy).iter().any(|line| line.contains("rallied") || line.contains("routed")));
    enemy.rallied = true;
    assert!(traits(&enemy).iter().any(|line| line == "rallied by its captain"));
    enemy.break_and_run();
    assert!(!enemy.rallied, "(it is not rallied while it runs)");
    assert!(traits(&enemy).iter().any(|line| line == "routed, running away"));
    enemy.rallied = true;
    let mut writer = SaveWriter::new();
    writer.put(&enemy);
    let text = writer.finish();
    let back: Enemy = SaveReader::new(&text).get().unwrap();
    assert_eq!((back.rout, back.rallied), (0.0, false));
    assert!(in_a_horde(&back) && back.name == "Grunt");
}
