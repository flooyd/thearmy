//! the picture on the title screen, in each of its looks. what is drawn is not tested (there is no graphics card), but what each look is made of is, and where things
//! stand: all of it on the screen, the player whole, and none of it where the words are

use super::*;

//the sizes a window or a screen is, from a small one to a big one: wide ones, and the 16:10 of a laptop
const SIZES: [(f32, f32); 7] = [(1280.0, 720.0), (1920.0, 1080.0), (1152.0, 720.0), (1366.0, 768.0), (1600.0, 900.0), (2880.0, 1800.0), (2560.0, 1080.0)];

//windows that are not wide: a square one, and the halves of a screen that a window is snapped to
const NARROW: [(f32, f32); 5] = [(800.0, 800.0), (960.0, 1080.0), (1440.0, 1800.0), (720.0, 1000.0), (1000.0, 900.0)];

fn looks() -> std::ops::Range<usize> {
    0..HOMESCREENS.len()
}

#[test]
fn there_are_three_to_five_looks_each_with_its_own_name() {
    assert!((3..=5).contains(&HOMESCREENS.len()), "{} looks", HOMESCREENS.len());
    let mut names: Vec<&str> = HOMESCREENS.iter().map(|home| home.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), HOMESCREENS.len(), "no two looks are called the same");
}

#[test]
fn the_looks_are_light_and_dark_and_have_different_bosses_armies_and_words() {
    let light = HOMESCREENS.iter().filter(|home| home.theme.light).count();
    assert!(light >= 2 && HOMESCREENS.len() - light >= 2, "{light} light of {}", HOMESCREENS.len());
    let mut bosses: Vec<&str> = HOMESCREENS.iter().map(|home| home.boss).collect();
    bosses.sort_unstable();
    bosses.dedup();
    assert!(bosses.len() >= HOMESCREENS.len() - 1, "the bosses are nearly all different: {bosses:?}");
    //(the colours of the words stand out from the backdrop: a light look has dark words and a dark one light words)
    for home in &HOMESCREENS {
        let (theme, bright) = (&home.theme, |color: Color| (color.r + color.g + color.b) / 3.0);
        let backdrop = (bright(theme.top) + bright(theme.bottom)) / 2.0;
        assert_eq!(theme.light, backdrop > 0.5, "{} is {} but its backdrop is {backdrop}", home.name, if theme.light { "light" } else { "dark" });
        for (words, color) in [("heading", theme.heading), ("ink", theme.ink), ("muted", theme.muted)] {
            assert!((bright(color) - backdrop).abs() > 0.3, "the {words} of {} is hard to read on its backdrop", home.name);
        }
    }
    //(and the player is not in the same place, or as big an army, in any two)
    for (k, one) in HOMESCREENS.iter().enumerate() {
        for other in &HOMESCREENS[k + 1..] {
            let (a, b) = (&one.army, &other.army);
            let same_place = a.at.across == b.at.across && a.at.down == b.at.down && (a.at.x - b.at.x).abs() < 40.0 && (a.at.y - b.at.y).abs() < 40.0;
            assert!(!same_place || a.ships() != b.ships(), "{} and {} have the army in the same place and the same size", one.name, other.name);
            assert!(a.ships() != b.ships() || a.zoom != b.zoom || a.faces_left != b.faces_left || a.at.across != b.at.across, "{} and {} have the same army", one.name, other.name);
        }
    }
}

