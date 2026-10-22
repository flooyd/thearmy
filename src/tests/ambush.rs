//ambushers that lie hidden until the player comes by, and enemies that fight back against the units hurting them

use super::*;

// ---------- ambushers ----------

fn ambusher_at(at: Vec2) -> Enemy {
    kind_at("Ambusher", at)
}

//an enemy that stays put, so only what is being tested moves it
fn rooted(name: &str, at: Vec2, health: f32) -> Enemy {
    let mut enemy = kind_at(name, at);
    enemy.speed = 0.0;
    enemy.max_health = health;
    enemy.health = health;
    enemy
}

#[test]
fn an_ambusher_lies_hidden_and_cannot_be_clicked_mapped_hurt_or_targeted() {
    let enemy = ambusher_at(MIDDLE);
    assert!(enemy.lurking() && enemy.behavior == Behavior::Ambusher && kind_named("Ambusher").pack > 0);
    let mut enemies = vec![enemy];
    //nobody can click it, in the world or on the map
    assert_eq!(enemy_at(MIDDLE, &enemies, 50.0), None);
    let map = Minimap::new(Vec2::new(1280.0, 720.0), MapView::Near, MIDDLE);
    assert_eq!(minimap_enemy_at(&map, map.to_screen(MIDDLE), &enemies), None);
    //nothing hurts it, whatever the source
    assert_eq!(enemies[0].take_damage(1.0e6), 0.0);
    assert_eq!(enemies[0].health, enemies[0].max_health);
    let mut fx = Effects::default();
    let mut walls = Walls::default();
    let tower = Wall::made(tile_of(MIDDLE.x, MIDDLE.y).0 + 2, tile_of(MIDDLE.x, MIDDLE.y).1, WallKind::Tower, Side::Friendly, 100.0);
    walls.insert((tower.col, tower.row), tower);
    walls.values_mut().for_each(|wall| wall.garrison = 5);
    fire_towers(&mut walls, &mut enemies, &Upgrades::default(), 1.0, &mut fx);
    assert!(fx.beams.is_empty(), "a tower has nothing to shoot at");
    assert_eq!(enemies[0].health, enemies[0].max_health);
    //it can't be frozen either
    assert!(!enemies[0].can_freeze());
}

#[test]
fn an_ambusher_springs_when_the_player_comes_close_and_not_before() {
    let mut enemies = vec![ambusher_at(MIDDLE)];
    let mut fx = Effects::default();
    let trigger = AMBUSH_TRIGGER_TILES * TILE_SIZE + enemies[0].size / 2.0;
    assert_eq!(spring_ambushes(&mut enemies, MIDDLE + Vec2::new(trigger + 10.0, 0.0), &mut fx), 0);
    assert!(enemies[0].lurking() && fx.popups.is_empty());
    assert_eq!(spring_ambushes(&mut enemies, MIDDLE + Vec2::new(trigger - 10.0, 0.0), &mut fx), 1);
    assert!(!enemies[0].lurking() && enemies[0].chasing, "it is after the player now");
    assert!(matches!(enemies[0].mode, Mode::Dash { dir, .. } if dir.x > 0.9), "it rushes at them");
    assert_eq!(fx.popups.iter().filter(|popup| popup.text == "AMBUSH!").count(), 1);
    assert!(fx.cues.contains(&Cue::Alert) && fx.shake > 0.0);
    //once is enough
    assert_eq!(spring_ambushes(&mut enemies, MIDDLE, &mut Effects::default()), 0);
}

#[test]
fn the_rest_of_a_pack_springs_with_the_first_but_a_far_pack_stays_hidden() {
    let near_pack: Vec<Enemy> = (0..5).map(|k| ambusher_at(MIDDLE + Vec2::new(100.0 * k as f32, 60.0))).collect();
    let far_pack: Vec<Enemy> = (0..3).map(|k| ambusher_at(MIDDLE + Vec2::new(5000.0 + 100.0 * k as f32, 0.0))).collect();
    let mut enemies = [near_pack, far_pack].concat();
    //the player is beside the first of them only
    let mut fx = Effects::default();
    let sprung = spring_ambushes(&mut enemies, MIDDLE - Vec2::new(150.0, 0.0), &mut fx);
    assert_eq!(sprung, 5);
    assert!(enemies[..5].iter().all(|enemy| !enemy.lurking()));
    assert!(enemies[5..].iter().all(|enemy| enemy.lurking()));
}

