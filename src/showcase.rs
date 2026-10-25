//! a way to look at the game without playing it, to check how it looks in the real game (the champions' pictures, the bolts): with ARMY_SHOWCASE set, the title
//! screen is skipped and a run starts with every kind of champion in the army at the top level, and a crowd of enemies ahead of it; with ARMY_SHOT set to a file,
//! the screen is saved to it as a PNG once ARMY_SHOT_SECONDS (4 unless set) have gone by in the run, and the game closes. the game never sets either. set
//! THE_ARMY_HOME as well, to a folder of its own, so that nothing of the player's (their settings, their saves) is touched. with ARMY_SHOWCASE=statuses there are
//! no champions, and the crowd is near and starts with what can be done to an enemy (frozen, chilled, burning, shocked, poisoned), to see the marks and a
//! shatter's pieces. with ARMY_SHOWCASE=telegraphs there are no champions or enemies, and one of each shape of warning is put down round the player
//! (see showcase_hazards), with a big one off the screen for its arrow
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
    let statuses = showcase_is("statuses");
    let telegraphs = showcase_is("telegraphs");
    run.zoom = SHOWCASE_ZOOM;
    for i in 0..SHOWCASE_ARMY {
        let spot = at + Vec2::from_angle(i as f32 * 2.399_963) * (40.0 + (i as f32).sqrt() * 14.0);
        run.units.push(Unit::new(spot.x, spot.y, UNIT_MAX_SPEED));
    }
    for kind in (0..CHAMPIONS.len()).filter(|_| !statuses && !telegraphs) {
        let spot = at + Vec2::from_angle(kind as f32 * TAU / CHAMPIONS.len() as f32) * 260.0;
        let mut champion = champion_unit(kind, spot);
        champion.level = CHAMPION_MAX_LEVEL;
        champion.evolutions = 0b01;
        champion.health = champion.max_health();
        run.units.push(champion);
    }
    for i in (0..SHOWCASE_ENEMIES).filter(|_| !telegraphs) {
        let spot = at + Vec2::new((i % 10) as f32 * 90.0 - 405.0, if statuses { -330.0 } else { -900.0 } - (i / 10) as f32 * 90.0);
        let mut enemy = Enemy::new(spot, kind_named(if i % 3 == 0 { "Brute" } else { "Grunt" }), 2.0);
        if statuses {
            match i % 5 {
                0 => freeze(&mut enemy, 30.0),
                1 => chill(&mut enemy, CHILL_FULL * 0.9, 0.0),
                2 => ignite(&mut enemy, 30.0, 1.0),
                3 => shock(&mut enemy, 30.0),
                _ => poison(&mut enemy, POISON_MOST),
            }
        }
        run.enemies.push(enemy);
    }
    run
}

//whether the showcase asked for is this one
fn showcase_is(kind: &str) -> bool {
    std::env::var("ARMY_SHOWCASE").is_ok_and(|asked| asked == kind)
}

//the warnings a showcase of them puts down round the player, `at`: a slam, a gravity ring, a fan of beams, a cleave and a piston line on the screen, each
//a long way from going off, and a nova off it. none for any other showcase, or none asked for
pub(crate) fn showcase_hazards(at: Vec2) -> Vec<Hazard> {
    if !showcase_is("telegraphs") {
        return Vec::new();
    }
    let laid = |position: Vec2, radius: f32, circle: BossCircle| {
        let mut hazard = Hazard::new(position, radius, 30.0, UNIT_HEALTH * 3.0, 1000, circle.color(), Cue::Slam).circle(circle);
        hazard.warn_total = 40.0;
        hazard
    };
    let mut hazards = vec![
        laid(at + Vec2::new(-560.0, -300.0), 170.0, BossCircle::Slam),
        laid(at + Vec2::new(-470.0, 120.0), 330.0, BossCircle::GravityRing).shaped(Shape::Ring { inner: 130.0 }),
        laid(at + Vec2::new(0.0, 160.0), 330.0, BossCircle::Cleave).shaped(Shape::Cone { dir: Vec2::Y, arc: CLEAVE_ARC }),
        laid(at + Vec2::new(470.0, 280.0), 0.0, BossCircle::Piston).shaped(Shape::Rect { dir: Vec2::X, length: 420.0, width: 130.0 }),
        laid(at + Vec2::new(2200.0, 0.0), 600.0, BossCircle::Nova),
    ];
    for k in 0..3 {
        let dir = Vec2::from_angle(-2.4 + k as f32 * 0.55);
        hazards.push(laid(at + Vec2::new(620.0, -80.0), 0.0, BossCircle::Beam).shaped(Shape::Line { dir, length: 420.0, width: 46.0 }).grouped(77));
    }
    hazards
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
