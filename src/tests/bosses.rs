//bosses, the director, relics and the messages

use super::*;


// ---------- bosses, the director, relics and the messages ----------

#[test]
fn the_boss_table_is_sane() {
    let mut names = HashSet::new();
    let mut last_army = 0;
    for def in BOSSES.iter() {
        assert!(names.insert(def.name), "{} twice", def.name);
        assert!(!def.phases.is_empty() && def.phases[0].below == 1.0, "{}", def.name);
        for pair in def.phases.windows(2) {
            assert!(pair[1].below < pair[0].below && pair[1].below > LAST_STAND_AT, "{}: phases must go down and leave room for the last stand", def.name);
        }
        for phase in def.phases {
            assert!(!phase.casts.is_empty() && phase.speed > 0.0 && phase.shield >= 0.0, "{}: {}", def.name, phase.name);
            assert!(phase.casts.iter().all(|cast| cast.every >= 2.0), "{}: {} attacks too often", def.name, phase.name);
        }
        assert_eq!(kind_named(def.minion).name, def.minion, "{} has no minion kind {}", def.name, def.minion);
        assert!(def.min_army >= last_army, "the bosses ask for a bigger army each time");
        last_army = def.min_army;
        assert!(def.kill_seconds > 0.0 && def.speed > 0.0 && def.attack_share > 0.0 && def.babies_share > 0.0 && def.brood > 0);
        assert!(def.size > 0.0 && def.growth > 0.0);
    }
}

#[test]
fn a_boss_is_sized_for_the_army_so_it_can_be_surrounded_and_beaten_in_about_the_same_time() {
    for def in BOSSES.iter() {
        let mut last_size = 0.0;
        for army in [100usize, 300, 1000, 3000, 6000] {
            let boss = Enemy::boss(def, army, 0, MIDDLE);
            let sized_for = army.max(def.min_army) as f32;
            assert!(boss.size >= last_size && boss.size < WORLD_SIZE / 8.0, "{} at {army}: {}", def.name, boss.size);
            last_size = boss.size;
            //the whole army, all touching it, would take kill_seconds
            let seconds = boss.max_health / (sized_for * UNIT_DAMAGE);
            assert!((seconds - def.kill_seconds).abs() < 0.01);
            //how many circles the size of a unit would fit in it: enough for a good share of the army
            let room = ((boss.size + 2.0 * RADIUS) / (2.0 * RADIUS)).powi(2);
            assert!(room >= sized_for * 0.2, "{} at {army}: room for {room}", def.name);
            assert_eq!(boss.babies, (sized_for * def.babies_share).round() as usize);
            assert!(boss.chasing && boss.boss.is_some() && boss.owner.is_none() && boss.health == boss.max_health);
            assert!(boss.attack > 0.0);
        }
    }
}

#[test]
fn each_round_of_bosses_is_tougher() {
    for def in BOSSES.iter() {
        let first = Enemy::boss(def, 500, 0, MIDDLE);
        let second = Enemy::boss(def, 500, 1, MIDDLE);
        assert!(second.max_health > first.max_health * 1.3 && second.attack > first.attack * 1.2, "{}", def.name);
    }
}

#[test]
fn a_boss_runs_at_the_player_from_the_start_at_its_own_speed_and_never_gives_up() {
    for def in BOSSES.iter() {
        let mut enemies = vec![Enemy::boss(def, 1000, 0, MIDDLE)];
        let player = MIDDLE + Vec2::new(10_000.0, 0.0);
        let mut fx = Effects::default();
        patrol_free(&mut enemies, player, 1.0, 1.0, &mut fx);
        let moved = enemies[0].position.distance(MIDDLE);
        assert!((moved - def.speed).abs() < 0.5, "{}: {moved} against {}", def.name, def.speed);
        assert!(fx.popups.is_empty() && fx.cues.is_empty(), "it doesn't need to notice anyone");
        //the army's speed multiplier applies to it like to everyone
        let before = enemies[0].position;
        patrol_free(&mut enemies, player, 1.0, 3.0, &mut fx);
        assert!((enemies[0].position.distance(before) - def.speed * 3.0).abs() < 1.0);
        //however far the player gets
        let far = Vec2::new(31_000.0, 31_000.0);
        patrol_free(&mut enemies, far, 0.1, 1.0, &mut fx);
        assert!(enemies[0].chasing);
    }
}

