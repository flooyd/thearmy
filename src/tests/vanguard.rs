//the vanguard: only the front of an army fires, ahead of it, at what is there, and it holds its fire with nothing there

use super::*;

//an enemy nothing kills in a test, and that does no harm
fn post(at: Vec2) -> Enemy {
    let mut enemy = kind_at("Grunt", at);
    enemy.max_health = 1.0e9;
    enemy.health = 1.0e9;
    enemy.attack = 0.0;
    enemy
}

//two hundred units in a block round the middle: 20 across (x, from -300 to 270) and 10 down (y, from -150 to 120)
fn block() -> Vec<Unit> {
    (0..200).map(|i| unit_at(MIDDLE.x - 300.0 + (i % 20) as f32 * 30.0, MIDDLE.y - 150.0 + (i / 20) as f32 * 30.0)).collect()
}

//the bolts an army fires over some seconds, not flown, so each is where it was fired from
fn fire_for(units: &[Unit], enemies: &[Enemy], facing: Vec2, formation: Formation, upgrades: &Upgrades, seconds: f32, density: f32) -> Vec<Bolt> {
    let fronts = [Front { position: MIDDLE, facing, formation, upgrades }];
    let mut bolts = Vec::new();
    let dt = 1.0 / 60.0;
    for frame in 0..(seconds / dt).round() as usize {
        fire_vanguard(&fronts, units, enemies, 10.0 + frame as f32 * dt, dt, density, true, &mut bolts, None);
    }
    bolts
}

fn plain(units: &[Unit], enemies: &[Enemy], facing: Vec2) -> Vec<Bolt> {
    fire_for(units, enemies, facing, Formation::Free, &Upgrades::default(), 1.0, 1.0)
}

//what the bolts do a second between them
fn damage_a_second(bolts: &[Bolt], seconds: f32) -> f32 {
    bolts.iter().map(|bolt| bolt.hurt.each).sum::<f32>() / seconds
}

#[test]
fn only_the_front_of_the_army_fires_and_only_ahead_of_it() {
    let units = block();
    let bolts = plain(&units, &[post(MIDDLE + Vec2::new(700.0, 0.0))], Vec2::X);
    assert!(bolts.len() > 20, "{} bolts", bolts.len());
    //the front GUNNERY_FRONT_SHARE of 570 pixels of depth, from the front at 270 (and a unit's radius on for the muzzle)
    let line = 270.0 - GUNNERY_FRONT_SHARE * 570.0;
    assert!(bolts.iter().all(|bolt| bolt.position.x >= MIDDLE.x + line), "{:?}", bolts.iter().map(|bolt| bolt.position.x - MIDDLE.x).fold(f32::MAX, f32::min));
    assert!(bolts.iter().all(|bolt| bolt.velocity.x > 0.0 && bolt.kind == BoltKind::Tracer));
    assert!(plain(&units, &[post(MIDDLE + Vec2::new(-900.0, 0.0))], Vec2::X).is_empty(), "an enemy behind the army is never shot");
}

#[test]
fn the_fire_turns_with_the_army() {
    let units = block();
    let behind_now = [post(MIDDLE + Vec2::new(700.0, 0.0))];
    assert!(plain(&units, &behind_now, -Vec2::X).is_empty());
    let bolts = plain(&units, &[post(MIDDLE + Vec2::new(-900.0, 0.0))], -Vec2::X);
    assert!(!bolts.is_empty());
    let line = 300.0 - GUNNERY_FRONT_SHARE * 570.0;
    assert!(bolts.iter().all(|bolt| bolt.position.x <= MIDDLE.x - line + 0.5 && bolt.velocity.x < 0.0), "the front is the other side now");
}

#[test]
fn with_nothing_ahead_in_reach_it_holds_its_fire() {
    let units = block();
    assert!(plain(&units, &[post(MIDDLE + Vec2::new(-400.0, 400.0))], Vec2::X).is_empty(), "off to the side and behind the gunners");
    assert!(plain(&units, &[post(MIDDLE + Vec2::new(2400.0, 0.0))], Vec2::X).is_empty(), "too far ahead");
    assert!(plain(&units, &[], Vec2::X).is_empty());
}

