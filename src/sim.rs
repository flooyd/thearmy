//! the world and its step: everything that is the same for every player, and nothing that is only for one of them to see. what goes in is the
//! players' input (see `PlayerInput`) and how long the step is, and what comes out is the events that were worth showing (see `Event`) and a report
//! of what the game itself has to know. drawing, sounds, menus, saves and the camera are all outside it, so a machine can step the world without a
//! window, and one that only watches the world needs nothing of it but the events

use crate::prelude::*;

pub(crate) const PLAYER_SPEED: f32 = 800.0; //pixels per second, before the army's boost

//what is chosen for the whole world, outside it: how many units an army can have and how hard the enemies are
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Rules {
    pub(crate) army_limit: usize,
    pub(crate) difficulty: Difficulty,
}

impl Rules {
    pub(crate) fn of(settings: &Settings) -> Rules {
        Rules { army_limit: settings.army_limit, difficulty: settings.difficulty }
    }
}

//how long a step is. the world runs for `dt`, which is the time the frame took unless something has slowed it or stopped it, the players for
//`player_dt` (shorter when they are held still), and a relic that has been on offer grows older by `real_dt`, whatever the world is doing.
//`stands_still` is for a world that waits (for a relic to be chosen, say): the bosses, hordes and sites in it do nothing
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Pace {
    pub(crate) real_dt: f32,
    pub(crate) dt: f32,
    pub(crate) player_dt: f32,
    pub(crate) stands_still: bool,
}

impl Pace {
    //every clock running at the same rate (for the tests: the game works the clocks out for itself)
    #[cfg(test)]
    pub(crate) fn steady(dt: f32) -> Pace {
        Pace { real_dt: dt, dt, player_dt: dt, stands_still: false }
    }
}

//what the game has to know of a step, apart from the events
#[derive(Default, Debug)]
pub(crate) struct Report {
    pub(crate) boosts: Vec<f32>,        //how many times faster than normal each player and their army were
    pub(crate) pickup_ranges: Vec<f32>, //how far each player reached for babies
    pub(crate) destroyed: usize,        //enemies destroyed
    pub(crate) boss_slain: bool,
    pub(crate) citadel: bool,           //the citadel fell
    pub(crate) save_soon: bool,         //a relic was taken or a chest opened: a good moment to save
    pub(crate) hit_stop: f32,           //seconds a world of one could stand still for a champion's crit
    pub(crate) difficulty: Difficulty,  //what the world is played on (the host's, for a guest)
}

pub(crate) struct Step {
    pub(crate) events: Vec<Event>,
    pub(crate) report: Report,
}

pub(crate) struct Sim {
    pub(crate) players: Players,
    pub(crate) run: Run,
    pub(crate) director: Director,
    pub(crate) hordes: Hordes,
    pub(crate) units: Vec<Unit>,
    pub(crate) babies: Vec<Baby>,
    pub(crate) walls: Walls,
    pub(crate) sites: Vec<Site>,
    pub(crate) patches: Vec<Patch>, //the ice, lava and swamp, laid out from the sites (see biomes.rs)
    pub(crate) enemies: Vec<Enemy>,
    pub(crate) shots: Vec<Shot>,
    pub(crate) hazards: Vec<Hazard>,
    pub(crate) pulls: Vec<Pull>,
    pub(crate) bolts: Vec<Bolt>, //the champions' bolts in flight
    fx: Effects,
    walls_dirty: bool, //walls went up or came down in the middle of the last step
}

impl Sim {
    //the world as a run was left, with the one player in it. the rules are applied to them at once
    pub(crate) fn from_snapshot(snapshot: Snapshot, rules: Rules) -> Sim {
        let Snapshot { player: start_at, run, rerolls, upgrades, wall_kind, wall_points, director, units, babies, walls, sites, enemies, choice, hordes, company, .. } = snapshot;
        let mut players = Players::solo(Player::new(0, "Player", start_at, &walls));
        {
            let player = &mut players.list[0];
            player.upgrades = upgrades;
            player.rerolls = rerolls;
            player.choice = choice;
            player.mason.selected = wall_kind;
            player.mason.points = wall_points;
        }
        //(the other players of a game that was saved with others in it are all away, with their armies held for them to come back to)
        for mate in company.into_iter().take(MAX_PLAYERS - 1) {
            let mut other = Player::new(players.len(), &mate.name, mate.position, &walls);
            other.upgrades = mate.upgrades;
            other.rerolls = mate.rerolls;
            other.choice = mate.choice;
            other.mason.selected = mate.wall_kind;
            other.mason.points = mate.wall_points;
            players.list.push(other);
        }
        players.apply_settings(rules.army_limit, rules.difficulty.relic_answer());
        let count = players.len();
        let patches = lay_patches(&sites);
        let mut sim = Sim { players, run, director, hordes, units, babies, walls, sites, patches, enemies, shots: Vec::new(), hazards: Vec::new(), pulls: Vec::new(), bolts: Vec::new(), fx: Effects::default(), walls_dirty: false };
        for player in 1..count {
            sim.hold(player);
        }
        sim
    }