#[test]
fn an_ambusher_that_a_unit_stumbles_on_springs_and_the_ones_beside_it_follow_next_frame() {
    let mut enemies = vec![ambusher_at(MIDDLE), ambusher_at(MIDDLE + Vec2::new(150.0, 0.0))];
    let mut units = vec![unit_on(MIDDLE)];
    let mut fx = Effects::default();
    let health = enemies[0].health;
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &Upgrades::default(), &mut fx);
    assert!(!enemies[0].lurking() && enemies[1].lurking());
    assert_eq!(enemies[0].health, health, "finding it is not hurting it yet");
    assert_eq!(units[0].health, UNIT_HEALTH, "and it has not hit yet");
    //the other one was far from any unit and from the player, but it is next to one that has sprung
    spring_ambushes(&mut enemies, MIDDLE + Vec2::new(9000.0, 0.0), &mut fx);
    assert!(!enemies[1].lurking());
}

#[test]
fn a_sprung_ambusher_hits_hard_in_its_rush_then_is_winded_and_keeps_chasing() {
    let mut enemies = vec![ambusher_at(MIDDLE)];
    let mut fx = Effects::default();
    spring_ambushes(&mut enemies, MIDDLE + Vec2::new(100.0, 0.0), &mut fx);
    assert_eq!(enemies[0].attack_now(), enemies[0].attack * CHARGE_DAMAGE);
    let start = enemies[0].position;
    let player = MIDDLE + Vec2::new(100.0, 0.0);
    let mut seconds = 0.0;
    while matches!(enemies[0].mode, Mode::Dash { .. }) && seconds < 2.0 {
        patrol_free(&mut enemies, player, 0.05, 1.0, &mut fx);
        seconds += 0.05;
    }
    assert!(enemies[0].position.x > start.x + 100.0, "it lunged forward");
    assert!(seconds <= AMBUSH_LUNGE_TIME + 0.2, "{seconds}");
    assert!(matches!(enemies[0].mode, Mode::Recover { .. }));
    assert_eq!(enemies[0].attack_now(), enemies[0].attack);
    while !matches!(enemies[0].mode, Mode::Roam) && seconds < 6.0 {
        patrol_free(&mut enemies, player, 0.05, 1.0, &mut fx);
        seconds += 0.05;
    }
    assert!(enemies[0].chasing, "once it has sprung it chases like any other");
}

#[test]
fn a_lurking_ambusher_does_nothing_at_all() {
    let mut enemies = vec![ambusher_at(MIDDLE)];
    let before = enemies[0].position;
    let mut fx = Effects::default();
    for _ in 0..100 {
        patrol_free(&mut enemies, MIDDLE + Vec2::new(5000.0, 0.0), 0.05, 1.0, &mut fx);
        enemy_abilities(&mut enemies, &[], MIDDLE, 0.05, &mut fx);
    }
    assert_eq!(enemies[0].position, before);
    assert!(enemies[0].lurking());
}

#[test]
fn an_ambusher_is_shown_only_as_faint_ground_and_a_pack_is_small_whatever_the_army() {
    let pack = (0..4000)
        .map(|_| spawn_group(MIDDLE.x, MIDDLE.y, MAX_ARMY_LIMIT))
        .find(|group| group[0].behavior == Behavior::Ambusher)
        .expect("an ambush turns up in a few thousand groups");
    assert!(pack.len() > 1 && pack.len() <= MAX_AMBUSH_PACK, "{}", pack.len());
    assert!(pack.iter().all(|enemy| enemy.lurking()));
    //a small army meets none: they come once there is something worth ambushing
    for _ in 0..500 {
        let group = spawn_group(MIDDLE.x, MIDDLE.y, kind_named("Ambusher").from_army - 1);
        assert!(group.iter().all(|enemy| enemy.behavior != Behavior::Ambusher));
    }
}

#[test]
fn ambushers_waiting_are_never_more_than_their_share_of_the_wild_enemies() {
    let mut enemies = Vec::new();
    for round in 0..40 {
        //the player stands about, killing what they meet: all but the ambushers in the way are replaced by new groups
        enemies.retain(|enemy: &Enemy| enemy.lurking() || round % 7 != 0);
        respawn_enemies(&mut enemies, MIDDLE.x, MIDDLE.y, 3000);
        let lurkers = enemies.iter().filter(|enemy| enemy.lurking()).count();
        assert!(lurkers <= ENEMY_COUNT / MAX_LURKERS_SHARE + MAX_AMBUSH_PACK, "{lurkers} lie in wait at round {round}");
        //anything that dies leaves room
        enemies.retain(|enemy| enemy.lurking() || enemy.id % 3 != 0);
    }
}

