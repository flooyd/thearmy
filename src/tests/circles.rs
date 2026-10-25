//the circles a boss puts on the ground: a colour for each kind, the gauge that counts down to the next, the warning that names the one the
//army is standing in, and the damage readout

use super::*;

fn army_near(at: Vec2, count: usize) -> Vec<Unit> {
    (0..count).map(|i| unit_on(at + Vec2::new((i % 20) as f32 * 30.0, (i / 20) as f32 * 30.0))).collect()
}

fn every_boss() -> impl Iterator<Item = &'static BossDef> {
    BOSSES.iter().chain(std::iter::once(&FINAL_BOSS))
}

fn state_of(def: &'static BossDef) -> BossState {
    Enemy::boss(def, 800, 0, MIDDLE).boss.expect("a boss has its state")
}

//the bosses' circle attacks as they are told apart: each pattern that puts a killing circle down has a kind, and no other has
#[test]
fn the_attacks_that_put_a_killing_circle_down_each_have_a_kind_of_circle_and_none_of_the_others_do() {
    let patterns = [Pattern::Slam, Pattern::Meteors, Pattern::Nova, Pattern::Brood, Pattern::Ring, Pattern::Burn, Pattern::Charge, Pattern::Pull, Pattern::Fortify, Pattern::Volley, Pattern::Piston, Pattern::GravityRing, Pattern::Beams, Pattern::Cleave, Pattern::AcidRain, Pattern::ShieldRush, Pattern::Whirlwind, Pattern::Leap, Pattern::Sweep, Pattern::BlackHole, Pattern::TwinSlam, Pattern::Surge, Pattern::Barrage, Pattern::Judgement];
    for pattern in patterns {
        assert_eq!(circle_of(pattern).is_some(), is_blast(pattern), "{pattern:?}");
    }
    assert_eq!(circle_of(Pattern::Slam), Some(BossCircle::Slam));
    assert_eq!(circle_of(Pattern::Meteors), Some(BossCircle::Meteor));
    assert_eq!(circle_of(Pattern::Nova), Some(BossCircle::Nova));
    assert_eq!(circle_of(Pattern::Pull), Some(BossCircle::Pull));
}

#[test]
fn every_kind_of_circle_has_a_name_a_colour_and_a_word_for_its_colour_of_its_own() {
    for (i, a) in CIRCLES.iter().enumerate() {
        assert!(!a.name().is_empty() && !a.advice().is_empty() && !a.colour_name().is_empty(), "{a:?}");
        assert_eq!(a.colour_name(), a.colour_name().to_uppercase(), "(it is read out in capitals)");
        for b in &CIRCLES[i + 1..] {
            assert_ne!(a.name(), b.name());
            assert_ne!(a.colour_name(), b.colour_name(), "{a:?} and {b:?}");
            let (c, d) = (a.color(), b.color());
            let apart = ((c.r - d.r).powi(2) + (c.g - d.g).powi(2) + (c.b - d.b).powi(2)).sqrt();
            assert!(apart > 0.25, "{a:?} and {b:?} are too alike to tell apart: {apart}");
        }
    }
    //(and the eighteen are all there is)
    assert_eq!(CIRCLES.len(), 18);
}

#[test]
fn the_circle_an_attack_puts_down_is_marked_with_its_kind_and_the_attacks_that_are_not_circles_do_not_mark_one() {
    let units = army_near(MIDDLE, 200);
    for pattern in [Pattern::Slam, Pattern::Meteors, Pattern::Nova, Pattern::Pull, Pattern::Burn, Pattern::Ring, Pattern::Volley, Pattern::Brood, Pattern::Charge, Pattern::Fortify, Pattern::Piston, Pattern::GravityRing, Pattern::Beams, Pattern::Cleave, Pattern::AcidRain, Pattern::ShieldRush, Pattern::Whirlwind, Pattern::Leap, Pattern::Sweep, Pattern::BlackHole, Pattern::TwinSlam, Pattern::Surge, Pattern::Barrage, Pattern::Judgement] {
        let mut boss = Enemy::boss(&BOSSES[0], 800, 0, MIDDLE + Vec2::new(0.0, 900.0));
        let mut state = boss.boss.take().unwrap();
        let (mut born, mut shots, mut fx) = (Vec::new(), Vec::new(), Effects::default());
        cast_pattern(&mut boss, &mut state, pattern, &units, MIDDLE, 0, &mut born, &mut shots, &mut fx);
        match circle_of(pattern) {
            Some(circle) => {
                assert!(!fx.hazards.is_empty(), "{pattern:?} puts a circle down");
                assert!(fx.hazards.iter().all(|hazard| hazard.circle == Some(circle)), "{pattern:?}");
            }
            None => assert!(fx.hazards.iter().all(|hazard| hazard.circle.is_none()), "{pattern:?} is not one of the circles"),
        }
    }
}

