//! the world as a player far from the host is shown it: what the host says (`Viewer`) is put into a world of the player's own (`Replica`), which has to
//! come to be the world the host has, as far as the player can see it, and to hold together whatever it is sent

use super::*;
use std::ops::Range;

const RULES: Rules = Rules { army_limit: DEFAULT_ARMY_LIMIT, difficulty: Difficulty::Normal };
const DT: f32 = 1.0 / 60.0;
const SCREEN: Vec2 = Vec2::new(1920.0, 1080.0);

// ---------- the host's side ----------

struct Host {
    sim: Sim,
    viewers: Vec<Viewer>,
    time: f32,
    boosts: Vec<f32>,
    pickups: Vec<f32>,
}

impl Host {
    //the world at the start of a run with `players` in it, each with the few units a run starts with
    fn new(players: usize) -> Host {
        let mut sim = Sim::from_snapshot(new_run(), RULES);
        for i in 1..players {
            sim.add_player(&format!("Player {}", i + 1)).expect("room for another player");
        }
        Host { viewers: (0..players).map(Viewer::new).collect(), sim, time: 0.0, boosts: vec![1.0; players], pickups: vec![BABY_PICKUP_RANGE; players] }
    }

    //the same with nothing in the world but the players and their armies
    fn bare(players: usize) -> Host {
        let mut host = Host::new(players);
        host.sim.enemies.clear();
        host.sim.sites.clear();
        host.sim.walls.clear();
        host.sim.babies.clear();
        host
    }

    fn step(&mut self, inputs: Vec<PlayerInput>) {
        let step = self.sim.step(inputs, Pace::steady(DT), RULES);
        self.time += DT;
        self.boosts = step.report.boosts.clone();
        self.pickups = step.report.pickup_ranges.clone();
        for viewer in &mut self.viewers {
            viewer.hear(&step.events);
        }
    }

    fn steps(&mut self, count: usize, inputs: impl Fn(usize) -> Vec<PlayerInput>) {
        for i in 0..count {
            self.step(inputs(i));
        }
    }

    //the next message for a player
    fn tick(&mut self, who: usize) -> Vec<u8> {
        self.tick_acking(who, 0)
    }

    fn tick_acking(&mut self, who: usize, ack: u32) -> Vec<u8> {
        let ranges: Vec<Range<usize>> = self.sim.arrange();
        let index = WallIndex::build(&self.sim.walls);
        let frame = Frame { sim: &self.sim, ranges: &ranges, walls: &index, time: self.time, boosts: &self.boosts, pickups: &self.pickups };
        self.viewers[who].tick(&frame, ack)
    }

    //what a player can see of the world, and how far past it they are still sent things
    fn view(&self, who: usize) -> (Rect, Rect) {
        let viewer = &self.viewers[who];
        let interest = interest_of(self.sim.players.list[who].position, viewer.screen, viewer.zoom);
        (interest, grown(interest, INTEREST_STAY))
    }
}

fn idle() -> PlayerInput {
    PlayerInput::idle(Formation::Free, SCREEN)
}

fn walking(direction: Vec2) -> PlayerInput {
    PlayerInput { direction, ..idle() }
}

//everyone standing still
fn all_idle(count: usize) -> impl Fn(usize) -> Vec<PlayerInput> {
    move |_| (0..count).map(|_| idle()).collect()
}

// ---------- the player's side ----------

struct Watcher {
    replica: Replica,
    sim: Sim,
}

impl Watcher {
    fn new() -> Watcher {
        Watcher { replica: Replica::new(), sim: Sim::mirror() }
    }

    //hears a message, and shows everything where the host said it was at that moment
    fn hear(&mut self, bytes: &[u8]) {
        self.replica.apply(&mut self.sim, bytes).expect("the message is understood");
        let now = self.replica.latest();
        self.replica.place_at(&mut self.sim, now, 0.0);
    }
}

//the mirror holds what the host has, as far as the player can see: all of what is in view is there as it is (to a pixel), nothing is there that the
//host does not have, and what is far from view is not
fn assert_mirrors(host: &Host, who: usize, watcher: &Watcher) {
    assert_mirrors_with(host, who, watcher, true);
}

