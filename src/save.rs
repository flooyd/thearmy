//! saving a run to a file and loading it back. the file is plain text, numbers and words with spaces between them, written
//! and read here by hand so the game needs nothing else for it. everything that makes up the world is kept: the army, the
//! walls, the sites, every enemy and the boss fight in progress, and the hordes and the chests they left. what is only in the air for a moment (shots, hazards,
//! particles, the squads' orders) is not, and the game carries on without it

use crate::prelude::*;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::str::SplitAsciiWhitespace;
use std::sync::Mutex;

const SAVE_MAGIC: &str = "THE-ARMY-SAVE";
const SAVE_VERSION: u32 = 1; //changes when the file can no longer be read by the game that came before
const MOST_ITEMS: usize = 5_000_000; //a list longer than this in a file is damage, not an army

#[derive(Debug)]
pub(crate) struct SaveError(pub(crate) String);

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub(crate) struct SaveWriter {
    text: String,
}

impl SaveWriter {
    pub(crate) fn new() -> SaveWriter {
        SaveWriter { text: String::new() }
    }

    pub(crate) fn finish(self) -> String {
        self.text
    }

    pub(crate) fn word(&mut self, word: &str) {
        self.text.push_str(word);
        self.text.push(' ');
    }

    pub(crate) fn number(&mut self, number: impl std::fmt::Display) {
        let _ = write!(self.text, "{number} ");
    }

    pub(crate) fn put<T: Stored>(&mut self, item: &T) {
        item.store(self);
    }

    //a new line in the file, only to make it easier to read
    pub(crate) fn end_line(&mut self) {
        self.text.push('\n');
    }
}

pub(crate) struct SaveReader<'a> {
    words: SplitAsciiWhitespace<'a>,
}

impl<'a> SaveReader<'a> {
    pub(crate) fn new(text: &'a str) -> SaveReader<'a> {
        SaveReader { words: text.split_ascii_whitespace() }
    }

    pub(crate) fn word(&mut self) -> Result<&'a str, SaveError> {
        self.words.next().ok_or_else(|| SaveError("the file ends too soon".to_owned()))
    }

    pub(crate) fn number<T: std::str::FromStr>(&mut self) -> Result<T, SaveError> {
        let word = self.word()?;
        word.parse().map_err(|_| SaveError(format!("{word:?} is not the number it should be")))
    }

    pub(crate) fn get<T: Stored>(&mut self) -> Result<T, SaveError> {
        T::restore(self)
    }

    //whether the next word is this one, without taking it: a section that older saves don't have is there or isn't
    pub(crate) fn next_is(&self, label: &str) -> bool {
        self.words.clone().next() == Some(label)
    }

    //whether every word has been taken: what is on the end of a file that older saves don't have is there or isn't
    pub(crate) fn is_done(&self) -> bool {
        self.words.clone().next().is_none()
    }

    //the next word has to be this one: the sections of the file are labelled so a file that has got out of step is noticed
    pub(crate) fn expect(&mut self, label: &str) -> Result<(), SaveError> {
        let word = self.word()?;
        if word == label { Ok(()) } else { Err(SaveError(format!("expected {label:?} but found {word:?}"))) }
    }
}

//something that can be written to a save and read back from it. the reader takes the words in the order the writer put them
pub(crate) trait Stored: Sized {
    fn store(&self, w: &mut SaveWriter);
    fn restore(r: &mut SaveReader) -> Result<Self, SaveError>;
}

macro_rules! stored_numbers {
    ($($type:ty),*) => {$(
        impl Stored for $type {
            fn store(&self, w: &mut SaveWriter) {
                w.number(*self);
            }

            fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
                r.number()
            }
        }
    )*};
}

stored_numbers!(f32, u32, i32, usize);

//a struct kept field by field, in the order given. the reader builds it with every field, so one that is left out of the
//list is an error when the game is built and not a save that quietly loses it (unless it is named after a `;`, as one that is
//meant not to be kept)
macro_rules! stored_fields {
    ($type:ident { $($field:ident),* $(,)? }) => {
        impl Stored for $type {
            fn store(&self, w: &mut SaveWriter) {
                $(self.$field.store(w);)*
                w.end_line();
            }

            fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
                Ok($type { $($field: r.get()?,)* })
            }
        }
    };
    //the same, with fields that are not kept: they start as the value after the colon when it is read back
    ($type:ident { $($field:ident),* $(,)? } ; $($left:ident: $value:expr),+ $(,)?) => {
        impl Stored for $type {
            fn store(&self, w: &mut SaveWriter) {
                $(self.$field.store(w);)*
                w.end_line();
            }

            fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
                Ok($type { $($field: r.get()?,)* $($left: $value,)+ })
            }
        }
    };
}

