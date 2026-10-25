//what is done to enemies: chill that builds to a freeze, the freeze that shatters, burning that spreads, a shock, poison, what a boss shrugs off, the pieces
//of a shatter and how it is told

use super::*;

//an enemy nothing kills by accident in a test, and that does no harm
fn post(at: Vec2) -> Enemy {
    let mut enemy = kind_at("Grunt", at);
    enemy.max_health = 1000.0;
    enemy.health = 1000.0;
    enemy.attack = 0.0;
    enemy
}

fn tick_for(enemies: &mut [Enemy], seconds: f32) {
    for _ in 0..(seconds * 60.0).round() as usize {
        tick_statuses(enemies, 1.0 / 60.0);
    }
}

#[test]
fn chill_builds_slows_and_at_full_freezes_and_a_thaw_takes_it_away() {
    let mut enemy = post(MIDDLE);
    chill(&mut enemy, CHILL_FULL * 0.5, FREEZE_SECONDS);
    assert_eq!(enemy.status.frozen, 0.0);
    let kept = speed_kept(&enemy);
    assert!(kept < 1.0 && kept > CHILL_SLOW, "half chilled, half as slowed: {kept}");
    chill(&mut enemy, CHILL_FULL, FREEZE_SECONDS);
    assert_eq!((enemy.status.frozen, enemy.status.chill), (FREEZE_SECONDS, CHILL_FULL));
    assert_eq!(enemy.attack_now(), 0.0, "frozen, it does nothing");
    let mut enemies = vec![enemy];
    tick_for(&mut enemies, FREEZE_SECONDS + 0.1);
    assert_eq!((enemies[0].status.frozen, enemies[0].status.chill), (0.0, 0.0), "thawed, and the chill gone with it");
    //chill that does not reach full ebbs away
    chill(&mut enemies[0], 40.0, FREEZE_SECONDS);
    tick_for(&mut enemies, 1.0);
    assert!((enemies[0].status.chill - (40.0 - CHILL_EBB)).abs() < 0.5, "{}", enemies[0].status.chill);
}

#[test]
fn a_boss_is_never_held_frozen_only_slowed_and_takes_less_of_it_all() {
    let mut boss = Enemy::boss(&BOSSES[0], 100, 0, MIDDLE);
    chill(&mut boss, 10.0, FREEZE_SECONDS);
    assert!((boss.status.chill - 10.0 * BOSS_CHILL).abs() < 1e-4);
    for _ in 0..100 {
        chill(&mut boss, 50.0, FREEZE_SECONDS);
    }
    freeze(&mut boss, FREEZE_SECONDS);
    assert_eq!(boss.status.frozen, 0.0, "a boss is never held");
    assert!((speed_kept(&boss) - FROZEN_BOSS_SLOW).abs() < 1e-4, "only slowed: {}", speed_kept(&boss));
    //and burns at a share
    let (mut grunt, mut boss) = (post(MIDDLE), Enemy::boss(&BOSSES[0], 100, 0, MIDDLE + Vec2::new(3000.0, 0.0)));
    boss.max_health = 1.0e9;
    boss.health = 1.0e9;
    ignite(&mut grunt, 1.0, 100.0);
    ignite(&mut boss, 1.0, 100.0);
    let mut enemies = vec![grunt, boss];
    tick_for(&mut enemies, 0.5);
    assert!((enemies[1].hurt / enemies[0].hurt - BOSS_CHILL).abs() < 0.05, "{} against {}", enemies[1].hurt, enemies[0].hurt);
}

#[test]
fn burning_hurts_over_time_catches_on_what_touches_it_and_dies_out() {
    let mut enemies = vec![post(MIDDLE), post(MIDDLE + Vec2::new(40.0, 0.0)), post(MIDDLE + Vec2::new(600.0, 0.0))];
    ignite(&mut enemies[0], BURN_SECONDS, 100.0);
    tick_for(&mut enemies, 0.6);
    assert!((enemies[0].hurt - 60.0).abs() < 2.0, "{}", enemies[0].hurt);
    assert!(enemies[1].status.burning > 0.0 && enemies[1].status.burn < 100.0, "it caught, weaker");
    assert_eq!(enemies[2].status.burning, 0.0, "not the one far off");
    tick_for(&mut enemies, BURN_SECONDS * 4.0);
    assert!(enemies.iter().all(|enemy| enemy.status.burning == 0.0), "and the fire dies out, however it is passed back and forth");
}

