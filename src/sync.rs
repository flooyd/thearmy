//! what the host tells a player who is watching the world it runs, and what they tell it back, as far as both of them have to agree on it: the sizes
//! and rates, how numbers are packed, the small records that things are sent as, and the part of the world a player is sent. the host's side of it is
//! in `viewer`, the player's in `replica`. everything is plain bytes (see `wire`) and every read is checked

use crate::prelude::*;

//the world is sent this often, in seconds, and what is on screen is shown this far behind the latest of it, so that there is always a later one to
//move towards (a little more than the gap between two, so one that is late does not stop everything)
pub(crate) const SNAPSHOT_SECONDS: f32 = 0.05;
pub(crate) const INTERP_DELAY: f32 = 0.12;
//the most a message from the host and from a player can be, which are the longest messages there are. the first the host sends a player is the whole
//of what they can see, which is the biggest
pub(crate) const MAX_DOWN_BYTES: usize = 32 << 20;
pub(crate) const MAX_UP_BYTES: usize = 64 << 10;
//how far past what a player can see they are sent things (so they are there before they come into view), and how much further it has to be before it
//is taken away again, so that something on the edge is not sent and taken away every other time
pub(crate) const INTEREST_MARGIN: f32 = TILE_SIZE * 10.0;
pub(crate) const INTEREST_STAY: f32 = 1.2;
//what the host has to say that is slow to change is said this often, in seconds, if it has changed
pub(crate) const SLOW_SECONDS: f32 = 0.5;
//a chunk of walls that has only been hurt, not built or broken, is not sent again sooner than this
pub(crate) const WALL_REFRESH_SECONDS: f32 = 0.3;
//the walls are sent in chunks of this many squares along a side
pub(crate) const WALL_CHUNK: i32 = 16;
pub(crate) const WALL_CHUNKS: i32 = (WORLD_TILES + WALL_CHUNK - 1) / WALL_CHUNK;

//the most things of a kind that one message can say it has, so a message that says it has more is not believed
pub(crate) const MAX_RECORDS: usize = 4_000_000;
pub(crate) const MAX_TEXT_BLOB: usize = 4 << 20;

// ---------- numbers ----------

//a place in the world in two bytes a coordinate: a pixel is as fine as it gets, which is finer than anything is drawn
const PACK_SCALE: f32 = 65535.0 / WORLD_SIZE;

pub(crate) fn pack(value: f32) -> u16 {
    (value * PACK_SCALE).clamp(0.0, 65535.0).round() as u16
}

pub(crate) fn unpack(packed: u16) -> f32 {
    packed as f32 / PACK_SCALE
}

