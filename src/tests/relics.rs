//the newer relics: rarities, what each does in a fight, the trade-offs, rerolls and the rewards for winning

use super::*;

fn with(relics: &[Relic]) -> Upgrades {
    let mut up = Upgrades::default();
    for &relic in relics {
        up.take(relic);
    }
    up
}

#[test]
fn every_relic_has_a_rarity_and_the_rares_are_few() {
    assert_eq!(RELICS.len(), 22);
    let count = |rarity: Rarity| RELICS.iter().filter(|relic| relic.rarity() == rarity).count();
    assert!(count(Rarity::Common) >= 5 && count(Rarity::Uncommon) >= 5);
    assert_eq!(count(Rarity::Rare), 4);
    assert!(Rarity::Common.weight() > Rarity::Uncommon.weight() && Rarity::Uncommon.weight() > Rarity::Rare.weight());
    assert_eq!(Rarity::Common.weight() / Rarity::Rare.weight(), 6.0);
    for relic in RELICS {
        assert!(!relic.blurb().is_empty() && !relic.name().is_empty());
    }
    //the trade-offs are rare
    assert_eq!(Relic::GlassCannon.rarity(), Rarity::Rare);
    assert_eq!(Relic::Overclock.rarity(), Rarity::Rare);
}

#[test]
fn common_relics_are_offered_more_often_than_rare_ones() {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for _ in 0..4000 {
        for relic in relic_choices() {
            *seen.entry(relic.name()).or_default() += 1;
        }
    }
    let average = |rarity: Rarity| {
        let relics: Vec<Relic> = RELICS.iter().copied().filter(|relic| relic.rarity() == rarity).collect();
        relics.iter().map(|relic| seen.get(relic.name()).copied().unwrap_or(0)).sum::<usize>() as f32 / relics.len() as f32
    };
    let (common, uncommon, rare) = (average(Rarity::Common), average(Rarity::Uncommon), average(Rarity::Rare));
    assert!(common > uncommon * 1.4 && uncommon > rare * 1.4, "{common} {uncommon} {rare}");
    assert!(rare > 20.0, "rare ones do come up: {rare}");
}

#[test]
fn winning_offers_three_different_rare_relics_and_a_reroll_stays_rare() {
    for _ in 0..50 {
        let choice = RelicChoice::rare();
        assert!(choice.options.iter().all(|relic| relic.rarity() == Rarity::Rare));
        assert!(choice.options[0] != choice.options[1] && choice.options[1] != choice.options[2] && choice.options[0] != choice.options[2]);
    }
    let mut choice = RelicChoice::rare();
    choice.reroll();
    assert!(choice.options.iter().all(|relic| relic.rarity() == Rarity::Rare));
}

#[test]
fn a_reroll_brings_other_relics_and_one_is_earned_for_every_three_bosses() {
    let mut changed = 0;
    for _ in 0..50 {
        let mut choice = RelicChoice::new();
        let before = choice.options;
        choice.reroll();
        assert!(choice.options[0] != choice.options[1] && choice.options[1] != choice.options[2] && choice.options[0] != choice.options[2]);
        if choice.options.iter().all(|relic| !before.contains(relic)) {
            changed += 1;
        }
    }
    assert!(changed >= 40, "a reroll nearly always shows three new ones: {changed}");
    assert_eq!(rerolls_earned(0), 1);
    assert_eq!(rerolls_earned(2), 1);
    assert_eq!(rerolls_earned(3), 2);
    assert_eq!(rerolls_earned(9), 4);
}

#[test]
fn the_trade_offs_trade() {
    let glass = with(&[Relic::GlassCannon]);
    assert!(glass.damage > 1.5 && glass.toughness > 1.3, "hits harder and takes more");
    let clock = with(&[Relic::Overclock]);
    assert!(clock.speed > 1.3 && clock.healing < 0.6, "faster but heals slower");
}

#[test]
fn masonry_quarry_and_archers_change_the_walls_and_towers() {
    let up = with(&[Relic::Masonry]);
    assert!((wall_health(WallKind::Wall, 1, &up) / wall_health(WallKind::Wall, 1, &Upgrades::default()) - 1.3).abs() < 1e-4);
    let up = with(&[Relic::Quarry]);
    assert!((up.quarry - 1.5).abs() < 1e-5);
    assert!((with(&[Relic::Archers]).archery - 1.4).abs() < 1e-5);
}

