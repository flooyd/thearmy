//saving a run to a file and reading it back: nothing is lost on the way, a damaged file is refused rather than loaded, and
//the saves are kept in a folder of their own

use super::*;

//a run with a bit of everything in it: every kind of wall, enemy and boss, the sites awake, relics taken and a choice pending
fn busy_world() -> Snapshot {
    let mut world = new_run();
    let mut fx = Effects::default();
    //an army in all states of health, some inside a tower and some sent at an enemy
    world.units.clear();
    for i in 0..300 {
        let mut unit = unit_at(15000.0 + (i % 20) as f32 * 30.0, 15000.0 + (i / 20) as f32 * 30.0);
        unit.health = UNIT_HEALTH - (i % 7) as f32 * 9.0;
        unit.guard = (i % 3) as f32 * 10.0;
        unit.velocity = Vec2::new(i as f32 * 0.5, -(i as f32));
        unit.calm = i as f32 * 0.1;
        if i % 11 == 0 {
            unit.steer = Steer::Point(Vec2::new(123.5, 456.25 + i as f32));
        }
        if i % 13 == 0 {
            unit.target = Some(7);
        }
        if i % 50 == 0 {
            unit.garrison = Some((600, 600));
        }
        world.units.push(unit);
    }
    //three champions among them, one with a timer part way, at different levels and part way to the next
    for (at, name, power, level, xp, evolutions) in [(7, "Juggernaut", 0.0, 1, 0.0, 0), (101, "Phoenix", 41.5, 4, 37.5, 0), (222, "Frostweaver", 3.25, CHAMPION_MAX_LEVEL, 0.0, 0b10)] {
        let unit = &mut world.units[at];
        unit.champion = champion_named(name).unwrap() as u8 + 1;
        unit.power = power;
        unit.level = level;
        unit.xp = xp;
        unit.evolutions = evolutions;
        unit.health = unit.max_health() - 13.0;
    }
    world.babies =(0..40).map(|i| Baby { position: Vec2::new(14000.0 + i as f32 * 17.0, 14500.0), age: i as f32 * 0.37 }).collect();
    //walls of every kind, hurt, wearing out, and a tower with its garrison
    for (i, kind) in [WallKind::Wall, WallKind::Gate, WallKind::Tower, WallKind::Spikes, WallKind::Tar, WallKind::Barrier].into_iter().enumerate() {
        for side in [Side::Friendly, Side::Hostile] {
            let mut wall = Wall::made(600 + i as i32, if side == Side::Friendly { 600 } else { 603 }, kind, side, 400.0 + i as f32);
            wall.health -= 33.3;
            wall.hurt = 0.07 * i as f32;
            wall.life = if kind == WallKind::Barrier || kind == WallKind::Spikes { 12.5 } else { f32::INFINITY };
            wall.garrison = if kind == WallKind::Tower { 4 } else { 0 };
            wall.fire_in = 0.3 * i as f32;
            world.walls.insert((wall.col, wall.row), wall);
        }
    }
    //every nest, a village and the citadel awake, with the walls and the enemies they raise
    let army = world.units.len();
    let mut wanted: Vec<usize> = Vec::new();
    for want in [SiteKind::Nest(0), SiteKind::Nest(1), SiteKind::Nest(2), SiteKind::Nest(3), SiteKind::Village, SiteKind::Barracks, SiteKind::Citadel] {
        if let Some(i) = world.sites.iter().position(|site| site.kind == want) {
            wanted.push(i);
        }
    }
    for i in wanted {
        awaken(&mut world.sites[i], &mut world.enemies, &mut world.walls, army, 0, &mut fx);
    }
    world.sites.iter_mut().find(|site| site.kind == SiteKind::Village && site.state == SiteState::Dormant).unwrap().state = SiteState::Cleared;
    world.sites.iter_mut().find(|site| site.kind == SiteKind::Village && site.state == SiteState::Cleared).unwrap().clock = 17.25;
    //elites of every kind of modifier, a minion, and every boss in a different place in its fight
    for (i, modifier) in MODIFIERS.into_iter().enumerate() {
        let mut elite = kind_at(ENEMY_KINDS[i].name, Vec2::new(5000.0 + i as f32 * 100.0, 5000.0));
        elite.make_elite(modifier);
        elite.mode = [Mode::Windup { dir: Vec2::X, left: 0.4 }, Mode::Dash { dir: Vec2::new(0.6, 0.8), left: 0.2 }, Mode::Recover { left: 1.1 }, Mode::Fuse { left: 0.25 }][i];
        elite.pressing = Some((30 + i as i32, -4));
        elite.chasing = i % 2 == 0;
        elite.steer = Steer::Point(Vec2::new(1.0, 2.0 * i as f32));
        world.enemies.push(elite);
    }
    let owner = world.enemies.last().unwrap().id;
    world.enemies.push(Enemy::minion(Vec2::new(5200.0, 5200.0), kind_named("Imp"), 3.0, owner));
    for (i, def) in BOSSES.iter().enumerate() {
        let mut boss = Enemy::boss(def, 300 + i * 100, i as u32 % 3, Vec2::new(8000.0 + i as f32 * 900.0, 9000.0));
        let mut state = boss.boss.take().unwrap();
        if def.phases.len() > 1 {
            enter_phase(&mut boss, &mut state, 1 + i % (def.phases.len() - 1), army, &mut fx);
        }
        state.last_stand = i % 2 == 1;
        state.generation = i as u32 % 3;
        state.anchor_left = 0.5 * i as f32;
        state.facing = Vec2::new(0.0, -1.0);
        boss.boss = Some(state);
        boss.mode = Mode::Cast { left: 0.8 };
        boss.health *= 0.37;
        world.enemies.push(boss);
    }
    world.director = Director { next_boss_in: 41.5, slain: 5, boss: world.enemies.last().map(|boss| boss.id), warned: true };
    for relic in [Relic::Whetstone, Relic::Freezing, Relic::Freezing, Relic::Tailwind, Relic::GlassCannon, Relic::Overclock, Relic::Aegis, Relic::Martyr] {
        world.upgrades.take(relic);
    }
    world.player = Vec2::new(15123.5, 15456.75);
    world.zoom = 0.73;
    world.run = Run { destroyed: 1234, lost: 567, peak_army: 2890, bosses: 5, nests: 12, villages: 3, seconds: 1801.25, citadels: 2 };
    world.rerolls = 3;
    world.wall_kind = WallKind::Tower;
    world.wall_points = 77.5;
    world.choice = Some(RelicChoice::new());
    //a horde on its way, one being fought, and two chests it left
    world.hordes = Hordes {
        next_in: 33.5,
        coming: Some(Coming { def: horde_named("Iron Column").unwrap(), at: Vec2::new(7000.5, 8000.25), left: 2.5, other: None }),
        fight: Some(Fight { def: horde_named("Firing Line").unwrap(), total: 14, lost: 3, worth: 61.5, last_at: Vec2::new(15200.0, 15300.5) }),
        chests: vec![Chest::new(Vec2::new(14800.0, 15000.0), 2, 40.0), Chest { left: 71.25, ..Chest::new(Vec2::new(15900.5, 15100.0), 3, 90.0) }],
    };
    world
}

