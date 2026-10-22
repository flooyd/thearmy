//nests that send out enemies and villages held by them, both asleep until the army comes and done with once it has won

use super::*;

fn army_of(count: usize, at: Vec2) -> Vec<Unit> {
    (0..count).map(|_| new_unit(at)).collect()
}

//a nest of a kind, on its own, and what a frame of the sites needs round it
struct Scene {
    sites: Vec<Site>,
    enemies: Vec<Enemy>,
    walls: Walls,
    babies: Vec<Baby>,
    units: Vec<Unit>,
    fx: Effects,
}

impl Scene {
    fn new(kind: SiteKind, army: usize) -> Scene {
        let at = Vec2::new(20000.0, 20000.0);
        Scene { sites: vec![Site::new(kind, at)], enemies: Vec::new(), walls: Walls::default(), babies: Vec::new(), units: army_of(army, at + Vec2::new(0.0, 2000.0)), fx: Effects::default() }
    }

    fn step(&mut self, player: Vec2, dt: f32) -> SiteNews {
        update_sites(&mut self.sites, &mut self.enemies, &mut self.walls, &mut self.babies, &mut self.units, player, dt, false, MAX_UNITS, 0, &mut self.fx)
    }

    fn near(&self) -> Vec2 {
        self.sites[0].position + Vec2::new(0.0, TILE_SIZE * 20.0)
    }

    fn far(&self) -> Vec2 {
        self.sites[0].position + Vec2::new(0.0, TILE_SIZE * (SLEEP_TILES + 10.0))
    }
}

#[test]
fn the_world_has_its_nests_villages_and_a_citadel_spread_out() {
    let sites = generate_sites();
    assert_eq!(sites.iter().filter(|site| site.kind == SiteKind::Citadel).count(), 1);
    let nests = sites.iter().filter(|site| matches!(site.kind, SiteKind::Nest(_))).count();
    let villages = sites.iter().filter(|site| site.kind == SiteKind::Village).count();
    assert!(nests >= NEST_COUNT - 4, "only {nests} nests found room");
    assert!(villages >= VILLAGE_COUNT - 4, "only {villages} villages found room");
    let barracks = sites.iter().filter(|site| site.kind == SiteKind::Barracks).count();
    assert!(barracks >= BARRACKS_COUNT - 3, "only {barracks} barracks found room");
    let centre = Vec2::splat(WORLD_SIZE / 2.0);
    for (i, site) in sites.iter().enumerate() {
        assert!(site.position.x > 0.0 && site.position.x < WORLD_SIZE && site.position.y > 0.0 && site.position.y < WORLD_SIZE);
        assert_eq!(site.state, SiteState::Dormant);
        if site.kind == SiteKind::Citadel {
            //in a corner
            let corner = |v: f32| v < WORLD_SIZE * 0.1 || v > WORLD_SIZE * 0.9;
            assert!(corner(site.position.x) && corner(site.position.y), "the citadel is at {:?}", site.position);
            continue;
        }
        assert!(site.position.distance(centre) >= SITE_START_CLEAR_TILES * TILE_SIZE - 1.0, "a site is on the start");
        for other in sites.iter().skip(i + 1).filter(|other| other.kind != SiteKind::Citadel) {
            assert!(site.position.distance(other.position) >= SITE_SEPARATION_TILES * TILE_SIZE - 1.0, "two sites are close together");
        }
    }
}

#[test]
fn the_nest_kinds_are_made_of_real_enemies() {
    for def in &NESTS {
        assert!(nest_named(def.name).is_some());
        assert_eq!(kind_named(def.child).name, def.child, "{} sends out a {} which isn't a kind", def.name, def.child);
        assert!(def.brood > 0 && def.cap >= def.brood);
    }
    assert_eq!(kind_named("Nest").behavior, Behavior::Nest);
}