//an enum with no data in its variants, kept as the name of the variant
macro_rules! stored_words {
    ($type:ident { $($variant:ident),* $(,)? }) => {
        impl Stored for $type {
            fn store(&self, w: &mut SaveWriter) {
                w.word(match self {
                    $($type::$variant => stringify!($variant),)*
                });
            }

            fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
                let word = r.word()?;
                $(if word == stringify!($variant) {
                    return Ok($type::$variant);
                })*
                Err(SaveError(format!("{word:?} is not a {}", stringify!($type))))
            }
        }
    };
}

impl Stored for bool {
    fn store(&self, w: &mut SaveWriter) {
        w.word(if *self { "yes" } else { "no" });
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        match r.word()? {
            "yes" => Ok(true),
            "no" => Ok(false),
            other => Err(SaveError(format!("{other:?} is neither yes nor no"))),
        }
    }
}

impl Stored for Vec2 {
    fn store(&self, w: &mut SaveWriter) {
        w.number(self.x);
        w.number(self.y);
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        Ok(Vec2::new(r.number()?, r.number()?))
    }
}

impl Stored for Color {
    fn store(&self, w: &mut SaveWriter) {
        for part in [self.r, self.g, self.b, self.a] {
            w.number(part);
        }
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        Ok(Color::new(r.number()?, r.number()?, r.number()?, r.number()?))
    }
}

impl<A: Stored, B: Stored> Stored for (A, B) {
    fn store(&self, w: &mut SaveWriter) {
        self.0.store(w);
        self.1.store(w);
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        Ok((r.get()?, r.get()?))
    }
}

impl<T: Stored> Stored for Option<T> {
    fn store(&self, w: &mut SaveWriter) {
        match self {
            None => w.word("-"),
            Some(item) => {
                w.word("+");
                item.store(w);
            }
        }
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        match r.word()? {
            "-" => Ok(None),
            "+" => Ok(Some(r.get()?)),
            other => Err(SaveError(format!("{other:?} is neither - nor +"))),
        }
    }
}

impl<T: Stored> Stored for Vec<T> {
    fn store(&self, w: &mut SaveWriter) {
        w.number(self.len());
        for item in self {
            item.store(w);
        }
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        let count: usize = r.number()?;
        if count > MOST_ITEMS {
            return Err(SaveError(format!("a list of {count} is more than any run has")));
        }
        //a damaged count must not be able to ask for memory that isn't needed, so the list grows as the items are read
        let mut items = Vec::new();
        for _ in 0..count {
            items.push(r.get()?);
        }
        Ok(items)
    }
}

//every name a thing in the world can have: the enemies' and the nests' and the bosses'
fn known_name(name: &str) -> Option<&'static str> {
    ENEMY_KINDS
        .iter()
        .map(|kind| kind.name)
        .chain(NESTS.iter().map(|nest| nest.name))
        .chain(BOSSES.iter().map(|boss| boss.name))
        .chain([FINAL_BOSS.name])
        .find(|known| *known == name)
}

//a name is kept with underscores for spaces, and has to be one of those the game has
impl Stored for &'static str {
    fn store(&self, w: &mut SaveWriter) {
        w.word(&self.replace(' ', "_"));
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        let name = r.word()?.replace('_', " ");
        known_name(&name).ok_or_else(|| SaveError(format!("{name:?} is not in this game")))
    }
}

//a boss is kept as the name of what it is, which the game looks up
impl Stored for &'static BossDef {
    fn store(&self, w: &mut SaveWriter) {
        self.name.store(w);
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        let name = r.word()?.replace('_', " ");
        BOSSES.iter().chain([&FINAL_BOSS]).find(|boss| boss.name == name).ok_or_else(|| SaveError(format!("{name:?} is not a boss")))
    }
}

stored_words!(Behavior { Walker, Shooter, Splitter, Charger, Bomber, Healer, Summoner, Mortar, Nest, Ambusher });
stored_words!(Modifier { None, Swift, Armored, Volatile, Frenzied, Captain });
stored_words!(WallKind { Wall, Gate, Tower, Spikes, Tar, Barrier });
stored_words!(Side { Friendly, Hostile });
stored_words!(SiteState { Dormant, Awake, Cleared });
//a relic is kept as its name. a save from when there was a Barracks relic still has it: it is a Freezing where it was one of three on
//offer (see `Upgrades`, where the ones taken drop it)
impl Stored for Relic {
    fn store(&self, w: &mut SaveWriter) {
        w.word(&format!("{self:?}"));
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        let word = r.word()?;
        relic_named(word).or_else(|| (word == "Barracks").then_some(Relic::Freezing)).ok_or_else(|| SaveError(format!("{word:?} is not a Relic")))
    }
}

