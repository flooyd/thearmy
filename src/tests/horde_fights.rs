//the kinds of horde, how one is fought and broken, and the chest it leaves

use super::*;

const CENTRE: Vec2 = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);

//a horde of this kind on its way, and arriving after one second. returns how many came
fn arriving(hordes: &mut Hordes, name: &str, enemies: &mut Vec<Enemy>, army: usize) -> usize {
    hordes.coming = Some(Coming { def: horde_named(name).unwrap(), at: CENTRE + Vec2::new(HORDE_DISTANCE_TILES * TILE_SIZE, 0.0), left: 0.5, other: None });
    match hordes.update(enemies, CENTRE, army, army, false, 1.0) {
        Some(HordeBeat::Arrived { count, .. }) => count,
        _ => panic!("{name} should have arrived"),
    }
}

#[test]
fn every_horde_is_made_of_real_enemies_has_a_tier_of_chest_and_the_best_are_for_the_biggest_armies() {
    let mut names: Vec<&str> = HORDES.iter().map(|def| def.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), HORDES.len(), "no two hordes share a name, as a save finds them by it");
    for def in HORDES.iter() {
        assert!((1..=3).contains(&def.prize), "{}", def.name);
        assert!(horde_named(def.name).is_some_and(|found| std::ptr::eq(found, def)));
        for (kind, _) in def.kinds {
            let kind = kind_named(kind);
            assert!(def.from_army >= kind.from_army, "{} has {} before the army is big enough for it", def.name, kind.name);
        }
    }
    assert!(horde_named("Nothing at all").is_none());
    for def in HORDES.iter().filter(|def| def.prize == 3) {
        assert!(def.from_army >= 250, "{} is gold, so only a big army meets it", def.name);
    }
    assert!(HORDES.iter().filter(|def| def.from_army < 100).all(|def| def.prize <= 2));
    assert!(HORDES.iter().any(|def| def.prize == 1) && HORDES.iter().any(|def| def.prize == 3));
}

#[test]
fn a_horde_with_a_modifier_is_all_elites_of_that_kind_and_the_others_are_not_forced_to_be() {
    rand::srand(11);
    let frenzy = horde_named("Blood Frenzy").unwrap();
    for army in [100, 3000] {
        let horde = make_horde(frenzy, CENTRE, army, army);
        assert!(horde.len() >= frenzy.base.min(frenzy.most) && horde.iter().all(|enemy| enemy.modifier == Modifier::Frenzied), "army {army}");
    }
    for (name, modifier) in [("Wildfire", Modifier::Volatile), ("Armoured Host", Modifier::Armored)] {
        let horde = make_horde(horde_named(name).unwrap(), CENTRE, 2000, 2000);
        assert!(horde.iter().all(|enemy| enemy.modifier == modifier), "{name}");
    }
    //an ordinary horde is elite only now and then, and not at all for a small army
    let tide = make_horde(horde_named("Swarm Tide").unwrap(), CENTRE, 10, 10);
    assert!(tide.iter().all(|enemy| enemy.modifier == Modifier::None));
}

#[test]
fn a_charger_of_a_horde_runs_at_the_army_and_dashes_only_when_it_is_close() {
    let mut fx = Effects::default();
    let mut charger = kind_at("Charger", CENTRE + Vec2::new(4000.0, 0.0));
    join_horde(&mut charger);
    assert!(charger.chasing && charger.aggro < HORDE_AGGRO_TILES * TILE_SIZE, "it keeps its own reach");
    let reach = charger.aggro;
    let mut enemies = vec![charger];
    patrol_free(&mut enemies, CENTRE, 0.1, 1.0, &mut fx);
    assert_eq!(enemies[0].mode, Mode::Roam, "far off it just runs");
    assert!(enemies[0].position.x < CENTRE.x + 4000.0, "and it has run towards the army");
    enemies[0].position = CENTRE + Vec2::new(reach * 0.5, 0.0);
    patrol_free(&mut enemies, CENTRE, 0.1, 1.0, &mut fx);
    assert!(matches!(enemies[0].mode, Mode::Windup { .. }), "close, it shows where it will dash");
    //the stampede is nearly all chargers
    let horde = make_horde(horde_named("Stampede").unwrap(), CENTRE, 3000, 3000);
    assert!(horde.iter().filter(|enemy| enemy.behavior == Behavior::Charger).count() * 2 > horde.len());
}

