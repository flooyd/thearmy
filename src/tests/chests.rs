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
    //(held for as long as it takes, which is all a silver chest wants)
    let long = CHEST_HOLD_SECONDS[1] + 1.0;
    let mut chests = vec![Chest::new(SPOT, 2, 5.0), other];
    assert!(take_reached(&mut chests, SPOT + Vec2::new(0.0, -2000.0), &units, &[], long).is_empty() && chests.len() == 2, "nobody is there");
    //the player
    let opened = take_reached(&mut chests, SPOT + Vec2::new(CHEST_REACH - 1.0, 0.0), &units, &[], long);
    assert_eq!((opened.len(), opened[0].tier, chests.len()), (1, 2, 1));
    assert_eq!(chests[0], other, "the other is left where it is");
    //a unit, with the player nowhere near
    let units = vec![unit_at(other.at.x, other.at.y + CHEST_REACH - 2.0)];
    assert_eq!(take_reached(&mut chests, Vec2::ZERO, &units, &[], 0.0).len(), 1, "a bronze chest opens as soon as it is reached");
    assert!(chests.is_empty());
    //and not a pixel too far
    let mut chests = vec![Chest::new(SPOT, 1, 5.0)];
    assert!(take_reached(&mut chests, SPOT + Vec2::new(CHEST_REACH + 2.0, 0.0), &[], &[], long).is_empty());
}

#[test]
fn a_silver_or_gold_chest_has_to_be_held_for_a_few_seconds_and_a_bronze_one_does_not() {
    assert_eq!(Chest::new(SPOT, 1, 1.0).hold(), 0.0);
    let (silver, gold) = (Chest::new(SPOT, 2, 1.0), Chest::new(SPOT, 3, 1.0));
    assert!(silver.hold() > 0.0 && gold.hold() > silver.hold(), "the better the chest the longer it takes");
    //a tier outside the three is the nearest one
    assert_eq!((Chest::new(SPOT, 0, 1.0).hold(), Chest::new(SPOT, 9, 1.0).hold()), (0.0, gold.hold()));
    for chest in [silver, gold] {
        let mut chests = vec![chest];
        let mut held = 0.0;
        //the player stands there, a frame at a time
        while held + 0.05 < chest.hold() {
            assert!(take_reached(&mut chests, SPOT, &[], &[], 0.05).is_empty(), "still held at {held:.2}s of {}", chest.hold());
            held += 0.05;
            assert!((chests[0].held - held).abs() < 0.001);
        }
        let opened = take_reached(&mut chests, SPOT, &[], &[], 0.1);
        assert_eq!(opened.len(), 1, "opened after {} seconds", chest.hold());
        assert!(chests.is_empty());
    }
}

#[test]
fn a_chest_that_is_not_being_held_loses_its_hold_and_a_unit_can_hold_it_for_the_player() {
    let gold = Chest::new(SPOT, 3, 1.0);
    let mut chests = vec![gold];
    take_reached(&mut chests, SPOT, &[], &[], 2.0);
    assert_eq!(chests[0].held, 2.0);
    //the player steps away: it gives up its hold faster than it was made, down to nothing
    take_reached(&mut chests, SPOT + Vec2::new(0.0, 900.0), &[], &[], 0.5);
    assert!((chests[0].held - (2.0 - 0.5 * CHEST_HOLD_DRAIN)).abs() < 0.001);
    take_reached(&mut chests, SPOT + Vec2::new(0.0, 900.0), &[], &[], 100.0);
    assert_eq!(chests[0].held, 0.0, "never below nothing");
    //the army can stand at it with the player somewhere else
    let units = vec![unit_at(SPOT.x + 10.0, SPOT.y)];
    let mut seconds = 0.0;
    while !chests.is_empty() && seconds < 10.0 {
        take_reached(&mut chests, Vec2::ZERO, &units, &[], 0.1);
        seconds += 0.1;
    }
    assert!(chests.is_empty(), "a unit holds it as well as the player");
    assert!((seconds - gold.hold()).abs() < 0.3, "{seconds}");
}

#[test]
fn the_hud_says_how_long_is_left_to_hold_and_a_held_chest_is_said_before_the_others() {
    let mut hordes = Hordes::new();
    hordes.next_in = 1.0e9;
    let gold = Chest::new(SPOT, 3, 1.0);
    hordes.chests.push(Chest { left: 5.0, ..Chest::new(SPOT + Vec2::X * 800.0, 1, 1.0) });
    hordes.chests.push(gold);
    let line = |hordes: &Hordes| hordes.status(&[]).map(|(line, _)| line).unwrap();
    assert_eq!(line(&hordes), "BRONZE CHEST: 5s left", "nothing is being held, so the one that goes first");
    take_reached(&mut hordes.chests, SPOT, &[], &[], 1.0);
    assert_eq!(line(&hordes), format!("GOLD CHEST: HOLD IT {}s", (gold.hold() - 1.0).ceil() as i32));
    take_reached(&mut hordes.chests, SPOT, &[], &[], gold.hold() - 1.0 - 0.01);
    assert_eq!(line(&hordes), "GOLD CHEST: HOLD IT 1s", "never 0 while it is still shut");
}

