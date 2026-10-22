//the shooters that pierce, splash and home in, besides the one that fires plain shots

use super::*;

const START: Vec2 = Vec2::new(1000.0, 1000.0);

fn ready(name: &str, scale: f32) -> Enemy {
    let mut enemy = Enemy::new(MIDDLE, kind_named(name), scale);
    enemy.position = MIDDLE;
    enemy.shot_in = 0.0;
    enemy
}

fn fly(shots: &mut Vec<Shot>, units: &mut [Unit], walls: &Walls, seconds: f32, fx: &mut Effects) {
    for _ in 0..(seconds * 60.0) as usize {
        update_shots(shots, units, walls, 1.0 / 60.0, &Upgrades::default(), fx);
    }
}

fn lost(unit: &Unit) -> f32 {
    UNIT_HEALTH - unit.health
}

#[test]
fn the_kinds_that_shoot_differently_are_in_the_table_and_the_rest_shoot_plain() {
    for (name, shot) in [("Lancer", ShotKind::Piercing), ("Grenadier", ShotKind::Splash), ("Seeker", ShotKind::Homing)] {
        let kind = kind_named(name);
        assert_eq!((kind.behavior, kind.shot), (Behavior::Shooter, shot), "{name}");
        assert!(kind.range > 0.0 && kind.from_army > 0, "{name}");
    }
    for kind in ENEMY_KINDS.iter().filter(|kind| !matches!(kind.name, "Lancer" | "Grenadier" | "Seeker")) {
        assert_eq!(kind.shot, ShotKind::Plain, "{}", kind.name);
    }
    //a spitter is still the plain one
    assert_eq!(the_spitter().shot, ShotKind::Plain);
}

#[test]
fn a_lancer_fires_bolts_that_go_through_more_of_a_bigger_army() {
    let mut pierce = Vec::new();
    for army in [10usize, 1200, 2400, 6000, 60_000] {
        let mut enemies = vec![ready("Lancer", 1.0)];
        let units: Vec<Unit> = (0..army).map(|_| unit_on(MIDDLE + Vec2::new(400.0, 0.0))).collect();
        let mut shots = Vec::new();
        shoot_enemies(&mut enemies, &units, &mut shots, 1.0 / 60.0, &mut Effects::default());
        assert!(!shots.is_empty() && shots.iter().all(|shot| shot.kind == ShotKind::Piercing), "army {army}");
        assert!(shots.iter().all(|shot| (shot.velocity.length() - LANCE_SPEED).abs() < 0.01 && shot.damage == SHOT_DAMAGE * LANCE_DAMAGE));
        assert!((shots[0].velocity.normalize() - Vec2::X).length() < 0.01, "straight at the units");
        pierce.push(shots[0].pierce);
    }
    assert_eq!(pierce, vec![LANCE_PIERCE, LANCE_PIERCE + 3, LANCE_PIERCE + 6, LANCE_PIERCE_MAX, LANCE_PIERCE_MAX]);
    //it takes longer to ready than a plain shot
    let mut enemies = vec![ready("Lancer", 1.0)];
    shoot_enemies(&mut enemies, &[unit_on(MIDDLE + Vec2::X * 400.0)], &mut Vec::new(), 1.0 / 60.0, &mut Effects::default());
    assert!(enemies[0].shot_in > SHOT_INTERVAL * 1.7 * 0.79);
}

#[test]
fn a_bigger_lancer_fires_more_bolts_side_by_side() {
    let units = vec![unit_on(MIDDLE + Vec2::new(500.0, 0.0))];
    let count = |scale: f32| {
        let mut enemies = vec![ready("Lancer", scale)];
        let mut shots = Vec::new();
        shoot_enemies(&mut enemies, &units, &mut shots, 1.0 / 60.0, &mut Effects::default());
        shots.len()
    };
    assert_eq!(count(1.0), 1);
    assert!(count(MAX_ENEMY_SCALE) > 1 && count(MAX_ENEMY_SCALE) <= MAX_VOLLEY / 3);
}