#[test]
fn shooters_of_a_horde_stop_at_a_distance_and_fire_from_there_instead_of_running_into_the_army() {
    let mut fx = Effects::default();
    for name in ["Spitter", "Mortar"] {
        let mut shooter = kind_at(name, CENTRE + Vec2::new(3000.0, 0.0));
        join_horde(&mut shooter);
        let wanted = shooter.range * CHASE_STANDOFF;
        let mut enemies = vec![shooter];
        for _ in 0..6000 {
            patrol_free(&mut enemies, CENTRE, 0.02, 1.0, &mut fx);
        }
        let distance = enemies[0].position.distance(CENTRE);
        assert!(distance <= wanted + 0.5 && distance > wanted - 8.0, "{name} stopped {distance} px away, and should be at {wanted}");
    }
    //one that is not in a horde is as it always was: it never chases
    let wild = kind_at("Spitter", CENTRE + Vec2::new(3000.0, 0.0));
    assert!(wild.aggro == 0.0 && !wild.chasing);
    let mut enemies = vec![wild];
    patrol_free(&mut enemies, CENTRE, 0.5, 1.0, &mut fx);
    assert!(!enemies[0].chasing);
}

#[test]
fn what_a_horde_splitter_breaks_into_is_still_part_of_the_horde_and_what_a_wild_one_breaks_into_is_not() {
    let mut fx = Effects::default();
    for in_horde in [true, false] {
        let mut splitter = kind_at("Splitter", CENTRE);
        if in_horde {
            join_horde(&mut splitter);
        }
        splitter.health = 0.0;
        let mut enemies = vec![splitter];
        resolve_deaths(&mut enemies, &mut Vec::new(), 100, &Upgrades::default(), &mut fx);
        assert_eq!(enemies.len(), 3);
        for child in &enemies {
            assert_eq!(child.name, "Splitling");
            assert_eq!((in_a_horde(child), child.chasing), (in_horde, in_horde));
        }
    }
}

#[test]
fn a_horde_that_is_destroyed_leaves_a_chest_where_the_last_of_it_fell() {
    rand::srand(21);
    let mut hordes = Hordes::new();
    let mut enemies = Vec::new();
    let army = 800;
    let count = arriving(&mut hordes, "Firing Line", &mut enemies, army);
    let fight = hordes.fight.expect("they are being fought");
    assert_eq!((fight.total, fight.lost, fight.def.name), (count, 0, "Firing Line"));
    assert!(fight.worth > 0.0 && (fight.worth - enemies.iter().map(kill_points).sum::<f32>()).abs() < 0.01);
    //the fight goes on: no chest while any of them is alive
    assert!(hordes.update(&mut enemies, CENTRE, army, army, false, 0.1).is_none() && hordes.chests.is_empty());
    //only one left, somewhere
    let spot = CENTRE + Vec2::new(900.0, -400.0);
    enemies.truncate(1);
    enemies[0].position = spot;
    assert!(hordes.update(&mut enemies, CENTRE, army, army, false, 0.1).is_none());
    assert_eq!(hordes.fight.unwrap().last_at, spot);
    assert_eq!(hordes.status(&enemies).map(|(line, _)| line), Some("HORDE: Firing Line (1 left)".to_owned()));
    enemies.clear();
    match hordes.update(&mut enemies, CENTRE, army, army, false, 0.1) {
        Some(HordeBeat::Broken { name, tier }) => assert_eq!((name, tier), ("Firing Line", 2)),
        _ => panic!("a horde that is gone is broken"),
    }
    assert!(hordes.fight.is_none());
    assert_eq!(hordes.chests.len(), 1);
    let chest = hordes.chests[0];
    assert_eq!((chest.at, chest.tier), (spot, 2));
    assert!((chest.points - fight.worth * CHEST_POINTS_PER_TIER * 2.0).abs() < 0.01);
    assert!(chest.left > CHEST_SECONDS - 1.0);
    //and it is not broken twice
    assert!(hordes.update(&mut enemies, CENTRE, army, army, false, 0.1).is_none() && hordes.chests.len() == 1);
}

