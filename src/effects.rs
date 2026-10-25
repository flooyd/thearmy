//! feedback: particles, floating numbers, screen shake and the sound cues asked for in a frame

use crate::prelude::*;

//a speck that flies out behind a moving circle, shrinking and fading as it ages
pub(crate) struct Particle {
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2, //pixels per second
    pub(crate) age: f32,       //seconds
    pub(crate) life: f32,      //seconds it lasts
    pub(crate) color: Color,
    pub(crate) size: f32,      //radius when new
}

//leave particles behind a circle that is moving at a velocity, flying out the opposite way. a circle that is making
//little headway compared to its top speed (stuck behind a wall, say) leaves none
pub(crate) fn emit_trail(particles: &mut Vec<Particle>, position: Vec2, velocity: Vec2, top_speed: f32, dt: f32, color: Color) {
    let speed = velocity.length();
    if speed < top_speed * TRAIL_MIN_SPEED {
        return;
    }
    let behind = -velocity / speed;
    let sideways = behind.perp();
    //the fuller the pool of particles the fewer are made, so a crowd's trails all get thinner together instead of the
    //first few circles using up every particle
    let room = (1.0 - particles.len() as f32 / MAX_PARTICLES as f32).max(0.0);
    //a fraction of a particle per frame is made up by sometimes making one more, so the rate is right at any frame rate
    let wanted = speed * dt / TRAIL_SPACING * room;
    let count = wanted.floor() as usize + (rand::gen_range(0.0, 1.0) < wanted.fract()) as usize;
    for _ in 0..count {
        let spread = rand::gen_range(-1.0, 1.0);
        //start somewhere along the way the circle came this frame, from behind its middle, so the trail has no gaps
        let start = position + behind * (RADIUS * 0.6 + speed * rand::gen_range(0.0, dt)) + sideways * spread * RADIUS * 0.6;
        let fly = behind * TRAIL_SPEED * rand::gen_range(0.4, 1.0) + sideways * spread * TRAIL_SPEED * 0.3;
        particles.push(Particle { position: start, velocity: fly, age: 0.0, life: TRAIL_LIFE * rand::gen_range(0.6, 1.0), color, size: TRAIL_SIZE });
    }
}

pub(crate) fn update_particles(particles: &mut Vec<Particle>, dt: f32) {
    let slow_down = (-TRAIL_DRAG * dt).exp();
    particles.retain_mut(|particle| {
        particle.age += dt;
        particle.position += particle.velocity * dt;
        particle.velocity *= slow_down;
        particle.age < particle.life
    });
}

//the particles as circles that shrink and fade as they age, there are thousands of them
pub(crate) fn particle_discs(particles: &[Particle]) -> Vec<Disc> {
    particles
        .iter()
        .map(|particle| {
            let fade = (1.0 - particle.age / particle.life).max(0.0);
            Disc::new(particle.position.x, particle.position.y, particle.size * fade, Color::new(particle.color.r, particle.color.g, particle.color.b, particle.color.a * fade * 0.7))
        })
        .collect()
}

pub(crate) const POPUP_INTERVAL: f32 = 0.4; //seconds between the numbers an enemy shows for the damage it takes and the units it kills
pub(crate) const POPUP_LIFE: f32 = 1.0; //seconds a number lasts
pub(crate) const POPUP_RISE: f32 = 50.0; //pixels on screen a number floats up in that time
pub(crate) const POPUP_FONT: f32 = 26.0; //pixels, twice the default font's 13 so it stays sharp
pub(crate) const MAX_POPUPS: usize = 80; //numbers on screen at once, more than that can't be read anyway
pub(crate) const MAX_BURSTS: usize = 90; //puffs made in a frame, more than that look the same
pub(crate) const MAX_CUES: usize = 64; //sounds asked for in a frame
pub(crate) const BURST_SIZE: f32 = RADIUS * 0.4; //radius of the particles in a puff
pub(crate) const SHAKE_PIXELS: f32 = 22.0; //how far the screen shakes at the most
pub(crate) const SHAKE_DECAY: f32 = 5.0; //how quickly it settles, per second

//a number that floats up from where something happened, and fades
pub(crate) struct Popup {
    pub(crate) position: Vec2,
    pub(crate) lane: f32, //pixels on screen it starts above that, so two numbers from the same place can both be read
    pub(crate) text: String,
    pub(crate) color: Color,
    pub(crate) age: f32, //seconds
}

