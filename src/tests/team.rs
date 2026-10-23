//a game with others in it, as it looks to each of them: the armies in the colours of their players, who is where, and the list of everyone

use super::*;

const VIEW: Rect = Rect { x: 0.0, y: 0.0, w: 1000.0, h: 1000.0 };

fn colors() -> Vec<Color> {
    PLAYER_COLORS.to_vec()
}

fn member(slot: u8, name: &str, ping_ms: u16) -> Member {
    Member { slot, name: name.to_owned(), ping_ms }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

// ---------- the colour of an army ----------

#[test]
fn an_army_is_tinted_half_way_towards_the_colour_of_its_player() {
    let tinted = team_color(WHITE, ORANGE_PLAYER);
    assert!(near(tinted.r, 1.0 + (ORANGE_PLAYER.r - 1.0) * TEAM_TINT));
    assert!(near(tinted.g, 1.0 + (ORANGE_PLAYER.g - 1.0) * TEAM_TINT));
    assert!(near(tinted.b, 1.0 + (ORANGE_PLAYER.b - 1.0) * TEAM_TINT));
    assert_eq!(team_color(WHITE, WHITE), WHITE);
    assert!(TEAM_TINT > 0.0 && TEAM_TINT < 1.0, "the units stay their own, with a hint of whose they are");
}

const ORANGE_PLAYER: Color = PLAYER_COLORS[1];

#[test]
fn the_colour_of_each_player_is_listed_in_the_order_they_joined() {
    let (players, _) = pair();
    assert_eq!(team_colors(&players), vec![PLAYER_COLORS[0], PLAYER_COLORS[1]]);
}

#[test]
fn a_game_of_one_looks_as_it_always_has() {
    let mut hurt = unit_at(100.0, 100.0);
    hurt.health = 20.0;
    let mut shielded = unit_at(200.0, 100.0);
    shielded.guard = 5.0;
    let mut sent = unit_at(300.0, 100.0);
    sent.target = Some(1);
    let units = vec![hurt, shielded, sent, unit_at(400.0, 100.0)];
    for unit in &units {
        assert_eq!(unit_color_in(unit, None), unit_color(unit));
    }
    let plain = unit_discs(&units, VIEW);
    let again = unit_discs_in(&units, VIEW, None, UnitShape::Circles);
    assert_eq!(plain, again);
    assert_eq!(unit_color(&units[3]), WHITE, "and an unhurt unit is white");
}

#[test]
fn each_army_is_drawn_in_the_colour_of_its_own_player() {
    let colors = colors();
    let ada = unit_at(100.0, 100.0);
    let bo = unit_at(200.0, 100.0).owned_by(1);
    let cy = unit_at(300.0, 100.0).owned_by(2);
    let ada_color = unit_color_in(&ada, Some(&colors));
    let bo_color = unit_color_in(&bo, Some(&colors));
    let cy_color = unit_color_in(&cy, Some(&colors));
    assert_eq!(ada_color, team_color(WHITE, colors[0]));
    assert_eq!(bo_color, team_color(WHITE, colors[1]));
    assert_eq!(cy_color, team_color(WHITE, colors[2]));
    assert!(ada_color != bo_color && bo_color != cy_color && ada_color != cy_color, "three armies, three colours");
    //(the discs say the same as the colours do)
    let discs = unit_discs_in(&[ada.clone(), bo.clone(), cy.clone()], VIEW, Some(&colors), UnitShape::Circles);
    assert_eq!(discs.len(), 3);
    for (disc, unit) in discs.iter().zip([&ada, &bo, &cy]) {
        assert_eq!(disc.color, Disc::new(0.0, 0.0, 1.0, unit_color_in(unit, Some(&colors))).color);
    }
}

#[test]
fn being_hurt_or_sent_at_an_enemy_still_shows_on_a_tinted_army() {
    let colors = colors();
    let healthy = unit_at(100.0, 100.0).owned_by(1);
    let mut hurt = healthy.clone();
    hurt.health = healthy.max_health() * 0.2;
    assert_ne!(unit_color_in(&hurt, Some(&colors)), unit_color_in(&healthy, Some(&colors)));
    assert!(unit_color_in(&hurt, Some(&colors)).r > unit_color_in(&hurt, Some(&colors)).g, "going red");
    //a squad's colour is a squad's, which is a colour the tint doesn't hide: it is the same for every player's, and not the colour of a unit that is not in one
    let mut sent = healthy.clone();
    sent.target = Some(7);
    assert_ne!(unit_color_in(&sent, Some(&colors)), unit_color_in(&healthy, Some(&colors)));
}

#[test]
fn a_unit_marked_for_a_player_who_is_not_there_takes_the_last_colour() {
    let colors = colors();
    let stray = unit_at(100.0, 100.0).owned_by(3);
    let two = &colors[..2];
    assert_eq!(unit_color_in(&stray, Some(two)), team_color(WHITE, colors[1]));
}

#[test]
fn a_champion_keeps_its_own_colour_and_wears_the_ring_of_its_player() {
    let colors = colors();
    let mut champion = unit_at(100.0, 100.0).owned_by(1);
    champion.champion = 1;
    assert_eq!(unit_color_in(&champion, Some(&colors)), unit_color(&champion), "the tint is for the ordinary units");
}

// ---------- the list of players ----------

#[test]
fn the_others_are_shown_with_their_names_colours_and_the_size_of_their_armies() {
    let (players, _) = pair();
    let members = [member(0, "Ada", 0), member(1, "Bo", 42)];
    let mates = mates_of(&players, &[12_500, 800], &members, 0);
    assert_eq!(mates.len(), 2);
    assert_eq!((mates[0].name.as_str(), mates[1].name.as_str()), ("Ada", "Bo"));
    assert_eq!((mates[0].color, mates[1].color), (PLAYER_COLORS[0], PLAYER_COLORS[1]));
    assert_eq!((mates[0].position, mates[1].position), (ADA, BO));
    assert_eq!((mates[0].army, mates[1].army), (12_500, 800));
    assert!(mates[0].you && !mates[1].you, "only one of them is the player looking");
}

#[test]
fn the_list_is_the_same_whoever_looks_at_it_but_for_who_is_you() {
    let (players, _) = pair();
    let members = [member(0, "Ada", 0), member(1, "Bo", 42)];
    let from_ada = mates_of(&players, &[10, 20], &members, 0);
    let from_bo = mates_of(&players, &[10, 20], &members, 1);
    assert_eq!(from_ada.iter().map(|m| &m.name).collect::<Vec<_>>(), from_bo.iter().map(|m| &m.name).collect::<Vec<_>>());
    assert_eq!(from_ada.iter().map(|m| m.you).collect::<Vec<_>>(), [true, false]);
    assert_eq!(from_bo.iter().map(|m| m.you).collect::<Vec<_>>(), [false, true]);
}

#[test]
fn how_far_away_each_is_is_shown_for_the_others_but_not_for_the_host_or_the_player_looking() {
    let (players, _) = pair();
    let members = [member(0, "Ada", 0), member(1, "Bo", 42)];
    let mates = mates_of(&players, &[1, 1], &members, 0);
    assert_eq!(mates[0].ping_ms, None, "nothing is between the host and the world");
    assert_eq!(mates[1].ping_ms, Some(42));
    //at the other machine the player looking is the one that is not shown a ping for themselves
    let mates = mates_of(&players, &[1, 1], &members, 1);
    assert_eq!(mates[0].ping_ms, None);
    assert_eq!(mates[1].ping_ms, None);
}

#[test]
fn a_player_who_is_not_on_a_connection_is_away_but_the_host_never_is() {
    let (players, _) = pair();
    let mates = mates_of(&players, &[5, 5], &[member(0, "Ada", 0)], 0);
    assert!(!mates[0].away, "the host is where the world is");
    assert!(mates[1].away, "Bo has gone, and their army is held for them");
    assert_eq!(mates[1].ping_ms, None);
    //names are matched whatever their capitals
    let mates = mates_of(&players, &[5, 5], &[member(0, "ADA", 0), member(1, "bo", 30)], 0);
    assert!(!mates[1].away);
    assert_eq!(mates[1].ping_ms, Some(30));
}

#[test]
fn a_player_with_relics_to_choose_from_is_said_to_be_choosing() {
    let (mut players, _) = pair();
    players.list[1].offer(RelicChoice::new());
    let mates = mates_of(&players, &[1, 1], &[member(0, "Ada", 0), member(1, "Bo", 5)], 0);
    assert!(!mates[0].choosing);
    assert!(mates[1].choosing);
    players.list[1].chosen();
    let mates = mates_of(&players, &[1, 1], &[member(0, "Ada", 0), member(1, "Bo", 5)], 0);
    assert!(!mates[1].choosing, "and not when they have chosen");
}

#[test]
fn an_army_with_no_size_given_is_taken_to_be_fallen() {
    let (players, _) = pair();
    let mates = mates_of(&players, &[7], &[], 0);
    assert_eq!(mates[1].army, 0);
}

#[test]
fn the_list_of_players_fits_in_the_corner_of_any_screen() {
    for (w, h) in [(800.0, 600.0), (1280.0, 720.0), (1920.0, 1080.0), (640.0, 480.0)] {
        let screen = Vec2::new(w, h);
        let mut last_height = 0.0;
        for count in 2..=MAX_PLAYERS {
            let rect = panel_rect(count, screen);
            assert!(rect.x >= 0.0 && rect.y >= 0.0 && rect.right() <= w && rect.bottom() <= h, "{count} players on {w}x{h}: {rect:?}");
            assert!(rect.h > last_height, "a row for each player");
            last_height = rect.h;
            assert!(rect.w <= w * 0.4 + 1e-3, "it leaves most of the screen to the game");
        }
    }
}

// ---------- the pause menu in a game of others ----------

#[test]
fn a_game_of_others_is_saved_by_its_host_and_has_no_going_back_to_an_earlier_save() {
    let solo = page_rows_in(Page::Main, MenuMode::Solo);
    assert!(solo.contains(&Act::Save) && solo.contains(&Act::Load));
    let hosting = page_rows_in(Page::Main, MenuMode::Hosting);
    assert!(hosting.contains(&Act::Save) && !hosting.contains(&Act::Load), "the host keeps the game, and does not go back in it");
    let guest = page_rows_in(Page::Main, MenuMode::Guest);
    assert!(!guest.contains(&Act::Save) && !guest.contains(&Act::Load), "a joined player has nothing of the game to save");
    for mode in [MenuMode::Hosting, MenuMode::Guest] {
        let rows = page_rows_in(Page::Main, mode);
        for act in [Act::Resume, Act::OpenSettings, Act::OpenControls, Act::Title, Act::Quit] {
            assert!(rows.contains(&act), "{mode:?} has {act:?}");
        }
        for page in [Page::Settings, Page::Controls] {
            assert_eq!(page_rows_in(page, mode), page_rows_in(page, MenuMode::Solo), "the other pages are the same");
        }
    }
}

#[test]
fn the_ways_out_say_what_they_do_to_the_others() {
    let settings = Settings::default();
    let words = |act, mode| describe_in(act, &settings, mode).0;
    assert!(words(Act::Title, MenuMode::Hosting).contains("everyone"));
    assert!(words(Act::Quit, MenuMode::Hosting).contains("everyone"));
    assert!(words(Act::Title, MenuMode::Guest).contains("Leave"));
    assert!(words(Act::Quit, MenuMode::Guest).contains("Leave"));
    assert!(words(Act::Title, MenuMode::Solo).contains("Save"), "alone they still save");
    for mode in [MenuMode::Solo, MenuMode::Hosting, MenuMode::Guest] {
        for act in [Act::Resume, Act::Title, Act::Quit, Act::Back] {
            assert!(!words(act, mode).is_empty());
            assert_eq!(describe_in(act, &settings, mode).1, Look::Button);
        }
    }
    assert!(describe_in(Act::Title, &settings, MenuMode::Hosting).0.contains("Save"), "ending it keeps it");
    assert!(explain_in(Act::Title, &settings, MenuMode::Hosting).contains("saved"));
    assert!(explain_in(Act::Title, &settings, MenuMode::Guest).contains("without you"));
    assert!(explain_in(Act::Save, &settings, MenuMode::Solo).contains("save"));
}

fn enter(menu: &mut Pause, settings: &mut Settings) -> PauseEvent {
    let mut input = MenuInput::default();
    input.enter = true;
    menu.update(settings, &input, Vec2::new(1280.0, 720.0), 1.0 / 60.0)
}

fn down(menu: &mut Pause, settings: &mut Settings) {
    let mut input = MenuInput::default();
    input.down = true;
    menu.update(settings, &input, Vec2::new(1280.0, 720.0), 1.0 / 60.0);
}

fn menu_on(mode: MenuMode, act: Act) -> Pause {
    let mut menu = Pause::new(None, Vec2::ZERO);
    menu.mode = mode;
    menu.selected = menu.rows().iter().position(|&other| other == act).unwrap();
    menu
}

#[test]
fn the_host_has_to_ask_twice_to_end_the_game_for_everyone() {
    for (act, expected) in [(Act::Title, PauseEvent::Title), (Act::Quit, PauseEvent::Quit)] {
        let mut settings = Settings::default();
        let mut menu = menu_on(MenuMode::Hosting, act);
        assert_eq!(enter(&mut menu, &mut settings), PauseEvent::Nothing, "{act:?} the first time only asks");
        assert!(menu.notice().is_some_and(|text| text.contains("again")), "and says so");
        assert_eq!(enter(&mut menu, &mut settings), expected, "{act:?} the second time does it");
    }
}

#[test]
fn asking_to_end_the_game_is_forgotten_when_the_host_goes_to_another_row() {
    let mut settings = Settings::default();
    let mut menu = menu_on(MenuMode::Hosting, Act::Title);
    assert_eq!(enter(&mut menu, &mut settings), PauseEvent::Nothing);
    down(&mut menu, &mut settings); //to Quit
    assert_eq!(enter(&mut menu, &mut settings), PauseEvent::Nothing, "Quit has not been asked for yet");
    assert_eq!(enter(&mut menu, &mut settings), PauseEvent::Quit);
}

#[test]
fn a_guest_leaves_at_once_and_a_game_of_one_is_not_asked_twice() {
    let mut settings = Settings::default();
    for mode in [MenuMode::Guest, MenuMode::Solo] {
        for (act, expected) in [(Act::Title, PauseEvent::Title), (Act::Quit, PauseEvent::Quit)] {
            let mut menu = menu_on(mode, act);
            assert_eq!(enter(&mut menu, &mut settings), expected, "{mode:?} {act:?}");
        }
    }
}

#[test]
fn resuming_and_the_settings_need_no_asking_in_a_game_of_others() {
    let mut settings = Settings::default();
    let mut menu = menu_on(MenuMode::Hosting, Act::Resume);
    assert_eq!(enter(&mut menu, &mut settings), PauseEvent::Resume);
    let mut menu = menu_on(MenuMode::Guest, Act::OpenSettings);
    assert_eq!(enter(&mut menu, &mut settings), PauseEvent::Nothing);
    assert_eq!(menu.page, Page::Settings);
}

// ---------- the arrows at the edge of the screen ----------

#[test]
fn an_arrow_that_would_land_on_something_over_the_game_slides_along_the_edge_off_it() {
    let screen = Vec2::new(1280.0, 720.0);
    let minimap = Minimap::new(screen, MapView::Near, Vec2::ZERO).rect;
    //on the right edge, beside the map: it goes up the edge, which is the shorter way
    let up = slid_clear_of(Vec2::new(1230.0, minimap.y + 30.0), minimap, screen);
    assert_eq!(up.x, 1230.0);
    assert!(up.y < minimap.y);
    //on the bottom edge, below the map's top: it goes left along the edge
    let left = slid_clear_of(Vec2::new(minimap.x + 30.0, 674.0), minimap, screen);
    assert_eq!(left.y, 674.0);
    assert!(left.x < minimap.x);
    //and not off the screen: the ways that lead off it are not taken
    let corner = slid_clear_of(Vec2::new(1270.0, 710.0), minimap, screen);
    assert!(corner.x >= 0.0 && corner.y >= 0.0 && corner.x <= screen.x && corner.y <= screen.y && !minimap.contains(corner));
    //one that is not on it stays where it is
    let clear = Vec2::new(100.0, 400.0);
    assert_eq!(slid_clear_of(clear, minimap, screen), clear);
}

#[test]
fn the_list_of_players_is_kept_clear_of_by_the_arrows_only_when_it_is_there() {
    let screen = Vec2::new(1280.0, 720.0);
    list_players(0);
    assert_eq!(team_panel(screen), None);
    list_players(1);
    assert_eq!(team_panel(screen), None, "a game of one has no list");
    list_players(3);
    assert_eq!(team_panel(screen), Some(panel_rect(3, screen)));
    list_players(0);
}

fn mate_at(name: &str, at: Vec2) -> Mate {
    Mate { name: name.to_owned(), color: WHITE, position: at, army: 100, you: false, choosing: false, ping_ms: None, away: false }
}

#[test]
fn others_whose_arrows_would_be_on_top_of_each_other_share_one_with_all_their_names() {
    let screen = Vec2::new(1280.0, 720.0);
    let camera = Vec2::new(5000.0, 5000.0);
    let mates = [
        mate_at("Ada", camera),
        mate_at("Bo", camera + Vec2::new(3000.0, 10.0)),
        mate_at("Cy", camera + Vec2::new(2000.0, 5.0)),
        mate_at("Di", camera + Vec2::new(-2500.0, 0.0)),
    ];
    let arrows = arrow_groups(&mates, camera, 1.0, screen);
    assert_eq!(arrows.len(), 2, "one to the right and one to the left");
    assert_eq!(arrows[0].2, "BO, CY");
    assert_eq!(arrows[0].0, mates[1].position, "it points at the first of them");
    assert_eq!(arrows[1].2, "DI");
}

#[test]
fn others_whose_arrows_are_apart_have_one_each() {
    let camera = Vec2::new(5000.0, 5000.0);
    let mates = [mate_at("Bo", camera + Vec2::new(3000.0, 0.0)), mate_at("Cy", camera + Vec2::new(3000.0, 900.0))];
    assert_eq!(arrow_groups(&mates, camera, 1.0, Vec2::new(1280.0, 720.0)).len(), 2);
}

#[test]
fn nobody_who_is_on_the_screen_or_is_you_or_is_away_gets_an_arrow() {
    let camera = Vec2::new(5000.0, 5000.0);
    let screen = Vec2::new(1280.0, 720.0);
    let mut away = mate_at("Away", camera + Vec2::new(4000.0, 0.0));
    away.away = true;
    let mut me = mate_at("Me", camera + Vec2::new(4000.0, 100.0));
    me.you = true;
    let mates = [mate_at("Near", camera + Vec2::new(100.0, 50.0)), away, me, mate_at("Far", camera + Vec2::new(0.0, -4000.0))];
    let arrows = arrow_groups(&mates, camera, 1.0, screen);
    assert_eq!(arrows.len(), 1);
    assert_eq!(arrows[0].2, "FAR");
    //(zoomed out far enough for the one who was far to be on the screen, none is needed)
    assert!(arrow_groups(&mates, camera, 0.05, screen).is_empty());
}
