//! a game played over a real connection on this machine (127.0.0.1): the host steps the world and sends it, the player who joined looks at their copy of it and
//! asks things of it, and what they ask for happens

use super::*;
use std::net::Ipv4Addr;

const RULES: Rules = Rules { army_limit: DEFAULT_ARMY_LIMIT, difficulty: Difficulty::Normal };
const PASSWORD: &str = "correct horse";
const PATIENCE: Duration = Duration::from_secs(12);
const DT: f32 = 1.0 / 60.0;

fn idle() -> PlayerInput {
    PlayerInput::idle(Formation::Free, Vec2::new(1920.0, 1080.0))
}

//a host that has started the run, and a player who has joined it and heard so
fn started() -> (NetHost, NetClient, Lobby, u8) {
    let mut config = HostConfig::new("Ada", PASSWORD, 0);
    config.address = Ipv4Addr::LOCALHOST.into();
    let host = NetHost::start(config).expect("the host starts");
    let mut client = NetClient::connect(&format!("127.0.0.1:{}", host.port), "Bo", PASSWORD);
    let mut lobby = Lobby::default();
    let until = Instant::now() + PATIENCE;
    while !lobby.joined {
        for event in client.poll() {
            assert!(lobby.apply(event).is_none(), "the player was turned away");
        }
        assert!(Instant::now() < until, "the player never joined");
        std::thread::sleep(Duration::from_millis(5));
    }
    let slot = lobby.slot;
    host.start_run();
    while !lobby.started {
        for event in client.poll() {
            assert!(lobby.apply(event).is_none());
        }
        assert!(Instant::now() < until, "the player never heard the run had started");
        std::thread::sleep(Duration::from_millis(5));
    }
    (host, client, lobby, slot)
}

//the host's world with the player seated in it, and a player's copy of it
struct Game {
    host: NetHost,
    sim: Sim,
    table: Table,
    seat: Guest,
    copy: Sim,
}

impl Game {
    fn new() -> Game {
        let (host, client, lobby, slot) = started();
        let mut sim = Sim::from_snapshot(new_run(), RULES);
        sim.players.list[0].name = "Ada".to_owned();
        let mut table = Table::new();
        table.seat(&mut sim, slot, "Bo").expect("room for the player");
        Game { host, sim, table, seat: Guest::new(client, lobby), copy: Sim::mirror() }
    }

    //one frame of each: the host takes a step with what it has been asked, the player asks for something and looks at what they have been sent
    fn frame(&mut self, asks: &PlayerInput) -> Result<(), String> {
        self.table.hear(self.host.take_inputs());
        let inputs = self.table.inputs(&self.sim, idle(), DT);
        let step = self.sim.step(inputs, Pace::steady(DT), RULES);
        self.table.tell(&self.host, &mut self.sim, &step.events, &step.report, SNAPSHOT_SECONDS);
        self.seat.hear(&mut self.copy)?;
        self.seat.frame(&mut self.copy, asks, 1.0, DT);
        std::thread::sleep(Duration::from_millis(8));
        Ok(())
    }

    fn until(&mut self, asks: &PlayerInput, mut done: impl FnMut(&Game) -> bool) {
        let until = Instant::now() + PATIENCE;
        while !done(self) {
            self.frame(asks).expect("the game goes on");
            assert!(Instant::now() < until, "it never came about");
        }
    }
}

#[test]
fn a_player_who_has_joined_is_shown_the_world_the_host_runs() {
    let mut game = Game::new();
    game.until(&idle(), |game| game.seat.ready());
    assert_eq!(game.seat.me(), 1);
    assert_eq!(game.copy.players.len(), 2);
    assert_eq!(game.copy.players.list[0].name, "Ada");
    assert_eq!(game.copy.players.list[1].name, "Bo");
    let mine = game.copy.units.iter().filter(|unit| unit.owner == 1).count();
    assert_eq!(mine, game.sim.units.iter().filter(|unit| unit.owner == 1).count());
    assert!(!game.copy.sites.is_empty(), "the sites are told");
}

