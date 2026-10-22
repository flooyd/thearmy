//new kinds of enemy, elites, groups and hazards

use super::*;


// ---------- new kinds of enemy, elites, groups and hazards ----------

pub(super) const MIDDLE: Vec2 = Vec2::new(16000.0, 16000.0);

//an enemy of the kind with this name standing at a spot, for an army too small for scaling
pub(super) fn kind_at(name: &str, at: Vec2) -> Enemy {
    let mut enemy = Enemy::new(at, kind_named(name), 1.0);
    enemy.position = at;
    enemy
}

pub(super) fn hazard_at(at: Vec2, radius: f32, warn: f32, damage: f32, victims: usize) -> Hazard {
    Hazard::new(at, radius, warn, damage, victims, RED, Cue::Explosion)
}

#[test]
fn the_kind_table_is_sane() {
    let mut names = HashSet::new();
    for kind in ENEMY_KINDS.iter() {
        assert!(names.insert(kind.name), "{} is in the table twice", kind.name);
        assert!(kind.size > 0.0 && kind.health > 0.0 && kind.attack > 0.0 && kind.weight >= 0.0 && kind.shield >= 0.0, "{}", kind.name);
        assert!((kind.min_speed > 0.0 || kind.behavior == Behavior::Nest) && kind.min_speed <= kind.max_speed, "{}", kind.name);
        assert_eq!(kind.range > 0.0, matches!(kind.behavior, Behavior::Shooter | Behavior::Mortar), "{}", kind.name);
        assert!(std::ptr::eq(kind_named(kind.name), kind));
        if kind.pack > 0 {
            assert_eq!(kind.behavior, Behavior::Walker, "{}", kind.name);
        }
        if kind.behavior == Behavior::Charger {
            assert!(kind.aggro_tiles > 0.0, "a charger charges when it notices");
        }
    }
    //what the others make or travel with is never found by itself
    for name in ["Healer", "Imp", "Splitling", "Drone", "Nest"] {
        assert_eq!(kind_named(name).weight, 0.0, "{name}");
    }
    for behavior in [Behavior::Walker, Behavior::Shooter, Behavior::Splitter, Behavior::Charger, Behavior::Bomber, Behavior::Healer, Behavior::Summoner, Behavior::Mortar] {
        assert!(ENEMY_KINDS.iter().any(|kind| kind.behavior == behavior), "no kind does {behavior:?}");
    }
    //the first six are as they always were, in the same order
    assert_eq!(ENEMY_KINDS.iter().take(6).map(|kind| kind.name).collect::<Vec<_>>(), ["Grunt", "Scout", "Brute", "Titan", "Colossus", "Spitter"]);
}

#[test]
fn armor_shield_and_a_boss_between_phases_decide_how_much_damage_gets_through() {
    let mut enemy = kind_at("Grunt", MIDDLE);
    enemy.max_health = 1000.0;
    enemy.health = 1000.0;
    assert_eq!(enemy.take_damage(100.0), 100.0);
    assert_eq!(enemy.health, 900.0);
    enemy.armor = 0.5;
    assert_eq!(enemy.take_damage(100.0), 50.0);
    assert_eq!(enemy.health, 850.0);
    enemy.armor = 0.0;
    enemy.max_shield = 200.0;
    enemy.shield = 200.0;
    //the shield soaks it up first
    assert_eq!(enemy.take_damage(150.0), 150.0);
    assert_eq!((enemy.shield, enemy.health), (50.0, 850.0));
    //and what it can't takes health
    assert_eq!(enemy.take_damage(80.0), 80.0);
    assert_eq!((enemy.shield, enemy.health), (0.0, 820.0));
    //being touched by nothing doesn't count as being hurt
    enemy.shield_idle = 3.0;
    enemy.take_damage(0.0);
    assert_eq!(enemy.shield_idle, 3.0);
    enemy.take_damage(1.0);
    assert_eq!(enemy.shield_idle, 0.0);

    let mut boss = Enemy::boss(&BOSSES[0], 100, 0, MIDDLE);
    boss.boss.as_mut().unwrap().shifting = 1.0;
    let health = boss.health;
    assert_eq!(boss.take_damage(1.0e6), 0.0);
    assert_eq!(boss.health, health);
}

#[test]
fn fighting_an_enemy_with_a_shield_and_armor_takes_longer_and_the_squad_sent_is_bigger() {
    let plain = still_enemy(MIDDLE, 1000.0);
    let mut shielded = still_enemy(MIDDLE, 1000.0);
    shielded.max_shield = 1000.0;
    shielded.shield = 1000.0;
    let mut armored = still_enemy(MIDDLE, 1000.0);
    armored.armor = 0.5;
    assert_eq!(squad_size(&shielded), squad_size(&plain) * 2);
    assert_eq!(squad_size(&armored), squad_size(&plain) * 2);

    //10 units for a second do 1000 damage: the plain one is gone, the shielded one has lost its shield and the armored half its health
    for (enemy, shield, health) in [(plain, 0.0, 0.0), (shielded, 0.0, 1000.0), (armored, 0.0, 500.0)] {
        let mut units = units_inside(&enemy, 10);
        let mut enemies = vec![enemy];
        for _ in 0..60 {
            fight(&mut enemies, &mut units, &mut Vec::new(), 1.0 / 60.0, &Upgrades::default(), &mut Effects::default());
        }
        let left = enemies.first().map(|enemy| (enemy.shield, enemy.health)).unwrap_or((0.0, 0.0));
        assert!((left.0 - shield).abs() < 1.0 && (left.1 - health).abs() < 1.0, "{left:?} against {shield} and {health}");
    }
}