#[test]
fn a_saved_run_reads_back_as_the_same_run() {
    let mut world = busy_world();
    //the orders units were given are not kept, so they are not part of what has to come back
    for unit in &mut world.units {
        unit.target = None;
    }
    let loaded = Snapshot::from_text(&world.to_text()).expect("the save reads back");
    assert_eq!(loaded.body(), world.body());
    assert_eq!((loaded.units[7].level, loaded.units[101].level, loaded.units[222].level), (1, 4, CHAMPION_MAX_LEVEL));
    assert_eq!((loaded.units[101].xp, loaded.units[101].health), (37.5, loaded.units[101].max_health() - 13.0));
    assert_eq!(loaded.units[0].level, 0, "an ordinary unit has no level");
    assert_eq!((loaded.units[7].evolutions, loaded.units[101].evolutions, loaded.units[222].evolutions), (0, 0, 0b10));
    assert_eq!(loaded.units.len(), world.units.len());
    assert_eq!(loaded.walls.len(), world.walls.len());
    assert_eq!(loaded.enemies.len(), world.enemies.len());
    assert_eq!(loaded.sites.len(), world.sites.len());
    assert_eq!(loaded.player, Vec2::new(15123.5, 15456.75));
    assert_eq!(loaded.zoom, 0.73);
    assert_eq!((loaded.run.destroyed, loaded.run.bosses, loaded.run.seconds), (1234, 5, 1801.25));
    assert_eq!((loaded.rerolls, loaded.wall_kind, loaded.wall_points), (3, WallKind::Tower, 77.5));
    //and a second trip changes nothing more
    let again = Snapshot::from_text(&loaded.to_text()).unwrap();
    assert_eq!(again.body(), loaded.body());
}