//a share, from 0 to 1, in a byte or two
pub(crate) fn share_byte(share: f32) -> u8 {
    (share.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub(crate) fn byte_share(byte: u8) -> f32 {
    byte as f32 / 255.0
}

pub(crate) fn share_word(share: f32) -> u16 {
    (share.clamp(0.0, 1.0) * 65535.0).round() as u16
}

pub(crate) fn word_share(word: u16) -> f32 {
    word as f32 / 65535.0
}

//a direction as a byte, a turn round in 256ths
pub(crate) fn angle_byte(direction: Vec2) -> u8 {
    ((direction.to_angle().rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU) * 256.0) as u32 as u8
}

pub(crate) fn byte_angle(byte: u8) -> Vec2 {
    Vec2::from_angle(byte as f32 / 256.0 * std::f32::consts::TAU)
}

//a few seconds, to the hundredth, in a byte (and no more than 2.55 of them)
pub(crate) fn seconds_byte(seconds: f32) -> u8 {
    (seconds.clamp(0.0, 2.55) * 100.0).round() as u8
}

pub(crate) fn byte_seconds(byte: u8) -> f32 {
    byte as f32 / 100.0
}

// ---------- text ----------

//something that can be saved, as the words it is saved as: what is sent when it is rare, or when there is too little structure in it to be worth a
//format of its own
pub(crate) fn text_of<T: Stored>(item: &T) -> String {
    let mut writer = SaveWriter::new();
    writer.put(item);
    writer.finish()
}

pub(crate) fn from_text<T: Stored>(bytes: &[u8]) -> Option<T> {
    let text = std::str::from_utf8(bytes).ok()?;
    SaveReader::new(text).get().ok()
}

// ---------- what a player can see ----------

//the part of the world a player is sent: what their screen shows at the most it could (zoomed out as far as they say they are), a margin all round it, and
//at least what the minimap shows
pub(crate) fn interest_of(at: Vec2, screen: Vec2, zoom: f32) -> Rect {
    let seen = screen / zoom.clamp(MIN_ZOOM, MAX_ZOOM) / 2.0;
    let half = seen.max(Vec2::splat(MINIMAP_NEAR_TILES * TILE_SIZE)) + Vec2::splat(INTEREST_MARGIN);
    Rect::new(at.x - half.x, at.y - half.y, half.x * 2.0, half.y * 2.0)
}

//a rectangle made bigger all round by a share of its size
pub(crate) fn grown(rect: Rect, factor: f32) -> Rect {
    let (w, h) = (rect.w * factor, rect.h * factor);
    Rect::new(rect.x - (w - rect.w) / 2.0, rect.y - (h - rect.h) / 2.0, w, h)
}

// ---------- units ----------

//what is seen of a unit besides where it is
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(crate) struct UnitLook {
    pub(crate) health: u8, //as a share of the most it could have
    pub(crate) bits: u8,   //see the LOOK_ numbers
    pub(crate) champion: u8,
    pub(crate) level: u8,
    pub(crate) evolutions: u8, //the ways a champion has grown (see `chosen_at`)
    pub(crate) age: u8,       //how long since it grew from a baby, in 255ths of UNIT_GROW_SECONDS, while it is that new
    pub(crate) target: u32,   //the enemy it was sent at, but only what a player is told of their own units
}

const LOOK_GUARD: u8 = 1;
const LOOK_SENT: u8 = 2;
const LOOK_GARRISON: u8 = 4;
const LOOK_YOUNG: u8 = 8;
const LOOK_CHAMPION: u8 = 16;
const LOOK_TARGET: u8 = 32;

impl UnitLook {
    pub(crate) fn of(unit: &Unit, own: bool) -> UnitLook {
        let mut bits = 0;
        if unit.guard > 0.0 {
            bits |= LOOK_GUARD;
        }
        if unit.target.is_some() {
            bits |= LOOK_SENT;
        }
        if unit.garrison.is_some() {
            bits |= LOOK_GARRISON;
        }
        let young = unit.age < UNIT_GROW_SECONDS;
        if young {
            bits |= LOOK_YOUNG;
        }
        if unit.champion != 0 {
            bits |= LOOK_CHAMPION;
        }
        let target = if own { unit.target.unwrap_or(0) } else { 0 };
        if own && unit.target.is_some() {
            bits |= LOOK_TARGET;
        }
        UnitLook {
            health: share_byte(unit.health / unit.max_health()),
            bits,
            champion: if unit.champion != 0 { unit.champion } else { 0 },
            level: if unit.champion != 0 { unit.level } else { 0 },
            evolutions: if unit.champion != 0 { unit.evolutions } else { 0 },
            age: if young { share_byte(unit.age / UNIT_GROW_SECONDS) } else { 0 },
            target,
        }
    }

    pub(crate) fn write(&self, out: &mut Writer) {
        out.u8(self.health);
        out.u8(self.bits);
        if self.bits & LOOK_CHAMPION != 0 {
            out.u8(self.champion);
            out.u8(self.level);
            out.u8(self.evolutions);
        }
        if self.bits & LOOK_YOUNG != 0 {
            out.u8(self.age);
        }
        if self.bits & LOOK_TARGET != 0 {
            out.varint(self.target);
        }
    }

    pub(crate) fn read(input: &mut Reader) -> Option<UnitLook> {
        let (health, bits) = (input.u8()?, input.u8()?);
        if bits & 0b1100_0000 != 0 {
            return None;
        }
        let (mut champion, mut level, mut evolutions, mut age, mut target) = (0, 0, 0, 0, 0);
        if bits & LOOK_CHAMPION != 0 {
            champion = input.u8()?;
            level = input.u8()?;
            evolutions = input.u8()?;
            if champion == 0 || champion as usize > CHAMPIONS.len() {
                return None;
            }
        }
        if bits & LOOK_YOUNG != 0 {
            age = input.u8()?;
        }
        if bits & LOOK_TARGET != 0 {
            target = input.varint()?;
        }
        let level = level.clamp(if champion != 0 { 1 } else { 0 }, CHAMPION_MAX_LEVEL);
        Some(UnitLook { health, bits, champion, level, evolutions: tidy_evolutions(evolutions, level), age, target })
    }

    //put what is seen onto a unit
    pub(crate) fn apply(&self, unit: &mut Unit) {
        unit.guard = if self.bits & LOOK_GUARD != 0 { 1.0 } else { 0.0 };
        unit.target = (self.bits & LOOK_SENT != 0).then_some(if self.bits & LOOK_TARGET != 0 { self.target } else { u32::MAX });
        unit.garrison = (self.bits & LOOK_GARRISON != 0).then_some((0, 0));
        unit.champion = self.champion;
        unit.level = self.level;
        unit.evolutions = self.evolutions;
        unit.age = if self.bits & LOOK_YOUNG != 0 { byte_share(self.age) * UNIT_GROW_SECONDS } else { 99.0 };
        unit.health = byte_share(self.health) * unit.max_health();
    }
}

//what a player is told of each of their own champions that is not in the look: their power's timer, their experience, and how much of the damage reaches them
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct ChampionExtra {
    pub(crate) id: u32,
    pub(crate) power: f32,
    pub(crate) xp: f32,
    pub(crate) ward: u8,
}

impl ChampionExtra {
    pub(crate) fn of(unit: &Unit) -> ChampionExtra {
        ChampionExtra { id: unit.id, power: unit.power, xp: unit.xp, ward: share_byte(unit.ward) }
    }

    pub(crate) fn write(&self, out: &mut Writer) {
        out.u32(self.id);
        out.f32(self.power);
        out.f32(self.xp);
        out.u8(self.ward);
    }

    pub(crate) fn read(input: &mut Reader) -> Option<ChampionExtra> {
        Some(ChampionExtra { id: input.u32()?, power: input.f32()?, xp: input.f32()?, ward: input.u8()? })
    }
}

// ---------- enemies ----------

//what an enemy does that is seen as it goes on: where it is, how hurt, and the few things about it that come and go. what it is (its kind, size, strength)
//is sent once, when it comes into view, as the words it would be saved with
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct EnemyDyn {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) health: u16,
    pub(crate) shield: u16,
    pub(crate) flags: u16,
    pub(crate) flash: u8,
    pub(crate) mode: u8,   //which of the modes (see MODE_)
    pub(crate) toward: u8, //the way a wind up or a dash is going
    pub(crate) left: u8,   //how long of it there is left
}

const FLAG_CHASING: u16 = 1;
const FLAG_FROZEN: u16 = 2;
const FLAG_ROUTED: u16 = 4;
const FLAG_RALLIED: u16 = 8;
const FLAG_PROVOKED: u16 = 16;
const FLAG_BLEW_UP: u16 = 32;
const FLAG_CHILLED: u16 = 64;
const FLAG_BURNING: u16 = 128;
const FLAG_SHOCKED: u16 = 256;
const FLAG_POISONED: u16 = 512;
const FLAG_SLOWED: u16 = 1024;
const FLAG_ALL: u16 = 2047;

const MODE_ROAM: u8 = 0;
const MODE_WINDUP: u8 = 1;
const MODE_DASH: u8 = 2;
const MODE_RECOVER: u8 = 3;
const MODE_FUSE: u8 = 4;
const MODE_CAST: u8 = 5;
const MODE_LURK: u8 = 6;

impl EnemyDyn {
    pub(crate) fn of(enemy: &Enemy) -> EnemyDyn {
        let mut flags = 0;
        for (on, flag) in [(enemy.chasing, FLAG_CHASING), (enemy.status.frozen > 0.0, FLAG_FROZEN), (enemy.routed(), FLAG_ROUTED), (enemy.rallied, FLAG_RALLIED), (enemy.provoked > 0.0, FLAG_PROVOKED),
            (enemy.blew_up, FLAG_BLEW_UP),
            (enemy.status.chill >= CHILL_FULL * CHILL_SEEN && enemy.status.frozen <= 0.0, FLAG_CHILLED),
            (enemy.status.burning > 0.0, FLAG_BURNING),
            (enemy.status.shocked > 0.0, FLAG_SHOCKED),
            (enemy.status.poison > 0.0, FLAG_POISONED),
            (enemy.status.slow < SLOW_SEEN, FLAG_SLOWED),
        ] {
            if on {
                flags |= flag;
            }
        }
        let (mode, toward, left) = match enemy.mode {
            Mode::Roam => (MODE_ROAM, 0, 0),
            Mode::Windup { dir, left } => (MODE_WINDUP, angle_byte(dir), seconds_byte(left)),
            Mode::Dash { dir, left } => (MODE_DASH, angle_byte(dir), seconds_byte(left)),
            Mode::Recover { left } => (MODE_RECOVER, 0, seconds_byte(left)),
            Mode::Fuse { left } => (MODE_FUSE, 0, seconds_byte(left)),
            Mode::Cast { left } => (MODE_CAST, 0, seconds_byte(left)),
            Mode::Lurk => (MODE_LURK, 0, 0),
        };
        EnemyDyn {
            x: pack(enemy.position.x),
            y: pack(enemy.position.y),
            health: share_word(enemy.health / enemy.max_health.max(1.0)),
            shield: share_word(enemy.shield / enemy.max_shield.max(1.0)),
            flags,
            flash: seconds_byte(enemy.flash / 1.0),
            mode,
            toward,
            left,
        }
    }

    pub(crate) fn write(&self, out: &mut Writer) {
        out.u16(self.x);
        out.u16(self.y);
        out.u16(self.health);
        out.u16(self.shield);
        out.u16(self.flags);
        out.u8(self.flash);
        out.u8(self.mode);
        if matches!(self.mode, MODE_WINDUP | MODE_DASH) {
            out.u8(self.toward);
        }
        if matches!(self.mode, MODE_WINDUP | MODE_DASH | MODE_RECOVER | MODE_FUSE | MODE_CAST) {
            out.u8(self.left);
        }
    }

    pub(crate) fn read(input: &mut Reader) -> Option<EnemyDyn> {
        let (x, y, health, shield) = (input.u16()?, input.u16()?, input.u16()?, input.u16()?);
        let flags = input.u16()?;
        let (flash, mode) = (input.u8()?, input.u8()?);
        if flags & !FLAG_ALL != 0 || mode > MODE_LURK {
            return None;
        }
        let toward = if matches!(mode, MODE_WINDUP | MODE_DASH) { input.u8()? } else { 0 };
        let left = if matches!(mode, MODE_WINDUP | MODE_DASH | MODE_RECOVER | MODE_FUSE | MODE_CAST) { input.u8()? } else { 0 };
        Some(EnemyDyn { x, y, health, shield, flags, flash, mode, toward, left })
    }

    //put what is seen onto an enemy (which has what it is already)
    pub(crate) fn apply(&self, enemy: &mut Enemy) {
        enemy.position = Vec2::new(unpack(self.x), unpack(self.y));
        enemy.health = word_share(self.health) * enemy.max_health;
        enemy.shield = word_share(self.shield) * enemy.max_shield;
        enemy.chasing = self.flags & FLAG_CHASING != 0;
        enemy.status.frozen = if self.flags & FLAG_FROZEN != 0 { 1.0 } else { 0.0 };
        enemy.rout = if self.flags & FLAG_ROUTED != 0 { 1.0 } else { 0.0 };
        enemy.rallied = self.flags & FLAG_RALLIED != 0;
        enemy.provoked = if self.flags & FLAG_PROVOKED != 0 { RETALIATE_SECONDS } else { 0.0 };
        enemy.blew_up = self.flags & FLAG_BLEW_UP != 0;
        //(what is being done to it, as far as there is anything to see of it)
        enemy.status.chill = if self.flags & FLAG_FROZEN != 0 { CHILL_FULL } else if self.flags & FLAG_CHILLED != 0 { CHILL_FULL * 0.5 } else { 0.0 };
        enemy.status.burning = if self.flags & FLAG_BURNING != 0 { 1.0 } else { 0.0 };
        enemy.status.shocked = if self.flags & FLAG_SHOCKED != 0 { 1.0 } else { 0.0 };
        enemy.status.poison = if self.flags & FLAG_POISONED != 0 { 1.0 } else { 0.0 };
        enemy.status.slow = if self.flags & FLAG_SLOWED != 0 { TAR_SLOW } else { 1.0 };
        enemy.flash = byte_seconds(self.flash);
        let dir = byte_angle(self.toward);
        let left = byte_seconds(self.left);
        enemy.mode = match self.mode {
            MODE_WINDUP => Mode::Windup { dir, left },
            MODE_DASH => Mode::Dash { dir, left },
            MODE_RECOVER => Mode::Recover { left },
            MODE_FUSE => Mode::Fuse { left },
            MODE_CAST => Mode::Cast { left },
            MODE_LURK => Mode::Lurk,
            _ => Mode::Roam,
        };
    }
}

//what an enemy is, which does not change as it goes on (and when something about it does, that is noticed, and it is sent again): a number that
//stands for all of it, so it can be compared without keeping it
pub(crate) fn enemy_print(enemy: &Enemy) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    enemy.site.hash(&mut hasher);
    enemy.owner.hash(&mut hasher);
    (enemy.modifier as u8).hash(&mut hasher);
    enemy.affixes.hash(&mut hasher);
    enemy.size.to_bits().hash(&mut hasher);
    enemy.max_health.to_bits().hash(&mut hasher);
    enemy.max_shield.to_bits().hash(&mut hasher);
    enemy.armor.to_bits().hash(&mut hasher);
    enemy.attack.to_bits().hash(&mut hasher);
    enemy.scale.to_bits().hash(&mut hasher);
    enemy.color.r.to_bits().hash(&mut hasher);
    enemy.aggro.to_bits().hash(&mut hasher);
    hasher.finish()
}

