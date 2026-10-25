//! the splash screen, the defeat screen, a boss's entrance and the victory screen (the picture on the splash screen is titlescene.rs)

use crate::prelude::*;

//behind the defeat screen and the others: a very dark blue, with a faint grid of the accent colour that drifts, so a screen with nothing moving on it still isn't flat
pub(crate) fn draw_backdrop(time: f32) {
    draw_backdrop_in(time, &HOMESCREENS[0].theme);
}

//the same in the colours of a look of the title screen: from the colour at the top to the one at the bottom, in bands
pub(crate) fn draw_backdrop_in(time: f32, theme: &Theme) {
    let (w, h) = (screen_width(), screen_height());
    if theme.top == theme.bottom {
        clear_background(theme.top);
    } else {
        const BANDS: usize = 48;
        for k in 0..BANDS {
            let (from, to) = (k as f32 / BANDS as f32, (k + 1) as f32 / BANDS as f32);
            draw_rectangle(0.0, from * h, w, (to - from) * h + 1.0, lerp_color(theme.top, theme.bottom, (from + to) / 2.0));
        }
    }
    let spacing = 56.0;
    let drift = (time * 6.0) % spacing;
    let line = theme.grid;
    let mut x = -drift;
    while x < screen_width() {
        draw_line(x, 0.0, x, screen_height(), 1.0, line);
        x += spacing;
    }
    let mut y = -drift;
    while y < screen_height() {
        draw_line(0.0, y, screen_width(), y, 1.0, line);
        y += spacing;
    }
}

//a heading in the header font with a glow of the accent colour behind it, centred on x
pub(crate) fn draw_glowing_heading(text: &str, x: f32, y: f32, size: f32, color: Color, alpha: f32) {
    let theme = &HOMESCREENS[0].theme;
    draw_heading_in(text, x, y, size, color, theme.glow, theme.shadow, alpha);
}

//the same with the colours of its glow and its shadow given (the shadow is what a dark title has behind it so it can be read over anything, and a light one has a pale
//haze instead)
pub(crate) fn draw_heading_in(text: &str, x: f32, y: f32, size: f32, color: Color, glow: Color, shadow: Color, alpha: f32) {
    let width = text_width(text, size, Face::Header);
    let glow = Color::new(glow.r, glow.g, glow.b, glow.a * alpha);
    let reach = (size / 28.0).max(2.0);
    for (dx, dy) in [(-reach, 0.0), (reach, 0.0), (0.0, -reach), (0.0, reach)] {
        draw_label(text, x - width / 2.0 + dx, y + dy, size, glow, Face::Header);
    }
    let offset = (size / 16.0).max(1.0).round();
    draw_label(text, x - width / 2.0 + offset, y + offset, size, Color::new(shadow.r, shadow.g, shadow.b, shadow.a * alpha), Face::Header);
    draw_label(text, x - width / 2.0, y, size, Color::new(color.r, color.g, color.b, alpha), Face::Header);
}

//rows of a label on the left and its value on the right, in a panel centred on x. returns how tall the panel was
fn draw_stats(rows: &[(&str, String)], x: f32, top: f32, row_height: f32, alpha: f32) -> f32 {
    let width = (screen_width() * 0.9).min(540.0);
    let height = row_height * rows.len() as f32 + 24.0;
    let left = x - width / 2.0;
    panel(left, top, width, height);
    let size = (row_height * 0.5).round().max(14.0);
    for (i, (label, value)) in rows.iter().enumerate() {
        let y = top + 12.0 + row_height * (i as f32 + 0.68);
        draw_label(label, left + 18.0, y, size, Color::new(MUTED.r, MUTED.g, MUTED.b, alpha), Face::Body);
        draw_label_right(value, left + width - 18.0, y, size * 1.1, Color::new(INK.r, INK.g, INK.b, alpha), Face::Bold);
        if i + 1 < rows.len() {
            let rule = top + 12.0 + row_height * (i + 1) as f32;
            draw_line(left + 14.0, rule, left + width - 14.0, rule, 1.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.12 * alpha));
        }
    }
    height
}