#[test]
fn champions_in_a_save_from_before_they_had_levels_come_back_at_level_one_in_full_health() {
    let text = busy_world().to_text();
    let cut = text.rfind("\nveterans ").expect("a save says what levels the champions have");
    let old = Snapshot::from_text(&text[..cut + 1]).expect("a save without levels still reads");
    for at in [7, 101, 222] {
        let unit = &old.units[at];
        assert_eq!((unit.level, unit.xp, unit.health), (1, 0.0, champion_health(1)), "champion {at}");
    }
    assert_eq!(old.units.iter().filter(|unit| unit.champion != 0).count(), 3);
    //a champion said to have a level that is more than there is, or less, is put right rather than refused
    let wild = text.replacen("\n101 4 37.5", "\n101 99 -5", 1);
    assert_ne!(wild, text, "the line to change is there");
    let loaded = Snapshot::from_text(&wild).expect("a level out of range is put right");
    assert_eq!((loaded.units[101].level, loaded.units[101].xp), (CHAMPION_MAX_LEVEL, 0.0));
    //but a level for a unit that is no champion is a damaged file
    assert!(Snapshot::from_text(&text.replacen("\n101 4 37.5", "\n102 4 37.5", 1)).is_err());
}

#[test]
fn every_number_comes_back_exactly_as_it_went_out() {
    let numbers = [0.0, -0.0, 1.0, 0.1, 1.0 / 3.0, 123456.789, 3.4028235e38, 1.17549435e-38, 1.0e-45, -987.654, f32::INFINITY, f32::NEG_INFINITY];
    let mut w = SaveWriter::new();
    for number in numbers {
        w.put(&number);
    }
    w.put(&f32::NAN);
    let text = w.finish();
    let mut r = SaveReader::new(&text);
    for number in numbers {
        let back: f32 = r.get().unwrap();
        assert_eq!(back.to_bits(), number.to_bits(), "{number}");
    }
    assert!(r.get::<f32>().unwrap().is_nan());
}

#[test]
fn the_relics_taken_add_up_the_same_after_loading() {
    let world = busy_world();
    let loaded = Snapshot::from_text(&world.to_text()).unwrap();
    assert_eq!(format!("{:?}", loaded.upgrades), format!("{:?}", world.upgrades));
    assert_eq!(loaded.upgrades.count(Relic::Freezing), 2);
    assert!((loaded.upgrades.freeze - 2.0 * FREEZE_CHANCE).abs() < 1e-6);
    //the relics on offer are the same three, shown from the start again
    let (before, after) = (world.choice.as_ref().unwrap(), loaded.choice.as_ref().unwrap());
    assert_eq!((after.options, after.rare_only), (before.options, before.rare_only));
    assert_eq!(after.age, 0.0);
}

#[test]
fn units_come_back_with_no_orders_as_the_squads_that_gave_them_are_gone() {
    let world = busy_world();
    assert!(world.units.iter().any(|unit| unit.target.is_some()));
    let loaded = Snapshot::from_text(&world.to_text()).unwrap();
    assert!(loaded.units.iter().all(|unit| unit.target.is_none()));
    //but where each stands, how hurt it is and the tower it is in are kept
    for (before, after) in world.units.iter().zip(&loaded.units) {
        assert_eq!((before.x, before.y, before.health, before.guard, before.garrison), (after.x, after.y, after.health, after.guard, after.garrison));
    }
    assert!(loaded.walls.get(&(602, 600)).is_some_and(|tower| tower.kind == WallKind::Tower && tower.garrison == 4));
}

#[test]
fn new_enemies_and_sites_are_not_given_the_ids_of_the_ones_that_were_saved() {
    let mut world = new_run();
    let mut enemy = kind_at("Grunt", Vec2::new(500.0, 500.0));
    enemy.id = 9_000_000;
    world.enemies.push(enemy);
    world.sites[0].id = 8_000_000;
    let _loaded = Snapshot::from_text(&world.to_text()).unwrap();
    assert!(kind_at("Grunt", Vec2::new(500.0, 500.0)).id > 9_000_000);
    assert!(Site::new(SiteKind::Village, Vec2::new(500.0, 500.0)).id > 8_000_000);
}

