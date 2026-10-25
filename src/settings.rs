//! what the player can change: how loud things are, how much the screen shakes, the colours for those who can't tell red from
//! green, how hard the enemies are, which formation the army keeps, what the units look like, how big the army can grow and whether hints are shown. kept in a small text file beside the
//! saves and read again the next time the game starts

use crate::prelude::*;
use std::sync::atomic::AtomicBool;

//how much harder than normal the enemies are made: the wild ones are built for an army this many times as big as the player's,
//the bosses are made rounds further on, and the harder ones make up for some of what the army's relics add to its damage
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Difficulty {
    Easy,
    Normal,
    Hard,
    Brutal,
}

pub(crate) const DIFFICULTIES: [Difficulty; 4] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard, Difficulty::Brutal];

//what the ordinary units of an army are drawn as
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum UnitShape {
    Ships,   //a small fighter that points the way it is going
    Circles, //a plain round disc with a black edge
}

pub(crate) const UNIT_SHAPES: [UnitShape; 2] = [UnitShape::Ships, UnitShape::Circles];

impl UnitShape {
    pub(crate) fn name(self) -> &'static str {
        match self {
            UnitShape::Ships => "Ships",
            UnitShape::Circles => "Circles",
        }
    }

    pub(crate) fn blurb(self) -> &'static str {
        match self {
            UnitShape::Ships => "each unit is a small fighter that points the way it is going",
            UnitShape::Circles => "each unit is a plain circle with a black edge",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<UnitShape> {
        UNIT_SHAPES.into_iter().find(|shape| shape.name() == name)
    }
}

impl Difficulty {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
            Difficulty::Brutal => "Brutal",
        }
    }

    //the enemies are made for an army this many times the size of the player's
    pub(crate) fn pressure(self) -> f32 {
        match self {
            Difficulty::Easy => 0.7,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 1.8,
            Difficulty::Brutal => 2.8,
        }
    }

    //how many rounds further on than the ones slain bosses are made: tougher, quicker and harder hitting, as they are each time round
    pub(crate) fn boss_rounds(self) -> u32 {
        match self {
            Difficulty::Easy | Difficulty::Normal => 0,
            Difficulty::Hard => 1,
            Difficulty::Brutal => 2,
        }
    }

    //the power of the damage the relics add that the enemies shrug off (see `Upgrades::hit`): 0 for none of it, 1 for all of it
    pub(crate) fn relic_answer(self) -> f32 {
        match self {
            Difficulty::Easy | Difficulty::Normal => 0.0,
            Difficulty::Hard => 0.5,
            Difficulty::Brutal => 0.55,
        }
    }

    //what a run played on it is worth next to one on Normal, in score
    pub(crate) fn score_factor(self) -> f32 {
        match self {
            Difficulty::Easy => 0.6,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 1.6,
            Difficulty::Brutal => 2.4,
        }
    }

    pub(crate) fn blurb(self) -> &'static str {
        match self {
            Difficulty::Easy => "enemies are made for an army 30% smaller",
            Difficulty::Normal => "as it was meant to be played",
            Difficulty::Hard => "enemies for an army 80% bigger, tougher bosses, damage relics count for less",
            Difficulty::Brutal => "enemies for an army 2.8x as big, much tougher bosses, damage relics count for far less",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Difficulty> {
        DIFFICULTIES.into_iter().find(|difficulty| difficulty.name() == name)
    }
}

//how many of the bolts the front of the army fires are flown and drawn: on Low fewer, each doing more, which is lighter on the computer. what they do
//between them is the same on every setting
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Density {
    Low,
    Medium,
    High,
}

pub(crate) const DENSITIES: [Density; 3] = [Density::Low, Density::Medium, Density::High];