#[test]
fn a_shield_grows_back_only_after_it_has_been_left_alone() {
    let mut enemies = vec![kind_at("Bulwark", MIDDLE)];
    let max = enemies[0].max_shield;
    assert!(max > enemies[0].max_health, "its shield is bigger than its health");
    enemies[0].shield = 0.0;
    enemies[0].take_damage(1.0);
    let mut fx = Effects::default();
    for _ in 0..((SHIELD_REGEN_DELAY - 0.5) * 10.0) as usize {
        enemy_abilities(&mut enemies, &[], NOBODY, 0.1, &mut fx);
    }
    assert_eq!(enemies[0].shield, 0.0);
    for _ in 0..((SHIELD_REGEN_DELAY + 1.0 / SHIELD_REGEN_RATE + 0.5) * 10.0) as usize {
        enemy_abilities(&mut enemies, &[], NOBODY, 0.1, &mut fx);
    }
    assert_eq!(enemies[0].shield, max);
    //hit again, it stops growing and starts over
    enemies[0].take_damage(max / 2.0);
    for _ in 0..30 {
        enemy_abilities(&mut enemies, &[], NOBODY, 0.1, &mut fx);
    }
    assert!(enemies[0].shield < max / 2.0 + 1.0);
}

#[test]
fn an_elite_is_bigger_tougher_and_worth_more_and_each_modifier_does_its_own_thing() {
    for modifier in MODIFIERS {
        let mut elite = kind_at("Brute", MIDDLE);
        let (size, health, babies, speed, attack) = (elite.size, elite.max_health, elite.babies, elite.speed, elite.attack);
        elite.make_elite(modifier);
        assert_eq!(elite.modifier, modifier);
        assert!(elite.size > size && elite.max_health > health && elite.babies > babies, "{modifier:?}");
        assert_eq!(elite.health, elite.max_health);
        match modifier {
            Modifier::Swift => assert!(elite.speed > speed * 1.5),
            Modifier::Armored => assert!(elite.armor >= 0.5),
            Modifier::Frenzied => assert!(elite.attack > attack * 1.9 && elite.speed > speed),
            Modifier::Volatile | Modifier::None => {}
        }
        assert!(elite.title().starts_with(modifier.label()) && elite.title().ends_with("Brute"));
        assert_ne!(modifier.color(), Modifier::None.color());
    }
    assert_eq!(kind_at("Brute", MIDDLE).title(), "Brute");
}

#[test]
fn elites_turn_up_once_the_army_is_big_enough_and_more_often_as_it_grows() {
    assert_eq!(elite_chance(0), 0.0);
    assert_eq!(elite_chance(ELITE_FROM_UNITS - 1), 0.0);
    let mut last = 0.0;
    for army in (ELITE_FROM_UNITS..3 * ELITE_FULL_UNITS).step_by(13) {
        let chance = elite_chance(army);
        assert!(chance >= last && chance <= ELITE_MAX_CHANCE + 1e-6, "{army}: {chance}");
        last = chance;
    }
    assert!((elite_chance(ELITE_FULL_UNITS) - ELITE_MAX_CHANCE).abs() < 1e-6);
    assert!((elite_chance(MAX_UNITS) - ELITE_MAX_CHANCE).abs() < 1e-6);
    assert_eq!((0..500).filter(|_| spawn_enemy(MIDDLE.x, MIDDLE.y, 10).modifier != Modifier::None).count(), 0);
    let mut seen = HashSet::new();
    let elites = (0..2000)
        .filter(|_| {
            let enemy = spawn_enemy(MIDDLE.x, MIDDLE.y, ELITE_FULL_UNITS);
            seen.insert(enemy.modifier as usize);
            enemy.modifier != Modifier::None
        })
        .count();
    assert!((450..750).contains(&elites), "{elites} elites in 2000, expected about {}", 2000.0 * ELITE_MAX_CHANCE);
    assert_eq!(seen.len(), MODIFIERS.len() + 1, "every modifier turns up");
}

#[test]
fn a_splitter_breaks_into_three_smaller_ones_where_it_was() {
    let mut splitter = kind_at("Splitter", MIDDLE);
    splitter.health = 0.0;
    let mut enemies = vec![splitter];
    let mut babies = Vec::new();
    let destroyed = resolve_deaths(&mut enemies, &mut babies, 100, &Upgrades::default(), &mut Effects::default());
    assert_eq!(destroyed, 1);
    assert_eq!(enemies.len(), 3);
    for child in &enemies {
        assert_eq!(child.name, "Splitling");
        assert!(child.owner.is_none() && child.health > 0.0 && child.max_health > 0.0);
        assert!(child.position.distance(MIDDLE) < 100.0);
    }
    assert!(!babies.is_empty(), "the splitter itself drops babies");
    //the ones that remain are enemies like any other, they are destroyed and counted once each
    for child in enemies.iter_mut() {
        child.health = 0.0;
    }
    assert_eq!(resolve_deaths(&mut enemies, &mut babies, 100, &Upgrades::default(), &mut Effects::default()), 3);
    assert!(enemies.is_empty());
}