#[test]
fn it_fires_first_at_what_is_nearly_straight_ahead_and_then_at_what_is_off_to_the_side() {
    let one = vec![unit_at(MIDDLE.x, MIDDLE.y)];
    let ahead = post(MIDDLE + Vec2::from_angle(20f32.to_radians()) * 600.0);
    let aside = post(MIDDLE + Vec2::from_angle(60f32.to_radians()) * 300.0);
    let at = |bolts: &[Bolt]| bolts.iter().map(|bolt| bolt.velocity.y.atan2(bolt.velocity.x).to_degrees()).collect::<Vec<f32>>();
    let both = at(&plain(&one, &[ahead.clone(), aside.clone()], Vec2::X));
    assert!(!both.is_empty() && both.iter().all(|angle| (angle - 20.0).abs() < 6.0), "the one in the narrow cone, though the other is nearer: {both:?}");
    let only_aside = at(&plain(&one, &[aside], Vec2::X));
    assert!(!only_aside.is_empty() && only_aside.iter().all(|angle| (angle - 60.0).abs() < 8.0), "{only_aside:?}");
}

#[test]
fn champions_and_units_in_towers_never_fire() {
    let mut units = vec![champion_unit(champion_named("Juggernaut").unwrap(), MIDDLE), unit_at(MIDDLE.x + 50.0, MIDDLE.y)];
    units[1].garrison = Some((10, 10));
    assert!(plain(&units, &[post(MIDDLE + Vec2::new(500.0, 0.0))], Vec2::X).is_empty());
}

#[test]
fn a_gunner_does_a_share_of_its_touch_and_a_huge_army_fires_no_more_than_the_most_a_frame_in_heavier_bolts() {
    let enemy = [post(MIDDLE + Vec2::new(600.0, 0.0))];
    let gunners = |units: &[Unit]| {
        let ahead: Vec<f32> = units.iter().map(|unit| unit.x - MIDDLE.x).collect();
        let (front, back) = (ahead.iter().copied().fold(f32::MIN, f32::max), ahead.iter().copied().fold(f32::MAX, f32::min));
        ahead.iter().filter(|&&x| x >= front - GUNNERY_FRONT_SHARE * (front - back)).count()
    };
    for (count, columns, spacing) in [(200usize, 20usize, 30.0), (20000, 200, 5.0)] {
        let units: Vec<Unit> = (0..count).map(|i| unit_at(MIDDLE.x - 500.0 + (i % columns) as f32 * spacing, MIDDLE.y - 250.0 + (i / columns) as f32 * spacing)).collect();
        let fronts = [Front { position: MIDDLE, facing: Vec2::X, formation: Formation::Free, upgrades: &Upgrades::default() }];
        let mut bolts = Vec::new();
        let mut most = 0;
        for frame in 0..90 {
            let before = bolts.len();
            fire_vanguard(&fronts, &units, &enemy, 10.0 + frame as f32 / 60.0, 1.0 / 60.0, 1.0, true, &mut bolts, None);
            most = most.max(bolts.len() - before);
        }
        let expected = gunners(&units) as f32 * UNIT_DAMAGE * GUNNER_POWER * vanguard_share(count);
        let done = damage_a_second(&bolts, 1.5);
        assert!((done / expected - 1.0).abs() < 0.15, "{count} units: {done} a second, not {expected}");
        assert!(most <= (GUNNERY_BOLTS_A_SECOND / 60.0).ceil() as usize, "{most} in a frame");
    }
}

#[test]
fn a_lower_density_fires_fewer_heavier_bolts_that_do_as_much() {
    //(two thousand units, so the share of them that fire on Low is near the share asked for)
    let units: Vec<Unit> = (0..2000).map(|i| unit_at(MIDDLE.x - 240.0 + (i % 40) as f32 * 12.0, MIDDLE.y - 300.0 + (i / 40) as f32 * 12.0)).collect();
    let enemy = [post(MIDDLE + Vec2::new(700.0, 0.0))];
    let full = fire_for(&units, &enemy, Vec2::X, Formation::Free, &Upgrades::default(), 4.0, 1.0);
    let low = fire_for(&units, &enemy, Vec2::X, Formation::Free, &Upgrades::default(), 4.0, Density::Low.share());
    let share = low.len() as f32 / full.len() as f32;
    assert!((share - Density::Low.share()).abs() < 0.15, "{} of {}", low.len(), full.len());
    assert!((damage_a_second(&low, 4.0) / damage_a_second(&full, 4.0) - 1.0).abs() < 0.25);
}

