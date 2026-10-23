//! a game with others in it, from the game loop's side: the host's table of the players at other machines (what they ask for goes into the step, and each is
//! sent the world as they can see it), and the joined player's seat (what they ask for goes to the host, and the world they are sent is what they play in)

use crate::prelude::*;

//a player is sent nothing more while this many bytes of what they were sent have not been written to their connection, nor while this many of the
//messages they were sent have not been taken in: they are behind, and what they are sent when they have caught up is all of what changed meanwhile
pub(crate) const MAX_WAITING_BYTES: usize = 256 << 10;
pub(crate) const MAX_UNTAKEN: u32 = 24;
//a player whose frames come slower than the host's steps goes on walking the way they were for this long between them
const HOLD_SECONDS: f32 = 0.25;

// ---------- how the game is being played ----------

//who else is in the game, as far as this machine is concerned
pub(crate) enum Link {
    Solo,
    Host(NetHost),
    Guest(NetClient, Lobby), //the game of another, which the player has joined and which has started
}

// ---------- the host ----------

//a player at another machine, as the host's game knows them. they keep their place in the world when they go (their units stand where they are) and have it
//back if someone of the same name joins again
struct Remote {
    slot: u8,       //which connection, while they are on one
    player: usize,  //who they are in the world
    name: String,
    viewer: Viewer, //what they have been told of the world
    here: bool,     //whether they are on a connection now
    asked: Option<PlayerInput>, //what they have asked for since the last step, as one
    last: PlayerInput,          //the last thing they asked for, to go on walking the way they were between frames
    stale: f32,                 //how long since they asked
    heard: u32,                 //the number of the latest of their frames that was heard
    acted: u32,                 //the number of the latest of them the world has taken a step for
    taken: u32,                 //how many of the host's messages they have taken in
    due: f32,                   //seconds until they are due another message
}

pub(crate) struct Table {
    remotes: Vec<Remote>,
    clock: f32,
}

impl Table {
    pub(crate) fn new() -> Table {
        Table { remotes: Vec::new(), clock: 0.0 }
    }

