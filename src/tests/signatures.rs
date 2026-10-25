//the bosses tuned up: tougher, an enraged last phase for three, cracks as they go, and a signature attack each; and the enemies that teach a rule each

use super::*;

use std::f32::consts::TAU;

fn army_near(at: Vec2, count: usize) -> Vec<Unit> {
    (0..count).map(|i| unit_on(at + Vec2::new((i % 20) as f32 * 30.0, (i / 20) as f32 * 30.0))).collect()
}

const ARMY_AT: Vec2 = Vec2::new(MIDDLE.x - 285.0, MIDDLE.y - 1200.0);

//what a boss puts down for an attack, with an army of 400 1200 pixels up from it: the boss after, what it put down, the rest, and how many it called up
fn cast(def: &'static BossDef, pattern: Pattern) -> (Enemy, Vec<Hazard>, Effects, usize) {
    let units = army_near(ARMY_AT, 400);
    let mut boss = Enemy::boss(def, 800, 0, MIDDLE);
    let mut state = boss.boss.take().unwrap();
    let (mut born, mut shots, mut fx) = (Vec::new(), Vec::new(), Effects::default());
    cast_pattern(&mut boss, &mut state, pattern, &units, ARMY_AT, 0, &mut born, &mut shots, &mut fx);
    boss.boss = Some(state);
    let hazards = std::mem::take(&mut fx.hazards);
    (boss, hazards, fx, born.len())
}

const NEW_ATTACKS: [Pattern; 9] = [Pattern::AcidRain, Pattern::ShieldRush, Pattern::Whirlwind, Pattern::Leap, Pattern::Sweep, Pattern::BlackHole, Pattern::TwinSlam, Pattern::Barrage, Pattern::Judgement];

#[test]
fn every_new_attack_is_warned_of_long_enough_and_is_marked_with_its_kind() {
    for pattern in NEW_ATTACKS {
        let (_, hazards, _, _) = cast(&BOSSES[0], pattern);
        let kind = circle_of(pattern).expect("it is one of the ones to get out of");
        assert!(!hazards.is_empty() && is_blast(pattern), "{pattern:?}");
        assert!(hazards.iter().all(|hazard| hazard.circle == Some(kind) && hazard.damage >= UNIT_HEALTH && hazard.warn_total >= BLAST_WARNING_MIN_SECONDS), "{pattern:?}");
    }
}

#[test]
fn acid_rain_falls_on_the_army_in_waves() {
    let (_, hazards, _, _) = cast(&BOSSES[0], Pattern::AcidRain);
    assert_eq!(hazards.len(), ACID_WAVES * ACID_DROPS);
    //(an army of 400 is too small for more)
    let mut times: Vec<f32> = hazards.iter().map(|hazard| hazard.warn).collect();
    times.dedup();
    assert_eq!(times.len(), ACID_WAVES, "{times:?}");
    assert!(hazards.iter().all(|hazard| hazard.group != 0 && hazard.group == hazards[0].group && hazard.position.y < MIDDLE.y - 200.0));
}

#[test]
fn the_shield_rush_shows_its_lane_and_charges_down_it() {
    let (mut boss, hazards, mut fx, _) = cast(&BOSSES[1], Pattern::ShieldRush);
    let Mode::Windup { dir, .. } = boss.mode else { panic!("it winds up") };
    assert!(dir.y < -0.9 && matches!(hazards[0].shape, Shape::Rect { .. }));
    let walls = walls_with(vec![]);
    for _ in 0..((BOSS_WARN_RUSH + CHARGE_TIME) * 60.0) as usize {
        boss.carry_on(1.0 / 60.0, speed_multiplier(400), &walls, &mut fx);
    }
    assert!(hazards[0].contains(boss.position), "it ends up in its lane");
    assert!(boss.position.distance(MIDDLE) > 300.0, "having charged");
}