#[test]
fn a_piercing_bolt_hurts_each_unit_in_its_line_once_and_stops_after_as_many_as_it_can_go_through() {
    //six units in a row, a bolt that can go through three of them
    let mut units: Vec<Unit> = (0..6).map(|i| unit_on(START + Vec2::new(200.0 + 70.0 * i as f32, 0.0))).collect();
    let mut shots = vec![Shot::lance(START, Vec2::new(LANCE_SPEED, 0.0), 3, RED)];
    let mut fx = Effects::default();
    fly(&mut shots, &mut units, &Walls::default(), 1.5, &mut fx);
    assert!(shots.is_empty(), "it is used up");
    let hurt: Vec<f32> = units.iter().map(lost).collect();
    for (i, took) in hurt.iter().enumerate() {
        if i < 3 {
            assert!((took - SHOT_DAMAGE * LANCE_DAMAGE).abs() < 0.01, "unit {i} took {took}");
        } else {
            assert_eq!(*took, 0.0, "unit {i} is behind the third");
        }
    }
}

#[test]
fn a_piercing_bolt_does_not_hurt_the_same_unit_twice_while_it_passes_through() {
    let mut units = vec![unit_on(START + Vec2::new(300.0, 0.0))];
    let mut shots = vec![Shot::lance(START, Vec2::new(LANCE_SPEED, 0.0), 4, RED)];
    fly(&mut shots, &mut units, &Walls::default(), 0.6, &mut Effects::default());
    assert!((lost(&units[0]) - SHOT_DAMAGE * LANCE_DAMAGE).abs() < 0.01, "{}", lost(&units[0]));
    assert!(shots.len() == 1 && shots[0].pierce == 3, "it flies on, having used one of its four");
}

#[test]
fn a_piercing_bolt_that_goes_through_nothing_gives_up_and_a_wall_stops_it() {
    let mut shots = vec![Shot::lance(Vec2::new(100.0, 16000.0), Vec2::new(LANCE_SPEED, 0.0), 4, RED)];
    fly(&mut shots, &mut [], &Walls::default(), SHOT_LIFE + 0.2, &mut Effects::default());
    assert!(shots.is_empty());
    //column 4 is x 256 to 320
    let walls = walls_of(&[(4, 1)]);
    let mut shots = vec![Shot::lance(Vec2::new(100.0, 100.0), Vec2::new(LANCE_SPEED, 0.0), 8, RED)];
    let mut units = vec![unit_on(Vec2::new(500.0, 100.0))];
    let mut fx = Effects::default();
    fly(&mut shots, &mut units, &walls, 1.0, &mut fx);
    assert!(shots.is_empty() && lost(&units[0]) == 0.0, "the wall took it");
    assert!(fx.wall_hits.iter().any(|(tile, damage)| *tile == (4, 1) && *damage > 0.0));
}

#[test]
fn a_grenade_goes_off_where_it_was_aimed_and_hurts_the_units_round_that_but_not_those_outside() {
    let landing = START + Vec2::new(300.0, 0.0);
    let mut units = vec![
        unit_on(landing + Vec2::new(0.0, 80.0)),
        unit_on(landing + Vec2::new(0.0, -60.0)),
        unit_on(landing + Vec2::new(0.0, 160.0)),
        unit_on(START + Vec2::new(0.0, 60.0)),
    ];
    let splash = 100.0;
    let mut shots = vec![Shot::grenade(START, Vec2::new(GRENADE_SPEED, 0.0), 300.0 / GRENADE_SPEED, splash, 10, ORANGE)];
    let mut fx = Effects::default();
    fly(&mut shots, &mut units, &Walls::default(), 1.5, &mut fx);
    assert!(shots.is_empty());
    assert_eq!(fx.blasts.len(), 1);
    assert!(fx.blasts[0].position.distance(landing) < GRENADE_SPEED / 60.0 + 1.0, "{:?}", fx.blasts[0].position);
    assert!((lost(&units[0]) - SHOT_DAMAGE * GRENADE_DAMAGE).abs() < 0.01 && (lost(&units[1]) - SHOT_DAMAGE * GRENADE_DAMAGE).abs() < 0.01);
    assert!(lost(&units[2]) == 0.0 && lost(&units[3]) == 0.0, "outside the blast");
    assert!(!fx.rings.is_empty() && !fx.marks.is_empty() && fx.cues.contains(&Cue::Explosion));
}