//the title and, under it, the prompt, which fades in and out so it is clear the game is waiting. time is how many seconds
//the screen has been up. with a save to go on from the prompt says so, and what is in it. `problem` is shown if the last
//attempt to load one went wrong. the scene of the army, its enemies and a boss is drawn behind it, with the units as `shape` and the enemies as plain squares
//if `simple`. the colours, and where the words stand, are those of the look the scene is in
pub(crate) fn draw_splash(time: f32, saved: Option<&SaveInfo>, problem: Option<&str>, best: u64, mouse: Vec2, scene: &mut TitleScene, shape: UnitShape, simple: bool) {
    let (home, screen) = (scene.home(), Vec2::new(screen_width(), screen_height()));
    let (theme, words) = (&home.theme, &home.words);
    draw_backdrop_in(time, theme);
    scene.draw(time, mouse, shape, simple);
    let (center_x, width, h, ts) = (words.center(screen, design_scale(screen)), words.width(screen), screen.y, words.size);
    let title = "THE ARMY";
    let title_size = fit_size(title, (h * 0.2 * ts).round(), width, Face::Header);
    let rise = (time / 0.8).clamp(0.0, 1.0);
    let title_y = h * words.title_y;
    draw_heading_in(title, center_x, title_y, title_size, theme.heading, theme.glow, theme.shadow, rise);
    //the line under it draws itself out from the middle
    let half = text_width(title, title_size, Face::Header) * 0.5 * rise.powi(2);
    let line_y = title_y + title_size * 0.22;
    draw_line(center_x - half, line_y, center_x + half, line_y, 4.0, theme.accent);

    let tagline = "grow the army   free the villages   fell the Sovereign";
    let tagline_size = fit_size(tagline, (h * 0.034 * ts).round(), width, Face::Body);
    draw_label_centered(tagline, center_x, title_y + title_size * 0.75, tagline_size, theme.muted, Face::Body);
    if best > 0 {
        let line = format!("BEST RUN  {}", grouped(best as usize));
        draw_label_centered(&line, center_x, title_y + title_size * 0.75 + tagline_size * 1.9, (tagline_size * 0.95).round(), theme.trophy, Face::Bold);
    }

    let prompt_size = (h * 0.04 * ts).round();
    let fade = 0.6 + 0.4 * (time * 3.0).sin();
    let pulse = Color::new(theme.accent.r, theme.accent.g, theme.accent.b, fade);
    let first = h * words.prompts_y;
    let step = |share: f32| first + h * share * ts;
    match saved {
        None => {
            draw_label_centered("PRESS ANY KEY TO CONTINUE", center_x, step(0.04), prompt_size, pulse, Face::Bold);
            draw_label_centered("OR M FOR MULTIPLAYER", center_x, step(0.11), (prompt_size * 0.7).round(), theme.muted, Face::Bold);
        }
        Some(info) => {
            draw_label_centered("PRESS ENTER TO CONTINUE", center_x, first, prompt_size, pulse, Face::Bold);
            let detail = format!("army of {}   {} played   bosses slain {}", grouped(info.army), clock(info.seconds), info.bosses);
            let detail_size = fit_size(&detail, (h * 0.034 * ts).round(), width, Face::Body);
            draw_label_centered(&detail, center_x, step(0.06), detail_size, theme.ink, Face::Body);
            draw_label_centered("PRESS N FOR A NEW RUN     M FOR MULTIPLAYER", center_x, step(0.13), (prompt_size * 0.7).round(), theme.muted, Face::Bold);
        }
    }
    if let Some(problem) = problem {
        draw_label_centered(problem, screen.x / 2.0, h * 0.95, (h * 0.03).round(), theme.alert, Face::Body);
    }
    draw_menu_button(mouse, theme);
}

//the button on the title screen that opens the menu, in the bottom left corner
fn menu_button() -> Rect {
    menu_button_of(Vec2::new(screen_width(), screen_height()))
}