#[test]
fn a_loaded_nest_is_still_held_by_its_core_and_a_loaded_boss_by_its_family() {
    let world = busy_world();
    let mut loaded = Snapshot::from_text(&world.to_text()).unwrap();
    let nest = loaded.sites.iter().find(|site| site.state == SiteState::Awake && matches!(site.kind, SiteKind::Nest(_))).unwrap();
    assert!(nest.core.is_some_and(|core| loaded.enemies.iter().any(|enemy| enemy.id == core)));
    let at = nest.position;
    //the player is at the nest, and it carries on as a nest that is still there: nothing is destroyed or liberated
    let mut fx = Effects::default();
    let news = update_sites(&mut loaded.sites, &mut loaded.enemies, &mut loaded.walls, &mut loaded.babies, &mut loaded.units, at, 0.016, false, DEFAULT_ARMY_LIMIT, 0, &mut fx);
    assert_eq!((news.nests, news.citadel), (0, false));
    let nest = loaded.sites.iter().find(|site| site.position == at).unwrap();
    assert_eq!(nest.state, SiteState::Awake);
    //the director is still waiting for the boss it sent, which is in the world with its family
    let waiting = loaded.director.boss.expect("a boss was out");
    assert!(loaded.enemies.iter().any(|enemy| enemy.boss.as_ref().is_some_and(|state| state.family == waiting)));
    assert!(current_boss(&loaded.enemies).is_some());
}

#[test]
fn a_barracks_that_is_awake_comes_back_awake_with_its_soldiers() {
    let world = busy_world();
    let loaded = Snapshot::from_text(&world.to_text()).unwrap();
    let barracks = loaded.sites.iter().find(|site| site.kind == SiteKind::Barracks && site.state == SiteState::Awake).expect("an awake barracks");
    assert!(loaded.enemies.iter().filter(|enemy| enemy.site == Some(barracks.id)).count() >= BARRACKS_GUARDS);
    assert!(!barracks.tiles.is_empty());
    assert_eq!(loaded.sites.iter().filter(|site| site.kind == SiteKind::Barracks).count(), world.sites.iter().filter(|site| site.kind == SiteKind::Barracks).count());
}

#[test]
fn a_save_from_before_there_were_barracks_gets_some_when_it_is_loaded() {
    let mut world = busy_world();
    world.sites.retain(|site| site.kind != SiteKind::Barracks);
    let others = world.sites.len();
    let text = world.to_text();
    let loaded = Snapshot::from_text(&text).unwrap();
    let added = loaded.sites.iter().filter(|site| site.kind == SiteKind::Barracks).count();
    assert!(added > 0 && added <= BARRACKS_COUNT, "{added} were added");
    assert_eq!(loaded.sites.len(), others + added);
    //none of them has the id of a site that was already there, and they all start asleep
    let ids: HashSet<u32> = loaded.sites.iter().map(|site| site.id).collect();
    assert_eq!(ids.len(), loaded.sites.len());
    assert!(loaded.sites.iter().filter(|site| site.kind == SiteKind::Barracks).all(|site| site.state == SiteState::Dormant && site.core.is_none()));
    //a world that has them already isn't given more
    let again = Snapshot::from_text(&loaded.to_text()).unwrap();
    assert_eq!(again.sites.len(), loaded.sites.len());
}

#[test]
fn the_same_walls_make_the_same_file_whatever_order_they_went_up_in() {
    let wall = |col: i32, row: i32| Wall::new(col, row);
    let tiles = [(3, 4), (-1, 9), (7, 7), (0, 0), (3, 3)];
    let forwards: Walls = tiles.iter().map(|&(col, row)| ((col, row), wall(col, row))).collect();
    let backwards: Walls = tiles.iter().rev().map(|&(col, row)| ((col, row), wall(col, row))).collect();
    let (mut a, mut b) = (SaveWriter::new(), SaveWriter::new());
    a.put(&forwards);
    b.put(&backwards);
    assert_eq!(a.finish(), b.finish());
}

