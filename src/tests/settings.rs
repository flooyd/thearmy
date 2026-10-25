//the settings the player can change: kept in a file, read back kindly, and the colours for those who can't tell red from green

use super::*;

fn changed() -> Settings {
    Settings { master: 0.35, effects: 0.8, music: 0.0, shake: 0.5, fullscreen: false, colourblind: true, difficulty: Difficulty::Hard, hints: false, formation: Formation::Wedge, unit_shape: UnitShape::Circles, simple_shapes: true, army_limit: 32000, player_name: "Ann Marie".to_owned(), host_port: 50000, multiplayer_role: Role::Local }
}

#[test]
fn settings_read_back_as_they_were_kept() {
    assert_eq!(Settings::from_text(&changed().to_text()), changed());
    assert_eq!(Settings::from_text(&Settings::default().to_text()), Settings::default());
    assert_eq!(Settings::from_text(""), Settings::default());
}

#[test]
fn a_settings_file_that_is_damaged_or_from_another_version_gives_what_it_can() {
    let text = "master 0.5\nbanana split\nshake 7\nmusic loud\nweather rainy\n\ndifficulty Impossible\nformation Phalanx\ncolourblind maybe\nhints no\n  effects   0.25  \n";
    let settings = Settings::from_text(text);
    assert_eq!(settings.master, 0.5);
    assert_eq!(settings.shake, 1.0, "too much is put right");
    assert_eq!(settings.music, Settings::default().music, "what is not a number is left as it was");
    assert_eq!(settings.effects, 0.25);
    assert_eq!(settings.difficulty, Difficulty::Normal);
    assert_eq!(settings.formation, Formation::Ring, "one that is not a formation leaves the usual one");
    assert!(!settings.colourblind && !settings.hints);
}

#[test]
fn the_settings_are_kept_in_the_saves_folder_and_come_back_from_it() {
    let dir = std::env::temp_dir().join(format!("the_army_settings_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let saves = Saves::in_dir(&dir);
    assert_eq!(saves.load_settings(), Settings::default(), "none kept yet");
    saves.save_settings(&changed());
    assert_eq!(saves.load_settings(), changed());
    std::fs::write(saves.settings_path(), "\u{0}\u{1}garbage").unwrap();
    assert_eq!(saves.load_settings(), Settings::default());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_harder_difficulties_make_enemies_for_a_bigger_army() {
    let pressures: Vec<f32> = DIFFICULTIES.iter().map(|difficulty| difficulty.pressure()).collect();
    assert!(pressures.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(Difficulty::Normal.pressure(), 1.0);
    for difficulty in DIFFICULTIES {
        assert!(!difficulty.blurb().is_empty() && !difficulty.name().is_empty());
    }
}

#[test]
fn the_harder_difficulties_have_bosses_further_on_and_answer_the_relics_that_add_damage() {
    let rounds: Vec<u32> = DIFFICULTIES.iter().map(|difficulty| difficulty.boss_rounds()).collect();
    assert!(rounds.windows(2).all(|pair| pair[0] <= pair[1]) && rounds[0] == 0 && rounds[3] > rounds[2] && rounds[2] > 0);
    let answers: Vec<f32> = DIFFICULTIES.iter().map(|difficulty| difficulty.relic_answer()).collect();
    assert_eq!((answers[0], answers[1]), (0.0, 0.0), "the easier ones leave the relics alone");
    assert!(answers[2] > 0.0 && answers[3] > answers[2] && answers[3] < 1.0, "never all of what they add, or they would be worth nothing");
}

#[test]
fn enemies_shrug_off_a_share_of_the_damage_the_relics_add_and_the_relics_still_always_help() {
    let mut upgrades = Upgrades::default();
    assert_eq!(upgrades.hit(), 1.0);
    for _ in 0..8 {
        upgrades.take(Relic::Whetstone);
    }
    upgrades.take(Relic::GlassCannon);
    assert_eq!(upgrades.hit(), upgrades.damage, "nothing is answered by default");
    let mut last = f32::MAX;
    for difficulty in DIFFICULTIES {
        upgrades.foe_answer = difficulty.relic_answer();
        let hit = upgrades.hit();
        assert!(hit <= last && hit > 1.0, "{}: {hit}", difficulty.name());
        last = hit;
        //one more relic is always worth something
        let mut more = upgrades.clone();
        more.take(Relic::Whetstone);
        assert!(more.hit() > hit, "{}", difficulty.name());
    }
    //an army with no damage relics is not weakened by it, and neither is one whose damage is somehow less than it starts with
    let mut plain = Upgrades::default();
    plain.foe_answer = 1.0;
    assert_eq!(plain.hit(), 1.0);
    plain.damage = 0.5;
    assert_eq!(plain.hit(), 0.5);
}

#[test]
fn the_colours_are_the_usual_ones_until_they_are_changed() {
    assert!(!colourblind());
    assert_eq!(pickup_color(), GREEN);
    assert_eq!(wound_color(), RED);
    assert_eq!(danger_color(0.5).a, 0.5);
}

#[test]
fn the_army_limit_is_kept_and_put_right_when_it_is_out_of_range_or_missing() {
    assert_eq!(Settings::default().army_limit, DEFAULT_ARMY_LIMIT);
    //a settings file from before the limit could be set has none, which is the usual one
    assert_eq!(Settings::from_text("master 0.5\nformation Ring\n").army_limit, DEFAULT_ARMY_LIMIT);
    assert_eq!(Settings::from_text("army_limit 30000\n").army_limit, 30000);
    assert_eq!(Settings::from_text("army_limit 100\n").army_limit, MIN_ARMY_LIMIT);
    assert_eq!(Settings::from_text("army_limit 999999999\n").army_limit, MAX_ARMY_LIMIT);
    assert_eq!(Settings::from_text("army_limit lots\n").army_limit, DEFAULT_ARMY_LIMIT);
    assert_eq!(Settings::from_text("army_limit -5\n").army_limit, DEFAULT_ARMY_LIMIT);
}
