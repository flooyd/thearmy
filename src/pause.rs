//! the pause menu (Esc): resume, save, load, the settings, the controls and the ways out. the world stands still while it is up

use crate::prelude::*;

//whose game the menu is in: a player's own, one that others have joined (which is not saved, and which going from ends for all of them), or one that
//the player has joined (which they can leave, and which goes on without them)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum MenuMode {
    #[default]
    Solo,
    Hosting,
    Guest,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Page {
    Main,
    Settings,
    Controls,
}

//everything a row of the menu can do
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Act {
    Resume,
    Save,
    Load,
    OpenSettings,
    OpenControls,
    Title,
    Quit,
    Back,
    Master,
    Effects,
    Music,
    Shake,
    Fullscreen,
    Colours,
    UnitShape,
    Hints,
    Difficulty,
    Formation,
    ArmyLimit,
    ResetArmyLimit,
}

//what the game is to do about what the player did in the menu
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum PauseEvent {
    Nothing,
    Resume,
    Save,
    Load,
    Title,
    Quit,
    Changed, //a setting was changed
}

//how a row looks and what is on it
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Look {
    Button,
    Slider(f32),
    Amount(f32, String), //a slider that shows a number of its own in place of a percentage
    Toggle(bool),
    Choice(String),
}

#[cfg(test)]
//the rows on a page, top to bottom
pub(crate) fn page_rows(page: Page) -> &'static [Act] {
    page_rows_in(page, MenuMode::Solo)
}

//(a game that others are in is saved by its host, and there is no going back to an earlier save in it)
pub(crate) fn page_rows_in(page: Page, mode: MenuMode) -> &'static [Act] {
    match page {
        Page::Main if mode == MenuMode::Hosting => &[Act::Resume, Act::Save, Act::OpenSettings, Act::OpenControls, Act::Title, Act::Quit],
        Page::Main if mode == MenuMode::Guest => &[Act::Resume, Act::OpenSettings, Act::OpenControls, Act::Title, Act::Quit],
        Page::Main => &[Act::Resume, Act::Save, Act::Load, Act::OpenSettings, Act::OpenControls, Act::Title, Act::Quit],
        Page::Settings => &[Act::Master, Act::Effects, Act::Music, Act::Shake, Act::Fullscreen, Act::Colours, Act::UnitShape, Act::Hints, Act::Difficulty, Act::Formation, Act::ArmyLimit, Act::ResetArmyLimit, Act::Back],
        Page::Controls => &[Act::Back],
    }
}

#[cfg(test)]
//the words on a row and what sort of row it is
pub(crate) fn describe(act: Act, settings: &Settings) -> (&'static str, Look) {
    describe_in(act, settings, MenuMode::Solo)
}

pub(crate) fn describe_in(act: Act, settings: &Settings, mode: MenuMode) -> (&'static str, Look) {
    match act {
        Act::Title if mode == MenuMode::Hosting => ("Save and end the game for everyone", Look::Button),
        Act::Quit if mode == MenuMode::Hosting => ("Save, end the game for everyone and quit", Look::Button),
        Act::Save if mode == MenuMode::Hosting => ("Save the game", Look::Button),
        Act::Title if mode == MenuMode::Guest => ("Leave the game", Look::Button),
        Act::Quit if mode == MenuMode::Guest => ("Leave the game and quit", Look::Button),
        Act::Resume => ("Resume", Look::Button),
        Act::Save => ("Save game", Look::Button),
        Act::Load => ("Load game", Look::Button),
        Act::OpenSettings => ("Settings", Look::Button),
        Act::OpenControls => ("Controls", Look::Button),
        Act::Title => ("Save and go to the title screen", Look::Button),
        Act::Quit => ("Save and quit", Look::Button),
        Act::Back => ("Back", Look::Button),
        Act::Master => ("Volume", Look::Slider(settings.master)),
        Act::Effects => ("Sound effects", Look::Slider(settings.effects)),
        Act::Music => ("Music", Look::Slider(settings.music)),
        Act::Shake => ("Screen shake", Look::Slider(settings.shake)),
        Act::Fullscreen => ("Fullscreen", Look::Toggle(settings.fullscreen)),
        Act::Colours => ("Colour-blind colours", Look::Toggle(settings.colourblind)),
        Act::UnitShape => ("Unit shape", Look::Choice(settings.unit_shape.name().to_owned())),
        Act::Hints => ("Hints", Look::Toggle(settings.hints)),
        Act::Difficulty => ("Enemy difficulty", Look::Choice(settings.difficulty.name().to_owned())),
        Act::Formation => ("Army formation", Look::Choice(settings.formation.name().to_owned())),
        Act::ArmyLimit => ("Army limit", Look::Amount(limit_level(settings.army_limit), grouped(settings.army_limit))),
        Act::ResetArmyLimit => ("Reset army limit to 15,000", Look::Button),
    }
}