#[test]
fn a_nest_sleeps_until_the_army_is_big_enough_and_close() {
    let mut scene = Scene::new(SiteKind::Nest(3), 5); //the bomb cache wants 30
    let near = scene.near();
    scene.step(near, 0.016);
    assert_eq!(scene.sites[0].state, SiteState::Dormant, "five units shouldn't wake a bomb cache");
    assert!(scene.enemies.is_empty());
    let mut scene = Scene::new(SiteKind::Nest(0), 10);
    let far = scene.far();
    scene.step(far, 0.016);
    assert_eq!(scene.sites[0].state, SiteState::Dormant, "too far to wake");
    let near = scene.near();
    scene.step(near, 0.016);
    assert_eq!(scene.sites[0].state, SiteState::Awake);
}

#[test]
fn waking_a_nest_makes_its_core_and_a_ring_of_hostile_walls_with_ways_in() {
    let mut scene = Scene::new(SiteKind::Nest(1), 20);
    let near = scene.near();
    let news = scene.step(near, 0.016);
    assert!(news.walls_changed);
    let nest: Vec<&Enemy> = scene.enemies.iter().filter(|enemy| enemy.behavior == Behavior::Nest).collect();
    assert_eq!(nest.len(), 1);
    assert_eq!(nest[0].name, "Warren");
    assert_eq!(nest[0].site, Some(scene.sites[0].id));
    assert!(!is_wild(nest[0]), "a nest isn't one of the wild enemies that are kept alive");
    assert!(!scene.walls.is_empty());
    assert!(scene.walls.values().all(|wall| wall.side == Side::Hostile));
    //two gaps of two squares: the ring has four sides of 2*half+1 squares less four for the corners that are shared
    let middle = tile_of(scene.sites[0].position.x, scene.sites[0].position.y);
    let half = (scene.walls.keys().map(|&(col, _)| (col - middle.0).abs()).max().unwrap()) as usize;
    assert_eq!(scene.walls.len(), 8 * half - 4, "{half}: a ring with four squares missing");
}

#[test]
fn a_nest_sends_out_its_own_up_to_a_cap_and_only_while_the_player_is_near() {
    let mut scene = Scene::new(SiteKind::Nest(0), 10);
    let near = scene.near();
    scene.step(near, 0.016);
    let id = scene.enemies[0].id;
    let mut fx = Effects::default();
    for _ in 0..20_000 {
        enemy_abilities(&mut scene.enemies, &scene.units, near, 0.05, &mut fx);
    }
    let brood = scene.enemies.iter().filter(|enemy| enemy.owner == Some(id)).count();
    let def = &NESTS[0];
    assert!(brood > def.brood, "the nest should keep sending them: {brood}");
    assert!(brood <= (def.cap + scene.units.len() / 40).min(NEST_MAX_BROOD), "{brood} is over the cap");
    assert!(scene.enemies.iter().filter(|enemy| enemy.owner == Some(id)).all(|enemy| enemy.name == def.child && enemy.babies == 0));
    //nobody near, nothing comes
    let mut scene = Scene::new(SiteKind::Nest(0), 10);
    let near = scene.near();
    scene.step(near, 0.016);
    let far = near + Vec2::new(0.0, TILE_SIZE * 100.0);
    for _ in 0..2000 {
        enemy_abilities(&mut scene.enemies, &scene.units, far, 0.05, &mut fx);
    }
    assert_eq!(scene.enemies.len(), 1);
}

#[test]
fn a_nest_the_player_leaves_goes_back_to_sleep_with_its_brood_and_walls() {
    let mut scene = Scene::new(SiteKind::Nest(0), 10);
    let (near, far) = (scene.near(), scene.far());
    scene.step(near, 0.016);
    let id = scene.enemies[0].id;
    scene.enemies.push(Enemy::minion(scene.sites[0].position, kind_named("Swarmer"), 1.0, id));
    let news = scene.step(far, 0.016);
    assert_eq!(scene.sites[0].state, SiteState::Dormant);
    assert!(scene.enemies.is_empty(), "the nest and what it sent out go");
    assert!(scene.walls.is_empty());
    assert!(news.walls_changed);
    assert_eq!(news.nests, 0, "leaving isn't winning");
    //and it comes back when they do
    scene.step(near, 0.016);
    assert_eq!(scene.sites[0].state, SiteState::Awake);
    assert_eq!(scene.enemies.iter().filter(|enemy| enemy.behavior == Behavior::Nest).count(), 1);
}