#[test]
fn a_volatile_elite_sets_off_a_hazard_where_it_was_destroyed_and_a_plain_one_does_not() {
    let mut volatile = kind_at("Grunt", MIDDLE);
    volatile.make_elite(Modifier::Volatile);
    volatile.health = 0.0;
    let mut fx = Effects::default();
    resolve_deaths(&mut vec![volatile], &mut Vec::new(), 100, &Upgrades::default(), &mut fx);
    assert_eq!(fx.hazards.len(), 1);
    let blast = &fx.hazards[0];
    assert!(blast.armed && blast.warn > 0.0 && blast.warn < 1.0 && blast.damage > 0.0 && blast.position.distance(MIDDLE) < 1.0);

    let mut plain = kind_at("Grunt", MIDDLE);
    plain.health = 0.0;
    let mut fx = Effects::default();
    resolve_deaths(&mut vec![plain], &mut Vec::new(), 100, &Upgrades::default(), &mut fx);
    assert!(fx.hazards.is_empty());
}

#[test]
fn minions_are_worth_nothing_and_die_with_what_made_them() {
    let owner = kind_at("Summoner", MIDDLE);
    let id = owner.id;
    let mut enemies = vec![owner];
    for _ in 0..5 {
        enemies.push(Enemy::minion(MIDDLE + Vec2::new(50.0, 0.0), kind_named("Imp"), 4.0, id));
    }
    assert!(enemies[1..].iter().all(|minion| minion.is_minion() && minion.babies == 0 && !is_wild(minion) && minion.owner == Some(id)));
    assert!(is_wild(&enemies[0]));
    //one of them dies: no kill and nothing dropped
    enemies[1].health = 0.0;
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    assert_eq!(resolve_deaths(&mut enemies, &mut babies, 100, &Upgrades::default(), &mut fx), 0);
    assert!(babies.is_empty());
    assert_eq!(enemies.len(), 5);
    //the owner dies: the rest go with it, and only the owner is a kill
    enemies[0].health = 0.0;
    assert_eq!(resolve_deaths(&mut enemies, &mut babies, 100, &Upgrades::default(), &mut fx), 1);
    assert!(enemies.is_empty());
    assert!(!babies.is_empty(), "the owner drops its own babies");
    //what an enemy makes is smaller than its own kind would be
    let imp = Enemy::minion(MIDDLE, kind_named("Imp"), 100.0, id);
    assert!(imp.size < kind_named("Imp").size * 100.0 / 2.0);
    assert!((minion_scale(100.0) - 10.0).abs() < 1e-3 && minion_scale(0.5) == 1.0);
}

#[test]
fn only_wild_enemies_count_towards_the_number_kept_alive() {
    let mut enemies = Vec::new();
    for _ in 0..30 {
        enemies.push(Enemy::minion(MIDDLE, kind_named("Imp"), 1.0, 99_999));
    }
    enemies.push(Enemy::boss(&BOSSES[0], 100, 0, MIDDLE));
    respawn_enemies(&mut enemies, MIDDLE.x, MIDDLE.y, 2);
    let wild = enemies.iter().filter(|enemy| is_wild(enemy)).count();
    assert!(wild >= ENEMY_COUNT && wild < ENEMY_COUNT + 30, "{wild}");
    assert_eq!(enemies.len(), wild + 31);
}

#[test]
fn a_bomber_that_touches_the_army_lights_its_fuse_and_blows_up_taking_some_of_it_along() {
    let mut bomber = kind_at("Bomber", MIDDLE);
    bomber.health = 1.0e6;
    bomber.max_health = 1.0e6;
    let mut enemies = vec![bomber];
    let mut units: Vec<Unit> = (0..100).map(|i| unit_on(MIDDLE + Vec2::new((i % 10) as f32 * 4.0 - 20.0, (i / 10) as f32 * 4.0 - 20.0))).collect();
    let mut fx = Effects::default();
    //nobody touching it: no fuse
    fight(&mut enemies, &mut [unit_on(MIDDLE + Vec2::new(900.0, 0.0))], &mut Vec::new(), 0.1, &Upgrades::default(), &mut fx);
    assert_eq!(enemies[0].mode, Mode::Roam);
    fight(&mut enemies, &mut units, &mut Vec::new(), 1.0 / 60.0, &Upgrades::default(), &mut fx);
    assert!(matches!(enemies[0].mode, Mode::Fuse { .. }));
    //it stands still while the fuse burns, and goes off when it is spent
    let at = enemies[0].position;
    let mut seconds = 0.0;
    while enemies[0].health > 0.0 && seconds < 2.0 {
        patrol_free(&mut enemies, MIDDLE + Vec2::new(300.0, 0.0), 1.0 / 60.0, 1.0, &mut fx);
        enemy_abilities(&mut enemies, &units, NOBODY, 1.0 / 60.0, &mut fx);
        seconds += 1.0 / 60.0;
    }
    assert!((seconds - BOMB_FUSE).abs() < 0.1, "{seconds} s");
    assert_eq!(enemies[0].position, at);
    assert!(enemies[0].blew_up);
    assert_eq!(fx.hazards.len(), 1);
    let mut hazards: Vec<Hazard> = fx.hazards.drain(..).collect();
    update_hazards(&mut hazards, &mut units, &Upgrades::default(), 1.0 / 60.0, &mut fx);
    assert_eq!(units.iter().filter(|unit| unit.health < UNIT_HEALTH).count(), victims_of(100, 0.02));
    //it isn't a kill and drops nothing
    let mut babies = Vec::new();
    assert_eq!(resolve_deaths(&mut enemies, &mut babies, units.len(), &Upgrades::default(), &mut fx), 0);
    assert!(babies.is_empty() && enemies.is_empty());
}