//a line that says more about the row the player is on
pub(crate) fn explain_in(act: Act, settings: &Settings, mode: MenuMode) -> &'static str {
    match act {
        Act::Title | Act::Quit if mode == MenuMode::Hosting => "saved for the party to carry on from, and everyone in it is sent back to their title screen",
        Act::Save if mode == MenuMode::Hosting => "everyone's army is kept, and the players rejoin it by their names: F5 does the same",
        Act::Title | Act::Quit if mode == MenuMode::Guest => "the game goes on without you, and you can join it again",
        Act::Resume => "back to the game",
        Act::Save => "the one save: F5 does the same",
        Act::Load => "go back to the save: F9 does the same",
        Act::Title | Act::Quit => "the run is kept, and carried on from the title screen next time",
        Act::Shake => "how far the screen shakes and the camera jumps in for big moments",
        Act::Fullscreen => "F11 does the same. the game always starts full screen",
        Act::Colours => "blue and orange where the game uses green and red",
        Act::UnitShape => settings.unit_shape.blurb(),
        Act::Hints => "tips at the start of a run",
        Act::Difficulty => settings.difficulty.blurb(),
        Act::Formation => settings.formation.blurb(),
        Act::ArmyLimit => "the most units the army can have, 5,000 to 50,000. a bigger army is harder on the computer",
        Act::ResetArmyLimit => "back to the usual 15,000",
        _ => "",
    }
}

//change a setting by a step in a direction (-1 or 1): sliders move, toggles flip, choices go to the next. returns whether
//anything changed
pub(crate) fn nudge(settings: &mut Settings, act: Act, direction: i32) -> bool {
    let before = settings.clone();
    let slide = |level: &mut f32| *level = ((*level * 20.0).round() / 20.0 + 0.05 * direction as f32).clamp(0.0, 1.0);
    match act {
        Act::Master => slide(&mut settings.master),
        Act::Effects => slide(&mut settings.effects),
        Act::Music => slide(&mut settings.music),
        Act::Shake => slide(&mut settings.shake),
        Act::ArmyLimit => settings.army_limit = (settings.army_limit as i64 + ARMY_LIMIT_STEP as i64 * direction.signum() as i64).clamp(MIN_ARMY_LIMIT as i64, MAX_ARMY_LIMIT as i64) as usize,
        Act::Fullscreen => settings.fullscreen = !settings.fullscreen,
        Act::Colours => settings.colourblind = !settings.colourblind,
        Act::Hints => settings.hints = !settings.hints,
        Act::UnitShape => {
            let at = UNIT_SHAPES.iter().position(|&shape| shape == settings.unit_shape).unwrap_or(0) as i32;
            settings.unit_shape = UNIT_SHAPES[(at + direction.signum()).rem_euclid(UNIT_SHAPES.len() as i32) as usize];
        }
        Act::Difficulty => {
            let at = DIFFICULTIES.iter().position(|&difficulty| difficulty == settings.difficulty).unwrap_or(1) as i32;
            settings.difficulty = DIFFICULTIES[(at + direction.signum()).rem_euclid(DIFFICULTIES.len() as i32) as usize];
        }
        Act::Formation => {
            let at = FORMATIONS.iter().position(|&formation| formation == settings.formation).unwrap_or(0) as i32;
            settings.formation = FORMATIONS[(at + direction.signum()).rem_euclid(FORMATIONS.len() as i32) as usize];
        }
        _ => {}
    }
    *settings != before
}

