//! one run of the game: the frame loop that ties everything together

use crate::prelude::*;

//how a run of the game is going, or went
#[derive(Clone)]
pub(crate) struct Run {
    pub(crate) destroyed: usize, //enemies
    pub(crate) lost: usize,      //units that died
    pub(crate) peak_army: usize, //the most units at once
    pub(crate) bosses: usize,    //bosses slain
    pub(crate) nests: usize,     //destroyed
    pub(crate) villages: usize,  //liberated
    pub(crate) seconds: f32,
    pub(crate) citadels: usize,  //times the lord of the citadel has fallen
}

//the moments after a boss falls: the game freezes, then crawls
#[derive(Default)]
pub(crate) struct Stillness {
    pub(crate) freeze: f32, //seconds left of standing still
    pub(crate) slow: f32,   //seconds left of running slowly, after that
}

impl Stillness {
    //how fast the world runs, as a share of normal
    pub(crate) fn pace(&self) -> f32 {
        if self.freeze > 0.0 {
            0.0
        } else if self.slow > 0.0 {
            SLOWMO_PACE
        } else {
            1.0
        }
    }

    pub(crate) fn tick(&mut self, real_dt: f32) {
        if self.freeze > 0.0 {
            self.freeze = (self.freeze - real_dt).max(0.0);
        } else {
            self.slow = (self.slow - real_dt).max(0.0);
        }
    }

    pub(crate) fn after_a_boss() -> Stillness {
        Stillness { freeze: HIT_STOP_SECONDS, slow: SLOWMO_SECONDS }
    }
}

//whether the world stands still: it does for one player while they choose a relic, see the victory or have the menu up. with others in it
//it never does, as it would stop for them all
pub(crate) fn world_stands_still(players: usize, modal: bool) -> bool {
    modal && players <= 1
}

//how a run came to an end
pub(crate) enum Ending {
    Fallen(Run),      //the army is gone
    Reload(Snapshot), //the player went back to their quick save
    Title,            //the player left for the title screen, the run kept as the game's own save
    Left(String),     //a stay in the game of another is over, and why
    Lost(Reconnect, String), //the connection to the game of another went, and why: the player is to be put back in it if it can be done
}

//how long a player with no army is left before they have a few units again, in a world that goes on without them
pub(crate) const REVIVE_SECONDS: f32 = 4.0;
const WORLD_WAIT_SECONDS: f32 = 20.0; //how long a joined player waits for the world before giving up

//the world at the start of a run: the player in the middle of it, with a few units, in the starting house
pub(crate) fn new_run() -> Snapshot {
    let player = Vec2::splat(WORLD_SIZE / 2.0);
    let units = starting_units(player.x, player.y);
    let mut enemies = Vec::new();
    respawn_enemies(&mut enemies, player.x, player.y, units.len());
    let mason = Mason::new();
    Snapshot {
        player,
        zoom: 1.0,
        run: Run { destroyed: 0, lost: 0, peak_army: units.len(), bosses: 0, nests: 0, villages: 0, seconds: 0.0, citadels: 0 },
        rerolls: rerolls_earned(0),
        upgrades: Upgrades::default(),
        wall_kind: mason.selected,
        wall_points: mason.points,
        director: Director::new(),
        units,
        babies: Vec::new(),
        walls: starting_house(),
        sites: generate_sites(),
        enemies,
        choice: None,
        victory: false,
        hordes: Hordes::new(),
        company: Vec::new(),
    }
}