fn relic_named(word: &str) -> Option<Relic> {
    RELICS.iter().copied().find(|relic| format!("{relic:?}") == word)
}

impl Stored for Mode {
    fn store(&self, w: &mut SaveWriter) {
        match *self {
            Mode::Roam => w.word("Roam"),
            Mode::Windup { dir, left } => {
                w.word("Windup");
                w.put(&dir);
                w.put(&left);
            }
            Mode::Dash { dir, left } => {
                w.word("Dash");
                w.put(&dir);
                w.put(&left);
            }
            Mode::Recover { left } => {
                w.word("Recover");
                w.put(&left);
            }
            Mode::Fuse { left } => {
                w.word("Fuse");
                w.put(&left);
            }
            Mode::Cast { left } => {
                w.word("Cast");
                w.put(&left);
            }
            Mode::Lurk => w.word("Lurk"),
        }
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        match r.word()? {
            "Roam" => Ok(Mode::Roam),
            "Windup" => Ok(Mode::Windup { dir: r.get()?, left: r.get()? }),
            "Dash" => Ok(Mode::Dash { dir: r.get()?, left: r.get()? }),
            "Recover" => Ok(Mode::Recover { left: r.get()? }),
            "Fuse" => Ok(Mode::Fuse { left: r.get()? }),
            "Cast" => Ok(Mode::Cast { left: r.get()? }),
            "Lurk" => Ok(Mode::Lurk),
            other => Err(SaveError(format!("{other:?} is not a mode"))),
        }
    }
}

impl Stored for Steer {
    fn store(&self, w: &mut SaveWriter) {
        match *self {
            Steer::Player => w.word("Player"),
            Steer::Point(point) => {
                w.word("Point");
                w.put(&point);
            }
        }
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        match r.word()? {
            "Player" => Ok(Steer::Player),
            "Point" => Ok(Steer::Point(r.get()?)),
            other => Err(SaveError(format!("{other:?} is not a way to steer"))),
        }
    }
}

impl Stored for SiteKind {
    fn store(&self, w: &mut SaveWriter) {
        match *self {
            SiteKind::Nest(which) => {
                w.word("Nest");
                w.put(&which);
            }
            SiteKind::Village => w.word("Village"),
            SiteKind::Barracks => w.word("Barracks"),
            SiteKind::Citadel => w.word("Citadel"),
        }
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        match r.word()? {
            "Nest" => {
                let which: usize = r.get()?;
                if which < NESTS.len() { Ok(SiteKind::Nest(which)) } else { Err(SaveError(format!("there is no nest {which}"))) }
            }
            "Village" => Ok(SiteKind::Village),
            "Barracks" => Ok(SiteKind::Barracks),
            "Citadel" => Ok(SiteKind::Citadel),
            other => Err(SaveError(format!("{other:?} is not a kind of site"))),
        }
    }
}

//(how long ago a unit grew is not kept: everyone loaded is full grown. a champion is kept apart, in its own section. nor is whose
//army it is in: a saved game has one army, the first player's)
impl Stored for Unit {
    fn store(&self, w: &mut SaveWriter) {
        self.x.store(w);
        self.y.store(w);
        self.speed.store(w);
        self.steer.store(w);
        self.steer_in.store(w);
        self.velocity.store(w);
        self.target.store(w);
        self.health.store(w);
        self.garrison.store(w);
        self.guard.store(w);
        self.calm.store(w);
        w.end_line();
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        Ok(Unit { x: r.get()?, y: r.get()?, speed: r.get()?, steer: r.get()?, steer_in: r.get()?, velocity: r.get()?, nose: Vec2::new(0.0, -1.0), target: r.get()?, health: r.get()?, garrison: r.get()?, guard: r.get()?, calm: r.get()?, age: 99.0, champion: 0, power: 0.0, might: 1.0, ward: 1.0, level: 0, xp: 0.0, slot: rand::rand(), id: NEXT_UNIT_ID.fetch_add(1, Ordering::Relaxed), owner: 0 })
    }
}
stored_fields!(Baby { position, age });
stored_fields!(Wall { col, row, size, color, kind, side, health, max_health, life, hurt, garrison, fire_in });
stored_fields!(Site { id, kind, position, state, core, tiles, clock });
stored_fields!(BossState { def, tier, phase, timers, shifting, last_stand, family, family_max, generation, facing, anchor_left });
stored_fields!(Director { next_boss_in, slain, boss, warned });