#[test]
fn a_shocked_enemy_takes_the_next_hit_from_a_power_harder_and_only_the_next() {
    let mut enemy = post(MIDDLE);
    shock(&mut enemy, SHOCK_SECONDS);
    strike(&mut enemy, 100.0, 0);
    assert!((enemy.hurt - 100.0 * SHOCK_BONUS).abs() < 1e-3);
    strike(&mut enemy, 100.0, 0);
    assert!((enemy.hurt - 100.0 * SHOCK_BONUS - 100.0).abs() < 1e-3);
    let mut enemies = vec![enemy];
    shock(&mut enemies[0], 1.0);
    tick_for(&mut enemies, 1.1);
    assert_eq!(enemies[0].status.shocked, 0.0, "it wears off");
}

#[test]
fn poison_stacks_hurts_the_big_ones_more_and_ebbs() {
    let mut small = post(MIDDLE);
    small.max_health = 100.0;
    small.health = 100.0;
    let mut big = post(MIDDLE + Vec2::new(2000.0, 0.0));
    big.max_health = 10_000.0;
    big.health = 10_000.0;
    poison(&mut small, 5.0);
    poison(&mut small, 5.0);
    poison(&mut big, 10.0);
    let mut enemies = vec![small, big];
    tick_for(&mut enemies, 1.0);
    assert!(enemies[1].hurt > enemies[0].hurt * 50.0, "a share of its health: {} and {}", enemies[0].hurt, enemies[1].hurt);
    assert!((enemies[0].status.poison - (10.0 - POISON_EBB)).abs() < 0.05);
    poison(&mut enemies[0], 1000.0);
    assert_eq!(enemies[0].status.poison, POISON_MOST);
}

#[test]
fn a_frozen_enemy_that_dies_shatters_in_a_burst_of_frost_that_hits_each_one_near_once() {
    let near = post(MIDDLE + Vec2::new(50.0, 0.0));
    let far = post(MIDDLE + Vec2::new(900.0, 0.0));
    //one that is not frozen just dies
    let mut dying = post(MIDDLE);
    dying.health = 0.0;
    let mut enemies = vec![dying, near.clone(), far.clone()];
    let mut fx = Effects::default();
    resolve_deaths(&mut enemies, &mut Vec::new(), 100, &Upgrades::default(), &mut fx);
    assert!(fx.shatters.is_empty() && enemies.iter().all(|enemy| enemy.hurt == 0.0));
    //a frozen one shatters: the ones near are hurt once and chilled, the one far off is not, and no new enemies are made
    let mut frozen = post(MIDDLE);
    freeze(&mut frozen, FREEZE_SECONDS);
    frozen.health = 0.0;
    let most = frozen.max_health;
    let mut enemies = vec![frozen, near, far];
    let mut fx = Effects::default();
    let destroyed = resolve_deaths(&mut enemies, &mut Vec::new(), 100, &Upgrades::default(), &mut fx);
    assert_eq!((destroyed, enemies.len(), fx.shatters.len()), (1, 2, 1));
    assert!(fx.shatters[0].frost);
    assert!((enemies[0].hurt - SHATTER_DAMAGE * most).abs() < 1e-2, "{}", enemies[0].hurt);
    assert!(enemies[0].status.chill > 0.0);
    assert_eq!((enemies[1].hurt, enemies[1].status.chill), (0.0, 0.0));
    assert!(fx.cues.contains(&Cue::Shatter));
}

#[test]
fn a_big_hit_shatters_a_frozen_enemy_and_a_frozen_crowd_goes_up_one_after_another() {
    let mut frozen = post(MIDDLE);
    freeze(&mut frozen, FREEZE_SECONDS);
    let most = frozen.max_health;
    strike(&mut frozen, SHATTER_HIT * most * 0.5, 0);
    assert!(frozen.health > 0.0, "a small hit does not");
    strike(&mut frozen, SHATTER_HIT * most, 0);
    assert_eq!(frozen.health, 0.0, "a big one does");
    //a row of frozen, weak enemies, each touching the next: the first going up takes the next, and so on
    let mut row: Vec<Enemy> = (0..6)
        .map(|i| {
            let mut enemy = post(MIDDLE + Vec2::new(i as f32 * 45.0, 0.0));
            enemy.max_health = 100.0;
            enemy.health = SHATTER_DAMAGE * 100.0 * 0.9;
            freeze(&mut enemy, FREEZE_SECONDS);
            enemy
        })
        .collect();
    row[0].health = 0.0;
    let mut bursts = 0;
    for _ in 0..10 {
        let mut fx = Effects::default();
        resolve_deaths(&mut row, &mut Vec::new(), 100, &Upgrades::default(), &mut fx);
        bursts += fx.shatters.len();
    }
    assert!(row.is_empty() && bursts == 6, "{} left after {bursts} bursts", row.len());
}

