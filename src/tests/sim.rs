//the world apart from whoever is playing it: the bytes inputs and events are written in, the events a step reports, and a step taken with scripted
//input and no window

use super::*;

const RULES: Rules = Rules { army_limit: DEFAULT_ARMY_LIMIT, difficulty: Difficulty::Normal };
const FRAME: f32 = 1.0 / 60.0;
const SCREEN: Vec2 = Vec2::new(1920.0, 1080.0);

fn red() -> Color {
    Color::from_rgba(255, 51, 25, 255)
}

// ---------- the bytes ----------

#[test]
fn numbers_text_and_colours_are_written_and_read_back() {
    let mut out = Writer::new();
    out.u8(7);
    out.flag(true);
    out.flag(false);
    out.u16(65_000);
    out.u32(4_000_000_000);
    out.i32(-123_456);
    out.f32(-1.5);
    out.vec2(Vec2::new(12.25, -80.0));
    out.text("Ada's army");
    out.color(red());
    let mut input = Reader::new(&out.bytes);
    assert_eq!(input.u8(), Some(7));
    assert_eq!(input.flag(), Some(true));
    assert_eq!(input.flag(), Some(false));
    assert_eq!(input.u16(), Some(65_000));
    assert_eq!(input.u32(), Some(4_000_000_000));
    assert_eq!(input.i32(), Some(-123_456));
    assert_eq!(input.f32(), Some(-1.5));
    assert_eq!(input.vec2(), Some(Vec2::new(12.25, -80.0)));
    assert_eq!(input.text().as_deref(), Some("Ada's army"));
    assert_eq!(input.color(), Some(red()));
    assert!(input.is_done());
}

#[test]
fn a_colour_is_kept_to_within_a_255th_and_cannot_go_out_of_range() {
    let mut out = Writer::new();
    out.color(Color::new(0.3333, 2.0, -1.0, 0.5));
    let color = Reader::new(&out.bytes).color().unwrap();
    assert!((color.r - 0.3333).abs() <= 1.0 / 255.0);
    assert_eq!((color.g, color.b), (1.0, 0.0));
    assert!((color.a - 0.5).abs() <= 1.0 / 255.0);
}

#[test]
fn a_message_that_is_cut_short_reads_as_nothing() {
    let mut out = Writer::new();
    out.u32(1);
    out.vec2(Vec2::ONE);
    out.text("hello");
    for cut in 0..out.bytes.len() {
        let mut input = Reader::new(&out.bytes[..cut]);
        let read = (|| Some((input.u32()?, input.vec2()?, input.text()?)))();
        assert!(read.is_none(), "{cut} bytes of {} were taken for the whole", out.bytes.len());
    }
    assert!(Reader::new(&[]).u8().is_none() && Reader::new(&[1]).u16().is_none());
}

#[test]
fn what_cannot_be_true_is_refused() {
    assert_eq!(Reader::new(&[2]).flag(), None, "a flag is 0 or 1");
    assert_eq!(Reader::new(&f32::NAN.to_le_bytes()).f32(), None);
    assert_eq!(Reader::new(&f32::INFINITY.to_le_bytes()).f32(), None);
    assert_eq!(Reader::new(&f32::NEG_INFINITY.to_le_bytes()).vec2(), None);
    assert_eq!(Reader::new(&[2, 0xff, 0xfe]).text(), None, "text is text");
    assert_eq!(Reader::new(&[5, b'a', b'b']).text(), None, "it says more than there is");
}

#[test]
fn text_is_cut_at_the_limit_where_a_letter_ends() {
    let long = "é".repeat(300);
    let mut out = Writer::new();
    out.text(&long);
    let text = Reader::new(&out.bytes).text().unwrap();
    assert!(text.len() <= MAX_TEXT_BYTES && text.len() >= MAX_TEXT_BYTES - 1, "{}", text.len());
    assert!(text.chars().all(|letter| letter == 'é'));
}

// ---------- events ----------

fn every_kind_of_event() -> Vec<Event> {
    let mut events = vec![
        Event::Popup { at: Vec2::new(100.5, -20.0), lane: 30.0, text: "42 sent".to_owned(), color: red() },
        Event::Burst { at: Vec2::new(5.0, 6.0), color: red(), count: 7, speed: 140.0 },
        Event::Ring { at: Vec2::new(1.0, 2.0), radius: 300.0, color: red() },
        Event::Beam { from: Vec2::new(1.0, 2.0), to: Vec2::new(300.0, 400.0), color: red() },
        Event::Mark { at: Vec2::new(8.0, 9.0), radius: 40.0, color: red(), kind: MarkKind::Scorch },
        Event::Mark { at: Vec2::new(8.0, 9.0), radius: 40.0, color: red(), kind: MarkKind::Splat },
        Event::Debris { at: Vec2::new(8.0, 9.0), color: red(), size: 24.0 },
        Event::Cue { cue: Cue::Pop, at: None },
        Event::Cue { cue: Cue::Slam, at: Some(Vec2::new(77.0, 78.0)) },
        Event::Announce { text: "THE ECLIPSE APPROACHES".to_owned(), color: red() },
        Event::Shake(0.6),
        Event::Punch(1.0),
        Event::PickedUp { player: 3, count: 12 },
        Event::Destroyed(250),
    ];
    events.extend(ALL_CUES.iter().map(|&cue| Event::Cue { cue, at: None }));
    events
}

#[test]
fn every_kind_of_event_and_every_sound_is_written_and_read_back() {
    let events = every_kind_of_event();
    assert_eq!(read_events(&write_events(&events)), Some(events));
    assert_eq!(read_events(&write_events(&[])), Some(Vec::new()));
}

#[test]
fn a_message_of_events_that_is_cut_short_or_has_something_after_it_is_refused() {
    let bytes = write_events(&every_kind_of_event());
    for cut in 0..bytes.len() {
        assert!(read_events(&bytes[..cut]).is_none(), "{cut} bytes of {} were taken for the whole", bytes.len());
    }
    let mut more = bytes.clone();
    more.push(0);
    assert!(read_events(&more).is_none());
}