//an enemy as it is sent when it comes into view: what it would be saved as, less the way it patrols (which no one watching needs, and which is most of it)
pub(crate) fn enemy_blob(enemy: &Enemy) -> Vec<u8> {
    let mut sent = enemy.clone();
    sent.route = vec![sent.position];
    text_of(&sent).into_bytes()
}

pub(crate) fn read_enemy_blob(bytes: &[u8]) -> Option<Enemy> {
    let mut enemy: Enemy = from_text(bytes)?;
    if enemy.route.is_empty() {
        enemy.route.push(enemy.position);
    }
    enemy.heading_for = 0;
    Some(enemy)
}

//whether an enemy is one that is always sent, wherever it is, because there are arrows and counts on the screen for it
pub(crate) fn always_seen(enemy: &Enemy) -> bool {
    enemy.boss.is_some() || enemy.is_captain() || in_a_horde(enemy)
}

// ---------- walls ----------

//a wall as it is sent: where in its chunk, what kind it is, and the few things about it that show
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct WallRec {
    pub(crate) col: i32,
    pub(crate) row: i32,
    pub(crate) kind: u8,
    pub(crate) hostile: bool,
    pub(crate) share: u8,    //of its health that is left
    pub(crate) hurt: u8,     //how long since it was hit, in 255ths of WALL_FLASH
    pub(crate) garrison: u8, //units in a tower
    pub(crate) life: u16,    //tenths of a second before it crumbles, or u16::MAX for a wall that stays
}

