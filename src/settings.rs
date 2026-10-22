//! what the player can change: how loud things are, how much the screen shakes, the colours for those who can't tell red from
//! green, how hard the enemies are, which formation the army keeps and whether hints are shown. kept in a small text file beside the
//! saves and read again the next time the game starts

use crate::prelude::*;
use std::sync::atomic::AtomicBool;

//how much harder than normal the enemies are made: they are built for an army this many times as big as the player's
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Difficulty {
    Easy,
    Normal,
    Hard,
    Brutal,
}

pub(crate) const DIFFICULTIES: [Difficulty; 4] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard, Difficulty::Brutal];

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
            Difficulty::Hard => 1.5,
            Difficulty::Brutal => 2.2,
        }
    }

    pub(crate) fn blurb(self) -> &'static str {
        match self {
            Difficulty::Easy => "enemies are made for an army 30% smaller",
            Difficulty::Normal => "as it was meant to be played",
            Difficulty::Hard => "enemies are made for an army 50% bigger",
            Difficulty::Brutal => "enemies are made for an army more than twice as big",
        }
    }

    fn from_name(name: &str) -> Option<Difficulty> {
        DIFFICULTIES.into_iter().find(|difficulty| difficulty.name() == name)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Settings {
    pub(crate) master: f32,  //0 to 1, how loud everything is
    pub(crate) effects: f32, //0 to 1, how loud the sounds are next to the music
    pub(crate) music: f32,   //0 to 1, how loud the music is
    pub(crate) shake: f32,   //0 to 1, how much the screen shakes and the camera jumps in for big moments
    pub(crate) fullscreen: bool, //whether the game starts filling the screen
    pub(crate) colourblind: bool,
    pub(crate) difficulty: Difficulty,
    pub(crate) hints: bool,
    pub(crate) formation: Formation,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { master: 1.0, effects: 1.0, music: 1.0, shake: 1.0, fullscreen: true, colourblind: false, difficulty: Difficulty::Normal, hints: true, formation: Formation::Free }
    }
}

impl Settings {
    //the settings as lines of text, a name and a value each
    pub(crate) fn to_text(&self) -> String {
        format!(
            "master {:.2}\neffects {:.2}\nmusic {:.2}\nshake {:.2}\nfullscreen {}\ncolourblind {}\ndifficulty {}\nhints {}\nformation {}\n",
            self.master,
            self.effects,
            self.music,
            self.shake,
            if self.fullscreen { "yes" } else { "no" },
            if self.colourblind { "yes" } else { "no" },
            self.difficulty.name(),
            if self.hints { "yes" } else { "no" },
            self.formation.name(),
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
                _ => {}
            }
        }
        settings
    }

    //put into effect what is not read from the settings each time it is used: the colours
    pub(crate) fn apply(&self) {
        COLOURBLIND.store(self.colourblind, Ordering::Relaxed);
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
