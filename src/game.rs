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

//how a run came to an end
pub(crate) enum Ending {
    Fallen(Run),      //the army is gone
    Reload(Snapshot), //the player went back to their quick save
    Title,            //the player left for the title screen, the run kept as the game's own save
}

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
    }
}

//one run of the game, from the starting house (or from a save) until the army is gone
pub(crate) async fn play(sounds: &mut Sounds, saves: &Saves, settings: &mut Settings, start: Option<Snapshot>) -> Ending {
    //without a seed every run gets the same random numbers
    rand::srand(miniquad::date::now() as u64);
    let Snapshot { player: start_at, mut zoom, mut run, mut rerolls, mut upgrades, wall_kind, wall_points, mut director, mut units, mut babies, mut walls, mut sites, mut enemies, mut choice, victory: was_victory, hordes: saved_hordes } =
        start.unwrap_or_else(new_run);
    //the most units the army can have is the player's to choose, in the settings, so it is theirs and not the run's
    upgrades.army_limit = settings.army_limit;
    let (mut x, mut y) = (start_at.x, start_at.y);
    let speed = 800.0; //pixels per second
    let mut frame_time = 1.0 / 60.0; //smoothed so the number on the HUD can be read
    let mut player_velocity = Vec2::ZERO;
    let mut particles = Vec::new();
    let mut mason = Mason::new();
    mason.selected = wall_kind;
    mason.points = wall_points;
    let mut shots = Vec::new();
    let mut hazards: Vec<Hazard> = Vec::new();
    let mut pulls: Vec<Pull> = Vec::new();
    let mut squads = Squads::new();
    let mut fx = Effects::default();
    //(`choice` is the relics on offer, the world stands still while they are, and comes from the save along with the rest)
    let mut victory: Option<f32> = was_victory.then_some(0.0); //how long the victory screen has been up, the world stands still while it is
    let mut intro: Option<Intro> = None; //a boss's entrance
    //the family of the last boss that had an entrance, which one already in the world when a save is loaded has had
    let mut shown_boss: Option<u32> = current_boss(&enemies).and_then(|boss| boss.boss.as_ref().map(|state| state.family));
    let mut stillness = Stillness::default();
    let mut salvo_wait = 0.0; //seconds until an opening salvo can go off again
    let mut walls_dirty = false; //walls went up or came down in the middle of the last frame
    let mut milestone = next_milestone(units.len(), upgrades.army_limit);
    let mut milestone_limit = upgrades.army_limit; //the limit it was worked out for
    let mut banners = Banners::new();
    let mut fallen_for = 0.0; //how long there has been no army and nothing to make one from
    let mut map_view = MapView::Near;
    let mut autosave_in = AUTOSAVE_SECONDS; //seconds until the game saves by itself
    let mut combo = Combo::default();
    let mut ground: Vec<Mark> = Vec::new(); //scorch marks and what is left of enemies, which fade
    let mut shards: Vec<Shard> = Vec::new(); //pieces of the enemies that have just been destroyed
    let mut heartbeat_in = 0.0; //seconds until the next beat while the army is failing
    let mut beat = 0.0; //1 as the heart beats, fading
    let mut pause: Option<Pause> = None; //the menu, while it is up
    let mut hordes = saved_hordes;
    let mut facing = Vec2::new(0.0, -1.0); //the way the army's formation is turned
    //the army and the enemies follow routes to the player that are built a bit each frame
    let mut routes = Routes::new(Vec2::new(x, y), &walls);
    //the run as it is now, to be kept
    macro_rules! capture {
        () => {
            Snapshot::capture(Vec2::new(x, y), zoom, &run, rerolls, &upgrades, &mason, &director, &units, &babies, &walls, &sites, &enemies, choice.as_ref(), victory.is_some(), &hordes)
        };
    }

    loop {
        upgrades.army_limit = settings.army_limit;
        let real_dt = get_frame_time().min(MAX_FRAME_TIME);
        frame_time += (get_frame_time() - frame_time) * 0.05;
        //the game is saved when the window is closed, when the player asks (F5), and by itself now and then and each time a
        //relic is taken. F9 goes back to what F5 saved. none of it while the army is falling, as there would be nothing to go
        //back to
        let quitting = is_quit_requested();
        let by_hand = is_key_pressed(KeyCode::F5);
        autosave_in -= real_dt;
        if fallen_for == 0.0 && (quitting || by_hand || autosave_in <= 0.0) {
            let snapshot = capture!();
            if quitting || by_hand {
                let slot = if quitting { Slot::Auto } else { Slot::Quick };
                match saves.write(slot, &snapshot.to_text()) {
                    Ok(()) if !quitting => {
                        banners.push("GAME SAVED".to_owned(), CYAN);
                        fx.cue(Cue::Pop);
                    }
                    Ok(()) => {}
                    Err(error) => {
                        eprintln!("could not save the game to {}: {error}", saves.path(slot).display());
                        banners.push("COULD NOT SAVE THE GAME".to_owned(), ALERT);
                    }
                }
            } else {
                saves.write_in_background(Slot::Auto, snapshot);
            }
            autosave_in = AUTOSAVE_SECONDS;
        }
        if quitting {
            std::process::exit(0);
        }
        if is_key_pressed(KeyCode::F9) {
            match saves.read(Slot::Quick) {
                Ok(snapshot) => return Ending::Reload(snapshot),
                Err(error) => {
                    eprintln!("could not load the quick save: {error}");
                    let problem = if saves.path(Slot::Quick).exists() { "THE QUICK SAVE CAN'T BE READ" } else { "NO QUICK SAVE YET: F5 MAKES ONE" };
                    banners.push(problem.to_owned(), ALERT);
                }
            }
        }
        //the world stands still while a relic is being chosen or the victory is shown, crawls while a boss makes its entrance,
        //and freezes for a moment when one falls
        let paused = choice.is_some() || victory.is_some() || pause.is_some();
        if is_key_pressed(KeyCode::F11) {
            settings.fullscreen = !settings.fullscreen;
            miniquad::window::set_fullscreen(settings.fullscreen);
            saves.save_settings(settings);
        }
        //(Esc opens the menu on a frame the world is running, so the key that opens it is not also taken to close it)
        if !paused && fallen_for == 0.0 && is_key_pressed(KeyCode::Escape) {
            pause = Some(Pause::new(saves.info(Slot::Quick), Vec2::from(mouse_position())));
        }
        if let Some(menu) = pause.as_mut().filter(|_| paused) {
            let input = MenuInput::read(&mut menu.last_mouse);
            let was_fullscreen = settings.fullscreen;
            match menu.update(settings, &input, Vec2::new(screen_width(), screen_height()), real_dt) {
                PauseEvent::Nothing => {}
                PauseEvent::Resume => pause = None,
                PauseEvent::Save => {
                    let snapshot = capture!();
                    match saves.write(Slot::Quick, &snapshot.to_text()) {
                        Ok(()) => {
                            menu.tell("game saved", CYAN);
                            fx.cue(Cue::Pop);
                            menu.saved = saves.info(Slot::Quick);
                        }
                        Err(error) => {
                            eprintln!("could not save the game to {}: {error}", saves.path(Slot::Quick).display());
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
                    let snapshot = capture!();
                    if let Err(error) = saves.write(Slot::Auto, &snapshot.to_text()) {
                        eprintln!("could not save the game to {}: {error}", saves.path(Slot::Auto).display());
                    }
                    saves.save_settings(settings);
                    if leaving == PauseEvent::Quit {
                        std::process::exit(0);
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
        let entrance = intro.is_some() && !paused;
        if !paused {
            stillness.tick(real_dt);
        }
        let pace = if paused {
            0.0
        } else if entrance {
            INTRO_PACE
        } else {
            stillness.pace()
        };
        let dt = real_dt * pace;
        //the player is held still for the entrance, and can skip it
        let player_dt = if entrance { 0.0 } else { dt };
        run.seconds += dt;
        if let Some(shown) = intro.as_mut() {
            shown.age += real_dt;
        }
        if intro.as_ref().is_some_and(|shown| shown.over()) || (entrance && is_key_pressed(KeyCode::Space)) {
            intro = None;
        }
        if !entrance {
            handle_zoom(&mut zoom);
        }
        //the bigger the army the faster everyone is, and relics speed up the player and the army but not the enemies
        let enemy_boost = speed_multiplier(units.len());
        //champions can add to what the relics do: faster, babies from further, more babies dropped
        let perks = Perks::of(&units);
        let perked = perks.applied(&upgrades);
        let boost = perked.army_boost(units.len());
        //a left click on an enemy sends the closest units at it, and doesn't build a wall
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
        let map = Minimap::new(screen, map_view, Vec2::new(x, y));
        let over_map = map.contains(Vec2::from(mouse_position()));
        if let Some(open) = choice.as_mut() {
            open.age += get_frame_time();
            if open.age >= RELIC_LOCKOUT && rerolls > 0 && is_key_pressed(KeyCode::R) {
                rerolls -= 1;
                open.reroll();
                fx.cue(Cue::Pop);
            }
            let key = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3].iter().position(|&key| is_key_pressed(key));
            let click = if is_mouse_button_pressed(MouseButton::Left) { relic_card_at(screen, Vec2::from(mouse_position())) } else { None };
            if let Some(picked) = key.or(click).filter(|_| open.age >= RELIC_LOCKOUT) {
                let relic = open.options[picked];
                upgrades.take(relic);
                if relic == Relic::Muster {
                    spread_babies(&mut babies, Vec2::new(x, y), (units.len() / 8).max(10), units.len(), upgrades.unit_cap(), TILE_SIZE * 3.0);
                }
                fx.cue(Cue::Relic);
                fx.add_punch(0.3);
                autosave_in = 0.0;
                fx.popup(Vec2::new(x, y), RADIUS * 2.0, 0.0, format!("{}!", relic.name()), relic.color());
                choice = None;
            }
        }
        if let Some(age) = victory.as_mut() {
            *age += get_frame_time();
            if *age >= VICTORY_LOCKOUT && get_last_key_pressed().is_some() {
                //the run goes on, with a rare relic for having won
                victory = None;
                choice = Some(RelicChoice::rare());
            }
        }
        let acting = !paused && !entrance;
        if acting && is_key_pressed(KeyCode::F) {
            settings.formation = settings.formation.next();
            saves.save_settings(settings);
            fx.cue(Cue::Pop);
            fx.popup(Vec2::new(x, y), RADIUS * 3.0, 0.0, format!("{} FORMATION", settings.formation.name().to_uppercase()), CYAN);
        }
        if acting && is_key_pressed(KeyCode::R) {
            recall_squads(&mut units, &mut squads, Vec2::new(x, y), &mut fx);
        }
        let clicked_enemy = acting && click_on_enemy(x, y, zoom, &map, &enemies, &mut units, &mut squads, &mut fx);
        //what hit the walls last frame is spent, then the player builds
        let mut walls_changed = update_walls(&mut walls, &mut units, dt, &mut fx);
        walls_changed |= std::mem::take(&mut walls_dirty);
        if acting && !over_map && is_key_pressed(KeyCode::G) {
            if let Some(tile) = hovered_tile(x, y, zoom) {
                man_tower(&mut walls, &mut units, tile, &mut fx);
            }
        }
        let army = units.len();
        walls_changed |= acting && edit_walls(&mut mason, &mut walls, &mut units, &babies, Vec2::new(x, y), zoom, clicked_enemy, over_map, dt, army, &upgrades, &mut fx);
        let player_was = Vec2::new(x, y);
        handle_input(&mut x, &mut y, speed * boost, player_dt, &walls);
        //a gravity well drags the army, and the player less
        let dragged = update_pulls(&mut pulls, &mut units, Vec2::new(x, y), &walls, dt);
        (x, y) = (dragged.x, dragged.y);
        let player = Vec2::new(x, y);
        player_velocity = smoothed_velocity(player_velocity, player_was, player, player_dt);
        facing = turn_to_face(facing, player_velocity, player_dt);
        let drill = Drill { formation: settings.formation, facing };
        //the enemies are made for an army this big: the player's, with the bosses they have slain and the difficulty chosen
        let pressure = ((enemy_pressure(units.len(), run.bosses) as f32 * settings.difficulty.pressure()) as usize).max(1);
        if !paused {
            let boss_busy = current_boss(&enemies).is_some();
            //a horde is announced, and then it is there
            match hordes.update(&mut enemies, player, units.len(), pressure, boss_busy, dt) {
                Some(HordeBeat::Warned { name, from }) => {
                    fx.announce(format!("{} FROM THE {from}", name.to_uppercase()), Color::new(1.0, 0.6, 0.2, 1.0));
                    fx.cue(Cue::Warning);
                }
                Some(HordeBeat::Arrived { name, count }) => {
                    fx.popup(player, RADIUS * 5.0, 60.0, format!("{} ({count})", name.to_uppercase()), Color::new(1.0, 0.6, 0.2, 1.0));
                    fx.add_shake(0.3);
                    fx.cue(Cue::PhaseShift);
                }
                Some(HordeBeat::Broken { name, tier }) => {
                    fx.announce(format!("{} BROKEN: A {} CHEST LIES WHERE IT FELL", name.to_uppercase(), tier_name(tier)), tier_color(tier));
                    fx.cue(Cue::Milestone);
                    fx.add_punch(0.25);
                }
                None => {}
            }
            //a chest the army has got to is opened (the world stands still for relics, so they wait if a choice is up already)
            if choice.is_none() {
                for chest in take_reached(&mut hordes.chests, player, &units) {
                    if choice.is_some() {
                        hordes.chests.push(chest);
                    } else if open_chest(&chest, units.len(), upgrades.unit_cap(), &mut babies, &mut fx) {
                        choice = Some(RelicChoice::from_chest(chest.tier));
                        autosave_in = 0.0;
                    }
                }
            }
            let news = update_sites(&mut sites, &mut enemies, &mut walls, &mut babies, &mut units, player, dt, boss_busy, upgrades.unit_cap(), run.citadels as u32, &mut fx);
            walls_changed |= news.walls_changed;
            run.nests += news.nests;
            run.villages += news.villages;
            if news.citadel {
                run.bosses += 1;
                run.citadels += 1;
                victory = Some(0.0);
                stillness = Stillness::after_a_boss();
                hazards.clear();
                shots.clear();
                pulls.clear();
                fx.cue(Cue::Victory);
                fx.add_punch(1.0);
                fx.announce("THE CITADEL HAS FALLEN".to_owned(), Color::new(1.0, 0.85, 0.3, 1.0));
                fx.recruits.push((player, Odds::Citadel));
            }
        }
        //the routes only change when the player moves to another grid square or a wall changes
        routes.update(player, &walls, walls_changed);
        //a squad whose units have all died is a sad moment
        for lost_at in update_squads(&mut squads, &enemies, &mut units, &walls, walls_changed) {
            fx.cue(Cue::SquadLost);
            fx.popup(lost_at, RADIUS * 3.0, 0.0, "SQUAD LOST".to_owned(), ALERT);
        }
        update_units_in(&mut units, x, y, dt, &walls, &routes.army, boost, &squads, &drill);
        //the champions' might and wards, auras and healing for the frame
        apply_champions(&mut units, dt);

        //the fighting: enemies walk, run at the player, dash, heal, summon or shoot, bosses wind up their attacks, the
        //shots fly and the hazards go off, everything touching an enemy hurts and is hurt, and whatever has run out of
        //health is gone
        terrain_effects(&mut enemies, &walls, dt, &mut fx);
        champion_acts(&mut units, &mut enemies, &mut babies, &perked, upgrades.unit_cap(), dt, &mut fx);
        patrol_enemies(&mut enemies, player, dt, enemy_boost, &walls, &routes.foes, &mut fx);
        chew_walls(&enemies, &walls, dt, &mut fx);
        enemy_abilities(&mut enemies, &units, player, dt, &mut fx);
        boss_act(&mut enemies, &units, player, dt, &mut shots, &mut fx);
        walls_dirty |= raise_walls(&mut walls, &mut fx.raise);
        pulls.append(&mut fx.pulls);
        shoot_enemies(&mut enemies, &units, &mut shots, dt, &mut fx);
        fire_towers(&mut walls, &mut enemies, &upgrades, dt, &mut fx);
        chew_hostile_walls(&units, &walls, &upgrades, dt, &mut fx);
        hazards.append(&mut fx.hazards);
        update_shots(&mut shots, &mut units, &walls, dt, &upgrades, &mut fx);
        update_hazards(&mut hazards, &mut units, &upgrades, dt, &mut fx);
        heal_units(&mut units, dt, &upgrades);
        let destroyed = fight(&mut enemies, &mut units, &mut babies, dt, &perked, &mut fx);
        run.destroyed += destroyed;
        train_champions(&mut units, std::mem::take(&mut fx.xp), &mut fx);
        if let Some((size, name)) = combo.add(destroyed) {
            fx.cue(Cue::Combo);
            fx.add_punch(0.25);
            fx.popup(player, RADIUS * 4.0, 30.0, format!("{name}!  x{size}"), TROPHY);
        }
        combo.update(dt);
        hazards.append(&mut fx.hazards);
        tick_enemy_numbers(&mut enemies, dt, &mut fx);
        run.lost += bury_units(&mut units, &mut fx);
        martyr_blasts(&mut enemies, &std::mem::take(&mut fx.fallen), &upgrades);
        opening_salvo(&mut enemies, player, &mut salvo_wait, dt, &upgrades, &mut fx);
        heal_everyone(&mut units, std::mem::take(&mut fx.heal_all));
        mason.earn(std::mem::take(&mut fx.points) * upgrades.quarry);
        mason.earn(perks.points * dt);
        respawn_enemies(&mut enemies, x, y, pressure);
        let pickup_range = baby_pickup_range(units.len(), Vec2::new(screen_width(), screen_height())) * perked.pickup;
        grow_babies(&mut babies, &mut units, x, y, dt, pickup_range, upgrades.unit_cap(), &mut fx);

        //bosses come and go
        if !paused {
            match director.update(&mut enemies, player, units.len(), dt) {
                Some(Beat::Warning) => {
                    fx.announce(format!("{} APPROACHES", director.next_def().name.to_uppercase()), Color::new(1.0, 0.6, 0.2, 1.0));
                    fx.cue(Cue::Warning);
                }
                Some(Beat::Arrived(name)) => {
                    fx.announce(name.to_uppercase(), Color::new(1.0, 0.3, 0.25, 1.0));
                    fx.add_shake(0.6);
                    fx.add_punch(0.6);
                    fx.cue(Cue::PhaseShift);
                }
                Some(Beat::Slain { name, tier }) => {
                    run.bosses += 1;
                    //whatever it had thrown is gone with it, the babies it dropped shouldn't grow up into its fire
                    hazards.clear();
                    shots.clear();
                    pulls.clear();
                    fx.announce(format!("{} IS SLAIN", name.to_uppercase()), Color::new(1.0, 0.85, 0.3, 1.0));
                    fx.cue(Cue::Milestone);
                    stillness = Stillness::after_a_boss();
                    fx.add_punch(1.0);
                    //another reroll for every third boss
                    if run.bosses % 3 == 0 {
                        rerolls += 1;
                    }
                    choice = Some(RelicChoice::new());
                    if boss_drops_champion(run.bosses, rand::gen_range(0.0, 1.0)) {
                        fx.recruits.push((player, Odds::Boss(tier)));
                    }
                }
                None => {}
            }
            //a boss that has just turned up is shown off, once
            if let Some(boss) = current_boss(&enemies) {
                let family = boss.boss.as_ref().map(|state| state.family);
                if family != shown_boss {
                    shown_boss = family;
                    intro = Some(Intro::new(boss));
                }
            }
        }

        award_recruits(&mut units, &mut babies, upgrades.unit_cap(), &mut fx);

        //round army sizes get a message, once. the army being gone is the end, but not while there are babies to be picked up
        run.peak_army = run.peak_army.max(units.len());
        if upgrades.army_limit != milestone_limit {
            //the limit was changed in the settings: the sizes ahead are worked out again
            milestone_limit = upgrades.army_limit;
            milestone = next_milestone(units.len(), milestone_limit);
        }
        while units.len() >= milestone {
            let text = if milestone == upgrades.army_limit { format!("FULL STRENGTH: {milestone} UNITS") } else { format!("ARMY OF {milestone}!") };
            banners.push(text, Color::new(1.0, 0.9, 0.4, 1.0));
            fx.cue(Cue::Milestone);
            fx.add_shake(0.25);
            fx.add_punch(0.5);
            milestone = next_milestone(milestone, upgrades.army_limit);
        }
        for (text, color) in fx.announcements.drain(..) {
            banners.push(text, color);
        }
        banners.update(real_dt);
        if units.is_empty() && babies.is_empty() {
            if fallen_for == 0.0 {
                fx.cue(Cue::Defeat);
                fx.add_shake(0.8);
            }
            fallen_for += dt;
        } else {
            fallen_for = 0.0;
        }

        //a failing army has a heart that beats faster the closer it is to the end
        let in_peril = peril(units.len(), run.peak_army);
        beat = (beat - real_dt * 3.0f32).max(0.0);
        if in_peril > 0.0 && !paused {
            heartbeat_in -= real_dt;
            if heartbeat_in <= 0.0 {
                fx.cue(Cue::Heartbeat);
                beat = 1.0;
                heartbeat_in = 1.15 - 0.65 * in_peril;
            }
        } else {
            heartbeat_in = 0.3;
        }
        fx.update(dt, player);
        fx.spawn_bursts(&mut particles);
        update_marks(&mut ground, &mut fx.marks, dt);
        throw_shards(&mut shards, &mut fx.shards);
        update_shards(&mut shards, dt);
        sounds.play(&fx.take_cues(), player, get_time());
        //the music follows the size of the army and how much danger there is: enemies that have noticed the player and are near
        let chasing = enemies.iter().filter(|enemy| enemy.chasing && enemy.position.distance(player) < MUSIC_DANGER_RANGE).count();
        sounds.tune_music(&Mood { army: units.len(), danger: (chasing as f32 / MUSIC_DANGER_ENEMIES).min(1.0), boss: current_boss(&enemies).is_some(), peril: in_peril }, real_dt);
        //the camera goes to the boss and back during its entrance
        let (camera_at, camera_zoom) = match &intro {
            Some(shown) if entrance => (player.lerp(shown.at, shown.ease()), zoom * (1.0 - 0.35 * shown.ease())),
            _ => (player, zoom),
        };
        //units off screen don't leave trails, nobody would see them
        let view = visible_area(camera_at.x, camera_at.y, camera_zoom);
        emit_trail(&mut particles, player, player_velocity, speed * boost, dt, BLUE);
        for unit in &units {
            let position = Vec2::new(unit.x, unit.y);
            if view.contains(position) {
                emit_trail(&mut particles, position, unit.velocity, unit.speed * boost, dt, unit_color(unit));
            }
        }
        update_particles(&mut particles, dt);

        clear_background(BLACK);
        //the camera jumps in a little for big moments
        let drawn_zoom = camera_zoom * (1.0 + PUNCH_ZOOM * fx.punch * settings.shake);
        let shaken = camera_at + shake_offset(fx.shake * settings.shake, drawn_zoom);
        let camera = follow_camera(shaken.x, shaken.y, drawn_zoom);
        set_camera(&camera);
        let drawn = visible_area(shaken.x, shaken.y, drawn_zoom);
        let hovered = if over_map || paused || entrance { None } else { hovered_tile(x, y, zoom) };
        draw_world(camera_at.x, camera_at.y, drawn_zoom, hovered);
        draw_floor(camera_at.x, camera_at.y, drawn_zoom, &walls, get_time() as f32);
        draw_formation_outline(drill.formation, player, drill.facing, units.len(), drawn_zoom);
        draw_discs(Layer::Marks,&mark_discs(&ground, drawn), &camera);
        draw_sites(&sites, view, get_time() as f32);
        draw_discs(Layer::Puffs, &particle_discs(&particles), &camera);
        draw_enemies(&enemies, &squads, get_time() as f32, enemy_boost);
        draw_shards(&shards, drawn);
        draw_hazards(&hazards, get_time() as f32);
        draw_pulls(&pulls, get_time() as f32);
        draw_chests(&hordes.chests, drawn, get_time() as f32);
        draw_discs(Layer::Army, &unit_discs(&units, drawn), &camera);
        draw_discs(Layer::Babies, &baby_discs(&babies, drawn), &camera);
        draw_champions(&units, drawn, drawn_zoom, get_time() as f32);
        draw_shots(&shots);
        draw_discs(Layer::Player, &[Disc::new(x, y, RADIUS, BLUE).edged(UNIT_BORDER)], &camera);
        //the walls go on top of everything in the world, the numbers and the screen over them are for reading
        draw_walls(camera_at.x, camera_at.y, drawn_zoom, hovered, &walls, if paused || entrance { None } else { Some(mason.selected) }, get_time() as f32);
        draw_beams(&fx.beams);
        draw_rings(&fx.rings);
        draw_popups(&fx.popups, drawn_zoom);
        set_default_camera();
        draw_vignette(in_peril, beat);
        draw_roster(&units, if pause.is_some() { Vec2::splat(-1000.0) } else { Vec2::from(mouse_position()) });
        draw_hud(x, y, frame_time, units.len(), &run, boost, pickup_range, sounds.muted, sounds.music.on, settings.formation, &director, &sites, upgrades.unit_cap(), hordes.status(&enemies));
        if settings.hints && !paused && run.seconds < HINT_SECONDS {
            draw_hints(run.seconds);
        }
        if let Some(warning) = blast_warning(&hazards, Vec2::new(x, y), &units) {
            draw_blast_warning(&warning, get_time() as f32);
        }
        draw_build_bar(&mason, get_time() as f32);
        if choice.is_none() {
            draw_combo(&combo);
        }
        //the player has moved since the map was made, so make it again to draw
        let map = Minimap::new(screen, map_view, player);
        let mouse = Vec2::from(mouse_position());
        if !paused && !entrance {
            draw_relic_bar(&upgrades, mouse);
        }
        //the enemy under the mouse, in the world or as a dot on the map
        let under_mouse = if paused || entrance {
            None
        } else if over_map {
            minimap_enemy_at(&map, mouse, &enemies)
        } else {
            enemy_at(mouse_world(x, y, zoom), &enemies, ENEMY_CLICK_MARGIN / zoom)
        };
        draw_minimap(&map, map_view, player, view, if over_map { under_mouse } else { None }, &units, &enemies, &babies, &squads, &walls, &hazards, &sites, get_time() as f32);
        if let Some(coming) = &hordes.coming {
            draw_horde_ping(&map, coming.at, get_time() as f32);
        }
        draw_chest_marks(&map, &hordes.chests, get_time() as f32);
        if let Some(enemy) = under_mouse.map(|i| &enemies[i]) {
            draw_enemy_tooltip(enemy, units.iter().filter(|unit| unit.target.is_none()).count(), map.rect);
        }
        if let Some(at) = hordes.heading(&enemies, player) {
            draw_horde_arrow(at, camera_at, camera_zoom, hordes.coming.map(|coming| coming.left));
        }
        draw_chest_arrow(&hordes.chests, player, camera_at, camera_zoom);
        if let Some(boss) = current_boss(&enemies) {
            let alive = enemies.iter().filter(|other| other.boss.as_ref().zip(boss.boss.as_ref()).is_some_and(|(a, b)| a.family == b.family)).count();
            draw_boss_bar(boss, family_health(&enemies, boss), alive);
            draw_boss_arrow(boss, camera_at, camera_zoom);
        }
        if let Some(shown) = intro.as_ref().filter(|_| entrance) {
            draw_intro(shown);
        }
        if let Some(menu) = &pause {
            menu.draw(settings, miniquad::date::now());
        }
        //the relic choice has the screen to itself
        if let (Some(banner), None, None, None) = (&banners.shown, &choice, &victory, &pause) {
            draw_banner(banner);
        }
        if let Some(open) = &choice {
            draw_relic_choice(open, &upgrades, rerolls);
        }
        if let Some(age) = victory {
            draw_victory(age, &run);
        }
        if fallen_for > 0.0 {
            let fade = (fallen_for / FALL_SECONDS).min(1.0);
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.85 * fade));
            if fallen_for >= FALL_SECONDS {
                return Ending::Fallen(run);
            }
        }
        next_frame().await;
    }
}