#[test]
fn a_bomber_that_is_destroyed_first_never_goes_off() {
    let mut bomber = kind_at("Bomber", MIDDLE);
    bomber.health = 0.0;
    let mut fx = Effects::default();
    let mut babies = Vec::new();
    assert_eq!(resolve_deaths(&mut vec![bomber], &mut babies, 100, &Upgrades::default(), &mut fx), 1);
    assert!(fx.hazards.is_empty());
    assert!(!babies.is_empty());
}

#[test]
fn a_healer_mends_the_hurt_enemies_near_it_but_not_bosses_or_ones_far_away() {
    let mut healer = kind_at("Healer", MIDDLE);
    healer.timer = 0.0;
    let mut near = kind_at("Brute", MIDDLE + Vec2::new(200.0, 0.0));
    near.health = near.max_health * 0.5;
    let mut far = kind_at("Brute", MIDDLE + Vec2::new(HEAL_RANGE + 100.0, 0.0));
    far.health = far.max_health * 0.5;
    let far_health = far.health;
    let mut boss = Enemy::boss(&BOSSES[1], 200, 0, MIDDLE + Vec2::new(100.0, 100.0));
    boss.health *= 0.5;
    let boss_health = boss.health;
    let mut enemies = vec![healer, near, far, boss];
    let mut fx = Effects::default();
    enemy_abilities(&mut enemies, &[], NOBODY, 0.01, &mut fx);
    let max = enemies[1].max_health;
    assert!((enemies[1].health - max * (0.5 + HEAL_SHARE)).abs() < 0.01);
    assert_eq!(enemies[2].health, far_health);
    assert_eq!(enemies[3].health, boss_health);
    assert!(fx.cues.contains(&Cue::Heal));
    //not again until the interval is up
    enemy_abilities(&mut enemies, &[], NOBODY, 0.5, &mut fx);
    assert!((enemies[1].health - max * (0.5 + HEAL_SHARE)).abs() < 0.01);
    enemy_abilities(&mut enemies, &[], NOBODY, 0.6, &mut fx);
    assert!((enemies[1].health - max * (0.5 + 2.0 * HEAL_SHARE)).abs() < 0.01);
    //and never past full health
    for _ in 0..50 {
        enemy_abilities(&mut enemies, &[], NOBODY, 0.5, &mut fx);
    }
    assert_eq!(enemies[1].health, max);
}

#[test]
fn a_healer_goes_to_the_other_enemies_and_stays_close() {
    let mut healer = kind_at("Healer", MIDDLE);
    healer.speed = 100.0;
    let mut other = kind_at("Brute", MIDDLE + Vec2::new(1000.0, 0.0));
    other.speed = 0.0;
    let mut enemies = vec![healer, other];
    for _ in 0..1000 {
        patrol_free(&mut enemies, NOBODY, 0.05, 1.0, &mut Effects::default());
    }
    let gap = enemies[0].position.distance(enemies[1].position);
    assert!((gap - HEALER_STAY).abs() < 1.0, "{gap}");
    //with nobody else about it patrols its own route like anything
    let mut alone = vec![kind_at("Healer", MIDDLE)];
    alone[0].speed = 100.0;
    let start = alone[0].position;
    patrol_free(&mut alone, NOBODY, 0.5, 1.0, &mut Effects::default());
    assert!(alone[0].position.distance(start) > 1.0);
}

#[test]
fn a_summoner_calls_imps_while_the_player_is_near_up_to_a_limit() {
    let mut summoner = kind_at("Summoner", MIDDLE);
    summoner.timer = 0.0;
    let id = summoner.id;
    let mut enemies = vec![summoner];
    let mut fx = Effects::default();
    for _ in 0..100 {
        enemy_abilities(&mut enemies, &[], NOBODY, 0.1, &mut fx);
    }
    assert_eq!(enemies.len(), 1, "nobody is near to be scared");
    let near = MIDDLE + Vec2::new(500.0, 0.0);
    for _ in 0..10 {
        enemy_abilities(&mut enemies, &[], near, 0.1, &mut fx);
    }
    assert_eq!(enemies.len(), 1 + SUMMON_COUNT);
    assert!(enemies[1..].iter().all(|minion| minion.owner == Some(id) && minion.name == "Imp" && minion.babies == 0));
    assert!(fx.cues.contains(&Cue::Summon));
    for _ in 0..600 {
        enemy_abilities(&mut enemies, &[], near, 0.1, &mut fx);
    }
    assert_eq!(enemies.len(), 1 + MAX_MINIONS);
    //a minion that dies is called up again
    enemies.remove(1);
    for _ in 0..100 {
        enemy_abilities(&mut enemies, &[], near, 0.1, &mut fx);
    }
    assert_eq!(enemies.len(), 1 + MAX_MINIONS);
}

