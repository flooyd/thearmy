//the circles that are drawn in one go: what is sent to the graphics card for the army, the puffs and the babies. there is no
//graphics card in a test, so these are the parts that decide what is drawn rather than the drawing itself

use super::*;

#[test]
fn a_disc_is_exactly_what_the_shader_reads_for_each_one() {
    //two floats for the place, one for the radius, one for the thickness, one for the border, four bytes of colour and two floats for the nose
    assert_eq!(std::mem::size_of::<Disc>(), 32);
    assert_eq!(std::mem::align_of::<Disc>(), 4);
}

#[test]
fn colours_become_bytes_that_stay_in_range() {
    let disc = Disc::new(1.0, 2.0, 3.0, Color::new(1.0, 0.5, 0.0, 0.25));
    assert_eq!(disc.color, [255, 128, 0, 64]);
    assert_eq!((disc.x, disc.y, disc.radius, disc.thickness), (1.0, 2.0, 3.0, 0.0));
    //a colour a little out of range doesn't wrap round
    assert_eq!(Disc::new(0.0, 0.0, 1.0, Color::new(1.3, -0.2, 0.0, 1.0)).color, [255, 0, 0, 255]);
}

#[test]
fn a_ring_is_centred_on_its_radius_like_a_line_drawn_round_a_circle() {
    let ring = Disc::ring(5.0, 6.0, 10.0, 2.0, GREEN);
    assert_eq!(ring.thickness, 2.0);
    //the line covers 9 to 11, so the ring reaches out to 11 and is 2 thick inwards from there
    assert_eq!(ring.radius, 11.0);
    assert_eq!(ring.color, Disc::new(0.0, 0.0, 1.0, GREEN).color);
}

#[test]
fn the_army_is_drawn_as_discs_the_size_of_a_unit_in_the_colour_of_its_health_and_orders() {
    let view = Rect::new(0.0, 0.0, 1000.0, 1000.0);
    let mut hurt = unit_on(Vec2::new(100.0, 100.0));
    hurt.health = 25.0;
    let mut sent = unit_on(Vec2::new(200.0, 100.0));
    sent.target = Some(3);
    let healthy = unit_on(Vec2::new(300.0, 100.0));
    let units = vec![hurt, sent, healthy];
    let discs = unit_discs(&units, view);
    assert_eq!(discs.len(), 3);
    for (disc, unit) in discs.iter().zip(&units) {
        assert_eq!((disc.x, disc.y, disc.radius, disc.thickness), (unit.x, unit.y, RADIUS, 0.0));
        assert_eq!(disc.border, UNIT_BORDER, "every unit has the black edge");
        assert_eq!(disc.color, Disc::new(0.0, 0.0, 1.0, unit_color(unit)).color);
    }
    assert_ne!(discs[0].color, discs[2].color, "a hurt unit goes red");
    assert_ne!(discs[1].color, discs[2].color, "a unit sent at an enemy is the squad's colour");
}

#[test]
fn a_border_is_something_a_disc_is_given_and_never_has_otherwise() {
    let plain = Disc::new(1.0, 2.0, 16.0, WHITE);
    assert_eq!(plain.border, 0.0);
    assert_eq!(Disc::ring(1.0, 2.0, 16.0, 2.0, WHITE).border, 0.0);
    let edged = plain.edged(2.5);
    assert_eq!((edged.x, edged.y, edged.radius, edged.thickness, edged.color, edged.border), (1.0, 2.0, 16.0, 0.0, plain.color, 2.5));
    //the babies and the puffs are drawn without one
    let babies = [Baby { position: Vec2::new(5.0, 5.0), age: 1.0 }];
    assert!(baby_discs(&babies, Rect::new(0.0, 0.0, 100.0, 100.0)).iter().all(|disc| disc.border == 0.0));
}

#[test]
fn units_off_screen_and_units_inside_towers_are_not_sent_to_be_drawn() {
    let view = Rect::new(0.0, 0.0, 1000.0, 1000.0);
    let mut in_tower = unit_on(Vec2::new(500.0, 500.0));
    in_tower.garrison = Some((3, 3));
    let units = vec![unit_on(Vec2::new(500.0, 500.0)), unit_on(Vec2::new(-1.0, 500.0)), unit_on(Vec2::new(500.0, 1001.0)), unit_on(Vec2::new(999.0, 1.0)), in_tower];
    let discs = unit_discs(&units, view);
    assert_eq!(discs.iter().map(|disc| (disc.x, disc.y)).collect::<Vec<_>>(), vec![(500.0, 500.0), (999.0, 1.0)]);
}

