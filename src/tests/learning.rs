//step 16: the elites' affixes, how groups of enemies fight, the tide of lulls and surges, what killed the army and the bestiary

use super::*;

fn elite(name: &str, at: Vec2) -> Enemy {
    let mut enemy = kind_at(name, at);
    enemy.make_elite(Modifier::Swift);
    enemy
}

fn with(name: &str, at: Vec2, affix: Affix) -> Enemy {
    let mut enemy = elite(name, at);
    enemy.add_affix(affix);
    enemy
}

#[test]
fn an_elite_has_more_affixes_the_harder_the_game_chosen_once() {
    let mut none = elite("Grunt", MIDDLE);
    none.roll_affixes(0.0);
    assert!(none.affixes.rolled() && none.affixes.list().count() == 0);
    let mut most = elite("Grunt", MIDDLE);
    most.roll_affixes(1.0);
    assert_eq!(most.affixes.list().count(), 2, "two more besides the first");
    assert!(!most.affixes.has(Affix::Swift), "never the one it has already");
    let before = most.affixes;
    most.roll_affixes(1.0);
    assert_eq!(most.affixes, before, "chosen once");
    let mut plain = kind_at("Grunt", MIDDLE);
    plain.roll_affixes(1.0);
    assert_eq!(plain.affixes.list().count(), 0, "only an elite");
    let mut boss = Enemy::boss(&BOSSES[0], 200, 0, MIDDLE);
    boss.roll_affixes(1.0);
    assert_eq!(boss.affixes.list().count(), 0);
    assert!(Difficulty::Easy.affix_chance() == 0.0 && Difficulty::Brutal.affix_chance() > Difficulty::Normal.affix_chance());
}

#[test]
fn an_affix_changes_its_numbers_and_its_name_and_its_marks() {
    let plain = elite("Grunt", MIDDLE);
    let shielded = with("Grunt", MIDDLE, Affix::Shielded);
    assert!(shielded.max_shield > plain.max_shield && shielded.shield == shielded.max_shield);
    let mut both = with("Grunt", MIDDLE, Affix::Vampiric);
    both.add_affix(Affix::Thorned);
    assert_eq!(both.title(), "Swift Vampiric Thorned Grunt");
    let marks: Vec<&str> = marks_of(&both).into_iter().map(|(name, _)| name).collect();
    assert!(marks.contains(&"Swift") && marks.contains(&"Vampiric") && marks.contains(&"Thorned"));
    assert!(both.has(Affix::Swift) && both.has(Affix::Thorned) && !both.has(Affix::Blazing));
}

#[test]
fn affixes_are_kept_in_a_save_and_sent_and_an_old_save_has_them_chosen_later() {
    let mut enemy = with("Brute", MIDDLE, Affix::Magnetic);
    enemy.affixes = enemy.affixes.chosen();
    let text = text_of(&enemy);
    let back: Enemy = from_text(text.as_bytes()).expect("it reads");
    assert_eq!(back.affixes, enemy.affixes);
    //(a save from before affixes has no word for them)
    let old = text.replace(" +Magnetic ", " ");
    assert_ne!(old, text);
    let back: Enemy = from_text(old.as_bytes()).expect("an old one reads");
    assert!(!back.affixes.rolled() && back.affixes.list().count() == 0);
}

//a fight of one enemy among units touching it, for a moment
fn fight_a_moment(enemy: Enemy, units: &mut Vec<Unit>) -> Enemy {
    let mut enemies = vec![enemy];
    fight(&mut enemies, units, &mut Vec::new(), 0.1, &Upgrades::default(), &mut Effects::default());
    enemies.remove(0)
}

#[test]
fn a_vampire_heals_from_the_harm_it_does_and_thorns_hurt_the_units_that_hurt_it() {
    let hurt_first = |mut enemy: Enemy| {
        enemy.health = enemy.max_health * 0.5;
        enemy.attack = 500.0;
        enemy
    };
    //(the same enemy, with the affix and without it)
    let bitten = hurt_first(with("Brute", MIDDLE, Affix::Vampiric));
    let mut unbitten = bitten.clone();
    unbitten.affixes = Affixes::default();
    let mut units = vec![unit_on(MIDDLE); 3];
    let plain = fight_a_moment(unbitten, &mut units);
    let mut units = vec![unit_on(MIDDLE); 3];
    let vampire = fight_a_moment(bitten, &mut units);
    assert!(vampire.health > plain.health, "{} against {}", vampire.health, plain.health);
    let mut thorned_units = vec![unit_on(MIDDLE); 3];
    let mut thorny = with("Brute", MIDDLE, Affix::Thorned);
    thorny.attack = 0.0;
    fight_a_moment(thorny, &mut thorned_units);
    assert!(thorned_units.iter().all(|unit| unit.health < UNIT_HEALTH), "they are hurt for hurting it");
}