#[test]
fn what_a_player_asks_for_is_done_by_the_host_and_seen_by_the_player() {
    let mut game = Game::new();
    game.until(&idle(), |game| game.seat.ready());
    let start = game.sim.players.list[1].position;
    let walking = PlayerInput { direction: Vec2::X, ..idle() };
    game.until(&walking, |game| game.sim.players.list[1].position.x > start.x + 150.0);
    //(they have not walked further in their copy than the host let them: the two agree to within a little once the host has caught up)
    for _ in 0..40 {
        game.frame(&PlayerInput { direction: Vec2::ZERO, ..idle() }).unwrap();
    }
    let host_at = game.sim.players.list[1].position;
    let seen = game.seat.replica.predicted;
    assert!(host_at.distance(seen) < 40.0, "the host has them at {host_at:?} and they think {seen:?}");
    assert!(host_at.x > start.x + 150.0);
    assert!((host_at.y - start.y).abs() < 100.0);
}

#[test]
fn a_relic_chosen_by_a_player_is_the_one_they_have() {
    let mut game = Game::new();
    game.until(&idle(), |game| game.seat.ready());
    game.sim.players.list[1].offer(RelicChoice::new());
    game.until(&idle(), |game| game.copy.players.list[1].choice.is_some());
    //(there is a moment after it is offered in which it can't be taken)
    game.until(&idle(), |game| game.sim.players.list[1].choice.as_ref().is_some_and(|choice| choice.age > RELIC_LOCKOUT + 0.1));
    let pick = PlayerInput { relic: Some(RelicAct::Pick(0)), ..idle() };
    game.frame(&pick).unwrap();
    game.until(&idle(), |game| game.sim.players.list[1].choice.is_none());
    game.until(&idle(), |game| game.copy.players.list[1].choice.is_none());
}

#[test]
fn a_player_who_goes_keeps_their_army_in_the_world_and_has_it_back_by_joining_again() {
    let mut game = Game::new();
    game.until(&idle(), |game| game.seat.ready());
    let before = game.sim.units.iter().filter(|unit| unit.owner == 1).count();
    game.seat.leave();
    let until = Instant::now() + PATIENCE;
    let mut left = false;
    while !left {
        for event in game.host.poll() {
            if let HostEvent::Left { slot, .. } = event {
                game.table.unseat(&mut game.sim, slot);
                left = true;
            }
        }
        assert!(Instant::now() < until, "the host never heard they had gone");
        std::thread::sleep(Duration::from_millis(5));
    }
    for _ in 0..30 {
        game.frame(&idle()).ok();
    }
    assert_eq!(game.sim.players.away_names(), ["Bo"]);
    assert_eq!(game.sim.units.iter().filter(|unit| unit.owner == 1).count(), 0, "their army is held, out of the world");
    assert_eq!(game.sim.players.list[1].held.as_ref().map(|held| held.len()), Some(before), "all of it");
    //joining again with the name takes the place again, and the army is where it was
    let player = game.table.seat(&mut game.sim, 1, "bo").expect("the same player");
    assert_eq!(player, 1);
    assert_eq!(game.sim.players.len(), 2, "no one new");
    assert!(game.sim.players.away_names().is_empty());
    assert_eq!(game.sim.units.iter().filter(|unit| unit.owner == 1).count(), before);
}

#[test]
fn the_world_is_not_sent_to_a_player_who_has_not_taken_in_what_they_were_sent() {
    let mut game = Game::new();
    game.until(&idle(), |game| game.seat.ready());
    //(the player stops looking: nothing is read from the connection and no frames are sent, as if their game had hung)
    let before = game.table_sent();
    for _ in 0..200 {
        game.table.hear(game.host.take_inputs());
        let inputs = game.table.inputs(&game.sim, idle(), DT);
        let step = game.sim.step(inputs, Pace::steady(DT), RULES);
        game.table.tell(&game.host, &mut game.sim, &step.events, &step.report, SNAPSHOT_SECONDS);
    }
    let sent = game.table_sent() - before;
    assert!(sent <= MAX_UNTAKEN + 4, "{sent} messages sent to a player who took in none");
}

impl Game {
    fn table_sent(&self) -> u32 {
        self.table.sent_to(1)
    }
}

// ---------- what the players ask ----------