//(a wall that is hurt is sent again no sooner than WALL_REFRESH_SECONDS after the last time, so while a fight is going on its health can be a little out of
//date, which is only looked for when the walls have had time to catch up: `walls_caught_up`)
fn assert_mirrors_with(host: &Host, who: usize, watcher: &Watcher, walls_caught_up: bool) {
    let (interest, stay) = host.view(who);
    let (sim, mirror) = (&host.sim, &watcher.sim);
    assert_eq!(mirror.players.len(), sim.players.len());
    for (a, b) in sim.players.list.iter().zip(&mirror.players.list) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.formation, b.formation);
        assert_eq!(a.color, b.color);
    }
    for owner in 0..sim.players.len() {
        let ours: HashMap<u32, &Unit> = sim.units.iter().filter(|unit| unit.owner as usize == owner).map(|unit| (unit.id, unit)).collect();
        let theirs: HashMap<u32, &Unit> = mirror.units.iter().filter(|unit| unit.owner as usize == owner).map(|unit| (unit.id, unit)).collect();
        assert_eq!(theirs.len(), mirror.units.iter().filter(|unit| unit.owner as usize == owner).count(), "a unit twice");
        for (id, unit) in &ours {
            let at = Vec2::new(unit.x, unit.y);
            match theirs.get(id) {
                Some(copy) => {
                    assert!(owner == who || stay.contains(at), "unit {id} of {owner} is shown but is out of reach");
                    assert!(Vec2::new(copy.x, copy.y).distance(at) < 0.7, "unit {id} is at {:?}, not {at:?}", (copy.x, copy.y));
                    assert!((copy.health - unit.health).abs() <= unit.max_health() / 255.0 + 0.01, "unit {id} health {} not {}", copy.health, unit.health);
                    assert_eq!(copy.champion, unit.champion);
                    assert_eq!(copy.level, unit.level);
                    assert_eq!(copy.evolutions, unit.evolutions);
                    assert_eq!(copy.guard > 0.0, unit.guard > 0.0);
                    assert_eq!(copy.target.is_some(), unit.target.is_some());
                    assert_eq!(copy.garrison.is_some(), unit.garrison.is_some());
                    if owner == who {
                        assert_eq!(copy.target, unit.target);
                    }
                }
                None => assert!(owner != who && !interest.contains(at), "unit {id} of {owner} at {at:?} is in view and not shown"),
            }
        }
        for id in theirs.keys() {
            assert!(ours.contains_key(id), "unit {id} is shown and is not there");
        }
    }
    let ours: HashMap<u32, &Enemy> = sim.enemies.iter().map(|enemy| (enemy.id, enemy)).collect();
    let theirs: HashMap<u32, &Enemy> = mirror.enemies.iter().map(|enemy| (enemy.id, enemy)).collect();
    assert_eq!(theirs.len(), mirror.enemies.len(), "an enemy twice");
    for (id, enemy) in &ours {
        match theirs.get(id) {
            Some(copy) => {
                assert!(always_seen(enemy) || stay.contains(enemy.position), "enemy {id} is shown and is out of reach");
                assert!(copy.position.distance(enemy.position) < 0.7, "enemy {id} is at {:?}, not {:?}", copy.position, enemy.position);
                assert!((copy.health - enemy.health).abs() <= enemy.max_health / 60000.0 + 0.01, "enemy {id} health");
                assert_eq!((copy.size, copy.max_health, copy.armor, copy.attack), (enemy.size, enemy.max_health, enemy.armor, enemy.attack));
                assert_eq!(copy.boss.is_some(), enemy.boss.is_some());
                assert_eq!(copy.chasing, enemy.chasing);
                assert_eq!(copy.site, enemy.site);
            }
            None => assert!(!always_seen(enemy) && !interest.contains(enemy.position), "enemy {id} at {:?} is in view and not shown", enemy.position),
        }
    }
    for id in theirs.keys() {
        assert!(ours.contains_key(id), "enemy {id} is shown and is not there");
    }
    //(the walls in view are all there, and the walls that are there are all the host's)
    for (&(col, row), wall) in &sim.walls {
        let at = tile_center((col, row));
        match mirror.walls.get(&(col, row)) {
            Some(copy) => {
                assert_eq!((copy.kind, copy.side), (wall.kind, wall.side));
                assert!(!walls_caught_up || (copy.health / copy.max_health - wall.health / wall.max_health).abs() <= 1.5 / 255.0, "wall at {col},{row} health");
            }
            None => assert!(!interest.contains(at), "the wall at {col},{row} is in view and not shown"),
        }
    }
    for key in mirror.walls.keys() {
        assert!(sim.walls.contains_key(key), "the wall at {key:?} is shown and is not there");
    }
    let keys = |babies: &[Baby]| -> HashSet<u32> { babies.iter().map(|baby| baby_key(baby.position)).collect() };
    let (ours, theirs) = (keys(&sim.babies), keys(&mirror.babies));
    assert!(theirs.is_subset(&ours), "a baby is shown that is not there");
    for baby in &sim.babies {
        assert!(theirs.contains(&baby_key(baby.position)) || !interest.contains(baby.position), "a baby in view at {:?} is not shown", baby.position);
    }
    assert_eq!(mirror.sites.len(), sim.sites.len());
    for (a, b) in sim.sites.iter().zip(&mirror.sites) {
        assert_eq!((a.id, a.state, a.core), (b.id, b.state, b.core));
    }
    assert_eq!(mirror.shots.len(), sim.shots.iter().filter(|shot| interest.contains(shot.position)).count());
    assert_eq!(mirror.hordes.chests.len(), sim.hordes.chests.len());
}

