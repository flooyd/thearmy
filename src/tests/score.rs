//what a run is worth, and the best runs kept to beat

use super::*;

fn run(destroyed: usize, bosses: usize, peak_army: usize, seconds: f32) -> Run {
    Run { destroyed, lost: 0, peak_army, bosses, nests: 0, villages: 0, seconds, citadels: 0 }
}

fn record(score: u64) -> Record {
    Record { score, difficulty: Difficulty::Normal, bosses: 1, peak_army: 100, seconds: 60.0, citadels: 0 }
}

#[test]
fn a_run_that_did_nothing_is_worth_nothing_and_every_part_of_a_run_adds_to_what_it_is_worth() {
    assert_eq!(score_of(&run(0, 0, 0, 0.0), Difficulty::Normal), 0);
    let base = run(100, 2, 300, 120.0);
    let worth = score_of(&base, Difficulty::Normal);
    assert_eq!(worth, 100 * SCORE_PER_KILL + 2 * SCORE_PER_BOSS + 300 * SCORE_PER_UNIT + (120.0 * SCORE_PER_SECOND) as u64);
    let more = |change: fn(&mut Run)| {
        let mut other = base.clone();
        change(&mut other);
        score_of(&other, Difficulty::Normal)
    };
    assert!(more(|run| run.destroyed += 1) > worth);
    assert!(more(|run| run.bosses += 1) > worth);
    assert!(more(|run| run.peak_army += 1) > worth);
    assert!(more(|run| run.seconds += 5.0) > worth);
    assert_eq!(more(|run| run.nests += 1), worth + SCORE_PER_NEST);
    assert_eq!(more(|run| run.villages += 1), worth + SCORE_PER_VILLAGE);
    assert_eq!(more(|run| run.citadels += 1), worth + SCORE_PER_CITADEL);
    //the units that died are not taken off: it is what the army grew to that counts
    assert_eq!(more(|run| run.lost += 500), worth);
    //a boss is worth a lot of ordinary enemies, and a citadel a lot of bosses
    assert!(SCORE_PER_BOSS > 50 * SCORE_PER_KILL && SCORE_PER_CITADEL > 2 * SCORE_PER_BOSS);
}

#[test]
fn the_same_run_is_worth_more_on_a_harder_difficulty() {
    let finished = run(400, 3, 800, 900.0);
    let scores: Vec<u64> = DIFFICULTIES.iter().map(|&difficulty| score_of(&finished, difficulty)).collect();
    assert!(scores.windows(2).all(|pair| pair[0] < pair[1]), "{scores:?}");
    let normal = score_of(&finished, Difficulty::Normal) as f32;
    for difficulty in DIFFICULTIES {
        let expected = normal * difficulty.score_factor();
        assert!((score_of(&finished, difficulty) as f32 - expected).abs() <= 1.0, "{}", difficulty.name());
    }
}

#[test]
fn a_negative_time_or_a_huge_run_cannot_break_the_score() {
    assert_eq!(score_of(&run(0, 0, 0, -50.0), Difficulty::Hard), 0);
    let huge = run(50_000_000, 400, 1_000_000, 1.0e7);
    let worth = score_of(&huge, Difficulty::Brutal);
    assert!(worth > 600_000_000 && worth < u64::MAX / 2, "{worth}");
}

#[test]
fn a_record_keeps_what_the_run_was_and_what_it_was_worth() {
    let mut finished = run(250, 4, 700, 321.0);
    finished.citadels = 1;
    let record = Record::of(&finished, Difficulty::Hard);
    assert_eq!(record, Record { score: score_of(&finished, Difficulty::Hard), difficulty: Difficulty::Hard, bosses: 4, peak_army: 700, seconds: 321.0, citadels: 1 });
}