    //a player has joined on a connection: they take their place in the world, which is the one they left if they have been in it before (or one a saved game
    //has for them), with their army held for them, or a new one. fails with why if the world can't take them
    pub(crate) fn seat(&mut self, sim: &mut Sim, slot: u8, name: &str) -> Result<usize, &'static str> {
        if let Some(remote) = self.remotes.iter_mut().find(|remote| remote.name.eq_ignore_ascii_case(name)) {
            if remote.here && remote.slot != slot {
                return Err("that name is in use");
            }
            remote.slot = slot;
            remote.here = true;
            remote.viewer = Viewer::new(remote.player);
            remote.due = 0.0;
            remote.heard = 0;
            remote.acted = 0;
            remote.taken = 0;
            remote.asked = None;
            sim.release(remote.player);
            return Ok(remote.player);
        }
        let known = sim.players.list.iter().skip(1).position(|player| player.name.eq_ignore_ascii_case(name)).map(|place| place + 1);
        let player = match known {
            Some(player) => {
                sim.release(player);
                player
            }
            None => sim.add_player(name).ok_or("the world has all the players it can hold")?,
        };
        let stagger = SNAPSHOT_SECONDS * (self.remotes.len() as f32 / MAX_PLAYERS as f32);
        self.remotes.push(Remote {
            slot,
            player,
            name: clean_name(name),
            viewer: Viewer::new(player),
            here: true,
            asked: None,
            last: PlayerInput::idle(Formation::Free, DEFAULT_SCREEN),
            stale: HOLD_SECONDS,
            heard: 0,
            acted: 0,
            taken: 0,
            due: stagger,
        });
        Ok(player)
    }

    //a player has gone: their army is held where it was
    pub(crate) fn unseat(&mut self, sim: &mut Sim, slot: u8) {
        if let Some(remote) = self.remotes.iter_mut().find(|remote| remote.here && remote.slot == slot) {
            remote.here = false;
            remote.asked = None;
            sim.hold(remote.player);
        }
    }

    //how many messages of the world a connection has been sent
    #[cfg(test)]
    pub(crate) fn sent_to(&self, slot: u8) -> u32 {
        self.remotes.iter().find(|remote| remote.slot == slot).map_or(0, |remote| remote.viewer.ticks())
    }

    //what the players have sent: each frame is kept with the ones since the last step, as one. a frame that is not one, or is not a later one than the last,
    //is ignored
    pub(crate) fn hear(&mut self, heard: Vec<(u8, Vec<u8>)>) {
        for (slot, bytes) in heard {
            let Some(remote) = self.remotes.iter_mut().find(|remote| remote.here && remote.slot == slot) else { continue };
            let Some(frame) = ClientFrame::from_bytes(&bytes) else { continue };
            if frame.seq <= remote.heard {
                continue;
            }
            remote.heard = frame.seq;
            //(no more than they have been sent, whatever they say)
            remote.taken = remote.taken.max(frame.tick_ack.min(remote.viewer.ticks()));
            remote.viewer.zoom = frame.zoom;
            remote.viewer.screen = frame.input.screen;
            remote.asked = Some(match remote.asked.take() {
                Some(before) => before.then(frame.input),
                None => frame.input,
            });
        }
    }

    //what everyone asks of the next step, the host's first: what each player at another machine has asked for since the last, or the way they were walking
    //for a moment if they have not asked, or nothing
    pub(crate) fn inputs(&mut self, sim: &Sim, local: PlayerInput, dt: f32) -> Vec<PlayerInput> {
        let mut inputs = Vec::with_capacity(sim.players.len());
        inputs.push(local);
        for player in 1..sim.players.len() {
            let formation = sim.players.list[player].formation;
            let input = match self.remotes.iter_mut().find(|remote| remote.player == player) {
                None => PlayerInput::idle(formation, DEFAULT_SCREEN),
                Some(remote) => {
                    remote.acted = remote.heard;
                    match remote.asked.take() {
                        Some(input) => {
                            remote.last = input.clone();
                            remote.stale = 0.0;
                            input
                        }
                        None => {
                            remote.stale += dt;
                            let direction = if remote.here && remote.stale < HOLD_SECONDS { remote.last.direction } else { Vec2::ZERO };
                            PlayerInput { direction, ..PlayerInput::idle(formation, remote.last.screen) }
                        }
                    }
                }
            };
            inputs.push(input);
        }
        inputs
    }

    //after a step: what happened is kept for the players who were not there to see it, and each who is due a message of the world is sent one. `dt` is how
    //long the frame was
    pub(crate) fn tell(&mut self, net: &NetHost, sim: &mut Sim, events: &[Event], report: &Report, dt: f32) {
        self.clock += dt;
        for remote in self.remotes.iter_mut().filter(|remote| remote.here) {
            remote.viewer.hear(events);
            remote.due -= dt;
        }
        if !self.remotes.iter().any(|remote| remote.here && remote.due <= 0.0) {
            return;
        }
        let ranges = sim.arrange();
        let index = WallIndex::build(&sim.walls);
        let frame = Frame { sim, ranges: &ranges, walls: &index, time: self.clock, boosts: &report.boosts, pickups: &report.pickup_ranges };
        for remote in self.remotes.iter_mut().filter(|remote| remote.here && remote.due <= 0.0) {
            //(one that is a long way behind is not caught up with in a burst)
            remote.due = (remote.due + SNAPSHOT_SECONDS).max(-SNAPSHOT_SECONDS);
            if net.waiting(remote.slot) > MAX_WAITING_BYTES || remote.viewer.ticks().saturating_sub(remote.taken) > MAX_UNTAKEN {
                continue;
            }
            let message = remote.viewer.tick(&frame, remote.acted);
            net.send_world(remote.slot, message);
        }
    }
}

// ---------- a player who has joined ----------

//how to join a game again, as the same player: where it is, who they are and the password
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Reconnect {
    pub(crate) target: String,
    pub(crate) name: String,
    pub(crate) password: String,
}

//how a stay in a game of another ended, and whether it is worth trying to get back into it (it is when the connection went, and the host has not said it is over)
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Lost {
    pub(crate) why: String,
    pub(crate) retry: bool,
}

impl Lost {
    fn over(why: impl Into<String>) -> Lost {
        Lost { why: why.into(), retry: false }
    }