#[test]
fn a_boss_changes_phase_as_its_health_runs_down_pausing_unhurt_and_pulsing() {
    for def in BOSSES.iter() {
        let units: Vec<Unit> = (0..200).map(|i| unit_on(MIDDLE + Vec2::new(i as f32, 0.0))).collect();
        let mut enemies = vec![Enemy::boss(def, 300, 0, MIDDLE + Vec2::new(2000.0, 0.0))];
        //one that splits would do it as soon as it is hurt, which is not what this is about
        enemies[0].boss.as_mut().unwrap().generation = SPLIT_AT.len() as u32;
        let mut shots = Vec::new();
        let mut fx = Effects::default();
        for phase in 1..def.phases.len() {
            let next = &def.phases[phase];
            enemies[0].health = enemies[0].max_health * (next.below - 0.01);
            boss_act(&mut enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
            let position = enemies[0].position;
            let state = enemies[0].boss.as_ref().unwrap();
            assert_eq!(state.phase, phase, "{}", def.name);
            assert!(state.shifting > 0.0);
            assert_eq!(state.timers.len(), next.casts.len());
            assert!(matches!(enemies[0].mode, Mode::Cast { .. }));
            if next.shield > 0.0 {
                assert!((enemies[0].shield - enemies[0].max_health * next.shield).abs() < 1.0, "{}: {}", def.name, next.name);
            }
            assert_eq!(enemies[0].take_damage(1.0e9), 0.0, "unhurt between phases");
            assert!(fx.hazards.iter().any(|hazard| hazard.position == position && hazard.warn >= PHASE_SHIFT_SECONDS - 0.01), "a pulse is coming");
            assert!(fx.announcements.iter().any(|(text, _)| text.contains(&next.name.to_uppercase())));
            assert!(fx.cues.contains(&Cue::PhaseShift));
            //the pause ends, the boss can be hurt again, and the new phase's attacks have begun to be timed
            for _ in 0..((PHASE_SHIFT_SECONDS + 0.2) * 100.0) as usize {
                patrol_free(&mut enemies, MIDDLE, 0.01, 1.0, &mut fx);
                boss_act(&mut enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
            }
            assert_eq!(enemies[0].boss.as_ref().unwrap().shifting, 0.0);
            assert!(enemies[0].take_damage(1.0) > 0.0);
            fx = Effects::default();
            shots.clear();
        }
    }
}

#[test]
fn a_boss_makes_a_last_stand_once_when_it_is_nearly_dead() {
    let units: Vec<Unit> = (0..100).map(|i| unit_on(MIDDLE + Vec2::new(i as f32, 0.0))).collect();
    let mut enemies = vec![Enemy::boss(&BOSSES[2], 400, 0, MIDDLE + Vec2::new(2000.0, 0.0))];
    let (mut shots, mut fx) = (Vec::new(), Effects::default());
    enemies[0].health = enemies[0].max_health * (LAST_STAND_AT / 2.0);
    //it shifts to its last phase first
    for _ in 0..((PHASE_SHIFT_SECONDS + 1.5) * 100.0) as usize {
        patrol_free(&mut enemies, MIDDLE, 0.01, 1.0, &mut fx);
        boss_act(&mut enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
    }
    let state = enemies[0].boss.as_ref().unwrap();
    assert!(state.last_stand && state.phase == BOSSES[2].phases.len() - 1);
    let phase_speed = BOSSES[2].speed / CHASE_SPEED * BOSSES[2].phases[state.phase].speed;
    assert!((enemies[0].speed - phase_speed * 1.3).abs() < 1e-3, "{} against {}", enemies[0].speed, phase_speed * 1.3);
    assert_eq!(fx.announcements.iter().filter(|(text, _)| text.contains("LAST STAND")).count(), 1);
    //and doesn't do it again
    for _ in 0..300 {
        boss_act(&mut enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
    }
    assert!((enemies[0].speed - phase_speed * 1.3).abs() < 1e-3);
    assert_eq!(fx.announcements.iter().filter(|(text, _)| text.contains("LAST STAND")).count(), 1);
}

#[test]
fn every_boss_pattern_does_what_it_says() {
    let units: Vec<Unit> = (0..400).map(|i| unit_on(MIDDLE + Vec2::new((i % 20) as f32 * 30.0, (i / 20) as f32 * 30.0))).collect();
    let player = MIDDLE + Vec2::new(300.0, 300.0);
    let cast = |pattern: Pattern, owned: usize, tier: u32| {
        let mut boss = Enemy::boss(&BOSSES[0], 400, tier, MIDDLE - Vec2::new(1500.0, 0.0));
        let mut state = BossState { def: &BOSSES[0], tier, phase: 0, timers: vec![], shifting: 0.0, last_stand: false, family: 0, family_max: 1.0, generation: 0, facing: Vec2::X, anchor_left: 0.0 };
        let (mut fx, mut born, mut shots) = (Effects::default(), Vec::new(), Vec::new());
        cast_pattern(&mut boss, &mut state, pattern, &units, player, owned, &mut born, &mut shots, &mut fx);
        (boss, fx, born, shots)
    };

    let (boss, fx, _, _) = cast(Pattern::Slam, 0, 0);
    assert_eq!(fx.hazards.len(), 1);
    let slam = &fx.hazards[0];
    assert_eq!(slam.position, boss.position);
    assert!(slam.radius > boss.size * 0.75 && slam.warn > 1.0);
    assert_eq!(slam.damage, UNIT_HEALTH * BOSS_BLAST_FACTOR, "it kills what it lands on, however tough");
    assert_eq!(slam.victims, victims_of(400, 0.4), "and takes a good part of an army that stands in it");
    assert!(matches!(boss.mode, Mode::Cast { left } if left > slam.warn), "it stands still until it lands");
    assert!(fx.cues.contains(&Cue::Warning));

    let (_, fx, _, _) = cast(Pattern::Meteors, 0, 0);
    assert_eq!(fx.hazards.len(), 5 + 400 / 400);
    assert!(fx.hazards.windows(2).all(|pair| pair[1].warn > pair[0].warn), "they land one after another");
    assert!(fx.hazards.iter().all(|hazard| hazard.warn >= 1.0 && hazard.damage >= UNIT_HEALTH && hazard.victims == victims_of(400, 0.04)));
    let army_bounds = Rect::new(MIDDLE.x - 200.0, MIDDLE.y - 200.0, 19.0 * 30.0 + 400.0, 19.0 * 30.0 + 400.0);
    assert!(fx.hazards.iter().all(|hazard| army_bounds.contains(hazard.position)), "meteors fall on the army");
    let (_, more, _, _) = cast(Pattern::Meteors, 0, 2);
    assert_eq!(more.hazards.len(), 5 + 4 + 1, "a later round throws more");

    let (boss, fx, _, _) = cast(Pattern::Nova, 0, 0);
    let nova = &fx.hazards[0];
    assert!(nova.radius >= 650.0 && nova.warn >= 2.0 && nova.damage >= UNIT_HEALTH && nova.position == boss.position);
    assert_eq!(nova.victims, victims_of(400, 0.55));

    let (boss, fx, born, _) = cast(Pattern::Brood, 0, 0);
    assert_eq!(born.len(), BOSSES[0].brood + 20 / 4);
    assert!(born.iter().all(|minion| minion.owner == Some(boss.id) && minion.name == "Imp" && minion.babies == 0));
    assert!(fx.cues.contains(&Cue::Summon));
    let (_, _, born, _) = cast(Pattern::Brood, MAX_BOSS_MINIONS - 4, 0);
    assert_eq!(born.len(), 4, "only as many as there is room for");
    let (_, fx, born, _) = cast(Pattern::Brood, MAX_BOSS_MINIONS, 0);
    assert!(born.is_empty() && !fx.cues.contains(&Cue::Summon));

    let (boss, fx, _, shots) = cast(Pattern::Ring, 0, 0);
    assert_eq!(shots.len(), 16 + 400 / 300);
    assert!(shots.iter().all(|shot| (shot.velocity.length() - 340.0).abs() < 1e-2 && shot.damage > SHOT_DAMAGE && shot.radius > SHOT_RADIUS));
    assert!(shots.iter().all(|shot| (shot.position.distance(boss.position) - boss.size / 2.0).abs() < 1e-2));
    let sum: Vec2 = shots.iter().map(|shot| shot.velocity).sum();
    assert!(sum.length() < 1.0, "evenly all round: {sum}");
    assert!(fx.cues.contains(&Cue::Shoot));

    let (_, fx, _, _) = cast(Pattern::Burn, 0, 1);
    assert_eq!(fx.hazards.len(), 4);
    assert!(fx.hazards.iter().all(|hazard| hazard.damage == 0.0 && hazard.linger >= 5.0 && hazard.burn > 0.0));

    let (boss, _, _, _) = cast(Pattern::Charge, 0, 0);
    let Mode::Windup { dir, left } = boss.mode else {
        panic!("{:?}", boss.mode);
    };
    assert!(dir.dot((player - boss.position).normalize()) > 0.999 && left > 0.3);
}

#[test]
fn a_boss_left_alone_attacks_regularly_and_only_one_attack_at_a_time() {
    for def in BOSSES.iter() {
        let units: Vec<Unit> = (0..300).map(|i| unit_on(MIDDLE + Vec2::new((i % 20) as f32 * 10.0, (i / 20) as f32 * 10.0))).collect();
        let mut enemies = vec![Enemy::boss(def, 300.max(def.min_army), 0, MIDDLE)];
        let (mut shots, mut hazards) = (Vec::new(), Vec::<Hazard>::new());
        let mut busy_before = false;
        let mut attacks = 0;
        for _ in 0..(30.0 / 0.02) as usize {
            let mut fx = Effects::default();
            patrol_free(&mut enemies, MIDDLE, 0.02, 1.0, &mut fx);
            boss_act(&mut enemies, &units, MIDDLE, 0.02, &mut shots, &mut fx);
            let busy = !matches!(enemies[0].mode, Mode::Roam);
            if busy && !busy_before {
                attacks += 1;
            }
            busy_before = busy;
            hazards.append(&mut fx.hazards);
            //the minions it calls are not what is being counted
            enemies.truncate(1);
            shots.clear();
        }
        let quickest = def.phases[0].casts.iter().map(|cast| cast.every).fold(f32::MAX, f32::min);
        assert!(attacks >= 3 && attacks as f32 <= 30.0 / quickest * 2.0 + 2.0, "{}: {attacks} attacks in 30 s", def.name);
    }
}

#[test]
fn the_hive_mother_takes_less_damage_the_more_minions_it_has_alive() {
    for (index, guarded) in [(0usize, true), (1, false)] {
        let mut enemies = vec![Enemy::boss(&BOSSES[index], 200.max(BOSSES[index].min_army), 0, MIDDLE)];
        let id = enemies[0].id;
        for n in [0usize, 5, 10, 40] {
            enemies.truncate(1);
            for _ in 0..n {
                enemies.push(Enemy::minion(MIDDLE, kind_named("Imp"), 1.0, id));
            }
            boss_act(&mut enemies, &[], MIDDLE, 0.0, &mut Vec::new(), &mut Effects::default());
            let expected = if guarded { (n as f32 * 0.04).min(0.6) } else { 0.0 };
            assert!((enemies[0].armor - expected).abs() < 1e-4, "{n} minions: {}", enemies[0].armor);
        }
    }
}

#[test]
fn the_director_sends_a_boss_after_a_countdown_that_waits_for_a_big_enough_army() {
    let mut director = Director::new();
    let mut enemies = Vec::new();
    for _ in 0..100 {
        assert_eq!(director.update(&mut enemies, MIDDLE, BOSSES[0].min_army - 1, 1.0), None);
    }
    assert_eq!(director.next_boss_in, FIRST_BOSS_SECONDS, "the countdown waits for the army");
    let mut beats = Vec::new();
    let mut seconds = 0.0;
    while director.boss.is_none() && seconds < FIRST_BOSS_SECONDS + 20.0 {
        if let Some(beat) = director.update(&mut enemies, MIDDLE, BOSSES[0].min_army + 400, 0.5) {
            beats.push((seconds, beat));
        }
        seconds += 0.5;
    }
    assert_eq!(beats.len(), 2, "{beats:?}");
    assert_eq!(beats[0].1, Beat::Warning);
    assert!((beats[0].0 - (FIRST_BOSS_SECONDS - BOSS_WARNING_SECONDS)).abs() <= 1.0, "{}", beats[0].0);
    assert_eq!(beats[1].1, Beat::Arrived(BOSSES[0].name));
    assert!((beats[1].0 - FIRST_BOSS_SECONDS).abs() <= 1.0);
    let boss = current_boss(&enemies).expect("the boss is there");
    let tiles = boss.position.distance(MIDDLE) / TILE_SIZE;
    assert!(tiles >= BOSS_SPAWN_MIN_TILES - 0.01 && tiles <= BOSS_SPAWN_MAX_TILES + 0.01, "{tiles} squares away");
    assert_eq!(director.boss, Some(boss.id));
    for _ in 0..100 {
        assert_eq!(director.update(&mut enemies, MIDDLE, 500, 0.5), None, "nothing more while the boss is alive");
    }
    enemies.clear();
    assert_eq!(director.update(&mut enemies, MIDDLE, 500, 0.5), Some(Beat::Slain { name: BOSSES[0].name, tier: 0 }));
    assert_eq!((director.slain, director.boss), (1, None));
    assert_eq!(director.next_boss_in, BOSS_GAP_SECONDS);
    assert_eq!(director.next_def().name, BOSSES[1].name);
    assert_eq!(director.update(&mut enemies, MIDDLE, 500, 0.5), None);
}

#[test]
fn the_bosses_come_in_turn_and_round_again_tougher() {
    let mut director = Director::new();
    let mut last_health = HashMap::new();
    for round in 0..9 {
        let (def, tier) = (director.next_def(), director.next_tier());
        assert_eq!(def.name, BOSSES[round % BOSSES.len()].name);
        assert_eq!(tier as usize, round / BOSSES.len());
        let mut enemies = Vec::new();
        director.next_boss_in = 0.0;
        assert_eq!(director.update(&mut enemies, MIDDLE, 10_000, 0.1), Some(Beat::Arrived(def.name)));
        assert_eq!(enemies[0].boss.as_ref().unwrap().tier, tier);
        let health = enemies[0].max_health;
        if let Some(&before) = last_health.get(def.name) {
            assert!(health > before, "{} is tougher the second time round", def.name);
        }
        last_health.insert(def.name, health);
        enemies.clear();
        assert_eq!(director.update(&mut enemies, MIDDLE, 10_000, 0.1), Some(Beat::Slain { name: def.name, tier }));
    }
}

#[test]
fn a_slain_boss_drops_its_babies_takes_its_minions_with_it_and_counts_once() {
    let boss = Enemy::boss(&BOSSES[0], 1000, 0, MIDDLE);
    let (id, babies_dropped) = (boss.id, boss.babies);
    let mut enemies = vec![boss];
    for _ in 0..10 {
        enemies.push(Enemy::minion(MIDDLE, kind_named("Imp"), 3.0, id));
    }
    enemies.push(kind_at("Grunt", MIDDLE));
    enemies[0].health = 0.0;
    let (mut babies, mut fx) = (Vec::new(), Effects::default());
    assert_eq!(resolve_deaths(&mut enemies, &mut babies, 1000, &Upgrades::default(), &mut fx), 1);
    assert_eq!(enemies.len(), 1);
    assert_eq!(enemies[0].name, "Grunt");
    assert_eq!(babies.len(), babies_dropped);
    assert!(fx.shake > 0.99, "{}", fx.shake);
    assert!(fx.cues.contains(&Cue::Thud) && fx.cues.contains(&Cue::Explosion));

    //a bounty relic brings more
    let mut upgrades = Upgrades::default();
    upgrades.take(Relic::Bounty);
    let mut boss = Enemy::boss(&BOSSES[0], 1000, 0, MIDDLE);
    boss.health = 0.0;
    let mut babies = Vec::new();
    resolve_deaths(&mut vec![boss], &mut babies, 1000, &upgrades, &mut Effects::default());
    assert_eq!(babies.len(), (babies_dropped as f32 * 1.4).round() as usize);
    //and there is never more than there is room for
    let mut boss = Enemy::boss(&BOSSES[0], 5000, 0, MIDDLE);
    boss.health = 0.0;
    let mut babies = Vec::new();
    resolve_deaths(&mut vec![boss], &mut babies, DEFAULT_ARMY_LIMIT - 10, &Upgrades::default(), &mut Effects::default());
    assert_eq!(babies.len(), 10);
}

#[test]
fn relics_multiply_what_they_say_and_stack() {
    let mut up = Upgrades::default();
    assert!(up.damage == 1.0 && up.toughness == 1.0 && up.healing == 1.0 && up.speed == 1.0 && up.pickup == 1.0 && up.bounty == 1.0 && up.taken.is_empty());
    up.take(Relic::Whetstone);
    up.take(Relic::Whetstone);
    assert!((up.damage - 1.25 * 1.25).abs() < 1e-5);
    assert_eq!((up.count(Relic::Whetstone), up.count(Relic::Venom), up.taken.len()), (2, 0, 2));
    let mut up = Upgrades::default();
    for relic in RELICS {
        up.take(relic);
    }
    assert!(up.damage > 1.0 && up.speed > 1.0 && up.bounty > 1.0 && up.venom > 0.0 && up.kindling > 0.0 && up.mire < 1.0 && up.charge > 0.0);
    assert!(up.thorns > 0.0 && up.bloodlust > 0.0 && up.guard > 0.0 && up.salvo > 0.0 && up.martyr > 0.0);
    let mut names = HashSet::new();
    for relic in RELICS {
        assert!(names.insert(relic.name()) && !relic.blurb().is_empty());
    }
}

#[test]
fn three_different_relics_are_offered_and_every_relic_gets_offered() {
    let mut seen = HashSet::new();
    for _ in 0..300 {
        let choices = relic_choices();
        assert!(choices[0] != choices[1] && choices[1] != choices[2] && choices[0] != choices[2]);
        seen.extend(choices.iter().map(|relic| relic.name()));
    }
    assert_eq!(seen.len(), RELICS.len());
}

#[test]
fn the_relics_change_the_fight_the_way_they_say() {
    //whetstone: 10 units for 0.9 s do 900, or 1125 with one whetstone
    for (relic, destroyed) in [(None, false), (Some(Relic::Whetstone), true)] {
        let mut up = Upgrades::default();
        if let Some(relic) = relic {
            up.take(relic);
        }
        let enemy = still_enemy(MIDDLE, 1000.0);
        let mut units = units_inside(&enemy, 10);
        let mut enemies = vec![enemy];
        let mut gone = 0;
        for _ in 0..54 {
            gone += fight(&mut enemies, &mut units, &mut Vec::new(), 1.0 / 60.0, &up, &mut Effects::default());
        }
        assert_eq!(gone == 1, destroyed, "{relic:?}");
    }
    //iron resolve: the same hit leaves a unit with more
    for (relic, health) in [(None, 0.0), (Some(Relic::IronResolve), UNIT_HEALTH - 80.0)] {
        let mut up = Upgrades::default();
        if let Some(relic) = relic {
            up.take(relic);
        }
        let enemy = striker(MIDDLE, 100.0);
        let mut units = units_inside(&enemy, 5);
        let mut enemies = vec![enemy];
        for _ in 0..60 {
            fight(&mut enemies, &mut units, &mut Vec::new(), 1.0 / 60.0, &up, &mut Effects::default());
        }
        assert!((units[0].health - health).abs() < 0.5, "{relic:?}: {}", units[0].health);
    }
    //iron resolve against a shot
    let mut up = Upgrades::default();
    up.take(Relic::IronResolve);
    let mut units = vec![unit_on(MIDDLE)];
    let mut shots = vec![Shot::new(MIDDLE - Vec2::new(5.0, 0.0), Vec2::new(SHOT_SPEED, 0.0))];
    update_shots(&mut shots, &mut units, &Walls::default(), 1.0 / 60.0, &up, &mut Effects::default());
    assert!((units[0].health - (UNIT_HEALTH - SHOT_DAMAGE * 0.8)).abs() < 1e-3);
    //regeneration
    let mut hurt = vec![unit_on(MIDDLE)];
    hurt[0].health = 10.0;
    heal_units(&mut hurt, 1.0, &Upgrades::default());
    assert!((hurt[0].health - (10.0 + UNIT_HEAL_RATE)).abs() < 1e-4);
    let mut up = Upgrades::default();
    up.take(Relic::Regeneration);
    hurt[0].health = 10.0;
    heal_units(&mut hurt, 1.0, &up);
    assert!((hurt[0].health - (10.0 + UNIT_HEAL_RATE * 2.0)).abs() < 1e-4);
    //bounty
    let mut up = Upgrades::default();
    up.take(Relic::Bounty);
    let mut enemy = kind_at("Grunt", MIDDLE);
    enemy.babies = 10;
    let mut babies = Vec::new();
    drop_babies(&enemy, &mut babies, 0, up.bounty, DEFAULT_ARMY_LIMIT);
    assert_eq!(babies.len(), 14);
}

#[test]
fn messages_are_shown_one_at_a_time_and_a_queue_shortens_them() {
    let mut banners = Banners::new();
    banners.update(0.1);
    assert!(banners.shown.is_none());
    banners.push("one".to_owned(), WHITE);
    banners.update(0.0);
    let shown = banners.shown.as_ref().unwrap();
    assert_eq!((shown.text.as_str(), shown.life), ("one", BANNER_SECONDS));
    banners.push("two".to_owned(), WHITE);
    banners.push("three".to_owned(), WHITE);
    banners.update(BANNER_SECONDS * 0.9);
    assert_eq!(banners.shown.as_ref().unwrap().text, "one");
    banners.update(BANNER_SECONDS * 0.2);
    let shown = banners.shown.as_ref().unwrap();
    assert_eq!(shown.text, "two");
    assert!(shown.life < BANNER_SECONDS, "with another waiting it is up for less");
    banners.update(BANNER_SECONDS);
    let shown = banners.shown.as_ref().unwrap();
    assert_eq!((shown.text.as_str(), shown.life), ("three", BANNER_SECONDS));
    for i in 0..10 {
        banners.push(format!("{i}"), WHITE);
    }
    assert_eq!(banners.waiting.len(), MAX_WAITING_BANNERS);
}

#[test]
fn the_hud_says_which_boss_is_next_and_when() {
    let mut director = Director::new();
    assert!(boss_status(&director, 1).contains(&format!("from {} units", BOSSES[0].min_army)));
    let seconds = FIRST_BOSS_SECONDS.ceil() as usize;
    assert!(boss_status(&director, 10_000).contains(&format!("in {}:{:02}", seconds / 60, seconds % 60)), "{}", boss_status(&director, 10_000));
    assert!(boss_status(&director, 10_000).contains(BOSSES[0].name));
    director.boss = Some(1);
    assert_eq!(boss_status(&director, 10_000), format!("BOSS: {}", BOSSES[0].name));
}

#[test]
fn the_three_relic_cards_fit_on_the_screen_without_touching() {
    for screen in [Vec2::new(1280.0, 720.0), Vec2::new(800.0, 600.0), Vec2::new(1920.0, 1080.0), Vec2::new(3840.0, 2160.0), Vec2::new(640.0, 480.0)] {
        let cards = relic_cards(screen);
        for card in &cards {
            assert!(card.x >= 0.0 && card.right() <= screen.x && card.y >= 0.0 && card.bottom() <= screen.y, "{card:?} on {screen}");
        }
        assert!(cards[0].right() < cards[1].x && cards[1].right() < cards[2].x);
        for (i, card) in cards.iter().enumerate() {
            assert_eq!(relic_card_at(screen, card.center()), Some(i));
        }
        assert_eq!(relic_card_at(screen, Vec2::new(1.0, 1.0)), None);
    }
}

#[test]
fn every_cue_is_in_the_list_at_its_own_place() {
    for (i, cue) in ALL_CUES.iter().enumerate() {
        assert_eq!(*cue as usize, i, "{cue:?}");
    }
    let cues: HashSet<usize> = ALL_CUES.iter().map(|cue| *cue as usize).collect();
    assert_eq!(cues.len(), ALL_CUES.len());
}


#[test]
fn every_round_is_stronger_in_health_attack_pace_and_what_it_telegraphs() {
    let def = &BOSSES[3];
    let (first, third) = (Enemy::boss(def, 500, 0, MIDDLE), Enemy::boss(def, 500, 2, MIDDLE));
    assert!((third.max_health / first.max_health - (1.0 + 2.0 * BOSS_TIER_HEALTH)).abs() < 1e-3);
    assert!((third.attack / first.attack - (1.0 + 2.0 * BOSS_TIER_ATTACK)).abs() < 1e-3);
    assert!(third.speed > first.speed * 1.09);
    //it gets no faster than a point, but the rest keeps growing
    assert_eq!(tier_pace(0), 1.0);
    assert_eq!(tier_pace(1000), 1.0 + BOSS_TIER_PACE_MAX);
    //what it warns of kills more of the army, and kills it harder, the further round it is
    let (early, late) = (blast(MIDDLE, 100.0, 1.6, 1.0, 0.4, 1000, 0, RED, Cue::Slam), blast(MIDDLE, 100.0, 1.6, 1.0, 0.4, 1000, 3, RED, Cue::Slam));
    assert!(late.damage > early.damage * 1.5 && late.victims > early.victims, "{} and {} against {} and {}", late.damage, late.victims, early.damage, early.victims);
    assert_eq!(early.wall_damage, late.wall_damage, "the walls are not what the rounds are about");
    assert_eq!(blast(MIDDLE, 100.0, 1.6, 1.0, 0.9, 1000, 20, RED, Cue::Slam).victims, 1000, "never more than the whole army");
}

#[test]
fn the_circles_a_boss_warns_of_kill_an_army_that_stays_in_them_however_it_was_toughened() {
    let mut tough = Upgrades::default();
    for _ in 0..3 {
        tough.take(Relic::IronResolve);
    }
    for def in BOSSES.iter() {
        let (mut boss, mut state) = (Enemy::boss(def, 400, 0, MIDDLE), BossState { def, tier: 0, phase: 0, timers: vec![], shifting: 0.0, last_stand: false, family: 0, family_max: 1.0, generation: 0, facing: Vec2::X, anchor_left: 0.0 });
        for pattern in [Pattern::Slam, Pattern::Nova, Pattern::Pull] {
            let (mut fx, mut born, mut shots) = (Effects::default(), Vec::new(), Vec::new());
            let units: Vec<Unit> = (0..400).map(|i| unit_on(MIDDLE + Vec2::new((i % 20) as f32 * 10.0, (i / 20) as f32 * 10.0))).collect();
            cast_pattern(&mut boss, &mut state, pattern, &units, MIDDLE, 0, &mut born, &mut shots, &mut fx);
            assert!(fx.hazards[0].damage * tough.toughness >= UNIT_HEALTH, "{} {pattern:?} does {} to a unit with {} toughness", def.name, fx.hazards[0].damage, tough.toughness);
        }
    }
}

#[test]
fn after_a_circle_that_kills_the_next_one_waits_for_the_army_to_get_back() {
    assert!([Pattern::Slam, Pattern::Meteors, Pattern::Nova, Pattern::Pull].into_iter().all(is_blast));
    assert!([Pattern::Brood, Pattern::Ring, Pattern::Burn, Pattern::Charge, Pattern::Fortify, Pattern::Volley].into_iter().all(|pattern| !is_blast(pattern)));
    //the iron warden slams and throws meteors, both due now
    let def = &BOSSES[1];
    let units: Vec<Unit> = (0..300).map(|i| unit_on(MIDDLE + Vec2::new((i % 20) as f32 * 10.0, (i / 20) as f32 * 10.0))).collect();
    let mut enemies = vec![Enemy::boss(def, 300, 0, MIDDLE)];
    enemies[0].boss.as_mut().unwrap().timers = vec![0.0, 0.0];
    let (mut shots, mut fx) = (Vec::new(), Effects::default());
    boss_act(&mut enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
    assert_eq!(fx.hazards.len(), 1, "one at a time");
    for timer in &enemies[0].boss.as_ref().unwrap().timers {
        assert!(*timer >= BOSS_BLAST_HOLD, "{timer}");
    }
}

#[test]
fn the_screen_warns_of_a_circle_about_to_kill_the_army_and_of_nothing_else() {
    let units: Vec<Unit> = (0..300).map(|i| unit_on(MIDDLE + Vec2::new((i % 20) as f32 * 10.0, (i / 20) as f32 * 10.0))).collect();
    let away = MIDDLE + Vec2::new(5000.0, 0.0);
    let slam = || blast(MIDDLE, 600.0, 1.6, 1.0, 0.4, 300, 0, RED, Cue::Slam);
    let warning = blast_warning(&[slam()], away, &units).expect("the army is in it");
    assert_eq!(warning.units, victims_of(300, 0.4));
    assert!(!warning.player_inside && (warning.seconds - 1.6).abs() < 1e-3);
    //it is the player's own place that matters most
    assert!(blast_warning(&[slam()], MIDDLE, &[]).is_some_and(|warning| warning.player_inside), "the player is in it and nobody else");
    //the worst of several is the one it tells of
    let small = blast(MIDDLE, 30.0, 1.6, 1.0, 0.4, 300, 0, RED, Cue::Slam);
    assert_eq!(blast_warning(&[small, slam()], away, &units).map(|warning| warning.units), Some(victims_of(300, 0.4)));
    //nothing near it, a shell that lands in a moment, one that is over, one that does no harm by itself
    assert!(blast_warning(&[blast(MIDDLE + Vec2::new(9000.0, 0.0), 600.0, 1.6, 1.0, 0.4, 300, 0, RED, Cue::Slam)], away, &units).is_none());
    assert!(blast_warning(&[Hazard::new(MIDDLE, 600.0, 0.6, UNIT_HEALTH * 2.5, 100, RED, Cue::Explosion)], away, &units).is_none());
    let mut over = slam();
    over.armed = false;
    assert!(blast_warning(&[over], away, &units).is_none());
    assert!(blast_warning(&[Hazard::new(MIDDLE, 600.0, 1.6, 0.0, 0, RED, Cue::Explosion).burning(5.0, 30.0)], away, &units).is_none());
    //a few units in the circle of a big army are not worth interrupting for
    let big: Vec<Unit> = (0..3000).map(|i| unit_on(MIDDLE + Vec2::new(4000.0 + (i % 60) as f32 * 10.0, (i / 60) as f32 * 10.0))).collect();
    assert!(blast_warning(&[blast(MIDDLE, 600.0, 1.6, 1.0, 0.4, 3000, 0, RED, Cue::Slam)], away, &big).is_none());
}

#[test]
fn the_warning_of_a_circle_is_long_enough_to_notice_it_press_recall_and_get_out_at_any_army_size() {
    const REACTION: f32 = 0.7; //seconds to see it and move
    for def in BOSSES.iter().chain([&FINAL_BOSS]) {
        for army in [def.min_army, 1000, 4000, 15000] {
            let units: Vec<Unit> = (0..10).map(|i| unit_on(MIDDLE + Vec2::new(i as f32, 0.0))).collect();
            let (mut boss, mut state) = (Enemy::boss(def, army, 0, MIDDLE), BossState { def, tier: 0, phase: 0, timers: vec![], shifting: 0.0, last_stand: false, family: 0, family_max: 1.0, generation: 0, facing: Vec2::X, anchor_left: 0.0 });
            //the slowest unit, at the speed the army this size runs
            let speed = UNIT_MIN_SPEED * speed_multiplier(army);
            for pattern in [Pattern::Slam, Pattern::Nova] {
                let (mut fx, mut born, mut shots) = (Effects::default(), Vec::new(), Vec::new());
                cast_pattern(&mut boss, &mut state, pattern, &units, MIDDLE, 0, &mut born, &mut shots, &mut fx);
                let circle = &fx.hazards[0];
                //the army fights at the boss's edge, and has to get from there to the edge of the circle
                let way_out = (circle.radius - boss.size / 2.0).max(0.0);
                assert!(circle.warn >= REACTION + way_out / speed, "{} {pattern:?} at {army}: {:.1}s to cover {way_out:.0} px at {speed:.0} px/s", def.name, circle.warn);
            }
        }
    }
    //and the longest circles still leave the army a chance to fight between them
    assert!(BOSS_WARN_NOVA + BOSS_WARN_PER_1000_PX * 2.0 < BOSS_BLAST_HOLD);
}

// ---------- a boss can't be hurt past a change it has yet to make ----------

//the changes a boss has to make before it can die: each phase after the first, each split, and its last stand
fn changes_of(def: &BossDef) -> usize {
    def.phases.len() - 1 + if def.splits { SPLIT_AT.len() } else { 0 } + 1
}

#[test]
fn no_blow_however_big_kills_a_boss_or_skips_a_change_it_has_yet_to_make() {
    for def in BOSSES.iter().chain([&FINAL_BOSS]) {
        let units: Vec<Unit> = (0..200).map(|i| unit_on(MIDDLE + Vec2::new(i as f32, 0.0))).collect();
        let mut enemies = vec![Enemy::boss(def, def.min_army.max(300), 0, MIDDLE + Vec2::new(3000.0, 0.0))];
        let (mut shots, mut fx) = (Vec::new(), Effects::default());
        let progress = |boss: &Enemy| {
            let state = boss.boss.as_ref().unwrap();
            state.phase + state.generation as usize + state.last_stand as usize
        };
        let mut blows = 0;
        loop {
            blows += 1;
            assert!(blows <= 3 * changes_of(def), "{}: it never stopped being held", def.name);
            let nothing_left = enemies[0].boss_gate() == f32::NEG_INFINITY;
            let before = progress(&enemies[0]);
            //a blow of a million times what it has
            let blow = enemies[0].max_health * 1.0e6;
            enemies[0].take_damage(blow);
            if nothing_left {
                assert!(enemies[0].health <= 0.0, "{}: with every change made it can be killed", def.name);
                break;
            }
            assert!(enemies[0].health > 0.0, "{}: killed with {} of {} changes made", def.name, before, changes_of(def));
            //more blows in the same frame do nothing: it is held where it is until it has made the change
            let held = enemies[0].health;
            let again = enemies[0].max_health;
            assert_eq!(enemies[0].take_damage(again), 0.0, "{}: still hurt at the gate", def.name);
            assert_eq!(enemies[0].health, held);
            boss_act(&mut enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
            assert!(progress(&enemies[0]) > before, "{}: no change after the blow", def.name);
            //it is unhurt for the pause (and the next, when two changes come at one health), then the blows start again
            for _ in 0..((PHASE_SHIFT_SECONDS + SPLIT_PAUSE + 1.0) * 100.0) as usize {
                boss_act(&mut enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
            }
            fx = Effects::default();
            shots.clear();
        }
        assert_eq!(progress(&enemies[0]), changes_of(def), "{}", def.name);
    }
}

#[test]
fn what_a_boss_is_held_from_is_not_counted_as_damage_done() {
    let mut boss = Enemy::boss(&BOSSES[0], 500, 0, MIDDLE);
    let before = boss.health;
    let dealt = boss.take_damage(boss.max_health * 10.0);
    assert!((dealt - (before - boss.health)).abs() < 1.0, "{dealt} dealt against {} lost", before - boss.health);
    assert!(boss.health / boss.max_health < BOSSES[0].phases[1].below && boss.health / boss.max_health > BOSSES[0].phases[1].below * 0.99);
    //an ordinary enemy is not held by anything
    let mut grunt = Enemy::new(MIDDLE, kind_named("Grunt"), 1.0);
    let dealt = grunt.take_damage(grunt.max_health * 10.0);
    assert_eq!(dealt, grunt.max_health * 10.0);
    assert!(grunt.health < 0.0);
}

#[test]
fn a_boss_that_is_already_past_a_change_is_not_healed_by_the_gate_and_just_makes_it() {
    let units: Vec<Unit> = (0..100).map(|i| unit_on(MIDDLE + Vec2::new(i as f32, 0.0))).collect();
    let mut enemies = vec![Enemy::boss(&BOSSES[2], 400, 0, MIDDLE + Vec2::new(3000.0, 0.0))];
    enemies[0].health = enemies[0].max_health * 0.05;
    let held = enemies[0].health;
    assert_eq!(enemies[0].take_damage(1.0e9), 0.0);
    assert_eq!(enemies[0].health, held);
    let (mut shots, mut fx) = (Vec::new(), Effects::default());
    boss_act(&mut enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
    let state = enemies[0].boss.as_ref().unwrap();
    assert_eq!(state.phase, BOSSES[2].phases.len() - 1, "it goes straight to the last phase it is in");
}

#[test]
fn the_lord_of_a_citadel_that_is_hurt_while_it_waits_still_changes_but_attacks_no_one() {
    let units: Vec<Unit> = (0..100).map(|i| unit_on(MIDDLE + Vec2::new(i as f32, 0.0))).collect();
    let mut lord = Enemy::boss(&FINAL_BOSS, 5000, 0, MIDDLE + Vec2::new(3000.0, 0.0));
    lord.site = Some(7);
    lord.chasing = false;
    let mut enemies = vec![lord];
    let blow = enemies[0].max_health * 10.0;
    enemies[0].take_damage(blow);
    assert!(enemies[0].health > 0.0, "held at the next phase");
    let (mut shots, mut fx) = (Vec::new(), Effects::default());
    for _ in 0..((PHASE_SHIFT_SECONDS + 20.0) * 100.0) as usize {
        boss_act(&mut enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
    }
    assert_eq!(enemies[0].boss.as_ref().unwrap().phase, 1);
    assert_eq!(enemies[0].boss.as_ref().unwrap().shifting, 0.0, "the pause ran out while it waited");
    assert_eq!(fx.hazards.len(), 1, "only the pulse of the phase, none of its attacks");
    assert!(shots.is_empty() && enemies.len() == 1);
}

#[test]
fn a_boss_is_made_rounds_further_on_by_a_harder_difficulty() {
    for def in BOSSES.iter() {
        let made = |tier: u32| Enemy::boss(def, 800, tier, MIDDLE);
        let mut hardened = made(1);
        hardened.harden(2);
        let later = made(3);
        assert!((hardened.max_health / later.max_health - 1.0).abs() < 1e-4, "{}", def.name);
        assert!((hardened.health / later.health - 1.0).abs() < 1e-4);
        assert!((hardened.attack / later.attack - 1.0).abs() < 1e-4);
        assert!((hardened.speed / later.speed - 1.0).abs() < 1e-4);
        assert!((hardened.max_shield - later.max_shield).abs() <= later.max_shield * 1e-4);
        let state = hardened.boss.as_ref().unwrap();
        assert_eq!((state.tier, state.family_max), (3, hardened.max_health));
    }
    //something that is not a boss is left alone
    let mut grunt = Enemy::new(MIDDLE, kind_named("Grunt"), 1.0);
    let before = (grunt.max_health, grunt.attack);
    grunt.harden(3);
    assert_eq!((grunt.max_health, grunt.attack), before);
}