// ---------- a world is mirrored ----------

#[test]
fn the_first_message_gives_a_watcher_the_world_as_the_host_has_it_in_view() {
    let mut host = Host::new(2);
    host.steps(30, all_idle(2));
    let mut watcher = Watcher::new();
    assert!(!watcher.replica.ready());
    watcher.hear(&host.tick(0));
    assert!(watcher.replica.ready());
    assert_eq!(watcher.replica.me, 0);
    assert_mirrors(&host, 0, &watcher);
    assert!(!watcher.sim.units.is_empty() && !watcher.sim.enemies.is_empty() && !watcher.sim.sites.is_empty());
    let mut other = Watcher::new();
    other.hear(&host.tick(1));
    assert_eq!(other.replica.me, 1);
    assert_mirrors(&host, 1, &other);
}

#[test]
fn a_watcher_keeps_up_with_a_world_that_is_going_on() {
    let mut host = Host::new(2);
    let mut watcher = Watcher::new();
    for round in 0..120 {
        //(they walk about, the one back and forth and the other round, so that there are fights and the armies come and go)
        let a = Vec2::from_angle(round as f32 * 0.07);
        let b = Vec2::from_angle(-(round as f32) * 0.11);
        host.steps(3, |_| vec![walking(a), walking(b)]);
        watcher.hear(&host.tick(0));
        assert_mirrors_with(&host, 0, &watcher, false);
    }
    assert!(host.sim.run.seconds > 5.0);
    //(and left alone for a moment, the walls are as they are on the host)
    host.time += WALL_REFRESH_SECONDS;
    watcher.hear(&host.tick(0));
    assert_mirrors(&host, 0, &watcher);
}

#[test]
fn a_message_when_nothing_has_moved_is_small() {
    let mut host = Host::bare(2);
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    host.time += SNAPSHOT_SECONDS;
    watcher.hear(&host.tick(0));
    //(the clock, the players, the player's own relics and nothing for the units, which have not moved)
    host.time += SNAPSHOT_SECONDS;
    let quiet = host.tick(0);
    assert!(quiet.len() < 200, "{} bytes for nothing", quiet.len());
    watcher.hear(&quiet);
    assert_mirrors(&host, 0, &watcher);
}

#[test]
fn only_the_units_that_moved_are_sent_again() {
    let mut host = Host::bare(2);
    let me = host.sim.players.list[0].position;
    for i in 0..200 {
        host.sim.units.push(unit_at(me.x + (i % 20) as f32 * 30.0, me.y + 100.0 + (i / 20) as f32 * 30.0).owned_by(0));
    }
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    host.time += SNAPSHOT_SECONDS;
    let quiet = host.tick(0);
    let still = quiet.len();
    watcher.hear(&quiet);
    //(a tenth of the player's army moves)
    let mine: Vec<usize> = (0..host.sim.units.len()).filter(|&i| host.sim.units[i].owner == 0).collect();
    let moved = mine.len().div_ceil(10);
    for &i in &mine[..moved] {
        host.sim.units[i].x += 25.0;
    }
    host.time += SNAPSHOT_SECONDS;
    let message = host.tick(0);
    assert!(message.len() > still + 3 * moved, "{} bytes for {moved} moved, {still} for none", message.len());
    assert!(message.len() < still + 8 * moved + 16, "{} bytes for {moved} moved, {still} for none", message.len());
    watcher.hear(&message);
    assert_mirrors(&host, 0, &watcher);
}