#[test]
fn a_destroyed_nest_takes_its_walls_and_brood_with_it_and_pays() {
    let mut scene = Scene::new(SiteKind::Nest(0), 40);
    let near = scene.near();
    scene.step(near, 0.016);
    let id = scene.enemies[0].id;
    scene.enemies.push(Enemy::minion(scene.sites[0].position, kind_named("Swarmer"), 1.0, id));
    //the army kills the core
    scene.enemies.retain(|enemy| enemy.behavior != Behavior::Nest);
    let news = scene.step(near, 0.016);
    assert_eq!(news.nests, 1);
    assert_eq!(scene.sites[0].state, SiteState::Cleared);
    assert!(scene.walls.is_empty(), "the lair comes down");
    assert!(news.walls_changed);
    assert!((scene.fx.points - NEST_REWARD_POINTS).abs() < 0.01);
    assert_eq!(scene.babies.len(), NEST_REWARD_BABIES + 40 / NEST_BABIES_PER);
    //a nest that has been destroyed stays that way
    let before = scene.babies.len();
    scene.step(near, 0.016);
    scene.step(near, 0.016);
    assert_eq!(scene.sites[0].state, SiteState::Cleared);
    assert_eq!(scene.babies.len(), before);
}

#[test]
fn the_walls_of_a_lair_leave_the_players_walls_alone() {
    let mut scene = Scene::new(SiteKind::Nest(0), 10);
    let at = tile_of(scene.sites[0].position.x, scene.sites[0].position.y);
    //a friendly wall right where the ring would be
    scene.walls.insert((at.0 + 5, at.1 + 1), made(WallKind::Wall, Side::Friendly, at.0 + 5, at.1 + 1));
    let near = scene.near();
    scene.step(near, 0.016);
    assert_eq!(scene.walls[&(at.0 + 5, at.1 + 1)].side, Side::Friendly);
    scene.step(scene.far(), 0.016);
    assert_eq!(scene.walls.len(), 1, "only the lair's own walls come down");
    assert_eq!(scene.walls[&(at.0 + 5, at.1 + 1)].side, Side::Friendly);
}

#[test]
fn a_village_has_guards_and_is_liberated_when_they_are_all_gone() {
    let mut scene = Scene::new(SiteKind::Village, 120);
    let near = scene.near();
    scene.step(near, 0.016);
    let guards = scene.enemies.len();
    assert_eq!(guards, 3 + 120 / 60);
    assert!(scene.enemies.iter().all(|enemy| enemy.site == Some(scene.sites[0].id) && !is_wild(enemy)));
    assert!(scene.enemies.iter().all(|enemy| enemy.owner.is_none() && enemy.behavior != Behavior::Healer && enemy.behavior != Behavior::Nest));
    //one left isn't enough
    scene.enemies.truncate(1);
    let news = scene.step(near, 0.016);
    assert_eq!(news.villages, 0);
    scene.enemies.clear();
    let news = scene.step(near, 0.016);
    assert_eq!(news.villages, 1);
    assert_eq!(scene.sites[0].state, SiteState::Cleared);
    assert_eq!(scene.babies.len(), VILLAGE_VILLAGERS + 120 / VILLAGERS_PER);
    assert!((scene.fx.points - VILLAGE_REWARD_POINTS).abs() < 0.01);
    let again = scene.step(near, 0.016);
    assert_eq!(again.villages, 0, "a village is only liberated once");
}

#[test]
fn a_village_that_is_left_before_it_is_free_puts_its_guards_back() {
    let mut scene = Scene::new(SiteKind::Village, 20);
    let (near, far) = (scene.near(), scene.far());
    scene.step(near, 0.016);
    assert!(!scene.enemies.is_empty());
    scene.step(far, 0.016);
    assert!(scene.enemies.is_empty());
    assert_eq!(scene.sites[0].state, SiteState::Dormant);
    scene.step(near, 0.016);
    assert!(!scene.enemies.is_empty(), "they are back when the army is");
}