//one run of the game, from the starting house (or from a save) until the army is gone. the world is a `Sim` that is stepped once a frame with what
//the player at this keyboard asked for (`Controls`), and what comes out of the step (`Feedback`) is what is shown and heard. everything else here is
//the player's own: the menu, the saves, the camera, the screens and what is drawn
pub(crate) async fn play(sounds: &mut Sounds, saves: &Saves, settings: &mut Settings, start: Option<Snapshot>, link: &mut Link) -> Ending {
    //without a seed every run gets the same random numbers
    rand::srand(miniquad::date::now() as u64);
    //a game that is another's is played in a copy of its world, which the host keeps up to date
    let mut guest = match std::mem::replace(link, Link::Solo) {
        Link::Guest(client, lobby) => Some(Guest::new(client, lobby)),
        other => {
            *link = other;
            None
        }
    };
    //(`choice`, the relics on offer, is in the world: it stands still while they are, and comes from the save along with the rest)
    //victory: how long the victory screen has been up, the world stands still while it is
    let (mut sim, mut zoom, mut victory): (Sim, f32, Option<f32>) = if guest.is_some() {
        (Sim::mirror(), 1.0, None)
    } else {
        let snapshot = start.unwrap_or_else(new_run);
        let (zoom, victory) = (snapshot.zoom, snapshot.victory.then_some(0.0));
        (Sim::from_snapshot(snapshot, Rules::of(settings)), zoom, victory)
    };
    if let Some(seat) = guest.as_mut() {
        if let Err(why) = await_world(seat, &mut sim).await {
            return Ending::Left(why);
        }
    }
    //the players at other machines that are in the game already
    let mut table = Table::new();
    if let Link::Host(host) = &*link {
        let members = host.members();
        sim.name_host(&members[0].name);
        for member in members.iter().skip(1) {
            if table.seat(&mut sim, member.slot, &member.name).is_err() {
                host.kick(member.slot);
            }
        }
    }
    //(a game that is hosted or joined has fewer shapes to choose from, see `Formation::choices`)
    let online = guest.is_some() || matches!(&*link, Link::Host(_));
    let mut empty_for = [0.0f32; MAX_PLAYERS]; //how long each player has had no army, in a world with others in it
    //the player at this keyboard
    let mut me = guest.as_ref().map_or(0, |guest| guest.me());
    let mut controls = Controls::new();
    let mut feedback = Feedback::new();
    let mut frame_time = 1.0 / 60.0; //smoothed so the number on the HUD can be read
    let mut intro: Option<Intro> = None; //a boss's entrance
    //the family of the last boss that had an entrance, which one already in the world when a save is loaded has had
    let mut shown_boss: Option<u32> = current_boss(&sim.enemies).and_then(|boss| boss.boss.as_ref().map(|state| state.family));
    let mut stillness = Stillness::default();
    let mut milestone = next_milestone(sim.units.len(), sim.players.list[me].upgrades.army_limit);
    let mut milestone_limit = sim.players.list[me].upgrades.army_limit; //the limit it was worked out for
    let mut banners = Banners::new();
    let mut fallen_for = 0.0; //how long there has been no army and nothing to make one from
    let mut down_for = 0.0; //how long the player has had no army in a game with others, which goes on
    let mut map_view = MapView::Near;
    let mut autosave_in = AUTOSAVE_SECONDS; //seconds until the game saves by itself
    let mut heartbeat_in = 0.0; //seconds until the next beat while the army is failing
    let mut beat = 0.0; //1 as the heart beats, fading
    let mut peril_now = 0.0; //how badly the army was doing at the end of the last frame
    let mut pause: Option<Pause> = None; //the menu, while it is up
    let best = saves.load_high_scores().best(); //the score to beat
    //the run as it is now, to be kept
    macro_rules! capture {
        () => {
            sim.capture(me, zoom, victory.is_some())
        };
    }
    //the game is closed, and so is the one that is hosted (the players in it are told) or the one that was joined (the host is told)
    macro_rules! quit_game {
        () => {{
            drop(guest.take());
            *link = Link::Solo;
            std::process::exit(0)
        }};
    }

    loop {
        let rules = Rules::of(settings);
        let real_dt = get_frame_time().min(MAX_FRAME_TIME);
        frame_time += (get_frame_time() - frame_time) * 0.05;
        //what the host has said since the last frame, which is the world as it is now (a game that is another's is its host's to step)
        if let Some(seat) = guest.as_mut() {
            if let Err(lost) = seat.hear(&mut sim) {
                return if lost.retry { Ending::Lost(seat.reconnection(), lost.why) } else { Ending::Left(lost.why) };
            }
            me = seat.me();
        }
        //a world with others in it is not saved: it is the host's, and goes on without them
        let multi = sim.players.len() > 1;
        //a game with others in it is kept in a save of its own, by the host (whose world it is, with everyone's army in it)
        let keeps = guest.is_none();
        let (quick_slot, auto_slot) = if multi { (Slot::Party, Slot::Party) } else { (Slot::Quick, Slot::Auto) };
        //the game is saved when the window is closed, when the player asks (F5), and by itself now and then and each time a
        //relic is taken. F9 goes back to what F5 saved. none of it while the army is falling, as there would be nothing to go
        //back to
        let quitting = is_quit_requested();
        let by_hand = keeps && is_key_pressed(KeyCode::F5);
        autosave_in -= real_dt;
        if keeps && fallen_for == 0.0 && (quitting || by_hand || autosave_in <= 0.0) {
            let snapshot = capture!();
            if quitting || by_hand {
                let slot = if quitting { auto_slot } else { quick_slot };
                match saves.write(slot, &snapshot.to_text()) {
                    Ok(()) if !quitting => {
                        banners.push("GAME SAVED".to_owned(), CYAN);
                        feedback.fx.cue(Cue::Pop);
                    }
                    Ok(()) => {}
                    Err(error) => {
                        eprintln!("could not save the game to {}: {error}", saves.path(slot).display());
                        banners.push("COULD NOT SAVE THE GAME".to_owned(), ALERT);
                    }
                }
            } else {
                saves.write_in_background(auto_slot, snapshot);
            }
            autosave_in = AUTOSAVE_SECONDS;
        }
        if quitting {
            quit_game!();
        }
        if !multi && is_key_pressed(KeyCode::F9) {
            match saves.read(Slot::Quick) {
                Ok(snapshot) => return Ending::Reload(snapshot),
                Err(error) => {
                    eprintln!("could not load the quick save: {error}");
                    let problem = if saves.path(Slot::Quick).exists() { "THE QUICK SAVE CAN'T BE READ" } else { "NO QUICK SAVE YET: F5 MAKES ONE" };
                    banners.push(problem.to_owned(), ALERT);
                }
            }
        }
        //people joining and leaving a game that is being hosted, and what the ones in it ask
        if let Link::Host(host) = &*link {
            for event in host.poll() {
                match event {
                    HostEvent::Joined { slot, name, .. } => match table.seat(&mut sim, slot, &name) {
                        Ok(_) => banners.push(format!("{} JOINED", name.to_uppercase()), CYAN),
                        Err(why) => {
                            banners.push(format!("{} COULD NOT JOIN: {}", name.to_uppercase(), why.to_uppercase()), ALERT);
                            host.kick(slot);
                        }
                    },
                    HostEvent::Left { slot, name, .. } => {
                        table.unseat(&mut sim, slot);
                        banners.push(format!("{} LEFT", name.to_uppercase()), MUTED);
                    }
                    HostEvent::TurnedAway { .. } => {}
                }
            }
            table.hear(host.take_inputs());
            //(a game that is on the lobby list says who is in it now, and the player is told if it falls off)
            host.announce(true);
            for news in host.listing_news() {
                if let DirectoryEvent::Unlisted(problem) = news {
                    eprintln!("the game is not on the lobby list: {problem}");
                    banners.push("NOT ON THE LOBBY LIST".to_owned(), ALERT);
                }
            }
        }
        //the world stands still while a relic is being chosen or the victory is shown, crawls while a boss makes its entrance,
        //and freezes for a moment when one falls, with one player to wait for. with others in it nothing slows or stops the world for one of them
        let paused = sim.players.list[me].choice.is_some() || victory.is_some() || pause.is_some();
        let solo = sim.players.len() == 1;
        let stands_still = guest.is_none() && world_stands_still(sim.players.len(), paused);
        if is_key_pressed(KeyCode::F11) {
            settings.fullscreen = !settings.fullscreen;
            miniquad::window::set_fullscreen(settings.fullscreen);
            saves.save_settings(settings);
        }
        //(Esc opens the menu on a frame the world is running, so the key that opens it is not also taken to close it)
        if !paused && fallen_for == 0.0 && is_key_pressed(KeyCode::Escape) {
            pause = Some(Pause::new(if multi { None } else { saves.info(Slot::Quick) }, Vec2::from(mouse_position())));
        }
        if let Some(menu) = pause.as_mut() {
            menu.mode = if guest.is_some() { MenuMode::Guest } else if multi { MenuMode::Hosting } else { MenuMode::Solo };
            menu.online = online;
        }
        if let Some(menu) = pause.as_mut().filter(|_| paused) {
            let input = MenuInput::read(&mut menu.last_mouse);
            let was_fullscreen = settings.fullscreen;
            match menu.update(settings, &input, Vec2::new(screen_width(), screen_height()), real_dt) {
                PauseEvent::Nothing => {}
                PauseEvent::Resume => pause = None,
                PauseEvent::Load if multi => menu.tell("loading is switched off while others are in the game", ALERT),
                PauseEvent::Save if !keeps => menu.tell("only the host can save the game", ALERT),
                PauseEvent::Save => {
                    let snapshot = capture!();
                    match saves.write(quick_slot, &snapshot.to_text()) {
                        Ok(()) => {
                            menu.tell("game saved", CYAN);
                            feedback.fx.cue(Cue::Pop);
                            menu.saved = saves.info(Slot::Quick);
                        }
                        Err(error) => {
                            eprintln!("could not save the game to {}: {error}", saves.path(quick_slot).display());
                            menu.tell("could not save the game, see the console for why", ALERT);
                        }
                    }
                }
                PauseEvent::Load => match saves.read(Slot::Quick) {
                    Ok(snapshot) => return Ending::Reload(snapshot),
                    Err(error) => {
                        eprintln!("could not load the quick save: {error}");
                        menu.tell("the quick save can't be read", ALERT);
                    }
                },
                leaving @ (PauseEvent::Title | PauseEvent::Quit) => {
                    //the run is kept as the game's own save, which the title screen offers to carry on from
                    if keeps {
                        let snapshot = capture!();
                        if let Err(error) = saves.write(auto_slot, &snapshot.to_text()) {
                            eprintln!("could not save the game to {}: {error}", saves.path(auto_slot).display());
                        }
                    }
                    saves.save_settings(settings);
                    if leaving == PauseEvent::Quit {
                        quit_game!();
                    }
                    if let Some(seat) = guest.as_mut() {
                        seat.leave();
                    }
                    return Ending::Title;
                }
                PauseEvent::Changed => {
                    settings.apply();
                    settings.tune(sounds);
                    if settings.fullscreen != was_fullscreen {
                        miniquad::window::set_fullscreen(settings.fullscreen);
                    }
                    saves.save_settings(settings);
                }
            }
        }
        //(with others in the world it can't be slowed for one of them, so a boss's entrance there is the camera going to it and nothing else: the player is
        //not held, and it is cut short if their army is in trouble)
        if !solo && peril_now > 0.0 {
            intro = None;
        }
        let entrance = intro.is_some() && !paused;
        let held = entrance && solo;
        if !paused {
            stillness.tick(real_dt);
        }
        let pace = if stands_still {
            0.0
        } else if held {
            INTRO_PACE
        } else {
            stillness.pace()
        };
        let dt = real_dt * pace;
        //the player alone is held still for the entrance, and anyone can skip it
        let player_dt = if held { 0.0 } else { dt };
        if let Some(shown) = intro.as_mut() {
            shown.age += real_dt;
        }
        if intro.as_ref().is_some_and(|shown| shown.over()) || (entrance && is_key_pressed(KeyCode::Space)) {
            intro = None;
        }
        if !entrance {
            handle_zoom(&mut zoom);
        }

        //what the player at this keyboard asks for: the keys that are about the screen and the sound are taken here, the rest are the world's
        if is_key_pressed(KeyCode::Tab) {
            map_view = map_view.other();
        }
        if is_key_pressed(KeyCode::M) {
            sounds.muted = !sounds.muted;
        }
        if is_key_pressed(KeyCode::N) {
            sounds.music.on = !sounds.music.on;
        }
        let screen = Vec2::new(screen_width(), screen_height());
        let at = sim.players.list[me].position; //where the player starts the frame
        let map = Minimap::new(screen, map_view, at);
        let over_map = map.contains(Vec2::from(mouse_position()));
        if let Some(age) = victory.as_mut() {
            *age += get_frame_time();
            if *age >= VICTORY_LOCKOUT && get_last_key_pressed().is_some() {
                //the run goes on, with a rare relic for having won
                victory = None;
                sim.players.list[me].offer(RelicChoice::rare());
            }
        }
        let acting = !paused && !held;
        //(what the mouse points at is what the camera is looking at, which is not the player while an entrance goes on)
        let (look_at, look_zoom) = match &intro {
            Some(shown) if entrance => shown.view(at, zoom),
            _ => (at, zoom),
        };
        if acting && is_key_pressed(KeyCode::F) {
            settings.formation = settings.formation.next(online);
            saves.save_settings(settings);
            feedback.fx.cue(Cue::Pop);
            feedback.fx.popup(at, RADIUS * 3.0, 0.0, format!("{} FORMATION", settings.formation.name().to_uppercase()), CYAN);
        }
        let formation = settings.formation.in_game(online);
        let input = if acting {
            controls.read(&ControlView { position: look_at, zoom: look_zoom, screen, formation, map: &map, enemies: &sim.enemies })
        } else {
            PlayerInput::idle(formation, screen)
        };
        //(choosing a relic is all they can do until they have)
        let relic = if sim.players.list[me].choice.is_some() { read_relic_act(screen) } else { None };

        //the world: stepped here, or by the host of a game that is another's
        let input = PlayerInput { relic, ..input };
        let Step { events, report } = match guest.as_mut() {
            Some(seat) => Step { events: seat.frame(&mut sim, &input, look_zoom, real_dt), report: seat.report(&sim) },
            None => {
                let step = sim.step(table.inputs(&sim, input, real_dt), Pace { real_dt, dt, player_dt, stands_still }, rules);
                if let Link::Host(host) = &*link {
                    table.tell(host, &mut sim, &step.events, &step.report, real_dt);
                }
                step
            }
        };
        let player = sim.players.list[me].position;
        feedback.absorb(events, me, player);
        if report.save_soon {
            autosave_in = 0.0;
        }
        if solo && report.citadel {
            victory = Some(0.0);
        }
        if solo && (report.citadel || report.boss_slain) {
            stillness = Stillness::after_a_boss();
        }
        //a boss that has just turned up is shown off, once
        if !stands_still {
            if let Some(boss) = current_boss(&sim.enemies) {
                let family = boss.boss.as_ref().map(|state| state.family);
                if family != shown_boss {
                    shown_boss = family;
                    //(not one that has been fought for a while: it is not new to a player who joins late)
                    if family_health(&sim.enemies, boss) > FRESH_BOSS {
                        intro = Some(Intro::new(boss));
                    }
                }
            }
        }
        let boost = report.boosts[me];
        let pickup_range = report.pickup_ranges[me];

        //round army sizes get a message, once. the army being gone is the end, but not while there are babies to be picked up
        let sizes = army_sizes(&sim.units);
        let my_army = sizes[me];
        //(how big every army is: the host tells the others, who are not sent the units that are out of their sight)
        let army_list: Vec<usize> = match &guest {
            Some(seat) if seat.replica.armies.len() == sim.players.len() => seat.replica.armies.clone(),
            _ => sizes[..sim.players.len()].to_vec(),
        };
        let members = match &*link {
            Link::Host(host) => host.members(),
            _ => guest.as_ref().map(|seat| seat.lobby.members.clone()).unwrap_or_default(),
        };
        let mates = if multi { mates_of(&sim.players, &army_list, &members, me) } else { Vec::new() };
        list_players(mates.len());
        let colors = team_colors(&sim.players);
        let team_tint = multi.then_some(&colors[..]); //what the armies are tinted towards, in a game with others
        let team = team_tint.map(|colors| Team { colors, mates: &mates });
        down_for = if multi && my_army == 0 { down_for + real_dt } else { 0.0 };
        //(in a world with others in it a player with no army has a few units again after a while, and the world goes on)
        if multi && guest.is_none() {
            for (player, empty) in empty_for.iter_mut().enumerate().take(sim.players.len()) {
                *empty = if sizes[player] == 0 && !sim.players.list[player].is_away() { *empty + real_dt } else { 0.0 };
                if *empty >= REVIVE_SECONDS {
                    sim.revive(player);
                    *empty = 0.0;
                    banners.push(if player == me { "YOUR ARMY FELL: A FEW UNITS COME TO YOUR SIDE".to_owned() } else { format!("{}'S ARMY FELL", sim.players.list[player].name.to_uppercase()) }, ALERT);
                }
            }
        }
        let limit = sim.players.list[me].upgrades.army_limit;
        if limit != milestone_limit {
            //the limit was changed in the settings: the sizes ahead are worked out again
            milestone_limit = limit;
            milestone = next_milestone(my_army, milestone_limit);
        }
        while my_army >= milestone {
            let text = if milestone == limit { format!("FULL STRENGTH: {milestone} UNITS") } else { format!("ARMY OF {milestone}!") };
            banners.push(text, Color::new(1.0, 0.9, 0.4, 1.0));
            feedback.fx.cue(Cue::Milestone);
            feedback.fx.add_shake(0.25);
            feedback.fx.add_punch(0.5);
            milestone = next_milestone(milestone, limit);
        }
        for (text, color) in feedback.fx.announcements.drain(..) {
            banners.push(text, color);
        }
        banners.update(real_dt);
        if !multi && my_army == 0 && sim.babies.is_empty() {
            if fallen_for == 0.0 {
                feedback.fx.cue(Cue::Defeat);
                feedback.fx.add_shake(0.8);
            }
            fallen_for += dt;
        } else {
            fallen_for = 0.0;
        }

        //a failing army has a heart that beats faster the closer it is to the end
        let in_peril = peril(my_army, sim.run.peak_army);
        peril_now = in_peril;
        beat = (beat - real_dt * 3.0f32).max(0.0);
        if in_peril > 0.0 && !paused {
            heartbeat_in -= real_dt;
            if heartbeat_in <= 0.0 {
                feedback.fx.cue(Cue::Heartbeat);
                beat = 1.0;
                heartbeat_in = 1.15 - 0.65 * in_peril;
            }
        } else {
            heartbeat_in = 0.3;
        }
        feedback.update(dt, player);
        sounds.play(&feedback.fx.take_cues(), player, get_time());
        //the music follows the size of the army and how much danger there is: enemies that have noticed the player and are near
        let chasing = sim.enemies.iter().filter(|enemy| enemy.chasing && enemy.position.distance(player) < MUSIC_DANGER_RANGE).count();
        sounds.tune_music(&Mood { army: my_army, danger: (chasing as f32 / MUSIC_DANGER_ENEMIES).min(1.0), boss: current_boss(&sim.enemies).is_some(), peril: in_peril }, real_dt);
        //the camera goes to the boss and back during its entrance
        let (camera_at, camera_zoom) = match &intro {
            Some(shown) if entrance => shown.view(player, zoom),
            _ => (player, zoom),
        };
        //units off screen don't leave trails, nobody would see them
        let view = visible_area(camera_at.x, camera_at.y, camera_zoom);
        for other in &sim.players.list {
            emit_trail(&mut feedback.particles, other.position, other.velocity, PLAYER_SPEED * boost, dt, other.color);
        }
        feedback.run(&sim.players.list, dt);
        for unit in &sim.units {
            let position = Vec2::new(unit.x, unit.y);
            if view.contains(position) {
                emit_trail(&mut feedback.particles, position, unit.velocity, unit.speed * boost, dt, unit_color_in(unit, team_tint));
            }
        }
        update_particles(&mut feedback.particles, dt);

        //what the run is worth now, and the best there has been (the best run kept, or this one if it is better already)
        let score = score_of(&sim.run, settings.difficulty);
        //(the enemies near the player at this keyboard are as fast as their army makes them)
        let enemy_boost = speed_multiplier(my_army);
        let ranges = sim.arrange();
        let Sim { players, run, director, hordes, units, babies, walls, sites, enemies, shots, hazards, pulls, .. } = &sim;
        let fx = &feedback.fx;

        clear_background(BLACK);
        //the camera jumps in a little for big moments
        let drawn_zoom = camera_zoom * (1.0 + PUNCH_ZOOM * fx.punch * settings.shake);
        let shaken = camera_at + shake_offset(fx.shake * settings.shake, drawn_zoom);
        let camera = follow_camera(shaken.x, shaken.y, drawn_zoom);
        set_camera(&camera);
        let drawn = visible_area(shaken.x, shaken.y, drawn_zoom);
        let hovered = if over_map || paused || entrance { None } else { hovered_tile(player.x, player.y, zoom) };
        draw_world(camera_at.x, camera_at.y, drawn_zoom, hovered);
        draw_floor(camera_at.x, camera_at.y, drawn_zoom, walls, get_time() as f32);
        draw_discs(Layer::Marks, &mark_discs(&feedback.ground, drawn), &camera);
        draw_sites(sites, view, get_time() as f32);
        draw_discs(Layer::Puffs, &particle_discs(&feedback.particles), &camera);
        let watch: Vec<Vec2> = players.list.iter().map(|other| other.position).collect();
        draw_enemies(enemies, &players.list[me].squads, get_time() as f32, enemy_boost, &camera, &watch, settings.simple_shapes);
        draw_shards(&feedback.shards, drawn);
        draw_hazards(hazards, get_time() as f32);
        draw_pulls(pulls, get_time() as f32);
        draw_chests(&hordes.chests, drawn, get_time() as f32);
        draw_discs(Layer::Army, &unit_discs_in(units, drawn, team_tint, settings.unit_shape), &camera);
        draw_discs(Layer::Babies, &baby_discs(babies, drawn), &camera);
        draw_champions(units, drawn, drawn_zoom, get_time() as f32);
        if let Some(colors) = team_tint {
            draw_champion_rings(units, colors, drawn, drawn_zoom);
        }
        draw_shots(shots);
        //(one that has gone is a faint mark of where they will find their army)
        //(a player is a little runner, or the circle they were when they are too small on the screen to make one out: never both)
        let figure = runner_shown(RADIUS * camera.zoom.x.abs() * screen_width() / 2.0);
        let is_gone = |i: usize| mates.get(i).is_some_and(|mate| mate.away);
        let avatars: Vec<Disc> = players.list.iter().enumerate().filter_map(|(i, other)| {
            if is_gone(i) {
                Some(Disc::new(other.position.x, other.position.y, RADIUS, Color::new(other.color.r, other.color.g, other.color.b, 0.3)))
            } else if !figure {
                Some(Disc::new(other.position.x, other.position.y, RADIUS, other.color).edged(UNIT_BORDER))
            } else {
                None
            }
        }).collect();
        draw_discs(Layer::Player, &avatars, &camera);
        draw_chest_holds(&hordes.chests, drawn);
        //the walls go on top of everything in the world, the numbers and the screen over them are for reading
        draw_walls(camera_at.x, camera_at.y, drawn_zoom, hovered, walls, if paused || entrance { None } else { Some(players.list[me].mason.selected) }, get_time() as f32);
        //(except the little runners, who are taller than the circle was and would lose their heads to a wall they stand under)
        if figure {
            for (i, (other, runner)) in players.list.iter().zip(&feedback.runners).enumerate() {
                if !is_gone(i) {
                    draw_runner(runner, other.position, other.color);
                }
            }
        }
        draw_beams(&fx.beams);
        draw_rings(&fx.rings);
        draw_popups(&fx.popups, drawn_zoom);
        set_default_camera();
        draw_tags(&mates, shaken, drawn_zoom);
        draw_vignette(in_peril, beat);
        let my_units = &units[ranges[me].clone()];
        draw_roster(my_units, if pause.is_some() { Vec2::splat(-1000.0) } else { Vec2::from(mouse_position()) });
        draw_hud(player.x, player.y, frame_time, my_army, run, boost, pickup_range, sounds.muted, sounds.music.on, formation, director, sites, players.list[me].upgrades.unit_cap(), hordes.status(enemies), &players.list[me].upgrades.damage_text(), score, best);
        if settings.hints && !paused && run.seconds < HINT_SECONDS {
            draw_hints(run.seconds);
        }
        if let Some(warning) = blast_warning(hazards, player, my_units) {
            draw_blast_warning(&warning, get_time() as f32);
        }
        draw_build_bar(&players.list[me].mason, get_time() as f32);
        if players.list[me].choice.is_none() {
            draw_combo(&feedback.combo);
        }
        //the player has moved since the map was made, so make it again to draw
        let map = Minimap::new(screen, map_view, player);
        let mouse = Vec2::from(mouse_position());
        if !paused && !entrance {
            draw_relic_bar(&players.list[me].upgrades, mouse);
        }
        //the enemy under the mouse, in the world or as a dot on the map
        let under_mouse = if paused || entrance {
            None
        } else if over_map {
            minimap_enemy_at(&map, mouse, enemies)
        } else {
            enemy_at(mouse_world(player.x, player.y, zoom), enemies, ENEMY_CLICK_MARGIN / zoom)
        };
        draw_minimap(&map, map_view, player, view, if over_map { under_mouse } else { None }, units, enemies, babies, &players.list[me].squads, walls, hazards, sites, team.as_ref(), get_time() as f32);
        if let Some(coming) = &hordes.coming {
            for at in [Some(coming.at), coming.other].into_iter().flatten() {
                draw_horde_ping(&map, at, get_time() as f32);
            }
        }
        draw_chest_marks(&map, &hordes.chests, get_time() as f32);
        if let Some(enemy) = under_mouse.map(|i| &enemies[i]) {
            draw_enemy_tooltip(enemy, my_units.iter().filter(|unit| unit.target.is_none()).count(), map.rect);
        }
        for at in hordes.headings(enemies, player) {
            draw_horde_arrow(at, camera_at, camera_zoom, hordes.coming.map(|coming| coming.left));
        }
        draw_captain_arrows(enemies, camera_at, camera_zoom);
        draw_mate_arrows(&mates, camera_at, camera_zoom);
        draw_team_panel(&mates);
        //(a player with no army in a world that goes on has a few units again in a moment)
        if down_for > 0.3 {
            draw_fallen((REVIVE_SECONDS - down_for).max(0.0), down_for - 0.3);
        }
        draw_chest_arrow(&hordes.chests, player, camera_at, camera_zoom);
        if let Some(boss) = current_boss(enemies) {
            let alive = enemies.iter().filter(|other| other.boss.as_ref().zip(boss.boss.as_ref()).is_some_and(|(a, b)| a.family == b.family)).count();
            draw_boss_bar(boss, family_health(enemies, boss), alive);
            draw_boss_arrow(boss, camera_at, camera_zoom);
        }
        if let Some(shown) = intro.as_ref().filter(|_| entrance) {
            draw_intro(shown);
        }
        if let Some(quiet) = guest.as_ref().map(|seat| seat.silence()).filter(|&quiet| quiet > QUIET_SECONDS) {
            draw_trouble(quiet);
        }
        if let Some(menu) = &pause {
            let hosting = match &*link {
                Link::Host(host) => Some(match sim.players.away_names().as_slice() {
                    [] => hosting_line(host),
                    away => format!("{} (AWAY: {})", hosting_line(host), away.join(", ")),
                }),
                _ => guest.as_ref().map(|seat| joined_line(&seat.lobby.members)),
            };
            menu.draw(settings, miniquad::date::now(), hosting.as_deref());
        }
        //the relic choice has the screen to itself
        if let (Some(banner), None, None, None) = (&banners.shown, &players.list[me].choice, &victory, &pause) {
            draw_banner(banner);
        }
        if let Some(open) = &players.list[me].choice {
            draw_relic_choice(open, &players.list[me].upgrades, players.list[me].rerolls);
        }
        if let Some(age) = victory {
            draw_victory(age, run, score);
        }
        if fallen_for > 0.0 {
            let fade = (fallen_for / FALL_SECONDS).min(1.0);
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.85 * fade));
            if fallen_for >= FALL_SECONDS {
                return Ending::Fallen(run.clone());
            }
        }
        next_frame().await;
    }
}

//the wait for the host to say enough of the world to show it, which is the first message it sends a player. Esc gives up
async fn await_world(seat: &mut Guest, sim: &mut Sim) -> Result<(), String> {
    let mut time = 0.0;
    loop {
        quit_if_asked();
        seat.hear(sim)?;
        if seat.ready() {
            return Ok(());
        }
        if is_key_pressed(KeyCode::Escape) {
            return Err("you left the game".to_owned());
        }
        if time > WORLD_WAIT_SECONDS {
            return Err("the host never sent the world".to_owned());
        }
        draw_backdrop(time);
        let (w, h) = (screen_width(), screen_height());
        let line = format!("JOINING THE HOST'S WORLD{}", ".".repeat(1 + (time * 2.0) as usize % 3));
        draw_label_centered(&line, w / 2.0, h * 0.5, fit_size(&line, (h * 0.04).round(), w * 0.9, Face::Bold), CYAN, Face::Bold);
        draw_label_centered("Esc to leave", w / 2.0, h * 0.5 + h * 0.06, (h * 0.026).round(), MUTED, Face::Body);
        time += get_frame_time();
        next_frame().await;
    }
}