pub(crate) fn wall_kind_byte(kind: WallKind) -> u8 {
    ALL_WALL_KINDS.iter().position(|&other| other == kind).unwrap_or(0) as u8
}

pub(crate) const ALL_WALL_KINDS: [WallKind; 6] = [WallKind::Wall, WallKind::Gate, WallKind::Tower, WallKind::Spikes, WallKind::Tar, WallKind::Barrier];

impl WallRec {
    pub(crate) fn of(wall: &Wall) -> WallRec {
        WallRec {
            col: wall.col,
            row: wall.row,
            kind: wall_kind_byte(wall.kind),
            hostile: wall.side == Side::Hostile,
            share: share_byte(wall.share_left()),
            hurt: share_byte(wall.hurt / WALL_FLASH),
            garrison: wall.garrison.min(255) as u8,
            life: if wall.life.is_finite() { (wall.life.max(0.0) * 10.0).min(60000.0) as u16 } else { u16::MAX },
        }
    }

    pub(crate) fn write(&self, out: &mut Writer) {
        out.u8((self.col.rem_euclid(WALL_CHUNK) * WALL_CHUNK + self.row.rem_euclid(WALL_CHUNK)) as u8);
        out.u8(self.kind | if self.hostile { 0x80 } else { 0 });
        out.u8(self.share);
        out.u8(self.hurt);
        out.u8(self.garrison);
        out.u16(self.life);
    }