#[test]
fn the_pulses_as_a_boss_changes_phase_and_tears_in_two_have_a_kind_of_their_own() {
    let units = army_near(MIDDLE, 200);
    let def = BOSSES.iter().find(|def| def.splits).expect("a boss that splits");
    let mut enemies = vec![Enemy::boss(def, 800, 0, MIDDLE + Vec2::new(3000.0, 0.0))];
    enemies[0].health = enemies[0].max_health * 0.5;
    let (mut fx, mut shots) = (Effects::default(), Vec::new());
    for _ in 0..((PHASE_SHIFT_SECONDS + 0.3) / 0.01) as usize {
        boss_act(&mut enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
    }
    let kinds: Vec<Option<BossCircle>> = fx.hazards.iter().map(|hazard| hazard.circle).collect();
    assert!(kinds.contains(&Some(BossCircle::Shift)), "{kinds:?}");
    assert!(kinds.contains(&Some(BossCircle::Split)), "{kinds:?}");
}

#[test]
fn the_gauge_names_the_circle_that_is_due_first_and_leaves_out_the_attacks_that_are_not_circles() {
    let (mut with, mut without) = (0, 0);
    for def in every_boss() {
        for (p, phase) in def.phases.iter().enumerate() {
            let mut state = state_of(def);
            state.phase = p;
            //the circles' timers read 5 seconds and a little more for each, the others' are nearly up: the gauge ignores those
            state.timers = phase.casts.iter().enumerate().map(|(k, cast)| if circle_of(cast.pattern).is_some() { 5.0 + k as f32 } else { 0.1 }).collect();
            let first = phase.casts.iter().position(|cast| circle_of(cast.pattern).is_some());
            match (state.next_circle(), first) {
                (None, None) => without += 1,
                (Some(next), Some(k)) => {
                    with += 1;
                    assert_eq!(Some(next.circle), circle_of(phase.casts[k].pattern), "{} phase {p}", def.name);
                    assert!((next.seconds - (5.0 + k as f32)).abs() < 1.0e-4, "{}", next.seconds);
                    assert!((0.0..=1.0).contains(&next.share));
                }
                (got, want) => panic!("{} phase {p}: {:?} against cast {want:?}", def.name, got.map(|next| next.circle)),
            }
        }
    }
    assert!(with > 10, "most phases have a circle: {with}");
    let _ = without;
}

#[test]
fn the_gauge_fills_as_the_circle_comes_due_and_is_full_when_it_is() {
    let (def, phase, cast) = every_boss()
        .flat_map(|def| def.phases.iter().enumerate().flat_map(move |(p, phase)| phase.casts.iter().enumerate().map(move |(k, cast)| (def, p, k, cast))))
        .find(|(_, _, _, cast)| circle_of(cast.pattern).is_some())
        .map(|(def, p, k, cast)| (def, (p, k), cast))
        .expect("a boss with a circle");
    let (p, k) = phase;
    let mut state = state_of(def);
    state.phase = p;
    let whole = (cast.every).max(BOSS_BLAST_HOLD);
    let mut last = -1.0;
    for step in 0..=20 {
        let left = whole * (1.0 - step as f32 / 20.0);
        state.timers = vec![1.0e6; def.phases[p].casts.len()];
        state.timers[k] = left;
        let next = state.next_circle().expect("it is the soonest");
        assert!(next.share >= last, "the gauge only fills: {} after {last}", next.share);
        last = next.share;
        if step == 0 {
            assert!(next.share < 0.05, "empty just after the last");
        }
    }
    assert!(last > 0.99, "and full when it is due");
    //a circle that is overdue (the boss was busy) is still full, and its seconds never go below nothing
    state.timers[k] = -3.0;
    let overdue = state.next_circle().unwrap();
    assert_eq!((overdue.seconds, overdue.share), (0.0, 1.0));
}

#[test]
fn the_gauge_counts_faster_in_the_last_stand_and_says_nothing_between_phases() {
    let def = every_boss().find(|def| def.phases[0].casts.iter().any(|cast| circle_of(cast.pattern).is_some())).expect("a boss with a circle in its first phase");
    let mut state = state_of(def);
    state.timers = def.phases[0].casts.iter().map(|cast| if circle_of(cast.pattern).is_some() { 8.0 } else { 1.0e6 }).collect();
    let normal = state.next_circle().unwrap().seconds;
    assert!((normal - 8.0).abs() < 1.0e-4);
    state.last_stand = true;
    let rushed = state.next_circle().unwrap().seconds;
    assert!((rushed - 8.0 / LAST_STAND_RATE).abs() < 1.0e-3, "{rushed}");
    state.shifting = 1.0;
    assert!(state.next_circle().is_none(), "a boss that is changing phase puts none down");
}

#[test]
fn the_warning_names_the_circle_the_army_is_in_and_a_plain_blast_has_no_kind() {
    let units = army_near(MIDDLE, 10);
    let far_player = MIDDLE + Vec2::new(5000.0, 0.0);
    let deadly = UNIT_HEALTH * 3.0;
    let plain = hazard_at(MIDDLE + Vec2::new(100.0, 100.0), 400.0, 3.0, deadly, 100);
    let warning = blast_warning(&[plain], far_player, &units).expect("the army is in it");
    assert_eq!((warning.circle, warning.player_inside), (None, false));
    for circle in CIRCLES {
        let hazard = hazard_at(MIDDLE + Vec2::new(100.0, 100.0), 400.0, 3.0, deadly, 100).circle(circle);
        let warning = blast_warning(&[hazard], far_player, &units).expect("the army is in it");
        assert_eq!(warning.circle, Some(circle));
        assert!(warning.units > 0 && warning.seconds > 0.0);
    }
    //of two, the one the player is in is the one that is named
    let near = hazard_at(far_player, 300.0, 3.0, deadly, 100).circle(BossCircle::Nova);
    let army = hazard_at(MIDDLE + Vec2::new(100.0, 100.0), 400.0, 3.0, deadly, 100).circle(BossCircle::Meteor);
    let warning = blast_warning(&[army, near], far_player, &units).unwrap();
    assert_eq!((warning.circle, warning.player_inside), (Some(BossCircle::Nova), true));
}

#[test]
fn the_damage_readout_is_one_number_unless_the_enemies_make_up_for_some_of_it() {
    let mut upgrades = Upgrades::default();
    assert_eq!(upgrades.damage_text(), "x1.0");
    upgrades.damage = 4.0;
    assert_eq!(upgrades.damage_text(), "x4.0", "(no difficulty answers it)");
    upgrades.foe_answer = 0.5;
    let hit = upgrades.hit();
    assert!(hit < upgrades.damage);
    assert_eq!(upgrades.damage_text(), format!("x{:.1} of x{:.1}", hit, 4.0));
    //what the enemies take is worth less than what the relics add up to, so the first number is the smaller
    upgrades.damage = 39.1;
    let text = upgrades.damage_text();
    let (hit_part, rest) = text.split_once(" of ").expect("two numbers");
    assert!(hit_part.trim_start_matches('x').parse::<f32>().unwrap() < rest.trim_start_matches('x').parse::<f32>().unwrap(), "{text}");
    //an answer that changes nothing (a damage of 1) is not shown as two numbers
    upgrades.damage = 1.0;
    assert_eq!(upgrades.damage_text(), "x1.0");
}