#[test]
fn thorns_hurt_an_enemy_by_a_share_of_what_it_does() {
    let hit = |relics: &[Relic]| {
        let up = with(relics);
        let enemy = striker(MIDDLE, 100.0);
        let mut units = units_inside(&enemy, 5);
        let mut enemies = vec![enemy];
        let before = enemies[0].health;
        fight(&mut enemies, &mut units, &mut Vec::new(), 0.5, &up, &mut Effects::default());
        let to_units: f32 = units.iter().map(|unit| UNIT_HEALTH - unit.health).sum();
        (before - enemies[0].health, to_units)
    };
    let (plain, spent) = hit(&[]);
    let (thorny, same) = hit(&[Relic::Thorns]);
    assert!(spent > 0.0 && (spent - same).abs() < 0.01);
    assert!((thorny - plain - spent * 0.2).abs() < 0.01, "{plain} then {thorny} after {spent}");
    let (more, _) = hit(&[Relic::Thorns, Relic::Thorns]);
    assert!((more - plain - spent * 0.4).abs() < 0.01);
}

#[test]
fn bloodlust_heals_every_unit_for_each_enemy_destroyed() {
    let up = with(&[Relic::Bloodlust, Relic::Bloodlust]);
    let mut enemy = kind_at("Grunt", MIDDLE);
    enemy.health = 0.0;
    let mut fx = Effects::default();
    resolve_deaths(&mut vec![enemy], &mut Vec::new(), 100, &up, &mut fx);
    assert!((fx.spoils[0].heal_all - 3.0).abs() < 1e-5, "two stacks of 1.5: {}", fx.spoils[0].heal_all);
    //a minion is worth nothing, a boss a lot
    let mut imp = Enemy::minion(MIDDLE, kind_named("Imp"), 1.0, 99);
    imp.health = 0.0;
    let mut fx = Effects::default();
    resolve_deaths(&mut vec![imp], &mut Vec::new(), 100, &up, &mut fx);
    assert_eq!(fx.spoils[0].heal_all, 0.0);
    let mut boss = Enemy::boss(&BOSSES[0], 200, 0, MIDDLE);
    boss.health = 0.0;
    let mut fx = Effects::default();
    resolve_deaths(&mut vec![boss], &mut Vec::new(), 100, &up, &mut fx);
    assert!((fx.spoils[0].heal_all - 30.0).abs() < 1e-4);
    //none without the relic
    let mut enemy = kind_at("Grunt", MIDDLE);
    enemy.health = 0.0;
    let mut fx = Effects::default();
    resolve_deaths(&mut vec![enemy], &mut Vec::new(), 100, &Upgrades::default(), &mut fx);
    assert_eq!(fx.spoils[0].heal_all, 0.0);
    //and it reaches the hurt units, no further than full
    let mut units = vec![unit_on(MIDDLE), unit_on(MIDDLE)];
    units[0].health = 50.0;
    units[1].health = 99.0;
    heal_everyone(&mut units, 3.0);
    assert_eq!((units[0].health, units[1].health), (53.0, UNIT_HEALTH));
}

#[test]
fn an_aegis_shield_takes_the_first_hits_and_comes_back_after_a_quiet_while() {
    let up = with(&[Relic::Aegis]);
    assert_eq!(up.guard, 30.0);
    let mut units = vec![unit_on(MIDDLE)];
    assert_eq!(units[0].guard, 0.0, "a new unit has none until it has had a moment");
    heal_units(&mut units, 0.1, &up);
    assert_eq!(units[0].guard, 30.0);
    units[0].hurt(20.0);
    assert_eq!((units[0].guard, units[0].health), (10.0, UNIT_HEALTH));
    units[0].hurt(25.0);
    assert_eq!((units[0].guard, units[0].health), (0.0, UNIT_HEALTH - 15.0));
    //it doesn't come back while the unit keeps getting hurt, or straight away
    heal_units(&mut units, AEGIS_CALM_SECONDS - 1.0, &up);
    assert_eq!(units[0].guard, 0.0);
    heal_units(&mut units, 1.5, &up);
    assert_eq!(units[0].guard, 30.0);
    //without the relic nothing is shielded
    let mut plain = vec![unit_on(MIDDLE)];
    heal_units(&mut plain, 100.0, &Upgrades::default());
    assert_eq!(plain[0].guard, 0.0);
    //a shielded unit lasts longer in a fight, a tough one longer still
    let lasts = |up: &Upgrades| {
        let enemy = striker(MIDDLE, 100.0);
        let mut units = units_inside(&enemy, 1);
        heal_units(&mut units, 20.0, up);
        let mut enemies = vec![enemy];
        for frame in 0..600 {
            fight(&mut enemies, &mut units, &mut Vec::new(), 1.0 / 60.0, up, &mut Effects::default());
            if units[0].health <= 0.0 {
                return frame as f32 / 60.0;
            }
        }
        10.0
    };
    assert!(lasts(&up) > lasts(&Upgrades::default()) + 0.2);
}