    //the wall in the chunk at (chunk_col, chunk_row), which is where its place is counted from
    pub(crate) fn read(input: &mut Reader, chunk: (i32, i32)) -> Option<WallRec> {
        let place = input.u8()? as i32;
        let (kind, share, hurt, garrison, life) = (input.u8()?, input.u8()?, input.u8()?, input.u8()?, input.u16()?);
        let hostile = kind & 0x80 != 0;
        let kind = kind & 0x7f;
        if kind as usize >= ALL_WALL_KINDS.len() {
            return None;
        }
        Some(WallRec { col: chunk.0 * WALL_CHUNK + place / WALL_CHUNK, row: chunk.1 * WALL_CHUNK + place % WALL_CHUNK, kind, hostile, share, hurt, garrison, life })
    }

    //the wall it was
    pub(crate) fn wall(&self) -> Wall {
        let kind = ALL_WALL_KINDS[self.kind as usize];
        let mut wall = Wall::made(self.col, self.row, kind, if self.hostile { Side::Hostile } else { Side::Friendly }, 1000.0);
        wall.health = byte_share(self.share) * wall.max_health;
        wall.hurt = byte_share(self.hurt) * WALL_FLASH;
        wall.garrison = self.garrison as usize;
        wall.life = if self.life == u16::MAX { f32::INFINITY } else { self.life as f32 / 10.0 };
        wall
    }
}