#[test]
fn a_horde_that_is_mostly_left_behind_instead_of_destroyed_leaves_nothing() {
    rand::srand(22);
    let mut hordes = Hordes::new();
    let mut enemies = Vec::new();
    let count = arriving(&mut hordes, "Raiding Party", &mut enemies, 300);
    //the army runs off, and every one of them is left far behind
    for enemy in &mut enemies {
        enemy.position = CENTRE + Vec2::new(ENEMY_LEASH_TILES * TILE_SIZE + 1000.0, 0.0);
    }
    assert!(hordes.update(&mut enemies, CENTRE, 300, 300, false, 0.1).is_none());
    assert!(enemies.is_empty() && hordes.fight.is_none() && hordes.chests.is_empty(), "{count} got away");
    //half or more destroyed is enough, though: it only takes the ones that got away off the count
    for (lost, chest) in [(3, true), (5, true), (6, false), (10, false)] {
        let def = horde_named("Swarm Tide").unwrap();
        let mut hordes = Hordes::new();
        hordes.fight = Some(Fight { def, total: 10, lost, worth: 20.0, last_at: CENTRE });
        let beat = hordes.update(&mut Vec::new(), CENTRE, 300, 300, false, 0.1);
        assert_eq!((matches!(beat, Some(HordeBeat::Broken { .. })), hordes.chests.len()), (chest, chest as usize), "{lost} of 10 left behind");
    }
}

#[test]
fn a_horde_that_arrives_while_another_is_still_being_fought_joins_it_and_the_better_decides_the_chest() {
    rand::srand(23);
    let mut hordes = Hordes::new();
    let mut enemies = Vec::new();
    let first = arriving(&mut hordes, "Swarm Tide", &mut enemies, 1000);
    let worth = hordes.fight.unwrap().worth;
    let second = arriving(&mut hordes, "Stampede", &mut enemies, 1000);
    let fight = hordes.fight.unwrap();
    assert_eq!((fight.total, fight.def.name, enemies.len()), (first + second, "Stampede", first + second));
    assert!(fight.worth > worth);
    //a worse one coming later doesn't lower it
    arriving(&mut hordes, "Fission Mob", &mut enemies, 1000);
    assert_eq!(hordes.fight.unwrap().def.name, "Stampede");
}

#[test]
fn a_chest_that_nobody_gets_to_is_gone_after_a_while_and_the_hud_says_what_there_is() {
    let mut hordes = Hordes::new();
    hordes.next_in = 1.0e9;
    hordes.chests.push(Chest::new(CENTRE, 3, 50.0));
    hordes.chests.push(Chest { left: 30.0, ..Chest::new(CENTRE + Vec2::X * 500.0, 1, 10.0) });
    let (line, _) = hordes.status(&[]).expect("a chest is something to say");
    assert_eq!(line, "BRONZE CHEST: 30s left", "the one that will go first");
    hordes.update(&mut Vec::new(), CENTRE, 500, 500, false, 31.0);
    assert_eq!(hordes.chests.len(), 1);
    assert_eq!(hordes.chests[0].tier, 3);
    hordes.update(&mut Vec::new(), CENTRE, 500, 500, false, CHEST_SECONDS);
    assert!(hordes.chests.is_empty() && hordes.status(&[]).is_none());
    //a horde on its way is said first
    hordes.coming = Some(Coming { def: &HORDES[0], at: CENTRE, left: 3.2, other: None });
    assert_eq!(hordes.status(&[]).map(|(line, _)| line), Some("HORDE: Swarm Tide in 4s".to_owned()));
}

//what a horde costs an army that goes for it: one run, the army in a formation round a player who walks at the nearest of the horde, the
//horde made for it and sent at it from where hordes come from, until it is gone or a minute is up
struct HordeFight {
    lost: usize,
    start: usize,
    seconds: f32,
    left: usize,
}