fn draw_menu_button(mouse: Vec2, theme: &Theme) {
    let rect = menu_button();
    let size = (screen_height() * 0.034).round().max(15.0);
    let hot = rect.contains(mouse);
    let (back, edge, text) = if theme.light {
        (Color::new(1.0, 1.0, 1.0, if hot { 0.7 } else { 0.4 }), if hot { theme.accent } else { theme.muted }, if hot { theme.ink } else { theme.muted })
    } else {
        (if hot { Color::new(theme.accent.r, theme.accent.g, theme.accent.b, 0.22) } else { Color::new(1.0, 1.0, 1.0, 0.04) }, if hot { theme.accent } else { Color::new(theme.muted.r, theme.muted.g, theme.muted.b, 0.6) }, if hot { theme.ink } else { theme.muted })
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, back);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.5, edge);
    let baseline = rect.y + rect.h / 2.0 + size * 0.35;
    draw_label("MENU", rect.x + 18.0, baseline, size, text, Face::Bold);
    draw_key("Esc", rect.right() - text_width("Esc", size * 0.8, Face::Bold) - 24.0, baseline, size * 0.8);
}

//the window being closed ends the game, wherever it is. every screen that waits calls this, as the game has taken over what
//closing the window does (so that it can save first)
pub(crate) fn quit_if_asked() {
    if is_quit_requested() {
        std::process::exit(0);
    }
}

//what the title screen leads to: a run on this machine alone, one that others can join, or one that is another's. each of the first two is the run to
//carry on with, None for a new one
pub(crate) enum Start {
    Solo(Option<Snapshot>),
    Hosting(NetHost, Option<Snapshot>),
    Joined(NetClient, Lobby),
}

impl Start {
    pub(crate) fn into_parts(self) -> (Link, Option<Snapshot>) {
        match self {
            Start::Solo(snapshot) => (Link::Solo, snapshot),
            Start::Hosting(host, snapshot) => (Link::Host(host), snapshot),
            Start::Joined(client, lobby) => (Link::Guest(client, lobby), None),
        }
    }
}

//show the title screen until a key is pressed. when there is a save to go on from, Enter picks it up and N starts afresh. M is the multiplayer screens. Escape (or
//the button) opens the menu, with the settings and the way out, and Tab goes to the next of the looks the screen has (which is not said anywhere on it)
pub(crate) async fn show_splash(saves: &Saves, settings: &mut Settings, notice: Option<String>) -> Start {
    let mut saved = saves.newest();
    let mut problem: Option<String> = notice;
    let best = saves.load_high_scores().best();
    let mut time = 0.0;
    let mut scene = TitleScene::new(random_homescreen());
    let mut menu: Option<Pause> = None;
    loop {
        quit_if_asked();
        let mouse = Vec2::from(mouse_position());
        draw_splash(time, saved.as_ref().map(|(_, info)| info), problem.as_deref(), best, mouse, &mut scene, settings.unit_shape, settings.simple_shapes);
        let key = get_last_key_pressed();
        //the menu is over the picture, and takes the keys
        if let Some(open) = menu.as_mut() {
            open.draw(settings, miniquad::date::now(), None);            let input = MenuInput::read(&mut open.last_mouse);
            let was_fullscreen = settings.fullscreen;
            match open.update(settings, &input, Vec2::new(screen_width(), screen_height()), get_frame_time()) {
                PauseEvent::Resume => menu = None,
                PauseEvent::Quit => {
                    saves.save_settings(settings);
                    std::process::exit(0);
                }
                PauseEvent::Changed => {
                    settings.apply();
                    if settings.fullscreen != was_fullscreen {
                        miniquad::window::set_fullscreen(settings.fullscreen);
                    }
                    saves.save_settings(settings);
                }
                _ => {}
            }
            scene.update(get_frame_time());
            time += get_frame_time();
            next_frame().await;
            continue;
        }
        if is_key_pressed(KeyCode::Escape) || (is_mouse_button_pressed(MouseButton::Left) && menu_button().contains(mouse)) {
            let mut opened = Pause::new(None, mouse);
            opened.mode = MenuMode::Title;
            menu = Some(opened);
            next_frame().await;
            continue;
        }
        //(Tab is the next look, and starts nothing)
        if key == Some(KeyCode::Tab) {
            scene = TitleScene::new(next_homescreen(scene.index()));
            time = 0.0;
        } else if let Some(key) = key {
            match (&saved, key) {
                (_, KeyCode::Escape) => {}
                (_, KeyCode::M) => {
                    problem = None;
                    match show_multiplayer(settings, saves).await {
                        Some(Multi::Host(host, slot)) => {
                            match slot.map(|slot| saves.read(slot)) {
                                None => return Start::Hosting(host, None),
                                Some(Ok(snapshot)) => return Start::Hosting(host, Some(snapshot)),
                                Some(Err(error)) => {
                                    eprintln!("could not load the save: {error}");
                                    problem = Some("THAT SAVE COULD NOT BE READ".to_owned());
                                    saved = None;
                                }
                            }
                        }
                        Some(Multi::Guest(client, lobby)) => return Start::Joined(client, lobby),
                        None => {}
                    }
                }
                (None, _) | (Some(_), KeyCode::N) => return Start::Solo(None),
                (Some((slot, _)), KeyCode::Enter | KeyCode::KpEnter | KeyCode::C | KeyCode::Space) => match saves.read(*slot) {
                    Ok(snapshot) => return Start::Solo(Some(snapshot)),
                    Err(error) => {
                        eprintln!("could not load the save: {error}");
                        problem = Some("THAT SAVE COULD NOT BE READ".to_owned());
                        saved = None;
                    }
                },
                _ => {}
            }
        }
        scene.update(get_frame_time());
        time += get_frame_time();
        next_frame().await;
    }
}

