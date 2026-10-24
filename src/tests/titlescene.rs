//! the picture on the title screen. what is drawn is not tested (there is no graphics card), but where things stand is: all of it on the screen, and none of it
//! where the words are

use super::*;

//the sizes a window or a screen is, from a small one to a big one: wide ones, and the 16:10 of a laptop
const SIZES: [(f32, f32); 7] = [(1280.0, 720.0), (1920.0, 1080.0), (1152.0, 720.0), (1366.0, 768.0), (1600.0, 900.0), (2880.0, 1800.0), (2560.0, 1080.0)];

#[test]
fn the_scene_has_the_player_an_army_enemies_and_the_sovereign() {
    let scene = TitleScene::new();
    let (units, foes) = scene.counts();
    assert!(units >= 24, "an army to look at: {units}");
    assert!((4..=8).contains(&foes), "a few small and medium enemies, not a crowd: {foes}");
    let boxes = TitleScene::boxes(Vec2::new(1280.0, 720.0));
    assert!(boxes.iter().any(|(name, _)| *name == "the Sovereign"));
    assert!(boxes.iter().any(|(name, _)| *name == "the army"));
    //(and it takes a stride without a window)
    let mut scene = scene;
    for _ in 0..120 {
        scene.update(1.0 / 60.0);
    }
}

#[test]
fn the_scene_is_all_on_the_screen_and_none_of_it_is_over_the_words() {
    for (w, h) in SIZES {
        let screen = Vec2::new(w, h);
        let words = TitleScene::text_area_of(screen);
        for (name, rect) in TitleScene::boxes(screen) {
            //(the Sovereign is let off the top of the screen a little, as it comes down from there)
            let top = if name == "the Sovereign" { -rect.h * 0.1 } else { 0.0 };
            assert!(rect.x >= 0.0 && rect.right() <= w && rect.y >= top && rect.bottom() <= h, "{name} is off the screen at {w} x {h}: {rect:?}");
            assert!(!words.overlaps(&rect), "{name} is over the words at {w} x {h}: {rect:?} and {words:?}");
        }
    }
}

//windows that are not wide: a square one, and the halves of a screen that a window is snapped to
const NARROW: [(f32, f32); 5] = [(800.0, 800.0), (960.0, 1080.0), (1440.0, 1800.0), (720.0, 1000.0), (1000.0, 900.0)];

#[test]
fn no_ship_is_left_out_of_the_army_in_a_wide_window() {
    let (total, _) = TitleScene::new().counts();
    for (w, h) in SIZES {
        let (_, shown) = TitleScene::player_and_ships_shown(Vec2::new(w, h));
        assert_eq!(shown, total, "a ship is missing in a wide window, {w} x {h}");
    }
}

#[test]
fn in_a_narrow_window_the_player_and_most_of_the_army_are_clear_of_the_prompts() {
    for (w, h) in NARROW {
        let screen = Vec2::new(w, h);
        let (player, shown) = TitleScene::player_and_ships_shown(screen);
        let prompts = TitleScene::prompts_of(screen);
        assert!(player.x > 0.0 && player.x < w && player.y > 0.0 && player.y < h, "the player is off the screen at {w} x {h}: {player:?}");
        assert!(!prompts.contains(player), "the player is over the prompts at {w} x {h}: {player:?} and {prompts:?}");
        assert!(shown >= 10, "too few of the ships are left to be an army at {w} x {h}: {shown}");
        //(and the quit button, which is in the corner under the army)
        assert!(player.y < h * 0.9 - 24.0, "the player is down on the quit button at {w} x {h}: {player:?}");
    }
}

#[test]
fn the_sovereign_is_above_the_title_and_the_army_below_it_on_the_other_side() {
    for (w, h) in SIZES {
        let boxes = TitleScene::boxes(Vec2::new(w, h));
        let find = |wanted: &str| boxes.iter().find(|(name, _)| *name == wanted).map(|(_, rect)| *rect).expect("in the scene");
        let (sovereign, army) = (find("the Sovereign"), find("the army"));
        //(the title's letters are from about a third of the way down the screen)
        assert!(sovereign.bottom() < h * 0.34, "the Sovereign is off the title at {w} x {h}: {sovereign:?}");
        assert!(sovereign.x > w / 2.0 && army.right() < w / 2.0, "the Sovereign is on the right and the army on the left at {w} x {h}");
        assert!(army.y > h * 0.5, "the army is in the lower half at {w} x {h}");
    }
}