//where on its slider (0 to 1) an army limit is
pub(crate) fn limit_level(limit: usize) -> f32 {
    (limit.saturating_sub(MIN_ARMY_LIMIT) as f32 / (MAX_ARMY_LIMIT - MIN_ARMY_LIMIT) as f32).clamp(0.0, 1.0)
}

//the army limit at a place on its slider (0 to 1), a whole number of steps
fn limit_at(level: f32) -> usize {
    let steps = ((MAX_ARMY_LIMIT - MIN_ARMY_LIMIT) / ARMY_LIMIT_STEP) as f32;
    MIN_ARMY_LIMIT + (level.clamp(0.0, 1.0) * steps).round() as usize * ARMY_LIMIT_STEP
}

//set a slider to a level (0 to 1)
pub(crate) fn set_level(settings: &mut Settings, act: Act, level: f32) -> bool {
    let before = settings.clone();
    if act == Act::ArmyLimit {
        settings.army_limit = limit_at(level);
        return *settings != before;
    }
    let level = ((level.clamp(0.0, 1.0) * 20.0).round() / 20.0).clamp(0.0, 1.0);
    match act {
        Act::Master => settings.master = level,
        Act::Effects => settings.effects = level,
        Act::Music => settings.music = level,
        Act::Shake => settings.shake = level,
        _ => {}
    }
    *settings != before
}

//where the menu is on a screen, as the panel and a rectangle for each of `count` rows
pub(crate) struct MenuLayout {
    pub(crate) panel: Rect,
    pub(crate) rows: Vec<Rect>,
    pub(crate) row_height: f32,
    pub(crate) tight: bool, //a page with so many rows for so small a screen that the title and the lines at the foot are squeezed to make room
}

//the room the panel keeps above its rows (for the title) and below them (for the line about the row and the keys), and how little there is on a screen
//with a page that would not otherwise fit. the rows are never less than MIN_ROW_HEIGHT, which is as small as a row is still easy to click
const HEADER: f32 = 78.0;
const FOOTER: f32 = 56.0;
const TIGHT_HEADER: f32 = 62.0;
const TIGHT_FOOTER: f32 = 44.0;
const MIN_ROW_HEIGHT: f32 = 28.0;

pub(crate) fn layout(page: Page, count: usize, screen: Vec2) -> MenuLayout {
    let width = (screen.x * 0.92).min(560.0);
    let extra = if page == Page::Controls { CONTROLS.len().div_ceil(2) as f32 * 28.0 + 20.0 } else { 0.0 };
    //rows are as tall as the screen has room for, down to 28 px: a page with a lot of them on a small screen has them squeezed
    let tight = (screen.y - 16.0 - HEADER - extra - FOOTER) / (count as f32) < MIN_ROW_HEIGHT;
    let (header, footer) = if tight { (TIGHT_HEADER, TIGHT_FOOTER) } else { (HEADER, FOOTER) };
    let room = (screen.y - 16.0 - header - extra - footer) / count as f32;
    let row_height = (screen.y * 0.056).clamp(32.0, 46.0).min(room).max(MIN_ROW_HEIGHT);
    let height = header + extra + row_height * count as f32 + footer;
    let panel = Rect::new((screen.x - width) / 2.0, ((screen.y - height) / 2.0).max(8.0), width, height);
    let first = panel.y + header - 4.0 + extra;
    let rows = (0..count).map(|i| Rect::new(panel.x + 16.0, first + row_height * i as f32, width - 32.0, row_height - 4.0)).collect();
    MenuLayout { panel, rows, row_height, tight }
}

//where the bar of a slider is in its row
pub(crate) fn slider_bar(row: Rect) -> Rect {
    Rect::new(row.x + row.w * 0.5, row.y + row.h / 2.0 - 4.0, row.w * 0.5 - 90.0, 8.0)
}