#[test]
fn what_is_out_of_view_is_not_sent_and_is_when_it_comes_into_view() {
    let mut host = Host::bare(1);
    let me = host.sim.players.list[0].position;
    let (interest, stay) = host.view(0);
    let far = Vec2::new(interest.right() + 4000.0, me.y);
    let near = me + Vec2::new(900.0, 0.0);
    let mut distant = kind_at("Grunt", far);
    let mut close = kind_at("Grunt", near);
    distant.aggro = 0.0;
    close.aggro = 0.0;
    host.sim.enemies = vec![distant, close];
    host.sim.babies = vec![Baby { position: far, age: 0.0 }, Baby { position: near, age: 1.0 }];
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    assert_eq!(watcher.sim.enemies.len(), 1);
    assert_eq!(watcher.sim.babies.len(), 1);
    assert_mirrors(&host, 0, &watcher);
    //it comes into view
    host.sim.enemies[0].position = near + Vec2::new(0.0, 300.0);
    host.sim.babies[0].position = near + Vec2::new(0.0, 300.0);
    watcher.hear(&host.tick(0));
    assert_eq!(watcher.sim.enemies.len(), 2);
    assert_eq!(watcher.sim.babies.len(), 2);
    assert_mirrors(&host, 0, &watcher);
    //it goes past the edge a little, which is not enough to take it away, and then far
    host.sim.enemies[0].position = Vec2::new((interest.right() + stay.right()) / 2.0, me.y);
    watcher.hear(&host.tick(0));
    assert_eq!(watcher.sim.enemies.len(), 2, "taken away as soon as it stepped out");
    host.sim.enemies[0].position = far;
    host.sim.babies[0].position = far;
    watcher.hear(&host.tick(0));
    assert_eq!(watcher.sim.enemies.len(), 1);
    assert_eq!(watcher.sim.babies.len(), 1);
    assert_mirrors(&host, 0, &watcher);
}

#[test]
fn a_boss_is_shown_wherever_it_is() {
    let mut host = Host::bare(1);
    let me = host.sim.players.list[0].position;
    let boss = Enemy::boss(&BOSSES[0], 500, 0, me + Vec2::new(30000.0, 0.0));
    host.sim.enemies = vec![boss];
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    assert_eq!(watcher.sim.enemies.len(), 1);
    let shown = &watcher.sim.enemies[0];
    assert!(shown.boss.is_some());
    assert_eq!(shown.position.distance(host.sim.enemies[0].position) < 1.0, true);
    assert_mirrors(&host, 0, &watcher);
}

#[test]
fn an_enemy_that_dies_is_gone_and_one_that_changes_is_changed() {
    let mut host = Host::bare(1);
    let me = host.sim.players.list[0].position;
    host.sim.enemies = (0..5).map(|i| {
        let mut enemy = kind_at("Grunt", me + Vec2::new(500.0 + 80.0 * i as f32, 200.0));
        enemy.aggro = 0.0;
        enemy
    }).collect();
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    assert_eq!(watcher.sim.enemies.len(), 5);
    let died = host.sim.enemies.remove(1).id;
    host.sim.enemies[2].health *= 0.5;
    host.sim.enemies[2].max_health *= 3.0;
    host.sim.enemies[3].status.frozen = 2.0;
    watcher.hear(&host.tick(0));
    assert!(watcher.sim.enemies.iter().all(|enemy| enemy.id != died));
    assert_eq!(watcher.sim.enemies.len(), 4);
    assert_mirrors(&host, 0, &watcher);
    assert!(watcher.sim.enemies[3].status.frozen > 0.0);
    //a new one comes
    let mut late = kind_at("Grunt", me + Vec2::new(700.0, -300.0));
    late.aggro = 0.0;
    host.sim.enemies.push(late);
    watcher.hear(&host.tick(0));
    assert_eq!(watcher.sim.enemies.len(), 5);
    assert_mirrors(&host, 0, &watcher);
}

#[test]
fn walls_are_built_hurt_and_broken_in_view_and_forgotten_out_of_it() {
    let mut host = Host::bare(1);
    let me = host.sim.players.list[0].position;
    let (col, row) = tile_of(me.x + 400.0, me.y);
    for i in 0..6 {
        host.sim.walls.insert((col + i, row), Wall::new(col + i, row));
    }
    host.step(vec![idle()]);
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    assert_eq!(watcher.sim.walls.len(), 6);
    assert_mirrors(&host, 0, &watcher);
    //one is hurt, which is told after a moment, and one broken
    host.sim.walls.get_mut(&(col, row)).unwrap().health *= 0.4;
    host.sim.walls.remove(&(col + 1, row));
    host.time += 1.0;
    watcher.hear(&host.tick(0));
    assert_eq!(watcher.sim.walls.len(), 5);
    assert_mirrors(&host, 0, &watcher);
    //the player goes far away, so the walls are forgotten, and they are broken while the player is away
    host.sim.players.list[0].position += Vec2::new(0.0, 12000.0);
    host.time += 1.0;
    watcher.hear(&host.tick(0));
    assert!(watcher.sim.walls.is_empty(), "{} walls kept", watcher.sim.walls.len());
    host.sim.walls.clear();
    host.sim.players.list[0].position -= Vec2::new(0.0, 12000.0);
    host.time += 1.0;
    watcher.hear(&host.tick(0));
    assert!(watcher.sim.walls.is_empty(), "the walls that were broken while away are still shown");
    assert_mirrors(&host, 0, &watcher);
}