#[test]
fn a_blazing_aura_burns_and_a_frigid_one_slows_the_units_near_it() {
    let near = MIDDLE + Vec2::new(kind_at("Brute", MIDDLE).size / 2.0 + AURA_REACH * 0.5, 0.0);
    let mut units = vec![unit_on(near), unit_on(MIDDLE + Vec2::new(2000.0, 0.0))];
    fight_a_moment(with("Brute", MIDDLE, Affix::Blazing), &mut units);
    assert!(units[0].health < UNIT_HEALTH && units[1].health == UNIT_HEALTH);
    let mut units = vec![unit_on(near)];
    fight_a_moment(with("Brute", MIDDLE, Affix::Frigid), &mut units);
    assert!(units[0].chilled > 0.0);
}

#[test]
fn now_and_then_a_teleporting_one_jumps_a_magnetic_one_drags_and_a_conjuring_one_calls_up_imps() {
    let player = MIDDLE + Vec2::new(1500.0, 0.0);
    let units = vec![unit_on(player)];
    for affix in [Affix::Teleporting, Affix::Magnetic, Affix::Conjuring, Affix::Regenerating] {
        let mut enemy = with("Brute", MIDDLE, affix);
        enemy.chasing = true;
        enemy.knack = 0.0;
        enemy.health = enemy.max_health * 0.5;
        let mut enemies = vec![enemy];
        let mut fx = Effects::default();
        enemy_abilities(&mut enemies, &units, player, 0.5, &mut fx);
        match affix {
            Affix::Teleporting => assert!((enemies[0].position.x - MIDDLE.x - TELEPORT_STEP).abs() < 1.0, "it jumps at the player"),
            Affix::Magnetic => assert_eq!(fx.pulls.len(), 1),
            Affix::Conjuring => assert_eq!(enemies.len(), 1 + CONJURE_COUNT),
            _ => assert!(enemies[0].health > enemies[0].max_health * 0.5),
        }
    }
}

//one pass of the enemies' walking, with one player and their army
fn walk_among(enemies: &mut [Enemy], player: Vec2, units: &[Unit], seconds: f32) {
    let field = FlowField::new();
    let prey = [Prey { position: player, units, team: units.len(), pressure: units.len(), boost: 1.0, cap: DEFAULT_ARMY_LIMIT, field: &field }];
    let mut fx = Effects::default();
    for _ in 0..(seconds * 60.0) as usize {
        patrol_enemies_among(enemies, &prey, 1.0 / 60.0, &Walls::default(), &mut fx);
    }
}

#[test]
fn a_shooter_backs_away_from_the_army_and_a_healer_keeps_out_of_its_reach() {
    let mut spitter = kind_at("Spitter", MIDDLE);
    spitter.chasing = true;
    let player = MIDDLE + Vec2::new(spitter.range * KITE_SHARE * 0.5, 0.0);
    let mut healer = kind_at("Healer", MIDDLE + Vec2::new(0.0, 300.0));
    healer.chasing = false;
    let mut enemies = vec![spitter, healer, kind_at("Grunt", MIDDLE + Vec2::new(0.0, 900.0))];
    let before: Vec<f32> = enemies.iter().map(|enemy| enemy.position.distance(player)).collect();
    walk_among(&mut enemies, player, &[], 1.0);
    assert!(enemies[0].position.distance(player) > before[0] + 30.0, "the shooter backs off");
    assert!(enemies[1].position.distance(player) > before[1] + 30.0, "the healer keeps away");
}

