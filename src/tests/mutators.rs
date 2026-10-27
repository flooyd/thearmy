//the rules of the Insane ladder (see Mutator), each against the same thing played on Brutal

use super::*;

const BRUTAL: Difficulty = Difficulty::Brutal;

#[test]
fn veterans_makes_elites_twice_as_common_and_gives_every_one_a_second_affix() {
    play_on(BRUTAL);
    let brutal = elite_chance(5000);
    play_on(Difficulty::Insane(1));
    assert!((elite_chance(5000) - brutal * VETERANS_ELITES).abs() < 1e-6, "{} against {brutal}", elite_chance(5000));
    assert_eq!(elite_chance(10), 0.0, "still none for the smallest army");
    let elite = || {
        let mut enemy = Enemy::new(MIDDLE, kind_named("Grunt"), 1.0);
        enemy.make_elite(Modifier::Swift);
        enemy
    };
    let affixes = |enemy: &Enemy| AFFIXES.iter().filter(|&&affix| enemy.has(affix)).count();
    play_on(BRUTAL);
    let mut plain = elite();
    plain.roll_affixes(0.0);
    assert_eq!(affixes(&plain), 1, "on Brutal a chance of nothing gives nothing more");
    play_on(Difficulty::Insane(1));
    let mut veteran = elite();
    veteran.roll_affixes(0.0);
    assert_eq!(affixes(&veteran), 2, "a veteran has two whatever the chance");
    let mut grunt = Enemy::new(MIDDLE, kind_named("Grunt"), 1.0);
    grunt.roll_affixes(1.0);
    assert_eq!(affixes(&grunt), 0, "and an enemy that isn't an elite none");
}

//a boss of the second kind (its first phase slams, throws meteors and rushes) with the first of its attacks due, or with `phase` set and its first two due
fn boss_ready(phase: usize) -> Vec<Enemy> {
    let mut boss = Enemy::boss(&BOSSES[1], 400, 0, MIDDLE - Vec2::new(600.0, 0.0));
    if let Some(state) = boss.boss.as_mut() {
        state.phase = phase;
        state.timers = BOSSES[1].phases[phase].casts.iter().enumerate().map(|(k, _)| if k == 0 { 0.0 } else { 5.0 + k as f32 }).collect();
    }
    vec![boss]
}

fn act(enemies: &mut Vec<Enemy>) -> Effects {
    let units: Vec<Unit> = (0..400).map(|i| unit_on(MIDDLE + Vec2::new((i % 20) as f32 * 30.0, (i / 20) as f32 * 30.0))).collect();
    let (mut fx, mut shots) = (Effects::default(), Vec::new());
    boss_act(enemies, &units, MIDDLE, 0.01, &mut shots, &mut fx);
    fx
}

#[test]
fn faster_fate_shortens_a_bosss_warnings_and_the_time_between_its_attacks() {
    play_on(BRUTAL);
    let mut brutal = boss_ready(0);
    let slow = act(&mut brutal);
    play_on(Difficulty::Insane(2));
    let mut fated = boss_ready(0);
    let fast = act(&mut fated);
    assert!(!slow.hazards.is_empty() && slow.hazards.len() == fast.hazards.len(), "it slams either way");
    for (slow, fast) in slow.hazards.iter().zip(&fast.hazards) {
        assert!((fast.warn - slow.warn * FATE_WARN).abs() < 1e-3, "{} against {}", fast.warn, slow.warn);
        assert_eq!(fast.warn_total, fast.warn);
    }
    //(the slam is a circle that kills what it lands on, so the next waits for the army to get back: less long with Faster Fate)
    let timer = |enemies: &[Enemy]| enemies[0].boss.as_ref().map_or(0.0, |state| state.timers[0]);
    assert!(timer(&brutal) >= BOSS_BLAST_HOLD - 1e-3, "{}", timer(&brutal));
    assert!(timer(&fated) <= BOSS_BLAST_HOLD * FATE_PACE + 1e-3, "{}", timer(&fated));
}

#[test]
fn wrath_has_a_boss_in_its_last_phase_cast_two_at_once() {
    let last = BOSSES[1].phases.len() - 1;
    let casts = BOSSES[1].phases[last].casts;
    assert!(casts.len() >= 2);
    //(its last phase rushes first and then has a piston, which puts down a box of its own)
    assert!(matches!((casts[0].pattern, casts[1].pattern), (Pattern::ShieldRush, Pattern::Piston)));
    play_on(BRUTAL);
    let rush = act(&mut boss_ready(last)).hazards.len();
    play_on(Difficulty::Insane(5));
    let both = act(&mut boss_ready(last)).hazards;
    assert!(both.len() > rush, "with Wrath the piston comes with the rush: {} against {rush}", both.len());
    assert!(both.iter().any(|hazard| matches!(hazard.shape, Shape::Rect { .. })));
    //(not before the last phase: the first slams, and the meteors wait)
    let early = act(&mut boss_ready(0)).hazards.len();
    play_on(BRUTAL);
    assert_eq!(act(&mut boss_ready(0)).hazards.len(), early);
}