pub(crate) const FALL_SECONDS: f32 = 2.2; //how long the game carries on after the army is gone before it shows the defeat screen
pub(crate) const DEFEAT_LOCKOUT: f32 = 1.5; //seconds before a key press leaves the defeat screen, so one that was already being pressed doesn't

//the screen after the army is gone: how the run went and what it scored, the best runs beside it when the screen is wide enough, and a
//prompt that appears once a key press would count. time is how many seconds it has been up. `place` is where the run came among the
//best, if it made the list
pub(crate) fn draw_defeat(time: f32, run: &Run, record: &Record, place: Option<usize>, scores: &HighScores) {
    draw_backdrop(time);
    let (w, h) = (screen_width(), screen_height());
    let title = "THE ARMY HAS FALLEN";
    let title_size = fit_size(title, (h * 0.1).round(), w * 0.92, Face::Header);
    let fade = (time / 0.6).clamp(0.0, 1.0);
    draw_glowing_heading(title, w / 2.0, h * 0.2, title_size, ALERT, fade);
    let half = text_width(title, title_size, Face::Header) * 0.5 * fade;
    draw_line(w / 2.0 - half, h * 0.2 + title_size * 0.3, w / 2.0 + half, h * 0.2 + title_size * 0.3, 3.0, CYAN);
    //the score, big, with a note if it is a best
    let score_size = fit_size(&grouped(record.score as usize), (h * 0.085).round(), w * 0.6, Face::Header);
    let score_y = h * 0.2 + title_size * 0.3 + score_size * 1.35;
    draw_label_centered(&grouped(record.score as usize), w / 2.0, score_y, score_size, Color::new(TROPHY.r, TROPHY.g, TROPHY.b, fade), Face::Header);
    let note = match place {
        Some(0) => "NEW BEST RUN!".to_owned(),
        Some(place) => format!("{} BEST RUN", ordinal(place + 1)),
        None => format!("SCORE ON {}", record.difficulty.name().to_uppercase()),
    };
    let note_size = (h * 0.032).round();
    draw_label_centered(&note, w / 2.0, score_y + note_size * 1.5, note_size, Color::new(CYAN.r, CYAN.g, CYAN.b, fade), Face::Bold);
    let rows = [
        ("enemies destroyed", grouped(run.destroyed)),
        ("bosses slain", grouped(run.bosses)),
        ("nests destroyed", grouped(run.nests)),
        ("villages freed", grouped(run.villages)),
        ("largest army", grouped(run.peak_army)),
        ("units lost", grouped(run.lost)),
        ("survived", clock(run.seconds)),
    ];
    let top = score_y + note_size * 2.4;
    let row_height = ((h * 0.95 - top) / (rows.len() as f32 + 1.0)).min(46.0);
    let used = draw_stats(&rows, w / 2.0, top, row_height, fade);
    //the best runs, when there is room beside the stats for them
    let panel_width = (w * 0.9).min(540.0);
    if w >= panel_width + 2.0 * 360.0 && !scores.records.is_empty() {
        let left = w / 2.0 + panel_width / 2.0 + 24.0;
        let width = (w - left - 24.0).min(420.0);
        let line = (used / (HIGH_SCORES_KEPT as f32 + 1.0)).min(44.0);
        panel(left, top, width, line * (scores.records.len() as f32 + 1.0) + 12.0);
        draw_label("BEST RUNS", left + 16.0, top + line * 0.8, (line * 0.5).round().max(14.0), Color::new(CYAN.r, CYAN.g, CYAN.b, fade), Face::Bold);
        for (i, best) in scores.records.iter().enumerate() {
            let y = top + line * (i as f32 + 1.8);
            let mine = place == Some(i);
            let color = if mine { TROPHY } else { INK };
            draw_label(&format!("{}", i + 1), left + 16.0, y, (line * 0.5).round().max(14.0), Color::new(color.r, color.g, color.b, fade), Face::Bold);
            draw_label_right(&grouped(best.score as usize), left + width - 16.0, y, (line * 0.5).round().max(14.0), Color::new(color.r, color.g, color.b, fade), Face::Bold);
            draw_label(best.difficulty.name(), left + 44.0, y, (line * 0.4).round().max(12.0), Color::new(MUTED.r, MUTED.g, MUTED.b, fade), Face::Body);
        }
    }
    if time >= DEFEAT_LOCKOUT {
        let pulse = 0.6 + 0.4 * (time * 3.0).sin();
        draw_label_centered("PRESS ANY KEY TO TRY AGAIN", w / 2.0, h * 0.95, (h * 0.036).round(), Color::new(CYAN.r, CYAN.g, CYAN.b, pulse), Face::Bold);
    }
}

