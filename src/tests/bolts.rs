//the champions' bolts: how a volley's bolts go out, how they fly and what they hit, that a guest makes the same ones and hurts nothing, and how a
//volley is told

use super::*;

//an enemy nothing kills in a test, and that does no harm
fn post(at: Vec2) -> Enemy {
    let mut enemy = kind_at("Grunt", at);
    enemy.max_health = 1.0e9;
    enemy.health = 1.0e9;
    enemy.attack = 0.0;
    enemy
}

//a volley from the middle along +x, of a sharpshooter's colour
fn volley(kind: BoltKind, shape: VolleyShape, count: u8) -> Volley {
    Volley {
        from: MIDDLE,
        aim: Vec2::X,
        kind,
        shape,
        count,
        scatter: 0.0,
        speed: 1000.0,
        reach: 900.0,
        pierce: 0,
        bounce: 0,
        homing: 0.0,
        splash: 0.0,
        embers: 0,
        champion: champion_named("Sharpshooter").unwrap() as u8 + 1,
        seed: 7,
    }
}

const HURT: Hurt = Hurt { each: 100.0, ember: 10.0, owner: 0 };

//the bolts flown, on the host's machine, until none is left or `seconds` have gone
fn fly_out(bolts: &mut Vec<Bolt>, enemies: &mut [Enemy], seconds: f32, fx: &mut Effects) {
    for _ in 0..(seconds * 60.0) as usize {
        if bolts.is_empty() {
            break;
        }
        fly_bolts(bolts, enemies, 1.0 / 60.0, Some(&mut *fx));
    }
}

fn hurt(enemies: &[Enemy]) -> Vec<f32> {
    enemies.iter().map(|enemy| enemy.hurt).collect()
}

#[test]
fn a_round_hurts_the_first_enemy_in_its_way_once_and_is_used_up() {
    let mut enemies = vec![post(MIDDLE + Vec2::new(300.0, 0.0)), post(MIDDLE + Vec2::new(600.0, 0.0)), post(MIDDLE + Vec2::new(300.0, 400.0))];
    let mut bolts = Vec::new();
    launch_volley(&mut bolts, &volley(BoltKind::Round, VolleyShape::Burst, 1), HURT);
    assert_eq!(bolts.len(), 1);
    let mut fx = Effects::default();
    fly_out(&mut bolts, &mut enemies, 3.0, &mut fx);
    assert!(bolts.is_empty());
    assert_eq!(hurt(&enemies), [100.0, 0.0, 0.0]);
    assert!(!fx.bursts.is_empty(), "a puff where it hit");
}

#[test]
fn a_round_that_pierces_goes_on_through_and_one_that_bounces_glances_off_to_the_next() {
    let line = || vec![post(MIDDLE + Vec2::new(300.0, 0.0)), post(MIDDLE + Vec2::new(600.0, 0.0)), post(MIDDLE + Vec2::new(300.0, 250.0))];
    let mut enemies = line();
    let mut bolts = Vec::new();
    launch_volley(&mut bolts, &Volley { pierce: 1, ..volley(BoltKind::Round, VolleyShape::Burst, 1) }, HURT);
    fly_out(&mut bolts, &mut enemies, 3.0, &mut Effects::default());
    assert_eq!(hurt(&enemies), [100.0, 100.0, 0.0], "through the first, and the one behind it");
    let mut enemies = line();
    launch_volley(&mut bolts, &Volley { bounce: 1, ..volley(BoltKind::Round, VolleyShape::Burst, 1) }, HURT);
    fly_out(&mut bolts, &mut enemies, 3.0, &mut Effects::default());
    assert_eq!(hurt(&enemies), [100.0, 0.0, 100.0], "off the first to the nearest it had not hit");
}

