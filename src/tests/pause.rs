//the pause menu: moving about it with keys and the mouse, changing what is on a row, and how it fits on a screen

use super::*;

const SCREEN: Vec2 = Vec2::new(1280.0, 720.0);

fn saved() -> Option<SaveInfo> {
    Some(SaveInfo { saved_at: 1000.0, army: 1234, seconds: 60.0, bosses: 1 })
}

fn press(menu: &mut Pause, settings: &mut Settings, input: MenuInput) -> PauseEvent {
    menu.update(settings, &input, SCREEN, 1.0 / 60.0)
}

fn key(f: impl FnOnce(&mut MenuInput)) -> MenuInput {
    let mut input = MenuInput::default();
    f(&mut input);
    input
}

//the row of a page that does something
fn row_of(page: Page, act: Act) -> usize {
    page_rows(page).iter().position(|&other| other == act).unwrap()
}

#[test]
fn every_row_is_on_a_page_and_says_what_it_is() {
    let settings = Settings::default();
    let all = [
        Act::Resume, Act::Save, Act::Load, Act::OpenSettings, Act::OpenControls, Act::Title, Act::Quit, Act::Back, Act::Master, Act::Effects, Act::Music, Act::Shake,
        Act::Fullscreen, Act::Colours, Act::Hints, Act::Difficulty, Act::Formation, Act::ArmyLimit, Act::ResetArmyLimit,
    ];
    for act in all {
        assert!([Page::Main, Page::Settings, Page::Controls].iter().any(|&page| page_rows(page).contains(&act)), "{act:?} is on no page");
        assert!(!describe(act, &settings).0.is_empty());
    }
    for page in [Page::Settings, Page::Controls] {
        assert_eq!(*page_rows(page).last().unwrap(), Act::Back, "every page leads back");
    }
    assert!(CONTROLS.iter().all(|(key, what)| !key.is_empty() && !what.is_empty()));
}

#[test]
fn the_menu_fits_on_any_screen_the_game_can_be_played_on() {
    for (w, h) in [(800.0, 600.0), (1024.0, 600.0), (1280.0, 720.0), (1920.0, 1080.0), (2560.0, 1440.0), (640.0, 480.0)] {
        let screen = Vec2::new(w, h);
        for page in [Page::Main, Page::Settings, Page::Controls] {
            let frame = layout(page, page_rows(page).len(), screen);
            let (panel, rows) = (frame.panel, &frame.rows);
            assert!(panel.x >= 0.0 && panel.y >= 0.0 && panel.right() <= w && panel.bottom() <= h, "{page:?} on {w}x{h}: {panel:?}");
            assert!(rows.iter().all(|row| row.x >= panel.x && row.right() <= panel.right() && row.y >= panel.y && row.bottom() <= panel.bottom()), "{page:?} on {w}x{h}");
            assert!(rows.windows(2).all(|pair| pair[0].bottom() <= pair[1].y), "no row sits on the next");
            assert!(rows.iter().all(|row| row.h >= 24.0), "big enough to click: {page:?} on {w}x{h}");
        }
    }
}

#[test]
fn up_and_down_move_about_the_rows_and_come_round_at_the_ends() {
    let mut menu = Pause::new(saved(), Vec2::ZERO);
    let mut settings = Settings::default();
    let count = page_rows(Page::Main).len();
    assert_eq!(menu.selected, 0);
    press(&mut menu, &mut settings, key(|i| i.up = true));
    assert_eq!(menu.selected, count - 1);
    press(&mut menu, &mut settings, key(|i| i.down = true));
    assert_eq!(menu.selected, 0);
    press(&mut menu, &mut settings, key(|i| i.down = true));
    assert_eq!(menu.selected, 1);
}

#[test]
fn escape_closes_the_menu_from_the_first_page_and_goes_back_from_the_others() {
    let mut menu = Pause::new(saved(), Vec2::ZERO);
    let mut settings = Settings::default();
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.back = true)), PauseEvent::Resume);
    menu.selected = row_of(Page::Main, Act::OpenSettings);
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.enter = true)), PauseEvent::Nothing);
    assert_eq!(menu.page, Page::Settings);
    assert_eq!(menu.selected, 0, "the new page starts at its top");
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.back = true)), PauseEvent::Nothing);
    assert_eq!(menu.page, Page::Main);
    menu.selected = row_of(Page::Main, Act::OpenControls);
    press(&mut menu, &mut settings, key(|i| i.enter = true));
    assert_eq!(menu.page, Page::Controls);
    press(&mut menu, &mut settings, key(|i| i.enter = true));
    assert_eq!(menu.page, Page::Main, "Back is the one row of the controls page");
}