#[test]
fn every_name_in_the_game_survives_a_save() {
    let names: Vec<&'static str> = ENEMY_KINDS.iter().map(|kind| kind.name).chain(NESTS.iter().map(|nest| nest.name)).chain(BOSSES.iter().map(|boss| boss.name)).chain([FINAL_BOSS.name]).collect();
    assert!(names.iter().any(|name| name.contains(' ')), "the names with spaces in them are the ones that could go wrong");
    for name in names {
        let mut w = SaveWriter::new();
        w.put(&name);
        let text = w.finish();
        assert_eq!(text.split_whitespace().count(), 1, "{name} is kept as one word");
        let back: &'static str = SaveReader::new(&text).get().unwrap();
        assert_eq!(back, name);
    }
    for def in BOSSES.iter().chain([&FINAL_BOSS]) {
        let mut w = SaveWriter::new();
        w.put(&def);
        let back: &'static BossDef = SaveReader::new(&w.finish()).get().unwrap();
        assert!(std::ptr::eq(back, def), "{}", def.name);
    }
}

#[test]
fn every_kind_of_each_thing_with_a_name_is_kept_under_its_own() {
    //each of these is written and read by name, so a name that is wrong for its thing would load as another
    macro_rules! round_trip {
        ($($value:expr),* $(,)?) => {$(
            let mut w = SaveWriter::new();
            w.put(&$value);
            let back = SaveReader::new(&w.finish()).get().unwrap();
            assert_eq!($value, back);
        )*};
    }
    for relic in RELICS {
        round_trip!(relic);
    }
    for kind in [WallKind::Wall, WallKind::Gate, WallKind::Tower, WallKind::Spikes, WallKind::Tar, WallKind::Barrier] {
        round_trip!(kind);
    }
    for behavior in ENEMY_KINDS.iter().map(|kind| kind.behavior) {
        round_trip!(behavior);
    }
    for modifier in [Modifier::None].into_iter().chain(MODIFIERS) {
        round_trip!(modifier);
    }
    for state in [SiteState::Dormant, SiteState::Awake, SiteState::Cleared] {
        round_trip!(state);
    }
    for side in [Side::Friendly, Side::Hostile] {
        round_trip!(side);
    }
    for kind in [SiteKind::Village, SiteKind::Citadel].into_iter().chain((0..NESTS.len()).map(SiteKind::Nest)) {
        round_trip!(kind);
    }
}

#[test]
fn a_file_that_is_not_a_save_is_refused() {
    let world = busy_world();
    let text = world.to_text();
    let refused = |text: &str| Snapshot::from_text(text).err().map(|error| error.to_string());
    //the first `from` in the save is changed to `to`, which has to be something the save has to begin with
    let changed = |from: &str, to: &str| {
        assert!(text.contains(from), "{from:?} is not in the save");
        refused(&text.replacen(from, to, 1))
    };
    assert!(refused("").is_some());
    assert!(refused("hello there").is_some());
    //another version of the game
    assert!(changed("THE-ARMY-SAVE 1 ", "THE-ARMY-SAVE 99 ").unwrap().contains("another version"));
    //a section gone missing, or a name the game doesn't have
    assert!(changed("walls ", "wall ").is_some());
    assert!(changed("Grunt", "Gnoll").is_some());
    assert!(changed(" Hostile ", " Neutral ").is_some());
    //a number that isn't, a count that is far too big, and a count that is bigger than what follows it
    assert!(changed("player ", "player twelve ").is_some());
    assert!(changed("units ", "units 99999999999 ").is_some());
    assert!(changed("units 300 ", "units 301 ").is_some());
}

#[test]
fn a_save_that_stops_part_way_is_refused_wherever_it_stops() {
    let text = busy_world().to_text();
    let words: Vec<usize> = text.match_indices(' ').map(|(i, _)| i).collect();
    assert!(words.len() > 1000);
    //a save cut short by a crash would end anywhere: none of them may panic, and none but the whole of it may load
    //(a save cut off just where the champions would begin is a save from before there were any, which is whole)
    let old_format = text[..text.find("champions ").unwrap()].trim_end();
    let mut cuts = 0;
    for i in (0..words.len() - 1).step_by(words.len() / 300) {
        let cut = &text[..words[i]];
        if cut.trim_end() == old_format {
            assert!(Snapshot::from_text(cut).is_ok());
        } else {
            assert!(Snapshot::from_text(cut).is_err(), "cut after {} words", i);
        }
        cuts += 1;
    }
    assert!(cuts >= 300);
    assert!(Snapshot::from_text(&text).is_ok());
}