#[test]
fn an_event_that_is_not_one_is_refused() {
    //(one event, of a kind that doesn't exist; a sound that doesn't exist; a mark of a kind there isn't)
    assert!(read_events(&[1, 0, 99]).is_none());
    assert!(read_events(&[1, 0, 7, 200, 0]).is_none());
    let mut mark = Writer::new();
    mark.u16(1);
    mark.u8(5);
    mark.vec2(Vec2::ZERO);
    mark.f32(1.0);
    mark.color(red());
    mark.u8(2);
    assert!(read_events(&mark.bytes).is_none());
    //a player who isn't one of the four
    let mut picked = Writer::new();
    picked.u16(1);
    picked.u8(11);
    picked.u8(MAX_PLAYERS as u8);
    picked.u32(1);
    assert!(read_events(&picked.bytes).is_none());
}

#[test]
fn a_message_that_says_it_has_more_events_than_a_message_holds_is_refused() {
    let mut out = Writer::new();
    out.u16(MAX_EVENTS_A_MESSAGE as u16 + 1);
    for _ in 0..MAX_EVENTS_A_MESSAGE + 1 {
        Event::Shake(0.1).write(&mut out);
    }
    assert!(read_events(&out.bytes).is_none());
    //and writing more than that sends the first of them
    let many = vec![Event::Shake(0.1); MAX_EVENTS_A_MESSAGE + 50];
    assert_eq!(read_events(&write_events(&many)).map(|read| read.len()), Some(MAX_EVENTS_A_MESSAGE));
}

#[test]
fn an_event_knows_where_it_happened_if_it_happened_anywhere() {
    let at = Vec2::new(10.0, 20.0);
    assert_eq!(Event::Popup { at, lane: 0.0, text: String::new(), color: WHITE }.place(), Some(at));
    assert_eq!(Event::Beam { from: at, to: Vec2::ZERO, color: WHITE }.place(), Some(at));
    assert_eq!(Event::Cue { cue: Cue::Slam, at: Some(at) }.place(), Some(at));
    assert_eq!(Event::Cue { cue: Cue::Pop, at: None }.place(), None);
    assert_eq!(Event::Announce { text: String::new(), color: WHITE }.place(), None);
    assert_eq!(Event::Shake(1.0).place(), None);
    assert_eq!(Event::Destroyed(3).place(), None);
}

// ---------- effects as events ----------

fn busy_effects() -> Effects {
    let mut fx = Effects::default();
    fx.popup(Vec2::new(100.0, 100.0), 20.0, 30.0, "+3".to_owned(), GREEN);
    fx.burst(Vec2::new(5.0, 5.0), RED, 6, 200.0);
    fx.ring(Vec2::new(9.0, 9.0), 120.0, BLUE);
    fx.beams.push(Beam { from: Vec2::ZERO, to: Vec2::new(50.0, 50.0), color: WHITE, age: 0.0 });
    fx.mark(Vec2::new(3.0, 3.0), 30.0, BLACK, MarkKind::Splat);
    fx.debris(Vec2::new(4.0, 4.0), GRAY, 16.0);
    fx.cue(Cue::Pop);
    fx.cue_at(Cue::Explosion, Vec2::new(60.0, 61.0));
    fx.announce("ARMY OF 100!".to_owned(), YELLOW);
    fx.add_shake(0.4);
    fx.add_punch(0.7);
    fx.grown[0] = 3;
    fx.grown[2] = 1;
    fx
}

#[test]
fn draining_the_effects_turns_what_is_for_the_eyes_and_ears_into_events_and_leaves_none_behind() {
    let mut fx = busy_effects();
    let events = fx.drain_events();
    assert!(events.contains(&Event::Popup { at: Vec2::new(100.0, 80.0), lane: 30.0, text: "+3".to_owned(), color: GREEN }));
    assert!(events.contains(&Event::Burst { at: Vec2::new(5.0, 5.0), color: RED, count: 6, speed: 200.0 }));
    assert!(events.contains(&Event::Ring { at: Vec2::new(9.0, 9.0), radius: 120.0, color: BLUE }));
    assert!(events.contains(&Event::Beam { from: Vec2::ZERO, to: Vec2::new(50.0, 50.0), color: WHITE }));
    assert!(events.contains(&Event::Mark { at: Vec2::new(3.0, 3.0), radius: 30.0, color: BLACK, kind: MarkKind::Splat }));
    assert!(events.contains(&Event::Debris { at: Vec2::new(4.0, 4.0), color: GRAY, size: 16.0 }));
    assert!(events.contains(&Event::Cue { cue: Cue::Pop, at: None }));
    assert!(events.contains(&Event::Cue { cue: Cue::Explosion, at: Some(Vec2::new(60.0, 61.0)) }));
    assert!(events.contains(&Event::Announce { text: "ARMY OF 100!".to_owned(), color: YELLOW }));
    assert!(events.iter().any(|event| matches!(event, Event::Shake(amount) if (amount - 0.4).abs() < 1e-6)));
    assert!(events.iter().any(|event| matches!(event, Event::Punch(amount) if (amount - 0.7).abs() < 1e-6)));
    assert!(events.contains(&Event::PickedUp { player: 0, count: 3 }) && events.contains(&Event::PickedUp { player: 2, count: 1 }));
    assert_eq!(events.len(), 13);
    assert!(fx.drain_events().is_empty(), "it was all taken");
    assert!(fx.popups.is_empty() && fx.bursts.is_empty() && fx.rings.is_empty() && fx.beams.is_empty() && fx.marks.is_empty());
    assert!(fx.cues.is_empty() && fx.announcements.is_empty() && fx.shake == 0.0 && fx.punch == 0.0 && fx.grown == [0; MAX_PLAYERS]);
}

#[test]
fn what_the_world_has_to_act_on_itself_is_not_taken_from_it() {
    let mut fx = Effects::default();
    fx.hazards.push(hazard_at(Vec2::new(5.0, 5.0), 100.0, 1.0, 10.0, 5));
    fx.blasts.push(Blast { position: Vec2::ZERO, radius: 50.0, damage: 10.0 });
    fx.wall_hits.push(((1, 1), 5.0));
    fx.fallen.push((Vec2::ZERO, 0));
    fx.earn(1, 25.0);
    assert!(fx.drain_events().is_empty());
    assert_eq!((fx.hazards.len(), fx.blasts.len(), fx.wall_hits.len(), fx.fallen.len()), (1, 1, 1, 1));
    assert_eq!(fx.spoils[1].points, 25.0);
}

