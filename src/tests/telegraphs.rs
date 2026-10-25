//the shapes a warning on the ground can have: what is inside each and the way out, the hatching that stays inside it, the attacks that put them down,
//the warning they make, the units and walls they hit, how they are told to the other players, the arrows at the edge of the screen and the ticking

use super::*;

const DEADLY: f32 = UNIT_HEALTH * 3.0;

fn army_near(at: Vec2, count: usize) -> Vec<Unit> {
    (0..count).map(|i| unit_on(at + Vec2::new((i % 20) as f32 * 30.0, (i / 20) as f32 * 30.0))).collect()
}

fn every_shape() -> [Shape; 5] {
    [
        Shape::Circle,
        Shape::Ring { inner: 150.0 },
        Shape::Line { dir: Vec2::X, length: 600.0, width: 100.0 },
        Shape::Cone { dir: Vec2::Y, arc: 1.4 },
        Shape::Rect { dir: Vec2::from_angle(0.5), length: 500.0, width: 200.0 },
    ]
}

//points spread over a square 1200 across round MIDDLE
fn spread(k: usize) -> Vec2 {
    MIDDLE + Vec2::new(((k * 37) % 41) as f32 - 20.0, ((k * 53) % 43) as f32 - 21.0) * 30.0
}

#[test]
fn each_shape_holds_what_it_should_and_not_what_it_should_not() {
    let (at, r) = (MIDDLE, 400.0);
    assert!(inside(at, r, Shape::Circle, at + Vec2::new(390.0, 0.0)) && !inside(at, r, Shape::Circle, at + Vec2::new(410.0, 0.0)));
    let ring = Shape::Ring { inner: 150.0 };
    assert!(!inside(at, r, ring, at), "the middle of a ring is safe");
    assert!(inside(at, r, ring, at + Vec2::new(0.0, 300.0)) && !inside(at, r, ring, at + Vec2::new(0.0, 420.0)));
    let line = Shape::Line { dir: Vec2::X, length: 600.0, width: 100.0 };
    assert!(inside(at, 0.0, line, at + Vec2::new(590.0, 45.0)));
    assert!(!inside(at, 0.0, line, at + Vec2::new(300.0, 60.0)), "beside the lane");
    assert!(!inside(at, 0.0, line, at + Vec2::new(-10.0, 0.0)), "behind where it starts");
    assert!(!inside(at, 0.0, line, at + Vec2::new(610.0, 0.0)), "past its end");
    let cone = Shape::Cone { dir: Vec2::Y, arc: 1.4 };
    let quarter = std::f32::consts::FRAC_PI_2;
    assert!(inside(at, r, cone, at + Vec2::new(0.0, 380.0)) && inside(at, r, cone, at + Vec2::from_angle(quarter + 0.65) * 300.0));
    assert!(!inside(at, r, cone, at + Vec2::new(0.0, -100.0)), "behind it");
    assert!(!inside(at, r, cone, at + Vec2::from_angle(quarter + 0.75) * 300.0), "to its side");
    let rect = Shape::Rect { dir: Vec2::Y, length: 500.0, width: 200.0 };
    assert!(inside(at, 0.0, rect, at + Vec2::new(95.0, 245.0)) && inside(at, 0.0, rect, at + Vec2::new(-95.0, -245.0)));
    assert!(!inside(at, 0.0, rect, at + Vec2::new(105.0, 0.0)) && !inside(at, 0.0, rect, at + Vec2::new(0.0, 255.0)));
}

#[test]
fn how_far_outside_a_shape_a_point_is_is_nothing_in_it_and_something_out_of_it() {
    for shape in every_shape() {
        let reach = shape.reach(400.0);
        for k in 0..400 {
            let at = spread(k);
            let out = outside_by(MIDDLE, reach, shape, at);
            if inside(MIDDLE, reach, shape, at) {
                assert!(out < 0.5, "{shape:?} at {at}: {out}");
            } else {
                assert!(out > 0.0, "{shape:?} at {at}");
            }
        }
    }
}

#[test]
fn walking_the_way_out_of_a_shape_gets_out_of_it_soon() {
    for shape in every_shape() {
        let reach = shape.reach(400.0);
        for k in 0..200 {
            let mut at = spread(k);
            if !inside(MIDDLE, reach, shape, at) {
                continue;
            }
            let mut steps = 0;
            while inside(MIDDLE, reach, shape, at) && steps < 1000 {
                at += way_out(MIDDLE, reach, shape, at) * 2.0;
                steps += 1;
            }
            assert!(steps as f32 * 2.0 <= reach + 4.0, "{shape:?} from {}: {steps} steps", spread(k));
        }
    }
}