//a horde is kept as the name of what it is, which the game looks up
impl Stored for &'static HordeDef {
    fn store(&self, w: &mut SaveWriter) {
        w.word(&self.name.replace(' ', "_"));
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        let name = r.word()?.replace('_', " ");
        horde_named(&name).ok_or_else(|| SaveError(format!("{name:?} is not a horde")))
    }
}
stored_fields!(Fight { def, total, lost, worth, last_at });
stored_fields!(Chest { at, tier, points, left }; held: 0.0, contested: false, stalled: false);

//a horde coming from two sides: the second side's place is not with the rest, which older saves (they have only hordes with one
//side) have no word for, so it comes after everything else, and they don't have it
impl Stored for Coming {
    fn store(&self, w: &mut SaveWriter) {
        self.def.store(w);
        self.at.store(w);
        self.left.store(w);
        w.end_line();
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        Ok(Coming { def: r.get()?, at: r.get()?, left: r.get()?, other: None })
    }
}

//(the second side of the horde that is coming, if it has one, is the last thing in the file, since nothing else follows the hordes:
//a save from before that ends after the chests)
impl Stored for Hordes {
    fn store(&self, w: &mut SaveWriter) {
        self.next_in.store(w);
        self.coming.store(w);
        self.fight.store(w);
        self.chests.store(w);
        w.end_line();
        self.coming.and_then(|coming| coming.other).store(w);
        w.end_line();
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        let mut hordes = Hordes { next_in: r.get()?, coming: r.get()?, fight: r.get()?, chests: r.get()? };
        if !r.is_done() {
            let other: Option<Vec2> = r.get()?;
            if let Some(coming) = hordes.coming.as_mut() {
                coming.other = other;
            }
        }
        Ok(hordes)
    }
}

//(how many times the citadel has fallen is on the end, which a save from before it came back has not got: the next word in
//one of those is the label of the section after)
impl Stored for Run {
    fn store(&self, w: &mut SaveWriter) {
        self.destroyed.store(w);
        self.lost.store(w);
        self.peak_army.store(w);
        self.bosses.store(w);
        self.nests.store(w);
        self.villages.store(w);
        self.seconds.store(w);
        self.citadels.store(w);
        w.end_line();
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        let mut run = Run { destroyed: r.get()?, lost: r.get()?, peak_army: r.get()?, bosses: r.get()?, nests: r.get()?, villages: r.get()?, seconds: r.get()?, citadels: 0 };
        if !r.next_is("rerolls") {
            run.citadels = r.get()?;
        }
        Ok(run)
    }
}
stored_fields!(
    Enemy {
        id, name, position, size, color, max_health, health, babies, speed, route, heading_for, attack, aggro, chasing, range, volley, shot_in,
        hurt, killed, popup_in, behavior, modifier, scale, owner, armor, shield, max_shield, shield_idle, mode, timer, flash, blew_up, boss,
        site, steer, steer_in, stuck, pressing, slow
    } ; frozen: 0.0, provoked: 0.0, grudge: Vec2::ZERO, rout: 0.0, rallied: false, credit: 0 //(a freeze lasts a moment, and so do a grudge and a rout, and the rally is worked out again at once, and whose army hurt it is a matter of the next frame, so they are not kept)
);

//the walls are kept in order of their squares, so the same walls make the same file
impl Stored for Walls {
    fn store(&self, w: &mut SaveWriter) {
        let mut walls: Vec<&Wall> = self.values().collect();
        walls.sort_by_key(|wall| (wall.col, wall.row));
        w.number(walls.len());
        for wall in walls {
            wall.store(w);
        }
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        let walls: Vec<Wall> = r.get()?;
        Ok(walls.into_iter().map(|wall| ((wall.col, wall.row), wall)).collect())
    }
}

//the relics taken, which are taken again in the same order when it is read, so the numbers they add up to come out the same
//(a save from when there was a Barracks relic has some, and they are left out: the most units the army can have is the player's to set now)
impl Stored for Upgrades {
    fn store(&self, w: &mut SaveWriter) {
        self.taken.store(w);
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        let count: usize = r.number()?;
        if count > MOST_ITEMS {
            return Err(SaveError(format!("a list of {count} is more than any run has")));
        }
        let mut upgrades = Upgrades::default();
        for _ in 0..count {
            let word = r.word()?;
            if word == "Barracks" {
                continue;
            }
            upgrades.take(relic_named(word).ok_or_else(|| SaveError(format!("{word:?} is not a Relic")))?);
        }
        Ok(upgrades)
    }
}