#[test]
fn the_sites_are_told_once_and_then_only_when_they_change() {
    let mut host = Host::new(1);
    host.step(vec![idle()]);
    let mut watcher = Watcher::new();
    let first = host.tick(0);
    watcher.hear(&first);
    host.time += 1.0;
    let second = host.tick(0);
    assert!(second.len() < first.len() / 4, "{} then {}", first.len(), second.len());
    watcher.hear(&second);
    let site = host.sim.sites.iter_mut().find(|site| site.state == SiteState::Dormant).expect("a site asleep");
    site.state = SiteState::Cleared;
    host.time += 1.0;
    watcher.hear(&host.tick(0));
    assert_mirrors(&host, 0, &watcher);
}

#[test]
fn a_player_is_told_what_they_have_chosen_and_not_what_the_others_have() {
    let mut host = Host::bare(2);
    host.sim.players.list[0].upgrades.take(Relic::Whetstone);
    host.sim.players.list[1].choice = Some(RelicChoice::new());
    host.sim.players.list[1].rerolls = 3;
    host.step(vec![idle(), idle()]);
    let (mut a, mut b) = (Watcher::new(), Watcher::new());
    a.hear(&host.tick(0));
    b.hear(&host.tick(1));
    assert!(a.sim.players.list[0].upgrades.hit() > Upgrades::default().hit());
    assert!(a.sim.players.list[1].choice.is_some(), "that they are choosing is said");
    assert!(b.sim.players.list[1].choice.is_some());
    assert_eq!(b.sim.players.list[1].choice.as_ref().map(|choice| choice.options), host.sim.players.list[1].choice.as_ref().map(|choice| choice.options));
    assert_eq!(b.sim.players.list[1].rerolls, 3);
    assert_eq!(a.sim.players.list[1].rerolls, 0, "the other's rerolls are theirs");
    assert_eq!(a.sim.players.list[0].choice.is_some(), false);
}

#[test]
fn what_happened_in_view_is_heard_and_what_happened_far_away_is_not() {
    let mut host = Host::bare(1);
    let me = host.sim.players.list[0].position;
    host.step(vec![idle()]);
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    watcher.replica.take_events();
    let heard = vec![Event::Cue { cue: Cue::Pop, at: None }];
    host.viewers[0].hear(&heard);
    host.viewers[0].hear(&[Event::Popup { at: me + Vec2::new(100.0, 0.0), lane: 0.0, text: "near".to_string(), color: WHITE }, Event::Popup { at: me + Vec2::new(40000.0, 0.0), lane: 0.0, text: "far".to_string(), color: WHITE }]);
    watcher.hear(&host.tick(0));
    let events = watcher.replica.take_events();
    assert!(events.iter().any(|event| matches!(event, Event::Cue { cue: Cue::Pop, .. })));
    assert!(events.iter().any(|event| matches!(event, Event::Popup { text, .. } if text == "near")));
    assert!(!events.iter().any(|event| matches!(event, Event::Popup { text, .. } if text == "far")));
    assert!(watcher.replica.take_events().is_empty(), "heard twice");
}

// ---------- what is not a message ----------

#[test]
fn a_message_that_is_cut_short_or_has_something_after_it_is_not_taken() {
    let mut host = Host::bare(2);
    host.step(vec![idle(), idle()]);
    let first = host.tick(0);
    for cut in (0..first.len()).step_by(7) {
        let mut watcher = Watcher::new();
        assert!(watcher.replica.apply(&mut watcher.sim, &first[..cut]).is_err(), "taken at {cut} of {}", first.len());
    }
    let mut longer = first.clone();
    longer.push(0);
    let mut watcher = Watcher::new();
    assert!(watcher.replica.apply(&mut watcher.sim, &longer).is_err());
    assert!(Watcher::new().replica.apply(&mut Sim::mirror(), &[]).is_err());
    assert!(Watcher::new().replica.apply(&mut Sim::mirror(), &[0x54]).is_err());
}