#[test]
fn the_hatching_of_a_shape_stays_inside_it() {
    for shape in every_shape() {
        let reach = shape.reach(400.0);
        let mut drawn = 0;
        for k in 0..40 {
            let along = Vec2::from_angle(k as f32 * 0.37);
            let from = MIDDLE + along.perp() * (k as f32 * 23.0 - 450.0);
            for (a, b) in clip_line(MIDDLE, reach, shape, from, along) {
                drawn += 1;
                for t in [0.05, 0.5, 0.95] {
                    assert!(inside(MIDDLE, reach, shape, a.lerp(b, t)), "{shape:?}");
                }
                assert!(outside_by(MIDDLE, reach, shape, a) < 1.0 && outside_by(MIDDLE, reach, shape, b) < 1.0, "{shape:?}");
            }
        }
        assert!(drawn > 5, "{shape:?} is hatched");
    }
}

#[test]
fn every_kind_of_a_bosss_warning_has_a_hatching_of_its_own_and_the_rest_have_none() {
    assert_eq!(hatch_of(None), None);
    for (i, a) in CIRCLES.iter().enumerate() {
        let one = hatch_of(Some(*a)).expect("hatched");
        for b in &CIRCLES[i + 1..] {
            assert_ne!(one, hatch_of(Some(*b)).unwrap(), "{a:?} and {b:?} are hatched alike");
        }
    }
}

#[test]
fn a_lane_hurts_only_the_units_in_it_and_breaks_only_the_walls_in_it() {
    let lane = Shape::Line { dir: Vec2::X, length: 800.0, width: 120.0 };
    let mut units: Vec<Unit> = (0..10).map(|i| unit_on(MIDDLE + Vec2::new(60.0 + i as f32 * 70.0, if i % 2 == 0 { 0.0 } else { 200.0 }))).collect();
    let mut hazards = vec![hazard_at(MIDDLE, 0.0, 0.0, DEADLY, 100).shaped(lane)];
    let mut fx = Effects::default();
    update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.016, &mut fx);
    for (i, unit) in units.iter().enumerate() {
        assert_eq!(unit.health <= 0.0, i % 2 == 0, "unit {i}");
    }
    assert_eq!(fx.blasts.len(), 1);
    let (col, row) = tile_of(MIDDLE.x + 400.0, MIDDLE.y);
    let mut walls = walls_with(vec![Wall::new(col, row), Wall::new(col, row + 4)]);
    update_walls(&mut walls, &mut [], 0.016, &mut fx);
    assert!(walls.get(&(col, row)).is_none_or(|wall| wall.health < WALL_HEALTH), "the wall in the lane is hurt or gone");
    assert_eq!(walls[&(col, row + 4)].health, WALL_HEALTH, "the wall beside it");
}

#[test]
fn a_fan_put_down_together_is_one_warning_of_all_that_is_at_stake_in_it() {
    let units = army_near(MIDDLE, 40);
    let far = MIDDLE + Vec2::new(-5000.0, 0.0);
    let beam = |y: f32| hazard_at(MIDDLE + Vec2::new(-20.0, y), 0.0, 3.0, DEADLY, 100).shaped(Shape::Line { dir: Vec2::X, length: 700.0, width: 20.0 }).circle(BossCircle::Beam);
    let apart = blast_warning(&[beam(0.0), beam(30.0)], far, &units).expect("the army is in them");
    assert_eq!(apart.units, 20);
    let together = blast_warning(&[beam(0.0).grouped(7), beam(30.0).grouped(7)], far, &units).expect("the army is in them");
    assert_eq!((together.units, together.circle, together.shape), (40, Some(BossCircle::Beam), beam(0.0).shape));
    //(and the player standing in one of them is in the fan)
    let mine = blast_warning(&[beam(0.0).grouped(7), beam(30.0).grouped(7)], MIDDLE + Vec2::new(300.0, 30.0), &units).unwrap();
    assert!(mine.player_inside);
}