#[test]
fn an_army_of_thousands_is_made_into_discs_quickly() {
    let units: Vec<Unit> = (0..DEFAULT_ARMY_LIMIT).map(|i| unit_on(Vec2::new((i % 100) as f32 * 20.0, (i / 100) as f32 * 20.0))).collect();
    let view = Rect::new(-100.0, -100.0, 5000.0, 5000.0);
    let started = Instant::now();
    let mut made = 0;
    for _ in 0..100 {
        made += unit_discs(&units, view).len();
    }
    let per_frame = started.elapsed().as_secs_f32() * 10.0;
    assert_eq!(made, DEFAULT_ARMY_LIMIT * 100);
    //it is part of every frame, so it should be a small share of one (other tests running at the same time slow this down)
    assert!(per_frame < 2.0, "{per_frame} ms a frame");
}

#[test]
fn a_puff_shrinks_and_fades_as_it_ages() {
    let aged = |age: f32| Particle { position: Vec2::new(10.0, 20.0), velocity: Vec2::ZERO, age, life: 1.0, color: Color::new(1.0, 1.0, 1.0, 1.0), size: 6.0 };
    let discs = particle_discs(&[aged(0.0), aged(0.5), aged(1.5)]);
    assert_eq!(discs.len(), 3);
    assert_eq!((discs[0].x, discs[0].y, discs[0].radius), (10.0, 20.0, 6.0));
    assert!((discs[1].radius - 3.0).abs() < 1e-6);
    assert!(discs[1].color[3] < discs[0].color[3], "{:?} {:?}", discs[1].color, discs[0].color);
    //one past the end of its life has nothing left to draw, and a negative size would flip it inside out
    assert_eq!(discs[2].radius, 0.0);
    assert_eq!(discs[2].color[3], 0);
}

#[test]
fn a_baby_is_a_green_disc_with_a_ring_round_it_that_pops_out_when_it_drops() {
    let view = Rect::new(0.0, 0.0, 1000.0, 1000.0);
    let babies = vec![Baby { position: Vec2::new(100.0, 100.0), age: 5.0 }, Baby { position: Vec2::new(200.0, 100.0), age: 0.075 }, Baby { position: Vec2::new(5000.0, 100.0), age: 5.0 }];
    let discs = baby_discs(&babies, view);
    //two each for the ones in view
    assert_eq!(discs.len(), 4);
    assert_eq!((discs[0].radius, discs[0].thickness), (BABY_RADIUS, 0.0));
    assert_eq!(discs[1].thickness, 2.0);
    assert!((discs[1].radius - (BABY_RADIUS + 4.0 + 1.0)).abs() < 1e-5, "{}", discs[1].radius);
    //half way through popping out it is half the size
    assert!((discs[2].radius - BABY_RADIUS * 0.5).abs() < 1e-5);
    assert!(discs.iter().all(|disc| disc.color == Disc::new(0.0, 0.0, 1.0, GREEN).color));
}

// ---------- the ship ----------

fn first_disc(shape: UnitShape, unit: &Unit) -> Disc {
    unit_discs_in(std::slice::from_ref(unit), Rect::new(-1000.0, -1000.0, 3000.0, 3000.0), None, shape)[0]
}

#[test]
fn circles_are_what_they_were_with_the_black_edge_and_no_nose() {
    let mut unit = unit_on(Vec2::new(100.0, 100.0));
    unit.nose = Vec2::new(1.0, 0.0);
    let disc = first_disc(UnitShape::Circles, &unit);
    assert_eq!(disc.border, UNIT_BORDER, "circles keep their border");
    assert_eq!(disc.nose, [0.0, 0.0], "a disc with no nose is drawn as a circle, whichever way the unit points");
    assert!(!disc.is_ship());
    assert_eq!((disc.x, disc.y, disc.radius, disc.thickness), (100.0, 100.0, RADIUS, 0.0));
    assert_eq!(disc, Disc::new(100.0, 100.0, RADIUS, unit_color(&unit)).edged(UNIT_BORDER));
}