//a puff of particles to be made where something happened
pub(crate) struct Burst {
    pub(crate) position: Vec2,
    pub(crate) color: Color,
    pub(crate) count: usize,
    pub(crate) speed: f32, //pixels per second the fastest particles fly at
}

//a ring that spreads out from a point and fades, for a shockwave
pub(crate) struct Ring {
    pub(crate) position: Vec2,
    pub(crate) radius: f32, //pixels it ends up
    pub(crate) age: f32,
    pub(crate) life: f32,   //seconds
    pub(crate) color: Color,
}

//an explosion that hurts the walls it reaches
pub(crate) struct Blast {
    pub(crate) position: Vec2,
    pub(crate) radius: f32,
    pub(crate) damage: f32, //to each wall
}

//a sound for main to play
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Cue {
    Pop,       //a baby grew into a unit
    Thud,      //an enemy was destroyed
    Dispatch,  //a squad was sent
    Alert,     //an enemy noticed the player
    Shoot,     //an enemy fired
    Death,     //a unit died
    Milestone, //the army got to a round number
    Defeat,    //the army is gone
    Warning,   //a boss is winding up an attack
    Slam,      //a boss's stomp or pulse landed
    Explosion, //a bomb, a shell or a meteor landed
    PhaseShift, //a boss changed phase
    Relic,     //a relic was chosen
    Charge,    //something started to dash
    Heal,      //a healer mended the enemies round it
    Summon,    //something called up minions
    WallBreak, //a wall gave way
    Arrow,     //a tower shot
    Pull,      //a gravity well started to drag the army in
    Split,     //a boss tore in two
    Victory,   //the citadel fell
    Liberate,  //a village was freed
    Heartbeat, //the army is nearly gone
    SquadLost, //every unit sent at an enemy has died
    Combo,     //a kill streak reached a new step
    Pop2,      //a baby grew into a unit, higher: the pickups are coming one after another...
    Pop3,
    Pop4,
    Pop5,
    Pop6,
    Champion,  //a champion joined the army
    Fall,      //a champion died
    Zap,       //lightning from a champion
    Frost,     //a frost pulse from a champion
    Hit,       //a bolt hit an enemy
    Crit,      //a champion's bolt hit hard
    Volley,    //the front of the army fired
    Shatter,   //a frozen enemy broke
}

//what a player's army has earned in a frame from the enemies it destroyed, which the game hands out to that player
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub(crate) struct Spoils {
    pub(crate) points: f32,   //wall points
    pub(crate) xp: f32,       //what the champions are taught
    pub(crate) heal_all: f32, //health every unit is to be given
}

//what happened in a frame that is worth showing and hearing, collected by the functions that move and fight and then spent
//by main: numbers float up, puffs become particles, the camera shakes and the sounds are played
#[derive(Default)]
pub(crate) struct Effects {
    pub(crate) shake: f32, //0 to 1, how hard the camera is shaking
    pub(crate) popups: Vec<Popup>,
    pub(crate) bursts: Vec<Burst>,
    pub(crate) cues: Vec<Cue>,
    pub(crate) picked_up: usize,    //babies that have grown since the player last saw a number for them (it is the watcher that counts them up, from the `grown` the world reports)
    pub(crate) picked_up_in: f32,   //seconds until they do
    pub(crate) grown: [usize; MAX_PLAYERS], //babies each player's army has taken since the world last told whoever is watching
    pub(crate) hazards: Vec<Hazard>, //hazards started this frame, for main to add to the ones in play
    pub(crate) announcements: Vec<(String, Color)>, //messages for the banner
    pub(crate) wall_hits: Vec<((i32, i32), f32)>, //damage to walls this frame, by grid square, for main to apply
    pub(crate) blasts: Vec<Blast>,  //explosions that hurt the walls they reach
    pub(crate) beams: Vec<Beam>,
    pub(crate) fallen: Vec<(Vec2, u8)>, //where units died this frame, and whose they were
    pub(crate) spoils: [Spoils; MAX_PLAYERS], //what each player's army earned this frame from the enemies it destroyed
    pub(crate) pulls: Vec<Pull>,    //gravity wells started this frame, for main to add to the ones in play
    pub(crate) raise: Vec<Wall>,    //walls a boss has raised this frame, for main to put in the world where there is room
    pub(crate) rings: Vec<Ring>,
    pub(crate) punch: f32,                   //0 to 1, how far the camera has jumped in for a moment
    pub(crate) marks: Vec<Mark>,             //marks to be left on the ground, which main takes
    pub(crate) shards: Vec<ShardBurst>,      //debris to be thrown, which main takes
    pub(crate) cue_spots: Vec<Option<Vec2>>, //where each of the cues was, if it was anywhere, in the same order
    pub(crate) recruits: Vec<(Vec2, Odds, usize)>,  //champions won this frame, where, and by which player, for the game to hand out
    pub(crate) shatters: Vec<ShatterBurst>, //the enemies that shattered or burst this frame, whose pieces are to be thrown
    pub(crate) hit_stop: f32, //seconds a world of one is to stand still for a champion's crit (see HIT_STOP)
    pub(crate) volleys: Vec<(Volley, Hurt)>, //the champions' volleys of the frame and what their bolts do: the world puts them in flight, and the watchers are told of them
    pub(crate) rebirths: Vec<(Vec2, u8, u8, u8, u8)>, //champions (their kind, plus one, their level and evolutions, and whose they are) that rose from the dead this frame and where, for their blaze
    pub(crate) alarms: Vec<(Vec2, Vec2)>,    //enemies that were hurt by units for the first time in a while: where each is, and where those units are, for the game to call the enemies near it down on them
}

