//a game with others is open to the network, so whatever is sent to it can be anything: a message that is damaged or is noise, and an input that is well made and
//asks for what no player could (a square on the far side of the world, a squad at an enemy that is not there). none of it may take the world down, or leave something
//in it that cannot be saved, and a saved game that has been damaged is refused or loaded, but never a panic

use super::*;

const RULES: Rules = Rules { army_limit: DEFAULT_ARMY_LIMIT, difficulty: Difficulty::Normal };
const DT: f32 = 1.0 / 60.0;
const SCREEN: Vec2 = Vec2::new(1920.0, 1080.0);

//noise that is the same every time
struct Noise(u64);

impl Noise {
    fn new(seed: u64) -> Noise {
        Noise(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, count: u64) -> u64 {
        self.next() % count
    }

    //a number from -1 to 1
    fn unit(&mut self) -> f32 {
        (self.below(20_001) as f32 - 10_000.0) / 10_000.0
    }

    fn bytes(&mut self, count: usize) -> Vec<u8> {
        (0..count).map(|_| self.next() as u8).collect()
    }
}

//a square that is somewhere in the world, or just off it, or as far off it as a number goes
fn any_tile(noise: &mut Noise) -> (i32, i32) {
    let middle = WORLD_TILES / 2;
    match noise.below(5) {
        0 => (noise.next() as i32, noise.next() as i32),
        1 => (-1, WORLD_TILES),
        2 => (i32::MIN, i32::MAX),
        3 => (0, 0),
        _ => (middle + noise.below(60) as i32 - 30, middle + noise.below(60) as i32 - 30),
    }
}

//an input of the kind nobody plays: every part of it as odd as it can be and still be read
fn hostile(noise: &mut Noise) -> PlayerInput {
    let squares = |noise: &mut Noise| -> Vec<(i32, i32)> { (0..noise.below(MAX_SQUARES_AN_INPUT as u64 + 1)).map(|_| any_tile(noise)).collect() };
    let input = PlayerInput {
        direction: Vec2::new(noise.unit(), noise.unit()) * (noise.below(1000) as f32),
        screen: Vec2::new(noise.unit() * 20_000.0, noise.unit() * 20_000.0),
        formation: FORMATIONS[noise.below(FORMATIONS.len() as u64) as usize],
        select: (noise.below(2) == 0).then(|| noise.next() as u8),
        place: squares(noise),
        delete: squares(noise),
        tower: (noise.below(2) == 0).then(|| any_tile(noise)),
        recall: noise.below(2) == 0,
        send: (noise.below(2) == 0).then(|| noise.next() as u32 % 400),
        relic: match noise.below(4) {
            0 => None,
            1 => Some(RelicAct::Pick(noise.next() as u8)),
            2 => Some(RelicAct::Reroll),
            _ => Some(RelicAct::Pick(noise.below(3) as u8)),
        },
        evolve: (noise.below(2) == 0).then(|| EvolvePick { champion: noise.next() as u32, tier: noise.next() as u8, choice: noise.next() as u8 }),
        ultimate: false,
    };
    //(as the host gets it: in a message, read, and put right as far as it can be)
    let frame = ClientFrame { seq: 1, tick_ack: noise.next() as u32, zoom: noise.unit() * 50.0, input };
    ClientFrame::from_bytes(&frame.to_bytes()).expect("a frame of well-made parts is read").input
}

fn assert_world_is_sound(sim: &Sim, what: &str) {
    for unit in &sim.units {
        assert!(unit.x.is_finite() && unit.y.is_finite(), "{what}: a unit at {}, {}", unit.x, unit.y);
        assert!((unit.owner as usize) < sim.players.len(), "{what}: a unit of someone who is not in the game");
    }
    for player in &sim.players.list {
        assert!(player.position.is_finite(), "{what}: {} is nowhere", player.name);
        assert!(player.position.cmpge(Vec2::ZERO).all() && player.position.cmple(Vec2::splat(WORLD_SIZE)).all(), "{what}: {} is off the world at {}", player.name, player.position);
    }
    for enemy in &sim.enemies {
        assert!(enemy.position.is_finite(), "{what}: an enemy at {}", enemy.position);
    }
}

// ---------- inputs that are well made and ask for too much ----------

#[test]
fn players_who_ask_for_what_no_one_could_do_not_take_the_world_down_or_make_it_unsaveable() {
    let mut sim = Sim::from_snapshot(new_run(), RULES);
    sim.name_host("Ada");
    let mut table = Table::new();
    assert_eq!(table.seat(&mut sim, 1, "Bo"), Ok(1));
    assert_eq!(table.seat(&mut sim, 2, "Cy"), Ok(2));
    let mut noise = Noise::new(0x5eed_1234_abcd_ef01);
    for round in 0..400 {
        let mut inputs = vec![PlayerInput { direction: Vec2::new(0.3, -0.2), ..PlayerInput::idle(Formation::Ring, SCREEN) }];
        inputs.push(hostile(&mut noise));
        inputs.push(hostile(&mut noise));
        sim.step(inputs, Pace::steady(DT), RULES);
        if round % 50 == 0 {
            assert_world_is_sound(&sim, &format!("round {round}"));
        }
    }
    assert_world_is_sound(&sim, "the end");
    assert_eq!(sim.players.len(), 3, "no one came or went");
    //and what is left is a world that can be kept, and kept in a form that is read back
    let kept = sim.capture(0, 1.0, false).to_text();
    let back = Snapshot::from_text(&kept).expect("the world after it all is a world that was kept");
    let mut again = Sim::from_snapshot(back, RULES);
    again.step(vec![PlayerInput::idle(Formation::Free, SCREEN); 3], Pace::steady(DT), RULES);
}

#[test]
fn what_a_hostile_input_asks_for_is_put_right_by_the_time_the_world_has_it() {
    let mut noise = Noise::new(77);
    for _ in 0..300 {
        let input = hostile(&mut noise);
        assert!(input.direction.length() <= 1.0 + 1e-4, "walking faster than walking: {}", input.direction);
        assert!(input.screen.cmpge(Vec2::splat(MIN_SCREEN)).all() && input.screen.cmple(Vec2::splat(MAX_SCREEN)).all(), "a window of {}", input.screen);
        assert!(input.select.is_none_or(|kind| (kind as usize) < BUILDABLE.len()));
        assert!(input.place.len() <= MAX_SQUARES_AN_INPUT && input.delete.len() <= MAX_SQUARES_AN_INPUT);
        assert!(!matches!(input.relic, Some(RelicAct::Pick(place)) if place >= 3));
        assert!(input.evolve.is_none_or(|pick| (pick.tier as usize) < EVOLUTION_LEVELS.len() && pick.choice < 2));
    }
}

#[test]
fn a_player_with_nothing_to_choose_from_who_picks_a_relic_gets_nothing_and_the_world_goes_on() {
    let mut sim = Sim::from_snapshot(new_run(), RULES);
    let mut table = Table::new();
    assert_eq!(table.seat(&mut sim, 1, "Bo"), Ok(1));
    assert!(sim.players.list[1].choice.is_none());
    let before = sim.players.list[1].upgrades.taken.clone();
    for place in 0..3 {
        let asks = PlayerInput { relic: Some(RelicAct::Pick(place)), ..PlayerInput::idle(Formation::Free, SCREEN) };
        sim.step(vec![PlayerInput::idle(Formation::Free, SCREEN), asks], Pace::steady(DT), RULES);
    }
    let asks = PlayerInput { relic: Some(RelicAct::Reroll), ..PlayerInput::idle(Formation::Free, SCREEN) };
    sim.step(vec![PlayerInput::idle(Formation::Free, SCREEN), asks], Pace::steady(DT), RULES);
    assert!(sim.players.list[1].choice.is_none(), "a reroll of nothing is nothing");
    assert_eq!(sim.players.list[1].upgrades.taken, before, "and nothing is taken");
}

// ---------- messages that are not messages ----------

//a frame as a player's game would send it, with as much in it as one holds
fn a_full_frame() -> Vec<u8> {
    ClientFrame {
        seq: 41,
        tick_ack: 40,
        zoom: 0.8,
        input: PlayerInput {
            direction: Vec2::new(0.6, -0.8),
            screen: SCREEN,
            formation: Formation::Wedge,
            select: Some(2),
            place: vec![(10, 20), (11, 20), (12, 20)],
            delete: vec![(-3, 4)],
            tower: Some((500, 500)),
            recall: true,
            send: Some(9),
            relic: Some(RelicAct::Pick(1)),
            evolve: Some(EvolvePick { champion: 77, tier: 0, choice: 0 }),
            ultimate: false,
        },
    }
    .to_bytes()
}

//whatever is read of a frame is a frame that has been put right
fn assert_tidy(frame: &ClientFrame) {
    assert!(frame.input.direction.length() <= 1.0 + 1e-4);
    assert!(frame.zoom >= MIN_ZOOM && frame.zoom <= MAX_ZOOM);
    assert!(frame.input.screen.cmpge(Vec2::splat(MIN_SCREEN)).all() && frame.input.screen.cmple(Vec2::splat(MAX_SCREEN)).all());
    assert!(frame.input.place.len() <= MAX_SQUARES_AN_INPUT && frame.input.delete.len() <= MAX_SQUARES_AN_INPUT);
}

#[test]
fn a_frame_with_any_one_bit_wrong_or_cut_short_anywhere_is_read_as_a_frame_that_is_in_order_or_not_at_all() {
    let good = a_full_frame();
    let frame = ClientFrame::from_bytes(&good).expect("the frame as sent");
    assert_tidy(&frame);
    for at in 0..good.len() {
        for bit in 0..8 {
            let mut bad = good.clone();
            bad[at] ^= 1 << bit;
            if let Some(read) = ClientFrame::from_bytes(&bad) {
                assert_tidy(&read);
            }
        }
        assert!(ClientFrame::from_bytes(&good[..at]).is_none(), "cut at {at}");
    }
    let mut longer = good.clone();
    longer.push(0);
    assert!(ClientFrame::from_bytes(&longer).is_none(), "something left over");
}

#[test]
fn noise_in_place_of_a_frame_or_an_event_or_an_enemy_is_never_a_panic() {
    let mut noise = Noise::new(0xfeed_beef);
    let events = write_events(&[
        Event::Shake(0.4),
        Event::Announce { text: "BOSS".to_owned(), color: WHITE },
        Event::Popup { at: Vec2::new(10.0, 10.0), lane: 0.5, text: "+3".to_owned(), color: WHITE },
        Event::Burst { at: Vec2::ONE, color: WHITE, count: 12, speed: 80.0 },
        Event::Destroyed(3),
    ]);
    assert!(read_events(&events).is_some());
    for length in 0..700 {
        let mut bytes = noise.bytes(length);
        let _ = ClientFrame::from_bytes(&bytes);
        let _ = PlayerInput::from_bytes(&bytes);
        let _ = read_events(&bytes);
        let _ = read_enemy_blob(&bytes);
        if let Some(first) = bytes.first_mut() {
            *first = UP_INPUT;
        }
        let _ = ClientFrame::from_bytes(&bytes);
    }
    for at in 0..events.len() {
        for bit in 0..8 {
            let mut bad = events.clone();
            bad[at] ^= 1 << bit;
            let _ = read_events(&bad);
        }
        assert!(read_events(&events[..at]).is_none(), "cut at {at}");
    }
    //an enemy as the host would send it, damaged anywhere
    let sim = Sim::from_snapshot(new_run(), RULES);
    let blob = enemy_blob(&sim.enemies[0]);
    assert!(read_enemy_blob(&blob).is_some());
    for at in 0..blob.len() {
        let mut bad = blob.clone();
        bad[at] = noise.next() as u8;
        let _ = read_enemy_blob(&bad);
        let _ = read_enemy_blob(&blob[..at]);
    }
}

#[test]
fn text_of_a_number_that_runs_on_and_on_is_not_one() {
    //(a varint of more bytes than a number has, the length of a stretch of bytes that is more than is there, and a flag that is neither)
    assert_eq!(Reader::new(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]).varint(), None);
    assert_eq!(Reader::new(&[0xff, 0xff, 0xff, 0xff, 0x7f]).varint(), None, "a fifth byte that does not fit");
    assert_eq!(Reader::new(&[0xff, 0xff, 0xff, 0xff, 0x0f]).varint(), Some(u32::MAX));
    assert_eq!(Reader::new(&[0xff, 0xff, 0xff, 0xff, 0x00, 0x00]).blob(10), None);
    assert_eq!(Reader::new(&[200, 0, 0, 0, 1, 2]).blob(1_000), None);
    assert_eq!(Reader::new(&[0xff, 0xff, 0xff, 0xff]).blob(usize::MAX), None, "four billion bytes of nothing");
    assert_eq!(Reader::new(&[2]).flag(), None);
    assert_eq!(Reader::new(&0x7fc0_0000u32.to_le_bytes()).f32(), None, "not a number");
    assert_eq!(Reader::new(&f32::INFINITY.to_le_bytes()).f32(), None);
}