#[test]
fn the_whirlwind_goes_off_a_step_further_along_its_path_each_time() {
    let (boss, hazards, _, _) = cast(&BOSSES[2], Pattern::Whirlwind);
    assert_eq!(hazards.len(), WHIRL_STEPS);
    for pair in hazards.windows(2) {
        assert!((pair[1].warn - pair[0].warn - WHIRL_GAP).abs() < 1e-4);
        assert!(pair[1].position.distance(boss.position) > pair[0].position.distance(boss.position));
    }
    assert!(hazards.iter().all(|hazard| hazard.group == hazards[0].group && hazard.group != 0));
}

#[test]
fn grakk_leaps_onto_the_spot_it_showed_as_the_circle_goes_off() {
    let (mut boss, hazards, mut fx, _) = cast(&BOSSES[2], Pattern::Leap);
    assert!(matches!(boss.mode, Mode::Windup { .. }));
    let spot = &hazards[0];
    assert!(!spot.contains(boss.position), "not where it stands");
    let walls = walls_with(vec![]);
    let mut waited = 0.0;
    while waited < spot.warn_total {
        boss.carry_on(1.0 / 60.0, speed_multiplier(400), &walls, &mut fx);
        waited += 1.0 / 60.0;
    }
    assert!(spot.contains(boss.position), "it lands in its circle, {} from the middle", boss.position.distance(spot.position));
}

#[test]
fn the_sweep_turns_round_the_eclipse_wedge_by_wedge_and_leaves_a_gap() {
    let (boss, hazards, _, _) = cast(&BOSSES[3], Pattern::Sweep);
    assert_eq!(hazards.len(), SWEEP_WEDGES - SWEEP_GAP_WEDGES);
    assert!(hazards.iter().all(|hazard| matches!(hazard.shape, Shape::Cone { .. }) && hazard.group == hazards[0].group));
    assert!(hazards.windows(2).all(|pair| pair[1].warn > pair[0].warn), "one after another");
    let around = boss.size / 2.0 + 500.0;
    let safe = (0..72).filter(|k| !hazards.iter().any(|hazard| hazard.contains(boss.position + Vec2::from_angle(*k as f32 * TAU / 72.0) * around))).count();
    let gap = 72 * SWEEP_GAP_WEDGES / SWEEP_WEDGES;
    assert!(safe + 2 >= gap && safe <= gap + 2, "{safe} of 72 spots round it are safe");
}

#[test]
fn the_black_hole_drags_the_army_to_a_spot_in_it_and_goes_off_there() {
    let (_, hazards, fx, _) = cast(&BOSSES[3], Pattern::BlackHole);
    assert_eq!((hazards.len(), fx.pulls.len()), (1, 1));
    assert_eq!(fx.pulls[0].position, hazards[0].position);
    assert!(hazards[0].position.distance(ARMY_AT) < 1000.0, "it is in the army");
}

#[test]
fn the_twin_slams_land_at_once_on_two_parts_of_the_army_far_apart() {
    let (_, hazards, _, _) = cast(&BOSSES[4], Pattern::TwinSlam);
    assert_eq!(hazards.len(), 2);
    assert!(hazards[0].group != 0 && hazards[0].group == hazards[1].group && hazards[0].warn == hazards[1].warn);
    assert!(hazards[0].position.distance(hazards[1].position) > 300.0);
}

#[test]
fn a_spawn_surge_calls_up_more_than_one_brood() {
    let (_, _, _, brood) = cast(&BOSSES[4], Pattern::Brood);
    let (_, hazards, _, surge) = cast(&BOSSES[4], Pattern::Surge);
    assert!(surge > brood && hazards.is_empty(), "{surge} against {brood}");
}

#[test]
fn the_barrage_lays_waves_of_lines_across_the_army() {
    let (_, hazards, _, _) = cast(&BOSSES[6], Pattern::Barrage);
    assert_eq!(hazards.len(), BARRAGE_WAVES * BARRAGE_LINES);
    assert!(hazards.iter().all(|hazard| matches!(hazard.shape, Shape::Rect { .. })));
    let mut times: Vec<f32> = hazards.iter().map(|hazard| hazard.warn).collect();
    times.dedup();
    assert_eq!(times.len(), BARRAGE_WAVES);
}

