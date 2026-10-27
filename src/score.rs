//! the score of a run, and the best runs kept beside the saves and the settings, to beat

use crate::prelude::*;

//what a run is worth: the enemies destroyed, the bosses slain, the nests destroyed and villages freed, the largest army it had and the
//time it lasted, all multiplied by what the difficulty it was played on is worth
pub(crate) fn score_of(run: &Run, difficulty: Difficulty) -> u64 {
    let base = run.destroyed as u64 * SCORE_PER_KILL
        + run.bosses as u64 * SCORE_PER_BOSS
        + run.citadels as u64 * SCORE_PER_CITADEL
        + run.nests as u64 * SCORE_PER_NEST
        + run.villages as u64 * SCORE_PER_VILLAGE
        + run.peak_army as u64 * SCORE_PER_UNIT;
    ((base as f32 + run.seconds.max(0.0) * SCORE_PER_SECOND) * difficulty.score_factor()).round() as u64
}

//a run that is over, as it is kept
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Record {
    pub(crate) score: u64,
    pub(crate) difficulty: Difficulty,
    pub(crate) bosses: usize,
    pub(crate) peak_army: usize,
    pub(crate) seconds: f32,
    pub(crate) citadels: usize,
}

impl Record {
    pub(crate) fn of(run: &Run, difficulty: Difficulty) -> Record {
        Record { score: score_of(run, difficulty), difficulty, bosses: run.bosses, peak_army: run.peak_army, seconds: run.seconds, citadels: run.citadels }
    }
}

//the best runs, the best first
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct HighScores {
    pub(crate) records: Vec<Record>,
}

impl HighScores {
    //the score to beat: none for no runs yet
    pub(crate) fn best(&self) -> u64 {
        self.records.first().map_or(0, |record| record.score)
    }

    //put a run among the best. returns its place (0 for the best of all) if it made it, and none if it was not good enough, or not
    //worth anything. a run that ties one already there goes after it
    pub(crate) fn add(&mut self, record: Record) -> Option<usize> {
        if record.score == 0 {
            return None;
        }
        let place = self.records.iter().position(|other| record.score > other.score).unwrap_or(self.records.len());
        if place >= HIGH_SCORES_KEPT {
            return None;
        }
        self.records.insert(place, record);
        self.records.truncate(HIGH_SCORES_KEPT);
        Some(place)
    }

    //a line of numbers and a word for each run
    pub(crate) fn to_text(&self) -> String {
        let mut text = String::new();
        for record in &self.records {
            text.push_str(&format!("{} {} {} {} {:.0} {}\n", record.score, record.difficulty.word(), record.bosses, record.peak_army, record.seconds, record.citadels));
        }
        text
    }

    //how far up the Insane ladder the runs kept show was opened: a run in which the Sovereign fell opened the tier above the one it was played on
    //(which counts the runs from before the ladder was kept)
    pub(crate) fn ladder(&self) -> u8 {
        self.records.iter().filter(|record| record.citadels > 0).map(|record| record.difficulty.ladder_opens()).max().unwrap_or(0)
    }

    //the runs read back. a line that isn't understood is left out, and the rest are put in order, so a file that was edited or damaged
    //still gives what it can
    pub(crate) fn from_text(text: &str) -> HighScores {
        let mut scores = HighScores::default();
        for line in text.lines() {
            let mut words = line.split_ascii_whitespace();
            let (Some(score), Some(difficulty), Some(bosses), Some(peak_army), Some(seconds), Some(citadels)) = (words.next(), words.next(), words.next(), words.next(), words.next(), words.next()) else {
                continue;
            };
            let (Ok(score), Some(difficulty), Ok(bosses), Ok(peak_army), Ok(seconds), Ok(citadels)) =
                (score.parse::<u64>(), Difficulty::from_name(difficulty), bosses.parse::<usize>(), peak_army.parse::<usize>(), seconds.parse::<f32>(), citadels.parse::<usize>())
            else {
                continue;
            };
            if seconds.is_finite() {
                scores.add(Record { score, difficulty, bosses, peak_army, seconds: seconds.max(0.0), citadels });
            }
        }
        scores
    }
}

impl Saves {
    pub(crate) fn high_scores_path(&self) -> std::path::PathBuf {
        self.folder().join("highscores.txt")
    }

    //the best runs so far, none if there are none yet
    pub(crate) fn load_high_scores(&self) -> HighScores {
        std::fs::read_to_string(self.high_scores_path()).map(|text| HighScores::from_text(&text)).unwrap_or_default()
    }

    pub(crate) fn ladder_path(&self) -> std::path::PathBuf {
        self.folder().join("ladder.txt")
    }

    //how far up the Insane ladder the player has opened ("insane 3"), 0 if there is no file of it yet
    pub(crate) fn load_ladder(&self) -> u8 {
        let text = std::fs::read_to_string(self.ladder_path()).unwrap_or_default();
        text.lines().filter_map(|line| line.strip_prefix("insane ")).filter_map(|tier| tier.trim().parse::<u8>().ok()).map(|tier| tier.min(INSANE_TIERS)).max().unwrap_or(0)
    }

    pub(crate) fn save_ladder(&self, open: u8) {
        let result = std::fs::create_dir_all(self.folder()).and_then(|()| std::fs::write(self.ladder_path(), format!("insane {open}\n")));
        if let Err(error) = result {
            eprintln!("could not keep how far the ladder is open in {}: {error}", self.ladder_path().display());
        }
    }

    pub(crate) fn save_high_scores(&self, scores: &HighScores) {
        let result = std::fs::create_dir_all(self.folder()).and_then(|()| std::fs::write(self.high_scores_path(), scores.to_text()));
        if let Err(error) = result {
            eprintln!("could not keep the high scores in {}: {error}", self.high_scores_path().display());
        }
    }
}