    //what the host's side of the connection said to end the stay: the host saying it is over (it has closed the game, or put the player out) is the end of it, and the
    //connection going, or something going wrong on it, is not
    pub(crate) fn by(event: ClientEvent) -> Lost {
        match event {
            ClientEvent::Refused(refusal) => Lost::over(refusal.text()),
            ClientEvent::Failed(problem) => Lost { why: problem, retry: true },
            ClientEvent::Closed(why) => Lost { retry: why != "you left the game", why },
            _ => Lost::over("the connection ended"),
        }
    }
}

impl From<Lost> for String {
    fn from(lost: Lost) -> String {
        lost.why
    }
}

//the game of another that the player has joined: the world is theirs to look at and to ask things of, and the host takes the steps
pub(crate) struct Guest {
    client: NetClient,
    pub(crate) lobby: Lobby,
    pub(crate) replica: Replica,
    early: Vec<Vec<u8>>, //messages of the world that came before the game did
    seq: u32,
    taken: u32,        //how many of the host's messages have been taken in
    heard_at: Instant, //when the host last said anything
}

impl Guest {
    pub(crate) fn new(client: NetClient, mut lobby: Lobby) -> Guest {
        let early = std::mem::take(&mut lobby.world);
        Guest { client, lobby, replica: Replica::new(), early, seq: 0, taken: 0, heard_at: Instant::now() }
    }

    //who the player is in the world (when they have been told)
    pub(crate) fn me(&self) -> usize {
        self.replica.me
    }

    //whether the host has said enough to show the world
    pub(crate) fn ready(&self) -> bool {
        self.replica.ready()
    }

    //how to join the game again
    pub(crate) fn reconnection(&self) -> Reconnect {
        self.client.reconnection()
    }

    //how many seconds it is since the host said anything (it says something several times a second)
    pub(crate) fn silence(&self) -> f32 {
        self.heard_at.elapsed().as_secs_f32()
    }

    //what the host has said since the last frame is taken in. fails with why if the stay is over, or the host has said what can't be understood
    pub(crate) fn hear(&mut self, sim: &mut Sim) -> Result<(), Lost> {
        for bytes in std::mem::take(&mut self.early) {
            self.take_in(sim, &bytes)?;
        }
        for event in self.client.poll() {
            self.heard_at = Instant::now();
            match event {
                ClientEvent::World(bytes) => self.take_in(sim, &bytes)?,
                ClientEvent::Roster(members) => self.lobby.members = members,
                ClientEvent::Joined { .. } | ClientEvent::Started => {}
                ending => return Err(Lost::by(ending)),
            }
        }
        Ok(())
    }

    fn take_in(&mut self, sim: &mut Sim, bytes: &[u8]) -> Result<(), Lost> {
        self.replica.apply(sim, bytes).map_err(|garbled| Lost::over(garbled.0))?;
        self.taken += 1;
        Ok(())
    }

    //a frame of the player's game: what they asked for is sent to the host, and their avatar goes where they ask at once (the host will say if that was wrong),
    //and everything else is moved on to where it should be shown. what happened that they could see or hear is returned
    pub(crate) fn frame(&mut self, sim: &mut Sim, input: &PlayerInput, zoom: f32, dt: f32) -> Vec<Event> {
        if !self.replica.ready() {
            return Vec::new();
        }
        let choosing = sim.players.list[self.replica.me].choice.is_some();
        if !choosing {
            self.replica.walked(sim, input.direction, dt);
        }
        self.seq += 1;
        let frame = ClientFrame { seq: self.seq, tick_ack: self.taken, zoom, input: input.clone() };
        self.client.send_input(frame.to_bytes());
        self.replica.sent(self.seq);
        self.replica.advance(sim, dt);
        self.replica.take_events()
    }

    //what the host's step would have reported, as far as it has said
    pub(crate) fn report(&self, sim: &Sim) -> Report {
        let count = sim.players.len();
        let mut boosts = vec![1.0; count];
        let mut pickup_ranges = vec![BABY_PICKUP_RANGE; count];
        if let (Some(boost), Some(range)) = (boosts.get_mut(self.replica.me), pickup_ranges.get_mut(self.replica.me)) {
            *boost = self.replica.boost;
            *range = self.replica.pickup_range;
        }
        Report { boosts, pickup_ranges, ..Report::default() }
    }

    //say goodbye to the host
    pub(crate) fn leave(&mut self) {
        self.client.leave();
    }
}