#[test]
fn a_lurking_ambusher_is_kept_in_a_save_and_still_lurks_when_it_comes_back() {
    let mut writer = SaveWriter::new();
    writer.put(&ambusher_at(MIDDLE));
    let mut sprung = ambusher_at(MIDDLE + Vec2::new(300.0, 0.0));
    sprung.spring(MIDDLE, &mut Effects::default());
    writer.put(&sprung);
    let text = writer.finish();
    let mut reader = SaveReader::new(&text);
    let back: Enemy = reader.get().unwrap();
    assert!(back.lurking() && back.behavior == Behavior::Ambusher);
    let back: Enemy = reader.get().unwrap();
    assert!(!back.lurking() && back.chasing && matches!(back.mode, Mode::Dash { .. }));
}

#[test]
fn the_tooltip_says_what_an_ambusher_does_and_what_a_provoked_enemy_is_up_to() {
    let mut enemy = ambusher_at(MIDDLE);
    assert!(traits(&enemy).iter().any(|line| line.contains("springs")));
    enemy.spring(MIDDLE, &mut Effects::default());
    enemy.chasing = false;
    assert!(!traits(&enemy).iter().any(|line| line.contains("fighting back")));
    enemy.provoke(MIDDLE);
    assert!(traits(&enemy).iter().any(|line| line.contains("fighting back")));
}

// ---------- retaliation ----------

#[test]
fn an_enemy_hurt_by_units_is_provoked_and_rings_the_alarm_once_while_the_fight_goes_on() {
    let mut enemies = vec![rooted("Grunt", MIDDLE, 1.0e6)];
    let mut units = units_inside(&enemies[0], 6);
    let mut fx = Effects::default();
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &Upgrades::default(), &mut fx);
    assert_eq!(enemies[0].provoked, RETALIATE_SECONDS);
    assert!(enemies[0].grudge.distance(MIDDLE) < enemies[0].size, "it knows where they are");
    assert_eq!(fx.alarms.len(), 1);
    assert_eq!(fx.alarms[0].0, MIDDLE);
    for _ in 0..20 {
        fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &Upgrades::default(), &mut fx);
    }
    assert_eq!(fx.alarms.len(), 1, "not again and again");
    assert_eq!(enemies[0].provoked, RETALIATE_SECONDS, "but the fight keeps it provoked");
}

#[test]
fn bosses_nests_frozen_enemies_and_healers_are_not_provoked() {
    let mut boss = Enemy::boss(&BOSSES[0], 500, 0, MIDDLE);
    boss.health = boss.max_health;
    let mut nest = kind_at("Nest", MIDDLE);
    nest.health = 1.0e6;
    nest.max_health = 1.0e6;
    let mut frozen = rooted("Grunt", MIDDLE, 1.0e6);
    frozen.frozen = 1.0;
    for mut enemy in [boss, nest, frozen] {
        let mut units = units_inside(&enemy, 6);
        let mut fx = Effects::default();
        let mut enemies = vec![enemy.clone()];
        fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &Upgrades::default(), &mut fx);
        assert_eq!(enemies[0].provoked, 0.0, "{}", enemy.name);
        assert!(fx.alarms.is_empty(), "{}", enemy.name);
        enemy.provoked = 0.0;
    }
}

#[test]
fn a_provoked_enemy_goes_to_the_units_that_hurt_it_instead_of_its_route() {
    let mut enemy = kind_at("Grunt", MIDDLE);
    enemy.speed = 100.0;
    enemy.aggro = 0.0;
    enemy.route = vec![MIDDLE + Vec2::new(0.0, 2000.0)];
    let mut plain = enemy.clone();
    enemy.provoke(MIDDLE + Vec2::new(600.0, 0.0));
    let mut enemies = vec![enemy, plain.clone()];
    plain.provoked = 0.0;
    let mut fx = Effects::default();
    patrol_free(&mut enemies, NOBODY, 0.1, 1.0, &mut fx);
    assert!((enemies[0].position - MIDDLE - Vec2::new(100.0 * CHASE_SPEED * 0.1, 0.0)).length() < 0.5, "it ran at them, at a chase's speed: {:?}", enemies[0].position - MIDDLE);
    assert!(enemies[1].position.y > MIDDLE.y && enemies[1].position.x == MIDDLE.x, "the one that was not provoked walked its route");
    for _ in 0..400 {
        patrol_free(&mut enemies[..1], NOBODY, 0.1, 1.0, &mut fx);
    }
    //it got there and stood its ground, but the time it was provoked for ran out long ago, so by now it is back on its route
    assert!(enemies[0].provoked == 0.0);
}