#[test]
fn the_rows_of_the_first_page_do_what_they_say() {
    let mut settings = Settings::default();
    for (act, expected) in [(Act::Resume, PauseEvent::Resume), (Act::Save, PauseEvent::Save), (Act::Load, PauseEvent::Load), (Act::Title, PauseEvent::Title), (Act::Quit, PauseEvent::Quit)] {
        let mut menu = Pause::new(saved(), Vec2::ZERO);
        menu.selected = row_of(Page::Main, act);
        assert_eq!(press(&mut menu, &mut settings, key(|i| i.enter = true)), expected, "{act:?}");
    }
    //there is nothing to load if there is no save
    let mut menu = Pause::new(None, Vec2::ZERO);
    menu.selected = row_of(Page::Main, Act::Load);
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.enter = true)), PauseEvent::Nothing);
}

#[test]
fn the_mouse_chooses_the_row_it_is_over_and_a_click_does_it() {
    let mut menu = Pause::new(saved(), Vec2::ZERO);
    let mut settings = Settings::default();
    let frame = layout(Page::Main, page_rows(Page::Main).len(), SCREEN);
    let save_row = row_of(Page::Main, Act::Save);
    let over = frame.rows[save_row].center();
    press(&mut menu, &mut settings, key(|i| {
        i.mouse = over;
        i.moved = true;
    }));
    assert_eq!(menu.selected, save_row);
    //a mouse that is not moving doesn't take the choice from the keys
    press(&mut menu, &mut settings, key(|i| {
        i.mouse = over;
        i.down = true;
    }));
    assert_eq!(menu.selected, save_row + 1);
    let event = press(&mut menu, &mut settings, key(|i| {
        i.mouse = over;
        i.click = true;
        i.held = true;
    }));
    assert_eq!(event, PauseEvent::Save);
    //a click on no row does nothing
    let event = press(&mut menu, &mut settings, key(|i| {
        i.mouse = Vec2::new(2.0, 2.0);
        i.click = true;
    }));
    assert_eq!(event, PauseEvent::Nothing);
}

#[test]
fn left_and_right_change_a_setting_and_report_it_when_they_do() {
    let mut menu = Pause::new(saved(), Vec2::ZERO);
    let mut settings = Settings::default();
    menu.page = Page::Settings;
    menu.selected = row_of(Page::Settings, Act::Master);
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.right = true)), PauseEvent::Nothing, "already as loud as it gets");
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.left = true)), PauseEvent::Changed);
    assert!((settings.master - 0.95).abs() < 1e-4, "{}", settings.master);
    for _ in 0..40 {
        press(&mut menu, &mut settings, key(|i| i.left = true));
    }
    assert_eq!(settings.master, 0.0, "it stops at nothing");
    menu.selected = row_of(Page::Settings, Act::Colours);
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.enter = true)), PauseEvent::Changed);
    assert!(settings.colourblind);
    menu.selected = row_of(Page::Settings, Act::Fullscreen);
    press(&mut menu, &mut settings, key(|i| i.right = true));
    assert!(!settings.fullscreen);
    menu.selected = row_of(Page::Settings, Act::Difficulty);
    press(&mut menu, &mut settings, key(|i| i.right = true));
    assert_eq!(settings.difficulty, Difficulty::Hard);
    press(&mut menu, &mut settings, key(|i| i.left = true));
    assert_eq!(settings.difficulty, Difficulty::Normal);
    menu.selected = row_of(Page::Settings, Act::Formation);
    press(&mut menu, &mut settings, key(|i| i.right = true));
    assert_eq!(settings.formation, Formation::Ring);
    press(&mut menu, &mut settings, key(|i| i.left = true));
    press(&mut menu, &mut settings, key(|i| i.left = true));
    assert_eq!(settings.formation, Formation::Line, "and back round the other way");
}