#[test]
fn shots_and_hazards_go_through_the_shield_first_too() {
    let up = with(&[Relic::Aegis, Relic::Aegis, Relic::Aegis]);
    let mut units = vec![unit_on(MIDDLE)];
    heal_units(&mut units, 20.0, &up);
    let mut shots = vec![Shot::new(MIDDLE - Vec2::new(5.0, 0.0), Vec2::new(SHOT_SPEED, 0.0))];
    update_shots(&mut shots, &mut units, &Walls::default(), 1.0 / 60.0, &up, &mut Effects::default());
    assert_eq!(units[0].health, UNIT_HEALTH, "a shot weaker than the shield");
    assert!((units[0].guard - (90.0 - SHOT_DAMAGE)).abs() < 1e-3);
    let mut hazards = vec![hazard_at(MIDDLE, 100.0, 0.0, 20.0, 5)];
    update_hazards(&mut hazards, &mut units, &up, 0.1, &mut Effects::default());
    assert!((units[0].guard - (90.0 - SHOT_DAMAGE - 20.0)).abs() < 1e-3 && units[0].health == UNIT_HEALTH, "{} {}", units[0].guard, units[0].health);
    //a shielded unit is drawn in a lighter colour
    let plain = unit_color(&unit_on(MIDDLE));
    let mut shielded = unit_on(MIDDLE);
    shielded.guard = 10.0;
    let tint = unit_color(&shielded);
    assert!(tint.r < plain.r - 0.2, "less red, more cyan");
}

#[test]
fn an_opening_salvo_goes_off_when_a_fight_begins_and_not_again_for_a_while() {
    let up = with(&[Relic::OpeningSalvo]);
    let player = MIDDLE;
    let near = |distance: f32| {
        let mut enemy = kind_at("Brute", player + Vec2::new(distance, 0.0));
        enemy.chasing = true;
        enemy
    };
    let mut wait = 0.0;
    let mut fx = Effects::default();
    //nobody is close: nothing
    let mut enemies = vec![near(2000.0)];
    opening_salvo(&mut enemies, player, &mut wait, 0.1, &up, &mut fx);
    assert_eq!((enemies[0].health, wait), (enemies[0].max_health, 0.0));
    //one gets close: everything near it is hurt, the rest isn't
    let mut enemies = vec![near(300.0), near(480.0), near(2000.0)];
    let mut boss = Enemy::boss(&BOSSES[0], 300, 0, player + Vec2::new(400.0, 300.0));
    boss.chasing = true;
    enemies.push(boss);
    opening_salvo(&mut enemies, player, &mut wait, 0.1, &up, &mut fx);
    assert!(wait > SALVO_COOLDOWN - 0.5);
    assert!((enemies[0].health / enemies[0].max_health - (1.0 - SALVO_SHARE)).abs() < 0.01);
    assert!(enemies[1].health < enemies[1].max_health);
    assert_eq!(enemies[2].health, enemies[2].max_health, "too far");
    let boss_share = 1.0 - enemies[3].health / enemies[3].max_health;
    assert!(boss_share > 0.0 && boss_share < SALVO_SHARE * 0.2, "a boss only loses a little: {boss_share}");
    assert!(fx.rings.len() == 1 && fx.cues.contains(&Cue::Slam));
    //it won't go again at once
    let hurt = enemies[0].health;
    opening_salvo(&mut enemies, player, &mut wait, 0.1, &up, &mut fx);
    assert_eq!(enemies[0].health, hurt);
    //but it does later
    opening_salvo(&mut enemies, player, &mut wait, SALVO_COOLDOWN, &up, &mut fx);
    assert!(enemies[0].health < hurt);
    //and never without the relic
    let mut enemies = vec![near(100.0)];
    let mut wait = 0.0;
    opening_salvo(&mut enemies, player, &mut wait, 0.1, &Upgrades::default(), &mut fx);
    assert_eq!(enemies[0].health, enemies[0].max_health);
}