#[test]
fn a_mortar_lobs_a_shell_at_a_unit_in_range_that_takes_a_moment_to_land() {
    let mut mortar = kind_at("Mortar", MIDDLE);
    mortar.timer = 0.0;
    let range = mortar.range;
    let mut enemies = vec![mortar];
    let mut fx = Effects::default();
    let far = vec![unit_on(MIDDLE + Vec2::new(range + 200.0, 0.0))];
    for _ in 0..30 {
        enemy_abilities(&mut enemies, &far, NOBODY, 0.1, &mut fx);
    }
    assert!(fx.hazards.is_empty(), "nothing in range to lob at");
    let target = MIDDLE + Vec2::new(range - 100.0, 0.0);
    let near = vec![unit_on(target)];
    for _ in 0..10 {
        enemy_abilities(&mut enemies, &near, NOBODY, 0.1, &mut fx);
    }
    assert_eq!(fx.hazards.len(), 1);
    let shell = &fx.hazards[0];
    assert!(shell.position.distance(target) < 1.0);
    assert_eq!(shell.warn, SHELL_WARN);
    assert!(shell.radius >= BLAST_RADIUS && shell.damage > 0.0);
    //a mortar doesn't fire volleys like a spitter
    let mut shots = Vec::new();
    shoot_enemies(&mut enemies, &near, &mut shots, 10.0, &mut fx);
    assert!(shots.is_empty());
}

#[test]
fn a_charger_stands_and_shows_where_it_will_dash_then_dashes_hard_and_is_winded() {
    let mut charger = kind_at("Charger", MIDDLE);
    charger.speed = 0.0;
    let aggro = charger.aggro;
    let attack = charger.attack;
    let mut enemies = vec![charger];
    let mut fx = Effects::default();
    patrol_free(&mut enemies, MIDDLE + Vec2::new(aggro + 50.0, 0.0), 0.1, 1.0, &mut fx);
    assert_eq!(enemies[0].mode, Mode::Roam);
    let player = MIDDLE + Vec2::new(aggro - 100.0, 0.0);
    patrol_free(&mut enemies, player, 0.01, 1.0, &mut fx);
    let Mode::Windup { dir, .. } = enemies[0].mode else {
        panic!("{:?}", enemies[0].mode);
    };
    assert!((dir - Vec2::X).length() < 1e-3);
    assert!(!enemies[0].chasing);
    assert!(fx.cues.contains(&Cue::Alert));
    //it stands still while it winds up
    let at = enemies[0].position;
    let mut seconds = 0.0;
    while matches!(enemies[0].mode, Mode::Windup { .. }) {
        patrol_free(&mut enemies, player, 0.01, 1.0, &mut fx);
        seconds += 0.01;
        assert!(seconds < 2.0);
    }
    assert!((seconds - CHARGE_WINDUP).abs() < 0.03, "{seconds}");
    assert_eq!(enemies[0].position, at);
    assert!(fx.cues.contains(&Cue::Charge));
    //the dash goes the way it showed, the whole way, and hits harder
    assert!(matches!(enemies[0].mode, Mode::Dash { .. }));
    assert_eq!(enemies[0].attack_now(), attack * CHARGE_DAMAGE);
    let mut dashed = 0.0;
    while matches!(enemies[0].mode, Mode::Dash { .. }) {
        patrol_free(&mut enemies, player, 0.01, 1.0, &mut fx);
        dashed += 0.01;
    }
    let distance = enemies[0].position.distance(at);
    assert!((distance - CHARGE_SPEED * CHARGE_TIME).abs() < CHARGE_SPEED * 0.02, "{distance}");
    assert!((dashed - CHARGE_TIME).abs() < 0.03);
    assert!((enemies[0].position - at).normalize().dot(Vec2::X) > 0.999);
    //then it is winded and doesn't dash again until it has recovered
    assert!(matches!(enemies[0].mode, Mode::Recover { .. }));
    assert_eq!(enemies[0].attack_now(), attack);
    let mut recovering = 0.0;
    while matches!(enemies[0].mode, Mode::Recover { .. }) {
        patrol_free(&mut enemies, player, 0.01, 1.0, &mut fx);
        recovering += 0.01;
    }
    assert!((recovering - CHARGE_RECOVER).abs() < 0.03);
}

#[test]
fn a_dash_never_leaves_the_world() {
    let mut charger = kind_at("Charger", Vec2::new(WORLD_SIZE - 100.0, 16000.0));
    charger.mode = Mode::Dash { dir: Vec2::X, left: 5.0 };
    let mut enemies = vec![charger];
    for _ in 0..100 {
        patrol_free(&mut enemies, NOBODY, 0.05, 3.0, &mut Effects::default());
    }
    assert!(enemies[0].position.x <= WORLD_SIZE - enemies[0].size / 2.0 + 0.01);
}