//the relics on offer. how long they have been up is not kept: they are shown again from the start
impl Stored for RelicChoice {
    fn store(&self, w: &mut SaveWriter) {
        for relic in self.options {
            w.put(&relic);
        }
        w.put(&self.rare_only);
        w.put(&false); //(it was whether the army was full, which a Barracks relic cared about. kept so a file is the shape it was)
        w.put(&self.chest);
    }

    fn restore(r: &mut SaveReader) -> Result<Self, SaveError> {
        let options = [r.get()?, r.get()?, r.get()?];
        let rare_only = r.get()?;
        let _army_full: bool = r.get()?;
        let mut choice = RelicChoice { options, age: 0.0, rare_only, chest: 0 };
        //(where it came from is on the end, which a save from before chests has not got: the next word in one of those is the label of the section after)
        if !r.next_is("victory") {
            choice.chest = r.get()?;
        }
        Ok(choice)
    }
}

//a player of a game with others, as a save has them: all but their army, which is among the units (marked with whose it is)
#[derive(Clone)]
pub(crate) struct Companion {
    pub(crate) name: String,
    pub(crate) position: Vec2,
    pub(crate) rerolls: u32,
    pub(crate) upgrades: Upgrades,
    pub(crate) wall_kind: WallKind,
    pub(crate) wall_points: f32,
    pub(crate) choice: Option<RelicChoice>,
}

//a name as one word of a file, which has no spaces in it: the bytes of it in hexadecimal
pub(crate) fn name_word(name: &str) -> String {
    name.bytes().map(|byte| format!("{byte:02x}")).collect()
}

fn name_from(word: &str) -> Result<String, SaveError> {
    let bytes: Option<Vec<u8>> = (0..word.len()).step_by(2).map(|at| word.get(at..at + 2).and_then(|pair| u8::from_str_radix(pair, 16).ok())).collect();
    let name = bytes.filter(|_| word.len() % 2 == 0).and_then(|bytes| String::from_utf8(bytes).ok()).ok_or_else(|| SaveError(format!("{word:?} is not a name")))?;
    let clean = clean_name(&name);
    if clean.is_empty() || clean != name {
        return Err(SaveError(format!("{name:?} is not a name a player can have")));
    }
    Ok(clean)
}

//a run at one moment: all that has to be put back to carry on from it. a game with others in it has a `company`: the players after the first, whose armies
//are among the units (in order of owner)
pub(crate) struct Snapshot {
    pub(crate) player: Vec2,
    pub(crate) zoom: f32,
    pub(crate) run: Run,
    pub(crate) rerolls: u32,
    pub(crate) upgrades: Upgrades,
    pub(crate) wall_kind: WallKind, //what the player has chosen to build...
    pub(crate) wall_points: f32,    //...and how many wall points they have saved up
    pub(crate) director: Director,
    pub(crate) units: Vec<Unit>,
    pub(crate) babies: Vec<Baby>,
    pub(crate) walls: Walls,
    pub(crate) sites: Vec<Site>,
    pub(crate) enemies: Vec<Enemy>,
    pub(crate) choice: Option<RelicChoice>, //the relics on offer, if the run was waiting for one to be chosen
    pub(crate) victory: bool,               //the citadel had just fallen and the victory screen was up
    pub(crate) hordes: Hordes,              //when the next horde comes, the one being fought and the chests it left
    pub(crate) company: Vec<Companion>,     //the other players, if there were any
}

impl Snapshot {
    //what is in a run now, copied so the game can carry on while it is written out
    pub(crate) fn capture(
        player: Vec2,
        zoom: f32,
        run: &Run,
        rerolls: u32,
        upgrades: &Upgrades,
        mason: &Mason,
        director: &Director,
        units: &[Unit],
        babies: &[Baby],
        walls: &Walls,
        sites: &[Site],
        enemies: &[Enemy],
        choice: Option<&RelicChoice>,
        victory: bool,
        hordes: &Hordes,
    ) -> Snapshot {
        Snapshot {
            player,
            zoom,
            run: run.clone(),
            rerolls,
            upgrades: upgrades.clone(),
            wall_kind: mason.selected,
            wall_points: mason.points,
            director: director.clone(),
            units: units.to_vec(),
            babies: babies.to_vec(),
            walls: walls.iter().map(|(&tile, wall)| (tile, wall.clone())).collect(),
            sites: sites.to_vec(),
            enemies: enemies.to_vec(),
            choice: choice.cloned(),
            victory,
            hordes: hordes.clone(),
            company: Vec::new(),
        }
    }