#[test]
fn messages_that_are_out_of_order_or_from_nowhere_are_not_taken() {
    let mut host = Host::bare(2);
    host.step(vec![idle(), idle()]);
    let (first, second, third) = (host.tick(0), host.tick(0), host.tick(0));
    let mut watcher = Watcher::new();
    assert!(watcher.replica.apply(&mut watcher.sim, &second).is_err(), "the world cannot start from a change");
    let mut watcher = Watcher::new();
    watcher.hear(&first);
    assert!(watcher.replica.apply(&mut watcher.sim, &third).is_err(), "one was skipped");
    let mut watcher = Watcher::new();
    watcher.hear(&first);
    assert!(watcher.replica.apply(&mut watcher.sim, &first).is_ok(), "a host that starts the world again is believed");
    assert_mirrors(&host, 0, &watcher);
    let mut watcher = Watcher::new();
    watcher.hear(&first);
    watcher.hear(&second);
    watcher.hear(&third);
}

#[test]
fn no_message_that_has_been_damaged_takes_the_player_down() {
    let mut host = Host::new(2);
    host.steps(20, |i| vec![walking(Vec2::from_angle(i as f32 * 0.3)), idle()]);
    host.sim.sites.truncate(6);
    let first = host.tick(0);
    host.steps(6, |i| vec![walking(Vec2::from_angle(i as f32 * 0.3)), idle()]);
    let second = host.tick(0);
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for round in 0..120 {
        //(each time a fresh watcher is told the first, and then something that is not quite the second)
        let mut watcher = Watcher::new();
        watcher.hear(&first);
        let mut damaged = second.clone();
        for _ in 0..1 + round % 4 {
            let at = next() as usize % damaged.len();
            damaged[at] ^= 1 << (next() % 8);
        }
        if round % 5 == 0 {
            damaged.truncate(next() as usize % damaged.len());
        }
        //(it may be taken, as a flipped bit can be a different number, but it must not panic or run away with memory)
        let _ = watcher.replica.apply(&mut watcher.sim, &damaged);
    }
    let mut noise = Writer::new();
    for _ in 0..4000 {
        noise.u8(next() as u8);
    }
    for round in 0..200 {
        let mut watcher = Watcher::new();
        let mut bytes = noise.bytes[round..round + 1000].to_vec();
        bytes[0] = DOWN_TICK;
        let _ = watcher.replica.apply(&mut watcher.sim, &bytes);
    }
}

// ---------- time ----------

#[test]
fn something_that_was_told_where_it_is_moves_there_over_the_time_between() {
    let mut track = Track::at(Vec2::new(0.0, 0.0), 0.0);
    track.told(Vec2::new(10.0, 0.0), 0.05);
    assert_eq!(track.place(0.0), Vec2::ZERO);
    assert!((track.place(0.025).x - 5.0).abs() < 1e-4);
    assert_eq!(track.place(0.05), Vec2::new(10.0, 0.0));
    assert_eq!(track.place(5.0), Vec2::new(10.0, 0.0), "it stays where it was last said to be");
    track.told(Vec2::new(30.0, 0.0), 0.10);
    assert!((track.place(0.075).x - 20.0).abs() < 1e-4, "it goes on from where it was last told");
    assert!((track.velocity(0.07).x - 400.0).abs() < 1e-2);
    //it was still for a while and then it moved: it was there until just before
    let mut track = Track::at(Vec2::ZERO, 0.0);
    track.told(Vec2::new(8.0, 0.0), 3.0);
    assert_eq!(track.place(2.9), Vec2::ZERO);
    assert!(track.place(2.99).x > 0.0 && track.place(2.99).x < 8.0);
}

#[test]
fn the_picture_is_kept_a_little_behind_the_host_and_does_not_run_away_from_it() {
    let mut host = Host::bare(1);
    let mut watcher = Watcher::new();
    host.step(vec![idle()]);
    watcher.hear(&host.tick(0));
    assert!((watcher.replica.lag() - INTERP_DELAY).abs() < 0.01 || watcher.replica.lag() == 0.0);
    //frames come at 120 a second and messages at 20, for a while
    for i in 0..600 {
        if i % 6 == 0 {
            host.steps(3, all_idle(1));
            watcher.replica.apply(&mut watcher.sim, &host.tick(0)).unwrap();
        }
        watcher.replica.advance(&mut watcher.sim, 1.0 / 120.0);
        let lag = watcher.replica.lag();
        assert!(lag > 0.0 && lag < 0.4, "{lag} behind at {i}");
    }
}