//the chunk of walls a square is in
pub(crate) fn chunk_of(col: i32, row: i32) -> (i32, i32) {
    (col.div_euclid(WALL_CHUNK), row.div_euclid(WALL_CHUNK))
}

// ---------- the other things in the air ----------

pub(crate) fn write_shot(out: &mut Writer, shot: &Shot) {
    out.vec2(shot.position);
    out.vec2(shot.velocity);
    out.f32(shot.radius);
    out.color(shot.color);
    out.u8(match shot.kind {
        ShotKind::Plain => 0,
        ShotKind::Piercing => 1,
        ShotKind::Splash => 2,
        ShotKind::Homing => 3,
    });
    if shot.kind == ShotKind::Splash {
        out.f32(shot.age);
        out.f32(shot.life);
        out.f32(shot.splash);
    }
}

pub(crate) fn read_shot(input: &mut Reader) -> Option<Shot> {
    let mut shot = Shot::new(input.vec2()?, input.vec2()?);
    shot.radius = input.f32()?;
    shot.color = input.color()?;
    shot.kind = match input.u8()? {
        0 => ShotKind::Plain,
        1 => ShotKind::Piercing,
        2 => ShotKind::Splash,
        3 => ShotKind::Homing,
        _ => return None,
    };
    if shot.kind == ShotKind::Splash {
        shot.age = input.f32()?;
        shot.life = input.f32()?;
        shot.splash = input.f32()?;
    }
    Some(shot)
}