fn fight_a_horde(def: &'static HordeDef, army: usize, formation: Formation, seed: u64) -> HordeFight {
    rand::srand(seed);
    let walls = Walls::default();
    let mut player = Vec2::splat(WORLD_SIZE / 2.0);
    let mut units: Vec<Unit> = (0..army)
        .map(|_| {
            let at = random_position_around(player.x, player.y, 60.0, 700.0);
            Unit::new(at.x, at.y, 400.0)
        })
        .collect();
    let mut enemies = make_horde(def, player + Vec2::new(HORDE_DISTANCE_TILES * TILE_SIZE, 0.0), army, army);
    let mut babies = Vec::new();
    let mut shots = Vec::new();
    let mut hazards: Vec<Hazard> = Vec::new();
    let squads = Squads::new();
    let mut fx = Effects::default();
    let upgrades = Upgrades::default();
    let mut routes = Routes::new(player, &walls);
    let drill = Drill { formation, facing: Vec2::X };
    let dt = 1.0 / 60.0;
    let mut out = HordeFight { lost: 0, start: army, seconds: 0.0, left: 0 };
    let settle_boost = speed_multiplier(units.len());
    for _ in 0..(8.0 / dt) as usize {
        update_units_in(&mut units, player.x, player.y, dt, &walls, &routes.army, settle_boost, &squads, &drill);
    }
    let mut facing = Vec2::X;
    for frame in 0..(60.0 / dt) as usize {
        let boost = speed_multiplier(units.len());
        //the player goes for the nearest of the horde and stops a little way from it, as someone fighting it would
        let nearest = enemies.iter().filter(|enemy| in_a_horde(enemy)).map(|enemy| enemy.position).min_by(|a, b| a.distance_squared(player).total_cmp(&b.distance_squared(player)));
        let before = player;
        if let Some(target) = nearest.filter(|target| target.distance(player) > 250.0) {
            player += (target - player).normalize_or_zero() * 500.0 * dt;
        }
        facing = turn_to_face(facing, (player - before) / dt, dt);
        let drill = Drill { formation, facing };
        routes.update(player, &walls, false);
        update_units_in(&mut units, player.x, player.y, dt, &walls, &routes.army, boost, &squads, &drill);
        terrain_effects(&mut enemies, &walls, dt, &mut fx);
        patrol_enemies(&mut enemies, player, dt, boost, &walls, &routes.foes, &mut fx);
        enemy_abilities(&mut enemies, &units, player, dt, &mut fx);
        shoot_enemies(&mut enemies, &units, &mut shots, dt, &mut fx);
        hazards.append(&mut fx.hazards);
        update_shots(&mut shots, &mut units, &walls, dt, &upgrades, &mut fx);
        update_hazards(&mut hazards, &mut units, &upgrades, dt, &mut fx);
        heal_units(&mut units, dt, &upgrades);
        fight(&mut enemies, &mut units, &mut babies, dt, &upgrades, &mut fx);
        hazards.append(&mut fx.hazards);
        tick_enemy_numbers(&mut enemies, dt, &mut fx);
        out.lost += bury_units(&mut units, &mut fx);
        fx.fallen.clear();
        fx.heal_all = 0.0;
        fx.points = 0.0;
        fx.xp = 0.0;
        let range = baby_pickup_range(units.len(), Vec2::new(1920.0, 1080.0));
        grow_babies(&mut babies, &mut units, player.x, player.y, dt, range, 30000, &mut fx);
        fx.update(dt, player);
        fx.cues.clear();
        fx.recruits.clear();
        out.seconds = frame as f32 * dt;
        if enemies.iter().all(|enemy| !in_a_horde(enemy)) || units.is_empty() {
            break;
        }
    }
    out.left = enemies.iter().filter(|enemy| in_a_horde(enemy)).count();
    out
}

#[test]
fn a_horde_of_each_kind_can_be_fought_and_broken_by_an_army_that_goes_for_it() {
    //a small check that the whole fight works for every kind: an army big enough for the horde, a few seconds' worth of fighting
    for def in HORDES.iter() {
        let army = def.from_army.max(500);
        let run = fight_a_horde(def, army, Formation::Ring, 7);
        assert!(run.lost < run.start, "{} wiped out the whole army of {army}", def.name);
        assert!(run.left == 0, "{}: {} of them were still standing after {:.0} s", def.name, run.left, run.seconds);
    }
}

//how much each horde costs: cargo test --release how_much_each_horde_costs -- --ignored --nocapture
//(HORDE_ARMIES=1000,13000 for some armies, HORDE_ONLY=Stampede for one horde)
#[test]
#[ignore]
fn how_much_each_horde_costs() {
    let armies: Vec<usize> = std::env::var("HORDE_ARMIES").map(|list| list.split(',').filter_map(|n| n.trim().parse().ok()).collect()).unwrap_or_else(|_| vec![300, 1000, 4000, 13000]);
    let only = std::env::var("HORDE_ONLY").ok();
    for def in HORDES.iter().filter(|def| only.as_deref().is_none_or(|name| def.name == name)) {
        for &army in armies.iter().filter(|&&army| army >= def.from_army) {
            let seeds = 3u64;
            let runs: Vec<HordeFight> = (0..seeds).map(|seed| fight_a_horde(def, army, Formation::Ring, 40 + seed)).collect();
            println!(
                "{:14} {:6} units, {:3} enemies: lost {:5.1}% ({}), done in {:4.1} s, {} left",
                def.name,
                army,
                def.count(army),
                runs.iter().map(|run| run.lost as f32 / run.start as f32 * 100.0).sum::<f32>() / seeds as f32,
                runs.iter().map(|run| format!("{:.1}%", run.lost as f32 / run.start as f32 * 100.0)).collect::<Vec<_>>().join(" "),
                runs.iter().map(|run| run.seconds).sum::<f32>() / seeds as f32,
                runs.iter().map(|run| run.left).max().unwrap_or(0)
            );
        }
    }
}