#[test]
fn a_chest_that_is_loaded_has_to_be_held_again() {
    let mut chest = Chest::new(SPOT, 3, 12.0);
    chest.held = 3.5;
    let mut writer = SaveWriter::new();
    writer.put(&chest);
    let text = writer.finish();
    let back: Chest = SaveReader::new(&text).get().unwrap();
    assert_eq!((back.at, back.tier, back.points, back.left, back.held), (chest.at, 3, chest.points, chest.left, 0.0));
}

#[test]
fn opening_a_chest_earns_the_points_puts_recruits_down_and_wins_a_champion_if_it_holds_one() {
    rand::srand(31);
    for tier in 1..=3 {
        let chest = Chest::new(SPOT, tier, 30.0);
        let mut babies = Vec::new();
        let mut fx = Effects::default();
        let relic = open_chest(&chest, 1200, DEFAULT_ARMY_LIMIT, &mut babies, &mut fx);
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
    open_chest(&chest, DEFAULT_ARMY_LIMIT, DEFAULT_ARMY_LIMIT, &mut babies, &mut fx);
    assert!(babies.is_empty(), "a full army has no room");
    assert!(fx.points > 0.0);
    //and a little room gives a few
    open_chest(&chest, DEFAULT_ARMY_LIMIT - 7, DEFAULT_ARMY_LIMIT, &mut babies, &mut fx);
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
    world.hordes.coming = Some(Coming { def: horde_named("Stampede").unwrap(), at: player + Vec2::new(-2500.0, -1500.0), left: 90.0, other: None });
    std::fs::write(path, world.to_text()).unwrap();
}

//(the chests that have to be held can be contested: enemies near one stop the hold)
fn foe_near(chest: &Chest, gap: f32) -> Enemy {
    let mut foe = kind_at("Grunt", chest.at + Vec2::new(gap, 0.0));
    foe.chasing = true;
    foe
}

#[test]
fn enemies_near_a_chest_that_has_to_be_held_stop_the_hold_and_it_goes_on_when_they_are_gone() {
    rand::srand(41);
    let gold = Chest::new(SPOT, 3, 1.0);
    let mut chests = vec![gold];
    //an enemy just inside the reach, one just outside it
    let mut foe = foe_by_the_edge(&gold, -3.0);
    let far = foe_by_the_edge(&gold, 40.0);
    assert!(contested_by(gold.at, &[foe.clone()]) && !contested_by(gold.at, &[far.clone()]));
    //the first second is held, with only the far one about
    take_reached(&mut chests, SPOT, &[], &[far.clone()], 1.0);
    assert_eq!((chests[0].held, chests[0].contested, chests[0].stalled), (1.0, false, false));
    //the near one comes: the hold stands still however long the player stays, and the chest does not open
    for _ in 0..100 {
        assert!(take_reached(&mut chests, SPOT, &[], &[far.clone(), foe.clone()], 0.1).is_empty());
    }
    assert_eq!(chests[0].held, 1.0, "neither made nor lost");
    assert!(chests[0].contested && chests[0].stalled);
    //it is destroyed (or gone): the hold goes on from where it was
    foe.health = 0.0;
    take_reached(&mut chests, SPOT, &[], &[far.clone(), foe.clone()], 0.5);
    assert_eq!((chests[0].held, chests[0].contested, chests[0].stalled), (1.5, false, false));
    let opened = take_reached(&mut chests, SPOT, &[], &[far], gold.hold());
    assert_eq!(opened.len(), 1, "and it opens");
}

//(one whose edge is this far outside the reach of a chest, or inside it if that is below nothing)
fn foe_by_the_edge(chest: &Chest, beyond: f32) -> Enemy {
    let mut foe = foe_near(chest, 0.0);
    foe.position = chest.at + Vec2::new(CHEST_CONTEST_TILES * TILE_SIZE + foe.size / 2.0 + beyond, 0.0);
    foe
}

#[test]
fn a_contested_chest_still_loses_its_hold_when_nobody_is_at_it_and_is_only_stalled_while_someone_is() {
    rand::srand(42);
    let gold = Chest::new(SPOT, 3, 1.0);
    let mut chests = vec![gold];
    take_reached(&mut chests, SPOT, &[], &[], 2.0);
    let foe = foe_near(&gold, 100.0);
    //the player is at it: stalled
    take_reached(&mut chests, SPOT, &[], &[foe.clone()], 1.0);
    assert_eq!((chests[0].held, chests[0].stalled), (2.0, true));
    //the player steps back: the chest is still contested, but nothing is being held up, and the hold drains as usual
    take_reached(&mut chests, SPOT + Vec2::new(0.0, 900.0), &[], &[foe.clone()], 0.5);
    assert!((chests[0].held - (2.0 - 0.5 * CHEST_HOLD_DRAIN)).abs() < 1.0e-4);
    assert_eq!((chests[0].contested, chests[0].stalled), (true, false));
    //a unit at it with the player away holds it up in the same way
    let units = vec![unit_at(SPOT.x + 5.0, SPOT.y)];
    take_reached(&mut chests, Vec2::ZERO, &units, &[foe], 3.0);
    assert!(chests[0].stalled && chests[0].held < 2.0, "{}", chests[0].held);
}

#[test]
fn only_enemies_that_are_awake_and_a_danger_contest_a_chest() {
    rand::srand(43);
    let chest = Chest::new(SPOT, 3, 1.0);
    let foe = foe_near(&chest, 50.0);
    assert!(contested_by(chest.at, &[foe.clone()]));
    let mut frozen = foe.clone();
    frozen.frozen = 3.0;
    let mut routed = foe.clone();
    routed.break_and_run();
    let mut hiding = kind_at("Ambusher", chest.at + Vec2::new(50.0, 0.0));
    assert!(hiding.lurking());
    let mut dead = foe.clone();
    dead.health = 0.0;
    for (name, quiet) in [("frozen", &frozen), ("routed", &routed), ("hiding", &hiding), ("dead", &dead)] {
        assert!(!contested_by(chest.at, &[quiet.clone()]), "a {name} one is no danger to a chest");
    }
    //an ambusher that has sprung is one
    hiding.spring(SPOT, &mut Effects::default());
    assert!(contested_by(chest.at, &[hiding]));
    //a big one reaches further, from its edge
    let mut giant = kind_at("Titan", chest.at + Vec2::new(CHEST_CONTEST_TILES * TILE_SIZE + 100.0, 0.0));
    giant.size = 400.0;
    assert!(contested_by(chest.at, &[giant.clone()]));
    giant.size = 100.0;
    assert!(!contested_by(chest.at, &[giant]));
}

#[test]
fn a_bronze_chest_cannot_be_contested_it_opens_the_moment_it_is_reached() {
    rand::srand(44);
    let bronze = Chest::new(SPOT, 1, 1.0);
    let mut chests = vec![bronze];
    let crowd: Vec<Enemy> = (0..5).map(|k| foe_near(&bronze, 60.0 + 20.0 * k as f32)).collect();
    let opened = take_reached(&mut chests, SPOT, &[], &crowd, 0.0);
    assert_eq!(opened.len(), 1);
    assert!(!opened[0].contested && !opened[0].stalled);
}

#[test]
fn the_hud_says_a_chest_is_contested_while_someone_is_at_it_and_that_comes_before_everything_else() {
    rand::srand(45);
    let mut hordes = Hordes::new();
    hordes.next_in = 1.0e9;
    let gold = Chest::new(SPOT, 3, 1.0);
    let silver = Chest::new(SPOT + Vec2::new(0.0, 2000.0), 2, 1.0);
    hordes.chests = vec![gold, silver];
    let foe = foe_near(&gold, 80.0);
    //held, and then contested with the player there
    take_reached(&mut hordes.chests, SPOT + Vec2::new(0.0, 2000.0), &[], &[], 1.0);
    take_reached(&mut hordes.chests, SPOT, &[], &[foe.clone()], 1.0);
    let (line, color) = hordes.status(&[foe.clone()]).unwrap();
    assert_eq!(line, "GOLD CHEST: CONTESTED, CLEAR THE ENEMIES AROUND IT");
    assert_eq!(color, danger_color(1.0));
    //contested with no one there is not worth a line of its own: the chests are told as they were
    take_reached(&mut hordes.chests, SPOT + Vec2::new(0.0, -2000.0), &[], &[foe], 0.1);
    assert!(hordes.chests.iter().any(|chest| chest.contested) && hordes.chests.iter().all(|chest| !chest.stalled));
    assert!(!hordes.status(&[]).unwrap().0.contains("CONTESTED"));
}

#[test]
fn a_chest_that_is_loaded_is_not_contested_until_the_enemies_are_looked_at_again() {
    let mut chest = Chest::new(SPOT, 3, 12.0);
    chest.contested = true;
    chest.stalled = true;
    let mut writer = SaveWriter::new();
    writer.put(&chest);
    let text = writer.finish();
    let back: Chest = SaveReader::new(&text).get().unwrap();
    assert_eq!((back.contested, back.stalled), (false, false));
}
