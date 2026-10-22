//the circles that are drawn in one go: what is sent to the graphics card for the army, the puffs and the babies. there is no
//graphics card in a test, so these are the parts that decide what is drawn rather than the drawing itself

use super::*;

#[test]
fn a_disc_is_exactly_what_the_shader_reads_for_each_one() {
    //two floats for the place, one for the radius, one for the thickness, one for the border and four bytes of colour
    assert_eq!(std::mem::size_of::<Disc>(), 24);
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
