//! what the player at this machine sees and hears of the world: the events its step reports are taken in here and become numbers that float up,
//! particles, rings and beams, marks on the ground, chips of enemies, a shaking camera, a kill streak and the sounds to play. none of it is the
//! world's to know, so a machine that only watches the world has all of it

use crate::prelude::*;

pub(crate) struct Feedback {
    pub(crate) fx: Effects,           //numbers, rings and beams while they last, and the camera's shake and jump, and the sounds asked for
    pub(crate) particles: Vec<Particle>,
    pub(crate) ground: Vec<Mark>,     //scorch marks and what is left of enemies, which fade
    pub(crate) shards: Vec<Shard>,    //pieces of the enemies that have just been destroyed
    pub(crate) combo: Combo,
}

impl Feedback {
    pub(crate) fn new() -> Feedback {
        Feedback { fx: Effects::default(), particles: Vec::new(), ground: Vec::new(), shards: Vec::new(), combo: Combo::default() }
    }

    //take in what happened in the world. `me` is the player at this machine and `at` is where they are: the babies they have taken show over
    //them, and so does a kill streak. what the world reports is not believed beyond what is possible, the camera cannot be shaken more than all the way
    pub(crate) fn absorb(&mut self, events: Vec<Event>, me: usize, at: Vec2) {
        let mut destroyed = 0;
        for event in events {
            match event {
                Event::Popup { at, lane, text, color } => self.fx.popup(at, 0.0, lane, text, color),
                Event::Burst { at, color, count, speed } => self.fx.burst(at, color, count as usize, speed),
                Event::Ring { at, radius, color } => self.fx.ring(at, radius, color),
                Event::Beam { from, to, color } => self.fx.beams.push(Beam { from, to, color, age: 0.0 }),
                Event::Mark { at, radius, color, kind } => self.fx.mark(at, radius, color, kind),
                Event::Debris { at, color, size } => self.fx.debris(at, color, size),
                Event::Cue { cue, at: Some(spot) } => self.fx.cue_at(cue, spot),
                Event::Cue { cue, at: None } => self.fx.cue(cue),
                Event::Announce { text, color } => self.fx.announce(text, color),
                Event::Shake(amount) => self.fx.add_shake(amount.clamp(0.0, 1.0)),
                Event::Punch(amount) => self.fx.add_punch(amount.clamp(0.0, 1.0)),
                Event::PickedUp { player, count } => {
                    if player as usize == me {
                        self.fx.picked_up += count as usize;
                    }
                }
                Event::Destroyed(count) => destroyed += count as usize,
            }
        }
        if let Some((size, name)) = self.combo.add(destroyed) {
            self.fx.cue(Cue::Combo);
            self.fx.add_punch(0.25);
            self.fx.popup(at, RADIUS * 4.0, 30.0, format!("{name}!  x{size}"), TROPHY);
        }
    }

    //a frame later: the numbers, rings and beams age, the shaking settles, the babies that were taken show as one number over the player, puffs
    //become particles, marks and chips are laid and thrown and the kill streak runs down
    pub(crate) fn update(&mut self, dt: f32, at: Vec2) {
        self.fx.update(dt, at);
        self.fx.spawn_bursts(&mut self.particles);
        update_marks(&mut self.ground, &mut self.fx.marks, dt);
        throw_shards(&mut self.shards, &mut self.fx.shards);
        update_shards(&mut self.shards, dt);
        self.combo.update(dt);
    }
}