#[test]
fn drones_shoot_and_chase_and_only_shooters_fire() {
    let drone = kind_at("Drone", MIDDLE);
    assert_eq!(drone.behavior, Behavior::Shooter);
    assert!(drone.range > 0.0 && drone.volley >= 1 && drone.babies == 0);
    let mut enemies = vec![drone, kind_at("Grunt", MIDDLE)];
    enemies[0].shot_in = 0.0;
    enemies[1].shot_in = 0.0;
    let units = vec![unit_on(MIDDLE + Vec2::new(300.0, 0.0))];
    let mut shots = Vec::new();
    shoot_enemies(&mut enemies, &units, &mut shots, 0.01, &mut Effects::default());
    assert_eq!(shots.len(), enemies[0].volley);
}

#[test]
fn a_hazard_warns_then_goes_off_once_hurting_the_units_in_it_and_no_others() {
    let mut units = vec![unit_on(MIDDLE), unit_on(MIDDLE + Vec2::new(90.0, 0.0)), unit_on(MIDDLE + Vec2::new(150.0, 0.0))];
    let mut hazards = vec![hazard_at(MIDDLE, 100.0, 1.0, 60.0, 10)];
    let mut fx = Effects::default();
    for _ in 0..9 {
        update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.1, &mut fx);
    }
    assert!(units.iter().all(|unit| unit.health == UNIT_HEALTH) && fx.cues.is_empty());
    assert!(hazards[0].armed);
    update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.15, &mut fx);
    assert_eq!(units[0].health, UNIT_HEALTH - 60.0);
    assert_eq!(units[1].health, UNIT_HEALTH - 60.0);
    assert_eq!(units[2].health, UNIT_HEALTH, "outside it");
    assert_eq!(fx.cues, vec![Cue::Explosion]);
    assert!(fx.shake > 0.0);
    //once only, then it's gone
    for _ in 0..40 {
        update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.02, &mut fx);
    }
    assert_eq!(units[0].health, UNIT_HEALTH - 60.0);
    assert_eq!(fx.cues.len(), 1);
    assert!(hazards.is_empty());
}

#[test]
fn a_hazard_with_no_warning_goes_off_in_the_first_update() {
    let mut units = vec![unit_on(MIDDLE)];
    let mut hazards = vec![hazard_at(MIDDLE, 50.0, 0.0, 30.0, 5)];
    update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.0, &mut Effects::default());
    assert_eq!(units[0].health, UNIT_HEALTH - 30.0);
}

#[test]
fn a_hazard_hurts_no_more_than_its_cap_and_iron_resolve_softens_it() {
    let mut units: Vec<Unit> = (0..50).map(|_| unit_on(MIDDLE)).collect();
    let mut hazards = vec![hazard_at(MIDDLE, 100.0, 0.0, UNIT_HEALTH, 7)];
    let mut fx = Effects::default();
    update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.01, &mut fx);
    assert_eq!(units.iter().filter(|unit| unit.health <= 0.0).count(), 7);
    assert!(fx.popups.iter().any(|popup| popup.text == "-7 units"));

    let mut tough = Upgrades::default();
    tough.take(Relic::IronResolve);
    let mut units: Vec<Unit> = (0..50).map(|_| unit_on(MIDDLE)).collect();
    let mut hazards = vec![hazard_at(MIDDLE, 100.0, 0.0, UNIT_HEALTH, 7)];
    update_hazards(&mut hazards, &mut units, &tough, 0.01, &mut Effects::default());
    assert_eq!(units.iter().filter(|unit| unit.health < UNIT_HEALTH).count(), 7);
    assert!(units.iter().all(|unit| unit.health > 0.0), "20% less of a lethal hit leaves them alive");
    assert!((units[0].health - UNIT_HEALTH * 0.2).abs() < 1e-3);
}

#[test]
fn a_burning_hazard_hurts_the_units_standing_in_it_for_as_long_as_it_burns() {
    let mut units = vec![unit_on(MIDDLE), unit_on(MIDDLE + Vec2::new(500.0, 0.0))];
    let mut hazards = vec![hazard_at(MIDDLE, 100.0, 0.0, 0.0, 0).burning(2.0, 25.0)];
    let mut fx = Effects::default();
    for _ in 0..150 {
        update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.01, &mut fx);
    }
    assert!((units[0].health - (UNIT_HEALTH - 25.0 * 1.5)).abs() < 1.0, "{}", units[0].health);
    assert_eq!(units[1].health, UNIT_HEALTH);
    assert!(!hazards.is_empty());
    for _ in 0..200 {
        update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.01, &mut fx);
    }
    assert!((units[0].health - (UNIT_HEALTH - 25.0 * 2.0)).abs() < 1.0, "{}", units[0].health);
    assert!(hazards.is_empty());
}