const ALL_CIRCLES: [BossCircle; 18] = [
    BossCircle::Slam,
    BossCircle::Meteor,
    BossCircle::Nova,
    BossCircle::Pull,
    BossCircle::Shift,
    BossCircle::Split,
    BossCircle::Piston,
    BossCircle::Beam,
    BossCircle::Cleave,
    BossCircle::GravityRing,
    BossCircle::Acid,
    BossCircle::Rush,
    BossCircle::Whirlwind,
    BossCircle::Leap,
    BossCircle::Twin,
    BossCircle::Barrage,
    BossCircle::Sweep,
    BossCircle::Judgement,
];
const SHAPE_MOST: f32 = 50_000.0; //a size of a shape a message tells of that is bigger than this is not believed

fn write_shape(out: &mut Writer, shape: Shape) {
    let angle = |dir: Vec2| dir.y.atan2(dir.x);
    match shape {
        Shape::Circle => out.u8(0),
        Shape::Ring { inner } => {
            out.u8(1);
            out.f32(inner);
        }
        Shape::Line { dir, length, width } => {
            out.u8(2);
            out.f32(angle(dir));
            out.f32(length);
            out.f32(width);
        }
        Shape::Cone { dir, arc } => {
            out.u8(3);
            out.f32(angle(dir));
            out.f32(arc);
        }
        Shape::Rect { dir, length, width } => {
            out.u8(4);
            out.f32(angle(dir));
            out.f32(length);
            out.f32(width);
        }
    }
}

fn read_shape(input: &mut Reader) -> Option<Shape> {
    let size = |value: f32| (value.is_finite() && (0.0..=SHAPE_MOST).contains(&value)).then_some(value);
    let way = |angle: f32| angle.is_finite().then(|| Vec2::from_angle(angle));
    Some(match input.u8()? {
        0 => Shape::Circle,
        1 => Shape::Ring { inner: size(input.f32()?)? },
        2 => Shape::Line { dir: way(input.f32()?)?, length: size(input.f32()?)?, width: size(input.f32()?)? },
        3 => {
            let dir = way(input.f32()?)?;
            let arc = input.f32()?;
            if !(arc > 0.0 && arc <= std::f32::consts::TAU) {
                return None;
            }
            Shape::Cone { dir, arc }
        }
        4 => Shape::Rect { dir: way(input.f32()?)?, length: size(input.f32()?)?, width: size(input.f32()?)? },
        _ => return None,
    })
}