#[test]
fn the_sovereigns_judgement_is_four_in_a_set_order() {
    let (boss, hazards, _, _) = cast(&FINAL_BOSS, Pattern::Judgement);
    assert_eq!(hazards.len(), 5);
    let Shape::Ring { inner } = hazards[0].shape else { panic!("first a ring") };
    assert!(hazards[1].shape == Shape::Circle && (hazards[1].radius - inner).abs() < 1e-3, "then its middle");
    assert!(matches!(hazards[2].shape, Shape::Cone { .. }), "then a cleave");
    assert!(hazards[3].shape == Shape::Circle && hazards[4].shape == Shape::Circle && hazards[3].warn == hazards[4].warn, "then two slams");
    assert!(hazards[0].warn < hazards[1].warn && hazards[1].warn < hazards[2].warn && hazards[2].warn < hazards[3].warn);
    assert!(hazards[0].contains(boss.position + Vec2::new(inner + 50.0, 0.0)) && !hazards[0].contains(boss.position + Vec2::new(inner - 50.0, 0.0)));
}

#[test]
fn the_bosses_are_tougher_and_three_of_them_enrage_at_the_end() {
    for index in [1, 2, 3] {
        assert_eq!(BOSSES[index].phases.len(), 4, "{}", BOSSES[index].name);
        assert!(BOSSES[index].phases[3].below > LAST_STAND_AT && BOSSES[index].phases[3].below < 0.25 && BOSSES[index].phases[3].speed > 1.0, "an enraged last phase just before the last stand");
    }
    assert!(BOSSES.iter().all(|def| def.kill_seconds >= 5.5) && FINAL_BOSS.kill_seconds >= 11.0);
    assert_eq!(Difficulty::Brutal.boss_rounds(), 3);
    assert!(LAST_STAND_AT >= 0.18 && LAST_STAND_RATE > 1.6);
    //every attack in every phase is one a boss can do
    for def in BOSSES.iter().chain(std::iter::once(&FINAL_BOSS)) {
        for phase in def.phases {
            assert!(!phase.casts.is_empty(), "{}: {}", def.name, phase.name);
        }
        assert!(def.phases.windows(2).all(|pair| pair[1].below < pair[0].below), "{}", def.name);
    }
}

#[test]
fn a_boss_cracks_more_with_each_phase_and_most_in_its_last_stand() {
    let mut boss = Enemy::boss(&BOSSES[2], 800, 0, MIDDLE);
    let cracks = |boss: &Enemy| marks_of(boss).into_iter().map(|(name, _)| name).filter(|name| name.starts_with("Cracked")).collect::<Vec<_>>();
    assert!(cracks(&boss).is_empty());
    for (phase, last, want) in [(1, false, "Cracked I"), (2, false, "Cracked II"), (3, false, "Cracked III"), (2, true, "Cracked III")] {
        let state = boss.boss.as_mut().unwrap();
        state.phase = phase;
        state.last_stand = last;
        assert_eq!(cracks(&boss), vec![want]);
    }
    assert!(cracks(&kind_at("Brute", MIDDLE)).is_empty());
}

//one of a new kind, ready to act
fn ready(name: &str, at: Vec2) -> Enemy {
    let mut enemy = kind_at(name, at);
    enemy.timer = 0.0;
    enemy
}