#[test]
fn a_slider_follows_the_mouse_while_the_button_is_held_and_lets_go_when_it_is_not() {
    let mut menu = Pause::new(saved(), Vec2::ZERO);
    let mut settings = Settings::default();
    menu.page = Page::Settings;
    let frame = layout(Page::Settings, page_rows(Page::Settings).len(), SCREEN);
    let row = frame.rows[row_of(Page::Settings, Act::Music)];
    let bar = slider_bar(row);
    let at = |share: f32| Vec2::new(bar.x + bar.w * share, row.center().y);
    let event = press(&mut menu, &mut settings, key(|i| {
        i.mouse = at(0.5);
        i.click = true;
        i.held = true;
    }));
    assert_eq!(event, PauseEvent::Changed);
    assert!((settings.music - 0.5).abs() < 0.03, "{}", settings.music);
    //dragged off the row and past the end, it still follows, to the end
    press(&mut menu, &mut settings, key(|i| {
        i.mouse = Vec2::new(bar.right() + 300.0, row.y - 200.0);
        i.held = true;
    }));
    assert_eq!(settings.music, 1.0);
    press(&mut menu, &mut settings, key(|i| {
        i.mouse = at(0.25);
        i.held = true;
    }));
    assert!((settings.music - 0.25).abs() < 0.03);
    //let go: the slider stays where it was left
    press(&mut menu, &mut settings, key(|i| i.mouse = at(0.9)));
    assert!((settings.music - 0.25).abs() < 0.03, "{}", settings.music);
}

#[test]
fn the_levels_of_a_slider_are_in_steps_of_five_percent() {
    let mut settings = Settings::default();
    for level in [0.0, 0.013, 0.49, 0.5, 0.97, 1.0, 3.0, -2.0] {
        set_level(&mut settings, Act::Shake, level);
        let steps = settings.shake * 20.0;
        assert!((steps - steps.round()).abs() < 1e-3 && (0.0..=1.0).contains(&settings.shake), "{level} gave {}", settings.shake);
    }
    assert!(!set_level(&mut settings, Act::Fullscreen, 0.5), "not a slider");
}

#[test]
fn the_time_since_a_save_is_said_in_the_biggest_unit_that_fits() {
    assert_eq!(ago(5.0), "5s");
    assert_eq!(ago(-3.0), "0s");
    assert_eq!(ago(300.0), "5 min");
    assert_eq!(ago(3.0 * 3600.0), "3 h");
    assert_eq!(ago(3.0 * 86400.0), "3 days");
}

#[test]
fn a_choice_can_be_taken_back_from_either_end_with_either_key() {
    for start in DIFFICULTIES {
        let mut menu = Pause::new(saved(), Vec2::ZERO);
        let mut settings = Settings { difficulty: start, ..Settings::default() };
        menu.page = Page::Settings;
        menu.selected = row_of(Page::Settings, Act::Difficulty);
        let at = DIFFICULTIES.iter().position(|&difficulty| difficulty == start).unwrap();
        press(&mut menu, &mut settings, key(|i| i.left = true));
        assert_eq!(settings.difficulty, DIFFICULTIES[(at + DIFFICULTIES.len() - 1) % DIFFICULTIES.len()], "left from {start:?}");
        press(&mut menu, &mut settings, key(|i| i.right = true));
        assert_eq!(settings.difficulty, start, "and right again");
        //Enter (or a click) goes on to the next, round to the first from the last
        press(&mut menu, &mut settings, key(|i| i.enter = true));
        assert_eq!(settings.difficulty, DIFFICULTIES[(at + 1) % DIFFICULTIES.len()], "enter on {start:?}");
    }
}

#[test]
fn something_the_menu_says_about_what_was_done_goes_away_after_a_while() {
    let mut menu = Pause::new(None, Vec2::ZERO);
    let mut settings = Settings::default();
    assert_eq!(menu.notice(), None);
    menu.tell("game saved", CYAN);
    let frames = (MENU_NOTICE_SECONDS * 60.0) as usize;
    for _ in 0..frames / 2 {
        press(&mut menu, &mut settings, MenuInput::default());
    }
    assert_eq!(menu.notice(), Some("game saved"), "still there half way");
    for _ in 0..frames {
        press(&mut menu, &mut settings, MenuInput::default());
    }
    assert_eq!(menu.notice(), None, "gone after its time");
}