    //a world with nothing in it, to be filled in from what a host says of its own (see `Replica`): it is only ever looked at, never stepped
    pub(crate) fn mirror() -> Sim {
        let run = Run { destroyed: 0, lost: 0, peak_army: 0, bosses: 0, nests: 0, villages: 0, seconds: 0.0, citadels: 0, killers: Vec::new() };
        Sim { players: Players { list: Vec::new() }, run, director: Director::new(), hordes: Hordes::new(), units: Vec::new(), babies: Vec::new(), walls: Walls::default(), sites: Vec::new(), patches: Vec::new(), enemies: Vec::new(), shots: Vec::new(), hazards: Vec::new(), pulls: Vec::new(), bolts: Vec::new(), fx: Effects::default(), walls_dirty: false }
    }

    //a player joins the world as it is being played: at the starting house, with the few units a run starts with. returns where they are in the list of
    //players, or None if the world has all the players it can hold, or someone has the name already
    pub(crate) fn add_player(&mut self, name: &str) -> Option<usize> {
        let slot = self.players.len();
        let middle = Vec2::splat(WORLD_SIZE / 2.0);
        let at = middle + Vec2::new((slot as f32 - 1.5) * 120.0, 140.0);
        let player = self.players.join(name, at, &self.walls)?;
        self.units.extend(starting_units(at.x, at.y).into_iter().map(|unit| unit.owned_by(player)));
        Some(player)
    }

    //the host's name, which a player of a saved game who is called the same is made unlike (the host is the one whose game it is)
    pub(crate) fn name_host(&mut self, name: &str) {
        let name = clean_name(name);
        self.players.list[0].name = name.clone();
        for other in self.players.list.iter_mut().skip(1).filter(|other| other.name.eq_ignore_ascii_case(&name)) {
            let room: String = other.name.chars().take(MAX_NAME_LENGTH - 2).collect();
            other.name = format!("{} 2", room.trim_end());
        }
    }

    //a player has gone: their army is held where it stood, out of the world's reach, for them to have back. the enemies find nothing of theirs to fight, and
    //nothing can hurt what is held. (the units in a tower stay in it, which is as safe a place as any.) the first player is the host, who is the game
    pub(crate) fn hold(&mut self, player: usize) {
        if player == 0 || player >= self.players.len() || self.players.list[player].is_away() {
            return;
        }
        let (mut held, kept): (Vec<Unit>, Vec<Unit>) = std::mem::take(&mut self.units).into_iter().partition(|unit| unit.owner as usize == player && unit.garrison.is_none());
        self.units = kept;
        for unit in &mut held {
            unit.target = None;
        }
        let away = &mut self.players.list[player];
        away.squads = Squads::new();
        away.held = Some(held);
    }

    //a player is back: the army that was held is where it was
    pub(crate) fn release(&mut self, player: usize) {
        if let Some(held) = self.players.list.get_mut(player).and_then(|away| away.held.take()) {
            self.units.extend(held);
        }
    }

    //whatever has come to a player who is away since they went (a champion sent to them, a baby that grew) is held with the rest
    fn tuck_strays(players: &mut Players, units: &mut Vec<Unit>) {
        if !players.list.iter().any(Player::is_away) {
            return;
        }
        let last = players.len() - 1;
        let away = |players: &Players, unit: &Unit| unit.garrison.is_none() && players.list[(unit.owner as usize).min(last)].is_away();
        if !units.iter().any(|unit| away(players, unit)) {
            return;
        }
        let mut strays = Vec::new();
        units.retain(|unit| {
            let stray = away(players, unit);
            if stray {
                strays.push(unit.clone());
            }
            !stray
        });
        for stray in strays {
            if let Some(held) = players.list[(stray.owner as usize).min(last)].held.as_mut() {
                held.push(stray);
            }
        }
    }