#[test]
fn a_ship_is_the_same_disc_with_a_nose_pointing_the_way_the_unit_does() {
    let mut unit = unit_on(Vec2::new(100.0, 100.0));
    unit.nose = Vec2::new(3.0, 4.0).normalize();
    let ship = first_disc(UnitShape::Ships, &unit);
    let circle = first_disc(UnitShape::Circles, &unit);
    assert!(ship.is_ship());
    assert!((Vec2::from(ship.nose) - Vec2::new(0.6, 0.8)).length() < 1e-6, "{:?}", ship.nose);
    assert_eq!(Disc { nose: [0.0, 0.0], ..ship }, circle, "same place, size, colour and border: only the nose tells them apart");
    assert_eq!(ship.border, UNIT_BORDER);
}

#[test]
fn a_nose_is_always_a_direction_or_nothing() {
    let disc = Disc::new(0.0, 0.0, RADIUS, WHITE);
    assert_eq!(disc.pointing(Vec2::ZERO).nose, [0.0, 0.0], "no direction leaves a circle");
    assert_eq!(disc.pointing(Vec2::new(f32::NAN, 1.0)).nose, [0.0, 0.0]);
    assert_eq!(disc.pointing(Vec2::new(0.0, -50.0)).nose, [0.0, -1.0], "a long one is brought down to length one");
    assert_eq!(Disc::ring(0.0, 0.0, 10.0, 2.0, WHITE).nose, [0.0, 0.0], "rings and puffs are never ships");
}

#[test]
fn a_unit_that_is_still_growing_swells_as_a_ship_too() {
    let mut baby = unit_on(Vec2::new(100.0, 100.0));
    baby.age = UNIT_GROW_SECONDS * 0.25;
    let ship = first_disc(UnitShape::Ships, &baby);
    let circle = first_disc(UnitShape::Circles, &baby);
    assert!(ship.radius < RADIUS * 1.2 && ship.radius > 0.0);
    assert_eq!(ship.radius, circle.radius);
    assert_eq!(ship.color, circle.color, "green at first");
    assert!(ship.is_ship());
}

#[test]
fn champions_and_the_garrisoned_are_still_left_out_of_the_ships() {
    let mut champion = unit_on(Vec2::new(10.0, 10.0));
    champion.champion = 1;
    let mut in_tower = unit_on(Vec2::new(20.0, 10.0));
    in_tower.garrison = Some((3, 3));
    let view = Rect::new(0.0, 0.0, 1000.0, 1000.0);
    assert_eq!(unit_discs_in(&[champion, in_tower, unit_on(Vec2::new(30.0, 10.0))], view, None, UnitShape::Ships).len(), 1);
}

#[test]
fn the_dart_is_a_delta_with_a_notch_and_a_flame_behind_it() {
    //inside the hull
    for point in [Vec2::new(0.0, 0.0), Vec2::new(0.0, -20.0), Vec2::new(10.0, 10.0), Vec2::new(-10.0, 10.0), Vec2::new(0.0, 12.0), Vec2::new(18.0, 20.0)] {
        assert!(hull_distance(point) < 0.0, "{point} is on the hull");
    }
    //outside it: past the nose, beyond the wings, in the notch and behind it
    for point in [Vec2::new(0.0, -29.0), Vec2::new(25.0, 0.0), Vec2::new(-25.0, 0.0), Vec2::new(0.0, 14.0), Vec2::new(0.0, 25.0), Vec2::new(19.0, -20.0), Vec2::new(22.0, 22.0)] {
        assert!(hull_distance(point) > 0.0, "{point} is off the hull");
    }
    //the two wings are mirrors
    assert_eq!(hull_distance(Vec2::new(7.0, 3.0)), hull_distance(Vec2::new(-7.0, 3.0)));
    //the cockpit is on the hull and the flame behind it
    assert!(cockpit_distance(COCKPIT_AT) < 0.0 && hull_distance(COCKPIT_AT) < 0.0);
    assert!(cockpit_distance(Vec2::new(0.0, 8.0)) > 0.0, "the glass is not the whole of the hull");
    assert!(flame_distance(Vec2::new(0.0, 20.0)) < 0.0 && hull_distance(Vec2::new(0.0, 20.0)) > 0.0, "the flame shows behind the notch");
    assert!(flame_distance(Vec2::new(0.0, 14.0)) > 0.0, "and starts where it should");
    assert!(flame_distance(Vec2::new(6.0, 20.0)) > 0.0 && flame_distance(Vec2::new(0.0, 28.0)) > 0.0);
}