#[test]
fn the_army_limit_moves_a_thousand_at_a_time_and_stops_at_both_ends() {
    let mut menu = Pause::new(saved(), Vec2::ZERO);
    let mut settings = Settings::default();
    menu.page = Page::Settings;
    menu.selected = row_of(Page::Settings, Act::ArmyLimit);
    assert_eq!(settings.army_limit, 15000);
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.right = true)), PauseEvent::Changed);
    assert_eq!(settings.army_limit, 16000);
    press(&mut menu, &mut settings, key(|i| i.left = true));
    press(&mut menu, &mut settings, key(|i| i.left = true));
    assert_eq!(settings.army_limit, 14000);
    for _ in 0..60 {
        press(&mut menu, &mut settings, key(|i| i.left = true));
    }
    assert_eq!(settings.army_limit, 5000, "it stops at the least");
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.left = true)), PauseEvent::Nothing);
    for _ in 0..60 {
        press(&mut menu, &mut settings, key(|i| i.right = true));
    }
    assert_eq!(settings.army_limit, 50000, "and at the most");
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.right = true)), PauseEvent::Nothing);
}

#[test]
fn the_army_limit_slider_follows_the_mouse_in_whole_steps_from_end_to_end() {
    let mut menu = Pause::new(saved(), Vec2::ZERO);
    let mut settings = Settings::default();
    menu.page = Page::Settings;
    let frame = layout(Page::Settings, page_rows(Page::Settings).len(), SCREEN);
    let row = frame.rows[row_of(Page::Settings, Act::ArmyLimit)];
    let bar = slider_bar(row);
    let at = |share: f32| Vec2::new(bar.x + bar.w * share, row.center().y);
    let event = press(&mut menu, &mut settings, key(|i| {
        i.mouse = at(0.0);
        i.click = true;
        i.held = true;
    }));
    assert_eq!((event, settings.army_limit), (PauseEvent::Changed, 5000));
    press(&mut menu, &mut settings, key(|i| {
        i.mouse = at(1.0);
        i.held = true;
    }));
    assert_eq!(settings.army_limit, 50000);
    for share in [0.0, 0.0137, 0.2222, 0.5, 0.777, 0.999, 1.0] {
        press(&mut menu, &mut settings, key(|i| {
            i.mouse = at(share);
            i.held = true;
        }));
        let limit = settings.army_limit;
        assert!((MIN_ARMY_LIMIT..=MAX_ARMY_LIMIT).contains(&limit) && limit % ARMY_LIMIT_STEP == 0, "{share} gave {limit}");
        assert!((limit_level(limit) - share).abs() <= 0.5 / 45.0 + 1e-4, "{share} gave {limit}, which is at {}", limit_level(limit));
    }
    //the knob is where the limit is, and the row says the number
    assert_eq!(limit_level(MIN_ARMY_LIMIT), 0.0);
    assert_eq!(limit_level(MAX_ARMY_LIMIT), 1.0);
    assert!((limit_level(DEFAULT_ARMY_LIMIT) - 10.0 / 45.0).abs() < 1e-6);
    settings.army_limit = 32000;
    assert!(matches!(describe(Act::ArmyLimit, &settings).1, Look::Amount(_, text) if text == "32,000"));
}

#[test]
fn the_reset_button_puts_the_army_limit_back_to_the_usual_one() {
    let mut menu = Pause::new(saved(), Vec2::ZERO);
    let mut settings = Settings { army_limit: 42000, ..Settings::default() };
    menu.page = Page::Settings;
    menu.selected = row_of(Page::Settings, Act::ResetArmyLimit);
    assert!(describe(Act::ResetArmyLimit, &settings).0.contains(&grouped(DEFAULT_ARMY_LIMIT)), "the button says what it resets to");
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.enter = true)), PauseEvent::Changed);
    assert_eq!(settings.army_limit, DEFAULT_ARMY_LIMIT);
    assert_eq!(DEFAULT_ARMY_LIMIT, 15000);
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.enter = true)), PauseEvent::Nothing, "nothing to change");
    //it changes nothing else
    assert_eq!(settings, Settings::default());
    //left and right on it do nothing
    assert_eq!(press(&mut menu, &mut settings, key(|i| i.right = true)), PauseEvent::Nothing);
}