impl Density {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Density::Low => "Low",
            Density::Medium => "Medium",
            Density::High => "High",
        }
    }

    pub(crate) fn blurb(self) -> &'static str {
        match self {
            Density::Low => "about a third of the army's bolts, each the stronger for it: lighter on the computer",
            Density::Medium => "about two thirds of the army's bolts, each the stronger for it",
            Density::High => "every bolt the front of the army fires",
        }
    }

    //the share of the bolts that are fired
    pub(crate) fn share(self) -> f32 {
        match self {
            Density::Low => 0.35,
            Density::Medium => 0.65,
            Density::High => 1.0,
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Density> {
        DENSITIES.into_iter().find(|density| density.name() == name)
    }
}

//the share of the bolts fired, as the setting has it (see `Settings::apply`), as the bits of a number
static BOLT_DENSITY: AtomicU32 = AtomicU32::new(0x3F80_0000);

pub(crate) fn bolt_density() -> f32 {
    f32::from_bits(BOLT_DENSITY.load(Ordering::Relaxed))
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Settings {
    pub(crate) master: f32,  //0 to 1, how loud everything is
    pub(crate) effects: f32, //0 to 1, how loud the sounds are next to the music
    pub(crate) music: f32,   //0 to 1, how loud the music is
    pub(crate) shake: f32,   //0 to 1, how much the screen shakes and the camera jumps in for big moments
    pub(crate) fullscreen: bool, //whether the window fills the screen now (it starts that way every time, and this is only kept as the game was left)
    pub(crate) colourblind: bool,
    pub(crate) difficulty: Difficulty,
    pub(crate) hints: bool,
    pub(crate) formation: Formation,
    pub(crate) unit_shape: UnitShape,
    pub(crate) simple_shapes: bool, //whether the enemies and bosses are drawn as plain squares, in place of the pictures they have
    pub(crate) bolts: Density, //how many of the army's bolts are fired, each the stronger for the ones that aren't
    pub(crate) army_limit: usize, //the most units the army can have, from MIN_ARMY_LIMIT to MAX_ARMY_LIMIT
    pub(crate) player_name: String, //what the player is called in a game with others, as `clean_name` leaves it
    pub(crate) host_port: u16, //the port they host on, or join a game on this computer by
    pub(crate) multiplayer_role: Role, //what they last did in multiplayer: hosted, joined from the list, or joined on this computer
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { master: 1.0, effects: 1.0, music: 1.0, shake: 1.0, fullscreen: true, colourblind: false, difficulty: Difficulty::Normal, hints: true, formation: Formation::Ring, unit_shape: UnitShape::Ships, simple_shapes: false, bolts: Density::High, army_limit: DEFAULT_ARMY_LIMIT, player_name: String::new(), host_port: DEFAULT_PORT, multiplayer_role: Role::Host }
    }
}

impl Settings {
    //the settings as lines of text, a name and a value each
    pub(crate) fn to_text(&self) -> String {
        format!(
            "master {:.2}\neffects {:.2}\nmusic {:.2}\nshake {:.2}\nfullscreen {}\ncolourblind {}\ndifficulty {}\nhints {}\nformation {}\nunit_shape {}\nsimple_shapes {}\nbolts {}\narmy_limit {}\nplayer_name {}\nhost_port {}\nmultiplayer_role {}\n",
            self.master,
            self.effects,
            self.music,
            self.shake,
            if self.fullscreen { "yes" } else { "no" },
            if self.colourblind { "yes" } else { "no" },
            self.difficulty.name(),
            if self.hints { "yes" } else { "no" },
            self.formation.name(),
            self.unit_shape.name(),
            if self.simple_shapes { "yes" } else { "no" },
            self.bolts.name(),
            self.army_limit,
            self.player_name,
            self.host_port,
            self.multiplayer_role.name(),
        )
    }

    //settings read back. a line that isn't understood, or a name the game doesn't know, is left alone, so a file from another
    //version of the game still gives what it can
    pub(crate) fn from_text(text: &str) -> Settings {
        let mut settings = Settings::default();
        for line in text.lines() {
            let mut words = line.split_ascii_whitespace();
            let (Some(name), Some(value)) = (words.next(), words.next()) else {
                continue;
            };
            let level = || value.parse::<f32>().ok().filter(|level| level.is_finite()).map(|level| level.clamp(0.0, 1.0));
            let yes = || match value {
                "yes" => Some(true),
                "no" => Some(false),
                _ => None,
            };
            match name {
                "master" => settings.master = level().unwrap_or(settings.master),
                "effects" => settings.effects = level().unwrap_or(settings.effects),
                "music" => settings.music = level().unwrap_or(settings.music),
                "shake" => settings.shake = level().unwrap_or(settings.shake),
                "fullscreen" => settings.fullscreen = yes().unwrap_or(settings.fullscreen),
                "colourblind" => settings.colourblind = yes().unwrap_or(settings.colourblind),
                "hints" => settings.hints = yes().unwrap_or(settings.hints),
                "difficulty" => settings.difficulty = Difficulty::from_name(value).unwrap_or(settings.difficulty),
                "formation" => settings.formation = Formation::from_name(value).unwrap_or(settings.formation),
                "unit_shape" => settings.unit_shape = UnitShape::from_name(value).unwrap_or(settings.unit_shape),
                "simple_shapes" => settings.simple_shapes = yes().unwrap_or(settings.simple_shapes),
                "bolts" => settings.bolts = Density::from_name(value).unwrap_or(settings.bolts),
                "army_limit" => settings.army_limit = value.parse::<usize>().map_or(settings.army_limit, |limit| limit.clamp(MIN_ARMY_LIMIT, MAX_ARMY_LIMIT)),
                "player_name" => settings.player_name = clean_name(line.split_once(char::is_whitespace).map_or("", |(_, rest)| rest)),
                "host_port" => settings.host_port = parse_port(value).unwrap_or(settings.host_port),
                "multiplayer_role" => settings.multiplayer_role = Role::from_name(value).unwrap_or(settings.multiplayer_role),
                _ => {}
            }
        }
        settings
    }

    //put into effect what is not read from the settings each time it is used: the colours, and how many bolts the army fires
    pub(crate) fn apply(&self) {
        COLOURBLIND.store(self.colourblind, Ordering::Relaxed);
        BOLT_DENSITY.store(self.bolts.share().to_bits(), Ordering::Relaxed);
    }

    //the sound, with the volumes asked for
    pub(crate) fn tune(&self, sounds: &mut Sounds) {
        sounds.volume = self.master;
        sounds.effects = self.effects;
        sounds.music_volume = self.music;
    }
}

impl Saves {
    pub(crate) fn settings_path(&self) -> std::path::PathBuf {
        self.folder().join("settings.txt")
    }

    //the settings the player left, or the usual ones if there are none yet
    pub(crate) fn load_settings(&self) -> Settings {
        std::fs::read_to_string(self.settings_path()).map(|text| Settings::from_text(&text)).unwrap_or_default()
    }

    pub(crate) fn save_settings(&self, settings: &Settings) {
        let result = std::fs::create_dir_all(self.folder()).and_then(|()| std::fs::write(self.settings_path(), settings.to_text()));
        if let Err(error) = result {
            eprintln!("could not keep the settings in {}: {error}", self.settings_path().display());
        }
    }
}

// ---------- colours ----------

static COLOURBLIND: AtomicBool = AtomicBool::new(false);

pub(crate) fn colourblind() -> bool {
    COLOURBLIND.load(Ordering::Relaxed)
}

//the colour of what is good for the army: babies waiting to be picked up, health. blue instead of green for those who can't tell
//green from red
pub(crate) fn pickup_color() -> Color {
    if colourblind() { Color::new(0.35, 0.65, 1.0, 1.0) } else { GREEN }
}

//the colour of a wound: units redden as they are hurt, health bars empty towards it. orange instead of red when colours are changed
pub(crate) fn wound_color() -> Color {
    if colourblind() { Color::new(1.0, 0.62, 0.05, 1.0) } else { RED }
}

//the colour of the warning on the ground and of what is after the army, bright yellow when colours are changed
pub(crate) fn danger_color(alpha: f32) -> Color {
    if colourblind() { Color::new(1.0, 0.95, 0.2, alpha) } else { Color::new(1.0, 0.3, 0.2, alpha) }
}