#[test]
fn a_unit_the_host_moves_at_a_steady_pace_is_shown_moving_at_that_pace() {
    let mut host = Host::bare(1);
    host.sim.units.truncate(1);
    host.sim.units[0].x = 20000.0;
    host.sim.units[0].y = 20000.0;
    host.sim.players.list[0].position = Vec2::new(20000.0, 20000.0);
    let mut watcher = Watcher::new();
    let mut seen = Vec::new();
    for i in 0..60 {
        host.time += SNAPSHOT_SECONDS;
        host.sim.units[0].x += 10.0;
        watcher.replica.apply(&mut watcher.sim, &host.tick(0)).unwrap();
        for _ in 0..6 {
            watcher.replica.advance(&mut watcher.sim, SNAPSHOT_SECONDS / 6.0);
            if i >= 20 {
                seen.push(watcher.sim.units[0].x);
            }
        }
    }
    //(every frame the unit is a little further on, with no jumps and no standing still)
    for pair in seen.windows(2) {
        let step = pair[1] - pair[0];
        assert!(step > 0.8 && step < 2.6, "a frame of {step}");
    }
}

// ---------- the player's own avatar ----------

#[test]
fn the_avatar_goes_where_the_player_says_at_once_and_is_put_right_gently_when_the_host_disagrees() {
    let mut host = Host::bare(1);
    host.step(vec![idle()]);
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    let start = watcher.replica.predicted;
    assert!(start.distance(host.sim.players.list[0].position) < 1.0);
    //it walks for three inputs, without waiting to hear
    for seq in 1..=3 {
        watcher.replica.walked(&watcher.sim, Vec2::X, 1.0 / 60.0);
        watcher.replica.sent(seq);
    }
    let walked = watcher.replica.predicted.x - start.x;
    assert!(walked > 30.0, "{walked} px in three frames");
    //the host has acted on the first of them only, and has them go the same distance in each
    host.sim.players.list[0].position.x += walked / 3.0;
    let message = host.tick_acking(0, 1);
    watcher.replica.apply(&mut watcher.sim, &message).unwrap();
    assert!(watcher.replica.predicted.x > start.x + walked * 0.9, "pulled back though the host agrees: {}", watcher.replica.predicted.x - start.x);
    assert!(watcher.replica.predicted.x < start.x + walked * 1.1);
}

#[test]
fn an_avatar_the_host_has_put_far_from_where_it_was_thought_to_be_is_put_there_at_once() {
    let mut host = Host::bare(1);
    host.step(vec![idle()]);
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    for seq in 1..=4 {
        watcher.replica.walked(&watcher.sim, Vec2::X, 1.0 / 60.0);
        watcher.replica.sent(seq);
    }
    //(a gravity well, or a wall in the way that was not known about)
    host.sim.players.list[0].position += Vec2::new(0.0, 900.0);
    let message = host.tick_acking(0, 2);
    watcher.replica.apply(&mut watcher.sim, &message).unwrap();
    assert!((watcher.replica.predicted.y - host.sim.players.list[0].position.y).abs() < 1.0);
}

#[test]
fn a_small_difference_is_made_up_over_a_few_messages_and_is_gone() {
    let mut host = Host::bare(1);
    host.step(vec![idle()]);
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    host.sim.players.list[0].position += Vec2::new(0.0, 20.0);
    let target = host.sim.players.list[0].position;
    let mut errors = Vec::new();
    for _ in 0..8 {
        host.time += SNAPSHOT_SECONDS;
        watcher.replica.apply(&mut watcher.sim, &host.tick_acking(0, 0)).unwrap();
        errors.push((watcher.replica.predicted - target).length());
    }
    assert!(errors[0] < 20.0 && errors[0] > 1.0, "{errors:?}");
    assert!(errors.windows(2).all(|pair| pair[1] <= pair[0]), "{errors:?}");
    assert!(errors[7] < 1.0, "{errors:?}");
}

// ---------- what it costs ----------

