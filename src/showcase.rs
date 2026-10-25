//! a way to look at the game without playing it, to check how it looks in the real game (the champions' pictures, the bolts): with ARMY_SHOWCASE set, the title
//! screen is skipped and a run starts with every kind of champion in the army at the top level, and a crowd of enemies ahead of it; with ARMY_SHOT set to a file,
//! the screen is saved to it as a PNG once ARMY_SHOT_SECONDS (4 unless set) have gone by in the run, and the game closes. the game never sets either. set
//! THE_ARMY_HOME as well, to a folder of its own, so that nothing of the player's (their settings, their saves) is touched
//!
//!     THE_ARMY_HOME=<folder> ARMY_SHOWCASE=1 ARMY_SHOT=<shot.png> mygame

use crate::prelude::*;
use std::f32::consts::TAU;

//whether a showcase was asked for
pub(crate) fn showcase_asked() -> bool {
    std::env::var_os("ARMY_SHOWCASE").is_some()
}

//a new run made into a showcase: SHOWCASE_ARMY more units round the player, one of every kind of champion at the top level (with its evolution chosen, so no
//card comes up), and SHOWCASE_ENEMIES grunts and brutes, twice the usual size, ahead (the army faces up when a run starts) to fire at, zoomed to SHOWCASE_ZOOM
pub(crate) fn showcase_run() -> Snapshot {
    let mut run = new_run();
    let at = run.player;
    run.zoom = SHOWCASE_ZOOM;
    for i in 0..SHOWCASE_ARMY {
        let spot = at + Vec2::from_angle(i as f32 * 2.399_963) * (40.0 + (i as f32).sqrt() * 14.0);
        run.units.push(Unit::new(spot.x, spot.y, UNIT_MAX_SPEED));
    }
    for kind in 0..CHAMPIONS.len() {
        let spot = at + Vec2::from_angle(kind as f32 * TAU / CHAMPIONS.len() as f32) * 260.0;
        let mut champion = champion_unit(kind, spot);
        champion.level = CHAMPION_MAX_LEVEL;
        champion.evolutions = 0b01;
        champion.health = champion.max_health();
        run.units.push(champion);
    }
    for i in 0..SHOWCASE_ENEMIES {
        let spot = at + Vec2::new((i % 10) as f32 * 90.0 - 405.0, -900.0 - (i / 10) as f32 * 90.0);
        run.enemies.push(Enemy::new(spot, kind_named(if i % 3 == 0 { "Brute" } else { "Grunt" }), 2.0));
    }
    run
}

//the screen saved, once a run has gone on for long enough, if a shot was asked for: true when it has been, and the game is to close
pub(crate) fn shot_taken(seconds: f32) -> bool {
    let Some(path) = std::env::var_os("ARMY_SHOT") else {
        return false;
    };
    let due = std::env::var("ARMY_SHOT_SECONDS").ok().and_then(|text| text.parse::<f32>().ok()).unwrap_or(4.0);
    if seconds < due {
        return false;
    }
    get_screen_data().export_png(&path.to_string_lossy());
    true
}