    //a player whose army is gone has a few units again, round their avatar
    pub(crate) fn revive(&mut self, player: usize) {
        let at = self.players.list[player].position;
        self.units.extend(starting_units(at.x, at.y).into_iter().map(|unit| unit.owned_by(player)));
    }

    //the world as it is now, to be kept: `me` is the player the save is of, and `zoom` and `victory` are what their screen was showing
    pub(crate) fn capture(&self, me: usize, zoom: f32, victory: bool) -> Snapshot {
        let player = &self.players.list[me];
        let mut snapshot = Snapshot::capture(player.position, zoom, &self.run, player.rerolls, &player.upgrades, &player.mason, &self.director, &self.units, &self.babies, &self.walls, &self.sites, &self.enemies, player.choice.as_ref(), victory, &self.hordes);
        //a world with others in it is the host's (the first player's) to keep: the others are with it, and the armies that are held with the rest
        if self.players.len() > 1 {
            for other in &self.players.list {
                snapshot.units.extend(other.held.iter().flatten().cloned());
            }
            snapshot.units.sort_by_key(|unit| unit.owner);
            snapshot.company = self
                .players
                .list
                .iter()
                .skip(1)
                .map(|other| Companion { name: other.name.clone(), position: other.position, rerolls: other.rerolls, upgrades: other.upgrades.clone(), wall_kind: other.mason.selected, wall_points: other.mason.points, choice: other.choice.clone() })
                .collect();
        }
        snapshot
    }

    //the units put in order of owner, and where each army is
    pub(crate) fn arrange(&mut self) -> Vec<std::ops::Range<usize>> {
        arrange(&mut self.units, self.players.len())
    }

    //what the world has made for the eyes and ears and not yet reported, which is nothing after a step
    #[cfg(test)]
    pub(crate) fn effects(&self) -> &Effects {
        &self.fx
    }

