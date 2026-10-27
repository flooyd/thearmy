//the Insane ladder: the tiers past Brutal, the rules each adds, how they are opened and kept, and how a guest is told the one the host plays on

use super::*;

#[test]
fn the_tiers_past_brutal_have_names_and_words_that_are_read_back() {
    assert_eq!(Difficulty::Insane(1).name(), "Insane 1");
    assert_eq!(Difficulty::Insane(10).name(), "Insane 10");
    assert_eq!(Difficulty::Insane(3).word(), "Insane3");
    assert_eq!(Difficulty::Brutal.word(), "Brutal");
    for difficulty in DIFFICULTIES {
        assert_eq!(Difficulty::from_name(difficulty.name()), Some(difficulty));
        assert_eq!(Difficulty::from_name(difficulty.word()), Some(difficulty));
        assert!(!difficulty.word().contains(' '), "a word, for the files read a word at a time");
        assert_eq!(Difficulty::from_byte(difficulty.byte()), Some(difficulty));
    }
    assert_eq!(Difficulty::from_name("Insane 11"), None);
    assert_eq!(Difficulty::from_name("Insane"), None);
    assert_eq!(Difficulty::from_byte(DIFFICULTIES.len() as u8), None);
    assert_eq!(DIFFICULTIES.len(), 4 + INSANE_TIERS as usize);
}

#[test]
fn each_tier_is_harder_than_the_one_below_and_worth_more() {
    for pair in DIFFICULTIES.windows(2) {
        let (lower, higher) = (pair[0], pair[1]);
        assert!(higher.pressure() > lower.pressure(), "{} against {}", higher.name(), lower.name());
        assert!(higher.boss_rounds() >= lower.boss_rounds());
        assert!(higher.relic_answer() >= lower.relic_answer() && higher.relic_answer() <= INSANE_RELIC_ANSWER_MOST);
        assert!(higher.score_factor() > lower.score_factor());
        assert!(higher.affix_chance() >= lower.affix_chance());
    }
    assert!(Difficulty::Insane(1).boss_rounds() > Difficulty::Brutal.boss_rounds(), "the bosses are further on from the first tier");
}

#[test]
fn a_tier_has_its_own_rule_and_every_one_below_it() {
    for difficulty in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard, Difficulty::Brutal] {
        assert!(difficulty.mutators().is_empty(), "{} has none", difficulty.name());
    }
    for tier in 1..=INSANE_TIERS {
        let difficulty = Difficulty::Insane(tier);
        let rules = difficulty.mutators();
        assert_eq!(rules.len(), tier as usize);
        assert_eq!(rules.last().map(|rule| rule.tier()), Some(tier), "the newest is the tier's own");
        assert_eq!(difficulty.blurb(), MUTATORS[tier as usize - 1].blurb());
        assert!(difficulty.has(MUTATORS[0]) && (tier == INSANE_TIERS || !difficulty.has(MUTATORS[tier as usize])));
    }
    for (i, rule) in MUTATORS.iter().enumerate() {
        assert_eq!(rule.tier() as usize, i + 1);
        assert!(!rule.name().is_empty() && !rule.what().is_empty() && !rule.blurb().is_empty());
    }
    assert!(Difficulty::Insane(10).has(Mutator::LastArmy) && !Difficulty::Insane(9).has(Mutator::LastArmy));
}

#[test]
fn slaying_the_sovereign_opens_the_next_tier_and_only_the_open_ones_are_offered() {
    assert_eq!(Difficulty::Hard.ladder_opens(), 0);
    assert_eq!(Difficulty::Brutal.ladder_opens(), 1);
    assert_eq!(Difficulty::Insane(4).ladder_opens(), 5);
    assert_eq!(Difficulty::Insane(INSANE_TIERS).ladder_opens(), INSANE_TIERS, "the top is as far as it goes");
    assert_eq!(Difficulty::choices(0), DIFFICULTIES[..4].to_vec(), "shut, the ladder offers nothing past Brutal");
    assert_eq!(Difficulty::choices(3).last(), Some(&Difficulty::Insane(3)));
    assert_eq!(Difficulty::choices(INSANE_TIERS).len(), DIFFICULTIES.len());
    //a setting that names a tier that isn't open is brought down to the highest that is
    assert_eq!(Difficulty::Insane(6).within(2), Difficulty::Insane(2));
    assert_eq!(Difficulty::Insane(6).within(0), Difficulty::Brutal);
    assert_eq!(Difficulty::Insane(2).within(5), Difficulty::Insane(2));
    assert_eq!(Difficulty::Easy.within(0), Difficulty::Easy);
}