#[test]
fn packs_and_warbands_move_together_and_a_warband_has_a_healer() {
    let (mut packs, mut warbands) = (0, 0);
    for _ in 0..600 {
        let group = spawn_group(MIDDLE.x, MIDDLE.y, 3000);
        if group.len() == 1 {
            continue;
        }
        let route = group[0].route.clone();
        assert!(group.iter().all(|enemy| enemy.route == route), "a group shares a route");
        assert!(group.iter().all(|enemy| enemy.position.distance(group[0].position) <= 2.0 * PACK_SPREAD + 1.0));
        if group.iter().all(|enemy| enemy.name == "Swarmer") {
            packs += 1;
            assert!(group.len() >= kind_named("Swarmer").pack);
        } else {
            warbands += 1;
            assert!(group.iter().any(|enemy| enemy.name == "Healer"), "a warband has a healer");
            assert!(group.len() >= 5 && group.len() <= 8, "{}", group.len());
            assert!(group.iter().filter(|enemy| enemy.name == "Healer").count() == 1);
        }
    }
    assert!(packs > 10 && warbands > 5, "{packs} packs and {warbands} warbands in 600 groups");
}

#[test]
fn a_small_army_meets_no_warbands_and_a_pack_of_swarmers_is_bigger_for_a_big_army() {
    for _ in 0..500 {
        let group = spawn_group(MIDDLE.x, MIDDLE.y, WARBAND_FROM_UNITS - 1);
        assert!(group.iter().all(|enemy| enemy.name != "Healer"));
    }
    let swarm = |army: usize| (0..2000).map(|_| spawn_group(MIDDLE.x, MIDDLE.y, army)).find(|group| group[0].name == "Swarmer").unwrap().len();
    assert_eq!(swarm(0), kind_named("Swarmer").pack);
    assert!(swarm(2900) > swarm(0));
    assert!(swarm(MAX_UNITS) <= 30);
}

#[test]
fn recall_sends_every_squad_home() {
    let mut units: Vec<Unit> = (0..10).map(|i| unit_on(MIDDLE + Vec2::new(i as f32 * 10.0, 0.0))).collect();
    let enemy = still_enemy(MIDDLE + Vec2::new(500.0, 0.0), 1.0e6);
    let mut squads = Squads::new();
    let mut fx = Effects::default();
    send_squad(&mut units[..6], &mut squads, &enemy, &mut fx);
    assert_eq!(units.iter().filter(|unit| unit.target.is_some()).count(), 6);
    assert_eq!(squads.len(), 1);
    let mut fx = Effects::default();
    assert_eq!(recall_squads(&mut units, &mut squads, MIDDLE, &mut fx), 6);
    assert!(units.iter().all(|unit| unit.target.is_none()) && squads.is_empty());
    assert!(fx.popups.iter().any(|popup| popup.text == "6 recalled"));
    //nothing to recall: nothing said
    let mut fx = Effects::default();
    assert_eq!(recall_squads(&mut units, &mut squads, MIDDLE, &mut fx), 0);
    assert!(fx.popups.is_empty() && fx.cues.is_empty());
}

#[test]
fn what_the_tooltip_says_about_an_enemy() {
    assert!(traits(&kind_at("Grunt", MIDDLE)).contains(&"chases".to_owned()));
    assert!(traits(&kind_at("Spitter", MIDDLE)).contains(&"shoots".to_owned()));
    assert!(!traits(&kind_at("Spitter", MIDDLE)).contains(&"chases".to_owned()));
    assert!(traits(&kind_at("Bulwark", MIDDLE)).contains(&"shielded".to_owned()));
    assert!(traits(&kind_at("Charger", MIDDLE)).contains(&"charges".to_owned()));
    assert!(traits(&kind_at("Bomber", MIDDLE)).contains(&"explodes on contact".to_owned()));
    assert!(traits(&kind_at("Splitter", MIDDLE)).contains(&"splits when destroyed".to_owned()));
    let mut elite = kind_at("Grunt", MIDDLE);
    elite.make_elite(Modifier::Armored);
    assert!(traits(&elite).contains(&"armor 50%".to_owned()));
    elite.make_elite(Modifier::Volatile);
    assert!(traits(&elite).contains(&"explodes when destroyed".to_owned()));
    let imp = Enemy::minion(MIDDLE, kind_named("Imp"), 1.0, 1);
    assert!(traits(&imp).contains(&"minion, worth nothing".to_owned()));
    assert!(traits(&Enemy::boss(&BOSSES[0], 100, 0, MIDDLE)).contains(&"boss".to_owned()));
    assert!(traits(&kind_at("Healer", MIDDLE)).contains(&"mends other enemies".to_owned()));
}

#[test]
fn fire_does_not_add_up_where_burning_places_overlap() {
    let one = {
        let mut units = vec![unit_on(MIDDLE)];
        let mut hazards = vec![hazard_at(MIDDLE, 100.0, 0.0, 0.0, 0).burning(2.0, 25.0)];
        for _ in 0..100 {
            update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.01, &mut Effects::default());
        }
        units[0].health
    };
    let mut units = vec![unit_on(MIDDLE)];
    let mut hazards: Vec<Hazard> = (0..5).map(|_| hazard_at(MIDDLE, 100.0, 0.0, 0.0, 0).burning(2.0, 25.0)).collect();
    for _ in 0..100 {
        update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.01, &mut Effects::default());
    }
    assert!((units[0].health - one).abs() < 1e-3, "five fires {} against one {one}", units[0].health);
    //a hotter one wins where they overlap
    let mut units = vec![unit_on(MIDDLE)];
    let mut hazards = vec![hazard_at(MIDDLE, 100.0, 0.0, 0.0, 0).burning(2.0, 25.0), hazard_at(MIDDLE, 100.0, 0.0, 0.0, 0).burning(2.0, 50.0)];
    for _ in 0..100 {
        update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.01, &mut Effects::default());
    }
    assert!((units[0].health - (UNIT_HEALTH - 50.0)).abs() < 1.0, "{}", units[0].health);
}

