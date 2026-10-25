//! a player's side of watching the world a host runs: the messages the host sends (see `Viewer`) are put into a world of the player's own (a `Sim` that is
//! only looked at, never stepped), which the game then draws as if it were the one it was running. things are shown a little behind the latest of what
//! the host has said, so there is always a later place for each to be moving towards; the player's own avatar is not, it moves at once as they ask and is put
//! right when the host says where it really is

use crate::prelude::*;
use std::collections::VecDeque;

//how many of the places something has been told to be at are kept: enough to cover the time the picture is behind the host (INTERP_DELAY), with a message
//that is late to spare
const TRACK_POINTS: usize = 4;

//when the host says the avatar is somewhere other than where the player has taken it: if that is further than this it is put there at once, and if not it is moved
//this share of the way each time (so a small difference is made up in a few messages without a jerk)
const SNAP_DISTANCE: f32 = 400.0;
const CORRECTION_SHARE: f32 = 0.35;

//where something has been, as far as it has been told: the last few places and the times (in seconds of the host's clock) it was at them, oldest first
#[derive(Clone, Copy, Debug)]
pub(crate) struct Track {
    at: [Vec2; TRACK_POINTS],
    t: [f32; TRACK_POINTS],
}

impl Track {
    pub(crate) fn at(at: Vec2, t: f32) -> Track {
        Track { at: [at; TRACK_POINTS], t: [t; TRACK_POINTS] }
    }

    fn push(&mut self, at: Vec2, t: f32) {
        self.at.rotate_left(1);
        self.t.rotate_left(1);
        self.at[TRACK_POINTS - 1] = at;
        self.t[TRACK_POINTS - 1] = t;
    }

    //it has been told where it is at a time. if it was still for a while before that, it was where it had been until just before, and is not shown
    //gliding there over all that time
    pub(crate) fn told(&mut self, at: Vec2, t: f32) {
        let last = TRACK_POINTS - 1;
        if t - self.t[last] > 1.5 * SNAPSHOT_SECONDS {
            self.push(self.at[last], t - SNAPSHOT_SECONDS);
        }
        self.push(at, t);
    }

    //the piece of its way that a time is in: from the place at this number to the one after it
    fn piece(&self, t: f32) -> usize {
        (0..TRACK_POINTS - 1).rev().find(|&i| self.t[i] <= t).unwrap_or(0)
    }

    pub(crate) fn place(&self, t: f32) -> Vec2 {
        let last = TRACK_POINTS - 1;
        if t >= self.t[last] {
            return self.at[last];
        }
        if t <= self.t[0] {
            return self.at[0];
        }
        let i = self.piece(t);
        self.at[i].lerp(self.at[i + 1], (t - self.t[i]) / (self.t[i + 1] - self.t[i]))
    }

    //the last place it was told to be at
    pub(crate) fn latest(&self) -> Vec2 {
        self.at[TRACK_POINTS - 1]
    }

    //how fast it is going, which is what trails are made from
    pub(crate) fn velocity(&self, t: f32) -> Vec2 {
        if t > self.t[TRACK_POINTS - 1] + SNAPSHOT_SECONDS || t < self.t[0] {
            return Vec2::ZERO;
        }
        let i = self.piece(t);
        let span = self.t[i + 1] - self.t[i];
        if span > 0.0 { (self.at[i + 1] - self.at[i]) / span } else { Vec2::ZERO }
    }
}

//where a unit is said to be
enum Place {
    At(Vec2),
    Moved(i32, i32), //by this many pixels (in the packed numbers) from where it was last said to be
}

impl Place {
    fn resolve(self, was: Vec2) -> Option<Vec2> {
        match self {
            Place::At(at) => Some(at),
            Place::Moved(dx, dy) => {
                let (x, y) = (pack(was.x) as i32 + dx, pack(was.y) as i32 + dy);
                ((0..=65535).contains(&x) && (0..=65535).contains(&y)).then(|| Vec2::new(unpack(x as u16), unpack(y as u16)))
            }
        }
    }
}