#[test]
fn an_executioner_stands_still_and_winds_up_a_cleave_at_the_units_in_front_of_it() {
    let mut enemies = vec![ready("Executioner", MIDDLE)];
    let units = army_near(MIDDLE + Vec2::new(60.0, 0.0), 40);
    let mut fx = Effects::default();
    enemy_abilities(&mut enemies, &units, MIDDLE + Vec2::new(300.0, 0.0), 1.0 / 60.0, &mut fx);
    assert_eq!(fx.hazards.len(), 1);
    let cleave = &fx.hazards[0];
    assert!(matches!(cleave.shape, Shape::Cone { .. }) && cleave.warn_total >= 1.4 && cleave.victims == EXECUTIONER_VICTIMS);
    assert!(cleave.contains(Vec2::new(units[0].x, units[0].y)) && !cleave.contains(MIDDLE + Vec2::new(-100.0, 0.0)), "in front, not behind");
    assert!(matches!(enemies[0].mode, Mode::Cast { .. }), "it stands still to wind up");
}

#[test]
fn a_sniper_shows_a_long_thin_line_at_a_unit_in_its_range() {
    let mut enemies = vec![ready("Sniper", MIDDLE)];
    let units = army_near(MIDDLE + Vec2::new(1000.0, 0.0), 20);
    let mut fx = Effects::default();
    enemy_abilities(&mut enemies, &units, MIDDLE + Vec2::new(1100.0, 0.0), 1.0 / 60.0, &mut fx);
    assert_eq!(fx.hazards.len(), 1);
    let Shape::Line { dir, length, width } = fx.hazards[0].shape else { panic!("a line") };
    assert!(dir.x > 0.95 && length > 1000.0 && width < 60.0 && fx.hazards[0].warn_total >= 1.5);
    assert!(fx.hazards[0].contains(Vec2::new(units[0].x, units[0].y)));
}

#[test]
fn a_hexers_curse_makes_the_units_in_it_take_more_harm_until_they_leave_it() {
    let mut enemies = vec![ready("Hexer", MIDDLE)];
    let mut units = vec![unit_on(MIDDLE + Vec2::new(500.0, 0.0))];
    let mut fx = Effects::default();
    enemy_abilities(&mut enemies, &units, MIDDLE + Vec2::new(500.0, 0.0), 1.0 / 60.0, &mut fx);
    let mut hazards = std::mem::take(&mut fx.hazards);
    assert!(hazards.len() == 1 && hazards[0].hex && hazards[0].damage == 0.0);
    for _ in 0..3 {
        update_hazards(&mut hazards, &mut units, &Upgrades::default(), 1.0 / 60.0, &mut fx);
    }
    assert!(units[0].hexed > 0.0, "it is cursed");
    let mut plain = unit_on(MIDDLE);
    let (before, plain_before) = (units[0].health, plain.health);
    units[0].hurt(10.0);
    plain.hurt(10.0);
    assert!((before - units[0].health - (plain_before - plain.health) * (1.0 + HEX_EXTRA)).abs() < 1e-3);
    //out of it, the curse wears off
    units[0].x += 2000.0;
    for _ in 0..30 {
        update_hazards(&mut hazards, &mut units, &Upgrades::default(), 1.0 / 60.0, &mut fx);
        heal_units(&mut units, 1.0 / 60.0, &Upgrades::default());
    }
    assert_eq!(units[0].hexed, 0.0);
}

#[test]
fn a_blinker_flickers_and_is_soon_on_the_far_side_of_the_army() {
    let mut enemies = vec![kind_at("Blinker", MIDDLE + Vec2::new(-900.0, 0.0))];
    enemies[0].timer = 1.0;
    let units = army_near(MIDDLE, 100);
    let mut fx = Effects::default();
    for _ in 0..90 {
        enemy_abilities(&mut enemies, &units, MIDDLE, 1.0 / 60.0, &mut fx);
    }
    assert!(enemies[0].position.x > MIDDLE.x + 100.0, "it is behind the army: {}", enemies[0].position.x - MIDDLE.x);
    assert!(enemies[0].chasing && !fx.rings.is_empty(), "and the ground showed where it would be");
}