fn frame_of(seq: u32, input: PlayerInput) -> Vec<u8> {
    ClientFrame { seq, tick_ack: 0, zoom: 1.0, input }.to_bytes()
}

#[test]
fn what_a_player_sends_is_written_and_read_back_and_put_right() {
    let input = PlayerInput { direction: Vec2::new(0.6, -0.8), place: vec![(3, 4), (5, 6)], recall: true, send: Some(77), ..idle() };
    let frame = ClientFrame { seq: 9, tick_ack: 4, zoom: 1.5, input };
    assert_eq!(ClientFrame::from_bytes(&frame.to_bytes()), Some(frame.clone()));
    let bytes = frame.to_bytes();
    for cut in 0..bytes.len() {
        assert_eq!(ClientFrame::from_bytes(&bytes[..cut]), None, "cut at {cut}");
    }
    let mut longer = bytes.clone();
    longer.push(0);
    assert_eq!(ClientFrame::from_bytes(&longer), None);
    let mut wrong = bytes.clone();
    wrong[0] = 0;
    assert_eq!(ClientFrame::from_bytes(&wrong), None);
    //(a camera that is further out than any, and a walk that is faster than walking, are put right)
    let eager = ClientFrame { seq: 1, tick_ack: 0, zoom: 90.0, input: PlayerInput { direction: Vec2::new(5.0, 0.0), ..idle() } };
    let read = ClientFrame::from_bytes(&eager.to_bytes()).expect("read");
    assert!(read.zoom <= MAX_ZOOM && read.input.direction.length() <= 1.0 + 1e-5);
}

#[test]
fn two_frames_before_a_step_are_one_asking_for_what_either_asked_for() {
    let first = PlayerInput { direction: Vec2::X, place: vec![(1, 1)], select: Some(2), relic: Some(RelicAct::Pick(1)), ..idle() };
    let second = PlayerInput { direction: Vec2::Y, place: vec![(2, 2)], delete: vec![(9, 9)], recall: true, send: Some(5), relic: Some(RelicAct::Reroll), ..idle() };
    let both = first.then(second);
    assert_eq!(both.direction, Vec2::Y, "the later way");
    assert_eq!(both.place, [(1, 1), (2, 2)]);
    assert_eq!(both.delete, [(9, 9)]);
    assert_eq!((both.select, both.send, both.recall), (Some(2), Some(5), true));
    assert_eq!(both.relic, Some(RelicAct::Pick(1)), "the first thing they did about the relics is what they meant");
    let mut many = idle();
    for i in 0..10 {
        many = many.then(PlayerInput { place: (0..100).map(|j| (i, j)).collect(), ..idle() });
    }
    assert_eq!(many.place.len(), MAX_SQUARES_AN_INPUT);
}

#[test]
fn the_table_gives_each_step_what_each_player_asked_and_keeps_walking_them_for_a_moment_between_frames() {
    let mut sim = Sim::from_snapshot(new_run(), RULES);
    let mut table = Table::new();
    assert_eq!(table.seat(&mut sim, 1, "Bo"), Ok(1));
    assert_eq!(table.seat(&mut sim, 2, "Cy"), Ok(2));
    let local = PlayerInput { direction: Vec2::Y, ..idle() };
    //nothing heard yet: they stand still
    let inputs = table.inputs(&sim, local.clone(), DT);
    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[0], local);
    assert_eq!(inputs[1].direction, Vec2::ZERO);
    //a frame is heard, and used once; the walk goes on for a moment after it, and then stops
    table.hear(vec![(1, frame_of(1, PlayerInput { direction: Vec2::X, ..idle() }))]);
    assert_eq!(table.inputs(&sim, local.clone(), DT)[1].direction, Vec2::X);
    assert_eq!(table.inputs(&sim, local.clone(), DT)[1].direction, Vec2::X);
    for _ in 0..30 {
        table.inputs(&sim, local.clone(), DT);
    }
    assert_eq!(table.inputs(&sim, local.clone(), DT)[1].direction, Vec2::ZERO);
    //a one-time ask is not repeated while the walk is held
    table.hear(vec![(2, frame_of(1, PlayerInput { direction: Vec2::X, recall: true, send: Some(4), ..idle() }))]);
    assert!(table.inputs(&sim, local.clone(), DT)[2].recall);
    let held = &table.inputs(&sim, local, DT)[2];
    assert!(!held.recall && held.send.is_none() && held.direction == Vec2::X);
}

