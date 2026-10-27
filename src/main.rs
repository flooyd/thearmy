//! The Army: a top-down game of a growing army, one file for each part of it.
//!
//! - `config`: every tuning constant, `prelude`: what every module starts with
//! - `world`, `walls`, `pathing`: the grid, the walls and the routes round them
//! - `units`, `squads`: the army and sending parts of it at enemies
//! - `enemy`, `enemy_ai`, `spawning`: the enemies; `boss`, `relics`: bosses and their rewards
//! - `combat`: damage, shots and hazards; `effects`, `sound`: the feedback; `discs`: thousands of circles drawn in one go
//! - `creatures`: the enemies, bosses and their marks drawn as the creatures they are, with `creature_art` (generated from the drawings in art/ by art/shader.py)
//! - `champions`: special units won as loot, each with a power of its own
//! - `parallel`: sharing the army's work out over the processor's cores
//! - `minimap`, `hud`, `screens`, `ui`: what is drawn on top of the world, and the fonts and panels it is drawn with
//! - `sites`: nests and villages in the world, `defenses`, `building`: what the player raises against the enemies
//! - `save`: a run kept in a file and read back; `game`: one run as the player at this keyboard has it (the menu, the saves, the camera, the drawing)
//! - `sim`: the world and its step, which takes the players' `input` and gives back `events` (and nothing else leaves it), so it can be stepped without a window;
//!   `feedback`: what is shown and heard of those events; `wire`: the bytes inputs and events are written in
//! - `tests`: only built by `cargo test`

//a release build on Windows is a window program, with no console beside it (that also means its `eprintln!` messages go nowhere: a
//debug build, `cargo run`, keeps the console, and so does `cargo test`)
#![cfg_attr(all(not(debug_assertions), not(test)), windows_subsystem = "windows")]

mod prelude;
mod armies;
mod boss;
mod bolts;
mod building;
mod champions;
mod chests;
mod creature_art;
mod creatures;
mod discs;
mod combat;
mod config;
mod defenses;
mod directory;
mod effects;
mod enemy;
mod enemy_ai;
mod events;
mod feedback;
mod formation;
mod game;
mod ground;
mod handshake;
mod hordes;
mod homescreens;mod hud;
mod icon;
mod input;
mod lobby;
mod minimap;
mod music;
mod net;
mod online;
mod parallel;
mod pause;
mod pathing;
mod players;
mod prey;
mod replica;
mod runner;
mod polish;
mod relics;
mod save;
mod score;
mod settings;
mod screens;
mod showcase;
mod sim;
mod sites;
mod sound;
mod deaths;
mod status;
mod telegraphs;
mod spawning;
mod squads;
mod sync;
mod team;
mod textfield;
mod titlescene;
mod ui;
mod units;
mod vanguard;
mod viewer;
mod walls;
mod wire;
mod world;

use prelude::*;

//the window starts filling the screen, whatever size the screen is and however the last game was left (F11 and the menu take it out of fullscreen for the game
//being played, and no further), unless `THE_ARMY_WINDOWED` is set (which a game being tried out sets, so it doesn't take the screen over)
fn starts_fullscreen() -> bool {
    std::env::var_os("THE_ARMY_WINDOWED").is_none()
}

fn window_conf() -> Conf {
    Conf {
        window_title: "The Army".to_owned(),
        window_width: 1280,
        window_height: 720,
        fullscreen: starts_fullscreen(),
        icon: Some(window_icon()),
        ..Default::default()
    }
}

#[cfg(windows)]
fn center_window() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    let (screen_w, screen_h) =
        unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    let x = ((screen_w as f32 - screen_width()) / 2.0).max(0.0) as u32;
    let y = ((screen_h as f32 - screen_height()) / 2.0).max(0.0) as u32;
    miniquad::window::set_window_position(x, y);
}

#[macroquad::main(window_conf)]
async fn main() {
    let saves = Saves::in_home();
    let mut settings = saves.load_settings();
    settings.fullscreen = starts_fullscreen();
    //a window that fills the screen is already where it should be
    #[cfg(windows)]
    if !settings.fullscreen {
        center_window();
    }
    //closing the window is the game's to deal with, so that it can save first
    prevent_quit();
    load_fonts();
    //the Insane ladder is open as far as its file says, or as far as the best runs show it was opened, and the difficulty is one that is open
    open_ladder(saves.load_ladder().max(saves.load_high_scores().ladder()));
    settings.difficulty = settings.difficulty.within(ladder_open());
    settings.apply();
    //the programs the graphics card draws the creatures with are made now, behind an empty window, for a moment, and the rest a little in each frame of the title
    //screen (and any still left when a run starts), not in the middle of a game
    clear_background(BLACK);
    next_frame().await;
    warm_up_some(Duration::from_millis(WARM_UP_AT_START));
    //a game being hosted (so that others can join it) is kept for as long as runs follow one another, and closed when the title screen is back
    //(a showcase, for screenshots, goes straight to a run: see showcase.rs)
    //(the sounds and the music are made before the title screen, which has the music too)
    let mut sounds = Sounds::load(1.0).await;
    settings.tune(&mut sounds);
    let (mut link, mut start) = if showcase_asked() { (Link::Solo, Some(showcase_run())) } else { show_splash(&saves, &mut settings, &mut sounds, None).await.into_parts() };
    loop {
        match play(&mut sounds, &saves, &mut settings, start.take(), &mut link).await {
            Ending::Fallen(run, difficulty) => {
                //a run that is over can't be gone back to, only the quick save the player made themselves
                saves.delete(Slot::Auto);
                //the run goes on the list of the best, if it is good enough
                let mut scores = saves.load_high_scores();
                let record = Record::of(&run, difficulty);
                let place = scores.add(record.clone());
                if place.is_some() {
                    saves.save_high_scores(&scores);
                }
                show_defeat(&run, &record, place, &scores, &mut sounds).await;
            }
            Ending::Reload(snapshot) => {
                start = Some(snapshot);
                //the key that asked for it is still down for the rest of this frame, and would ask again
                next_frame().await;
            }
            Ending::Title => {
                //the run was kept as the game's own save, which the title screen offers to carry on from
                next_frame().await;
                drop(std::mem::replace(&mut link, Link::Solo));
                (link, start) = show_splash(&saves, &mut settings, &mut sounds, None).await.into_parts();
            }
            Ending::Left(why) => {
                //a stay in the game of another is over
                next_frame().await;
                drop(std::mem::replace(&mut link, Link::Solo));
                (link, start) = show_splash(&saves, &mut settings, &mut sounds, Some(why.to_uppercase())).await.into_parts();
            }
            Ending::Lost(reconnect, why) => {
                //the connection went: the player is put back in the game, as the same player, if the host has it still
                next_frame().await;
                drop(std::mem::replace(&mut link, Link::Solo));
                match reconnect_to(&reconnect, &why).await {
                    Ok((client, lobby)) => link = Link::Guest(client, lobby),
                    Err(why) => (link, start) = show_splash(&saves, &mut settings, &mut sounds, Some(why.to_uppercase())).await.into_parts(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
