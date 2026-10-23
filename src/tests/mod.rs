//the tests, only built by `cargo test`. each file is a module that starts with `use super::*`, and the helpers
//a file defines (marked pub(super)) are re-exported here so the other files can use them

use crate::prelude::*;

mod ambush;
mod arena;
mod army;
mod boost;
mod boss_fights;
mod champions;
mod chests;
mod bosses;
mod circles;
mod crowds;
mod danger;
mod directory;
mod discs;
mod effects;
mod enemies;
mod formations;
mod fortify;
mod frame;
mod horde_fights;
mod handshake;
mod hardening;
mod hordes;
mod lobby;
mod minimap;
mod online;
mod morale;
mod relics;
mod replication;
mod runner;
mod movement;
mod netplay;
mod party;
mod pause;
mod pincer;
mod players;
mod coop;
mod save;
mod score;
mod sim;
mod scenes;
mod parallel;
mod polish;
mod pressure;
mod siege;
mod sites;
mod settings;
mod shooters;
mod shortcuts;
mod sound;
mod squads;
mod team;
mod trails;
mod variety;
mod wilds;

use army::*;
use danger::*;
use enemies::*;
use fortify::*;
use movement::*;
use players::*;
use squads::*;
use variety::*;

//the tests that need the starting house were written when the world was 32000 px across, with the house round (16000, 16000)
pub(super) const SHIFT: f32 = WORLD_SIZE / 2.0 - 16000.0;

//enemies walking over open ground, with no walls to keep them out
pub(super) fn patrol_free(enemies: &mut [Enemy], player: Vec2, dt: f32, speed_multiplier: f32, fx: &mut Effects) {
    patrol_enemies(enemies, player, dt, speed_multiplier, &Walls::default(), &FlowField::new(), fx);
}