#[test]
fn the_enemies_follow_the_player_across_the_world_instead_of_staying_where_the_run_began() {
    rand::srand(21);
    let leash = ENEMY_LEASH_TILES * TILE_SIZE;
    let mut enemies = Vec::new();
    respawn_enemies(&mut enemies, MIDDLE.x, MIDDLE.y, 2);
    //what something else made, or a boss, or a lair's own, stays wherever it is
    let far = Vec2::new(60000.0, 16000.0);
    let owner = 99_999;
    enemies.push(Enemy::minion(far, kind_named("Imp"), 1.0, owner));
    enemies.push(Enemy::boss(&BOSSES[0], 100, 0, far));
    let mut lair_guard = kind_at("Brute", far);
    lair_guard.site = Some(7);
    enemies.push(lair_guard);
    //standing still lets nobody go and makes nobody new
    let ids = |enemies: &[Enemy]| enemies.iter().map(|enemy| enemy.id).collect::<HashSet<u32>>();
    let before = ids(&enemies);
    respawn_enemies(&mut enemies, MIDDLE.x, MIDDLE.y, 2);
    assert_eq!(before, ids(&enemies));
    //the player walks off in steps, and every one of the wild ones is always near them
    //(the random numbers are shared with the tests that seed them, which can skew one walk, so a wrong spread has to get it
    //wrong on every walk to fail)
    let mut ahead = 0;
    for _ in 0..4 {
        //each walk starts from the middle with a fresh crowd
        let mut player = MIDDLE;
        enemies.retain(|enemy| !is_wild(enemy));
        respawn_enemies(&mut enemies, player.x, player.y, 2);
        for _ in 0..40 {
            player.x += 15.0 * TILE_SIZE;
            respawn_enemies(&mut enemies, player.x, player.y, 2);
            let wild: Vec<&Enemy> = enemies.iter().filter(|enemy| is_wild(enemy)).collect();
            assert!(wild.len() >= ENEMY_COUNT, "{} wild enemies at {}", wild.len(), player.x);
            for enemy in &wild {
                assert!(enemy.position.distance(player) <= leash, "an enemy {} squares away was kept", enemy.position.distance(player) / TILE_SIZE);
            }
        }
        //the ones that turned up on the way are there to be met, on every side of where the player is heading
        ahead = enemies.iter().filter(|enemy| is_wild(enemy) && enemy.position.x > player.x).count();
        if ahead * 100 > ENEMY_COUNT * 28 {
            break;
        }
    }
    assert_eq!(enemies.iter().filter(|enemy| !is_wild(enemy)).count(), 3, "what is not wild is never let go");
    assert!(ahead * 100 > ENEMY_COUNT * 28, "only {ahead} of {ENEMY_COUNT} are ahead of the player");
}

#[test]
fn new_enemies_start_where_there_are_the_fewest_already() {
    rand::srand(3);
    let ring = (ENEMY_SPAWN_MIN_TILES + 5.0) * TILE_SIZE;
    let mut east = 0;
    let trials = 200;
    for _ in 0..trials {
        //every one but one of them is crowded into the west of the ring
        let mut enemies: Vec<Enemy> = (0..ENEMY_COUNT - 1)
            .map(|i| {
                let at = MIDDLE + Vec2::new(-ring - (i % 25) as f32 * 40.0, (i / 25) as f32 * 40.0 - 400.0);
                kind_at("Brute", at)
            })
            .collect();
        let before = enemies.len();
        respawn_enemies(&mut enemies, MIDDLE.x, MIDDLE.y, 2);
        let fresh = &enemies[before];
        east += (fresh.position.x > MIDDLE.x) as usize;
    }
    //picked at random it would be half and half
    assert!(east * 100 > trials * 68, "{east} of {trials} started in the empty half");
}

#[test]
fn a_shifted_enemy_takes_its_route_with_it_and_stays_inside_the_world() {
    let mut enemy = kind_at("Brute", MIDDLE);
    let route = enemy.route.clone();
    let step = Vec2::new(1000.0, -500.0);
    enemy.shift(step);
    assert!(enemy.position.distance(MIDDLE + step) < 1e-3);
    assert_eq!(enemy.route.len(), route.len());
    for (new, old) in enemy.route.iter().zip(&route) {
        assert!(new.distance(*old + step) < 1e-3);
    }
    enemy.shift(Vec2::splat(-1.0e7));
    let edge = enemy.size / 2.0;
    assert!(enemy.position.x >= edge - 1e-3 && enemy.position.y >= edge - 1e-3);
    assert!(enemy.route.iter().all(|point| point.x >= edge - 1e-3 && point.y >= edge - 1e-3));
    enemy.shift(Vec2::splat(2.0e7));
    assert!(enemy.position.x <= WORLD_SIZE - edge + 1e-3 && enemy.position.y <= WORLD_SIZE - edge + 1e-3);
}