#[test]
fn a_sappers_mine_waits_for_a_unit_and_goes_off_a_moment_after_one_comes() {
    let mut enemies = vec![ready("Sapper", MIDDLE)];
    let mut fx = Effects::default();
    enemy_abilities(&mut enemies, &[], MIDDLE + Vec2::new(200.0, 0.0), 1.0 / 60.0, &mut fx);
    let mut hazards = std::mem::take(&mut fx.hazards);
    assert!(hazards.len() == 1 && hazards[0].mine);
    let mut units = vec![unit_on(MIDDLE + Vec2::new(600.0, 0.0))];
    for _ in 0..120 {
        update_hazards(&mut hazards, &mut units, &Upgrades::default(), 1.0 / 60.0, &mut fx);
    }
    assert!(hazards.len() == 1 && hazards[0].mine, "it waits");
    assert!(blast_warning(&hazards, MIDDLE, &units).is_none() && edge_marks(&hazards, MIDDLE + Vec2::new(5000.0, 0.0), 1.0, Vec2::new(1600.0, 900.0)).is_empty(), "and is not warned of");
    units[0].x = MIDDLE.x;
    update_hazards(&mut hazards, &mut units, &Upgrades::default(), 1.0 / 60.0, &mut fx);
    assert!(!hazards[0].mine && hazards[0].armed && hazards[0].warn <= MINE_FUSE, "its fuse burns");
    for _ in 0..40 {
        update_hazards(&mut hazards, &mut units, &Upgrades::default(), 1.0 / 60.0, &mut fx);
    }
    assert!(units[0].health <= 0.0, "and it goes off");
    //one that nothing comes near fades away
    let mut lonely = vec![Hazard::new(MIDDLE, MINE_RADIUS, MINE_LIFE, UNIT_HEALTH, 3, RED, Cue::Explosion).mined()];
    for _ in 0..((MINE_LIFE + 0.5) * 10.0) as usize {
        update_hazards(&mut lonely, &mut [], &Upgrades::default(), 0.1, &mut fx);
    }
    assert!(lonely.is_empty());
}

#[test]
fn an_overseer_marks_a_champion_in_reach_before_any_other_unit() {
    let mut enemies = vec![ready("Overseer", MIDDLE)];
    let mut units = army_near(MIDDLE + Vec2::new(300.0, 0.0), 20);
    let champion = MIDDLE + Vec2::new(900.0, 0.0);
    units.push(champion_unit(0, champion));
    let mut fx = Effects::default();
    enemy_abilities(&mut enemies, &units, MIDDLE + Vec2::new(400.0, 0.0), 1.0 / 60.0, &mut fx);
    assert_eq!(fx.hazards.len(), 1);
    assert!(fx.hazards[0].position.distance(champion) < 1.0 && fx.hazards[0].warn_total >= 2.0 && fx.hazards[0].damage > UNIT_HEALTH * 5.0);
}

#[test]
fn mines_and_curses_are_told_to_the_other_players() {
    for hazard in [Hazard::new(MIDDLE, 90.0, 10.0, UNIT_HEALTH, 3, RED, Cue::Explosion).mined(), Hazard::new(MIDDLE, 200.0, 0.0, 0.0, 0, RED, Cue::Summon).hexing(6.0)] {
        let mut out = Writer::new();
        write_hazard(&mut out, &hazard);
        let back = read_hazard(&mut Reader::new(&out.bytes)).expect("it reads");
        assert_eq!((back.mine, back.hex, back.linger), (hazard.mine, hazard.hex, hazard.linger));
    }
}

#[test]
fn the_new_kinds_are_drawn_and_come_only_once_the_army_is_big_enough() {
    for name in ["Executioner", "Sniper", "Hexer", "Blinker", "Sapper", "Overseer"] {
        let kind = kind_named(name);
        assert_eq!(kind.name, name);
        assert!(shape_named(name).is_some(), "{name} has a picture");
        assert!(kind.from_army >= 200 && kind.weight > 0.0, "{name}");
        assert!((0..200).all(|_| random_kind(kind.from_army - 1).name != name), "{name} is not met before");
    }
}