#[test]
fn wrath_enrages_an_enemy_brought_low_but_not_a_boss() {
    let hurt = |share: f32| {
        let mut enemy = Enemy::new(MIDDLE, kind_named("Grunt"), 1.0);
        enemy.health = enemy.max_health * share;
        enemy
    };
    let mut boss = Enemy::boss(&BOSSES[0], 400, 0, MIDDLE);
    boss.health = boss.max_health * 0.1;
    play_on(BRUTAL);
    let mut enemies = vec![hurt(0.2)];
    enrage_enemies(&mut enemies);
    assert!(!enemies[0].has(Affix::Frenzied));
    play_on(Difficulty::Insane(5));
    let mut enemies = vec![hurt(0.2), hurt(0.5), boss];
    let attack = enemies[0].attack;
    enrage_enemies(&mut enemies);
    assert!(enemies[0].has(Affix::Frenzied) && enemies[0].attack > attack, "it hits harder");
    assert!(!enemies[1].has(Affix::Frenzied), "one with half its health left is not");
    assert!(!enemies[2].has(Affix::Frenzied), "nor is a boss");
    let again = enemies[0].attack;
    enrage_enemies(&mut enemies);
    assert_eq!(enemies[0].attack, again, "and it is enraged once");
}

#[test]
fn crossfire_has_shooters_fire_faster_and_snipers_and_lancers_in_the_hordes() {
    let units: Vec<Unit> = (0..50).map(|i| unit_on(MIDDLE + Vec2::new(300.0 + i as f32, 0.0))).collect();
    let fired = |difficulty: Difficulty| {
        play_on(difficulty);
        let mut enemies = vec![Enemy::new(MIDDLE, kind_named("Lancer"), 1.0)];
        let (mut shots, mut fx, mut volleys) = (Vec::new(), Effects::default(), 0);
        for _ in 0..1200 {
            let before = shots.len();
            shoot_enemies(&mut enemies, &units, &mut shots, 0.05, &mut fx);
            volleys += (shots.len() > before) as usize;
            shots.clear();
        }
        volleys as f32
    };
    let (brutal, crossfire) = (fired(BRUTAL), fired(Difficulty::Insane(3)));
    assert!(crossfire > brutal * 1.3, "{crossfire} volleys against {brutal}");
    let def = horde_named("Swarm Tide").unwrap();
    let lanes = |horde: &[Enemy]| horde.iter().filter(|enemy| matches!(enemy.name, "Sniper" | "Lancer")).count();
    play_on(BRUTAL);
    assert_eq!(lanes(&make_horde(def, MIDDLE, 4000, 4000)), 0);
    play_on(Difficulty::Insane(3));
    assert!(lanes(&make_horde(def, MIDDLE, 4000, 4000)) > 0);
}

#[test]
fn hunters_brings_more_hexers_and_executioners_into_the_wild() {
    let share = |difficulty: Difficulty| {
        play_on(difficulty);
        (0..6000).filter(|_| matches!(random_kind(5000).name, "Hexer" | "Executioner")).count() as f32 / 6000.0
    };
    let (brutal, hunters) = (share(BRUTAL), share(Difficulty::Insane(4)));
    assert!(hunters > brutal * 2.0, "{hunters} against {brutal}");
}

#[test]
fn famine_drops_a_quarter_fewer_babies() {
    let mut enemy = Enemy::new(MIDDLE, kind_named("Grunt"), 1.0);
    enemy.babies = 100;
    let dropped = |difficulty: Difficulty| {
        play_on(difficulty);
        let mut babies = Vec::new();
        drop_babies(&enemy, &mut babies, 10, 1.0, 100_000);
        babies.len()
    };
    assert_eq!(dropped(BRUTAL), 100);
    assert_eq!(dropped(Difficulty::Insane(6)), 75);
}

fn burning() -> Hazard {
    let mut hazard = Hazard::new(MIDDLE, 200.0, 2.0, 50.0, 30, RED, Cue::Explosion);
    hazard.linger = 4.0;
    hazard.burn = 10.0;
    hazard
}

#[test]
fn ruin_has_burning_ground_burn_twice_as_long() {
    let laid = |difficulty: Difficulty| {
        play_on(difficulty);
        let mut hazards = Vec::new();
        lay_hazards(&mut hazards, &mut vec![burning(), Hazard::new(MIDDLE, 100.0, 1.0, 50.0, 10, RED, Cue::Explosion)]);
        hazards
    };
    let brutal = laid(BRUTAL);
    assert_eq!((brutal[0].linger, brutal.len()), (4.0, 2));
    let ruin = laid(Difficulty::Insane(7));
    assert_eq!(ruin[0].linger, 4.0 * RUIN_BURN);
    assert_eq!(ruin[1].linger, 0.0, "what does not burn is left as it was");
}

#[test]
fn echo_lays_every_warning_again_at_half_the_size_after_it_goes_off() {
    let mut mine = Hazard::new(MIDDLE, 90.0, MINE_LIFE, 50.0, 10, RED, Cue::Explosion).mined();
    mine.warn = MINE_LIFE;
    let beam = Hazard { shape: Shape::Line { dir: Vec2::X, length: 800.0, width: 60.0 }, ..Hazard::new(MIDDLE, 0.0, 1.5, 50.0, 10, RED, Cue::Slam) };
    play_on(Difficulty::Insane(8));
    let mut hazards = Vec::new();
    lay_hazards(&mut hazards, &mut vec![burning(), mine, beam]);
    assert_eq!(hazards.len(), 5, "the circle and the beam come again, the mine does not");
    let echo = &hazards[3];
    assert_eq!((echo.position, echo.radius, echo.warn, echo.warn_total), (MIDDLE, 200.0 * ECHO_SIZE, 2.0 + ECHO_DELAY, 2.0 + ECHO_DELAY));
    assert_eq!(echo.victims, 15);
    assert!(matches!(hazards[4].shape, Shape::Line { length, width, .. } if length == 400.0 && width == 30.0));
    play_on(Difficulty::Insane(7));
    let mut hazards = Vec::new();
    lay_hazards(&mut hazards, &mut vec![burning()]);
    assert_eq!(hazards.len(), 1, "not below the tier that adds it");
}
