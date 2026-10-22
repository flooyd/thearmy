//the chest a broken horde leaves: who can open it, what is in it, and when it goes

use super::*;

const SPOT: Vec2 = Vec2::new(12000.0, 12000.0);

#[test]
fn a_chest_holds_more_points_the_better_it_is_and_there_are_three_kinds() {
    let worth = 40.0;
    let points: Vec<f32> = (1..=3).map(|tier| Chest::new(SPOT, tier, worth).points).collect();
    assert_eq!(points, vec![worth * CHEST_POINTS_PER_TIER, worth * CHEST_POINTS_PER_TIER * 2.0, worth * CHEST_POINTS_PER_TIER * 3.0]);
    //a tier outside the three is the nearest one
    assert_eq!((Chest::new(SPOT, 0, 1.0).tier, Chest::new(SPOT, 9, 1.0).tier), (1, 3));
    let names: Vec<&str> = (1..=3).map(|tier| Chest::new(SPOT, tier, 1.0).name()).collect();
    assert_eq!(names, vec!["BRONZE", "SILVER", "GOLD"]);
    assert_eq!(Chest::new(SPOT, 1, 1.0).left, CHEST_SECONDS);
}

#[test]
fn what_is_in_a_chest_grows_with_the_army_and_the_tier_and_only_the_better_ones_hold_a_champion_or_a_relic() {
    let bronze = Chest::new(SPOT, 1, 10.0).loot(4000);
    let silver = Chest::new(SPOT, 2, 10.0).loot(4000);
    let gold = Chest::new(SPOT, 3, 10.0).loot(4000);
    assert_eq!((bronze.champion, bronze.relic), (false, false));
    assert_eq!((silver.champion, silver.relic), (true, false));
    assert_eq!((gold.champion, gold.relic), (true, true));
    assert_eq!(gold.recruits, bronze.recruits * 3);
    //recruits are a share of the army, but never fewer or more than the limits
    assert_eq!(Chest::new(SPOT, 1, 1.0).loot(0).recruits, CHEST_RECRUITS_MIN);
    assert_eq!(Chest::new(SPOT, 1, 1.0).loot(10_000_000).recruits, CHEST_RECRUITS_MAX);
    let mut last = 0;
    for army in [0, 400, 1000, 3000, 8000, 20_000] {
        let recruits = Chest::new(SPOT, 2, 1.0).loot(army).recruits;
        assert!(recruits >= last, "army {army}");
        last = recruits;
    }
}

#[test]
fn the_player_or_any_unit_getting_to_a_chest_opens_it_and_no_one_far_from_it_does() {
    let units = vec![unit_at(SPOT.x + 3000.0, SPOT.y), unit_at(SPOT.x - 500.0, SPOT.y)];
    let other = Chest::new(SPOT + Vec2::new(0.0, 4000.0), 1, 5.0);
    let mut chests = vec![Chest::new(SPOT, 2, 5.0), other];
    assert!(take_reached(&mut chests, SPOT + Vec2::new(0.0, -2000.0), &units).is_empty() && chests.len() == 2, "nobody is there");
    //the player
    let opened = take_reached(&mut chests, SPOT + Vec2::new(CHEST_REACH - 1.0, 0.0), &units);
    assert_eq!((opened.len(), opened[0].tier, chests.len()), (1, 2, 1));
    assert_eq!(chests[0], other, "the other is left where it is");
    //a unit, with the player nowhere near
    let units = vec![unit_at(other.at.x, other.at.y + CHEST_REACH - 2.0)];
    assert_eq!(take_reached(&mut chests, Vec2::ZERO, &units).len(), 1);
    assert!(chests.is_empty());
    //and not a pixel too far
    let mut chests = vec![Chest::new(SPOT, 1, 5.0)];
    assert!(take_reached(&mut chests, SPOT + Vec2::new(CHEST_REACH + 2.0, 0.0), &[]).is_empty());
}