#[test]
fn the_best_runs_are_kept_in_order_with_the_best_first_and_only_so_many() {
    let mut scores = HighScores::default();
    assert_eq!(scores.best(), 0);
    assert_eq!(scores.add(record(500)), Some(0));
    assert_eq!(scores.add(record(900)), Some(0), "a better run goes first");
    assert_eq!(scores.add(record(700)), Some(1));
    assert_eq!(scores.best(), 900);
    assert_eq!(scores.records.iter().map(|record| record.score).collect::<Vec<_>>(), vec![900, 700, 500]);
    //a tie goes after the one that was there first
    assert_eq!(scores.add(record(700)), Some(2));
    assert_eq!(scores.records.iter().map(|record| record.score).collect::<Vec<_>>(), vec![900, 700, 700, 500]);
    //a run worth nothing is not kept
    assert_eq!(scores.add(record(0)), None);
    //only the best few stay, and a run that is not good enough is turned away
    for score in [800, 600, 400, 300] {
        scores.add(record(score));
    }
    assert_eq!(scores.records.len(), HIGH_SCORES_KEPT);
    assert_eq!(scores.records.iter().map(|record| record.score).collect::<Vec<_>>(), vec![900, 800, 700, 700, 600]);
    assert_eq!(scores.add(record(100)), None);
    assert_eq!(scores.add(record(600)), None, "level with the last is not enough");
    assert_eq!(scores.add(record(650)), Some(4));
    assert_eq!(scores.records.len(), HIGH_SCORES_KEPT);
}

#[test]
fn the_best_runs_are_written_as_text_and_read_back_as_they_were() {
    let mut scores = HighScores::default();
    for (score, difficulty, bosses, peak_army, seconds, citadels) in [(12_345, Difficulty::Brutal, 5, 4000, 1800.0, 1), (9000, Difficulty::Easy, 2, 150, 420.0, 0)] {
        scores.add(Record { score, difficulty, bosses, peak_army, seconds, citadels });
    }
    let text = scores.to_text();
    assert_eq!(text.lines().count(), 2);
    assert_eq!(HighScores::from_text(&text), scores);
    assert_eq!(HighScores::from_text(""), HighScores::default());
}

#[test]
fn a_damaged_or_edited_file_gives_what_it_can() {
    let text = "5000 Normal 1 100 60 0\nnot a line at all\n9000 Hard 2 200 90 0\n7000 Nonsense 1 1 1 0\n3000 Easy 1 50 30\n6000 Easy x 5 5 0\n8000 Normal 1 100 inf 0\n\n2000 Brutal 0 20 10 0 trailing words\n";
    let scores = HighScores::from_text(text);
    let kept: Vec<u64> = scores.records.iter().map(|record| record.score).collect();
    assert_eq!(kept, vec![9000, 5000, 2000], "the lines that make sense, in order");
    assert_eq!(scores.records[0].difficulty, Difficulty::Hard);
    //(no more than are kept, however long the file)
    let long: String = (1..=50).map(|score| format!("{} Normal 0 10 10 0\n", score * 10)).collect();
    assert_eq!(HighScores::from_text(&long).records.len(), HIGH_SCORES_KEPT);
    assert_eq!(HighScores::from_text(&long).best(), 500);
}

#[test]
fn the_best_runs_are_kept_in_a_file_beside_the_saves_and_a_missing_file_is_no_runs() {
    let dir = std::env::temp_dir().join(format!("the_army_scores_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let saves = Saves::in_dir(&dir);
    assert_eq!(saves.load_high_scores(), HighScores::default(), "nothing yet");
    assert!(saves.high_scores_path().starts_with(&dir));
    let mut scores = HighScores::default();
    scores.add(record(4321));
    scores.add(record(1234));
    saves.save_high_scores(&scores);
    assert_eq!(saves.load_high_scores(), scores);
    assert_eq!(saves.load_high_scores().best(), 4321);
    //a file that is rubbish is no runs, not a crash
    std::fs::write(saves.high_scores_path(), [0xff, 0xfe, 0x00, 0x41]).unwrap();
    assert_eq!(saves.load_high_scores(), HighScores::default());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_places_are_read_out_as_ordinals() {
    assert_eq!([1, 2, 3, 4, 11, 12, 13, 21, 22, 23, 101, 111].map(ordinal), ["1ST", "2ND", "3RD", "4TH", "11TH", "12TH", "13TH", "21ST", "22ND", "23RD", "101ST", "111TH"]);
}