#[test]
fn a_shatter_breaks_into_the_same_pieces_for_the_same_seed_and_as_many_as_its_size_has_room_for() {
    let cells = shatter_cells(60.0, 7);
    assert_eq!(format!("{cells:?}"), format!("{:?}", shatter_cells(60.0, 7)), "the same on every machine");
    assert_ne!(format!("{cells:?}"), format!("{:?}", shatter_cells(60.0, 8)));
    for size in [10.0, 44.0, 130.0, 400.0] {
        let count = shatter_cells(size, 3).len();
        assert!((FEWEST_FRAGMENTS..=MOST_FRAGMENTS).contains(&count), "{size} px: {count}");
    }
    assert!(shatter_cells(30.0, 3).len() < shatter_cells(200.0, 3).len());
    //the pieces are the square, all of it and nothing more
    let area = |corners: &[Vec2]| (0..corners.len()).map(|k| corners[k].perp_dot(corners[(k + 1) % corners.len()])).sum::<f32>().abs() / 2.0;
    let whole: f32 = cells.iter().map(|(_, corners)| area(corners)).sum();
    assert!((whole - 60.0 * 60.0).abs() < 1.0, "{whole}");
    //and there are never more in the air than the most there can be
    let mut fragments = Vec::new();
    for seed in 0..100 {
        throw_fragments(&mut fragments, MIDDLE, 300.0, RED, seed, true);
    }
    assert_eq!(fragments.len(), MAX_FRAGMENTS);
    update_fragments(&mut fragments, FRAGMENT_LIFE * 2.0);
    assert!(fragments.is_empty(), "they fade");
}

#[test]
fn a_shatter_is_told_in_a_few_bytes_and_one_out_of_all_reason_is_not_believed() {
    let told = Event::Shatter { at: MIDDLE, size: 44.0, color: Color::from_rgba(255, 0, 0, 255), seed: 99, frost: true };
    let bytes = write_events(std::slice::from_ref(&told));
    assert_eq!(read_events(&bytes), Some(vec![told]));
    assert!(bytes.len() < 32);
    let wild = Event::Shatter { at: MIDDLE, size: 1.0e7, color: RED, seed: 1, frost: false };
    assert_eq!(read_events(&write_events(&[wild])), None);
}

#[test]
fn what_is_being_done_to_an_enemy_is_seen_by_the_other_players() {
    let mut enemy = post(MIDDLE);
    chill(&mut enemy, CHILL_FULL * 0.5, FREEZE_SECONDS);
    ignite(&mut enemy, 2.0, 10.0);
    shock(&mut enemy, 2.0);
    poison(&mut enemy, 3.0);
    enemy.status.slow = TAR_SLOW;
    let mut out = Writer::new();
    EnemyDyn::of(&enemy).write(&mut out);
    let mut seen = post(Vec2::ZERO);
    EnemyDyn::read(&mut Reader::new(&out.bytes)).expect("it reads").apply(&mut seen);
    assert!(seen.status.chill > 0.0 && seen.status.frozen == 0.0);
    assert!(seen.status.burning > 0.0 && seen.status.shocked > 0.0 && seen.status.poison > 0.0 && seen.status.slow < SLOW_SEEN);
    assert!(["Chilled", "Burning", "Shocked", "Poisoned", "Slowed"].iter().all(|name| marks_of(&seen).iter().any(|(mark, _)| mark == name)));
    freeze(&mut enemy, 1.0);
    let mut out = Writer::new();
    EnemyDyn::of(&enemy).write(&mut out);
    EnemyDyn::read(&mut Reader::new(&out.bytes)).unwrap().apply(&mut seen);
    assert!(seen.status.frozen > 0.0);
}

#[test]
fn the_frostweavers_aura_chills_and_its_pulse_freezes_what_it_has_chilled() {
    let mut units = vec![champion_unit(champion_named("Frostweaver").unwrap(), MIDDLE)];
    units[0].power = 5.0;
    let mut enemies = vec![post(MIDDLE + Vec2::new(300.0, 0.0)), post(MIDDLE + Vec2::new(1500.0, 0.0))];
    let mut fx = Effects::default();
    for _ in 0..60 {
        champion_acts(&mut units, &mut enemies, &mut Vec::new(), &Upgrades::default(), 1.0 / 60.0, &mut fx);
    }
    assert!(enemies[0].status.chill > 0.0 && enemies[0].status.frozen == 0.0, "the aura chills");
    assert_eq!(enemies[1].status.chill, 0.0, "not the one out of reach");
    units[0].power = 0.0;
    champion_acts(&mut units, &mut enemies, &mut Vec::new(), &Upgrades::default(), 1.0 / 60.0, &mut fx);
    assert!(enemies[0].status.frozen > 0.0, "the pulse freezes it: {}", enemies[0].status.chill);
}