#[test]
fn the_first_line_of_a_save_says_what_is_in_it() {
    let world = busy_world();
    let header = world.header(1_700_000_000.0);
    let info = SaveInfo::from_header(&header).unwrap();
    assert_eq!((info.saved_at, info.army, info.seconds, info.bosses), (1_700_000_000.0, 300, 1801.25, 5));
    assert!(SaveInfo::from_header("THE-ARMY-SAVE 2 1 1 1 1").is_err());
    assert!(SaveInfo::from_header("something else").is_err());
    assert!(SaveInfo::from_header("THE-ARMY-SAVE 1 1700000000 300").is_err());
}

//a folder for a test to keep saves in, empty to start with and gone afterwards
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("the-army-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_save_goes_to_a_file_and_comes_back_and_leaves_nothing_else_behind() {
    let scratch = Scratch::new("round-trip");
    let saves = Saves::in_dir(scratch.0.join("not-made-yet"));
    assert!(saves.newest().is_none());
    assert!(saves.read(Slot::Quick).is_err());
    let world = busy_world();
    saves.write(Slot::Quick, &world.to_text()).unwrap();
    let back = saves.read(Slot::Quick).unwrap();
    assert_eq!(back.units.len(), 300);
    //saving over a save replaces it, and the file written beside it first is gone
    saves.write(Slot::Quick, &new_run().to_text()).unwrap();
    assert_ne!(saves.read(Slot::Quick).unwrap().units.len(), 300);
    let files: Vec<String> = std::fs::read_dir(&saves.path(Slot::Quick).parent().unwrap()).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(files, ["quicksave.sav"]);
    saves.delete(Slot::Quick);
    assert!(saves.newest().is_none());
    //deleting what isn't there is no trouble
    saves.delete(Slot::Auto);
}

#[test]
fn the_title_screen_goes_on_from_the_save_that_was_made_last() {
    let scratch = Scratch::new("newest");
    let saves = Saves::in_dir(&scratch.0);
    let world = busy_world();
    let write = |slot: Slot, at: f64| saves.write(slot, &(world.header(at) + &world.body())).unwrap();
    write(Slot::Auto, 100.0);
    assert_eq!(saves.newest().map(|(slot, _)| slot), Some(Slot::Auto));
    write(Slot::Quick, 200.0);
    assert_eq!(saves.newest().map(|(slot, _)| slot), Some(Slot::Quick));
    write(Slot::Auto, 300.0);
    let (slot, info) = saves.newest().unwrap();
    assert_eq!((slot, info.saved_at, info.army), (Slot::Auto, 300.0, 300));
    //a file that is not a save is not one to go on from
    std::fs::write(saves.path(Slot::Auto), "not a save at all").unwrap();
    assert_eq!(saves.newest().map(|(slot, _)| slot), Some(Slot::Quick));
}