#[test]
fn a_shell_flies_over_everything_and_goes_off_where_it_was_thrown_burning_all_round_it() {
    let mut enemies = vec![post(MIDDLE + Vec2::new(200.0, 0.0)), post(MIDDLE + Vec2::new(500.0, 0.0)), post(MIDDLE + Vec2::new(500.0, 120.0)), post(MIDDLE + Vec2::new(900.0, 0.0))];
    let mut bolts = Vec::new();
    launch_volley(&mut bolts, &Volley { reach: 500.0, splash: 150.0, embers: 6, ..volley(BoltKind::Shell, VolleyShape::Burst, 1) }, HURT);
    let mut fx = Effects::default();
    //(half a second to get there, at a thousand pixels a second)
    for _ in 0..25 {
        fly_bolts(&mut bolts, &mut enemies, 1.0 / 60.0, Some(&mut fx));
    }
    assert!(hurt(&enemies).iter().all(|&taken| taken == 0.0), "nothing until it lands, not even what it flew over");
    fly_bolts(&mut bolts, &mut enemies, 0.1, Some(&mut fx));
    assert_eq!(bolts.len(), 6, "it has gone off and thrown out its embers");
    assert!(bolts.iter().all(|bolt| bolt.kind == BoltKind::Ember && bolt.hurt.each == HURT.ember));
    let taken = hurt(&enemies);
    assert!(taken[0] == 0.0 && taken[1] >= 100.0 && taken[2] >= 100.0 && taken[3] == 0.0, "{taken:?}");
    assert!(fx.cues.contains(&Cue::Explosion));
}

#[test]
fn on_a_guests_machine_the_same_volley_makes_the_same_bolts_which_hurt_nothing() {
    for shape in VOLLEY_SHAPES {
        let told = Volley { scatter: 0.6, ..volley(BoltKind::Spark, shape, 5) };
        let (mut host, mut guest) = (Vec::new(), Vec::new());
        launch_volley(&mut host, &told, HURT);
        launch_told(&mut guest, &[Event::Shake(0.1), Event::Volley(told)]);
        let flight = |bolts: &[Bolt]| bolts.iter().map(|bolt| (bolt.position, bolt.velocity, bolt.wait, bolt.life)).collect::<Vec<_>>();
        assert_eq!(flight(&host), flight(&guest), "{shape:?}");
        assert!(guest.iter().all(|bolt| bolt.hurt == Hurt::default()));
    }
    //and it stops where it would have, but hurts nothing
    let mut enemies = vec![post(MIDDLE + Vec2::new(300.0, 0.0))];
    let mut bolts = Vec::new();
    launch_told(&mut bolts, &[Event::Volley(volley(BoltKind::Round, VolleyShape::Burst, 1))]);
    for _ in 0..30 {
        fly_bolts(&mut bolts, &mut enemies, 1.0 / 60.0, None);
    }
    assert!(bolts.is_empty(), "it stopped at the enemy rather than flying on");
    assert_eq!(enemies[0].hurt, 0.0);
}

#[test]
fn a_fan_spreads_its_bolts_across_its_scatter_a_ring_all_the_way_round_and_a_burst_one_after_another() {
    let mut bolts = Vec::new();
    launch_volley(&mut bolts, &Volley { scatter: 1.0, ..volley(BoltKind::Round, VolleyShape::Fan, 5) }, HURT);
    let angles: Vec<f32> = bolts.iter().map(|bolt| Vec2::X.angle_between(bolt.velocity)).collect();
    assert!((angles[0] + 0.5).abs() < 0.03 && (angles[4] - 0.5).abs() < 0.03 && angles[2].abs() < 0.03, "{angles:?}");
    bolts.clear();
    launch_volley(&mut bolts, &volley(BoltKind::Spark, VolleyShape::Ring, 8), HURT);
    let sum: Vec2 = bolts.iter().map(|bolt| bolt.velocity.normalize()).sum();
    assert!(sum.length() < 0.2, "a ring has no side: {sum}");
    bolts.clear();
    launch_volley(&mut bolts, &Volley { scatter: 0.05, ..volley(BoltKind::Round, VolleyShape::Burst, 4) }, HURT);
    assert_eq!(bolts.iter().map(|bolt| bolt.wait).collect::<Vec<_>>(), [0.0, BURST_GAP, BURST_GAP * 2.0, BURST_GAP * 3.0]);
    assert!(bolts.iter().all(|bolt| Vec2::X.angle_between(bolt.velocity).abs() <= 0.026));
}