#[test]
fn a_player_at_another_machine_is_never_in_free_whatever_they_ask_for() {
    let mut sim = Sim::from_snapshot(new_run(), RULES);
    let mut table = Table::new();
    table.seat(&mut sim, 1, "Bo").unwrap();
    //(before they are heard from they are standing as the world has them, which starts as a ring)
    assert_eq!(sim.players.list[1].formation, Formation::Ring);
    assert_eq!(table.inputs(&sim, idle(), DT)[1].formation, Formation::Ring);
    //(and one the world has as Free, as a save from before there was none may, is put right as well)
    sim.players.list[1].formation = Formation::Free;
    assert_eq!(table.inputs(&sim, idle(), DT)[1].formation, Formation::Ring);
    for (seq, asked, given) in [(1, Formation::Free, Formation::Ring), (2, Formation::Wedge, Formation::Wedge), (3, Formation::Line, Formation::Line), (4, Formation::Free, Formation::Ring)] {
        table.hear(vec![(1, frame_of(seq, PlayerInput { formation: asked, ..idle() }))]);
        assert_eq!(table.inputs(&sim, idle(), DT)[1].formation, given, "asked for {asked:?}");
    }
    //(the host's own is theirs to choose, and is put right where the host's game takes it)
    let local = PlayerInput { formation: Formation::Free, ..idle() };
    assert_eq!(table.inputs(&sim, local, DT)[0].formation, Formation::Free);
}

#[test]
fn a_frame_that_is_old_or_is_not_one_or_is_from_nobody_does_nothing() {
    let mut sim = Sim::from_snapshot(new_run(), RULES);
    let mut table = Table::new();
    table.seat(&mut sim, 1, "Bo").unwrap();
    table.hear(vec![(1, frame_of(5, PlayerInput { direction: Vec2::X, ..idle() }))]);
    table.hear(vec![(1, frame_of(5, PlayerInput { direction: Vec2::Y, ..idle() })), (1, frame_of(3, PlayerInput { direction: Vec2::Y, ..idle() })), (1, vec![1, 2, 3]), (1, Vec::new()), (3, frame_of(9, idle()))]);
    assert_eq!(table.inputs(&sim, idle(), DT)[1].direction, Vec2::X, "the first of frame 5 stands");
}

#[test]
fn the_world_holds_as_many_players_as_there_are_places_and_a_name_is_for_one() {
    let mut sim = Sim::from_snapshot(new_run(), RULES);
    let mut table = Table::new();
    assert_eq!(table.seat(&mut sim, 1, "Bo"), Ok(1));
    assert_eq!(table.seat(&mut sim, 2, "Cy"), Ok(2));
    assert_eq!(table.seat(&mut sim, 3, "Di"), Ok(3));
    assert!(table.seat(&mut sim, 4, "Ed").is_err(), "a fifth");
    assert!(table.seat(&mut sim, 5, "bo").is_err(), "the name of someone who is here");
    assert_eq!(sim.players.len(), MAX_PLAYERS);
    table.unseat(&mut sim, 1);
    assert_eq!(sim.players.away_names(), ["Bo"]);
    assert_eq!(table.seat(&mut sim, 1, "BO"), Ok(1), "the same name takes the place again");
    assert!(sim.players.away_names().is_empty());
}

#[test]
fn a_player_is_told_when_the_host_ends_the_game() {
    let mut game = Game::new();
    game.until(&idle(), |game| game.seat.ready());
    let Game { host, mut seat, mut copy, .. } = game;
    drop(host);
    let until = Instant::now() + PATIENCE;
    let why = loop {
        if let Err(why) = seat.hear(&mut copy) {
            break why;
        }
        assert!(Instant::now() < until, "they were never told");
        std::thread::sleep(Duration::from_millis(8));
    };
    assert!(why.why.contains("host") && !why.retry, "the host said it was over, which is not a reason to try again: {why:?}");
}