#[test]
fn a_ship_fits_the_square_it_is_drawn_on_whichever_way_it_points() {
    //the corners of the hull, with the outline at its widest, are all inside the square (whose half-width is REACH radiuses, and a corner of it is further)
    let reach = SHIP_REACH * SHIP_SPAN;
    for corner in dart_corners().into_iter().chain([FLAME_TIP]) {
        assert!(corner.length() + OUTLINE_MAX < reach, "{corner} is too far out");
    }
    //and it is about the size of a unit: the wing tips are no further out than the radius and a little (the outline goes outside, as the circle's edge is inside)
    assert!(dart_corners().iter().all(|corner| corner.length() < SHIP_SPAN * 1.05));
}

#[test]
fn the_art_turns_with_the_nose() {
    let nose_of = |direction: Vec2| art_to_world(DART_NOSE, direction);
    //pointing up the nose is above the middle, a radius or so away
    let up = nose_of(Vec2::new(0.0, -1.0));
    assert!(up.x.abs() < 1e-6 && (up.y + 27.0 / SHIP_SPAN).abs() < 1e-6, "{up}");
    //pointing right (clockwise a quarter turn, y being down) the nose is to the right
    let right = nose_of(Vec2::new(1.0, 0.0));
    assert!((right.x - 27.0 / SHIP_SPAN).abs() < 1e-6 && right.y.abs() < 1e-6, "{right}");
    //the right wing stays on the right of the nose: pointing down it is on the left of the screen
    let wing = art_to_world(DART_WING, Vec2::new(0.0, 1.0));
    assert!(wing.x < 0.0, "{wing}");
    //a turn keeps sizes
    assert!((art_to_world(DART_WING, Vec2::new(0.6, 0.8)).length() - DART_WING.length() / SHIP_SPAN).abs() < 1e-6);
}

#[test]
fn the_shaders_have_every_number_of_the_art_they_use() {
    let constants = art_constants();
    let (vertex, fragment) = (vertex_source(), fragment_source());
    assert!(vertex.starts_with("#version 100\n") && fragment.starts_with("#version 100\nprecision highp float;\n"), "the version comes first, and the fragment's precision before its numbers");
    assert!(fragment.find("precision").unwrap() < fragment.find("const").unwrap());
    let defined: Vec<&str> = constants.lines().filter_map(|line| line.strip_prefix("const ")).filter_map(|rest| rest.split_whitespace().nth(1)).collect();
    assert!(defined.len() >= 17, "{defined:?}");
    //a name that is all capitals in the code is one of those numbers
    for (which, code) in [("vertex", VERTEX), ("fragment", FRAGMENT)] {
        let mut word = String::new();
        let mut used = Vec::new();
        for c in code.chars().chain(std::iter::once(' ')) {
            if c.is_ascii_alphanumeric() || c == '_' {
                word.push(c);
            } else {
                if word.len() > 1 && word.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                    used.push(word.clone());
                }
                word.clear();
            }
        }
        for name in &used {
            assert!(defined.contains(&name.as_str()), "{which} shader uses {name}, which is not defined");
        }
    }
    //none of the numbers is left over from a format string that was not filled in, and they are all written as floats
    assert!(!constants.contains('{') && !constants.contains("NaN") && !constants.contains("inf"));
    assert!(constants.contains(&format!("const float SPAN = {SHIP_SPAN:?};")));
}

#[test]
fn the_nose_is_the_last_thing_a_disc_holds() {
    //the vertex attributes are read in the order of the fields of a disc, so the nose is the last, eight bytes in at the end of thirty-two
    assert_eq!(std::mem::offset_of!(Disc, border), 16);
    assert_eq!(std::mem::offset_of!(Disc, color), 20);
    assert_eq!(std::mem::offset_of!(Disc, nose), 24);
}

#[test]
fn the_setting_is_ships_unless_the_player_picked_circles_and_is_kept_with_the_others() {
    assert_eq!(Settings::default().unit_shape, UnitShape::Ships);
    assert_eq!(Settings::from_text("master 0.5\n").unit_shape, UnitShape::Ships, "a file from before the ships");
    assert_eq!(Settings::from_text("unit_shape Circles\n").unit_shape, UnitShape::Circles);
    assert_eq!(Settings::from_text("unit_shape Triangles\n").unit_shape, UnitShape::Ships, "what is not one is left as it was");
    let mut settings = Settings::default();
    settings.unit_shape = UnitShape::Circles;
    assert_eq!(Settings::from_text(&settings.to_text()).unit_shape, UnitShape::Circles);
    for shape in UNIT_SHAPES {
        assert_eq!(UnitShape::from_name(shape.name()), Some(shape));
        assert!(!shape.blurb().is_empty());
    }
}