#[test]
fn there_are_never_more_bolts_than_the_cap() {
    let mut bolts = Vec::new();
    for _ in 0..(BOLT_CAP / MAX_BOLTS_A_VOLLEY as usize + 10) {
        launch_volley(&mut bolts, &volley(BoltKind::Spark, VolleyShape::Ring, MAX_BOLTS_A_VOLLEY), HURT);
    }
    assert_eq!(bolts.len(), BOLT_CAP);
}

#[test]
fn a_homing_bolt_bends_onto_an_enemy_it_would_have_missed() {
    let shoot = |homing: f32| {
        let mut enemies = vec![post(MIDDLE + Vec2::new(300.0, 160.0))];
        let mut bolts = Vec::new();
        launch_volley(&mut bolts, &Volley { homing, ..volley(BoltKind::Spark, VolleyShape::Burst, 1) }, HURT);
        fly_out(&mut bolts, &mut enemies, 2.0, &mut Effects::default());
        enemies[0].hurt
    };
    assert_eq!(shoot(0.0), 0.0);
    assert_eq!(shoot(6.0), 100.0);
}

#[test]
fn a_boss_takes_only_its_share_of_what_a_bolt_does() {
    let mut boss = Enemy::boss(&BOSSES[0], 100, 0, MIDDLE + Vec2::new(400.0, 0.0));
    boss.max_health = 1.0e9;
    boss.health = 1.0e9;
    let mut enemies = vec![boss];
    let mut bolts = Vec::new();
    launch_volley(&mut bolts, &volley(BoltKind::Round, VolleyShape::Burst, 1), HURT);
    fly_out(&mut bolts, &mut enemies, 3.0, &mut Effects::default());
    assert!((enemies[0].hurt - 100.0 * CHAMPION_BOSS_SHARE).abs() < 1e-3, "{}", enemies[0].hurt);
}

#[test]
fn a_volley_is_told_in_a_few_bytes_and_one_no_champion_could_fire_is_not_believed() {
    let told = Event::Volley(Volley { scatter: 0.3, pierce: 2, bounce: 1, homing: 2.5, embers: 4, splash: 90.0, ..volley(BoltKind::Shell, VolleyShape::Fan, 3) });
    let bytes = write_events(std::slice::from_ref(&told));
    assert_eq!(read_events(&bytes), Some(vec![told]));
    assert!(bytes.len() < 64, "{} bytes", bytes.len());
    for wild in [
        Volley { count: MAX_BOLTS_A_VOLLEY + 1, ..volley(BoltKind::Round, VolleyShape::Burst, 1) },
        Volley { speed: MAX_BOLT_SPEED * 2.0, ..volley(BoltKind::Round, VolleyShape::Burst, 1) },
        Volley { pierce: MAX_PIERCE + 1, ..volley(BoltKind::Round, VolleyShape::Burst, 1) },
        Volley { champion: CHAMPIONS.len() as u8 + 1, ..volley(BoltKind::Round, VolleyShape::Burst, 1) },
    ] {
        assert_eq!(read_events(&write_events(&[Event::Volley(wild)])), None, "{wild:?}");
    }
    //nor one of a kind or a shape there is not
    let mut bytes = write_events(&[Event::Volley(volley(BoltKind::Round, VolleyShape::Burst, 1))]);
    bytes[2 + 1 + 16] = 9;
    assert_eq!(read_events(&bytes), None);
}

#[test]
fn a_full_sky_of_bolts_among_hundreds_of_enemies_is_quick_to_fly() {
    let mut enemies: Vec<Enemy> = (0..600).map(|i| post(MIDDLE + Vec2::new((i % 30) as f32 * 70.0 - 1000.0, (i / 30) as f32 * 70.0 - 700.0))).collect();
    let mut bolts = Vec::new();
    for k in 0..(BOLT_CAP / 20) {
        let aim = Vec2::from_angle(k as f32 * 0.37);
        launch_volley(&mut bolts, &Volley { aim, pierce: 3, homing: 2.0, ..volley(BoltKind::Spark, VolleyShape::Ring, 20) }, HURT);
    }
    assert_eq!(bolts.len(), BOLT_CAP);
    let started = Instant::now();
    let mut fx = Effects::default();
    for _ in 0..20 {
        fly_bolts(&mut bolts, &mut enemies, 1.0 / 120.0, Some(&mut fx));
    }
    let each = started.elapsed().as_secs_f32() * 1000.0 / 20.0;
    println!("{} bolts among {} enemies fly in {each:.2} ms a frame", BOLT_CAP, enemies.len());
    assert!(each < 6.0, "{each} ms a frame");
}