    //the line at the top of a file: what the title screen tells the player about it without reading the rest
    pub(crate) fn header(&self, saved_at: f64) -> String {
        let party = if self.company.is_empty() { String::new() } else { format!(" {}", self.company.len() + 1) };
        format!("{SAVE_MAGIC} {SAVE_VERSION} {saved_at:.0} {} {} {}{party}\n", self.units.len(), self.run.seconds, self.run.bosses)
    }

    //everything below the header
    pub(crate) fn body(&self) -> String {
        let mut w = SaveWriter::new();
        w.text.reserve(64 * 1024 + 100 * self.units.len());
        w.word("player");
        w.put(&self.player);
        w.put(&self.zoom);
        w.end_line();
        w.word("run");
        w.put(&self.run);
        w.word("rerolls");
        w.put(&self.rerolls);
        w.end_line();
        w.word("relics");
        w.put(&self.upgrades);
        w.end_line();
        w.word("building");
        w.put(&self.wall_kind);
        w.put(&self.wall_points);
        w.end_line();
        w.word("director");
        w.put(&self.director);
        w.word("offer");
        w.put(&self.choice);
        w.word("victory");
        w.put(&self.victory);
        w.end_line();
        w.word("units");
        w.put(&self.units);
        w.word("babies");
        w.put(&self.babies);
        w.word("walls");
        w.put(&self.walls);
        w.word("sites");
        w.put(&self.sites);
        w.word("enemies");
        w.put(&self.enemies);
        //the champions are units, kept here by their place in the list, so a save from before there were any is still a save
        let champions: Vec<(usize, &Unit)> = self.units.iter().enumerate().filter(|(_, unit)| unit.champion_kind().is_some()).collect();
        w.word("champions");
        w.number(champions.len());
        w.end_line();
        for (i, unit) in &champions {
            w.number(*i);
            w.word(unit.champion_kind().map_or("", |kind| kind.name));
            w.number(unit.power);
            w.end_line();
        }
        //what levels they have reached, in a section of their own which a save from before they could rise has not got
        w.word("veterans");
        w.number(champions.len());
        w.end_line();
        for (i, unit) in &champions {
            w.number(*i);
            w.number(unit.level);
            w.number(unit.xp);
            w.end_line();
        }
        //the hordes, in a section of their own which a save from before they were kept has not got
        w.word("hordes");
        w.put(&self.hordes);
        //the other players, in a section of their own, and whose each army is (which is told by how many in a row are of each, as the armies are together)
        if !self.company.is_empty() {
            w.word("party");
            w.number(self.company.len());
            w.end_line();
            for mate in &self.company {
                w.word(&name_word(&mate.name));
                w.put(&mate.position);
                w.put(&mate.rerolls);
                w.put(&mate.upgrades);
                w.put(&mate.wall_kind);
                w.put(&mate.wall_points);
                w.put(&mate.choice);
                w.end_line();
            }
            let mut runs: Vec<(u8, usize)> = Vec::new();
            for unit in &self.units {
                match runs.last_mut() {
                    Some((owner, count)) if *owner == unit.owner => *count += 1,
                    _ => runs.push((unit.owner, 1)),
                }
            }
            w.word("owners");
            w.number(runs.len());
            for (owner, count) in runs {
                w.number(owner);
                w.number(count);
            }
            w.end_line();
        }
        w.finish()
    }

    pub(crate) fn to_text(&self) -> String {
        self.header(miniquad::date::now()) + &self.body()
    }