#[test]
fn a_grenade_that_meets_a_unit_on_the_way_goes_off_there() {
    let mut units = vec![unit_on(START + Vec2::new(150.0, 0.0)), unit_on(START + Vec2::new(150.0, 60.0))];
    let mut shots = vec![Shot::grenade(START, Vec2::new(GRENADE_SPEED, 0.0), 1.0, 100.0, 10, ORANGE)];
    let mut fx = Effects::default();
    fly(&mut shots, &mut units, &Walls::default(), 1.0, &mut fx);
    assert!(shots.is_empty() && fx.blasts.len() == 1);
    assert!(fx.blasts[0].position.x < START.x + 150.0, "it did not carry on to where it was aimed: {:?}", fx.blasts[0].position);
    assert!(lost(&units[0]) > 0.0 && lost(&units[1]) > 0.0, "and the one beside it feels it as well");
}

#[test]
fn a_grenade_hurts_only_so_many_units_and_those_are_the_nearest() {
    //ten units in a column beside the landing place (out of the way of the flight), each a little further from it than the last
    let landing = START + Vec2::new(300.0, 0.0);
    let mut units: Vec<Unit> = (0..10).map(|i| unit_on(landing + Vec2::new(0.0, 40.0 + 5.0 * i as f32))).collect();
    let mut shots = vec![Shot::grenade(START, Vec2::new(GRENADE_SPEED, 0.0), 300.0 / GRENADE_SPEED, 100.0, 3, ORANGE)];
    let mut fx = Effects::default();
    fly(&mut shots, &mut units, &Walls::default(), 1.5, &mut fx);
    let hurt: Vec<usize> = (0..10).filter(|&i| lost(&units[i]) > 0.0).collect();
    assert_eq!(hurt, vec![0, 1, 2], "the three nearest");
}

#[test]
fn a_grenade_that_hits_a_wall_goes_off_at_the_wall_and_the_army_beyond_the_blast_is_safe() {
    //column 4 is x 256 to 320
    let walls = walls_of(&[(4, 1)]);
    let mut units = vec![unit_on(Vec2::new(900.0, 100.0))];
    let mut shots = vec![Shot::grenade(Vec2::new(100.0, 100.0), Vec2::new(GRENADE_SPEED, 0.0), 3.0, 100.0, 10, ORANGE)];
    let mut fx = Effects::default();
    fly(&mut shots, &mut units, &walls, 1.0, &mut fx);
    assert!(shots.is_empty() && fx.blasts.len() == 1);
    assert!(fx.blasts[0].position.x > 200.0 && fx.blasts[0].position.x < 330.0, "{:?}", fx.blasts[0].position);
    assert_eq!(lost(&units[0]), 0.0);
}

#[test]
fn a_grenadier_lobs_a_grenade_that_lands_on_the_nearest_unit_and_a_bigger_one_lobs_more() {
    rand::srand(41);
    let target = MIDDLE + Vec2::new(500.0, 0.0);
    let units = vec![unit_on(target), unit_on(MIDDLE + Vec2::new(0.0, 640.0))];
    let mut enemies = vec![ready("Grenadier", 1.0)];
    let mut shots = Vec::new();
    let mut fx = Effects::default();
    shoot_enemies(&mut enemies, &units, &mut shots, 1.0 / 60.0, &mut fx);
    assert_eq!(shots.len(), 1);
    let grenade = &shots[0];
    assert_eq!(grenade.kind, ShotKind::Splash);
    assert!((grenade.velocity.length() - GRENADE_SPEED).abs() < 0.01);
    let lands = grenade.position + grenade.velocity * grenade.life;
    assert!(lands.distance(target) < 1.0, "{lands:?}");
    assert_eq!((grenade.splash, grenade.victims), (GRENADE_RADIUS * blast_scale(1.0), victims_of(units.len(), GRENADE_VICTIMS)));
    assert!(fx.cues.contains(&Cue::Shoot));
    //the grenades of a big one come down round the first
    let mut enemies = vec![ready("Grenadier", MAX_ENEMY_SCALE)];
    let mut shots = Vec::new();
    shoot_enemies(&mut enemies, &units, &mut shots, 1.0 / 60.0, &mut Effects::default());
    let count = enemies[0].volley / 3;
    assert!(count > 1 && shots.len() == count, "{} grenades", shots.len());
    let splash = GRENADE_RADIUS * blast_scale(MAX_ENEMY_SCALE);
    for grenade in &shots {
        assert!((grenade.position + grenade.velocity * grenade.life).distance(target) <= splash * 1.5 + 1.0);
    }
    assert!(shots.iter().skip(1).any(|grenade| (grenade.position + grenade.velocity * grenade.life).distance(target) > 1.0), "scattered");
}