#[test]
fn a_martyr_hurts_the_enemy_it_dies_in_and_only_that_one() {
    let up = with(&[Relic::Martyr, Relic::Martyr]);
    let mut enemies = vec![kind_at("Brute", MIDDLE), kind_at("Brute", MIDDLE + Vec2::new(1000.0, 0.0))];
    let before = enemies[0].health;
    martyr_blasts(&mut enemies, &[(MIDDLE + Vec2::new(5.0, 0.0), 0), (MIDDLE + Vec2::new(-5.0, 0.0), 0), (MIDDLE + Vec2::new(500.0, 0.0), 0)], &up);
    assert!((before - enemies[0].health - MARTYR_DAMAGE * 2.0 * 2.0).abs() < 0.01, "two units died in it, with two relics");
    assert_eq!(enemies[1].health, enemies[1].max_health);
    martyr_blasts(&mut enemies, &[(MIDDLE, 0)], &Upgrades::default());
    assert!((before - enemies[0].health - MARTYR_DAMAGE * 4.0).abs() < 0.01, "nothing without the relic");
}

#[test]
fn bury_units_reports_where_they_fell() {
    let mut units = vec![unit_on(MIDDLE), unit_on(MIDDLE + Vec2::new(50.0, 0.0))];
    units[1].health = 0.0;
    let mut fx = Effects::default();
    bury_units(&mut units, &mut fx);
    assert_eq!(fx.fallen, vec![(MIDDLE + Vec2::new(50.0, 0.0), 0)]);
}

#[test]
fn the_choice_screen_labels_and_the_hud_has_room_for_every_relic_taken() {
    //the longest blurb still wraps into a few lines on a card
    for relic in RELICS {
        let words = relic.blurb().split_whitespace().count();
        assert!(words <= 14, "{}: {words} words", relic.name());
    }
}

#[test]
fn every_relic_has_its_own_tag_that_fits_its_tile() {
    for (i, relic) in RELICS.iter().enumerate() {
        assert!(relic.tag().chars().count() == 2, "{} has the tag {}", relic.name(), relic.tag());
        for other in &RELICS[i + 1..] {
            assert_ne!(relic.tag(), other.tag(), "{} and {}", relic.name(), other.name());
        }
    }
}

//everything a relic can change, as numbers
fn signature(up: &Upgrades) -> [f32; 19] {
    [up.damage, up.toughness, up.healing, up.speed, up.pickup, up.bounty, up.masonry, up.archery, up.quarry, up.thorns, up.bloodlust, up.guard, up.salvo, up.martyr, up.freeze, up.extra_boost, up.gunnery, up.pierce as f32, up.sight]
}

#[test]
fn every_relic_does_more_the_second_time_it_is_taken() {
    for relic in RELICS {
        let once = signature(&with(&[relic]));
        let twice = signature(&with(&[relic, relic]));
        //the volunteers a Muster brings are the game's to bring, one batch each time it is chosen
        if relic != Relic::Muster {
            assert_ne!(once, twice, "{} doesn't stack", relic.name());
        }
        assert_eq!(with(&[relic, relic]).count(relic), 2);
    }
}

#[test]
fn shockwaves_stack_without_a_ceiling_and_never_kill_outright() {
    let mut shares = Vec::new();
    for copies in 1..=12 {
        let mut up = Upgrades::default();
        for _ in 0..copies {
            up.take(Relic::OpeningSalvo);
        }
        shares.push(up.salvo_share());
    }
    assert!((shares[0] - SALVO_SHARE).abs() < 1e-6);
    assert!(shares.windows(2).all(|pair| pair[1] > pair[0]), "{shares:?}");
    assert!(shares[11] < 1.0, "{shares:?}");
}

