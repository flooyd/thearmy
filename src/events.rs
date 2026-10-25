//! what a frame of the world tells whoever is watching it: numbers that float up, puffs, shakes, sounds, messages. each is plain data, so it
//! can be written to bytes and read back, and each has a place in the world (or none), so a watcher can be sent only the ones it can see

use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Event {
    Popup { at: Vec2, lane: f32, text: String, color: Color }, //a number or a word that floats up from a place
    Burst { at: Vec2, color: Color, count: u16, speed: f32 },  //a puff of particles
    Ring { at: Vec2, radius: f32, color: Color },              //a shockwave
    Beam { from: Vec2, to: Vec2, color: Color },               //a streak of light
    Mark { at: Vec2, radius: f32, color: Color, kind: MarkKind }, //a mark left on the ground
    Debris { at: Vec2, color: Color, size: f32 },              //chips thrown out from something that broke
    Cue { cue: Cue, at: Option<Vec2> },                        //a sound, which is quieter the further it is from the listener if it happened somewhere
    Announce { text: String, color: Color },                   //a message for the banner
    Shake(f32),                                                //the camera shakes
    Punch(f32),                                                //the camera jumps in
    PickedUp { player: u8, count: u32 },                       //babies grew into units of a player's army
    Volley(Volley), //a champion's volley, which every machine makes the same bolts from (see `launch_volley`)
    Destroyed(u32),                                            //enemies were destroyed, which is what a kill streak is made of
}

//the tags the kinds of event are written with
const POPUP: u8 = 1;
const BURST: u8 = 2;
const RING: u8 = 3;
const BEAM: u8 = 4;
const MARK: u8 = 5;
const DEBRIS: u8 = 6;
const CUE: u8 = 7;
const ANNOUNCE: u8 = 8;
const SHAKE: u8 = 9;
const PUNCH: u8 = 10;
const PICKED_UP: u8 = 11;
const DESTROYED: u8 = 12;
const VOLLEY: u8 = 13;

//the most events one message holds, so a message that says it holds more is not believed
pub(crate) const MAX_EVENTS_A_MESSAGE: usize = 1024;

impl Event {
    //where it happened, for the ones that happened somewhere: a watcher far from it has no use for it
    pub(crate) fn place(&self) -> Option<Vec2> {
        match self {
            Event::Popup { at, .. } | Event::Burst { at, .. } | Event::Ring { at, .. } | Event::Mark { at, .. } | Event::Debris { at, .. } => Some(*at),
            Event::Beam { from, .. } => Some(*from),
            Event::Volley(volley) => Some(volley.from),
            Event::Cue { at, .. } => *at,
            Event::Announce { .. } | Event::Shake(_) | Event::Punch(_) | Event::PickedUp { .. } | Event::Destroyed(_) => None,
        }
    }

    pub(crate) fn write(&self, out: &mut Writer) {
        match self {
            Event::Popup { at, lane, text, color } => {
                out.u8(POPUP);
                out.vec2(*at);
                out.f32(*lane);
                out.text(text);
                out.color(*color);
            }
            Event::Burst { at, color, count, speed } => {
                out.u8(BURST);
                out.vec2(*at);
                out.color(*color);
                out.u16(*count);
                out.f32(*speed);
            }
            Event::Ring { at, radius, color } => {
                out.u8(RING);
                out.vec2(*at);
                out.f32(*radius);
                out.color(*color);
            }
            Event::Beam { from, to, color } => {
                out.u8(BEAM);
                out.vec2(*from);
                out.vec2(*to);
                out.color(*color);
            }
            Event::Mark { at, radius, color, kind } => {
                out.u8(MARK);
                out.vec2(*at);
                out.f32(*radius);
                out.color(*color);
                out.u8(match kind {
                    MarkKind::Scorch => 0,
                    MarkKind::Splat => 1,
                });
            }
            Event::Debris { at, color, size } => {
                out.u8(DEBRIS);
                out.vec2(*at);
                out.color(*color);
                out.f32(*size);
            }
            Event::Cue { cue, at } => {
                out.u8(CUE);
                out.u8(ALL_CUES.iter().position(|other| other == cue).unwrap_or(0) as u8);
                out.flag(at.is_some());
                if let Some(at) = at {
                    out.vec2(*at);
                }
            }
            Event::Announce { text, color } => {
                out.u8(ANNOUNCE);
                out.text(text);
                out.color(*color);
            }
            Event::Shake(amount) => {
                out.u8(SHAKE);
                out.f32(*amount);
            }
            Event::Punch(amount) => {
                out.u8(PUNCH);
                out.f32(*amount);
            }
            Event::PickedUp { player, count } => {
                out.u8(PICKED_UP);
                out.u8(*player);
                out.u32(*count);
            }
            Event::Destroyed(count) => {
                out.u8(DESTROYED);
                out.u32(*count);
            }
            Event::Volley(volley) => {
                out.u8(VOLLEY);
                out.vec2(volley.from);
                out.vec2(volley.aim);
                out.u8(volley.kind as u8);
                out.u8(volley.shape as u8);
                out.u8(volley.count);
                out.f32(volley.scatter);
                out.f32(volley.speed);
                out.f32(volley.reach);
                out.u8(volley.pierce);
                out.u8(volley.bounce);
                out.f32(volley.homing);
                out.f32(volley.splash);
                out.u8(volley.embers);
                out.u8(volley.champion);
                out.u32(volley.seed);
            }
        }
    }