#[test]
fn a_provoked_enemy_stands_where_the_units_are_until_it_is_no_longer_provoked() {
    let mut enemy = kind_at("Grunt", MIDDLE);
    enemy.speed = 100.0;
    enemy.aggro = 0.0;
    enemy.route = vec![MIDDLE + Vec2::new(0.0, 2000.0)];
    enemy.provoke(MIDDLE + Vec2::new(40.0, 0.0));
    let mut enemies = vec![enemy];
    let mut fx = Effects::default();
    for _ in 0..40 {
        enemies[0].provoked = RETALIATE_SECONDS; //(the fight keeps it so)
        patrol_free(&mut enemies, NOBODY, 0.1, 1.0, &mut fx);
    }
    assert!(enemies[0].position.distance(MIDDLE + Vec2::new(40.0, 0.0)) <= CHASE_STOP + 0.5, "it is where they are");
    //the fight is over: it goes back to its route
    enemies[0].provoked = 0.0;
    for _ in 0..20 {
        patrol_free(&mut enemies, NOBODY, 0.1, 1.0, &mut fx);
    }
    assert!(enemies[0].position.y > MIDDLE.y + 50.0);
}

#[test]
fn an_alarm_calls_the_enemies_near_the_one_that_was_hurt_and_only_those() {
    let reach = RETALIATE_ALARM_TILES * TILE_SIZE;
    let near = rooted("Grunt", MIDDLE + Vec2::new(reach - 50.0, 0.0), 100.0);
    let far = rooted("Grunt", MIDDLE + Vec2::new(reach + 50.0, 0.0), 100.0);
    let mut chasing = rooted("Grunt", MIDDLE + Vec2::new(20.0, 0.0), 100.0);
    chasing.chasing = true;
    let mut already = rooted("Grunt", MIDDLE + Vec2::new(0.0, 80.0), 100.0);
    already.provoke(MIDDLE - Vec2::new(500.0, 0.0));
    let healer = rooted("Healer", MIDDLE + Vec2::new(0.0, -80.0), 100.0);
    let nest = rooted("Nest", MIDDLE + Vec2::new(-60.0, 0.0), 100.0);
    let boss = Enemy::boss(&BOSSES[0], 500, 0, MIDDLE + Vec2::new(60.0, 60.0));
    let hidden = ambusher_at(MIDDLE + Vec2::new(60.0, -60.0));
    let mut enemies = vec![near, far, chasing, already, healer, nest, boss, hidden];
    let squad_at = MIDDLE + Vec2::new(10.0, 10.0);
    let mut fx = Effects::default();
    let called = call_for_help(&mut enemies, vec![(MIDDLE, squad_at)], &mut fx);
    assert_eq!(called, 2, "the near one, and the healer (which is only held back from walking)");
    assert_eq!(enemies[0].provoked, RETALIATE_SECONDS);
    assert_eq!(enemies[0].grudge, squad_at);
    assert_eq!(enemies[1].provoked, 0.0, "too far to hear");
    assert_eq!(enemies[2].provoked, 0.0, "it is after the player already");
    assert_eq!(enemies[3].grudge, MIDDLE - Vec2::new(500.0, 0.0), "one that is fighting somewhere else is left to it");
    assert!(enemies[5].provoked == 0.0 && enemies[6].provoked == 0.0 && enemies[7].provoked == 0.0 && enemies[7].lurking());
    assert!(fx.popups.iter().any(|popup| popup.text == "!") && !fx.rings.is_empty() && fx.cues.contains(&Cue::Alert));
}