// ---------- a saved game that is damaged ----------

//a game with others in it, played a little and kept
fn kept() -> String {
    let mut sim = Sim::from_snapshot(new_run(), RULES);
    sim.name_host("Ada");
    let mut table = Table::new();
    assert_eq!(table.seat(&mut sim, 1, "Bo"), Ok(1));
    for _ in 0..40 {
        sim.step(vec![PlayerInput::idle(Formation::Free, SCREEN); 2], Pace::steady(DT), RULES);
    }
    table.unseat(&mut sim, 1);
    sim.capture(0, 1.0, false).to_text()
}

#[test]
fn a_saved_game_with_any_part_of_it_changed_is_refused_or_loaded_and_never_a_panic() {
    let good = kept();
    let mut noise = Noise::new(0x0bad_cafe);
    let mut loaded = 0;
    for round in 0..250 {
        let mut bad = good.clone().into_bytes();
        for _ in 0..1 + round % 3 {
            let at = noise.below(bad.len() as u64) as usize;
            //(a digit, a letter, a space, a new line, or a minus)
            bad[at] = *b"0123456789abcdefz -\n.".get(noise.below(21) as usize).unwrap();
        }
        if round % 7 == 0 {
            bad.truncate(noise.below(bad.len() as u64) as usize);
        }
        let Ok(text) = String::from_utf8(bad) else { continue };
        if let Ok(snapshot) = Snapshot::from_text(&text) {
            //(a change that is still a saved game: the world it makes has to be one that can take a step)
            loaded += 1;
            let mut sim = Sim::from_snapshot(snapshot, RULES);
            let count = sim.players.len();
            sim.step(vec![PlayerInput::idle(Formation::Free, SCREEN); count], Pace::steady(DT), RULES);
        }
    }
    assert!(loaded < 250, "some of the changes were noticed");
}