#[test]
fn the_warning_names_a_shape_that_is_not_a_circle_and_says_what_to_do() {
    let warning = |shape: Shape, circle: Option<BossCircle>, player_inside: bool| BlastWarning { seconds: 2.0, units: 10, player_inside, circle, shape };
    assert_eq!(warning_title(&warning(Shape::Circle, Some(BossCircle::Slam), true)), "GET OUT OF THE ORANGE!");
    assert_eq!(warning_title(&warning(Shape::Circle, None, false)), "THE ARMY IS IN THE RED!");
    assert_eq!(warning_title(&warning(Shape::Ring { inner: 100.0 }, Some(BossCircle::GravityRing), true)), "GET OUT OF THE WHITE RING: STAND CLOSE!");
    assert_eq!(warning_title(&warning(Shape::Line { dir: Vec2::X, length: 9.0, width: 9.0 }, Some(BossCircle::Beam), false)), "THE ARMY IS IN THE CYAN BEAM: STEP ASIDE!");
    assert_eq!(warning_title(&warning(Shape::Cone { dir: Vec2::X, arc: 1.0 }, Some(BossCircle::Cleave), true)), "GET OUT OF THE LIME CONE: GET BEHIND IT!");
    assert_eq!(warning_title(&warning(Shape::Rect { dir: Vec2::X, length: 9.0, width: 9.0 }, Some(BossCircle::Piston), true)), "GET OUT OF THE STEEL LANE: STEP ASIDE!");
}

#[test]
fn the_new_attacks_put_their_shapes_down_round_the_boss_and_at_the_army() {
    let army_at = MIDDLE + Vec2::new(-285.0, -1200.0);
    let units = army_near(army_at, 200);
    for pattern in [Pattern::Piston, Pattern::GravityRing, Pattern::Beams, Pattern::Cleave] {
        let mut boss = Enemy::boss(&BOSSES[1], 800, 0, MIDDLE);
        let mut state = boss.boss.take().unwrap();
        let (mut born, mut shots, mut fx) = (Vec::new(), Vec::new(), Effects::default());
        cast_pattern(&mut boss, &mut state, pattern, &units, army_at, 0, &mut born, &mut shots, &mut fx);
        assert!(!fx.hazards.is_empty() && matches!(boss.mode, Mode::Cast { .. }), "{pattern:?}");
        let hazards = &fx.hazards;
        let edge = boss.size / 2.0;
        match pattern {
            Pattern::Piston => {
                let Shape::Rect { dir, length, .. } = hazards[0].shape else { panic!("a piston line is a box") };
                assert!(dir.y < -0.9, "it slams at the army: {dir}");
                assert!(hazards[0].contains(boss.position + dir * 10.0) && hazards[0].contains(boss.position + dir * length * 0.9));
                assert!(!hazards[0].contains(boss.position - dir * (edge + 50.0)), "behind it is safe");
            }
            Pattern::GravityRing => {
                assert!(matches!(hazards[0].shape, Shape::Ring { .. }));
                assert!(!hazards[0].contains(boss.position + Vec2::new(edge + 50.0, 0.0)), "close to the boss is safe");
                assert!(hazards[0].contains(boss.position + Vec2::new(edge + RING_SAFE + 100.0, 0.0)));
            }
            Pattern::Beams => {
                assert_eq!(hazards.len(), BEAMS);
                assert!(hazards.iter().all(|hazard| hazard.group != 0 && hazard.group == hazards[0].group && matches!(hazard.shape, Shape::Line { .. })));
                //(there are gaps between the beams to stand in)
                let around = edge + 400.0;
                let safe = (0..72).filter(|k| !hazards.iter().any(|hazard| hazard.contains(boss.position + Vec2::from_angle((*k as f32 * 5.0).to_radians()) * around))).count();
                assert!(safe > 36, "{safe} of 72 spots round it are safe");
            }
            Pattern::Cleave => {
                let Shape::Cone { dir, .. } = hazards[0].shape else { panic!("a cleave is a cone") };
                assert!(dir.y < -0.9, "it cleaves at the army: {dir}");
                assert!(hazards[0].contains(boss.position + dir * (edge + 200.0)));
                assert!(!hazards[0].contains(boss.position - dir * (edge + 100.0)), "behind it is safe");
            }
            _ => unreachable!(),
        }
        assert!(hazards.iter().all(|hazard| hazard.damage >= UNIT_HEALTH && hazard.warn_total >= BLAST_WARNING_MIN_SECONDS), "{pattern:?} is warned of");
    }
}