#[test]
fn a_liberated_village_mends_the_units_near_it_and_sends_volunteers() {
    let mut scene = Scene::new(SiteKind::Village, 20);
    let near = scene.near();
    scene.step(near, 0.016);
    scene.enemies.clear();
    scene.step(near, 0.016);
    scene.babies.clear();
    let at = scene.sites[0].position;
    scene.units = army_of(3, at + Vec2::new(100.0, 0.0));
    scene.units.extend(army_of(3, at + Vec2::new(0.0, TILE_SIZE * 30.0)));
    for unit in &mut scene.units {
        unit.health = 10.0;
    }
    for _ in 0..60 {
        scene.step(near, 0.1);
    }
    assert!(scene.units[..3].iter().all(|unit| unit.health > 100.0 || unit.health >= UNIT_HEALTH), "the ones in the village are mended: {}", scene.units[0].health);
    assert!(scene.units[3..].iter().all(|unit| unit.health == 10.0), "the ones away from it aren't");
    //six seconds is not long enough for a volunteer, and twenty-five is
    assert!(scene.babies.is_empty(), "no volunteer yet");
    for _ in 0..2000 {
        scene.step(near, 0.1);
    }
    assert!(scene.babies.len() <= 3, "they stop at three waiting: {}", scene.babies.len());
    scene.babies.clear();
    for _ in 0..(VILLAGE_TRICKLE_SECONDS * 10.0) as usize + 10 {
        scene.step(near, 0.1);
    }
    assert_eq!(scene.babies.len(), 1);
}

#[test]
fn a_barracks_sleeps_until_the_army_is_big_enough_and_close() {
    let mut scene = Scene::new(SiteKind::Barracks, BARRACKS_MIN_ARMY - 1);
    let near = scene.near();
    scene.step(near, 0.016);
    assert_eq!(scene.sites[0].state, SiteState::Dormant, "too small an army");
    scene.units = army_of(BARRACKS_MIN_ARMY, near);
    scene.step(scene.far(), 0.016);
    assert_eq!(scene.sites[0].state, SiteState::Dormant, "too far away");
    scene.step(near, 0.016);
    assert_eq!(scene.sites[0].state, SiteState::Awake);
}

#[test]
fn a_barracks_is_a_walled_yard_held_by_soldiers_led_by_an_elite_captain() {
    let mut scene = Scene::new(SiteKind::Barracks, 200);
    let near = scene.near();
    scene.step(near, 0.016);
    let id = scene.sites[0].id;
    let soldiers: Vec<&Enemy> = scene.enemies.iter().filter(|enemy| enemy.site == Some(id)).collect();
    assert_eq!(soldiers.len(), scene.enemies.len());
    assert_eq!(soldiers.len(), BARRACKS_GUARDS + 200 / BARRACKS_GUARDS_PER);
    assert!(soldiers.iter().all(|enemy| !is_wild(enemy) && enemy.owner.is_none() && enemy.behavior != Behavior::Healer && enemy.behavior != Behavior::Nest));
    //they stand inside the walls
    assert!(soldiers.iter().all(|enemy| enemy.position.distance(scene.sites[0].position) < TILE_SIZE * BARRACKS_RING as f32));
    assert_eq!(soldiers.iter().filter(|enemy| enemy.modifier != Modifier::None).count(), 1, "one captain");
    //a ring of hostile walls with ways in, that are the site's own to take down
    assert!(scene.walls.len() > 20);
    assert!(scene.walls.values().all(|wall| wall.side == Side::Hostile));
    assert_eq!(scene.walls.len(), scene.sites[0].tiles.len());
    //the smallest army meets the smallest garrison, and a huge one is held to the most
    assert_eq!(barracks_guards(&scene.sites[0], 1).len(), BARRACKS_GUARDS);
    assert_eq!(barracks_guards(&scene.sites[0], 100_000).len(), BARRACKS_MAX_GUARDS);
}