    //read a run back. the ids new enemies and sites are given start above all those in it, so none is ever given twice, and
    //the units are no longer on their way to any enemy, as the squads that sent them are gone
    pub(crate) fn from_text(text: &str) -> Result<Snapshot, SaveError> {
        let mut r = SaveReader::new(text);
        r.expect(SAVE_MAGIC)?;
        let version: u32 = r.number()?;
        if version != SAVE_VERSION {
            return Err(SaveError(format!("it was saved by another version of the game (file {version}, game {SAVE_VERSION})")));
        }
        //the rest of the header is for the title screen
        for _ in 0..4 {
            r.word()?;
        }
        if !r.next_is("player") {
            r.word()?; //(how many players the run had)
        }
        r.expect("player")?;
        let player = r.get()?;
        let zoom = r.get()?;
        r.expect("run")?;
        let run = r.get()?;
        r.expect("rerolls")?;
        let rerolls = r.get()?;
        r.expect("relics")?;
        let upgrades = r.get()?;
        r.expect("building")?;
        let wall_kind = r.get()?;
        let wall_points = r.get()?;
        r.expect("director")?;
        let director = r.get()?;
        r.expect("offer")?;
        let choice = r.get()?;
        r.expect("victory")?;
        let victory = r.get()?;
        r.expect("units")?;
        let mut units: Vec<Unit> = r.get()?;
        r.expect("babies")?;
        let babies = r.get()?;
        r.expect("walls")?;
        let walls = r.get()?;
        r.expect("sites")?;
        let mut sites: Vec<Site> = r.get()?;
        r.expect("enemies")?;
        let mut enemies: Vec<Enemy> = r.get()?;
        for enemy in &mut enemies {
            //(an enemy walks a route and is heading for a stop on it: one that is not is put right, the way a copy of an enemy on another computer is)
            if enemy.route.is_empty() {
                enemy.route.push(enemy.position);
            }
            if enemy.heading_for >= enemy.route.len() {
                enemy.heading_for = 0;
            }
        }
        if r.next_is("champions") {
            r.expect("champions")?;
            let count: usize = r.number()?;
            if count > units.len() {
                return Err(SaveError(format!("{count} champions in an army of {}", units.len())));
            }
            for _ in 0..count {
                let at: usize = r.number()?;
                let name = r.word()?;
                let power: f32 = r.number()?;
                let kind = champion_named(name).ok_or_else(|| SaveError(format!("{name:?} is not a champion")))?;
                let unit = units.get_mut(at).ok_or_else(|| SaveError(format!("champion {name} is unit {at}, and there is no such unit")))?;
                unit.champion = kind as u8 + 1;
                unit.power = power;
                unit.level = 1;
            }
        }
        if !r.next_is("veterans") {
            //(champions saved before they had levels are level 1, mended to the health that has)
            for unit in units.iter_mut().filter(|unit| unit.champion != 0) {
                unit.health = champion_health(1);
            }
        } else {
            r.expect("veterans")?;
            let count: usize = r.number()?;
            if count > units.len() {
                return Err(SaveError(format!("{count} veterans in an army of {}", units.len())));
            }
            for _ in 0..count {
                let at: usize = r.number()?;
                let level: u8 = r.number()?;
                let xp: f32 = r.number()?;
                let unit = units.get_mut(at).ok_or_else(|| SaveError(format!("veteran {at} is not a unit")))?;
                if unit.champion == 0 {
                    return Err(SaveError(format!("unit {at} has a level but is not a champion")));
                }
                unit.level = level.clamp(1, CHAMPION_MAX_LEVEL);
                unit.xp = xp.max(0.0);
                unit.health = unit.health.min(unit.max_health());
            }
        }
        let hordes: Hordes = if r.next_is("hordes") {
            r.expect("hordes")?;
            r.get()?
        } else {
            Hordes::new()
        };
        let mut company: Vec<Companion> = Vec::new();
        if r.next_is("party") {
            r.expect("party")?;
            let count: usize = r.number()?;
            if count == 0 || count >= MAX_PLAYERS {
                return Err(SaveError(format!("{count} other players in a game for {MAX_PLAYERS}")));
            }
            for _ in 0..count {
                let name = name_from(r.word()?)?;
                if company.iter().any(|other| other.name.eq_ignore_ascii_case(&name)) {
                    return Err(SaveError(format!("two players are called {name}")));
                }
                company.push(Companion { name, position: r.get()?, rerolls: r.get()?, upgrades: r.get()?, wall_kind: r.get()?, wall_points: r.get()?, choice: r.get()? });
            }
            r.expect("owners")?;
            let runs: usize = r.number()?;
            let mut at = 0;
            for _ in 0..runs {
                let owner: usize = r.number()?;
                let length: usize = r.number()?;
                if owner > company.len() || length > units.len() - at {
                    return Err(SaveError(format!("{length} units of player {owner} do not fit in an army of {}", units.len())));
                }
                for unit in &mut units[at..at + length] {
                    unit.owner = owner as u8;
                }
                at += length;
            }
            if at != units.len() {
                return Err(SaveError(format!("{at} of {} units have an owner", units.len())));
            }
        }
        for unit in &mut units {
            unit.target = None;
        }
        if let Some(last) = enemies.iter().map(|enemy| enemy.id).max() {
            NEXT_ENEMY_ID.fetch_max(last + 1, Ordering::Relaxed);
        }
        if let Some(last) = sites.iter().map(|site| site.id).max() {
            NEXT_SITE_ID.fetch_max(last + 1, Ordering::Relaxed);
        }
        //a save from before there were barracks has none, and gets them now, which is why the ids above come first
        if !sites.iter().any(|site| site.kind == SiteKind::Barracks) {
            place_sites(&mut sites, vec![SiteKind::Barracks; BARRACKS_COUNT]);
        }
        Ok(Snapshot { player, zoom, run, rerolls, upgrades, wall_kind, wall_points, director, units, babies, walls, sites, enemies, choice, victory, hordes, company })
    }
}

