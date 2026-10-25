//step 13: the elemental champions (the Emberlord, the Chronomancer, the Venomancer, the Gravitron and the Frostweaver's ice bolts)

use super::*;

//a champion of a kind, ready to use its power at once, among a few units of its army
fn ready(name: &str, at: Vec2) -> Vec<Unit> {
    let mut champion = champion_unit(champion_named(name).expect("a kind of champion"), at);
    champion.power = 0.0;
    let mut units = vec![champion];
    units.extend((0..50).map(|i| unit_on(at + Vec2::new(-300.0 - (i % 10) as f32 * 20.0, (i / 10) as f32 * 20.0))));
    units
}

//a crowd of enemies that nothing kills by accident in a test, round a place
fn crowd(at: Vec2) -> Vec<Enemy> {
    (0..6)
        .map(|i| {
            let mut enemy = kind_at("Grunt", at + Vec2::from_angle(i as f32) * 90.0);
            enemy.max_health = 1.0e6;
            enemy.health = 1.0e6;
            enemy.attack = 0.0;
            enemy
        })
        .collect()
}

fn act(units: &mut [Unit], enemies: &mut [Enemy], seconds: f32) -> Effects {
    let mut fx = Effects::default();
    for _ in 0..(seconds * 60.0).round() as usize {
        champion_acts(units, enemies, &mut Vec::new(), &Upgrades::default(), 1.0 / 60.0, &mut fx);
    }
    fx
}

#[test]
fn the_elemental_champions_are_drawn_and_can_grow() {
    for name in ["Emberlord", "Chronomancer", "Venomancer", "Gravitron"] {
        let kind = champion_named(name).expect("in the table");
        assert!(champion_shape(kind).is_some(), "{name} has a picture");
        assert!(evolutions_of(kind).is_some(), "{name} can evolve");
        assert!(power_numbers(&CHAMPIONS[kind].power).is_some());
    }
}

#[test]
fn an_emberlord_sets_a_crowd_alight_and_a_venomancer_poisons_it() {
    let at = MIDDLE;
    let target = MIDDLE + Vec2::new(500.0, 0.0);
    let mut units = ready("Emberlord", at);
    let mut enemies = crowd(target);
    act(&mut units, &mut enemies, 0.05);
    assert!(enemies.iter().filter(|enemy| enemy.status.burning > 0.0).count() >= 4, "most of the crowd burns");
    let mut units = ready("Venomancer", at);
    let mut enemies = crowd(target);
    act(&mut units, &mut enemies, 0.05);
    assert!(enemies.iter().filter(|enemy| enemy.status.poison > 0.0).count() >= 4, "most of the crowd is poisoned");
}

#[test]
fn a_gravitron_drags_a_crowd_together_but_not_a_boss() {
    let target = MIDDLE + Vec2::new(500.0, 0.0);
    let mut units = ready("Gravitron", MIDDLE);
    let mut enemies = crowd(target);
    enemies.push(Enemy::boss(&BOSSES[0], 200, 0, target + Vec2::new(0.0, 250.0)));
    //(how far the six are from their own middle: they are dragged to the one with the most round it)
    let spread = |enemies: &[Enemy]| {
        let middle = enemies[..6].iter().map(|enemy| enemy.position).sum::<Vec2>() / 6.0;
        enemies[..6].iter().map(|enemy| enemy.position.distance(middle)).sum::<f32>()
    };
    let (before, boss_at) = (spread(&enemies), enemies[6].position);
    act(&mut units, &mut enemies, 0.05);
    assert!(spread(&enemies) < before * 0.7, "{} from {before}", spread(&enemies));
    assert_eq!(enemies[6].position, boss_at, "a boss is too heavy");
}

#[test]
fn a_chronomancer_slows_what_is_near_it_and_a_boss_less() {
    let mut units = ready("Chronomancer", MIDDLE);
    let mut enemies = crowd(MIDDLE + Vec2::new(200.0, 0.0));
    enemies.push(Enemy::boss(&BOSSES[0], 200, 0, MIDDLE + Vec2::new(0.0, 250.0)));
    enemies.push(kind_at("Grunt", MIDDLE + Vec2::new(5000.0, 0.0)));
    act(&mut units, &mut enemies, 0.05);
    assert!(enemies[0].status.slow <= 0.35 + 1e-4);
    assert!(enemies[6].status.slow >= TIME_BOSS_SLOW - 1e-4 && enemies[6].status.slow < 1.0);
    assert_eq!(enemies[7].status.slow, 1.0, "out of its reach");
    assert!(CHAMPIONS[champion_named("Chronomancer").unwrap()].power.aura().is_some(), "its aura is drawn");
}

#[test]
fn a_frostweavers_pulse_throws_ice_bolts_that_chill_what_they_hit() {
    let mut units = ready("Frostweaver", MIDDLE);
    let mut enemies = crowd(MIDDLE + Vec2::new(5000.0, 0.0));
    let fx = act(&mut units, &mut enemies, 0.05);
    let (volley, hurt) = fx.volleys.first().expect("ice bolts thrown");
    assert!(volley.count >= ICE_BOLTS && hurt.each > 0.0 && is_ice(volley.champion));
    //one of them flying into an enemy chills it
    let mut target = vec![crowd(MIDDLE + Vec2::new(120.0, 0.0)).remove(0)];
    let mut bolts = Vec::new();
    launch_volley(&mut bolts, &Volley { aim: target[0].position - MIDDLE, shape: VolleyShape::Burst, count: 1, ..*volley }, *hurt);
    let mut fx = Effects::default();
    for _ in 0..60 {
        fly_bolts(&mut bolts, &mut target, 1.0 / 60.0, Some(&mut fx));
    }
    assert!(target[0].status.chill > 0.0 || target[0].status.frozen > 0.0);
    assert!(!is_ice(units[0].champion + 1), "only a frostweaver's");
}