#[test]
fn a_saved_game_of_noise_or_of_nothing_is_refused() {
    let mut noise = Noise::new(5);
    assert!(Snapshot::from_text("").is_err());
    assert!(Snapshot::from_text("\n\n\n").is_err());
    for length in [1, 10, 100, 1000, 5000] {
        let text: String = noise.bytes(length).into_iter().map(|byte| (b' ' + byte % 90) as char).collect();
        assert!(Snapshot::from_text(&text).is_err(), "noise of {length}");
    }
    //numbers too large to be believed, where a count is wanted
    let good = kept();
    let first_line_end = good.find('\n').unwrap();
    let huge = format!("{}\nunits 4000000000\n", &good[..first_line_end]);
    assert!(Snapshot::from_text(&huge).is_err());
}

#[test]
fn an_enemy_in_a_saved_game_that_is_heading_for_a_stop_that_is_not_on_its_route_is_put_right() {
    let mut sim = Sim::from_snapshot(new_run(), RULES);
    assert!(!sim.enemies.is_empty(), "a new run has enemies");
    let (long, empty) = (0, sim.enemies.len() - 1);
    sim.enemies[long].heading_for = 99;
    sim.enemies[empty].route.clear();
    let snapshot = Snapshot::from_text(&sim.capture(0, 1.0, false).to_text()).expect("a save of enemies a little off is still a save");
    for enemy in &snapshot.enemies {
        assert!(!enemy.route.is_empty() && enemy.heading_for < enemy.route.len(), "enemy {}: stop {} of {}", enemy.id, enemy.heading_for, enemy.route.len());
    }
    let mut again = Sim::from_snapshot(snapshot, RULES);
    again.step(vec![PlayerInput::idle(Formation::Free, SCREEN); 1], Pace::steady(DT), RULES);
}