impl Effects {
    //wall points a player earned from the world: a nest, a village, a chest. what the enemies their army destroyed were worth is added to their `spoils` as it happens
    pub(crate) fn earn(&mut self, player: usize, points: f32) {
        self.spoils[player.min(MAX_PLAYERS - 1)].points += points;
    }

    pub(crate) fn announce(&mut self, text: String, color: Color) {
        self.announcements.push((text, color));
    }

    //a number floating up from `above` pixels over a position
    pub(crate) fn popup(&mut self, position: Vec2, above: f32, lane: f32, text: String, color: Color) {
        if self.popups.len() < MAX_POPUPS {
            self.popups.push(Popup { position: position - Vec2::new(0.0, above), lane, text, color, age: 0.0 });
        }
    }

    pub(crate) fn burst(&mut self, position: Vec2, color: Color, count: usize, speed: f32) {
        if self.bursts.len() < MAX_BURSTS {
            self.bursts.push(Burst { position, color, count, speed });
        }
    }

    //a shockwave ring
    pub(crate) fn ring(&mut self, position: Vec2, radius: f32, color: Color) {
        if self.rings.len() < 24 {
            self.rings.push(Ring { position, radius, age: 0.0, life: 0.5, color });
        }
    }

    pub(crate) fn add_shake(&mut self, amount: f32) {
        self.shake = (self.shake + amount).min(1.0);
    }

    pub(crate) fn cue(&mut self, cue: Cue) {
        if self.cues.len() < MAX_CUES {
            self.cues.push(cue);
            self.cue_spots.push(None);
        }
    }

    //a sound that happened somewhere in the world, quieter the further it is from the player
    pub(crate) fn cue_at(&mut self, cue: Cue, at: Vec2) {
        if self.cues.len() < MAX_CUES {
            self.cues.push(cue);
            self.cue_spots.push(Some(at));
        }
    }

    //the sounds asked for, with where each was, and none left to be asked for again
    pub(crate) fn take_cues(&mut self) -> Vec<(Cue, Option<Vec2>)> {
        let cues = std::mem::take(&mut self.cues);
        let mut spots = std::mem::take(&mut self.cue_spots);
        spots.resize(cues.len(), None);
        cues.into_iter().zip(spots).collect()
    }

    //the camera jumps in a little and settles back
    pub(crate) fn add_punch(&mut self, amount: f32) {
        self.punch = (self.punch + amount).min(1.0);
    }

    //a mark left on the ground where something happened
    pub(crate) fn mark(&mut self, position: Vec2, radius: f32, color: Color, kind: MarkKind) {
        if self.marks.len() < MAX_MARKS_A_FRAME {
            self.marks.push(Mark { position, radius, color, kind, age: 0.0 });
        }
    }

    //pieces of something that broke, thrown out from where it was. `size` is how big it was
    pub(crate) fn debris(&mut self, position: Vec2, color: Color, size: f32) {
        if self.shards.len() < MAX_BURSTS {
            self.shards.push(ShardBurst { position, color, size });
        }
    }