//enemies in a ring, tough enough to see what a reaction does
fn ring_of(count: usize) -> Vec<Enemy> {
    (0..count)
        .map(|i| {
            let mut enemy = kind_at("Grunt", MIDDLE + Vec2::from_angle(i as f32) * 70.0);
            enemy.max_health = 1000.0;
            enemy.health = 1000.0;
            enemy
        })
        .collect()
}

#[test]
fn burning_and_frozen_make_steam_that_scalds_and_blinds_the_ones_round_it_once() {
    let mut enemies = ring_of(5);
    freeze(&mut enemies[0], 2.0);
    ignite(&mut enemies[0], 3.0, 10.0);
    let mut fx = Effects::default();
    assert_eq!(react_statuses(&mut enemies, &mut fx), 1);
    assert!(enemies.iter().all(|enemy| enemy.health < 1000.0), "all of them are scalded");
    assert!(enemies[0].status.burning == 0.0 && enemies[0].status.frozen == 0.0, "it used up what made it");
    assert!(matches!(enemies[1].mode, Mode::Cast { .. }), "and blinds them");
    assert_eq!(react_statuses(&mut enemies, &mut fx), 0, "once");
}

#[test]
fn shocked_and_burning_overload_and_poisoned_and_burning_spread_plague_fire_that_dies_out() {
    let mut enemies = ring_of(5);
    shock(&mut enemies[0], 2.0);
    ignite(&mut enemies[0], 3.0, 10.0);
    let mut fx = Effects::default();
    assert_eq!(react_statuses(&mut enemies, &mut fx), 1);
    assert!(enemies[0].status.shocked == 0.0 && enemies.iter().all(|enemy| enemy.health < 1000.0));
    let mut enemies = ring_of(5);
    poison(&mut enemies[0], 8.0);
    ignite(&mut enemies[0], 3.0, 10.0);
    assert_eq!(react_statuses(&mut enemies, &mut fx), 1);
    assert!(enemies[1].status.poison > 0.0 && enemies[1].status.burning > 0.0, "the poison and the fire spread");
    //and the plague goes round a few times, a weaker one each time, and stops
    let mut total = 1;
    for _ in 0..120 {
        tick_statuses(&mut enemies, 1.0 / 60.0);
        total += react_statuses(&mut enemies, &mut fx);
    }
    assert!(total < 40, "{total} went off");
    assert_eq!(react_statuses(&mut enemies, &mut fx), 0);
}

#[test]
fn q_sets_off_the_ultimates_that_are_ready_and_they_wait_their_cooldown() {
    let names = ["Frostweaver", "Pyromancer", "Stormcaller", "Earthshaker", "Medic", "Juggernaut"];
    let mut units: Vec<Unit> = names.iter().enumerate().map(|(i, name)| champion_unit(champion_named(name).unwrap(), MIDDLE + Vec2::new(i as f32 * 40.0, 0.0))).collect();
    units.extend((0..40).map(|i| {
        let mut unit = unit_on(MIDDLE + Vec2::new((i % 10) as f32 * 20.0, 60.0 + (i / 10) as f32 * 20.0));
        unit.health = 10.0;
        unit
    }));
    assert_eq!(units.iter().filter(|unit| ultimate_ready(unit)).count(), 5, "a juggernaut has none");
    let mut enemies = crowd(MIDDLE + Vec2::new(0.0, -600.0));
    enemies.push(Enemy::boss(&BOSSES[0], 200, 0, MIDDLE + Vec2::new(600.0, -600.0)));
    let mut fx = Effects::default();
    let fired = fire_ultimates(&mut units, &mut enemies, 0, Vec2::NEG_Y, 200, &Upgrades::default(), &mut fx);
    assert_eq!(fired, 5);
    assert!(enemies[..6].iter().all(|enemy| enemy.status.frozen > 0.0), "absolute zero freezes them");
    assert_eq!(enemies[6].status.frozen, 0.0, "a boss is not frozen");
    assert!(fx.volleys.len() >= 6, "a storm of meteors");
    assert!(enemies[..6].iter().all(|enemy| enemy.health < enemy.max_health), "the tempest and the fault line hurt them");
    assert!(units[6..].iter().all(|unit| unit.health == unit.max_health()), "rapture mends the army");
    assert_eq!(fire_ultimates(&mut units, &mut enemies, 0, Vec2::NEG_Y, 200, &Upgrades::default(), &mut fx), 0, "and they wait");
    assert!(units[0].ultimate > 0.0 && CONTROLS.iter().any(|(key, _)| *key == "Q"));
    //(the key goes to the host with the rest of what the player did)
    let input = PlayerInput { ultimate: true, ..PlayerInput::idle(Formation::Ring, Vec2::new(1280.0, 720.0)) };
    let mut out = Writer::new();
    input.write(&mut out);
    assert!(PlayerInput::read(&mut Reader::new(&out.bytes)).expect("it reads").ultimate);
}