//the keys the game uses, for the controls page
pub(crate) const CONTROLS: [(&str, &str); 18] = [
    ("W A S D", "move"),
    ("mouse wheel", "zoom"),
    ("click an enemy", "send a squad"),
    ("R", "recall the squads"),
    ("F", "change the formation"),
    ("click, drag", "build walls"),
    ("right click", "delete a wall"),
    ("1 - 5", "what to build"),
    ("G", "man a tower"),
    ("Tab", "minimap: near or world"),
    ("M", "mute"),
    ("N", "music on or off"),
    ("F5", "save"),
    ("F9", "load"),
    ("F11", "fullscreen"),
    ("Space", "skip a boss's entrance"),
    ("1 2 3", "choose a relic"),
    ("Esc", "this menu"),
];

//what the player did this frame that the menu cares about
#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct MenuInput {
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
    pub(crate) right: bool,
    pub(crate) enter: bool,
    pub(crate) back: bool,
    pub(crate) click: bool, //the left button went down
    pub(crate) held: bool,  //and is down
    pub(crate) mouse: Vec2,
    pub(crate) moved: bool, //the mouse moved, which makes the row under it the one chosen
}

impl MenuInput {
    pub(crate) fn read(last_mouse: &mut Vec2) -> MenuInput {
        let mouse = Vec2::from(mouse_position());
        let moved = mouse.distance(*last_mouse) > 0.5;
        *last_mouse = mouse;
        MenuInput {
            up: is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W),
            down: is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S),
            left: is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A),
            right: is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D),
            enter: is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || is_key_pressed(KeyCode::Space),
            back: is_key_pressed(KeyCode::Escape),
            click: is_mouse_button_pressed(MouseButton::Left),
            held: is_mouse_button_down(MouseButton::Left),
            mouse,
            moved,
        }
    }
}

pub(crate) struct Pause {
    pub(crate) mode: MenuMode,
    confirm: Option<Act>, //the way out that has been asked for once, in a game of others, and is waiting to be asked for again
    pub(crate) page: Page,
    pub(crate) selected: usize,
    pub(crate) age: f32,   //seconds it has been open
    dragging: Option<Act>, //the slider the mouse has hold of
    pub(crate) last_mouse: Vec2,
    pub(crate) saved: Option<SaveInfo>, //what the save that Load goes back to says about itself, if there is one
    notice: Option<(String, Color, f32)>, //what the last thing done came to, in the colour it is shown in, and how many seconds are left to show it
}

impl Pause {
    pub(crate) fn new(saved: Option<SaveInfo>, mouse: Vec2) -> Pause {
        Pause { mode: MenuMode::Solo, confirm: None, page: Page::Main, selected: 0, age: 0.0, dragging: None, last_mouse: mouse, saved, notice: None }
    }