    //the next event in the bytes, or None if they have run out or it isn't one
    pub(crate) fn read(input: &mut Reader) -> Option<Event> {
        Some(match input.u8()? {
            POPUP => Event::Popup { at: input.vec2()?, lane: input.f32()?, text: input.text()?, color: input.color()? },
            BURST => Event::Burst { at: input.vec2()?, color: input.color()?, count: input.u16()?, speed: input.f32()? },
            RING => Event::Ring { at: input.vec2()?, radius: input.f32()?, color: input.color()? },
            BEAM => Event::Beam { from: input.vec2()?, to: input.vec2()?, color: input.color()? },
            MARK => Event::Mark {
                at: input.vec2()?,
                radius: input.f32()?,
                color: input.color()?,
                kind: match input.u8()? {
                    0 => MarkKind::Scorch,
                    1 => MarkKind::Splat,
                    _ => return None,
                },
            },
            DEBRIS => Event::Debris { at: input.vec2()?, color: input.color()?, size: input.f32()? },
            CUE => {
                let cue = *ALL_CUES.get(input.u8()? as usize)?;
                let at = if input.flag()? { Some(input.vec2()?) } else { None };
                Event::Cue { cue, at }
            }
            ANNOUNCE => Event::Announce { text: input.text()?, color: input.color()? },
            SHAKE => Event::Shake(input.f32()?),
            PUNCH => Event::Punch(input.f32()?),
            PICKED_UP => Event::PickedUp { player: input.u8().filter(|&player| (player as usize) < MAX_PLAYERS)?, count: input.u32()? },
            DESTROYED => Event::Destroyed(input.u32()?),
            VOLLEY => {
                let volley = Volley {
                    from: input.vec2()?,
                    aim: input.vec2()?,
                    kind: *BOLT_KINDS.get(input.u8()? as usize)?,
                    shape: *VOLLEY_SHAPES.get(input.u8()? as usize)?,
                    count: input.u8()?,
                    scatter: input.f32()?,
                    speed: input.f32()?,
                    reach: input.f32()?,
                    pierce: input.u8()?,
                    bounce: input.u8()?,
                    homing: input.f32()?,
                    splash: input.f32()?,
                    embers: input.u8()?,
                    champion: input.u8()?,
                    seed: input.u32()?,
                };
                if !volley.is_believable() {
                    return None;
                }
                Event::Volley(volley)
            }
            _ => return None,
        })
    }
}

//a message that holds some events: how many, then each
pub(crate) fn write_events(events: &[Event]) -> Vec<u8> {
    let mut out = Writer::new();
    let events = &events[..events.len().min(MAX_EVENTS_A_MESSAGE)];
    out.u16(events.len() as u16);
    for event in events {
        event.write(&mut out);
    }
    out.bytes
}

//the events in a message, or None if it is cut short, garbled, or has anything left over after them
pub(crate) fn read_events(bytes: &[u8]) -> Option<Vec<Event>> {
    let mut input = Reader::new(bytes);
    let count = input.u16()? as usize;
    if count > MAX_EVENTS_A_MESSAGE {
        return None;
    }
    let mut events = Vec::with_capacity(count);
    for _ in 0..count {
        events.push(Event::read(&mut input)?);
    }
    input.is_done().then_some(events)
}

impl Effects {
    //what this frame made that is for the eyes and ears, as events, and none of it left here: the numbers, puffs, rings, beams, marks, debris,
    //sounds and messages, how much the camera is to shake or jump, and how many babies each player's army took. what the world itself still
    //has to do with (the hazards that were started, the walls that were hit, who earned what) is left for it
    pub(crate) fn drain_events(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        for popup in self.popups.drain(..) {
            events.push(Event::Popup { at: popup.position, lane: popup.lane, text: popup.text, color: popup.color });
        }
        for burst in self.bursts.drain(..) {
            events.push(Event::Burst { at: burst.position, color: burst.color, count: burst.count.min(u16::MAX as usize) as u16, speed: burst.speed });
        }
        for ring in self.rings.drain(..) {
            events.push(Event::Ring { at: ring.position, radius: ring.radius, color: ring.color });
        }
        for beam in self.beams.drain(..) {
            events.push(Event::Beam { from: beam.from, to: beam.to, color: beam.color });
        }
        for mark in self.marks.drain(..) {
            events.push(Event::Mark { at: mark.position, radius: mark.radius, color: mark.color, kind: mark.kind });
        }
        for shards in self.shards.drain(..) {
            events.push(Event::Debris { at: shards.position, color: shards.color, size: shards.size });
        }
        for (volley, _) in self.volleys.drain(..) {
            events.push(Event::Volley(volley));
        }
        for (cue, at) in self.take_cues() {
            events.push(Event::Cue { cue, at });
        }
        for (text, color) in self.announcements.drain(..) {
            events.push(Event::Announce { text, color });
        }
        //(the camera's shaking and jumping are only added to here, as it is a watcher that lets them settle)
        let shake = std::mem::take(&mut self.shake);
        if shake > 0.0 {
            events.push(Event::Shake(shake));
        }
        let punch = std::mem::take(&mut self.punch);
        if punch > 0.0 {
            events.push(Event::Punch(punch));
        }
        for (player, grown) in self.grown.iter_mut().enumerate() {
            if *grown > 0 {
                events.push(Event::PickedUp { player: player as u8, count: *grown as u32 });
                *grown = 0;
            }
        }
        events
    }
}