//the size of a message and the time it takes to make and to take in, for armies of 12500 (the most a player has) that are all moving, as they are when a fight
//is on. run with `--ignored --nocapture`
#[test]
#[ignore]
fn where_the_bandwidth_and_time_go() {
    for (players, army) in [(2usize, 3000usize), (4, 12_500)] {
        let mut host = Host::bare(players);
        for owner in 0..players {
            let at = host.sim.players.list[owner].position;
            host.sim.units.retain(|unit| unit.owner as usize != owner);
            for i in 0..army {
                let side = (army as f32).sqrt().ceil() as usize;
                host.sim.units.push(unit_at(at.x + (i % side) as f32 * 18.0 - side as f32 * 9.0, at.y + (i / side) as f32 * 18.0 - side as f32 * 9.0).owned_by(owner));
            }
        }
        let mut watcher = Watcher::new();
        let started = Instant::now();
        let first = host.tick(0);
        let made = started.elapsed();
        let started = Instant::now();
        watcher.hear(&first);
        println!("{players} players with {army} units each: first message {} bytes, made in {made:?}, taken in in {:?}", first.len(), started.elapsed());
        let (mut bytes, mut make, mut take) = (0usize, Duration::ZERO, Duration::ZERO);
        let rounds = 40;
        for round in 0..rounds {
            for unit in host.sim.units.iter_mut() {
                let (dx, dy) = ((round as f32 * 0.37 + unit.id as f32).sin() * 5.0, (round as f32 * 0.21 + unit.id as f32).cos() * 5.0);
                unit.x += dx;
                unit.y += dy;
            }
            host.time += SNAPSHOT_SECONDS;
            let started = Instant::now();
            let message = host.tick(0);
            make += started.elapsed();
            bytes += message.len();
            let started = Instant::now();
            watcher.hear(&message);
            take += started.elapsed();
        }
        let per = bytes / rounds;
        println!("  everything moving: {per} bytes a message = {:.2} MB/s at {} a second, made in {:?}, taken in in {:?}", per as f64 * (1.0 / SNAPSHOT_SECONDS as f64) / 1e6, (1.0 / SNAPSHOT_SECONDS) as u32, make / rounds as u32, take / rounds as u32);
    }
}

#[test]
fn how_a_champion_has_grown_is_seen_by_the_other_players() {
    let mut unit = champion_unit(champion_named("Stormcaller").unwrap(), MIDDLE);
    unit.level = CHAMPION_MAX_LEVEL;
    unit.evolutions = 0b10;
    let look = UnitLook::of(&unit, false);
    let mut out = Writer::new();
    look.write(&mut out);
    let read = UnitLook::read(&mut Reader::new(&out.bytes)).expect("a look reads back");
    assert_eq!(read, look);
    let mut copy = Unit::replica(unit.id, 0, MIDDLE);
    read.apply(&mut copy);
    assert_eq!((copy.level, copy.evolutions), (CHAMPION_MAX_LEVEL, 0b10));
    assert_eq!(copy.champion_power().map(|power| format!("{power:?}")), unit.champion_power().map(|power| format!("{power:?}")));
    //a look that says a champion chose at a level it hasn't reached is put right
    let mut out = Writer::new();
    UnitLook { level: CHAMPION_MAX_LEVEL - 1, ..look }.write(&mut out);
    assert_eq!(UnitLook::read(&mut Reader::new(&out.bytes)).expect("it reads").evolutions, 0);
}

#[test]
fn a_champions_volley_reaches_a_player_as_one_event_and_flies_there_hurting_nothing() {
    let mut host = Host::bare(1);
    let at = host.sim.players.list[0].position;
    let mut champion = champion_unit(champion_named("Sharpshooter").unwrap(), at);
    champion.power = 0.0;
    host.sim.units.push(champion);
    let mut target = kind_at("Grunt", at + Vec2::new(300.0, 0.0));
    target.max_health = 1.0e9;
    target.health = 1.0e9;
    target.attack = 0.0;
    host.sim.enemies.push(target);
    host.step(vec![idle()]);
    assert!(!host.sim.bolts.is_empty(), "the host flies its round");
    let mut watcher = Watcher::new();
    watcher.hear(&host.tick(0));
    let events = watcher.replica.take_events();
    assert_eq!(events.iter().filter(|event| matches!(event, Event::Volley(_))).count(), 1, "{events:?}");
    launch_told(&mut watcher.sim.bolts, &events);
    assert_eq!(watcher.sim.bolts.len(), 1);
    let health: Vec<f32> = watcher.sim.enemies.iter().map(|enemy| enemy.health).collect();
    assert!(!health.is_empty(), "the player sees the enemy");
    for _ in 0..30 {
        fly_bolts(&mut watcher.sim.bolts, &mut watcher.sim.enemies, DT, None);
    }
    assert!(watcher.sim.bolts.is_empty(), "it stopped where it hit");
    assert_eq!(health, watcher.sim.enemies.iter().map(|enemy| enemy.health).collect::<Vec<f32>>(), "and hurt nothing there");
}