#[test]
fn the_most_units_the_army_can_have_is_the_limit_it_is_given_and_no_relic_changes_it() {
    assert_eq!(Upgrades::default().unit_cap(), DEFAULT_ARMY_LIMIT);
    let mut up = with(&RELICS);
    assert_eq!(up.unit_cap(), DEFAULT_ARMY_LIMIT);
    //a full army takes no more babies until the limit is raised, and then it takes them
    let player = Vec2::new(1000.0, 1000.0);
    let mut units: Vec<Unit> = (0..DEFAULT_ARMY_LIMIT).map(|_| Unit::new(player.x, player.y, 300.0)).collect();
    let mut babies: Vec<Baby> = (0..3).map(|_| Baby { position: player, age: 1.0 }).collect();
    grow_babies(&mut babies, &mut units, player.x, player.y, 0.0, BABY_PICKUP_RANGE, up.unit_cap(), &mut Effects::default());
    assert_eq!((units.len(), babies.len()), (DEFAULT_ARMY_LIMIT, 3));
    up.army_limit += 3;
    grow_babies(&mut babies, &mut units, player.x, player.y, 0.0, BABY_PICKUP_RANGE, up.unit_cap(), &mut Effects::default());
    assert_eq!((units.len(), babies.len()), (DEFAULT_ARMY_LIMIT + 3, 0));
}

#[test]
fn enemies_drop_babies_for_the_room_a_bigger_limit_gives() {
    let mut enemy = plain_enemy(Vec2::new(5000.0, 5000.0));
    enemy.babies = 10;
    let mut none = Vec::new();
    drop_babies(&enemy, &mut none, DEFAULT_ARMY_LIMIT, 1.0, Upgrades::default().unit_cap());
    assert!(none.is_empty());
    let mut some = Vec::new();
    drop_babies(&enemy, &mut some, DEFAULT_ARMY_LIMIT, 1.0, DEFAULT_ARMY_LIMIT + 10);
    assert_eq!(some.len(), 10);
}

#[test]
fn tailwind_raises_the_most_an_armys_size_can_speed_it_up() {
    assert!((speed_multiplier_with(SPEED_BOOST_UNITS, 0.0) - MAX_SPEED_MULTIPLIER).abs() < 1e-6);
    assert!((speed_multiplier_with(SPEED_BOOST_UNITS, TAILWIND_BOOST) - (MAX_SPEED_MULTIPLIER + TAILWIND_BOOST)).abs() < 1e-6);
    //it helps a smaller army too, and past the old limit nothing holds it at the new one but the army's size
    assert!(speed_multiplier_with(SPEED_BOOST_UNITS / 2, TAILWIND_BOOST) > speed_multiplier(SPEED_BOOST_UNITS / 2));
    assert_eq!(speed_multiplier_with(SPEED_BOOST_UNITS * 4, TAILWIND_BOOST), speed_multiplier_with(SPEED_BOOST_UNITS, TAILWIND_BOOST));
    let up = with(&[Relic::Tailwind, Relic::Tailwind]);
    assert!((up.army_boost(SPEED_BOOST_UNITS) - (MAX_SPEED_MULTIPLIER + 2.0 * TAILWIND_BOOST)).abs() < 1e-5);
    //the enemies' speed is the army's, without the relics
    assert_eq!(speed_multiplier(SPEED_BOOST_UNITS), MAX_SPEED_MULTIPLIER);
}

#[test]
fn speed_relics_have_no_limit_of_their_own() {
    let mut up = Upgrades::default();
    for _ in 0..12 {
        up.take(Relic::FleetFeet);
        up.take(Relic::Overclock);
    }
    let expected = MAX_SPEED_MULTIPLIER * 1.12f32.powi(12) * 1.35f32.powi(12);
    assert!((up.army_boost(SPEED_BOOST_UNITS) - expected).abs() < expected * 1e-4, "{} against {expected}", up.army_boost(SPEED_BOOST_UNITS));
}

#[test]
fn an_offer_is_three_different_relics_and_a_reroll_is_three_others() {
    for _ in 0..200 {
        let choices = relic_choices();
        assert!(choices[0] != choices[1] && choices[1] != choices[2] && choices[0] != choices[2]);
    }
    let mut choice = RelicChoice::new();
    let before = choice.options;
    choice.reroll();
    let after = choice.options;
    assert!(after[0] != after[1] && after[1] != after[2] && after[0] != after[2]);
    //(a reroll avoids what it replaces when it can, which is nearly always)
    assert!(before.iter().all(|relic| !after.contains(relic)) || (0..20).any(|_| { choice.reroll(); before.iter().all(|relic| !choice.options.contains(relic)) }));
    //the rare ones the winner gets are all rare
    assert!(RelicChoice::rare().options.iter().all(|relic| relic.rarity() == Rarity::Rare));
}