    //age the numbers, let the shaking settle, and show the player the babies that grew as one number
    pub(crate) fn update(&mut self, dt: f32, player: Vec2) {
        self.popups.retain_mut(|popup| {
            popup.age += dt;
            popup.age < POPUP_LIFE
        });
        self.beams.retain_mut(|beam| {
            beam.age += dt;
            beam.age < BEAM_LIFE
        });
        self.rings.retain_mut(|ring| {
            ring.age += dt;
            ring.age < ring.life
        });
        self.punch *= (-PUNCH_DECAY * dt).exp();
        if self.punch < 0.005 {
            self.punch = 0.0;
        }
        self.shake *= (-SHAKE_DECAY * dt).exp();
        if self.shake < 0.01 {
            self.shake = 0.0;
        }
        self.picked_up_in -= dt;
        if self.picked_up > 0 && self.picked_up_in <= 0.0 {
            let text = format!("+{}", self.picked_up);
            self.popup(player, RADIUS * 2.0, 0.0, text, GREEN);
            self.picked_up = 0;
            self.picked_up_in = POPUP_INTERVAL;
        }
    }

    //turn the puffs into particles
    pub(crate) fn spawn_bursts(&mut self, particles: &mut Vec<Particle>) {
        for burst in self.bursts.drain(..) {
            for _ in 0..burst.count {
                //a puff can go a bit past the number of particles trails stop at
                if particles.len() >= MAX_PARTICLES * 3 / 2 {
                    break;
                }
                let velocity = Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * burst.speed * rand::gen_range(0.25, 1.0);
                particles.push(Particle { position: burst.position, velocity, age: 0.0, life: rand::gen_range(0.35, 0.8), color: burst.color, size: BURST_SIZE });
            }
        }
    }
}

//1234 as 1.2k, 56789 as 57k, and so on, so a number is short enough to read at a glance
pub(crate) fn compact_number(value: f32) -> String {
    if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 10_000.0 {
        format!("{:.0}k", value / 1000.0)
    } else if value >= 1000.0 {
        format!("{:.1}k", value / 1000.0)
    } else {
        format!("{:.0}", value)
    }
}

//how far the camera is moved from where it should be, in a random direction: the harder the shaking the further, and
//more than in proportion to it so a little is barely there. SHAKE_PIXELS is on screen, whatever the zoom
pub(crate) fn shake_offset(shake: f32, zoom: f32) -> Vec2 {
    if shake <= 0.0 {
        return Vec2::ZERO;
    }
    Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * shake * shake * SHAKE_PIXELS / zoom
}

//the floating numbers, at the same size on screen at any zoom, with a shadow so they can be read over anything
pub(crate) fn draw_popups(popups: &[Popup], zoom: f32) {
    for popup in popups {
        let life = popup.age / POPUP_LIFE;
        let alpha = (1.0 - life * life).clamp(0.0, 1.0);
        let rise = (POPUP_RISE * life + popup.lane) / zoom;
        let width = text_width(&popup.text, POPUP_FONT, Face::Bold) / zoom;
        let (x, y) = (popup.position.x - width / 2.0, popup.position.y - rise);
        draw_label_scaled(&popup.text, x + 2.0 / zoom, y + 2.0 / zoom, POPUP_FONT, 1.0 / zoom, Color::new(0.0, 0.0, 0.0, alpha * 0.8), Face::Bold);
        draw_label_scaled(&popup.text, x, y, POPUP_FONT, 1.0 / zoom, Color::new(popup.color.r, popup.color.g, popup.color.b, alpha), Face::Bold);
    }
}


//the rings, in the world: each grows to its size and fades
pub(crate) fn draw_rings(rings: &[Ring]) {
    for ring in rings {
        let t = (ring.age / ring.life).clamp(0.0, 1.0);
        let radius = ring.radius * (1.0 - (1.0 - t) * (1.0 - t));
        let fade = 1.0 - t;
        draw_circle(ring.position.x, ring.position.y, radius, Color::new(ring.color.r, ring.color.g, ring.color.b, 0.12 * fade));
        draw_circle_lines(ring.position.x, ring.position.y, radius, 8.0 * fade + 2.0, Color::new(ring.color.r, ring.color.g, ring.color.b, 0.8 * fade));
    }
}