#[test]
fn every_look_has_the_player_an_army_enemies_and_a_boss_that_have_art() {
    for home in looks() {
        let look = &HOMESCREENS[home];
        let scene = TitleScene::new(home);
        let (units, foes) = scene.counts();
        assert_eq!(units, look.army.ships(), "{}: every ship of the army is made", look.name);
        assert!(units >= 18, "{}: an army to look at: {units}", look.name);
        assert!((4..=8).contains(&foes), "{}: a few enemies, not a crowd: {foes}", look.name);
        assert_eq!(foes, look.foes.len());
        let boxes = TitleScene::boxes(home, Vec2::new(1280.0, 720.0));
        assert!(boxes.iter().any(|(name, _)| *name == "the boss") && boxes.iter().any(|(name, _)| *name == "the army"));
        assert!(shape_named(look.boss).is_some(), "{}: {} has no drawing", look.name, look.boss);
        assert!(BOSSES.iter().chain(std::iter::once(&FINAL_BOSS)).any(|def| def.name == look.boss), "{}: {} is no boss", look.name, look.boss);
        for placed in look.foes {
            assert!(ENEMY_KINDS.iter().any(|kind| kind.name == placed.name), "{}: {} is no kind of enemy", look.name, placed.name);
            assert!(shape_named(placed.name).is_some(), "{}: {} has no drawing", look.name, placed.name);
            assert_ne!(placed.name, "Ambusher", "{}: it hides, and would not be seen", look.name);
            assert!((0.0..=1.0).contains(&placed.health) && placed.scale > 0.0);
        }
        //(and it takes a stride without a window)
        let mut scene = scene;
        for _ in 0..120 {
            scene.update(1.0 / 60.0);
        }
    }
}

#[test]
fn the_scene_is_all_on_the_screen_and_none_of_it_is_over_the_words() {
    for home in looks() {
        for (w, h) in SIZES {
            let screen = Vec2::new(w, h);
            let words = TitleScene::words_in(home, screen);
            for (name, rect) in TitleScene::boxes(home, screen) {
                //(the boss is let off the top of the screen a little, as it comes down from there, and the army is round, so that its corners may be off it, and the
                //ships on the edge are let off the sides and the foot)
                let (top, side) = match name {
                    "the boss" => (-rect.h * 0.12, rect.w * 0.05),
                    "the army" => (-rect.h * 0.1, rect.w * 0.3),
                    _ => (0.0, 0.0),
                };
                let name = format!("{} in {}", name, HOMESCREENS[home].name);
                assert!(rect.x >= -side && rect.right() <= w + side && rect.y >= top && rect.bottom() <= h + side, "{name} is off the screen at {w} x {h}: {rect:?}");
                for words in words {
                    assert!(!words.overlaps(&rect), "{name} is over the words at {w} x {h}: {rect:?} and {words:?}");
                }
            }
        }
    }
}

#[test]
fn the_player_is_whole_on_the_screen_in_every_look_and_every_window() {
    for home in looks() {
        for (w, h) in SIZES.into_iter().chain(NARROW) {
            let screen = Vec2::new(w, h);
            let (player, _) = TitleScene::player_and_ships_shown(home, screen);
            let head = TitleScene::head_room_of(home, screen);
            let name = HOMESCREENS[home].name;
            assert!(player.y - head >= 0.0, "the player's head is cut off by the top of the window in {name} at {w} x {h}: {player:?} and {head}");
            assert!(player.y + head <= h,"the player's feet are cut off in {name} at {w} x {h}: {player:?}");
            assert!(player.x - head >= 0.0 && player.x + head <= w, "the player is cut off at a side in {name} at {w} x {h}: {player:?}");
        }
    }
}

#[test]
fn no_ship_is_left_out_of_the_army_in_a_wide_window() {
    for home in looks() {
        let (total, _) = TitleScene::new(home).counts();
        for (w, h) in SIZES {
            let (_, shown) = TitleScene::player_and_ships_shown(home, Vec2::new(w, h));
            assert_eq!(shown, total, "a ship is missing in {} in a wide window, {w} x {h}", HOMESCREENS[home].name);
        }
    }
}

#[test]
fn in_a_narrow_window_the_player_and_most_of_the_army_are_clear_of_the_prompts_and_the_menu_button() {
    for home in looks() {
        for (w, h) in NARROW {
            let screen = Vec2::new(w, h);
            let (player, shown) = TitleScene::player_and_ships_shown(home, screen);
            let name = HOMESCREENS[home].name;
            for words in TitleScene::words_in(home, screen) {
                assert!(!words.contains(player), "the player is over the words in {name} at {w} x {h}: {player:?} and {words:?}");
            }
            assert!(!menu_button_of(screen).contains(player), "the player is on the menu button in {name} at {w} x {h}: {player:?}");
            assert!(shown >= 8, "too few of the ships are left to be an army in {name} at {w} x {h}: {shown}");
        }
    }
}