    //the rows of the page the menu is on
    pub(crate) fn rows(&self) -> &'static [Act] {
        page_rows_in(self.page, self.mode)
    }

    //say something in the menu itself, where the banner would be in the way
    pub(crate) fn tell(&mut self, text: &str, color: Color) {
        self.notice = Some((text.to_owned(), color, MENU_NOTICE_SECONDS));
    }

    //what the menu is saying about what was just done, if anything
    #[cfg(test)]
    pub(crate) fn notice(&self) -> Option<&str> {
        self.notice.as_ref().map(|(text, _, _)| text.as_str())
    }

    fn go_to(&mut self, page: Page) {
        self.page = page;
        self.selected = 0;
    }

    //one frame of the menu: move about it, change what is on a row, or leave
    pub(crate) fn update(&mut self, settings: &mut Settings, input: &MenuInput, screen: Vec2, dt: f32) -> PauseEvent {
        self.age += dt.min(0.1);
        if let Some(notice) = &mut self.notice {
            notice.2 -= dt;
        }
        if self.notice.as_ref().is_some_and(|notice| notice.2 <= 0.0) {
            self.notice = None;
        }
        let acts = self.rows();
        let frame = layout(self.page, acts.len(), screen);
        //the mouse chooses the row it is over, but never over a key pressed in the same frame: a mouse resting on one row that
        //jolts a little must not take the key's choice away
        let keyed = input.up || input.down || input.left || input.right || input.enter;
        if input.moved && !keyed {
            if let Some(i) = frame.rows.iter().position(|row| row.contains(input.mouse)) {
                self.selected = i;
            }
        }
        if input.up {
            self.selected = (self.selected + acts.len() - 1) % acts.len();
        }
        if input.down {
            self.selected = (self.selected + 1) % acts.len();
        }
        self.selected = self.selected.min(acts.len() - 1);
        //(a way out that was asked for is only asked for again on the same row)
        if self.confirm.is_some_and(|asked| asked != acts[self.selected]) {
            self.confirm = None;
        }
        if input.back {
            return match self.page {
                Page::Main => PauseEvent::Resume,
                _ => {
                    self.go_to(Page::Main);
                    PauseEvent::Nothing
                }
            };
        }
        //a slider being dragged follows the mouse until the button is let go
        if let Some(act) = self.dragging {
            if !input.held {
                self.dragging = None;
            } else if let Some(i) = acts.iter().position(|&other| other == act) {
                let bar = slider_bar(frame.rows[i]);
                return if set_level(settings, act, (input.mouse.x - bar.x) / bar.w) { PauseEvent::Changed } else { PauseEvent::Nothing };
            }
        }
        let clicked = input.click.then(|| frame.rows.iter().position(|row| row.contains(input.mouse))).flatten();
        if let Some(i) = clicked {
            self.selected = i;
        }
        let act = acts[self.selected];
        let (_, face) = describe_in(act, settings, self.mode);
        //left and right change a row's value, enter or a click does what the row says
        let direction = input.right as i32 - input.left as i32;
        if direction != 0 {
            return if nudge(settings, act, direction) { PauseEvent::Changed } else { PauseEvent::Nothing };
        }
        if !(input.enter || clicked.is_some()) {
            return PauseEvent::Nothing;
        }
        if let (Look::Slider(_) | Look::Amount(..), Some(i)) = (&face, clicked) {
            self.dragging = Some(act);
            let bar = slider_bar(frame.rows[i]);
            return if set_level(settings, act, (input.mouse.x - bar.x) / bar.w) { PauseEvent::Changed } else { PauseEvent::Nothing };
        }
        //a game that others are in is not ended by a slip: the way out has to be asked for twice
        if matches!(act, Act::Title | Act::Quit) && self.mode == MenuMode::Hosting && self.confirm != Some(act) {
            self.confirm = Some(act);
            self.tell("press again to end the game for everyone", ALERT);
            return PauseEvent::Nothing;
        }
        match act {
            Act::Resume => PauseEvent::Resume,
            Act::Save => PauseEvent::Save,
            Act::Load if self.saved.is_some() => PauseEvent::Load,
            Act::Load => PauseEvent::Nothing,
            Act::OpenSettings => {
                self.go_to(Page::Settings);
                PauseEvent::Nothing
            }
            Act::OpenControls => {
                self.go_to(Page::Controls);
                PauseEvent::Nothing
            }
            Act::Title => PauseEvent::Title,
            Act::Quit => PauseEvent::Quit,
            Act::Back => {
                self.go_to(Page::Main);
                PauseEvent::Nothing
            }
            Act::ResetArmyLimit if settings.army_limit != DEFAULT_ARMY_LIMIT => {
                settings.army_limit = DEFAULT_ARMY_LIMIT;
                PauseEvent::Changed
            }
            Act::ResetArmyLimit => PauseEvent::Nothing,
            _ => {
                if nudge(settings, act, 1) { PauseEvent::Changed } else { PauseEvent::Nothing }
            }
        }
    }

    //the menu over a darkened game. `now` is the time in seconds since 1970, `hosting` what to say of the game being hosted
    pub(crate) fn draw(&self, settings: &Settings, now: f64, hosting: Option<&str>) {
        let saved = self.saved;
        let screen = Vec2::new(screen_width(), screen_height());
        draw_rectangle(0.0, 0.0, screen.x, screen.y, Color::new(0.0, 0.01, 0.03, 0.62 * (self.age / 0.12).min(1.0)));
        let acts = self.rows();
        let frame = layout(self.page, acts.len(), screen);
        let p = frame.panel;
        panel(p.x, p.y, p.w, p.h);
        //(a game that others can join says who is in it, above the menu when there is room for that)
        if let Some(hosting) = hosting.filter(|_| p.y >= 30.0) {
            draw_label_centered(hosting, p.x + p.w / 2.0, p.y - 12.0, fit_size(hosting, 17.0, p.w, Face::Bold), CYAN, Face::Bold);
        }
        //(the game goes on while a menu is open in a game of others, which is said where it can be seen)
        if self.mode != MenuMode::Solo {
            draw_label_centered("THE GAME GOES ON", p.x + p.w / 2.0, p.y + if frame.tight { 14.0 } else { 17.0 }, if frame.tight { 12.0 } else { 13.0 }, TROPHY, Face::Bold);
        }
        let title = match self.page {
            Page::Main if self.mode != MenuMode::Solo => "MENU",
            Page::Main => "PAUSED",
            Page::Settings => "SETTINGS",
            Page::Controls => "CONTROLS",
        };
        let (title_at, title_size, line_at) = if frame.tight { (36.0, 26.0, 44.0) } else { (50.0, 34.0, 60.0) };
        draw_label_centered(title, p.x + p.w / 2.0, p.y + title_at, title_size, INK, Face::Header);
        draw_line(p.x + p.w / 2.0 - 60.0, p.y + line_at, p.x + p.w / 2.0 + 60.0, p.y + line_at, 2.0, CYAN);
        if self.page == Page::Controls {
            let column = (p.w - 40.0) / 2.0;
            for (i, (key, what)) in CONTROLS.iter().enumerate() {
                let (cx, cy) = (p.x + 20.0 + column * (i % 2) as f32, p.y + 84.0 + 28.0 * (i / 2) as f32);
                let wide = draw_key(key, cx, cy + 14.0, 15.0);
                draw_label(what, cx + wide + 8.0, cy + 14.0, 16.0, MUTED, Face::Body);
            }
        }
        for (i, (&act, row)) in acts.iter().zip(&frame.rows).enumerate() {
            let chosen = i == self.selected;
            let (label, face) = describe_in(act, settings, self.mode);
            let label = if self.confirm == Some(act) { "Press again to end it" } else { label };
            let disabled = act == Act::Load && saved.is_none();
            draw_rectangle(row.x, row.y, row.w, row.h, if chosen { Color::new(CYAN.r, CYAN.g, CYAN.b, 0.16) } else { Color::new(1.0, 1.0, 1.0, 0.03) });
            if chosen {
                draw_rectangle(row.x, row.y, 4.0, row.h, CYAN);
                draw_rectangle_lines(row.x, row.y, row.w, row.h, 1.5, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.7));
            }
            let size = (frame.row_height * 0.46).round();
            let baseline = row.y + row.h / 2.0 + size * 0.35;
            let color = if disabled { Color::new(MUTED.r, MUTED.g, MUTED.b, 0.5) } else if chosen { INK } else { MUTED };
            draw_label(label, row.x + 18.0, baseline, size, color, Face::Bold);
            match face {
                Look::Button => {
                    if act == Act::Load {
                        let note = match saved {
                            Some(info) => format!("saved {} ago, army of {}", ago(now - info.saved_at), grouped(info.army)),
                            None => "no save yet".to_owned(),
                        };
                        draw_label_right(&note, row.x + row.w - 14.0, baseline, size * 0.78, MUTED, Face::Body);
                    }
                }
                Look::Slider(level) => draw_slider(*row, level, &format!("{:.0}%", level * 100.0), chosen, baseline, size),
                Look::Amount(level, text) => draw_slider(*row, level, &text, chosen, baseline, size),
                Look::Toggle(on) => {
                    draw_label_right(if on { "ON" } else { "OFF" }, row.x + row.w - 14.0, baseline, size, if on { CYAN } else { MUTED }, Face::Bold);
                }
                Look::Choice(value) => {
                    draw_label_right(&format!("<  {value}  >"), row.x + row.w - 14.0, baseline, size, CYAN, Face::Bold);
                }
            }
        }
        let (note, tint) = match &self.notice {
            Some((text, color, _)) => (text.as_str(), *color),
            None => (acts.get(self.selected).map_or("", |&act| explain_in(act, settings, self.mode)), MUTED),
        };
        let (note_up, keys_up) = if frame.tight { (28.0, 9.0) } else { (34.0, 12.0) };
        draw_label_centered(note, p.x + p.w / 2.0, p.y + p.h - note_up, 16.0, tint, Face::Body);
        draw_label_centered("up and down choose   left and right change   enter selects   esc goes back", p.x + p.w / 2.0, p.y + p.h - keys_up, 13.0, Color::new(MUTED.r, MUTED.g, MUTED.b, 0.6), Face::Body);
    }
}