    //one step of the world. `inputs` are the players' in the order of the players: a player without any stands still, and an input is only
    //believed as far as it could have been asked for (see `PlayerInput::tidy`), and not at all for a player who is choosing a relic, who can do
    //nothing else until they have
    pub(crate) fn step(&mut self, inputs: Vec<PlayerInput>, pace: Pace, rules: Rules) -> Step {
        let Sim { players, run, director, hordes, units, babies, walls, sites, patches, enemies, shots, hazards, pulls, bolts, fx, walls_dirty } = self;
        let Pace { real_dt, dt, player_dt, stands_still } = pace;
        play_on(rules.difficulty);
        players.apply_settings(rules.army_limit, rules.difficulty.relic_answer());
        let count = players.len();
        let mut report = Report { difficulty: rules.difficulty, ..Report::default() };
        run.seconds += dt;

        let mut inputs: Vec<PlayerInput> = inputs.into_iter().take(count).map(PlayerInput::tidy).collect();
        while inputs.len() < count {
            inputs.push(PlayerInput::idle(players.list[inputs.len()].formation, DEFAULT_SCREEN));
        }
        for (player, input) in players.list.iter().zip(inputs.iter_mut()) {
            if player.choice.is_some() {
                *input = PlayerInput { relic: input.relic, ..PlayerInput::idle(input.formation, input.screen) };
            }
        }
        //the armies are kept in order of owner. champions can add to what the relics do: faster, babies from further, more babies dropped
        let armies = Armies::of(units, players);
        let at: Vec<Vec2> = players.positions();

        //the relics on offer to a player grow older, and they take one or ask for others
        for (i, player) in players.list.iter_mut().enumerate() {
            let Some(open) = player.choice.as_mut() else {
                continue;
            };
            open.age += real_dt;
            if open.age < RELIC_LOCKOUT {
                continue;
            }
            match inputs[i].relic {
                Some(RelicAct::Reroll) if player.rerolls > 0 => {
                    player.rerolls -= 1;
                    open.reroll();
                    fx.cue(Cue::Pop);
                }
                Some(RelicAct::Pick(place)) => {
                    let relic = open.options[place as usize];
                    player.upgrades.take(relic);
                    if relic == Relic::Muster {
                        spread_babies(babies, at[i], (armies.size(i) / 8).max(10), armies.size(i), player.upgrades.unit_cap(), TILE_SIZE * 3.0);
                    }
                    fx.cue(Cue::Relic);
                    fx.add_punch(0.3);
                    report.save_soon = true;
                    fx.popup(at[i], RADIUS * 2.0, 0.0, format!("{}!", relic.name()), relic.color());
                    player.chosen();
                }
                _ => {}
            }
        }
        //the champions their owners chose a way to grow for grow that way (a choice that can't be made, for a champion that isn't theirs or at a
        //level it hasn't reached or has chosen at already, is nothing)
        for (i, input) in inputs.iter().enumerate() {
            let Some(pick) = input.evolve else {
                continue;
            };
            if let Some(unit) = units.iter_mut().find(|unit| owner_index(unit.owner) == i && unit.id == pick.champion) {
                if evolve(unit, pick.tier as usize, pick.choice as usize, fx) {
                    report.save_soon = true;
                }
            }
        }
        for (player, input) in players.list.iter_mut().zip(&inputs) {
            player.formation = input.formation;
        }
        //squads are called back, and sent at the enemies they were told to go for
        for (i, input) in inputs.iter().enumerate() {
            let mine = armies.ranges[i].clone();
            if input.recall {
                recall_squads(&mut units[mine.clone()], &mut players.list[i].squads, at[i], fx);
            }
            if let Some(id) = input.send {
                send_squad_at(&mut units[mine.clone()], &mut players.list[i].squads, enemies, id, fx);
            }
            //and the champions' ultimates that are ready are set off
            if input.ultimate {
                let player = &players.list[i];
                fire_ultimates(&mut units[mine], enemies, i as u8, player.facing, armies.size(i), &player.upgrades, fx);
            }
        }
        //what hit the walls last step is spent, then the players build and walk
        let mut walls_changed = update_walls(walls, units, dt, fx);
        walls_changed |= std::mem::take(walls_dirty);
        for (i, input) in inputs.iter().enumerate() {
            if let Some(tile) = input.tower.filter(|&tile| input.reaches(at[i], tile)) {
                man_tower(walls, &mut units[armies.ranges[i].clone()], tile, fx);
            }
            let select = input.select.map(|kind| BUILDABLE[kind as usize]);
            let place: Vec<(i32, i32)> = input.place.iter().copied().filter(|&tile| input.reaches(at[i], tile)).collect();
            let delete: Vec<(i32, i32)> = input.delete.iter().copied().filter(|&tile| input.reaches(at[i], tile)).collect();
            let player = &mut players.list[i];
            walls_changed |= edit_walls(&mut player.mason, walls, units, babies, at[i], select, &place, &delete, dt, armies.size(i), fx);
            let pace = ground_pace(patches, player.position);
            player.walk(input.direction, PLAYER_SPEED * armies.boosts[i] * pace, player_dt, walls);
        }
        //a gravity well drags the army, and the players less
        let mut positions = players.positions();
        update_pulls_among(pulls, units, &mut positions, walls, dt);
        for (i, (player, position)) in players.list.iter_mut().zip(positions).enumerate() {
            player.position = position;
            player.settle(at[i], player_dt);
        }
        //the enemies are made for the size of army of the fight they are in (the army of the player they are near, and the armies of the players
        //near them), with the bosses that have been slain and the difficulty chosen
        //(and with the tide: a lull, the usual or a surge, which is announced as it comes)
        let tide = tide_at(run.seconds, current_boss(enemies).is_some());
        if tide == Tide::Surge && tide_at(run.seconds - dt, current_boss(enemies).is_some()) != Tide::Surge && !stands_still {
            fx.announce("THE ENEMY SURGES".to_owned(), Color::new(1.0, 0.45, 0.2, 1.0));
            fx.cue(Cue::Warning);
        }
        let (bosses, harder) = (run.bosses, rules.difficulty.pressure() * tide.pressure());
        let pressure_of = move |team: usize| ((enemy_pressure(team, bosses) as f32 * harder) as usize).max(1);
        if !stands_still {
            let boss_busy = current_boss(enemies).is_some();
            let prey = armies.prey(players, units, pressure_of);
            //a horde is announced, and then it is there
            match hordes.update_among(enemies, &prey, boss_busy, dt) {
                Some(HordeBeat::Warned { name, from }) => {
                    fx.announce(format!("{} FROM THE {from}", name.to_uppercase()), Color::new(1.0, 0.6, 0.2, 1.0));
                    fx.cue(Cue::Warning);
                }
                Some(HordeBeat::Arrived { name, count }) => {
                    for player in &players.list {
                        fx.popup(player.position, RADIUS * 5.0, 60.0, format!("{} ({count})", name.to_uppercase()), Color::new(1.0, 0.6, 0.2, 1.0));
                    }
                    fx.add_shake(0.3);
                    fx.cue(Cue::PhaseShift);
                }
                Some(HordeBeat::Broken { name, tier }) => {
                    let hold = CHEST_HOLD_SECONDS[(tier.clamp(1, 3) - 1) as usize];
                    let hold = if hold > 0.0 { format!(": HOLD IT {hold:.0}s") } else { String::new() };
                    fx.announce(format!("{} BROKEN: A {} CHEST LIES WHERE IT FELL{hold}", name.to_uppercase(), tier_name(tier)), tier_color(tier));
                    fx.cue(Cue::Milestone);
                    fx.add_punch(0.25);
                }
                None => {}
            }
            //a chest an army has got to is opened, by the player nearest it of those whose armies are there
            let reached = take_reached_among(&mut hordes.chests, &prey, enemies, dt);
            let news = update_sites_among(sites, enemies, walls, babies, &prey, dt, boss_busy, run.citadels as u32, fx);
            for (chest, owner) in reached {
                let opener = &mut players.list[owner];
                if opener.choice.is_some() {
                    //(they are choosing from the relics they have, and it waits)
                    hordes.chests.push(chest);
                } else if open_chest(owner, &chest, armies.size(owner), opener.upgrades.unit_cap(), babies, fx) {
                    opener.offer(RelicChoice::from_chest(chest.tier));
                    report.save_soon = true;
                }
            }
            mend_at_villages(sites, units, &players.positions(), dt);
            walls_changed |= news.walls_changed;
            if news.citadel_woke {
                //(with The Last Army, the top of the Insane ladder, it comes back a round tougher again for each time it has fallen, as if a tier higher)
                let last_army = if rules.difficulty.has(Mutator::LastArmy) { run.citadels as u32 } else { 0 };
                if let Some(lord) = enemies.iter_mut().find(|enemy| enemy.boss.is_some() && enemy.site.is_some()) {
                    lord.harden(rules.difficulty.boss_rounds() + last_army);
                }
            }
            run.nests += news.nests;
            run.villages += news.villages;
            if news.citadel {
                run.bosses += 1;
                run.citadels += 1;
                hazards.clear();
                shots.clear();
                pulls.clear();
                fx.cue(Cue::Victory);
                fx.add_punch(1.0);
                fx.announce("THE CITADEL HAS FALLEN".to_owned(), Color::new(1.0, 0.85, 0.3, 1.0));
                report.citadel = true;
                players.share_citadel(fx);
            }
        }
        //the routes only change when a player moves to another grid square or a wall changes (the armies' own are kept up to date with the
        //armies' moving), and the time a step has for them is shared by the players
        let foe_budget = FIELD_BUDGET / 2 / count as u32;
        for other in players.list.iter_mut() {
            other.foes.update(other.position, walls, walls_changed, foe_budget);
        }
        //squads follow their enemies, the armies walk and the champions in them give their auras to the units near
        //(and the ground under them has its say: see biomes.rs)
        let footing = footing_of(units, patches, &players.positions());
        step_armies(players, &armies, units, enemies, walls, walls_changed, dt, fx);
        ground_the_army(units, &footing, &*players, dt);

        //the fighting: enemies walk, run at the player, dash, heal, summon or shoot, bosses wind up their attacks, the shots fly and the hazards
        //go off, everything touching an enemy hurts and is hurt, and whatever has run out of health is gone
        terrain_effects(enemies, walls, dt, fx);
        ground_effects_on_enemies(enemies, patches, dt);
        champion_acts(units, enemies, babies, &armies.perked, dt, fx);
        //(the champions' volleys go into flight, and the bolts fly, before the fighting counts what has been destroyed)
        for (volley, hurt) in &fx.volleys {
            launch_volley(bolts, volley, *hurt);
        }
        //(and the front of each army fires ahead of it)
        let fronts = fronts_of(&*players, &armies.perked);
        fire_vanguard(&fronts, units, enemies, run.seconds, dt, bolt_density(), true, bolts, Some(&mut *fx));
        fly_bolts(bolts, enemies, dt, Some(&mut *fx));
        {
            //(each enemy goes after the player nearest to it, and what it does is for that player's army)
            let prey = armies.prey(players, units, pressure_of);
            spring_ambushes_among(enemies, &prey, fx);
            rally_followers(enemies);
            patrol_enemies_among(enemies, &prey, dt, walls, fx);
            chew_walls(enemies, walls, dt, fx);
            enemy_abilities_among(enemies, &prey, dt, fx);
            boss_act_among(enemies, &prey, dt, shots, fx);
        }
        *walls_dirty |= raise_walls(walls, &mut fx.raise);
        pulls.append(&mut fx.pulls);
        shoot_enemies(enemies, units, shots, dt, fx);
        fire_towers(walls, enemies, players.archer(), dt, fx);
        chew_hostile_walls(units, walls, players, dt, fx);
        lay_hazards(hazards, &mut fx.hazards);
        update_shots(shots, units, walls, dt, players, fx);
        update_hazards(hazards, units, players, dt, fx);
        heal_armies(players, units, dt);
        let destroyed = fight(enemies, units, babies, dt, &armies.perked, fx);
        call_for_help(enemies, std::mem::take(&mut fx.alarms), fx);
        run.destroyed += destroyed;
        report.destroyed = destroyed;
        train_armies(players, units, fx);
        lay_hazards(hazards, &mut fx.hazards);
        enrage_enemies(enemies);
        tick_enemy_numbers(enemies, dt, fx);
        run.lost += bury_units(units, fx);
        tally_killers(&mut run.killers, &std::mem::take(&mut fx.banes));
        martyr_blasts(enemies, &std::mem::take(&mut fx.fallen), players);
        for other in players.list.iter_mut() {
            opening_salvo(enemies, other.position, &mut other.salvo_wait, dt, &other.upgrades, fx);
        }
        refresh_armies(players, units, fx);
        earn_wall_points(players, &armies, dt, fx);
        {
            //(the army has been added to and lost from since the step began, and the armies are where they are in the list now)
            let ranges = arrange(units, count);
            respawn_enemies_up_to(enemies, &pressures(&prey_of(players, units, &ranges, pressure_of)), (ENEMY_COUNT as f32 * tide.crowd()).round() as usize);
        }
        //an elite that has come into the world this step has its affixes chosen, more of them the harder the game
        let chance = rules.difficulty.affix_chance();
        for enemy in enemies.iter_mut().filter(|enemy| !enemy.affixes.rolled()) {
            enemy.roll_affixes(chance);
        }
        let screens: Vec<Vec2> = inputs.iter().map(|input| input.screen).collect();
        report.pickup_ranges = grow_armies(players, &armies, units, babies, &screens, dt, fx);
        report.boosts = armies.boosts.clone();

        //bosses come and go
        if !stands_still {
            let ranges = arrange(units, count);
            let prey = prey_of(players, units, &ranges, pressure_of);
            match director.update_among(enemies, &prey, dt) {
                Some(Beat::Warning) => {
                    fx.announce(format!("{} APPROACHES", director.next_def().name.to_uppercase()), Color::new(1.0, 0.6, 0.2, 1.0));
                    fx.cue(Cue::Warning);
                }
                Some(Beat::Arrived(name)) => {
                    if let Some(boss) = enemies.iter_mut().find(|enemy| director.boss == Some(enemy.id)) {
                        boss.harden(rules.difficulty.boss_rounds());
                    }
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
                    fx.add_punch(1.0);
                    report.boss_slain = true;
                    players.share_boss(run.bosses, tier, fx);
                }
                None => {}
            }
        }
        award_recruits(units, babies, players, fx);
        Sim::tuck_strays(players, units);
        run.peak_army = run.peak_army.max(army_sizes(units)[..count].iter().copied().max().unwrap_or(0));

        report.hit_stop = std::mem::take(&mut fx.hit_stop);
        let mut events = fx.drain_events();
        if destroyed > 0 {
            events.push(Event::Destroyed(destroyed as u32));
        }
        Step { events, report }
    }
}