//1st, 2nd, 3rd, 4th...
pub(crate) fn ordinal(number: usize) -> String {
    let suffix = match (number % 10, number % 100) {
        (_, 11..=13) => "TH",
        (1, _) => "ST",
        (2, _) => "ND",
        (3, _) => "RD",
        _ => "TH",
    };
    format!("{number}{suffix}")
}

//show the defeat screen until a key is pressed
pub(crate) async fn show_defeat(run: &Run, record: &Record, place: Option<usize>, scores: &HighScores) {
    let mut time = 0.0;
    loop {
        quit_if_asked();
        draw_defeat(time, run, record, place, scores);
        if time >= DEFEAT_LOCKOUT && get_last_key_pressed().is_some() {
            return;
        }
        time += get_frame_time();
        next_frame().await;
    }
}

//a boss's entrance: the camera goes to it and back while the world crawls, and its name and what it is about to do come up
pub(crate) struct Intro {
    pub(crate) age: f32, //seconds, in real time
    pub(crate) at: Vec2, //where the boss is
    pub(crate) name: &'static str,
    pub(crate) tagline: &'static str,
    pub(crate) color: Color,
}

impl Intro {
    pub(crate) fn new(boss: &Enemy) -> Intro {
        let state = boss.boss.as_ref().expect("a boss");
        Intro { age: 0.0, at: boss.position, name: state.def.name, tagline: state.def.phases[0].name, color: boss.color }
    }

    pub(crate) fn over(&self) -> bool {
        self.age >= INTRO_SECONDS
    }

    //0 to 1 to 0: how far the camera has gone from the player to the boss
    pub(crate) fn ease(&self) -> f32 {
        let smooth = |from: f32, to: f32, t: f32| {
            let k = ((t - from) / (to - from)).clamp(0.0, 1.0);
            k * k * (3.0 - 2.0 * k)
        };
        let t = (self.age / INTRO_SECONDS).clamp(0.0, 1.0);
        smooth(0.0, 0.3, t) * (1.0 - smooth(0.78, 1.0, t))
    }

    //where the camera is and how far in it is during the entrance, for a player at `player` who is zoomed to `zoom`: it goes to the boss, and out a
    //little to see more
    pub(crate) fn view(&self, player: Vec2, zoom: f32) -> (Vec2, f32) {
        let ease = self.ease();
        (player.lerp(self.at, ease), zoom * (1.0 - INTRO_ZOOM_OUT * ease))
    }
}

