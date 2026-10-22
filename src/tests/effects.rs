//screen shake, numbers and particles

use super::*;

// ---------- effects ----------

#[test]
fn numbers_are_shortened_to_be_read_at_a_glance() {
    assert_eq!(compact_number(0.0), "0");
    assert_eq!(compact_number(999.0), "999");
    assert_eq!(compact_number(1234.0), "1.2k");
    assert_eq!(compact_number(56789.0), "57k");
    assert_eq!(compact_number(2_500_000.0), "2.5M");
}

#[test]
fn the_screen_shakes_in_proportion_to_the_square_of_how_hard_and_the_same_on_screen_at_any_zoom() {
    assert_eq!(shake_offset(0.0, 1.0), Vec2::ZERO);
    for _ in 0..20 {
        let full = shake_offset(1.0, 0.5).length();
        assert!((full - SHAKE_PIXELS / 0.5).abs() < 0.01);
        let quarter = shake_offset(0.5, 1.0).length();
        assert!((quarter - SHAKE_PIXELS * 0.25).abs() < 0.01);
    }
    //zoomed in the camera moves less in the world, the same on the screen
    assert!((shake_offset(1.0, 2.0).length() * 2.0 - SHAKE_PIXELS).abs() < 0.01);
}

#[test]
fn shaking_settles_to_nothing_and_never_goes_past_full() {
    let mut fx = Effects::default();
    fx.add_shake(0.8);
    fx.add_shake(0.8);
    assert_eq!(fx.shake, 1.0);
    let mut last = fx.shake;
    for _ in 0..600 {
        fx.update(1.0 / 60.0, Vec2::ZERO);
        assert!(fx.shake <= last);
        last = fx.shake;
    }
    assert_eq!(fx.shake, 0.0);
}

#[test]
fn effects_are_capped_so_a_huge_fight_cannot_flood_the_frame() {
    let mut fx = Effects::default();
    for _ in 0..(MAX_POPUPS + MAX_BURSTS + MAX_CUES) * 2 {
        fx.popup(Vec2::ZERO, 0.0, 0.0, "x".to_owned(), WHITE);
        fx.burst(Vec2::ZERO, WHITE, 5, 100.0);
        fx.cue(Cue::Pop);
    }
    assert_eq!(fx.popups.len(), MAX_POPUPS);
    assert_eq!(fx.bursts.len(), MAX_BURSTS);
    assert_eq!(fx.cues.len(), MAX_CUES);
    //bursts become particles, but not without limit
    let mut particles = Vec::new();
    for _ in 0..200 {
        fx.burst(Vec2::ZERO, WHITE, 40, 100.0);
        fx.spawn_bursts(&mut particles);
    }
    assert!(particles.len() <= MAX_PARTICLES * 3 / 2);
    assert!(fx.bursts.is_empty());
}

#[test]
fn numbers_float_up_for_a_second_and_are_gone() {
    let mut fx = Effects::default();
    fx.popup(Vec2::new(10.0, 10.0), 0.0, 0.0, "hi".to_owned(), WHITE);
    fx.update(POPUP_LIFE * 0.9, Vec2::ZERO);
    assert_eq!(fx.popups.len(), 1);
    fx.update(POPUP_LIFE * 0.2, Vec2::ZERO);
    assert!(fx.popups.is_empty());
}

#[test]
fn babies_that_grow_are_shown_as_one_number_every_so_often_not_one_each() {
    let player = Vec2::new(500.0, 500.0);
    let mut fx = Effects::default();
    fx.picked_up = 7;
    fx.update(1.0 / 60.0, player);
    assert_eq!(fx.popups.len(), 1);
    assert_eq!(fx.popups[0].text, "+7");
    assert_eq!(fx.picked_up, 0);
    //more grow straight after: they wait their turn, and are one number again
    fx.picked_up = 3;
    fx.update(1.0 / 60.0, player);
    assert_eq!(fx.popups.len(), 1);
    fx.update(POPUP_INTERVAL, player);
    assert_eq!(fx.popups.len(), 2);
    assert_eq!(fx.popups[1].text, "+3");
}

#[test]
fn an_enemy_shows_what_it_has_suffered_every_so_often_and_then_starts_counting_again() {
    let at = Vec2::new(16000.0, 16000.0);
    let mut enemy = plain_enemy(at);
    enemy.hurt = 1234.0;
    enemy.killed = 3;
    enemy.popup_in = 0.0;
    let mut enemies = vec![enemy];
    let mut fx = Effects::default();
    tick_enemy_numbers(&mut enemies, 1.0 / 60.0, &mut fx);
    let texts: Vec<&str> = fx.popups.iter().map(|popup| popup.text.as_str()).collect();
    assert_eq!(texts, vec!["-1.2k", "-3 units"]);
    assert_eq!(enemies[0].hurt, 0.0);
    assert_eq!(enemies[0].killed, 0);
    assert!(enemies[0].popup_in > 0.3);
    //nothing new has happened to it
    tick_enemy_numbers(&mut enemies, POPUP_INTERVAL * 2.0, &mut fx);
    assert_eq!(fx.popups.len(), 2);
}