#[test]
fn an_orb_bends_towards_the_nearest_unit_without_slowing_and_hits_it() {
    let mut units = vec![unit_on(START + Vec2::new(250.0, 200.0))];
    let mut shots = vec![Shot::seeker(START, Vec2::new(SEEKER_SPEED, 0.0), PURPLE)];
    let mut fx = Effects::default();
    update_shots(&mut shots, &mut units, &Walls::default(), 1.0 / 60.0, &Upgrades::default(), &mut fx);
    assert!(shots[0].velocity.y > 0.0, "it has begun to turn towards the unit");
    assert!((shots[0].velocity.length() - SEEKER_SPEED).abs() < 0.01);
    assert!(shots[0].velocity.y.atan2(shots[0].velocity.x) <= SEEKER_TURN / 60.0 + 1.0e-4, "and no faster than it can");
    fly(&mut shots, &mut units, &Walls::default(), SEEKER_LIFE, &mut fx);
    assert!(shots.is_empty());
    assert!((lost(&units[0]) - SHOT_DAMAGE * SEEKER_DAMAGE).abs() < 0.01, "it found the unit: {}", lost(&units[0]));
}

#[test]
fn an_orb_with_nothing_in_reach_flies_straight_and_gives_up_after_its_life() {
    //(a unit just past its reach is ignored)
    let mut units = vec![unit_on(START + Vec2::new(0.0, SEEKER_REACH + 60.0))];
    let mut shots = vec![Shot::seeker(START, Vec2::new(SEEKER_SPEED, 0.0), PURPLE)];
    fly(&mut shots, &mut units, &Walls::default(), 0.2, &mut Effects::default());
    assert_eq!(shots[0].velocity, Vec2::new(SEEKER_SPEED, 0.0));
    assert!((shots[0].position.x - (START.x + SEEKER_SPEED * 0.2)).abs() < 6.0);
    fly(&mut shots, &mut units, &Walls::default(), SEEKER_LIFE, &mut Effects::default());
    assert!(shots.is_empty());
    assert_eq!(lost(&units[0]), 0.0);
}

#[test]
fn a_seeker_fires_a_few_orbs_in_a_fan_and_the_tooltip_says_what_each_shooter_does() {
    let units = vec![unit_on(MIDDLE + Vec2::new(500.0, 0.0))];
    let mut enemies = vec![ready("Seeker", MAX_ENEMY_SCALE)];
    let mut shots = Vec::new();
    shoot_enemies(&mut enemies, &units, &mut shots, 1.0 / 60.0, &mut Effects::default());
    assert!(shots.len() > 1 && shots.len() <= 5 && shots.iter().all(|shot| shot.kind == ShotKind::Homing && shot.life == SEEKER_LIFE));
    let says = |name: &str| traits(&kind_at(name, MIDDLE)).join(", ");
    assert!(says("Lancer").contains("go through"), "{}", says("Lancer"));
    assert!(says("Grenadier").contains("grenades"), "{}", says("Grenadier"));
    assert!(says("Seeker").contains("home in"), "{}", says("Seeker"));
    assert!(says("Spitter").contains("shoots"), "{}", says("Spitter"));
}