//what a save says about itself in its first line
#[derive(Clone, Copy, Debug)]
pub(crate) struct SaveInfo {
    pub(crate) saved_at: f64, //seconds since 1970
    pub(crate) army: usize,
    pub(crate) seconds: f32, //played
    pub(crate) bosses: usize,
    pub(crate) party: usize, //how many players were in it
}

impl SaveInfo {
    pub(crate) fn from_header(line: &str) -> Result<SaveInfo, SaveError> {
        let mut r = SaveReader::new(line);
        r.expect(SAVE_MAGIC)?;
        let version: u32 = r.number()?;
        if version != SAVE_VERSION {
            return Err(SaveError(format!("it was saved by another version of the game (file {version}, game {SAVE_VERSION})")));
        }
        let (saved_at, army, seconds, bosses) = (r.number()?, r.number()?, r.number()?, r.number()?);
        Ok(SaveInfo { saved_at, army, seconds, bosses, party: if r.is_done() { 1 } else { r.number()? } })
    }
}

//the places a run is kept: the one the game keeps by itself, the one the player saves to, and the one for a game with others in it (which is the host's, and
//is carried on from by hosting it again, when the players join it by their names)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Slot {
    Auto,
    Quick,
    Party,
}

pub(crate) const AUTOSAVE_SECONDS: f32 = 60.0; //how often the game saves by itself

//two saves are never written at once, however they were asked for
static WRITING: Mutex<()> = Mutex::new(());

//the folder the saves are in
#[derive(Clone, Debug)]
pub(crate) struct Saves {
    dir: PathBuf,
}

impl Saves {
    //the folder the player's saves go in: the one for application data, or failing that the one the game is run from
    pub(crate) fn in_home() -> Saves {
        let var = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from);
        let base = var("APPDATA").or_else(|| var("XDG_DATA_HOME")).or_else(|| var("HOME").map(|home| home.join(".local").join("share")));
        Saves::in_dir(base.map_or_else(|| PathBuf::from("."), |base| base.join("TheArmy")))
    }

    pub(crate) fn in_dir(dir: impl Into<PathBuf>) -> Saves {
        Saves { dir: dir.into() }
    }

    pub(crate) fn folder(&self) -> &PathBuf {
        &self.dir
    }

    pub(crate) fn path(&self, slot: Slot) -> PathBuf {
        self.dir.join(match slot {
            Slot::Auto => "autosave.sav",
            Slot::Quick => "quicksave.sav",
            Slot::Party => "partysave.sav",
        })
    }

    //write a save: to a file beside it first, so one that is cut short by a crash or a full disk never replaces a good save
    pub(crate) fn write(&self, slot: Slot, text: &str) -> std::io::Result<()> {
        let _one_at_a_time = WRITING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        std::fs::create_dir_all(&self.dir)?;
        let path = self.path(slot);
        let partial = path.with_extension("sav.part");
        std::fs::write(&partial, text)?;
        std::fs::rename(&partial, &path)
    }

    //write a save on another thread, so a big army doesn't hold the game up. a failure can only be reported on the console
    pub(crate) fn write_in_background(&self, slot: Slot, snapshot: Snapshot) {
        let saves = self.clone();
        std::thread::spawn(move || {
            if let Err(error) = saves.write(slot, &snapshot.to_text()) {
                eprintln!("could not save the game to {}: {error}", saves.path(slot).display());
            }
        });
    }

    pub(crate) fn read(&self, slot: Slot) -> Result<Snapshot, SaveError> {
        let text = std::fs::read_to_string(self.path(slot)).map_err(|error| SaveError(format!("could not read the save: {error}")))?;
        Snapshot::from_text(&text)
    }

    //what a save says about itself, read from only the first line of it
    pub(crate) fn info(&self, slot: Slot) -> Option<SaveInfo> {
        use std::io::BufRead;
        let file = std::fs::File::open(self.path(slot)).ok()?;
        let mut line = String::new();
        std::io::BufReader::new(file).read_line(&mut line).ok()?;
        SaveInfo::from_header(&line).ok()
    }

    pub(crate) fn delete(&self, slot: Slot) {
        let _ = std::fs::remove_file(self.path(slot));
    }

    //the save that was made last, if there is one that can be read
    pub(crate) fn newest(&self) -> Option<(Slot, SaveInfo)> {
        [Slot::Auto, Slot::Quick]
            .into_iter()
            .filter_map(|slot| self.info(slot).map(|info| (slot, info)))
            .max_by(|a, b| a.1.saved_at.total_cmp(&b.1.saved_at))
    }
}
