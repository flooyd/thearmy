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
    assert_eq!(RELICS.len(), 19);
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
        for relic in relic_choices(false) {
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
        let mut choice = RelicChoice::new(false);
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
    assert!((fx.heal_all - 3.0).abs() < 1e-5, "two stacks of 1.5: {}", fx.heal_all);
    //a minion is worth nothing, a boss a lot
    let mut imp = Enemy::minion(MIDDLE, kind_named("Imp"), 1.0, 99);
    imp.health = 0.0;
    let mut fx = Effects::default();
    resolve_deaths(&mut vec![imp], &mut Vec::new(), 100, &up, &mut fx);
    assert_eq!(fx.heal_all, 0.0);
    let mut boss = Enemy::boss(&BOSSES[0], 200, 0, MIDDLE);
    boss.health = 0.0;
    let mut fx = Effects::default();
    resolve_deaths(&mut vec![boss], &mut Vec::new(), 100, &up, &mut fx);
    assert!((fx.heal_all - 30.0).abs() < 1e-4);
    //none without the relic
    let mut enemy = kind_at("Grunt", MIDDLE);
    enemy.health = 0.0;
    let mut fx = Effects::default();
    resolve_deaths(&mut vec![enemy], &mut Vec::new(), 100, &Upgrades::default(), &mut fx);
    assert_eq!(fx.heal_all, 0.0);
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
    martyr_blasts(&mut enemies, &[MIDDLE + Vec2::new(5.0, 0.0), MIDDLE + Vec2::new(-5.0, 0.0), MIDDLE + Vec2::new(500.0, 0.0)], &up);
    assert!((before - enemies[0].health - MARTYR_DAMAGE * 2.0 * 2.0).abs() < 0.01, "two units died in it, with two relics");
    assert_eq!(enemies[1].health, enemies[1].max_health);
    martyr_blasts(&mut enemies, &[MIDDLE], &Upgrades::default());
    assert!((before - enemies[0].health - MARTYR_DAMAGE * 4.0).abs() < 0.01, "nothing without the relic");
}

#[test]
fn bury_units_reports_where_they_fell() {
    let mut units = vec![unit_on(MIDDLE), unit_on(MIDDLE + Vec2::new(50.0, 0.0))];
    units[1].health = 0.0;
    let mut fx = Effects::default();
    bury_units(&mut units, &mut fx);
    assert_eq!(fx.fallen, vec![MIDDLE + Vec2::new(50.0, 0.0)]);
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
fn signature(up: &Upgrades) -> [f32; 16] {
    [up.damage, up.toughness, up.healing, up.speed, up.pickup, up.bounty, up.masonry, up.archery, up.quarry, up.thorns, up.bloodlust, up.guard, up.salvo, up.martyr, up.extra_units as f32, up.extra_boost]
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
fn barracks_let_the_army_grow_past_the_most_it_could() {
    assert_eq!(Upgrades::default().unit_cap(), MAX_UNITS);
    let up = with(&[Relic::Barracks, Relic::Barracks]);
    assert_eq!(up.unit_cap(), MAX_UNITS + 2 * BARRACKS_UNITS);
    //a full army takes no more babies until there is room, and then it takes them
    let player = Vec2::new(1000.0, 1000.0);
    let mut units: Vec<Unit> = (0..MAX_UNITS).map(|_| Unit::new(player.x, player.y, 300.0)).collect();
    let mut babies: Vec<Baby> = (0..3).map(|_| Baby { position: player, age: 1.0 }).collect();
    grow_babies(&mut babies, &mut units, player.x, player.y, 0.0, BABY_PICKUP_RANGE, Upgrades::default().unit_cap(), &mut Effects::default());
    assert_eq!((units.len(), babies.len()), (MAX_UNITS, 3));
    grow_babies(&mut babies, &mut units, player.x, player.y, 0.0, BABY_PICKUP_RANGE, up.unit_cap(), &mut Effects::default());
    assert_eq!((units.len(), babies.len()), (MAX_UNITS + 3, 0));
}

#[test]
fn enemies_drop_babies_for_the_room_the_bigger_cap_gives() {
    let mut enemy = plain_enemy(Vec2::new(5000.0, 5000.0));
    enemy.babies = 10;
    let mut none = Vec::new();
    drop_babies(&enemy, &mut none, MAX_UNITS, 1.0, Upgrades::default().unit_cap());
    assert!(none.is_empty());
    let mut some = Vec::new();
    drop_babies(&enemy, &mut some, MAX_UNITS, 1.0, with(&[Relic::Barracks]).unit_cap());
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
fn a_full_army_is_offered_a_barracks_far_more_often_because_it_is_the_only_way_to_grow() {
    let offered = |army_full: bool| {
        (0..3000).filter(|_| relic_choices(army_full).contains(&Relic::Barracks)).count() as f32 / 3000.0
    };
    //three of nineteen, weighted: a barracks turns up in roughly one offer in eight, and a full army's in nearly half. the
    //random numbers are shared with the tests that seed them, which can skew one measurement, so a wrong weighting has to
    //get it wrong every time to fail
    let mut rates = (0.0, 0.0);
    for _ in 0..4 {
        rates = (offered(false), offered(true));
        if rates.0 > 0.06 && rates.0 < 0.2 && rates.1 > rates.0 * 2.5 && rates.1 > 0.3 {
            break;
        }
    }
    let (usually, full) = rates;
    assert!(usually > 0.06 && usually < 0.2, "{usually}");
    assert!(full > usually * 2.5 && full > 0.3, "{full} against {usually}");
    //the offers are still three different relics, and only a barracks is likelier
    for _ in 0..200 {
        let choices = relic_choices(true);
        assert!(choices[0] != choices[1] && choices[1] != choices[2] && choices[0] != choices[2]);
    }
    //a reroll of the offer a full army got keeps it likelier
    //(a reroll avoids the relics it replaces, which takes some of the barracks out of it, so only a clear rise is asked for)
    let mut rerolled = 0.0;
    for _ in 0..4 {
        let count = (0..1000)
            .filter(|_| {
                let mut choice = RelicChoice::new(true);
                choice.reroll();
                choice.options.contains(&Relic::Barracks)
            })
            .count();
        rerolled = count as f32 / 1000.0;
        if rerolled > usually * 1.6 {
            break;
        }
    }
    assert!(rerolled > usually * 1.6, "{rerolled} against {usually}");
    //the rare ones the winner gets are not changed
    assert!(RelicChoice::rare().options.iter().all(|relic| *relic != Relic::Barracks));
}