//a slider at a level (0 to 1), with what it says it is at on the right
fn draw_slider(row: Rect, level: f32, text: &str, chosen: bool, baseline: f32, size: f32) {
    let bar = slider_bar(row);
    draw_rectangle(bar.x, bar.y, bar.w, bar.h, Color::new(0.0, 0.0, 0.0, 0.55));
    draw_rectangle(bar.x, bar.y, bar.w * level, bar.h, CYAN);
    draw_circle(bar.x + bar.w * level, bar.y + bar.h / 2.0, 8.0, if chosen { WHITE } else { INK });
    draw_label_right(text, row.x + row.w - 12.0, baseline, size * 0.9, INK, Face::Bold);
}

//a time that has gone by, in the biggest unit that fits
pub(crate) fn ago(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    if seconds < 90.0 {
        format!("{:.0}s", seconds)
    } else if seconds < 5400.0 {
        format!("{:.0} min", seconds / 60.0)
    } else if seconds < 129_600.0 {
        format!("{:.0} h", seconds / 3600.0)
    } else {
        format!("{:.0} days", seconds / 86400.0)
    }
}

//a few tips in the first minute of a new run, which fade away. `age` is how many seconds the run has gone on
pub(crate) fn draw_hints(age: f32) {
    let shown = HINT_SECONDS;
    if age >= shown {
        return;
    }
    let fade = ((shown - age) / 2.0).clamp(0.0, 1.0) * (age / 0.8).clamp(0.0, 1.0);
    let lines = [
        ("WASD", "move and your army follows"),
        ("click an enemy", "to send a squad at it"),
        ("F", "changes the shape the army stands in"),
        ("Esc", "pauses, saves and has the settings"),
    ];
    let size = 17.0;
    //the explanations line up in a column after the widest key
    let keys = lines.iter().map(|(key, _)| text_width(key, size, Face::Bold)).fold(0.0, f32::max);
    let width = keys + 12.0 + lines.iter().map(|(_, what)| text_width(what, size, Face::Body)).fold(0.0, f32::max) + 28.0;
    let height = 20.0 + 26.0 * lines.len() as f32;
    let (x, y) = ((screen_width() - width) / 2.0, screen_height() - height - 86.0 - BUILD_BAR_HEIGHT);
    draw_rectangle(x, y, width, height, Color::new(PANEL.r, PANEL.g, PANEL.b, PANEL.a * fade));
    draw_rectangle_lines(x, y, width, height, 1.5, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.5 * fade));
    for (i, (key, what)) in lines.iter().enumerate() {
        let baseline = y + 28.0 + 26.0 * i as f32;
        draw_label(key, x + 14.0, baseline, size, Color::new(CYAN.r, CYAN.g, CYAN.b, fade), Face::Bold);
        draw_label(what, x + 14.0 + keys + 12.0, baseline, size, Color::new(INK.r, INK.g, INK.b, fade), Face::Body);
    }
}
