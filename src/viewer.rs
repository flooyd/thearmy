//! the host's side of showing the world to a player: what that player has been told is kept (`Viewer`), and each time they are due another message
//! (a `tick`) it holds what they have not been told yet, and nothing else: units, enemies, babies and walls that have come into their view or changed,
//! and the ones that have gone out of it. a message is for the player's own part of the world, so what is far from them costs nothing. they arrive in
//! order and none is lost (the connection sees to that), so what a message says is only ever said against what the one before it said

use crate::prelude::*;
use std::collections::hash_map::Entry;
use std::ops::Range;

pub(crate) const DOWN_TICK: u8 = 0x54;

//the walls of the world cut into chunks, with what is needed to tell whether one has changed: made once, and looked at by every viewer
pub(crate) struct WallIndex {
    pub(crate) chunks: HashMap<(i32, i32), ChunkWalls>,
}

pub(crate) struct ChunkWalls {
    pub(crate) hard: u64, //of what is built where, which only changes when something is built or broken
    pub(crate) soft: u64, //of all of it, which changes as walls are hurt
    pub(crate) walls: Vec<WallRec>,
}

//the same for two walls that are alike and different for ones that are not, and added up in any order, so the order the walls were found in does not matter
fn mix(mut value: u64) -> u64 {
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51_afd7_ed55_8ccd);
    value ^= value >> 33;
    value = value.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    value ^ (value >> 33)
}

impl WallIndex {
    pub(crate) fn build(walls: &Walls) -> WallIndex {
        let mut chunks: HashMap<(i32, i32), ChunkWalls> = HashMap::new();
        for wall in walls.values() {
            let rec = WallRec::of(wall);
            let chunk = chunks.entry(chunk_of(wall.col, wall.row)).or_insert_with(|| ChunkWalls { hard: 0, soft: 0, walls: Vec::new() });
            let hard = mix(((rec.col as u32 as u64) << 32 | rec.row as u32 as u64) ^ ((rec.kind as u64) << 56) ^ ((rec.hostile as u64) << 62));
            let soft = mix(hard ^ ((rec.share as u64) << 8 | (rec.hurt as u64) << 16 | (rec.garrison as u64) << 24 | (rec.life as u64) << 32));
            chunk.hard = chunk.hard.wrapping_add(hard);
            chunk.soft = chunk.soft.wrapping_add(soft);
            chunk.walls.push(rec);
        }
        WallIndex { chunks }
    }
}

//what is the same for every viewer in a round: the world, where the armies are in it, and what the last step reported
pub(crate) struct Frame<'a> {
    pub(crate) sim: &'a Sim,
    pub(crate) ranges: &'a [Range<usize>],
    pub(crate) walls: &'a WallIndex,
    pub(crate) time: f32,
    pub(crate) boosts: &'a [f32],
    pub(crate) pickups: &'a [f32],
}

#[derive(Clone, Copy)]
struct UnitShadow {
    id: u32,
    x: u16,
    y: u16,
    look: UnitLook,
}

struct EnemyShadow {
    print: u64,
    seen: EnemyDyn,
    round: u32, //the last tick it was in, so the ones that were not in this one are found
}

struct WallShadow {
    hard: u64,
    soft: u64,
    at: f32,
}

pub(crate) struct Viewer {
    pub(crate) player: usize,
    pub(crate) screen: Vec2, //their window and how far out they have zoomed, which is what decides how much of the world they are sent
    pub(crate) zoom: f32,
    seq: u32,
    units: Vec<Vec<UnitShadow>>,
    enemies: HashMap<u32, EnemyShadow>,
    babies: Vec<u32>,
    walls: HashMap<(i32, i32), WallShadow>,
    sites: Option<HashMap<u32, (u8, Option<u32>)>>,
    names: Option<u64>,
    relics: Option<u64>,
    choice: Option<u64>,
    slow_at: Option<f32>,
    pending: Vec<Event>,
}