#[test]
fn opening_a_chest_earns_the_points_puts_recruits_down_and_wins_a_champion_if_it_holds_one() {
    rand::srand(31);
    for tier in 1..=3 {
        let chest = Chest::new(SPOT, tier, 30.0);
        let mut babies = Vec::new();
        let mut fx = Effects::default();
        let relic = open_chest(&chest, 1200, MAX_UNITS, &mut babies, &mut fx);
        let loot = chest.loot(1200);
        assert_eq!(relic, tier >= CHEST_RELIC_TIER, "tier {tier}");
        assert_eq!(fx.points, loot.points);
        assert_eq!(babies.len(), loot.recruits, "tier {tier}");
        assert!(babies.iter().all(|baby| baby.position.distance(SPOT) <= TILE_SIZE * 2.5 + 1.0));
        assert_eq!(fx.recruits.len(), (tier >= CHEST_CHAMPION_TIER) as usize);
        assert!(fx.recruits.iter().all(|(at, odds)| *at == SPOT && *odds == Odds::Horde(tier)));
        assert!(fx.announcements.iter().any(|(text, _)| text.contains(chest.name())), "the player is told");
        assert!(!fx.cues.is_empty() && !fx.popups.is_empty());
    }
}

#[test]
fn recruits_that_the_army_has_no_room_for_are_not_made_and_the_chest_still_opens() {
    let chest = Chest::new(SPOT, 3, 30.0);
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    open_chest(&chest, MAX_UNITS, MAX_UNITS, &mut babies, &mut fx);
    assert!(babies.is_empty(), "a full army has no room");
    assert!(fx.points > 0.0);
    //and a little room gives a few
    open_chest(&chest, MAX_UNITS - 7, MAX_UNITS, &mut babies, &mut fx);
    assert_eq!(babies.len(), 7);
}

#[test]
fn a_chest_that_is_about_to_go_blinks_and_one_with_time_to_spare_does_not() {
    let fresh = Chest::new(SPOT, 1, 1.0);
    let dying = Chest { left: CHEST_BLINK_SECONDS - 1.0, ..fresh };
    let seen = |chest: &Chest| (0..100).filter(|&i| chest.shown(i as f32 * 0.037)).count();
    assert_eq!(seen(&fresh), 100);
    assert!(seen(&dying) > 30 && seen(&dying) < 90, "{}", seen(&dying));
}

#[test]
fn a_gold_chest_holds_nothing_common_and_a_better_chest_is_likelier_to_hold_a_rare_champion() {
    let gold = Odds::Horde(3).weights();
    let silver = Odds::Horde(2).weights();
    assert_eq!(gold[0], 0.0, "a gold chest holds nothing common");
    assert!(gold[2] > silver[2] && silver[2] > 0.0 && silver[0] > 0.0);
    //picked from a gold chest it is always uncommon or rare
    rand::srand(5);
    for _ in 0..200 {
        let kind = pick_champion(&[], gold, rand::gen_range(0.0, 1.0));
        assert!(CHAMPIONS[kind].grade != Rarity::Common);
    }
}

//a save with chests and a horde on its way, to load in the real game and look at: LOOK_SAVE_TO=path/autosave.sav cargo test --release
//write_a_save_with_chests_to_look_at -- --ignored
#[test]
#[ignore]
fn write_a_save_with_chests_to_look_at() {
    let Ok(path) = std::env::var("LOOK_SAVE_TO") else {
        return;
    };
    let mut world = new_run();
    let player = world.player;
    world.hordes.chests = vec![
        Chest::new(player + Vec2::new(130.0, -70.0), 3, 40.0),
        Chest::new(player + Vec2::new(-170.0, 110.0), 2, 40.0),
        Chest { left: 12.0, ..Chest::new(player + Vec2::new(60.0, 330.0), 1, 40.0) },
        Chest::new(player + Vec2::new(2600.0, -900.0), 2, 40.0),
    ];
    world.hordes.coming = Some(Coming { def: horde_named("Stampede").unwrap(), at: player + Vec2::new(-2500.0, -1500.0), left: 90.0 });
    std::fs::write(path, world.to_text()).unwrap();
}