#[test]
fn the_menu_row_flips_between_the_two() {
    let mut settings = Settings::default();
    assert_eq!(describe(Act::UnitShape, &settings), ("Unit shape", Look::Choice("Ships".to_owned())));
    assert!(nudge(&mut settings, Act::UnitShape, 1));
    assert_eq!(settings.unit_shape, UnitShape::Circles);
    assert!(nudge(&mut settings, Act::UnitShape, 1), "and round again");
    assert_eq!(settings.unit_shape, UnitShape::Ships);
    assert!(nudge(&mut settings, Act::UnitShape, -1));
    assert_eq!(settings.unit_shape, UnitShape::Circles);
    assert!(!explain_in(Act::UnitShape, &settings, MenuMode::Solo).is_empty());
}

// ---------- which way a ship points ----------

#[test]
fn a_nose_turns_towards_the_way_the_unit_is_going() {
    let mut nose = Vec2::new(0.0, -1.0);
    let going = Vec2::new(400.0, 0.0);
    let mut last = nose.dot(going.normalize());
    for _ in 0..60 {
        nose = turned_nose(nose, going, 1.0 / 120.0);
        let now = nose.dot(going.normalize());
        assert!(now >= last - 1e-6, "it only turns one way");
        assert!((nose.length() - 1.0).abs() < 1e-5, "and stays a direction");
        last = now;
    }
    //half a second is plenty to come round
    assert!(last > 0.98, "{nose}");
}

#[test]
fn a_nose_is_left_alone_while_the_unit_is_hardly_moving_or_there_is_no_time() {
    let nose = Vec2::new(0.6, 0.8);
    assert_eq!(turned_nose(nose, Vec2::new(UNIT_TURN_MIN_SPEED * 0.9, 0.0), 0.1), nose, "shoved about slowly in a crowd");
    assert_eq!(turned_nose(nose, Vec2::ZERO, 0.1), nose);
    assert_eq!(turned_nose(nose, Vec2::new(500.0, 0.0), 0.0), nose, "a frame of no time");
    assert_ne!(turned_nose(nose, Vec2::new(UNIT_TURN_MIN_SPEED * 1.1, 0.0), 0.1), nose);
}

#[test]
fn a_nose_never_collapses_even_going_straight_back() {
    //turning all the way round: the middle of the turn is a nose that would be zero if it were just averaged
    let nose = Vec2::new(0.0, -1.0);
    let back = turned_nose(nose, Vec2::new(0.0, 300.0), 1.0 / 9.0);
    assert!((back.length() - 1.0).abs() < 1e-5 && back.is_finite(), "{back}");
    //a big step does not overshoot
    let big = turned_nose(nose, Vec2::new(300.0, 0.0), 10.0);
    assert!((big - Vec2::new(1.0, 0.0)).length() < 1e-5, "{big}");
}

#[test]
fn a_unit_turns_as_it_moves_on_the_host() {
    let walls = Walls::default();
    let mut units = vec![unit_on(Vec2::new(500.0, 500.0))];
    assert_eq!(units[0].nose, Vec2::new(0.0, -1.0), "pointing up until it goes somewhere");
    //walking right at 300 px a second for a quarter of a second
    for _ in 0..30 {
        let started = [Vec2::new(units[0].x, units[0].y)];
        units[0].x += 300.0 / 120.0;
        settle_units(&mut units, &started, 1.0 / 120.0, &walls);
    }
    assert!(units[0].nose.x > 0.9 && units[0].nose.y.abs() < 0.4, "{}", units[0].nose);
    //and standing still it keeps pointing that way
    let nose = units[0].nose;
    for _ in 0..120 {
        let started = [Vec2::new(units[0].x, units[0].y)];
        settle_units(&mut units, &started, 1.0 / 120.0, &walls);
    }
    assert!((units[0].nose - nose).length() < 0.35, "{} then {}", nose, units[0].nose);
}