// ---------- freezing ----------

#[test]
fn freezing_relics_add_up_to_a_chance_a_second_that_never_reaches_certain() {
    assert_eq!(Upgrades::default().freeze, 0.0);
    assert_eq!(Upgrades::default().freeze_chance(1.0), 0.0);
    let one = with(&[Relic::Freezing]);
    assert!((one.freeze - FREEZE_CHANCE).abs() < 1e-6);
    assert!((one.freeze_chance(1.0) - FREEZE_CHANCE).abs() < 1e-6, "a second of fighting is the chance a second");
    assert!((with(&[Relic::Freezing; 2]).freeze - 2.0 * FREEZE_CHANCE).abs() < 1e-6, "they stack");
    let many = with(&[Relic::Freezing; 40]);
    assert!(many.freeze <= MOST_FREEZE_CHANCE && many.freeze_chance(1.0) < 1.0);
    //a frame is a small part of a second's chance, and sixty of them come to it
    let frame = one.freeze_chance(1.0 / 60.0);
    assert!(frame > 0.0 && frame < 0.01);
    assert!(((1.0 - frame).powi(60) - (1.0 - FREEZE_CHANCE)).abs() < 1e-3);
    //the relic is on the list, and has a rarity, a tag and words of its own
    assert!(RELICS.contains(&Relic::Freezing));
    assert_eq!((Relic::Freezing.name(), Relic::Freezing.tag(), Relic::Freezing.rarity()), ("Freezing", "Fz", Rarity::Uncommon));
}

#[test]
fn an_enemy_being_fought_freezes_now_and_then_and_one_left_alone_does_not() {
    let up = with(&[Relic::Freezing; 4]);
    let (mut fought, mut alone) = (0, 0);
    for _ in 0..200 {
        let enemy = still_enemy(MIDDLE, 1.0e6);
        let mut units = units_inside(&enemy, 5);
        let mut enemies = vec![enemy];
        fight(&mut enemies, &mut units, &mut Vec::new(), 1.0, &up, &mut Effects::default());
        if enemies[0].status.frozen > 0.0 {
            fought += 1;
            assert_eq!(enemies[0].status.frozen, FREEZE_SECONDS);
        }
        let mut lonely = vec![still_enemy(MIDDLE, 1.0e6)];
        fight(&mut lonely, &mut [], &mut Vec::new(), 1.0, &up, &mut Effects::default());
        alone += (lonely[0].status.frozen > 0.0) as usize;
    }
    //four relics make it four chances in five a second
    assert!(fought > 130 && fought < 195, "{fought} of 200 froze");
    assert_eq!(alone, 0);
    //and with no relic nothing freezes
    let enemy = still_enemy(MIDDLE, 1.0e6);
    let mut units = units_inside(&enemy, 5);
    let mut enemies = vec![enemy];
    for _ in 0..50 {
        fight(&mut enemies, &mut units, &mut Vec::new(), 1.0, &Upgrades::default(), &mut Effects::default());
    }
    assert_eq!(enemies[0].status.frozen, 0.0);
}