#[test]
fn a_barracks_is_taken_when_its_soldiers_are_all_gone_and_pays_once() {
    let mut scene = Scene::new(SiteKind::Barracks, 120);
    let near = scene.near();
    scene.step(near, 0.016);
    //one left isn't enough
    scene.enemies.truncate(1);
    assert_eq!(scene.step(near, 0.016).barracks, 0);
    scene.enemies.clear();
    let news = scene.step(near, 0.016);
    assert_eq!(news.barracks, 1);
    assert_eq!((news.nests, news.villages, news.citadel), (0, 0, false));
    assert_eq!(scene.sites[0].state, SiteState::Cleared);
    assert!(scene.walls.is_empty(), "the walls come down");
    assert!(news.walls_changed);
    assert!((scene.fx.points - BARRACKS_REWARD_POINTS).abs() < 0.01);
    assert_eq!(scene.babies.len(), BARRACKS_RECRUITS + 120 / BARRACKS_RECRUITS_PER);
    let again = scene.step(near, 0.016);
    assert_eq!((again.barracks, again.barracks_relics), (0, 0), "it is only taken once");
    //and it announces itself
    assert!(scene.fx.announcements.iter().any(|(text, _)| text.starts_with("BARRACKS TAKEN")));
}

#[test]
fn a_barracks_the_player_leaves_before_it_is_taken_puts_its_soldiers_back() {
    let mut scene = Scene::new(SiteKind::Barracks, 60);
    let (near, far) = (scene.near(), scene.far());
    scene.step(near, 0.016);
    assert!(!scene.enemies.is_empty());
    scene.step(far, 0.016);
    assert!(scene.enemies.is_empty() && scene.walls.is_empty());
    assert_eq!(scene.sites[0].state, SiteState::Dormant);
    scene.step(near, 0.016);
    assert!(!scene.enemies.is_empty(), "they are back when the army is");
}

#[test]
fn taking_a_barracks_drops_the_relic_about_as_often_as_the_chance_says() {
    //the line between a roll that drops it and one that doesn't is the chance
    assert!(barracks_relic_dropped(0.0));
    assert!(barracks_relic_dropped(BARRACKS_RELIC_CHANCE - 0.001));
    assert!(!barracks_relic_dropped(BARRACKS_RELIC_CHANCE));
    assert!(!barracks_relic_dropped(0.999));
    //and what the game does when it is taken: the random numbers are shared with the tests that seed them, so a wrong chance has
    //to be wrong on every try to fail
    let taken = |times: usize| {
        (0..times)
            .filter(|_| {
                let mut scene = Scene::new(SiteKind::Barracks, 60);
                let near = scene.near();
                scene.step(near, 0.016);
                scene.enemies.clear();
                let news = scene.step(near, 0.016);
                assert_eq!(news.barracks, 1);
                assert!(news.barracks_relics <= 1);
                news.barracks_relics == 1
            })
            .count() as f32
            / times as f32
    };
    let ok = (0..4).any(|_| (BARRACKS_RELIC_CHANCE - 0.12..BARRACKS_RELIC_CHANCE + 0.12).contains(&taken(300)));
    assert!(ok, "a barracks should drop a relic about {BARRACKS_RELIC_CHANCE} of the time");
}

#[test]
fn a_barracks_relic_that_drops_makes_the_army_bigger_and_stacks() {
    let mut upgrades = Upgrades::default();
    assert_eq!(upgrades.unit_cap(), MAX_UNITS);
    upgrades.take(Relic::Barracks);
    upgrades.take(Relic::Barracks);
    assert_eq!(upgrades.unit_cap(), MAX_UNITS + 2 * BARRACKS_UNITS);
    //it is the relic that the one on the minimap and in the world takes its colour from
    assert_eq!(Relic::Barracks.name(), "Barracks");
}