#[test]
fn an_elite_goes_for_a_champion_near_it_and_a_pack_comes_round_the_side() {
    let player = MIDDLE;
    let champion = MIDDLE + Vec2::new(600.0, 600.0);
    let units = vec![champion_unit(champion_named("Pyromancer").unwrap(), champion), unit_on(player)];
    let mut hunter = elite("Grunt", champion + Vec2::new(300.0, 0.0));
    hunter.chasing = true;
    hunter.aggro = 5000.0;
    let mut swarmer = kind_at("Swarmer", MIDDLE + Vec2::new(-1500.0, 0.0));
    swarmer.chasing = true;
    swarmer.aggro = 5000.0;
    swarmer.id = 2;
    let mut enemies = vec![hunter, swarmer];
    let start = enemies[0].position;
    walk_among(&mut enemies, player, &units, 0.5);
    assert!(enemies[0].position.distance(champion) < start.distance(champion), "it closes on the champion");
    assert!(enemies[0].position.distance(champion) < enemies[0].position.distance(player));
    assert!((enemies[1].position.y - MIDDLE.y).abs() > 20.0, "the pack swings out to the side: {}", enemies[1].position.y - MIDDLE.y);
}

#[test]
fn the_tide_is_a_lull_then_the_usual_then_a_surge_and_no_surge_against_a_boss() {
    assert_eq!(tide_at(5.0, false), Tide::Lull);
    assert_eq!(tide_at(50.0, false), Tide::Usual);
    assert_eq!(tide_at(TIDE_PERIOD - 1.0, false), Tide::Surge);
    assert_eq!(tide_at(TIDE_PERIOD - 1.0, true), Tide::Usual);
    assert_eq!(tide_at(TIDE_PERIOD + 5.0, false), Tide::Lull, "and round again");
    assert!(Tide::Lull.crowd() < 1.0 && Tide::Surge.crowd() > 1.0 && Tide::Surge.pressure() > Tide::Lull.pressure());
    let mut enemies = Vec::new();
    respawn_enemies_up_to(&mut enemies, &[(MIDDLE, 100)], 40);
    let first = enemies.len();
    respawn_enemies_up_to(&mut enemies, &[(MIDDLE, 100)], 80);
    assert!(first >= 40 && enemies.len() >= 80 && enemies.len() > first);
}

#[test]
fn every_death_is_put_down_to_what_did_it_and_the_worst_are_listed() {
    assert_eq!(cause_label(cause_of("Grunt")), "Grunt");
    assert_eq!(cause_label(cause_of("ECLIPSE SWEEP")), "Eclipse Sweep");
    assert_eq!(cause_label(cause_of("The Eclipse")), "The Eclipse");
    assert_eq!((cause_of("nothing at all"), cause_label(0)), (0, "unknown".to_owned()));
    //a boss's attack, an enemy's touch
    let mut units = vec![unit_on(MIDDLE), unit_on(MIDDLE + Vec2::new(5000.0, 0.0))];
    let mut hazards = vec![hazard_at(MIDDLE, 100.0, 0.0, UNIT_HEALTH * 3.0, 10).circle(BossCircle::Nova)];
    let mut fx = Effects::default();
    update_hazards(&mut hazards, &mut units, &Upgrades::default(), 0.016, &mut fx);
    assert_eq!(units[0].bane, cause_of("NOVA"));
    let mut brute = kind_at("Brute", Vec2::new(units[1].x, units[1].y));
    brute.attack = UNIT_HEALTH * 100.0;
    let mut enemies = vec![brute];
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &Upgrades::default(), &mut fx);
    assert_eq!(units[1].bane, cause_of("Brute"));
    assert_eq!(bury_units(&mut units, &mut fx), 2);
    let mut killers = Vec::new();
    tally_killers(&mut killers, &fx.banes);
    tally_killers(&mut killers, &[cause_of("Brute")]);
    assert_eq!(worst_killers(&killers, 6), vec![("Brute".to_owned(), 2), ("Nova".to_owned(), 1)]);
}

#[test]
fn the_bestiary_has_a_line_for_everything_and_keeps_what_has_been_met() {
    let all = lessons();
    assert_eq!(all.len(), ENEMY_KINDS.len() + BOSSES.len() + 1 + CIRCLES.len() + REACTIONS.len() + AFFIXES.len());
    assert!(all.iter().all(|lesson| !lesson.lesson.is_empty() && !lesson.shown.is_empty()), "every line says something");
    meet(["Sapper", "LEAP"].into_iter());
    assert!(has_met("Sapper") && has_met("LEAP"));
    let pages = bestiary_pages();
    assert!(pages * BESTIARY_PER_PAGE >= all.len());
    let at = bestiary_page();
    turn_bestiary(-1);
    assert_eq!(bestiary_page(), (at + pages - 1) % pages, "it turns back round");
    turn_bestiary(1);
}