#[test]
fn a_frozen_enemy_cannot_attack_move_or_shoot_and_thaws_after_a_while() {
    let up = Upgrades::default();
    //it does no damage to the units inside it
    let mut enemy = striker(MIDDLE, 100.0);
    enemy.status.frozen = FREEZE_SECONDS;
    assert_eq!(enemy.attack_now(), 0.0);
    let mut units = units_inside(&enemy, 5);
    let mut enemies = vec![enemy];
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.5, &up, &mut Effects::default());
    assert!(units.iter().all(|unit| unit.health == UNIT_HEALTH));
    //it thaws after FREEZE_SECONDS of the fight, and then it attacks again
    for _ in 0..(FREEZE_SECONDS * 10.0) as usize {
        fight(&mut enemies, &mut units, &mut Vec::new(), 0.1, &up, &mut Effects::default());
    }
    assert_eq!(enemies[0].status.frozen, 0.0);
    assert!(enemies[0].attack_now() > 0.0);
    let mut units = units_inside(&enemies[0], 5);
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.5, &up, &mut Effects::default());
    assert!(units.iter().any(|unit| unit.health < UNIT_HEALTH));
    //it does not walk
    let player = MIDDLE + Vec2::new(1200.0, 0.0);
    let walker = || {
        let mut enemy = plain_enemy(MIDDLE);
        enemy.speed = 100.0;
        enemy.chasing = true;
        enemy.aggro = 5000.0;
        enemy
    };
    let mut free = vec![walker()];
    patrol_free(&mut free, player, 1.0, 1.0, &mut Effects::default());
    assert!(free[0].position.distance(MIDDLE) > 10.0, "a thawed one walks");
    let mut held = vec![walker()];
    held[0].status.frozen = FREEZE_SECONDS;
    patrol_free(&mut held, player, 1.0, 1.0, &mut Effects::default());
    assert_eq!(held[0].position, MIDDLE);
    //and does not shoot
    let mut shooter = Enemy::new(MIDDLE, the_spitter(), 1.0);
    shooter.position = MIDDLE;
    shooter.shot_in = 0.0;
    let target = [unit_on(MIDDLE + Vec2::new(200.0, 0.0))];
    let mut shots = Vec::new();
    let mut shooters = vec![shooter];
    shooters[0].status.frozen = FREEZE_SECONDS;
    shoot_enemies(&mut shooters, &target, &mut shots, 0.1, &mut Effects::default());
    assert!(shots.is_empty());
    shooters[0].status.frozen = 0.0;
    shoot_enemies(&mut shooters, &target, &mut shots, 0.1, &mut Effects::default());
    assert!(!shots.is_empty(), "a thawed one does");
}

#[test]
fn a_boss_and_a_nest_cannot_be_frozen() {
    let up = with(&[Relic::Freezing; 10]);
    let boss = Enemy::boss(&BOSSES[0], 100, 1, MIDDLE);
    let nest = kind_at("Nest", MIDDLE + Vec2::new(5000.0, 0.0));
    assert!(!boss.can_freeze() && !nest.can_freeze());
    for mut enemy in [boss, nest] {
        enemy.health = 1.0e9;
        enemy.max_health = 1.0e9;
        let mut units = units_inside(&enemy, 5);
        let mut enemies = vec![enemy];
        for _ in 0..20 {
            fight(&mut enemies, &mut units, &mut Vec::new(), 1.0, &up, &mut Effects::default());
        }
        assert_eq!(enemies[0].status.frozen, 0.0, "{}", enemies[0].name);
        enemies[0].status.frozen = 1.0;
        assert!(!enemies[0].shatters(), "{} does not shatter", enemies[0].name);
    }
}

#[test]
fn the_enemies_other_enemies_made_do_not_shatter() {
    let mut imp = Enemy::minion(MIDDLE, kind_named("Imp"), 1.0, 99);
    freeze(&mut imp, FREEZE_SECONDS);
    assert!(imp.status.frozen > 0.0 && !imp.shatters());
}

#[test]
fn a_frozen_horde_enemy_shatters_and_leaves_no_new_enemies() {
    let mut enemy = kind_at("Brute", MIDDLE);
    join_horde(&mut enemy);
    enemy.status.frozen = 1.0;
    enemy.health = 0.0;
    let mut enemies = vec![enemy];
    let mut fx = Effects::default();
    resolve_deaths(&mut enemies, &mut Vec::new(), 100, &Upgrades::default(), &mut fx);
    assert!(enemies.is_empty());
    assert_eq!(fx.shatters.len(), 1);
}

#[test]
fn the_army_destroys_a_frozen_enemy_and_it_shatters_in_the_middle_of_the_fight() {
    let mut enemy = kind_at("Grunt", MIDDLE);
    enemy.status.frozen = FREEZE_SECONDS;
    enemy.health = 1.0;
    let mut units = units_inside(&enemy, 20);
    let mut enemies = vec![enemy];
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    let destroyed = fight(&mut enemies, &mut units, &mut babies, 0.5, &Upgrades::default(), &mut fx);
    assert_eq!(destroyed, 1);
    assert!(enemies.is_empty(), "it leaves no shards behind");
    assert!(fx.shatters.len() == 1 && fx.shatters[0].frost);
    assert!(fx.cues.contains(&Cue::Shatter));
}