fn print_of(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

impl Viewer {
    pub(crate) fn new(player: usize) -> Viewer {
        Viewer {
            player,
            screen: DEFAULT_SCREEN,
            zoom: 1.0,
            seq: 0,
            units: vec![Vec::new(); MAX_PLAYERS],
            enemies: HashMap::new(),
            babies: Vec::new(),
            walls: HashMap::new(),
            sites: None,
            names: None,
            relics: None,
            choice: None,
            slow_at: None,
            pending: Vec::new(),
        }
    }

    //what happened in the world, which is held until the next tick
    pub(crate) fn hear(&mut self, events: &[Event]) {
        self.pending.extend_from_slice(events);
        //(a player who is not being sent anything, as they are too far behind, is not owed every puff and number there has been)
        if self.pending.len() > 4 * MAX_EVENTS_A_MESSAGE {
            let spare = self.pending.len() - 2 * MAX_EVENTS_A_MESSAGE;
            self.pending.drain(..spare);
        }
    }

    //how many messages there have been
    pub(crate) fn ticks(&self) -> u32 {
        self.seq
    }

    //the next message: everything this player has not been told, as of now. `ack` is the last of their inputs that was acted on, so they can tell how far
    //ahead of the host they are
    pub(crate) fn tick(&mut self, frame: &Frame, ack: u32) -> Vec<u8> {
        let sim = frame.sim;
        let first = self.seq == 0;
        let me = &sim.players.list[self.player];
        let interest = interest_of(me.position, self.screen, self.zoom);
        let stay = grown(interest, INTEREST_STAY);
        let mut out = Writer::new();
        out.u8(DOWN_TICK);
        out.u32(self.seq);
        out.u32((frame.time * 1000.0) as u32);
        out.u32(ack);
        out.u8(self.player as u8);
        out.flag(first);
        self.seq += 1;

        self.write_players(&mut out, frame);
        self.write_mine(&mut out, frame);
        self.write_slow(&mut out, frame);
        self.write_sites(&mut out, sim);
        self.write_walls(&mut out, frame, interest, stay);
        for owner in 0..sim.players.len() {
            self.write_units(&mut out, frame, owner, interest, stay);
        }
        self.write_extras(&mut out, frame);
        self.write_enemies(&mut out, sim, interest, stay);
        self.write_babies(&mut out, sim, interest, stay);
        let seen = interest;
        let shots: Vec<&Shot> = sim.shots.iter().filter(|shot| seen.contains(shot.position)).take(MAX_RECORDS).collect();
        out.varint(shots.len() as u32);
        for shot in shots {
            write_shot(&mut out, shot);
        }
        let hazards: Vec<&Hazard> = sim.hazards.iter().filter(|hazard| seen.contains(hazard.position) || hazard.position.distance(me.position) < hazard.radius + 2.0 * TILE_SIZE).take(MAX_RECORDS).collect();
        out.varint(hazards.len() as u32);
        for hazard in hazards {
            write_hazard(&mut out, hazard);
        }
        let pulls: Vec<&Pull> = sim.pulls.iter().filter(|pull| seen.contains(pull.position)).collect();
        out.varint(pulls.len() as u32);
        for pull in pulls {
            write_pull(&mut out, pull);
        }
        out.varint(sim.hordes.chests.len() as u32);
        for chest in &sim.hordes.chests {
            write_chest(&mut out, chest);
        }
        //(what happened that they could see or hear)
        let heard = std::mem::take(&mut self.pending);
        let events: Vec<Event> = heard.into_iter().filter(|event| event.place().is_none_or(|at| interest.contains(at))).take(MAX_EVENTS_A_MESSAGE).collect();
        out.raw(&write_events(&events));
        out.bytes
    }

    //where everyone is, and how many are in each army
    fn write_players(&mut self, out: &mut Writer, frame: &Frame) {
        let sim = frame.sim;
        out.u8(sim.players.len() as u8);
        for (i, player) in sim.players.list.iter().enumerate() {
            out.vec2(player.position);
            out.vec2(player.velocity);
            out.vec2(player.facing);
            out.u8(formation_byte(player.formation));
            out.u32(frame.ranges[i].len() as u32);
            out.u8(player.choice.is_some() as u8);
        }
        let names: Vec<&str> = sim.players.list.iter().map(|player| player.name.as_str()).collect();
        let print = print_of(&names.join("\n"));
        if self.names != Some(print) {
            self.names = Some(print);
            out.flag(true);
            for name in names {
                out.text(name);
            }
        } else {
            out.flag(false);
        }
    }

    //what is the player's own: what their army reaches, their relics and the wall points, and the relics on offer to them
    fn write_mine(&mut self, out: &mut Writer, frame: &Frame) {
        let me = &frame.sim.players.list[self.player];
        out.f32(frame.boosts.get(self.player).copied().unwrap_or(1.0));
        out.f32(frame.pickups.get(self.player).copied().unwrap_or(BABY_PICKUP_RANGE));
        out.u8(wall_kind_byte(me.mason.selected));
        out.f32(me.mason.points);
        out.u32(me.rerolls);
        out.u32(me.upgrades.army_limit as u32);
        let relics = text_of(&me.upgrades);
        let print = print_of(&relics);
        if self.relics != Some(print) {
            self.relics = Some(print);
            out.flag(true);
            out.blob(relics.as_bytes());
        } else {
            out.flag(false);
        }
        match &me.choice {
            None => {
                self.choice = None;
                out.u8(0);
            }
            Some(choice) => {
                let text = text_of(choice);
                let print = print_of(&text);
                if self.choice == Some(print) {
                    out.u8(1);
                } else {
                    self.choice = Some(print);
                    out.u8(2);
                    out.blob(text.as_bytes());
                }
            }
        }
    }

    //the run, the director and the hordes, which only need saying now and then
    fn write_slow(&mut self, out: &mut Writer, frame: &Frame) {
        let sim = frame.sim;
        if self.slow_at.is_some_and(|at| frame.time - at < SLOW_SECONDS) {
            out.flag(false);
            return;
        }
        self.slow_at = Some(frame.time);
        out.flag(true);
        out.blob(text_of(&sim.run).as_bytes());
        out.blob(text_of(&sim.director).as_bytes());
        //(the chests are said every time, as they change every time)
        let hordes = Hordes { chests: Vec::new(), ..sim.hordes.clone() };
        out.blob(text_of(&hordes).as_bytes());
    }

    //the sites, in full the first time and after that only the ones that have changed
    fn write_sites(&mut self, out: &mut Writer, sim: &Sim) {
        let now: HashMap<u32, (u8, Option<u32>)> = sim.sites.iter().map(|site| (site.id, (site_state_byte(site.state), site.core))).collect();
        match &self.sites {
            None => {
                let bare: Vec<Site> = sim.sites.iter().map(|site| Site { tiles: Vec::new(), ..site.clone() }).collect();
                out.u8(1);
                out.blob(text_of(&bare).as_bytes());
            }
            Some(before) => {
                let changed: Vec<(&u32, &(u8, Option<u32>))> = now.iter().filter(|(id, state)| before.get(id) != Some(state)).collect();
                if changed.is_empty() {
                    out.u8(0);
                } else {
                    out.u8(2);
                    out.varint(changed.len() as u32);
                    for (id, (state, core)) in changed {
                        out.u32(*id);
                        out.u8(*state);
                        out.flag(core.is_some());
                        out.u32(core.unwrap_or(0));
                        out.f32(sim.sites.iter().find(|site| site.id == *id).map_or(0.0, |site| site.clock));
                    }
                }
            }
        }
        self.sites = Some(now);
    }

    //the chunks of walls near the player that they do not have as they are now
    fn write_walls(&mut self, out: &mut Writer, frame: &Frame, interest: Rect, stay: Rect) {
        let chunk_span = |rect: Rect| {
            let (first, last) = (chunk_of(tile_of(rect.x, rect.y).0, tile_of(rect.x, rect.y).1), chunk_of(tile_of(rect.right(), rect.bottom()).0, tile_of(rect.right(), rect.bottom()).1));
            (first.0.max(0)..=last.0.min(WALL_CHUNKS - 1), first.1.max(0)..=last.1.min(WALL_CHUNKS - 1))
        };
        let (columns, rows) = chunk_span(interest);
        let mut body = Writer::new();
        let mut count = 0u32;
        for chunk_row in rows {
            for chunk_col in columns.clone() {
                let chunk = (chunk_col, chunk_row);
                let here = frame.walls.chunks.get(&chunk);
                let (hard, soft) = here.map_or((0, 0), |here| (here.hard, here.soft));
                let send = match self.walls.get(&chunk) {
                    None => here.is_some(),
                    Some(sent) => sent.hard != hard || (sent.soft != soft && frame.time - sent.at >= WALL_REFRESH_SECONDS),
                };
                if !send {
                    continue;
                }
                count += 1;
                body.u8(chunk_col as u8);
                body.u8(chunk_row as u8);
                let walls = here.map_or(&[][..], |here| &here.walls[..]);
                body.varint(walls.len() as u32);
                for wall in walls {
                    wall.write(&mut body);
                }
                self.walls.insert(chunk, WallShadow { hard, soft, at: frame.time });
            }
        }
        out.varint(count);
        out.raw(&body.bytes);
        //(a chunk the player has gone far from is forgotten, and told again when they come back, as it may have changed in the meantime)
        let (keep_columns, keep_rows) = chunk_span(stay);
        let forgotten: Vec<(i32, i32)> = self.walls.keys().copied().filter(|&(col, row)| !(keep_columns.contains(&col) && keep_rows.contains(&row))).collect();
        out.varint(forgotten.len() as u32);
        for chunk in forgotten {
            self.walls.remove(&chunk);
            out.u8(chunk.0 as u8);
            out.u8(chunk.1 as u8);
        }
    }

    //one army's units: the ones that are new or have changed, and the ones that are gone. a player has all of their own, and of the others' what is in view
    fn write_units(&mut self, out: &mut Writer, frame: &Frame, owner: usize, interest: Rect, stay: Rect) {
        let units = &frame.sim.units[frame.ranges[owner].clone()];
        let own = owner == self.player;
        let old = std::mem::take(&mut self.units[owner]);
        let mut now: Vec<UnitShadow> = Vec::with_capacity(old.len());
        let mut removed: Vec<u32> = Vec::new();
        let mut entries = Writer::new();
        let mut changed = 0u32;
        let mut last = 0u32;
        //(in order of id, which the list is in unless something has put it out of order, which is put right)
        let mut in_order: Vec<&Unit> = units.iter().collect();
        if !units.windows(2).all(|pair| pair[0].id < pair[1].id) {
            in_order.sort_by_key(|unit| unit.id);
            in_order.dedup_by_key(|unit| unit.id);
        }
        let mut s = 0;
        for unit in in_order {
            while s < old.len() && old[s].id < unit.id {
                removed.push(old[s].id);
                s += 1;
            }
            let before = if s < old.len() && old[s].id == unit.id {
                s += 1;
                Some(old[s - 1])
            } else {
                None
            };
            let at = Vec2::new(unit.x, unit.y);
            let show = own || if before.is_some() { stay.contains(at) } else { interest.contains(at) };
            if !show {
                if let Some(before) = before {
                    removed.push(before.id);
                }
                continue;
            }
            let shadow = UnitShadow { id: unit.id, x: pack(unit.x), y: pack(unit.y), look: UnitLook::of(unit, own) };
            let flags = match before {
                None => 3,
                Some(before) => (before.x != shadow.x || before.y != shadow.y) as u8 | ((before.look != shadow.look) as u8) << 1,
            };
            if flags != 0 {
                //(a unit that has only moved a little is said to have moved by that much, which is half the bytes, and the flags go in with how far on in the list it is)
                let step = before.map(|before| (shadow.x as i32 - before.x as i32, shadow.y as i32 - before.y as i32)).filter(|&(dx, dy)| flags & 1 != 0 && dx.abs() <= 127 && dy.abs() <= 127);
                entries.varint(((unit.id - last) << 3) | flags as u32 | if step.is_some() { 4 } else { 0 });
                last = unit.id;
                if flags & 1 != 0 {
                    match step {
                        Some((dx, dy)) => {
                            entries.u8(dx as i8 as u8);
                            entries.u8(dy as i8 as u8);
                        }
                        None => {
                            entries.u16(shadow.x);
                            entries.u16(shadow.y);
                        }
                    }
                }
                if flags & 2 != 0 {
                    shadow.look.write(&mut entries);
                }
                changed += 1;
            }
            now.push(shadow);
        }
        removed.extend(old[s..].iter().map(|shadow| shadow.id));
        out.varint(removed.len() as u32);
        let mut before = 0;
        for id in removed {
            out.varint(id - before);
            before = id;
        }
        out.varint(changed);
        out.raw(&entries.bytes);
        self.units[owner] = now;
    }

    //what the player's champions are doing that a look does not say
    fn write_extras(&mut self, out: &mut Writer, frame: &Frame) {
        let units = &frame.sim.units[frame.ranges[self.player].clone()];
        let champions: Vec<ChampionExtra> = units.iter().filter(|unit| unit.champion != 0).map(ChampionExtra::of).collect();
        out.varint(champions.len() as u32);
        for champion in champions {
            champion.write(out);
        }
    }

    //the enemies that have come into view or changed, and the ones that are gone. what an enemy is is sent when it comes into view (and again if it
    //changes, which is rare), and where it is and how it is when anything about it has changed. a boss is sent whole every time, as so much of it changes
    fn write_enemies(&mut self, out: &mut Writer, sim: &Sim, interest: Rect, stay: Rect) {
        let round = self.seq;
        let mut entries = Writer::new();
        let mut changed = 0u32;
        for enemy in &sim.enemies {
            let known = self.enemies.contains_key(&enemy.id);
            let show = always_seen(enemy) || if known { stay.contains(enemy.position) } else { interest.contains(enemy.position) };
            if !show {
                continue;
            }
            let (print, seen) = (enemy_print(enemy), EnemyDyn::of(enemy));
            let (blob, dynamic) = match self.enemies.entry(enemy.id) {
                Entry::Vacant(spot) => {
                    spot.insert(EnemyShadow { print, seen, round });
                    (true, true)
                }
                Entry::Occupied(mut kept) => {
                    let kept = kept.get_mut();
                    kept.round = round;
                    let blob = kept.print != print || enemy.boss.is_some();
                    let dynamic = blob || kept.seen != seen;
                    kept.print = print;
                    kept.seen = seen;
                    (blob, dynamic)
                }
            };
            if !blob && !dynamic {
                continue;
            }
            changed += 1;
            entries.varint(enemy.id);
            entries.u8(blob as u8 | (dynamic as u8) << 1);
            if blob {
                entries.blob(&enemy_blob(enemy));
            }
            if dynamic {
                seen.write(&mut entries);
            }
        }
        let gone: Vec<u32> = self.enemies.iter().filter(|(_, shadow)| shadow.round != round).map(|(&id, _)| id).collect();
        self.enemies.retain(|_, shadow| shadow.round == round);
        out.varint(gone.len() as u32);
        for id in gone {
            out.varint(id);
        }
        out.varint(changed);
        out.raw(&entries.bytes);
    }

    //the babies, which only come and go: each is known by where it is
    fn write_babies(&mut self, out: &mut Writer, sim: &Sim, interest: Rect, stay: Rect) {
        let mut now: Vec<(u32, f32)> = sim
            .babies
            .iter()
            .filter(|baby| {
                let key = baby_key(baby.position);
                if self.babies.binary_search(&key).is_ok() { stay.contains(baby.position) } else { interest.contains(baby.position) }
            })
            .map(|baby| (baby_key(baby.position), baby.age))
            .collect();
        now.sort_by_key(|&(key, _)| key);
        now.dedup_by_key(|&mut (key, _)| key);
        let before = std::mem::take(&mut self.babies);
        let mut gone = Vec::new();
        let mut came = Writer::new();
        let mut arrivals = 0u32;
        let (mut last_came, mut s) = (0u32, 0usize);
        for &(key, age) in &now {
            while s < before.len() && before[s] < key {
                gone.push(before[s]);
                s += 1;
            }
            if s < before.len() && before[s] == key {
                s += 1;
                continue;
            }
            came.varint(key - last_came);
            last_came = key;
            came.u16((age * 100.0).min(65535.0) as u16);
            arrivals += 1;
        }
        gone.extend_from_slice(&before[s..]);
        out.varint(gone.len() as u32);
        let mut last_gone = 0u32;
        for key in gone {
            out.varint(key - last_gone);
            last_gone = key;
        }
        out.varint(arrivals);
        out.raw(&came.bytes);
        self.babies = now.into_iter().map(|(key, _)| key).collect();
    }
}

//a baby is known by its place, to the pixel
pub(crate) fn baby_key(at: Vec2) -> u32 {
    (pack(at.x) as u32) << 16 | pack(at.y) as u32
}

pub(crate) fn key_baby(key: u32) -> Vec2 {
    Vec2::new(unpack((key >> 16) as u16), unpack(key as u16))
}