#[test]
fn a_save_made_in_the_background_arrives() {
    let scratch = Scratch::new("background");
    let saves = Saves::in_dir(&scratch.0);
    saves.write_in_background(Slot::Auto, busy_world());
    let start = Instant::now();
    while saves.info(Slot::Auto).is_none() {
        assert!(start.elapsed() < Duration::from_secs(20), "the save never arrived");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(saves.read(Slot::Auto).unwrap().units.len(), 300);
}

#[test]
fn what_a_save_copies_is_what_the_run_had_when_it_was_made() {
    let world = busy_world();
    let mut mason = Mason::new();
    mason.selected = WallKind::Gate;
    mason.points = 12.5;
    let mut run = world.run.clone();
    let snapshot = Snapshot::capture(world.player, 0.9, &run, 2, &world.upgrades, &mason, &world.director, &world.units, &world.babies, &world.walls, &world.sites, &world.enemies, None, true, &world.hordes);
    //the run goes on without the copy changing
    run.destroyed += 100;
    mason.points = 0.0;
    assert_eq!((snapshot.run.destroyed, snapshot.wall_kind, snapshot.wall_points), (1234, WallKind::Gate, 12.5));
    assert_eq!((snapshot.zoom, snapshot.rerolls, snapshot.victory, snapshot.choice.is_none()), (0.9, 2, true, true));
    assert_eq!(snapshot.walls.len(), world.walls.len());
    assert_eq!(snapshot.units.len(), world.units.len());
}

//how big and how slow a save of a full army is, for a look and not a check
#[test]
#[ignore]
fn how_big_and_how_quick_a_full_save_is() {
    let mut world = busy_world();
    world.units = (0..4000).map(|i| unit_at(10000.0 + (i % 80) as f32 * 31.0, 10000.0 + (i / 80) as f32 * 31.0)).collect();
    for i in 0..3000 {
        world.walls.insert((100 + i % 100, 100 + i / 100), Wall::new(100 + i % 100, 100 + i / 100));
    }
    let start = Instant::now();
    let text = world.to_text();
    let wrote = start.elapsed();
    let start = Instant::now();
    let back = Snapshot::from_text(&text).unwrap();
    let read = start.elapsed();
    let start = Instant::now();
    let _copy = Snapshot::capture(world.player, world.zoom, &world.run, world.rerolls, &world.upgrades, &Mason::new(), &world.director, &world.units, &world.babies, &world.walls, &world.sites, &world.enemies, None, false, &world.hordes);
    let copied = start.elapsed();
    println!("{} units, {} walls, {} enemies: {} KB, copied in {copied:?}, written in {wrote:?}, read in {read:?}", back.units.len(), back.walls.len(), back.enemies.len(), text.len() / 1024);
}

#[test]
fn how_many_times_the_citadel_has_fallen_is_kept_and_an_older_save_without_it_still_loads() {
    let mut world = busy_world();
    world.run.citadels = 3;
    let text = world.to_text();
    assert_eq!(Snapshot::from_text(&text).unwrap().run.citadels, 3);
    //the run line used to end after the seconds played
    let at = text.find("\nrerolls").expect("the rerolls come after the run");
    let before = text[..at].trim_end();
    let old = format!("{}{}", before.rsplit_once(' ').unwrap().0, &text[at..]);
    let loaded = Snapshot::from_text(&old).expect("a save from before the citadel came back still loads");
    assert_eq!((loaded.run.citadels, loaded.run.bosses, loaded.run.seconds, loaded.rerolls), (0, 5, 1801.25, 3));
}

#[test]
fn the_hordes_are_kept_with_the_run_the_one_coming_the_one_being_fought_and_the_chests() {
    let world = busy_world();
    let loaded = Snapshot::from_text(&world.to_text()).unwrap();
    let hordes = &loaded.hordes;
    assert_eq!(hordes.next_in, 33.5);
    let coming = hordes.coming.expect("the horde on its way is kept");
    assert_eq!((coming.def.name, coming.at, coming.left), ("Iron Column", Vec2::new(7000.5, 8000.25), 2.5));
    let fight = hordes.fight.expect("and the one being fought");
    assert_eq!((fight.def.name, fight.total, fight.lost, fight.worth, fight.last_at), ("Firing Line", 14, 3, 61.5, Vec2::new(15200.0, 15300.5)));
    assert_eq!(hordes.chests, world.hordes.chests);
    assert_eq!((hordes.chests[0].tier, hordes.chests[1].tier, hordes.chests[1].left), (2, 3, 71.25));
}

#[test]
fn the_members_of_a_horde_in_a_save_are_still_a_horde_when_it_is_read() {
    let mut world = busy_world();
    let mut member = kind_at("Brute", Vec2::new(15300.0, 15300.0));
    join_horde(&mut member);
    world.enemies.push(member);
    let loaded = Snapshot::from_text(&world.to_text()).unwrap();
    assert_eq!(loaded.enemies.iter().filter(|enemy| in_a_horde(enemy)).count(), 1);
}

#[test]
fn a_save_from_before_hordes_were_kept_starts_their_timer_again_and_one_that_names_a_horde_wrongly_is_refused() {
    let text = busy_world().to_text();
    let cut = text.rfind("\nhordes ").expect("a save says what the hordes are doing");
    let old = Snapshot::from_text(&text[..cut + 1]).expect("a save without them still reads");
    assert_eq!(old.hordes.next_in, HORDE_FIRST_SECONDS);
    assert!(old.hordes.coming.is_none() && old.hordes.fight.is_none() && old.hordes.chests.is_empty());
    assert!(text.contains("Iron_Column"));
    assert!(Snapshot::from_text(&text.replacen("Iron_Column", "Iron_Fist", 1)).is_err());
}

#[test]
fn a_choice_of_relics_remembers_that_it_came_from_a_chest_and_an_older_save_without_that_still_loads() {
    let mut world = busy_world();
    world.choice = Some(RelicChoice::from_chest(3));
    let text = world.to_text();
    assert_eq!(Snapshot::from_text(&text).unwrap().choice.map(|choice| choice.chest), Some(3));
    //the offer used to end after the word for whether the army was full
    assert!(text.contains("no 3 victory"), "the chest is the last thing in the offer");
    let old = Snapshot::from_text(&text.replacen("no 3 victory", "no victory", 1)).expect("a save from before chests still loads");
    assert_eq!(old.choice.map(|choice| choice.chest), Some(0));
    //and a boss's relics have none
    world.choice = Some(RelicChoice::new());
    assert_eq!(Snapshot::from_text(&world.to_text()).unwrap().choice.map(|choice| choice.chest), Some(0));
}

#[test]
fn a_save_from_when_there_was_a_barracks_relic_still_loads_without_it() {
    let mut world = busy_world();
    world.choice = Some(RelicChoice { options: [Relic::Whetstone, Relic::Freezing, Relic::Aegis], age: 0.0, rare_only: false, chest: 0 });
    assert_eq!(world.upgrades.count(Relic::Freezing), 2);
    //the relic that is Freezing now was the Barracks then, so a file from then has that word where this one has the other
    let old = world.to_text().replace("Freezing", "Barracks");
    assert!(!old.contains("Freezing") && old.contains("Barracks"));
    let loaded = Snapshot::from_text(&old).expect("a save with a Barracks relic in it loads");
    //the relics taken lose it, the others are the same
    assert_eq!(loaded.upgrades.count(Relic::Freezing), 0);
    assert_eq!(loaded.upgrades.taken.len(), world.upgrades.taken.len() - 2);
    assert_eq!(loaded.upgrades.count(Relic::Tailwind), 1);
    assert_eq!(loaded.upgrades.unit_cap(), DEFAULT_ARMY_LIMIT);
    //a Barracks on offer is a Freezing, so the choice still has three different relics
    assert_eq!(loaded.choice.as_ref().unwrap().options, [Relic::Whetstone, Relic::Freezing, Relic::Aegis]);
    //and the rest of the run is all there
    assert_eq!((loaded.units.len(), loaded.enemies.len(), loaded.walls.len()), (world.units.len(), world.enemies.len(), world.walls.len()));
    //a relic that was never in the game is still a damaged file
    assert!(Snapshot::from_text(&world.to_text().replace("Freezing", "Banana")).is_err());
}

#[test]
fn the_relics_on_offer_from_a_chest_are_kept_with_where_they_came_from() {
    let mut world = busy_world();
    world.choice = Some(RelicChoice::from_chest(2));
    let loaded = Snapshot::from_text(&world.to_text()).unwrap();
    let (before, after) = (world.choice.as_ref().unwrap(), loaded.choice.as_ref().unwrap());
    assert_eq!((after.options, after.rare_only, after.chest), (before.options, before.rare_only, before.chest));
}

#[test]
fn a_freeze_is_not_kept_in_a_save() {
    let mut world = busy_world();
    world.enemies[0].status.frozen = 2.0;
    let loaded = Snapshot::from_text(&world.to_text()).unwrap();
    assert_eq!(loaded.enemies[0].status.frozen, 0.0);
    assert_eq!(loaded.enemies[0].id, world.enemies[0].id, "and it is the same enemy otherwise");
}

#[test]
fn champions_in_a_save_from_before_they_could_evolve_come_back_with_nothing_chosen() {
    let text = busy_world().to_text();
    let cut = text.rfind("\nevolutions ").expect("a save says how the champions have grown");
    let old = Snapshot::from_text(&text[..cut + 1]).expect("a save without evolutions still reads");
    assert!(old.units.iter().all(|unit| unit.evolutions == 0));
    assert_eq!(old.units[222].level, CHAMPION_MAX_LEVEL);
    //a choice that can't have been made is dropped rather than refused, but an evolution for a unit that is no champion is a damaged file
    let (head, tail) = text.split_at(cut);
    let wild = format!("{head}{}", tail.replacen("\n222 2", "\n222 6", 1));
    assert_ne!(wild, text, "the line to change is there");
    assert_eq!(Snapshot::from_text(&wild).expect("a choice out of reach is put right").units[222].evolutions, 0b10);
    assert!(Snapshot::from_text(&format!("{head}{}", tail.replacen("\n222 2", "\n223 2", 1))).is_err());
}