#[test]
fn the_vanguards_relics_and_a_wedge_make_it_fire_more_often_further_and_through_more() {
    let units = block();
    let enemy = [post(MIDDLE + Vec2::new(700.0, 0.0))];
    let with = |relic: Relic| {
        let mut upgrades = Upgrades::default();
        upgrades.take(relic);
        upgrades
    };
    let base = fire_for(&units, &enemy, Vec2::X, Formation::Free, &Upgrades::default(), 3.0, 1.0);
    let drilled = fire_for(&units, &enemy, Vec2::X, Formation::Free, &with(Relic::VolleyDrill), 3.0, 1.0);
    assert!(drilled.len() as f32 > base.len() as f32 * 1.15 && damage_a_second(&drilled, 3.0) > damage_a_second(&base, 3.0) * 1.15);
    let wedge = fire_for(&units, &enemy, Vec2::X, Formation::Wedge, &Upgrades::default(), 3.0, 1.0);
    assert!(wedge.len() as f32 > base.len() as f32 * 1.15, "{} against {}", wedge.len(), base.len());
    assert!(base.iter().all(|bolt| bolt.pierce == 0));
    assert!(fire_for(&units, &enemy, Vec2::X, Formation::Free, &with(Relic::ChainShot), 1.0, 1.0).iter().all(|bolt| bolt.pierce == 1));
    let far = [post(MIDDLE + Vec2::new(2100.0, 0.0))];
    assert!(fire_for(&units, &far, Vec2::X, Formation::Free, &Upgrades::default(), 1.0, 1.0).is_empty(), "out of sight");
    assert!(!fire_for(&units, &far, Vec2::X, Formation::Free, &with(Relic::TracerRounds), 1.0, 1.0).is_empty(), "and in sight with tracer rounds");
}

#[test]
fn a_guest_works_out_the_same_fire_and_it_hurts_nothing() {
    let units = block();
    let enemies = [post(MIDDLE + Vec2::new(700.0, 100.0))];
    let fronts = [Front { position: MIDDLE, facing: Vec2::X, formation: Formation::Line, upgrades: &Upgrades::default() }];
    let (mut host, mut guest) = (Vec::new(), Vec::new());
    for frame in 0..60 {
        let clock = 33.0 + frame as f32 / 60.0;
        fire_vanguard(&fronts, &units, &enemies, clock, 1.0 / 60.0, 1.0, true, &mut host, None);
        fire_vanguard(&fronts, &units, &enemies, clock, 1.0 / 60.0, 1.0, false, &mut guest, None);
    }
    let flight = |bolts: &[Bolt]| bolts.iter().map(|bolt| (bolt.position, bolt.velocity, bolt.life)).collect::<Vec<_>>();
    assert!(!host.is_empty());
    assert_eq!(flight(&host), flight(&guest));
    assert!(guest.iter().all(|bolt| bolt.hurt == Hurt::default()) && host.iter().all(|bolt| bolt.hurt.each > 0.0));
}

#[test]
fn the_bigger_the_army_the_further_its_front_sees_until_it_is_screen_wide_at_ten_thousand() {
    assert_eq!(gunnery_range(0), GUNNERY_RANGE);
    let ranges: Vec<f32> = [0, 100, 1000, 2500, 5000, GUNNERY_WIDE_ARMY].into_iter().map(gunnery_range).collect();
    assert!(ranges.windows(2).all(|pair| pair[0] < pair[1]), "{ranges:?}");
    assert_eq!(gunnery_range(GUNNERY_WIDE_ARMY), GUNNERY_WIDE_RANGE);
    assert!(gunnery_range(30000) > GUNNERY_WIDE_RANGE && gunnery_range(30000) < GUNNERY_HUGE_RANGE, "it goes on growing, more slowly");
    assert_eq!(gunnery_range(GUNNERY_HUGE_ARMY), GUNNERY_HUGE_RANGE);
    assert_eq!(gunnery_range(200000), GUNNERY_HUGE_RANGE, "and no further");
    //an enemy two thousand pixels past the front of the army: too far for a small one, in reach of one of ten thousand
    let small = block();
    assert!(plain(&small, &[post(MIDDLE + Vec2::new(2270.0, 0.0))], Vec2::X).is_empty());
    let big: Vec<Unit> = (0..GUNNERY_WIDE_ARMY).map(|i| unit_at(MIDDLE.x - 300.0 + (i % 100) as f32 * 6.0, MIDDLE.y - 300.0 + (i / 100) as f32 * 6.0)).collect();
    let bolts = plain(&big, &[post(MIDDLE + Vec2::new(2294.0, 0.0))], Vec2::X);
    assert!(!bolts.is_empty(), "the big army reaches it");
    assert!(bolts.iter().all(|bolt| bolt.velocity.length() > TRACER_SPEED * 1.5), "and its tracers fly faster to get there");
}