#[test]
fn a_tier_is_kept_in_the_settings_and_a_hand_written_name_with_a_space_is_read() {
    let settings = Settings { difficulty: Difficulty::Insane(7), ..Settings::default() };
    assert!(settings.to_text().contains("difficulty Insane7\n"));
    assert_eq!(Settings::from_text(&settings.to_text()).difficulty, Difficulty::Insane(7));
    assert_eq!(Settings::from_text("difficulty Insane 3\n").difficulty, Difficulty::Insane(3));
    assert_eq!(Settings::from_text("difficulty Insane 30\n").difficulty, Difficulty::Normal);
}

#[test]
fn the_best_runs_keep_their_tier_and_show_how_far_the_ladder_was_opened() {
    let run = |difficulty, citadels| Record { score: 1000 + citadels as u64, difficulty, bosses: 3, peak_army: 900, seconds: 600.0, citadels };
    let mut scores = HighScores::default();
    scores.add(run(Difficulty::Insane(4), 0));
    scores.add(run(Difficulty::Hard, 2));
    assert_eq!(HighScores::from_text(&scores.to_text()), scores, "a tier is one word in the file");
    assert_eq!(scores.ladder(), 0, "the Sovereign slain on Hard opens nothing, and the run on Insane 4 never slew it");
    scores.add(run(Difficulty::Brutal, 1));
    assert_eq!(scores.ladder(), 1);
    scores.add(run(Difficulty::Insane(4), 2));
    assert_eq!(scores.ladder(), 5);
}

#[test]
fn how_far_the_ladder_is_open_is_kept_in_a_file_beside_the_saves() {
    let dir = std::env::temp_dir().join(format!("the_army_ladder_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let saves = Saves::in_dir(&dir);
    assert_eq!(saves.load_ladder(), 0, "shut until it has been opened");
    assert!(saves.ladder_path().starts_with(&dir));
    saves.save_ladder(3);
    assert_eq!(saves.load_ladder(), 3);
    std::fs::write(saves.ladder_path(), "insane 99\n").unwrap();
    assert_eq!(saves.load_ladder(), INSANE_TIERS);
    std::fs::write(saves.ladder_path(), [0xff, 0xfe, 0x00]).unwrap();
    assert_eq!(saves.load_ladder(), 0, "a file that is rubbish is a shut ladder, not a crash");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_step_reports_the_difficulty_it_was_played_on() {
    let mut sim = Sim::from_snapshot(new_run(), Rules { army_limit: DEFAULT_ARMY_LIMIT, difficulty: Difficulty::Insane(5) });
    let step = sim.step(Vec::new(), Pace::steady(1.0 / 60.0), Rules { army_limit: DEFAULT_ARMY_LIMIT, difficulty: Difficulty::Insane(5) });
    assert_eq!(step.report.difficulty, Difficulty::Insane(5));
}

#[test]
fn a_guest_can_see_the_difficulty_but_not_change_it() {
    let mut menu = Pause::new(None, Vec2::ZERO);
    menu.mode = MenuMode::Guest;
    menu.page = Page::Settings;
    menu.selected = page_rows(Page::Settings).iter().position(|&act| act == Act::Difficulty).unwrap();
    let mut settings = Settings { difficulty: Difficulty::Hard, ..Settings::default() };
    for input in [MenuInput { left: true, ..MenuInput::default() }, MenuInput { right: true, ..MenuInput::default() }, MenuInput { enter: true, ..MenuInput::default() }] {
        assert!(matches!(menu.update(&mut settings, &input, Vec2::new(1280.0, 720.0), 1.0 / 60.0), PauseEvent::Nothing));
        assert_eq!(settings.difficulty, Difficulty::Hard);
    }
    assert!(menu.locked(Act::Difficulty) && !menu.locked(Act::Master));
    assert!(explain_in(Act::Difficulty, &settings, MenuMode::Guest).contains("host"));
    //the host and a player alone change it as before
    menu.mode = MenuMode::Hosting;
    menu.update(&mut settings, &MenuInput { right: true, ..MenuInput::default() }, Vec2::new(1280.0, 720.0), 1.0 / 60.0);
    assert_eq!(settings.difficulty, Difficulty::Brutal);
}