#[test]
fn a_watcher_shown_the_events_has_what_the_effects_would_have_shown() {
    //(the same things asked for of the effects directly, and told to a watcher by events)
    let mut direct = busy_effects();
    let mut feedback = Feedback::new();
    feedback.absorb(busy_effects().drain_events(), 0, Vec2::ZERO);
    let seen = &mut feedback.fx;
    assert_eq!(seen.popups.len(), direct.popups.len());
    assert_eq!((seen.popups[0].position, seen.popups[0].lane, seen.popups[0].text.clone()), (direct.popups[0].position, direct.popups[0].lane, "+3".to_owned()));
    assert_eq!((seen.bursts.len(), seen.bursts[0].count), (direct.bursts.len(), 6));
    assert_eq!((seen.rings.len(), seen.beams.len(), seen.marks.len(), seen.shards.len()), (1, 1, 1, 1));
    assert_eq!(seen.take_cues(), direct.take_cues());
    assert_eq!(seen.announcements, direct.announcements);
    assert!((seen.shake - direct.shake).abs() < 1e-6 && (seen.punch - direct.punch).abs() < 1e-6);
    assert_eq!(seen.picked_up, 3, "the babies the player at this machine took");
}

#[test]
fn the_babies_another_player_took_are_not_counted_for_this_one() {
    let mut feedback = Feedback::new();
    feedback.absorb(vec![Event::PickedUp { player: 1, count: 9 }, Event::PickedUp { player: 0, count: 2 }], 0, Vec2::ZERO);
    assert_eq!(feedback.fx.picked_up, 2);
}

#[test]
fn enemies_destroyed_make_a_kill_streak_for_the_watcher() {
    let mut feedback = Feedback::new();
    let here = Vec2::new(500.0, 500.0);
    feedback.absorb(vec![Event::Destroyed(COMBO_STEPS[0].0 as u32)], 0, here);
    assert_eq!(feedback.combo.count, COMBO_STEPS[0].0);
    assert_eq!(feedback.fx.take_cues(), vec![(Cue::Combo, None)]);
    assert!(feedback.fx.popups.iter().any(|popup| popup.text.starts_with(COMBO_STEPS[0].1)));
    assert!(feedback.fx.popups[0].position.distance(here) < RADIUS * 5.0, "over the player");
    feedback.absorb(Vec::new(), 0, here);
    assert_eq!(feedback.combo.count, COMBO_STEPS[0].0, "nothing destroyed, nothing added");
}

#[test]
fn a_watcher_cannot_be_told_to_shake_the_camera_more_than_all_the_way() {
    let mut feedback = Feedback::new();
    feedback.absorb(vec![Event::Shake(1.0e6), Event::Punch(50.0)], 0, Vec2::ZERO);
    assert_eq!((feedback.fx.shake, feedback.fx.punch), (1.0, 1.0));
    feedback.absorb(vec![Event::Shake(-5.0), Event::Punch(-1.0)], 0, Vec2::ZERO);
    assert_eq!((feedback.fx.shake, feedback.fx.punch), (1.0, 1.0));
}

#[test]
fn the_watchers_things_age_and_the_babies_show_as_one_number() {
    let mut feedback = Feedback::new();
    let here = Vec2::new(300.0, 300.0);
    feedback.absorb(vec![Event::PickedUp { player: 0, count: 5 }, Event::Burst { at: here, color: RED, count: 4, speed: 100.0 }, Event::Shake(1.0)], 0, here);
    feedback.update(FRAME, here);
    assert_eq!(feedback.particles.len(), 4, "the puff is particles");
    assert!(feedback.fx.popups.iter().any(|popup| popup.text == "+5"));
    assert!(feedback.fx.shake < 1.0, "settling");
    for _ in 0..600 {
        feedback.update(FRAME, here);
    }
    assert!(feedback.fx.popups.is_empty() && feedback.fx.shake == 0.0);
}

// ---------- the input ----------

fn busy_input() -> PlayerInput {
    PlayerInput {
        direction: Vec2::new(0.5, -0.5),
        screen: SCREEN,
        formation: Formation::Wedge,
        select: Some(3),
        place: vec![(10, 11), (11, 11), (12, 11)],
        delete: vec![(-1, 4)],
        tower: Some((200, 300)),
        recall: true,
        send: Some(4_000_000_001),
        relic: Some(RelicAct::Pick(2)),
        evolve: Some(EvolvePick { champion: 4_000_000_002, tier: 0, choice: 1 }),
    }
}

#[test]
fn an_input_is_written_and_read_back_whatever_is_in_it() {
    let input = busy_input();
    assert_eq!(PlayerInput::from_bytes(&input.to_bytes()), Some(input));
    let idle = PlayerInput::idle(Formation::Ring, Vec2::new(1280.0, 720.0));
    assert_eq!(PlayerInput::from_bytes(&idle.to_bytes()), Some(idle));
    for relic in [None, Some(RelicAct::Reroll), Some(RelicAct::Pick(0))] {
        let input = PlayerInput { relic, ..busy_input() };
        assert_eq!(PlayerInput::from_bytes(&input.to_bytes()), Some(input));
    }
}

#[test]
fn an_input_that_is_cut_short_or_has_something_after_it_is_refused() {
    let bytes = busy_input().to_bytes();
    for cut in 0..bytes.len() {
        assert!(PlayerInput::from_bytes(&bytes[..cut]).is_none(), "{cut} bytes of {} were taken for the whole", bytes.len());
    }
    let mut more = bytes;
    more.push(1);
    assert!(PlayerInput::from_bytes(&more).is_none());
}