pub(crate) fn write_hazard(out: &mut Writer, hazard: &Hazard) {
    out.vec2(hazard.position);
    out.f32(hazard.radius);
    out.flag(hazard.armed);
    out.f32(hazard.warn);
    out.f32(hazard.warn_total);
    out.f32(hazard.damage);
    out.u32(hazard.victims.min(u32::MAX as usize) as u32);
    out.f32(hazard.linger);
    out.f32(hazard.burn);
    out.f32(hazard.flash);
    out.color(hazard.color);
    out.u8(hazard.circle.map_or(0, |circle| 1 + ALL_CIRCLES.iter().position(|&other| other == circle).unwrap_or(0) as u8));
    write_shape(out, hazard.shape);
    out.u32(hazard.group);
    out.u8(hazard.mine as u8 | (hazard.hex as u8) << 1);
}

pub(crate) fn read_hazard(input: &mut Reader) -> Option<Hazard> {
    let position = input.vec2()?;
    let radius = input.f32()?;
    let armed = input.flag()?;
    let (warn, warn_total, damage) = (input.f32()?, input.f32()?, input.f32()?);
    let victims = input.u32()? as usize;
    let (linger, burn, flash) = (input.f32()?, input.f32()?, input.f32()?);
    let color = input.color()?;
    let circle = match input.u8()? {
        0 => None,
        tag => Some(*ALL_CIRCLES.get(tag as usize - 1)?),
    };
    let shape = read_shape(input)?;
    let group = input.u32()?;
    let kind = input.u8()?;
    if kind > 3 {
        return None;
    }
    let mut hazard = Hazard::new(position, radius, warn, damage, victims, color, Cue::Pop);
    hazard.shape = shape;
    hazard.group = group;
    hazard.mine = kind & 1 != 0;
    hazard.hex = kind & 2 != 0;
    hazard.armed = armed;
    hazard.warn_total = warn_total;
    hazard.linger = linger;
    hazard.burn = burn;
    hazard.flash = flash;
    hazard.circle = circle;
    Some(hazard)
}

pub(crate) fn write_pull(out: &mut Writer, pull: &Pull) {
    out.vec2(pull.position);
    out.f32(pull.radius);
    out.f32(pull.left);
    out.f32(pull.total);
}

pub(crate) fn read_pull(input: &mut Reader) -> Option<Pull> {
    Some(Pull { position: input.vec2()?, radius: input.f32()?, left: input.f32()?, total: input.f32()? })
}

pub(crate) fn write_chest(out: &mut Writer, chest: &Chest) {
    out.vec2(chest.at);
    out.u8(chest.tier as u8);
    out.f32(chest.points);
    out.f32(chest.left);
    out.f32(chest.held);
    out.u8(chest.contested as u8 | (chest.stalled as u8) << 1);
}

pub(crate) fn read_chest(input: &mut Reader) -> Option<Chest> {
    let at = input.vec2()?;
    let tier = input.u8()? as u32;
    let (points, left, held) = (input.f32()?, input.f32()?, input.f32()?);
    let flags = input.u8()?;
    if flags > 3 || !(1..=3).contains(&tier) {
        return None;
    }
    Some(Chest { at, tier, points, left, held, contested: flags & 1 != 0, stalled: flags & 2 != 0 })
}

//a boss circle is all that is said of the kinds of site: its state, and which enemy it is built round
pub(crate) fn site_state_byte(state: SiteState) -> u8 {
    match state {
        SiteState::Dormant => 0,
        SiteState::Awake => 1,
        SiteState::Cleared => 2,
    }
}

pub(crate) fn byte_site_state(byte: u8) -> Option<SiteState> {
    match byte {
        0 => Some(SiteState::Dormant),
        1 => Some(SiteState::Awake),
        2 => Some(SiteState::Cleared),
        _ => None,
    }
}

pub(crate) fn formation_byte(formation: Formation) -> u8 {
    FORMATIONS.iter().position(|&other| other == formation).unwrap_or(0) as u8
}

pub(crate) fn byte_formation(byte: u8) -> Option<Formation> {
    FORMATIONS.get(byte as usize).copied()
}