//the name of the boss across the screen, with bars above and below like a film
pub(crate) fn draw_intro(intro: &Intro) {
    let (w, h) = (screen_width(), screen_height());
    let show = intro.ease().min(1.0);
    let bar = h * 0.1 * show;
    draw_rectangle(0.0, 0.0, w, bar, BLACK);
    draw_rectangle(0.0, h - bar, w, bar, BLACK);
    draw_line(0.0, bar, w, bar, 2.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.8 * show));
    draw_line(0.0, h - bar, w, h - bar, 2.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.8 * show));
    let alpha = ((intro.age - 0.5) / 0.4).clamp(0.0, 1.0) * show;
    let size = fit_size(intro.name, (h * 0.11).round(), w * 0.92, Face::Header);
    draw_label_shadowed(intro.name, (w - text_width(intro.name, size, Face::Header)) / 2.0, h * 0.82, size, Color::new(intro.color.r, intro.color.g, intro.color.b, alpha), Face::Header);
    let tag_size = (h * 0.04).round();
    let tag = intro.tagline.to_uppercase();
    let tag_y = h * 0.82 + tag_size * 1.4;
    draw_label_centered(&tag, w / 2.0, tag_y, tag_size, Color::new(CYAN.r, CYAN.g, CYAN.b, alpha), Face::Bold);
    //a short line either side of the tagline
    let reach = text_width(&tag, tag_size, Face::Bold) / 2.0;
    let rule = Color::new(CYAN.r, CYAN.g, CYAN.b, alpha * 0.7);
    draw_line(w / 2.0 - reach - 60.0, tag_y - tag_size * 0.3, w / 2.0 - reach - 16.0, tag_y - tag_size * 0.3, 2.0, rule);
    draw_line(w / 2.0 + reach + 16.0, tag_y - tag_size * 0.3, w / 2.0 + reach + 60.0, tag_y - tag_size * 0.3, 2.0, rule);
}

pub(crate) const VICTORY_LOCKOUT: f32 = 2.0; //seconds before a key press leaves the victory screen

//the screen after the citadel falls: how the run went so far, and that it goes on. `age` is how many seconds it has been up
pub(crate) fn draw_victory(age: f32, run: &Run, score: u64) {
    let (w, h) = (screen_width(), screen_height());
    let fade = (age / 0.8).clamp(0.0, 1.0);
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.01, 0.03, 0.82 * fade));
    let title_size = fit_size("VICTORY", (h * 0.17).round(), w * 0.9, Face::Header);
    draw_glowing_heading("VICTORY", w / 2.0, h * 0.25, title_size, TROPHY, fade);
    let sub = if run.citadels > 1 { format!("THE SOVEREIGN HAS FALLEN {} TIMES", run.citadels) } else { "THE SOVEREIGN HAS FALLEN".to_owned() };
    let sub_size = fit_size(&sub, (h * 0.045).round(), w * 0.9, Face::Bold);
    draw_label_centered(&sub, w / 2.0, h * 0.25 + title_size * 0.5, sub_size, Color::new(CYAN.r, CYAN.g, CYAN.b, fade), Face::Bold);
    let rows = [
        ("score so far", grouped(score as usize)),
        ("enemies destroyed", grouped(run.destroyed)),
        ("bosses slain", grouped(run.bosses)),
        ("nests destroyed", grouped(run.nests)),
        ("villages freed", grouped(run.villages)),
        ("largest army", grouped(run.peak_army)),
        ("units lost", grouped(run.lost)),
        ("time", clock(run.seconds)),
    ];
    draw_stats(&rows, w / 2.0, h * 0.25 + title_size * 0.5 + sub_size, (h * 0.052).min(42.0), fade);
    if age >= VICTORY_LOCKOUT {
        let pulse = 0.6 + 0.4 * (age * 3.0).sin();
        let line = format!("THE SOVEREIGN RETURNS IN {}, STRONGER - PRESS ANY KEY", clock(CITADEL_RESPAWN_SECONDS));
        let size = fit_size(&line, (h * 0.034).round(), w * 0.9, Face::Bold);
        draw_label_centered(&line, w / 2.0, h * 0.95, size, Color::new(CYAN.r, CYAN.g, CYAN.b, pulse), Face::Bold);
    }
}