//two shapes alike, but for the rounding of a way told as an angle
fn alike(a: Shape, b: Shape) -> bool {
    let near = |x: f32, y: f32| (x - y).abs() < 1e-3;
    let close = |a: Vec2, b: Vec2| a.distance(b) < 1e-4;
    match (a, b) {
        (Shape::Circle, Shape::Circle) => true,
        (Shape::Ring { inner: x }, Shape::Ring { inner: y }) => near(x, y),
        (Shape::Line { dir: a, length: l, width: w }, Shape::Line { dir: b, length: m, width: v }) | (Shape::Rect { dir: a, length: l, width: w }, Shape::Rect { dir: b, length: m, width: v }) => {
            close(a, b) && near(l, m) && near(w, v)
        }
        (Shape::Cone { dir: a, arc: x }, Shape::Cone { dir: b, arc: y }) => close(a, b) && near(x, y),
        _ => false,
    }
}

#[test]
fn every_shape_and_its_group_are_told_to_the_other_players_and_a_wild_one_is_refused() {
    for shape in every_shape() {
        let hazard = hazard_at(MIDDLE, 400.0, 2.0, DEADLY, 10).shaped(shape).grouped(42).circle(BossCircle::Beam);
        let mut out = Writer::new();
        write_hazard(&mut out, &hazard);
        let back = read_hazard(&mut Reader::new(&out.bytes)).expect("it reads");
        assert_eq!((back.group, back.circle, back.radius), (42, Some(BossCircle::Beam), hazard.radius));
        assert!(alike(back.shape, shape), "{:?} came back as {:?}", shape, back.shape);
    }
    for wild in [Shape::Cone { dir: Vec2::X, arc: 10.0 }, Shape::Line { dir: Vec2::X, length: f32::NAN, width: 10.0 }, Shape::Rect { dir: Vec2::X, length: 1.0e7, width: 10.0 }, Shape::Ring { inner: -5.0 }] {
        let mut out = Writer::new();
        write_hazard(&mut out, &hazard_at(MIDDLE, 400.0, 2.0, DEADLY, 10).shaped(wild));
        assert!(read_hazard(&mut Reader::new(&out.bytes)).is_none(), "{wild:?}");
    }
}

#[test]
fn a_big_warning_off_the_screen_gets_an_arrow_and_one_on_it_or_far_from_it_does_not() {
    let (screen, zoom, player) = (Vec2::new(1600.0, 900.0), 1.0, MIDDLE);
    let at = |x: f32| player + Vec2::new(x, 0.0);
    let slam = |x: f32| hazard_at(at(x), 300.0, 3.0, DEADLY, 10).circle(BossCircle::Slam);
    assert!(edge_marks(&[slam(200.0)], player, zoom, screen).is_empty(), "it is on the screen");
    let marks = edge_marks(&[slam(1400.0)], player, zoom, screen);
    assert_eq!(marks.len(), 1);
    assert!(marks[0].2.starts_with("SLAM"), "{}", marks[0].2);
    assert!(edge_marks(&[slam(9000.0)], player, zoom, screen).is_empty(), "too far off to matter");
    assert!(edge_marks(&[hazard_at(at(1400.0), 300.0, 3.0, DEADLY, 10).circle(BossCircle::Meteor)], player, zoom, screen).is_empty(), "meteors fall on the army");
    assert!(edge_marks(&[hazard_at(at(1400.0), 100.0, 3.0, DEADLY, 10)], player, zoom, screen).is_empty(), "a small plain one is not worth it");
    let fan: Vec<Hazard> = (0..5).map(|k| hazard_at(at(2000.0), 0.0, 3.0, DEADLY, 10).shaped(Shape::Line { dir: Vec2::from_angle(k as f32 * 1.2566), length: 400.0, width: 60.0 }).grouped(9).circle(BossCircle::Beam)).collect();
    assert_eq!(edge_marks(&fan, player, zoom, screen).len(), 1, "a fan is one arrow");
}

#[test]
fn the_ticking_quickens_as_the_warning_comes_due() {
    assert_eq!(tick_gap(10.0), TICK_SLOWEST);
    assert_eq!(tick_gap(0.0), TICK_FASTEST);
    let mut last = f32::MAX;
    for k in (0..30).rev() {
        let gap = tick_gap(k as f32 * 0.1);
        assert!(gap <= last);
        last = gap;
    }
    assert!(tick_gap(1.0) < tick_gap(2.5));
    assert!(ALL_CUES.contains(&Cue::Tick));
}