#[test]
fn sites_can_be_added_to_a_world_that_has_some_without_crowding_them() {
    let mut sites = generate_sites();
    sites.retain(|site| site.kind != SiteKind::Barracks);
    let before = sites.len();
    place_sites(&mut sites, vec![SiteKind::Barracks; 5]);
    assert!(sites.len() > before && sites.len() <= before + 5);
    let centre = Vec2::splat(WORLD_SIZE / 2.0);
    for (i, site) in sites.iter().enumerate().skip(before) {
        assert!(site.position.distance(centre) >= SITE_START_CLEAR_TILES * TILE_SIZE - 1.0);
        for (j, other) in sites.iter().enumerate() {
            assert!(i == j || site.position.distance(other.position) >= SITE_SEPARATION_TILES * TILE_SIZE - 1.0, "two sites are close together");
        }
    }
}

#[test]
fn the_citadel_stays_shut_while_a_boss_is_alive() {
    let mut scene = Scene::new(SiteKind::Citadel, CITADEL_MIN_ARMY);
    let near = scene.near();
    update_sites(&mut scene.sites, &mut scene.enemies, &mut scene.walls, &mut scene.babies, &mut scene.units, near, 0.016, true, MAX_UNITS, 0, &mut scene.fx);
    assert_eq!(scene.sites[0].state, SiteState::Dormant);
}

#[test]
fn rewards_do_not_overfill_the_army() {
    let mut babies = Vec::new();
    spread_babies(&mut babies, Vec2::splat(20000.0), 50, MAX_UNITS - 5, MAX_UNITS, 100.0);
    assert_eq!(babies.len(), 5);
}

#[test]
fn villages_look_the_same_every_time_they_are_drawn() {
    let site = Site::new(SiteKind::Village, Vec2::splat(20000.0));
    let first: Vec<Rect> = village_houses(&site);
    let second: Vec<Rect> = village_houses(&site);
    assert_eq!(first, second);
    assert!(first.iter().all(|house| house.center().distance(site.position) > TILE_SIZE));
}

#[test]
fn the_hud_counts_what_has_been_done() {
    let mut sites = vec![Site::new(SiteKind::Village, Vec2::ZERO), Site::new(SiteKind::Village, Vec2::ZERO), Site::new(SiteKind::Nest(0), Vec2::ZERO)];
    sites[0].state = SiteState::Cleared;
    assert_eq!(site_counts(&sites), ((1, 2), (0, 1), (0, 0)));
    sites.push(Site::new(SiteKind::Barracks, Vec2::ZERO));
    sites.push(Site::new(SiteKind::Barracks, Vec2::ZERO));
    sites[4].state = SiteState::Cleared;
    assert_eq!(site_counts(&sites), ((1, 2), (0, 1), (1, 2)));
}

#[test]
fn taking_a_barracks_sends_a_champion_about_as_often_as_the_chance_says() {
    //the random numbers are shared with the tests that seed them, so a wrong chance has to be wrong on every try to fail
    let taken = |times: usize| {
        let mut both = 0;
        let champions = (0..times)
            .filter(|_| {
                let mut scene = Scene::new(SiteKind::Barracks, 60);
                let near = scene.near();
                scene.step(near, 0.016);
                scene.enemies.clear();
                let news = scene.step(near, 0.016);
                assert_eq!(news.barracks, 1);
                assert!(news.barracks_champions <= 1 && news.barracks_relics <= 1);
                both += (news.barracks_champions + news.barracks_relics > 1) as usize;
                //a champion that is sent is waiting to be handed out where the barracks was, and says so
                assert_eq!(scene.fx.recruits.len(), news.barracks_champions);
                if news.barracks_champions == 1 {
                    assert!(scene.fx.recruits[0].0.distance(scene.sites[0].position) < 1.0 && scene.fx.recruits[0].1 == Odds::Barracks);
                    assert!(scene.fx.announcements.iter().any(|(text, _)| text.contains("CHAMPION")));
                }
                news.barracks_champions == 1
            })
            .count() as f32
            / times as f32;
        assert_eq!(both, 0);
        champions
    };
    let ok = (0..4).any(|_| (BARRACKS_CHAMPION_CHANCE - 0.12..BARRACKS_CHAMPION_CHANCE + 0.12).contains(&taken(300)));
    assert!(ok, "a barracks should send a champion about {BARRACKS_CHAMPION_CHANCE} of the time");
}