#[test]
fn a_bolt_that_hits_flashes_the_enemy_and_knocks_a_light_one_back_but_not_a_boss() {
    let small = post(MIDDLE + Vec2::new(300.0, 0.0));
    let was = small.position;
    let mut enemies = vec![small];
    let mut bolts = Vec::new();
    launch_volley(&mut bolts, &volley(BoltKind::Round, VolleyShape::Burst, 1), HURT);
    fly_out(&mut bolts, &mut enemies, 1.0, &mut Effects::default());
    assert!(enemies[0].struck > 0.0, "it shows white for a moment");
    assert!(enemies[0].position.x > was.x + 1.0, "and is knocked back the way the round went");
    let mut boss = Enemy::boss(&BOSSES[0], 100, 0, MIDDLE + Vec2::new(400.0, 0.0));
    boss.max_health = 1.0e9;
    boss.health = 1.0e9;
    let at = boss.position;
    let mut enemies = vec![boss];
    launch_volley(&mut bolts, &volley(BoltKind::Round, VolleyShape::Burst, 1), HURT);
    fly_out(&mut bolts, &mut enemies, 1.0, &mut Effects::default());
    assert_eq!(enemies[0].position, at);
    //(a guest's machine shows the flash of a hit it worked out for itself, and moves nothing)
    let mut enemies = vec![post(MIDDLE + Vec2::new(300.0, 0.0))];
    launch_told(&mut bolts, &[Event::Volley(volley(BoltKind::Round, VolleyShape::Burst, 1))]);
    for _ in 0..30 {
        fly_bolts(&mut bolts, &mut enemies, 1.0 / 60.0, None);
    }
    assert!(enemies[0].struck > 0.0 && enemies[0].position == MIDDLE + Vec2::new(300.0, 0.0));
}

#[test]
fn a_champions_bolt_now_and_then_hits_twice_as_hard_and_says_so_and_a_tracer_never_does() {
    let crits = |champion: u8| {
        let mut enemies = vec![post(MIDDLE + Vec2::new(300.0, 0.0))];
        let (mut doubled, mut fx) = (0, Effects::default());
        for seed in 0..300 {
            let mut bolts = Vec::new();
            launch_volley(&mut bolts, &Volley { seed, champion, ..volley(BoltKind::Round, VolleyShape::Burst, 1) }, HURT);
            let before = enemies[0].hurt;
            fly_out(&mut bolts, &mut enemies, 1.0, &mut fx);
            if enemies[0].hurt - before > 150.0 {
                doubled += 1;
            }
        }
        (doubled, fx)
    };
    let (doubled, fx) = crits(champion_named("Sharpshooter").unwrap() as u8 + 1);
    assert!((15..=60).contains(&doubled), "{doubled} crits in 300");
    assert!(fx.hit_stop > 0.0 && fx.cues.contains(&Cue::Crit) && !fx.popups.is_empty());
    assert_eq!(crits(0).0, 0);
}

#[test]
fn zoomed_far_out_a_bolt_is_its_core_alone() {
    let mut bolts = Vec::new();
    launch_volley(&mut bolts, &volley(BoltKind::Round, VolleyShape::Burst, 1), HURT);
    fly_bolts(&mut bolts, &mut [], 0.05, None);
    let view = Rect::new(MIDDLE.x - 2000.0, MIDDLE.y - 2000.0, 4000.0, 4000.0);
    assert!(bolt_discs(&bolts, view, 1.0).len() > 2);
    assert_eq!(bolt_discs(&bolts, view, BOLT_DETAIL_ZOOM * 0.5).len(), 1);
}