//no walls on any of the squares of a chunk
fn clear_chunk(walls: &mut Walls, chunk: (i32, i32)) {
    for col in chunk.0 * WALL_CHUNK..(chunk.0 + 1) * WALL_CHUNK {
        for row in chunk.1 * WALL_CHUNK..(chunk.1 + 1) * WALL_CHUNK {
            walls.remove(&(col, row));
        }
    }
}

//what is wrong with a message
#[derive(Debug, PartialEq)]
pub(crate) struct Garbled(pub(crate) &'static str);

pub(crate) struct Replica {
    pub(crate) me: usize,
    ready: bool,
    base: Option<f64>,   //the host's clock at the first message, which times are counted from so they stay small
    latest: f32,         //the host's clock at the latest message
    shown: f32,          //the time that is being shown, a little behind it
    seq: u32,            //the latest message
    pub(crate) acked: u32,
    unit_tracks: Vec<Track>,
    enemy_tracks: Vec<Track>,
    enemy_at: HashMap<u32, usize>,
    baby_keys: Vec<u32>,
    player_tracks: Vec<Option<Track>>,
    pub(crate) boost: f32,
    pub(crate) pickup_range: f32,
    pub(crate) armies: Vec<usize>, //how many units each player has, in view or not
    //what the player has asked of their avatar and where that puts it
    pub(crate) predicted: Vec2,
    history: VecDeque<(u32, Vec2)>,
    events: Vec<Event>,
}

impl Replica {
    pub(crate) fn new() -> Replica {
        Replica {
            me: 0,
            ready: false,
            base: None,
            latest: 0.0,
            shown: 0.0,
            seq: 0,
            acked: 0,
            unit_tracks: Vec::new(),
            enemy_tracks: Vec::new(),
            enemy_at: HashMap::new(),
            baby_keys: Vec::new(),
            player_tracks: Vec::new(),
            boost: 1.0,
            pickup_range: BABY_PICKUP_RANGE,
            armies: Vec::new(),
            predicted: Vec2::ZERO,
            history: VecDeque::new(),
            events: Vec::new(),
        }
    }

    //whether the host has said enough to show anything
    pub(crate) fn ready(&self) -> bool {
        self.ready
    }

    //what happened in the world that was in view, since this was last asked
    pub(crate) fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    //how far behind the host the picture is, in seconds
    #[cfg(test)]
    pub(crate) fn lag(&self) -> f32 {
        (self.latest - self.shown).max(0.0)
    }

    //put what the host said into the world. a message that is not one is an error, and the world is no good after it
    pub(crate) fn apply(&mut self, sim: &mut Sim, bytes: &[u8]) -> Result<(), Garbled> {
        self.parse(sim, bytes).ok_or(Garbled("the host sent what could not be understood"))
    }

    fn parse(&mut self, sim: &mut Sim, bytes: &[u8]) -> Option<()> {
        let mut input = Reader::new(bytes);
        if input.u8()? != DOWN_TICK {
            return None;
        }
        let (seq, millis, ack, me, first) = (input.u32()?, input.u32()?, input.u32()?, input.u8()? as usize, input.flag()?);
        //(they arrive in order, so one that is not the next is a host that has lost its place)
        //(a first one starts the world again, as a host does that when a new run begins)
        if !first && (!self.ready || seq != self.seq.wrapping_add(1)) {
            return None;
        }
        if me >= MAX_PLAYERS {
            return None;
        }
        let clock = millis as f64 / 1000.0;
        if first {
            self.base = None;
        }
        let base = *self.base.get_or_insert(clock);
        let t = (clock - base) as f32;
        if first {
            *sim = Sim::mirror();
            *self = Replica { base: self.base, ..Replica::new() };
        }
        self.seq = seq;
        self.me = me;
        self.acked = ack;
        self.latest = t;
        if first {
            self.shown = t - INTERP_DELAY;
        }

        self.read_players(sim, &mut input, t, first)?;
        self.read_mine(sim, &mut input)?;
        if input.flag()? {
            sim.run = from_text(input.blob(MAX_TEXT_BLOB)?)?;
            sim.director = from_text(input.blob(MAX_TEXT_BLOB)?)?;
            let chests = std::mem::take(&mut sim.hordes.chests);
            sim.hordes = from_text(input.blob(MAX_TEXT_BLOB)?)?;
            sim.hordes.chests = chests;
        }
        self.read_sites(sim, &mut input)?;
        self.read_walls(sim, &mut input)?;
        for owner in 0..sim.players.len() {
            self.read_units(sim, &mut input, owner, t)?;
        }
        self.read_extras(sim, &mut input)?;
        self.read_enemies(sim, &mut input, t)?;
        self.read_babies(sim, &mut input)?;
        let count = input.varint()? as usize;
        if count > MAX_RECORDS {
            return None;
        }
        sim.shots = (0..count).map(|_| read_shot(&mut input)).collect::<Option<Vec<_>>>()?;
        let count = input.varint()? as usize;
        if count > MAX_RECORDS {
            return None;
        }
        sim.hazards = (0..count).map(|_| read_hazard(&mut input)).collect::<Option<Vec<_>>>()?;
        let count = input.varint()? as usize;
        if count > MAX_RECORDS {
            return None;
        }
        sim.pulls = (0..count).map(|_| read_pull(&mut input)).collect::<Option<Vec<_>>>()?;
        let count = input.varint()? as usize;
        if count > MAX_RECORDS {
            return None;
        }
        sim.hordes.chests = (0..count).map(|_| read_chest(&mut input)).collect::<Option<Vec<_>>>()?;
        let events = read_events(input.rest())?;
        self.events.extend(events);
        if first {
            self.ready = true;
            self.predicted = sim.players.list[me].position;
        }
        Some(())
    }

    fn read_players(&mut self, sim: &mut Sim, input: &mut Reader, t: f32, first: bool) -> Option<()> {
        let count = input.u8()? as usize;
        if count == 0 || count > MAX_PLAYERS || count < sim.players.len() || self.me >= count {
            return None;
        }
        while sim.players.len() < count {
            let slot = sim.players.len();
            sim.players.list.push(Player::viewing(slot, "Player"));
            self.player_tracks.push(None);
        }
        self.armies.clear();
        for i in 0..count {
            let (position, velocity, facing) = (input.vec2()?, input.vec2()?, input.vec2()?);
            let formation = byte_formation(input.u8()?)?;
            let army = input.u32()? as usize;
            let choosing = input.u8()? != 0;
            self.armies.push(army);
            let player = &mut sim.players.list[i];
            player.velocity = velocity;
            player.facing = facing;
            player.formation = formation;
            //(whether they are choosing is all that is known of what they are choosing from: a placeholder, which is never shown)
            if i != self.me {
                player.choice = choosing.then(|| RelicChoice { options: [Relic::Whetstone; 3], age: 0.0, rare_only: false, chest: 0 });
            }
            match &mut self.player_tracks[i] {
                Some(track) => track.told(position, t),
                none => {
                    *none = Some(Track::at(position, t));
                    player.position = position;
                }
            }
            if i == self.me && !first {
                self.reconcile(position);
            }
        }
        if input.flag()? {
            for i in 0..count {
                let name = input.text()?;
                sim.players.list[i].name = clean_name(&name);
            }
        }
        Some(())
    }

    //the host says where the avatar is, as of the last of its inputs it acted on: if that is not where it was thought to be, the avatar is moved most of
    //the way there (all of it if it is far), and so is everything it has been thought to do since
    fn reconcile(&mut self, authoritative: Vec2) {
        while self.history.front().is_some_and(|&(seq, _)| seq < self.acked) {
            self.history.pop_front();
        }
        let thought = match self.history.front() {
            Some(&(seq, at)) if seq == self.acked => Some(at),
            _ => None,
        };
        let error = match thought {
            Some(at) => authoritative - at,
            //(nothing is waiting to be acted on, so the host's is where it should be)
            None if self.history.is_empty() => authoritative - self.predicted,
            None => return,
        };
        let share = if error.length() > SNAP_DISTANCE { 1.0 } else { CORRECTION_SHARE };
        let shift = error * share;
        for (_, at) in self.history.iter_mut() {
            *at += shift;
        }
        self.predicted += shift;
    }

    fn read_mine(&mut self, sim: &mut Sim, input: &mut Reader) -> Option<()> {
        let me = &mut sim.players.list[self.me];
        self.boost = input.f32()?.clamp(0.1, 100.0);
        self.pickup_range = input.f32()?.max(0.0);
        me.mason.selected = *ALL_WALL_KINDS.get(input.u8()? as usize)?;
        me.mason.points = input.f32()?;
        me.rerolls = input.u32()?;
        let limit = input.u32()? as usize;
        if input.flag()? {
            me.upgrades = from_text(input.blob(MAX_TEXT_BLOB)?)?;
        }
        me.upgrades.army_limit = limit;
        match input.u8()? {
            0 => me.choice = None,
            1 => {}
            2 => {
                let mut choice: RelicChoice = from_text(input.blob(MAX_TEXT_BLOB)?)?;
                choice.age = 0.0;
                me.choice = Some(choice);
            }
            _ => return None,
        }
        Some(())
    }

    fn read_sites(&mut self, sim: &mut Sim, input: &mut Reader) -> Option<()> {
        match input.u8()? {
            0 => {}
            1 => sim.sites = from_text(input.blob(MAX_TEXT_BLOB)?)?,
            2 => {
                let count = input.varint()? as usize;
                if count > MAX_RECORDS {
                    return None;
                }
                for _ in 0..count {
                    let (id, state, core, clock) = (input.u32()?, byte_site_state(input.u8()?)?, (input.flag()?, input.u32()?), input.f32()?);
                    if let Some(site) = sim.sites.iter_mut().find(|site| site.id == id) {
                        site.state = state;
                        site.core = core.0.then_some(core.1);
                        site.clock = clock.max(0.0);
                    }
                }
            }
            _ => return None,
        }
        Some(())
    }

    fn read_walls(&mut self, sim: &mut Sim, input: &mut Reader) -> Option<()> {
        let chunks = input.varint()? as usize;
        if chunks > (WALL_CHUNKS * WALL_CHUNKS) as usize {
            return None;
        }
        for _ in 0..chunks {
            let chunk = (input.u8()? as i32, input.u8()? as i32);
            if chunk.0 >= WALL_CHUNKS || chunk.1 >= WALL_CHUNKS {
                return None;
            }
            let count = input.varint()? as usize;
            if count > (WALL_CHUNK * WALL_CHUNK) as usize {
                return None;
            }
            clear_chunk(&mut sim.walls, chunk);
            for _ in 0..count {
                let rec = WallRec::read(input, chunk)?;
                sim.walls.insert((rec.col, rec.row), rec.wall());
            }
        }
        let forgotten = input.varint()? as usize;
        if forgotten > (WALL_CHUNKS * WALL_CHUNKS) as usize {
            return None;
        }
        for _ in 0..forgotten {
            let chunk = (input.u8()? as i32, input.u8()? as i32);
            if chunk.0 >= WALL_CHUNKS || chunk.1 >= WALL_CHUNKS {
                return None;
            }
            clear_chunk(&mut sim.walls, chunk);
        }
        Some(())
    }

    //one army: the units that are gone, and the ones that are new or have changed, both in order of id
    fn read_units(&mut self, sim: &mut Sim, input: &mut Reader, owner: usize, t: f32) -> Option<()> {
        let removed = {
            let count = input.varint()? as usize;
            if count > MAX_RECORDS {
                return None;
            }
            let mut ids = Vec::with_capacity(count.min(1 << 16));
            let mut id = 0u32;
            for _ in 0..count {
                id = id.checked_add(input.varint()?)?;
                ids.push(id);
            }
            ids
        };
        let count = input.varint()? as usize;
        if count > MAX_RECORDS {
            return None;
        }
        //(the army is the stretch of the list that is theirs: the list is in order of owner)
        let start = sim.units.partition_point(|unit| (unit.owner as usize) < owner);
        let end = sim.units.partition_point(|unit| (unit.owner as usize) <= owner);
        let mut taken: Vec<(Unit, Track)> = sim.units.splice(start..end, std::iter::empty()).zip(self.unit_tracks.splice(start..end, std::iter::empty())).collect();
        let mut made: Vec<(Unit, Track)> = Vec::with_capacity(taken.len() + count.min(1 << 16));
        let mut kept = taken.drain(..).peekable();
        let mut removed = removed.into_iter().peekable();
        let mut id = 0u32;
        let mut previous: Option<u32> = None;
        for _ in 0..count {
            let head = input.varint()?;
            id = id.checked_add(head >> 3)?;
            if previous.is_some_and(|before| id <= before) {
                return None;
            }
            previous = Some(id);
            let (flags, small) = ((head & 3) as u8, head & 4 != 0);
            if flags == 0 || (small && flags & 1 == 0) {
                return None;
            }
            //(where it is: said as it is, or as how far it has moved from where it was last said to be)
            let at = match (flags & 1 != 0, small) {
                (false, _) => None,
                (true, false) => Some(Place::At(Vec2::new(unpack(input.u16()?), unpack(input.u16()?)))),
                (true, true) => Some(Place::Moved(input.u8()? as i8 as i32, input.u8()? as i8 as i32)),
            };
            let look = if flags & 2 != 0 { Some(UnitLook::read(input)?) } else { None };
            //(every unit before this one that is not changed is kept, unless it was told to be gone)
            while kept.peek().is_some_and(|(unit, _)| unit.id < id) {
                let (unit, track) = kept.next()?;
                while removed.peek().is_some_and(|&gone| gone < unit.id) {
                    removed.next();
                }
                if removed.peek() == Some(&unit.id) {
                    removed.next();
                } else {
                    made.push((unit, track));
                }
            }
            if kept.peek().is_some_and(|(unit, _)| unit.id == id) {
                let (mut unit, mut track) = kept.next()?;
                if let Some(place) = at {
                    track.told(place.resolve(track.latest())?, t);
                }
                if let Some(look) = look {
                    look.apply(&mut unit);
                }
                made.push((unit, track));
            } else {
                //(it is new, which is said with all of it)
                let (Place::At(at), Some(look)) = (at?, look) else { return None };
                let mut unit = Unit::replica(id, owner, at);
                look.apply(&mut unit);
                made.push((unit, Track::at(at, t)));
            }
        }
        for (unit, track) in kept {
            while removed.peek().is_some_and(|&gone| gone < unit.id) {
                removed.next();
            }
            if removed.peek() == Some(&unit.id) {
                removed.next();
            } else {
                made.push((unit, track));
            }
        }
        //(back in their place in the list)
        let (units, tracks): (Vec<Unit>, Vec<Track>) = made.into_iter().unzip();
        let at = start;
        sim.units.splice(at..at, units);
        self.unit_tracks.splice(at..at, tracks);
        Some(())
    }

    fn read_extras(&mut self, sim: &mut Sim, input: &mut Reader) -> Option<()> {
        let count = input.varint()? as usize;
        if count > MAX_RECORDS {
            return None;
        }
        let start = sim.units.partition_point(|unit| (unit.owner as usize) < self.me);
        let end = sim.units.partition_point(|unit| (unit.owner as usize) <= self.me);
        for _ in 0..count {
            let extra = ChampionExtra::read(input)?;
            if let Ok(at) = sim.units[start..end].binary_search_by_key(&extra.id, |unit| unit.id) {
                let unit = &mut sim.units[start + at];
                unit.power = extra.power;
                unit.xp = extra.xp;
                unit.ward = byte_share(extra.ward);
            }
        }
        Some(())
    }

    fn read_enemies(&mut self, sim: &mut Sim, input: &mut Reader, t: f32) -> Option<()> {
        let count = input.varint()? as usize;
        if count > MAX_RECORDS {
            return None;
        }
        let gone: HashSet<u32> = (0..count).map(|_| input.varint()).collect::<Option<_>>()?;
        if !gone.is_empty() {
            let mut at = 0;
            let tracks = std::mem::take(&mut self.enemy_tracks);
            let mut kept_tracks = Vec::with_capacity(tracks.len());
            sim.enemies.retain(|enemy| {
                let keep = !gone.contains(&enemy.id);
                if keep {
                    kept_tracks.push(tracks[at]);
                }
                at += 1;
                keep
            });
            self.enemy_tracks = kept_tracks;
            self.enemy_at = sim.enemies.iter().enumerate().map(|(i, enemy)| (enemy.id, i)).collect();
        }
        let count = input.varint()? as usize;
        if count > MAX_RECORDS {
            return None;
        }
        for _ in 0..count {
            let id = input.varint()?;
            let flags = input.u8()?;
            if flags == 0 || flags > 3 {
                return None;
            }
            let blob = if flags & 1 != 0 { Some(read_enemy_blob(input.blob(MAX_TEXT_BLOB)?)?) } else { None };
            let seen = if flags & 2 != 0 { Some(EnemyDyn::read(input)?) } else { None };
            let known = self.enemy_at.get(&id).copied();
            let i = match (known, blob) {
                (Some(i), Some(enemy)) => {
                    if enemy.id != id {
                        return None;
                    }
                    sim.enemies[i] = enemy;
                    i
                }
                (Some(i), None) => i,
                (None, Some(enemy)) => {
                    if enemy.id != id {
                        return None;
                    }
                    let at = enemy.position;
                    sim.enemies.push(enemy);
                    self.enemy_tracks.push(Track::at(at, t));
                    self.enemy_at.insert(id, sim.enemies.len() - 1);
                    sim.enemies.len() - 1
                }
                (None, None) => return None,
            };
            if let Some(seen) = seen {
                seen.apply(&mut sim.enemies[i]);
                if known.is_some() {
                    self.enemy_tracks[i].told(sim.enemies[i].position, t);
                } else {
                    self.enemy_tracks[i] = Track::at(sim.enemies[i].position, t);
                }
            }
        }
        Some(())
    }

    fn read_babies(&mut self, sim: &mut Sim, input: &mut Reader) -> Option<()> {
        let count = input.varint()? as usize;
        if count > MAX_RECORDS {
            return None;
        }
        let mut gone = Vec::with_capacity(count.min(1 << 16));
        let mut key = 0u32;
        for _ in 0..count {
            key = key.checked_add(input.varint()?)?;
            gone.push(key);
        }
        let count = input.varint()? as usize;
        if count > MAX_RECORDS {
            return None;
        }
        let mut came = Vec::with_capacity(count.min(1 << 16));
        let mut key = 0u32;
        for _ in 0..count {
            key = key.checked_add(input.varint()?)?;
            came.push((key, input.u16()? as f32 / 100.0));
        }
        //(both lists and the babies are in order of key: the ones that are gone are taken out, and the ones that have come are put in their places)
        let mut babies = Vec::with_capacity(self.baby_keys.len() + came.len());
        let mut keys = Vec::with_capacity(self.baby_keys.len() + came.len());
        let (mut gone, mut came) = (gone.into_iter().peekable(), came.into_iter().peekable());
        for (&key, baby) in self.baby_keys.iter().zip(sim.babies.iter()) {
            while let Some((new, age)) = came.next_if(|&(new, _)| new < key) {
                babies.push(Baby { position: key_baby(new), age });
                keys.push(new);
            }
            while gone.next_if(|&old| old < key).is_some() {}
            if gone.next_if_eq(&key).is_none() {
                babies.push(baby.clone());
                keys.push(key);
            }
        }
        for (new, age) in came {
            babies.push(Baby { position: key_baby(new), age });
            keys.push(new);
        }
        sim.babies = babies;
        self.baby_keys = keys;
        Some(())
    }

    //the avatar is where the player has taken it, and what it has been thought to do is remembered against the number of the input that asked for it,
    //so that it can be put right when the host says where it was after that one
    pub(crate) fn walked(&mut self, sim: &Sim, direction: Vec2, dt: f32) {
        self.predicted = walk_player(self.predicted, direction, PLAYER_SPEED * self.boost, dt, &sim.walls);
    }

    //the input with this number has been sent, with the avatar where it is now
    pub(crate) fn sent(&mut self, seq: u32) {
        self.history.push_back((seq, self.predicted));
        while self.history.len() > 600 {
            self.history.pop_front();
        }
    }

    //everything that moves, put where it is at a time (in the host's clock, from the first message)
    pub(crate) fn place_at(&self, sim: &mut Sim, t: f32, dt: f32) {
        for (unit, track) in sim.units.iter_mut().zip(&self.unit_tracks) {
            let at = track.place(t);
            unit.x = at.x;
            unit.y = at.y;
            unit.velocity = track.velocity(t);
            unit.nose = turned_nose(unit.nose, unit.velocity, dt);
            unit.age += dt;
        }
        for (enemy, track) in sim.enemies.iter_mut().zip(&self.enemy_tracks) {
            enemy.position = track.place(t);
            enemy.flash = (enemy.flash + dt).min(10.0);
        }
        for (i, track) in self.player_tracks.iter().enumerate() {
            if let (Some(track), Some(player)) = (track, sim.players.list.get_mut(i)) {
                player.position = if i == self.me { self.predicted } else { track.place(t) };
            }
        }
    }

    //the latest of the host's clock
    #[cfg(test)]
    pub(crate) fn latest(&self) -> f32 {
        self.latest
    }

    //a frame later: time goes on, and everything is shown where it is on its way to being. `dt` is how long the frame was, `stands_still` for a world that
    //the host has stopped
    pub(crate) fn advance(&mut self, sim: &mut Sim, dt: f32) {
        if !self.ready {
            return;
        }
        //(what is shown follows the host's clock, kept from running away from it)
        self.shown += dt;
        let target = self.latest - INTERP_DELAY;
        let error = target - self.shown;
        self.shown = if error.abs() > 0.6 { target } else { self.shown + error * 0.05 };
        self.shown = self.shown.min(self.latest);
        self.place_at(sim, self.shown, dt);
        //what is on its way carries on, until the host says where it is
        for shot in sim.shots.iter_mut() {
            shot.position += shot.velocity * dt;
            shot.age += dt;
        }
        let fronts = fronts_of(&sim.players, &sim.players);
        fire_vanguard(&fronts, &sim.units, &sim.enemies, sim.run.seconds, dt, bolt_density(), false, &mut sim.bolts, None);
        fly_bolts(&mut sim.bolts, &mut sim.enemies, dt, None);
        for enemy in sim.enemies.iter_mut() {
            enemy.struck = (enemy.struck - dt).max(0.0);
        }
        for hazard in sim.hazards.iter_mut() {
            if hazard.armed {
                hazard.warn = (hazard.warn - dt).max(0.0);
            } else {
                hazard.flash = (hazard.flash - dt).max(0.0);
            }
        }
        for pull in sim.pulls.iter_mut() {
            pull.left = (pull.left - dt).max(0.0);
        }
        for chest in sim.hordes.chests.iter_mut() {
            chest.left = (chest.left - dt).max(0.0);
        }
        for baby in sim.babies.iter_mut() {
            baby.age += dt;
        }
        for site in sim.sites.iter_mut() {
            site.clock = (site.clock - dt).max(0.0);
        }
        sim.run.seconds += dt;
        sim.director.next_boss_in = (sim.director.next_boss_in - dt).max(0.0);
        sim.hordes.next_in = (sim.hordes.next_in - dt).max(0.0);
        if let Some(coming) = sim.hordes.coming.as_mut() {
            coming.left = (coming.left - dt).max(0.0);
        }
        if let Some(choice) = sim.players.list.get_mut(self.me).and_then(|me| me.choice.as_mut()) {
            choice.age += dt;
        }
        //(the player's squads are the enemies their units were sent at)
        let me = self.me.min(sim.players.len().saturating_sub(1));
        let sent: HashSet<u32> = sim.units.iter().filter(|unit| unit.owner as usize == me).filter_map(|unit| unit.target).collect();
        let squads = &mut sim.players.list[me].squads;
        squads.retain(|id, _| sent.contains(id));
        for id in sent {
            squads.entry(id).or_insert_with(|| Squad { position: Vec2::ZERO, field: FlowField::new(), next_field: FlowField::new() });
        }
    }
}