#[test]
fn only_so_many_alarms_are_answered_in_a_frame() {
    let mut enemies: Vec<Enemy> = (0..200).map(|k| rooted("Grunt", MIDDLE + Vec2::new(3000.0 * (k % 10) as f32, 3000.0 * (k / 10) as f32), 100.0)).collect();
    let alarms: Vec<(Vec2, Vec2)> = enemies.iter().map(|enemy| (enemy.position, enemy.position)).collect();
    let called = call_for_help(&mut enemies, alarms, &mut Effects::default());
    assert_eq!(called, RETALIATE_MAX_ALARMS);
    assert!(fx_rings_are_limited());
}

fn fx_rings_are_limited() -> bool {
    let mut fx = Effects::default();
    let mut enemies: Vec<Enemy> = (0..60).map(|k| rooted("Grunt", MIDDLE + Vec2::new(3000.0 * (k % 10) as f32, 3000.0 * (k / 10) as f32), 100.0)).collect();
    let alarms: Vec<(Vec2, Vec2)> = enemies.iter().map(|enemy| (enemy.position, enemy.position)).collect();
    call_for_help(&mut enemies, alarms, &mut fx);
    fx.rings.len() <= 24 && fx.popups.len() <= 6
}

#[test]
fn a_provoked_mortar_lobs_its_shells_at_the_units_hurting_it() {
    let mut mortar = rooted("Mortar", MIDDLE, 1000.0);
    mortar.timer = 0.0;
    let grudge = MIDDLE + Vec2::new(30.0, 20.0);
    mortar.provoke(grudge);
    //the army is in range elsewhere, which it would otherwise pick from
    let far_unit = unit_on(MIDDLE + Vec2::new(0.0, mortar.range * 0.8));
    let mut enemies = vec![mortar];
    let mut fx = Effects::default();
    enemy_abilities(&mut enemies, &[far_unit], NOBODY, 0.01, &mut fx);
    assert_eq!(fx.hazards.len(), 1);
    assert_eq!(fx.hazards[0].position, grudge);
}

#[test]
fn a_provoked_summoner_calls_up_imps_even_when_the_player_is_far() {
    let mut summoner = rooted("Summoner", MIDDLE, 1000.0);
    summoner.timer = 0.0;
    let mut enemies = vec![summoner.clone()];
    let mut fx = Effects::default();
    enemy_abilities(&mut enemies, &[], MIDDLE + Vec2::new(9000.0, 0.0), 0.01, &mut fx);
    assert_eq!(enemies.len(), 1, "nobody near, nothing to call them for");
    summoner.timer = 0.0;
    summoner.provoke(MIDDLE);
    let mut enemies = vec![summoner];
    enemy_abilities(&mut enemies, &[], MIDDLE + Vec2::new(9000.0, 0.0), 0.01, &mut fx);
    assert!(enemies.len() > 1, "somebody is hurting it");
}

#[test]
fn hitting_one_enemy_of_a_warband_brings_the_rest_down_on_the_squad() {
    //four enemies stand a few tiles apart, patrolling nowhere in particular; a squad of units hits the first
    let mut enemies: Vec<Enemy> = (0..4)
        .map(|k| {
            //(all within the alarm's reach of the first, which is the one that gets hit)
            let mut enemy = kind_at("Brute", MIDDLE + Vec2::new(0.25 * RETALIATE_ALARM_TILES * TILE_SIZE * k as f32, 0.0));
            enemy.aggro = 0.0;
            enemy.speed = 60.0;
            enemy.route = vec![enemy.position + Vec2::new(0.0, 3000.0)];
            enemy.max_health = 1.0e7;
            enemy.health = 1.0e7;
            enemy
        })
        .collect();
    let mut units = units_inside(&enemies[0], 30);
    let mut fx = Effects::default();
    for _ in 0..30 {
        patrol_free(&mut enemies, NOBODY, 0.1, 1.0, &mut fx);
        fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &Upgrades::default(), &mut fx);
        call_for_help(&mut enemies, std::mem::take(&mut fx.alarms), &mut fx);
    }
    assert!(enemies.iter().all(|enemy| enemy.provoked > 0.0), "they all came: {:?}", enemies.iter().map(|enemy| enemy.provoked).collect::<Vec<_>>());
    //they went for the squad (on the first) and not off along their routes
    assert!(enemies.iter().all(|enemy| enemy.position.distance(enemies[0].position) < 330.0 && enemy.position.y < MIDDLE.y + 100.0));
}