#[test]
fn the_words_of_every_look_are_in_the_window_and_in_order_down_it() {
    for home in looks() {
        for (w, h) in SIZES.into_iter().chain(NARROW) {
            let screen = Vec2::new(w, h);
            let look = &HOMESCREENS[home];
            let name = look.name;
            let (center, width) = (look.words.center(screen, design_scale(screen)), look.words.width(screen));
            assert!(width > 0.0 && width <= 0.9 * w + 0.01, "{name} at {w} x {h}: {width}");
            assert!(center - width / 2.0 >= -0.01 && center + width / 2.0 <= w + 0.01, "the column of words is off the window in {name} at {w} x {h}: {center} {width}");
            let rects = TitleScene::words_in(home, screen);
            for rect in rects {
                assert!(rect.x >= -0.01 && rect.right() <= w + 0.01 && rect.y >= 0.0 && rect.bottom() <= h, "words are off the window in {name} at {w} x {h}: {rect:?}");
            }
            assert!(rects[0].y < rects[1].y && rects[1].y < rects[2].y, "the title, then the line under it, then the prompts, in {name}");
        }
    }
}

#[test]
fn the_title_and_the_boss_stand_where_the_look_says() {
    //(the boss is above the title in Ironclad, over the left in Eclipse, and on the right in Vanguard; the words are in a column at the side in Daybreak and Eclipse)
    let screen = Vec2::new(1920.0, 1080.0);
    let find = |name: &str, wanted: &str| {
        let home = HOMESCREENS.iter().position(|home| home.name == name).expect("a look");
        let boxes = TitleScene::boxes(home, screen);
        (TitleScene::words_in(home, screen), boxes.iter().find(|(thing, _)| *thing == wanted).map(|(_, rect)| *rect).expect("in the scene"))
    };
    let (words, boss) = find("Ironclad", "the boss");
    assert!(boss.bottom() <= words[0].y + 1.0, "the boss is over the title");
    let (words, boss) = find("Eclipse", "the boss");
    assert!(boss.right() <= words[0].x + 1.0 && boss.center().x < screen.x / 2.0, "the boss is on the left, off the words that are on the right");
    let (words, boss) = find("Vanguard", "the boss");
    assert!(boss.center().x > screen.x * 0.6 && boss.bottom() < words[2].y, "the boss is on the right and higher than the prompts");
    let (words, _) = find("Daybreak", "the army");
    assert!(words[0].center().x < screen.x * 0.4, "the words are in a column on the left");
}

#[test]
fn tab_goes_round_the_looks_and_a_new_screen_starts_on_one_of_them() {
    let mut seen = vec![0];
    let mut look = 0;
    for _ in 0..HOMESCREENS.len() {
        look = next_homescreen(look);
        seen.push(look);
    }
    assert_eq!(*seen.last().unwrap(), 0, "round to the first again: {seen:?}");
    seen.pop();
    seen.sort_unstable();
    assert_eq!(seen, (0..HOMESCREENS.len()).collect::<Vec<_>>(), "every look comes up");
    assert!(random_homescreen() < HOMESCREENS.len());
    //(a number that is past the last goes round)
    assert_eq!(TitleScene::new(HOMESCREENS.len() + 1).index(), 1);
}

#[test]
fn a_spot_is_from_the_edge_it_says_in_a_window_of_any_size() {
    let screen = Vec2::new(1000.0, 600.0);
    let at = |across, x, down, y| Spot { across, x, down, y }.at(screen, 2.0);
    assert_eq!(at(Edge::Near, 10.0, Edge::Near, 20.0), Vec2::new(20.0, 40.0));
    assert_eq!(at(Edge::Far, 10.0, Edge::Far, 20.0), Vec2::new(980.0, 560.0));
    assert_eq!(at(Edge::Middle, 10.0, Edge::Middle, -20.0), Vec2::new(520.0, 260.0));
    let spot = Spot { across: Edge::Far, x: 100.0, down: Edge::Near, y: 50.0 };
    assert_eq!(spot.side(screen, 1.0), Vec2::new(1.0, -1.0));
}