#[test]
fn an_input_that_asks_for_what_does_not_exist_is_refused() {
    let mut out = Writer::new();
    out.vec2(Vec2::ZERO);
    out.vec2(SCREEN);
    out.u8(FORMATIONS.len() as u8);
    let mut bad_shape = out.bytes.clone();
    bad_shape.extend(vec![0; 64]);
    assert!(PlayerInput::from_bytes(&bad_shape).is_none(), "there is no such formation");
    //too many squares to build on
    let mut out = Writer::new();
    out.vec2(Vec2::ZERO);
    out.vec2(SCREEN);
    out.u8(0);
    out.flag(false);
    out.u8(0);
    out.u16(MAX_SQUARES_AN_INPUT as u16 + 1);
    for _ in 0..MAX_SQUARES_AN_INPUT + 1 {
        out.i32(1);
        out.i32(1);
    }
    out.bytes.extend(vec![0; 64]);
    assert!(PlayerInput::from_bytes(&out.bytes).is_none());
    //a number that isn't one
    let mut nan = busy_input().to_bytes();
    nan[..4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(PlayerInput::from_bytes(&nan).is_none());
    //a flag that isn't 0 or 1 (the one for the kind of wall, after the formation)
    let mut flag = busy_input().to_bytes();
    flag[17] = 7;
    assert!(PlayerInput::from_bytes(&flag).is_none());
}

#[test]
fn an_input_is_put_right_as_far_as_it_can_be() {
    let wild = PlayerInput {
        direction: Vec2::new(300.0, -400.0),
        screen: Vec2::new(10.0, 100_000.0),
        select: Some(200),
        place: vec![(1, 1); 1000],
        delete: vec![(2, 2); 1000],
        relic: Some(RelicAct::Pick(9)),
        evolve: Some(EvolvePick { champion: 1, tier: 1, choice: 0 }),
        ..busy_input()
    }
    .tidy();
    assert!((wild.direction.length() - 1.0).abs() < 1e-5 && wild.direction.x > 0.0 && wild.direction.y < 0.0);
    assert_eq!(wild.screen, Vec2::new(MIN_SCREEN, MAX_SCREEN));
    assert_eq!(wild.select, None);
    assert_eq!((wild.place.len(), wild.delete.len()), (MAX_SQUARES_AN_INPUT, MAX_SQUARES_AN_INPUT));
    assert_eq!(wild.relic, None);
    assert_eq!(wild.evolve, None, "there is only the one choice");
    assert_eq!(PlayerInput { evolve: Some(EvolvePick { champion: 1, tier: 0, choice: 2 }), ..busy_input() }.tidy().evolve, None, "and two ways at each");
    //an input that is fine is left as it is
    assert_eq!(busy_input().tidy(), busy_input());
    //(walking slower than walking is fine)
    assert_eq!(PlayerInput { direction: Vec2::new(0.25, 0.0), ..busy_input() }.tidy().direction, Vec2::new(0.25, 0.0));
}

#[test]
fn a_square_is_in_reach_if_it_is_in_the_world_and_in_the_window() {
    let input = PlayerInput::idle(Formation::Free, SCREEN);
    let here = Vec2::splat(WORLD_SIZE / 2.0);
    let tile = tile_of(here.x, here.y);
    assert!(input.reaches(here, tile));
    assert!(input.reaches(here, (tile.0 + 20, tile.1 - 20)));
    assert!(!input.reaches(here, (-1, tile.1)) && !input.reaches(here, (tile.0, WORLD_TILES)) && !input.reaches(here, (WORLD_TILES, tile.1)));
    assert!(!PlayerInput::idle(Formation::Free, Vec2::splat(MAX_SCREEN)).reaches(Vec2::splat(RADIUS), (WORLD_TILES + 1, 0)));
    let far = (tile.0 + (SCREEN.x / MIN_ZOOM / 2.0 / TILE_SIZE) as i32 + 5, tile.1);
    assert!(!input.reaches(here, far), "across more than the window shows");
    assert!(PlayerInput::idle(Formation::Free, Vec2::splat(MAX_SCREEN)).reaches(here, far), "a bigger window shows more");
}

// ---------- a step ----------

//a world with nothing in it but one player in the middle of the open ground, and the army they start with
fn quiet() -> Sim {
    let mut sim = Sim::from_snapshot(new_run(), RULES);
    sim.enemies.clear();
    sim.sites.clear();
    sim.walls.clear();
    sim.babies.clear();
    sim
}

fn step(sim: &mut Sim, inputs: Vec<PlayerInput>) -> Step {
    sim.step(inputs, Pace::steady(FRAME), RULES)
}

fn idle() -> PlayerInput {
    PlayerInput::idle(Formation::Free, SCREEN)
}

fn walking(direction: Vec2) -> PlayerInput {
    PlayerInput { direction, ..idle() }
}

#[test]
fn a_player_walks_where_the_input_says_at_the_speed_their_army_gives_them() {
    let mut sim = quiet();
    let start = sim.players.list[0].position;
    let mut walked = 0.0;
    for _ in 0..30 {
        let step = step(&mut sim, vec![walking(Vec2::new(1.0, 0.0))]);
        walked += PLAYER_SPEED * step.report.boosts[0] * FRAME;
    }
    let now = sim.players.list[0].position;
    assert!((now.x - start.x - walked).abs() < 0.5, "{} of {walked}", now.x - start.x);
    assert!((now.y - start.y).abs() < 0.5);
}

#[test]
fn a_player_with_no_input_stands_still_and_so_does_one_who_walks_too_fast() {
    let mut sim = quiet();
    let start = sim.players.list[0].position;
    for _ in 0..10 {
        step(&mut sim, Vec::new());
    }
    assert_eq!(sim.players.list[0].position, start);
    //walking at 100 times the speed is walking, as the input is put right
    let mut fast = quiet();
    let mut slow = quiet();
    for _ in 0..10 {
        step(&mut fast, vec![walking(Vec2::new(100.0, 0.0))]);
        step(&mut slow, vec![walking(Vec2::new(1.0, 0.0))]);
    }
    assert!((fast.players.list[0].position.x - slow.players.list[0].position.x).abs() < 3.0);
}

#[test]
fn the_formation_an_input_names_is_the_one_the_army_keeps_and_a_missing_input_leaves_it_alone() {
    let mut sim = quiet();
    sim.players.join("Bo", Vec2::new(sim.players.list[0].position.x + 4000.0, sim.players.list[0].position.y), &sim.walls).unwrap();
    sim.units.extend(army_for(1, 5, sim.players.list[1].position));
    step(&mut sim, vec![PlayerInput::idle(Formation::Ring, SCREEN), PlayerInput::idle(Formation::Line, SCREEN)]);
    assert_eq!((sim.players.list[0].formation, sim.players.list[1].formation), (Formation::Ring, Formation::Line));
    step(&mut sim, vec![PlayerInput::idle(Formation::Wedge, SCREEN)]);
    assert_eq!((sim.players.list[0].formation, sim.players.list[1].formation), (Formation::Wedge, Formation::Line), "no input, no change");
}

#[test]
fn walls_are_built_on_the_squares_an_input_names_if_the_player_could_reach_them_and_deleted_for_a_refund() {
    let mut sim = quiet();
    let here = tile_of(sim.players.list[0].position.x, sim.players.list[0].position.y);
    let near = (here.0 + 6, here.1);
    let far = (here.0 + 4000, here.1);
    let points = sim.players.list[0].mason.points;
    step(&mut sim, vec![PlayerInput { place: vec![near, far, (-3, 5)], ..idle() }]);
    assert!(sim.walls.contains_key(&near), "built");
    assert!(!sim.walls.contains_key(&far) && sim.walls.len() == 1, "the one across the world and the one outside it were not");
    let cost = points - sim.players.list[0].mason.points;
    assert!(cost > 0.0 && cost <= WallKind::Wall.cost(), "{cost}");
    step(&mut sim, vec![PlayerInput { delete: vec![near, far], ..idle() }]);
    assert!(sim.walls.is_empty());
    assert!(sim.players.list[0].mason.points > points - cost, "part of it came back");
}

#[test]
fn the_kind_of_wall_an_input_chooses_is_the_kind_it_builds() {
    let mut sim = quiet();
    let here = tile_of(sim.players.list[0].position.x, sim.players.list[0].position.y);
    sim.players.list[0].mason.points = MAX_WALL_POINTS;
    step(&mut sim, vec![PlayerInput { select: Some(2), place: vec![(here.0 + 5, here.1)], ..idle() }]);
    assert_eq!(sim.players.list[0].mason.selected, BUILDABLE[2]);
    assert_eq!(sim.walls[&(here.0 + 5, here.1)].kind, BUILDABLE[2]);
    //(there is no sixth kind, and an input that says there is is put right before the world sees it)
    step(&mut sim, vec![PlayerInput { select: Some(5), ..idle() }]);
    assert_eq!(sim.players.list[0].mason.selected, BUILDABLE[2]);
}

#[test]
fn squads_are_sent_at_an_enemy_by_its_id_and_called_back() {
    let mut sim = quiet();
    let player = sim.players.list[0].position;
    let enemy = dummy(player + Vec2::new(900.0, 0.0));
    let id = enemy.id;
    sim.enemies.push(enemy);
    //an id that isn't there sends no one
    let missing = sim.enemies.iter().map(|enemy| enemy.id).max().unwrap() + 1;
    step(&mut sim, vec![PlayerInput { send: Some(missing), ..idle() }]);
    assert!(sim.units.iter().all(|unit| unit.target.is_none()));
    let step_sent = step(&mut sim, vec![PlayerInput { send: Some(id), ..idle() }]);
    let sent = sim.units.iter().filter(|unit| unit.target == Some(id)).count();
    assert!(sent > 0, "someone went");
    assert!(step_sent.events.iter().any(|event| matches!(event, Event::Popup { text, .. } if text.ends_with("sent"))));
    assert!(step_sent.events.contains(&Event::Cue { cue: Cue::Dispatch, at: None }));
    assert!(!sim.players.list[0].squads.is_empty());
    let step_back = step(&mut sim, vec![PlayerInput { recall: true, ..idle() }]);
    assert!(sim.units.iter().all(|unit| unit.target.is_none()) && sim.players.list[0].squads.is_empty());
    assert!(step_back.events.iter().any(|event| matches!(event, Event::Popup { text, .. } if text.ends_with("recalled"))));
}

fn choosing(sim: &mut Sim) {
    sim.players.list[0].choice = Some(RelicChoice::rare());
    sim.players.list[0].rerolls = 2;
}

#[test]
fn a_player_choosing_a_relic_can_do_nothing_else_until_they_have() {
    let mut sim = quiet();
    choosing(&mut sim);
    let start = sim.players.list[0].position;
    let here = tile_of(start.x, start.y);
    let busy = PlayerInput { place: vec![(here.0 + 4, here.1)], recall: true, ..walking(Vec2::new(1.0, 0.0)) };
    step(&mut sim, vec![busy]);
    assert_eq!(sim.players.list[0].position, start);
    assert!(sim.walls.is_empty());
    assert!(sim.players.list[0].choice.is_some());
}

#[test]
fn a_relic_cannot_be_taken_the_moment_it_is_offered_and_can_after() {
    let mut sim = quiet();
    choosing(&mut sim);
    let before = sim.players.list[0].upgrades.clone();
    let options = sim.players.list[0].choice.as_ref().unwrap().options;
    let pick = PlayerInput { relic: Some(RelicAct::Pick(1)), ..idle() };
    let early = step(&mut sim, vec![pick.clone()]);
    assert!(sim.players.list[0].choice.is_some() && !early.report.save_soon, "so a key that was already down doesn't choose");
    let mut taken = None;
    for _ in 0..((RELIC_LOCKOUT / FRAME) as usize + 5) {
        let late = step(&mut sim, vec![pick.clone()]);
        if sim.players.list[0].choice.is_none() {
            taken = Some(late);
            break;
        }
    }
    let late = taken.expect("taken in the end");
    assert!(late.report.save_soon, "a good moment to save");
    assert_ne!(format!("{:?}", sim.players.list[0].upgrades), format!("{before:?}"));
    assert!(late.events.contains(&Event::Cue { cue: Cue::Relic, at: None }));
    assert!(late.events.iter().any(|event| matches!(event, Event::Popup { text, .. } if *text == format!("{}!", options[1].name()))));
    assert!(late.events.iter().any(|event| matches!(event, Event::Punch(_))));
}

#[test]
fn others_are_asked_for_with_a_reroll_and_there_are_only_so_many() {
    let mut sim = quiet();
    choosing(&mut sim);
    sim.players.list[0].rerolls = 1;
    sim.players.list[0].choice.as_mut().unwrap().age = RELIC_LOCKOUT;
    let reroll = PlayerInput { relic: Some(RelicAct::Reroll), ..idle() };
    step(&mut sim, vec![reroll.clone()]);
    assert_eq!(sim.players.list[0].rerolls, 0);
    assert!(sim.players.list[0].choice.is_some(), "still choosing");
    step(&mut sim, vec![reroll]);
    assert_eq!(sim.players.list[0].rerolls, 0, "no more to spend");
}

#[test]
fn a_relic_that_is_not_on_offer_is_not_taken() {
    let mut sim = quiet();
    choosing(&mut sim);
    sim.players.list[0].choice.as_mut().unwrap().age = RELIC_LOCKOUT;
    let before = sim.players.list[0].upgrades.clone();
    step(&mut sim, vec![PlayerInput { relic: Some(RelicAct::Pick(7)), ..idle() }]);
    assert!(sim.players.list[0].choice.is_some());
    assert_eq!(format!("{:?}", sim.players.list[0].upgrades), format!("{before:?}"));
}

#[test]
fn a_relic_choice_is_for_the_player_it_was_offered_to_and_the_others_carry_on() {
    let mut sim = quiet();
    let second = Vec2::new(sim.players.list[0].position.x + 5000.0, sim.players.list[0].position.y);
    sim.players.join("Bo", second, &sim.walls).unwrap();
    sim.units.extend(army_for(1, 5, second));
    choosing(&mut sim);
    let (first_at, second_at) = (sim.players.list[0].position, sim.players.list[1].position);
    for _ in 0..10 {
        step(&mut sim, vec![walking(Vec2::new(0.0, 1.0)), walking(Vec2::new(0.0, 1.0))]);
    }
    assert_eq!(sim.players.list[0].position, first_at, "choosing");
    assert!(sim.players.list[1].position.y > second_at.y + 10.0, "the other one went on");
}

#[test]
fn two_players_are_stepped_together_each_by_their_own_input() {
    let mut sim = quiet();
    let second = Vec2::new(sim.players.list[0].position.x + 5000.0, sim.players.list[0].position.y);
    sim.players.join("Bo", second, &sim.walls).unwrap();
    sim.units.extend(army_for(1, 8, second));
    let (a, b) = (sim.players.list[0].position, sim.players.list[1].position);
    for _ in 0..20 {
        step(&mut sim, vec![walking(Vec2::new(-1.0, 0.0)), walking(Vec2::new(0.0, -1.0))]);
    }
    let (now_a, now_b) = (sim.players.list[0].position, sim.players.list[1].position);
    assert!(now_a.x < a.x - 10.0 && (now_a.y - a.y).abs() < 0.01);
    assert!(now_b.y < b.y - 10.0 && (now_b.x - b.x).abs() < 0.01);
    let report = step(&mut sim, vec![idle(), idle()]).report;
    assert_eq!((report.boosts.len(), report.pickup_ranges.len()), (2, 2));
}

#[test]
fn a_bigger_window_reaches_further_for_babies() {
    let mut sim = quiet();
    sim.units = army_for(0, 400, sim.players.list[0].position);
    let small = step(&mut sim, vec![PlayerInput::idle(Formation::Free, Vec2::new(800.0, 600.0))]).report.pickup_ranges[0];
    let big = step(&mut sim, vec![PlayerInput::idle(Formation::Free, Vec2::new(3000.0, 2000.0))]).report.pickup_ranges[0];
    assert!(big > small, "{big} against {small}");
}

#[test]
fn an_enemy_destroyed_is_reported_and_the_events_survive_being_sent() {
    let mut sim = quiet();
    let player = sim.players.list[0].position;
    let mut enemy = kind_at("Grunt", player + Vec2::new(0.0, 300.0));
    enemy.health = 1.0;
    enemy.speed = 0.0;
    sim.units = on(&enemy, 0, 30);
    sim.enemies.push(enemy);
    let mut destroyed = 0;
    let mut all = Vec::new();
    for _ in 0..30 {
        let step = step(&mut sim, vec![idle()]);
        destroyed += step.report.destroyed;
        all.extend(step.events);
    }
    assert!(destroyed >= 1, "the grunt fell");
    assert!(sim.run.destroyed >= destroyed);
    assert!(all.iter().any(|event| matches!(event, Event::Destroyed(count) if *count >= 1)));
    assert!(all.iter().any(|event| matches!(event, Event::Cue { cue: Cue::Thud, .. })));
    //every one of them, a frame at a time, can be sent and read back as it was (a colour to within a 255th)
    let mut sent = 0;
    for chunk in all.chunks(MAX_EVENTS_A_MESSAGE) {
        let back = read_events(&write_events(chunk)).expect("read back");
        assert_eq!(back.len(), chunk.len());
        sent += back.len();
    }
    assert_eq!(sent, all.len());
}

#[test]
fn a_step_leaves_nothing_in_the_world_for_the_eyes_and_ears() {
    let mut sim = quiet();
    let player = sim.players.list[0].position;
    let mut enemy = kind_at("Grunt", player + Vec2::new(0.0, 200.0));
    enemy.health = 1.0;
    sim.units = on(&enemy, 0, 20);
    sim.enemies.push(enemy);
    for _ in 0..20 {
        step(&mut sim, vec![idle()]);
        assert!(sim.effects().popups.is_empty() && sim.effects().bursts.is_empty() && sim.effects().cues.is_empty() && sim.effects().announcements.is_empty());
        assert!(sim.effects().shake == 0.0 && sim.effects().punch == 0.0 && sim.effects().marks.is_empty() && sim.effects().shards.is_empty());
    }
}

#[test]
fn the_army_the_run_has_seen_at_most_is_kept() {
    let mut sim = quiet();
    let size = sim.units.len();
    sim.units.extend(army_for(0, 40, sim.players.list[0].position));
    step(&mut sim, vec![idle()]);
    assert_eq!(sim.run.peak_army, size + 40);
    sim.units.truncate(10);
    step(&mut sim, vec![idle()]);
    assert_eq!(sim.run.peak_army, size + 40, "it doesn't go back down");
}

#[test]
fn the_time_of_a_run_is_the_time_the_world_ran_for() {
    let mut sim = quiet();
    let seconds = sim.run.seconds;
    sim.step(vec![idle()], Pace { real_dt: 0.5, dt: 0.0, player_dt: 0.0, stands_still: true }, RULES);
    assert_eq!(sim.run.seconds, seconds, "it stood still");
    sim.step(vec![idle()], Pace { real_dt: 0.5, dt: 0.25, player_dt: 0.0, stands_still: false }, RULES);
    assert!((sim.run.seconds - seconds - 0.25).abs() < 1e-6);
}

#[test]
fn a_world_that_stands_still_leaves_the_enemies_and_the_player_where_they_are() {
    let mut sim = quiet();
    let player = sim.players.list[0].position;
    let enemy = kind_at("Grunt", player + Vec2::new(2000.0, 0.0));
    let at = enemy.position;
    sim.enemies.push(enemy);
    for _ in 0..20 {
        sim.step(vec![walking(Vec2::new(1.0, 0.0))], Pace { real_dt: FRAME, dt: 0.0, player_dt: 0.0, stands_still: true }, RULES);
    }
    assert_eq!(sim.players.list[0].position, player);
    assert_eq!(sim.enemies.iter().find(|enemy| enemy.position == at).map(|enemy| enemy.position), Some(at));
}

#[test]
fn a_world_that_is_captured_can_be_stood_up_again_as_it_was() {
    let mut sim = quiet();
    let here = tile_of(sim.players.list[0].position.x, sim.players.list[0].position.y);
    step(&mut sim, vec![PlayerInput { place: vec![(here.0 + 3, here.1), (here.0 + 4, here.1)], ..walking(Vec2::new(0.0, 1.0)) }]);
    for _ in 0..5 {
        step(&mut sim, vec![idle()]);
    }
    let snapshot = sim.capture(0, 0.5, false);
    assert_eq!(snapshot.zoom, 0.5);
    let again = Sim::from_snapshot(snapshot, RULES);
    assert_eq!(again.units.len(), sim.units.len());
    assert_eq!(again.walls.len(), sim.walls.len());
    assert_eq!(again.players.list[0].position, sim.players.list[0].position);
    assert_eq!(again.players.list[0].mason.points, sim.players.list[0].mason.points);
    assert_eq!(again.run.seconds, sim.run.seconds);
}

#[test]
fn the_rules_are_the_settings_the_world_is_played_by() {
    let mut settings = Settings::default();
    settings.difficulty = Difficulty::Brutal;
    settings.army_limit = 5000;
    assert_eq!(Rules::of(&settings), Rules { army_limit: 5000, difficulty: Difficulty::Brutal });
    let mut sim = quiet();
    sim.step(vec![idle()], Pace::steady(FRAME), Rules::of(&settings));
    assert_eq!(sim.players.list[0].upgrades.army_limit, 5000);
}

#[test]
fn the_player_at_this_keyboard_is_a_player_like_any_other_to_the_step() {
    //(a step takes inputs from anywhere: one made by hand, one read from bytes, and the same one that came from the other end of a wire)
    let mut by_hand = quiet();
    let mut by_wire = quiet();
    let input = PlayerInput { direction: Vec2::new(0.3, 0.9), ..idle() };
    let sent = PlayerInput::from_bytes(&input.to_bytes()).unwrap();
    for _ in 0..30 {
        step(&mut by_hand, vec![input.clone()]);
        step(&mut by_wire, vec![sent.clone()]);
    }
    //(to within a little: the two worlds are not the same world, and an enemy that turns up in one can cost it a unit and so a little speed)
    assert!(by_hand.players.list[0].position.distance(by_wire.players.list[0].position) < 3.0);
}

// ---------- whatever comes in ----------

//a square within `spread` squares of one (across, and down)
fn square_near(seed: &mut u64, here: (i32, i32), spread: u64) -> (i32, i32) {
    (here.0 + (noise(seed) % spread) as i32 - (spread / 2) as i32, here.1 + (noise(seed) % spread) as i32 - (spread / 2) as i32)
}

//numbers that look random and are the same every time
fn noise(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *seed >> 33
}

#[test]
fn bytes_of_any_kind_are_read_as_something_or_nothing_and_never_a_panic() {
    let mut seed = 7;
    for _ in 0..3000 {
        let length = (noise(&mut seed) % 120) as usize;
        let bytes: Vec<u8> = (0..length).map(|_| noise(&mut seed) as u8).collect();
        let _ = PlayerInput::from_bytes(&bytes);
        let _ = read_events(&bytes);
    }
    //(and a good message with any one bit wrong: what is read, if anything, is something a watcher can take in)
    let good = write_events(&every_kind_of_event());
    for bit in 0..good.len() * 8 {
        let mut bad = good.clone();
        bad[bit / 8] ^= 1 << (bit % 8);
        if let Some(events) = read_events(&bad) {
            Feedback::new().absorb(events, 0, Vec2::ZERO);
        }
    }
}

#[test]
fn a_world_stepped_with_what_an_input_with_a_bit_wrong_says_stays_sane() {
    let good = busy_input().to_bytes();
    let mut sim = quiet();
    for bit in 0..good.len() * 8 {
        let mut bad = good.clone();
        bad[bit / 8] ^= 1 << (bit % 8);
        if let Some(input) = PlayerInput::from_bytes(&bad) {
            step(&mut sim, vec![input]);
        }
        let at = sim.players.list[0].position;
        assert!(at.x.is_finite() && at.y.is_finite() && (0.0..=WORLD_SIZE).contains(&at.x) && (0.0..=WORLD_SIZE).contains(&at.y), "bit {bit}: {at}");
    }
}

#[test]
fn two_players_stepped_for_a_while_with_every_kind_of_input_keep_the_world_sane() {
    let mut sim = quiet();
    let second = Vec2::new(sim.players.list[0].position.x + 3000.0, sim.players.list[0].position.y);
    sim.players.join("Bo", second, &sim.walls).unwrap();
    sim.units.extend(army_for(1, 200, second));
    sim.units.extend(army_for(0, 150, sim.players.list[0].position));
    let mut seed = 3;
    for frame in 0..900 {
        if frame % 150 == 0 {
            sim.players.list[(frame / 150) % 2].offer(RelicChoice::rare());
        }
        let ids: Vec<u32> = sim.enemies.iter().map(|enemy| enemy.id).collect();
        let inputs: Vec<PlayerInput> = (0..2)
            .map(|who| {
                let here = tile_of(sim.players.list[who].position.x, sim.players.list[who].position.y);
                PlayerInput {
                    direction: Vec2::from_angle((noise(&mut seed) % 628) as f32 / 100.0) * (noise(&mut seed) % 3) as f32 / 2.0,
                    screen: SCREEN,
                    formation: FORMATIONS[(noise(&mut seed) % 4) as usize],
                    select: (noise(&mut seed) % 4 == 0).then_some((noise(&mut seed) % 5) as u8),
                    place: (0..(noise(&mut seed) % 4)).map(|_| square_near(&mut seed, here, 30)).collect(),
                    delete: (0..(noise(&mut seed) % 3)).map(|_| square_near(&mut seed, here, 30)).collect(),
                    tower: (noise(&mut seed) % 50 == 0).then(|| square_near(&mut seed, here, 20)),
                    recall: noise(&mut seed) % 97 == 0,
                    send: (noise(&mut seed) % 40 == 0 && !ids.is_empty()).then(|| ids[(noise(&mut seed) as usize) % ids.len()]),
                    relic: match noise(&mut seed) % 20 {
                        0 => Some(RelicAct::Pick((noise(&mut seed) % 3) as u8)),
                        1 => Some(RelicAct::Reroll),
                        _ => None,
                    },
                    evolve: (noise(&mut seed) % 30 == 0).then(|| EvolvePick { champion: noise(&mut seed) as u32, tier: (noise(&mut seed) % 4) as u8, choice: (noise(&mut seed) % 3) as u8 }),
                }
            })
            .collect();
        let step = step(&mut sim, inputs);
        assert!(read_events(&write_events(&step.events)).is_some(), "frame {frame}: the events can be sent");
    }
    for player in &sim.players.list {
        assert!(player.position.x.is_finite() && player.position.y.is_finite());
        assert!(player.mason.points >= 0.0 && player.mason.points <= MAX_WALL_POINTS);
    }
    assert!(sim.units.iter().all(|unit| unit.x.is_finite() && unit.y.is_finite() && (unit.owner as usize) < 2));
    assert!(sim.run.seconds > 14.0);
}

#[test]
fn a_player_chooses_how_their_own_champion_grows_and_only_once() {
    let mut sim = quiet();
    let mut champion = champion_unit(champion_named("Medic").unwrap(), sim.players.list[0].position);
    champion.level = EVOLUTION_LEVELS[0];
    let id = champion.id;
    sim.units.push(champion);
    sim.add_player("Bob").expect("room for another player");
    let evolutions = |sim: &Sim| sim.units.iter().find(|unit| unit.id == id).expect("the medic is there").evolutions;
    let pick = |champion: u32, choice: u8| PlayerInput { evolve: Some(EvolvePick { champion, tier: 0, choice }), ..idle() };
    //a choice for a champion that isn't there, a way that isn't one of the two, or a choice by another player, is nothing
    step(&mut sim, vec![pick(id + 100_000, 0)]);
    step(&mut sim, vec![pick(id, 7)]);
    step(&mut sim, vec![idle(), pick(id, 0)]);
    assert_eq!(evolutions(&sim), 0);
    let chose = step(&mut sim, vec![pick(id, 1)]);
    assert_eq!(evolutions(&sim), 0b10);
    assert!(chose.report.save_soon, "a good moment to save");
    step(&mut sim, vec![pick(id, 0)]);
    assert_eq!(evolutions(&sim), 0b10, "a choice once made is kept");
}

#[test]
fn a_champions_volley_flies_in_the_world_and_its_watchers_are_told_of_it_once() {
    let mut sim = quiet();
    let at = sim.players.list[0].position;
    let mut champion = champion_unit(champion_named("Sharpshooter").unwrap(), at);
    champion.power = 0.0;
    sim.units.push(champion);
    let mut target = kind_at("Grunt", at + Vec2::new(350.0, 0.0));
    target.max_health = 1.0e9;
    target.health = 1.0e9;
    target.attack = 0.0;
    sim.enemies.push(target);
    let fired = step(&mut sim, vec![idle()]);
    assert_eq!(fired.events.iter().filter(|event| matches!(event, Event::Volley(_))).count(), 1);
    assert!(read_events(&write_events(&fired.events)).is_some(), "the volley can be sent");
    for _ in 0..30 {
        step(&mut sim, vec![idle()]);
    }
    assert!(sim.enemies.iter().any(|enemy| enemy.health < 1.0e9), "the round hit");
    assert!(sim.bolts.is_empty() || sim.bolts.iter().all(|bolt| bolt.kind == BoltKind::Round));
}

#[test]
fn the_front_of_the_army_fires_at_an_enemy_ahead_and_no_message_carries_its_bolts() {
    let mut sim = quiet();
    let at = sim.players.list[0].position;
    let ahead = sim.players.list[0].facing.normalize() * 600.0;
    let mut target = kind_at("Grunt", at + ahead);
    target.max_health = 1.0e9;
    target.health = 1.0e9;
    target.attack = 0.0;
    sim.enemies.push(target);
    let mut told = 0;
    let mut tracers = 0;
    for _ in 0..90 {
        let step = step(&mut sim, vec![idle()]);
        told += step.events.iter().filter(|event| matches!(event, Event::Volley(_))).count();
        tracers = tracers.max(sim.bolts.iter().filter(|bolt| bolt.kind == BoltKind::Tracer).count());
    }
    assert!(tracers > 0, "the army fired");
    assert_eq!(told, 0, "a guest works the vanguard's fire out for itself");
    assert!(sim.enemies.iter().any(|enemy| enemy.health < 1.0e9), "and it hit");
}
