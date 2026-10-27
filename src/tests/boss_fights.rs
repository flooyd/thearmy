//whole boss fights played by simple rules, to see how they balance

use super::*;

// ---------- whole boss fights, played by simple rules, to see how they balance ----------

const DODGE_REACTION: f32 = 0.7; //seconds between a circle turning up on the ground and the player moving

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Style {
    Tank,  //send everyone at the boss and stand still
    Dodge, //the same, but when something is about to land on the army call it back and run, then send it again
    Flank, //send everyone, then stand on the far side of the boss from the army, so it turns its armoured front away from them
}

#[derive(Debug)]
pub(super) struct Fought {
    won: bool,
    seconds: f32,
    lost: usize,
    army: usize,
    recalls: usize,
    changes: usize, //the phases it made it to, its splits and its last stand, counted
}

//a whole fight with one boss with an army of `army` units, in an empty world, played the way `style` says. there is nothing
//else about, and nobody picks up babies
pub(super) fn fight_a_boss(def: &'static BossDef, army: usize, tier: u32, style: Style, upgrades: &Upgrades, seed: u64) -> Fought {
    fight_a_hardened_boss(def, army, tier, style, upgrades, seed, 0)
}

//a fight on a difficulty as the late-run tables have it: the boss `tier` rounds on and the difficulty's rounds further, the enemies answering the
//relics as the difficulty says, and the rules of the Insane ladder in play (see `play_on`). the boss is made for the army itself, as in the tables
//the difficulties were tuned with, and there are no champions
pub(super) fn fight_on(def: &'static BossDef, army: usize, tier: u32, difficulty: Difficulty, relics: &Upgrades, seed: u64) -> Fought {
    play_on(difficulty);
    let mut upgrades = relics.clone();
    upgrades.foe_answer = difficulty.relic_answer();
    fight_a_hardened_boss(def, army, tier, Style::Dodge, &upgrades, seed, difficulty.boss_rounds())
}

//the same with a boss `rounds` further on, as the difficulty makes it
pub(super) fn fight_a_hardened_boss(def: &'static BossDef, army: usize, tier: u32, style: Style, upgrades: &Upgrades, seed: u64, rounds: u32) -> Fought {
    rand::srand(seed);
    let dt = 1.0 / 60.0;
    let mut walls = Walls::default();
    let mut pulls: Vec<Pull> = Vec::new();
    let mut player = MIDDLE;
    let side = (army as f32).sqrt().ceil() as usize;
    let mut units: Vec<Unit> = (0..army)
        .map(|i| {
            let at = player + Vec2::new((i % side) as f32 - side as f32 / 2.0, (i / side) as f32 - side as f32 / 2.0) * 34.0;
            Unit::new(at.x, at.y, rand::gen_range(UNIT_MIN_SPEED, UNIT_MAX_SPEED))
        })
        .collect();
    let mut enemies = vec![Enemy::boss(def, army, tier, player + Vec2::new(BOSS_SPAWN_MIN_TILES * TILE_SIZE, 0.0))];
    enemies[0].harden(rounds);
    let (mut babies, mut shots, mut hazards, mut squads) = (Vec::new(), Vec::new(), Vec::<Hazard>::new(), Squads::new());
    let mut fx = Effects::default();
    let mut field = field_for(player, &walls);
    let mut next_field = FlowField::new();
    let (mut recalls, mut away_for, mut seconds) = (0, 0.0f32, 0.0f32);
    let trace = std::env::var("BOSS_TRACE").ok().and_then(|v| v.parse::<u64>().ok()) == Some(seed);
    let mut next_trace = 0.0;
    let mut dodging = false;
    let mut changes = 0;
    //the front of the army fires the way the player walks, as in the game (see vanguard.rs): at the boss on the way to it, not while running from it
    let mut bolts: Vec<Bolt> = Vec::new();
    let mut facing = Vec2::X;
    let mut was = player;
    while seconds < 150.0 && !units.is_empty() && current_boss(&enemies).is_some() {
        seconds += dt;
        changes = changes.max(enemies.iter().filter_map(|enemy| enemy.boss.as_ref()).map(|state| state.phase + state.generation as usize + state.last_stand as usize).max().unwrap_or(0));
        if trace && seconds >= next_trace {
            next_trace += 1.5;
            let family: Vec<String> = enemies.iter().filter(|e| e.boss.is_some()).map(|e| format!("{:.0}%@{:.0}px", e.health / e.max_health * 100.0, e.position.distance(player))).collect();
            let others = enemies.iter().filter(|e| e.boss.is_none()).count();
            eprintln!("{seconds:>5.1}s army {:>5} bosses {family:?} others {others} hazards {} shots {} squads {}", units.len(), hazards.len(), shots.len(), squads.len());
        }
        let enemy_boost = speed_multiplier(units.len());
        let boost = upgrades.army_boost(units.len());
        //what the player does
        let mut threat: Option<Vec2> = None; //the way out of the one to get out of
        if let (Style::Dodge, Some(_)) = (style, current_boss(&enemies)) {
            //the circles on the ground that count down for a while and would kill what is in them (the slams, waves, pulses and
            //meteors), once there are a few of the army in one. the player takes a moment to see one, then runs
            let in_the_red = |hazard: &Hazard| units.iter().filter(|unit| hazard.contains(Vec2::new(unit.x, unit.y))).count() >= (units.len() / 50).max(3);
            threat = hazards
                .iter()
                .find(|hazard| hazard.armed && hazard.damage >= UNIT_HEALTH && hazard.warn_total >= 1.2 && hazard.warn_total - hazard.warn >= DODGE_REACTION && in_the_red(hazard))
                .map(|hazard| hazard.way_out(player));
        }
        if let Some(away) = threat {
            if !dodging {
                dodging = true;
                recalls += 1;
                recall_squads(&mut units, &mut squads, player, &mut fx);
            }
            away_for = 0.3;
            player = (player + away * 800.0 * boost * dt).clamp(Vec2::splat(RADIUS), Vec2::splat(WORLD_SIZE - RADIUS));
        } else {
            away_for -= dt;
            if away_for <= 0.0 {
                dodging = false;
            }
            if !dodging && (seconds * 2.0).fract() < dt * 2.0 {
                if let Some(boss) = current_boss(&enemies) {
                    send_squad(&mut units, &mut squads, boss, &mut fx);
                }
            }
            //and having got clear, the dodger walks back to where the fight is
            if style == Style::Dodge && !dodging {
                if let Some(boss) = current_boss(&enemies) {
                    let to_boss = boss.position - player;
                    if to_boss.length() > boss.size / 2.0 + 1000.0 {
                        player = (player + to_boss.normalize_or_zero() * 800.0 * boost * dt).clamp(Vec2::splat(RADIUS), Vec2::splat(WORLD_SIZE - RADIUS));
                    }
                }
            }
        }
        let player_tile = tile_of(player.x, player.y);
        if !next_field.is_building() && field.target != player_tile {
            next_field.start(player, &walls);
        }
        if next_field.is_building() && next_field.work(FIELD_BUDGET) {
            std::mem::swap(&mut field, &mut next_field);
        }
        if style == Style::Flank {
            if let Some(boss) = current_boss(&enemies) {
                let centre = units.iter().map(|unit| Vec2::new(unit.x, unit.y)).sum::<Vec2>() / units.len().max(1) as f32;
                let behind = (boss.position - centre).normalize_or_zero();
                player = (boss.position + behind * 900.0).clamp(Vec2::splat(RADIUS), Vec2::splat(WORLD_SIZE - RADIUS));
            }
        }
        player = update_pulls(&mut pulls, &mut units, player, &walls, dt);
        update_squads(&mut squads, &enemies, &mut units, &walls, false);
        update_units(&mut units, player.x, player.y, dt, &walls, &field, boost, &squads);
        //the same order as in the game
        patrol_free(&mut enemies, player, dt, enemy_boost, &mut fx);
        enemy_abilities(&mut enemies, &units, player, dt, &mut fx);
        boss_act(&mut enemies, &units, player, dt, &mut shots, &mut fx);
        if raise_walls(&mut walls, &mut fx.raise) {
            field = field_for(player, &walls);
        }
        pulls.append(&mut fx.pulls);
        shoot_enemies(&mut enemies, &units, &mut shots, dt, &mut fx);
        lay_hazards(&mut hazards, &mut fx.hazards);
        let h0 = army_health(&units);
        update_shots(&mut shots, &mut units, &walls, dt, upgrades, &mut fx);
        let h1 = army_health(&units);
        update_hazards(&mut hazards, &mut units, upgrades, dt, &mut fx);
        let h2 = army_health(&units);
        heal_units(&mut units, dt, upgrades);
        if player.distance(was) > 0.5 {
            facing = (player - was).normalize();
        }
        was = player;
        let front = [Front { position: player, facing, formation: Formation::Ring, upgrades }];
        fire_vanguard(&front, &units, &enemies, seconds, dt, 1.0, true, &mut bolts, Some(&mut fx));
        fly_bolts(&mut bolts, &mut enemies, dt, Some(&mut fx));
        fight(&mut enemies, &mut units, &mut babies, dt, upgrades, &mut fx);
        let h3 = army_health(&units);
        if trace && h0 - h3 > 1500.0 {
            let burning: Vec<String> = hazards.iter().map(|h| format!("r{:.0} dmg{:.0} burn{:.0} linger{:.1} armed{}", h.radius, h.damage, h.burn, h.linger, h.armed)).collect();
            eprintln!("{seconds:>6.2}s lost health: shots {:.0}, hazards {:.0}, contact {:.0}; hazards {burning:?}", h0 - h1, h1 - h2, h2 - h3);
        }
        lay_hazards(&mut hazards, &mut fx.hazards);
        enrage_enemies(&mut enemies);
        tick_enemy_numbers(&mut enemies, dt, &mut fx);
        bury_units(&mut units, &mut fx);
        fx = Effects::default();
    }
    Fought { won: current_boss(&enemies).is_none() && !units.is_empty(), seconds, lost: army - units.len().min(army), army, recalls, changes }
}

#[test]
#[ignore]
fn the_boss_balance_table() {
    let upgrades = Upgrades::default();
    for (index, def) in BOSSES.iter().enumerate() {
        //BOSS_ONLY=4,5 plays only those
        if let Ok(only) = std::env::var("BOSS_ONLY") {
            if !only.split(',').any(|part| part.trim().parse::<usize>() == Ok(index)) {
                continue;
            }
        }
        for factor in [1usize, 4] {
            let army = def.min_army * factor;
            for style in [Style::Tank, Style::Dodge, Style::Flank] {
                let runs: Vec<Fought> = (0..2).map(|seed| fight_a_boss(def, army, 0, style, &upgrades, 100 + seed)).collect();
                let wins = runs.iter().filter(|run| run.won).count();
                let seconds = runs.iter().map(|run| run.seconds).sum::<f32>() / 2.0;
                let lost = runs.iter().map(|run| run.lost as f32 / run.army as f32).sum::<f32>() / 2.0;
                let recalls = runs.iter().map(|run| run.recalls).sum::<usize>() as f32 / 2.0;
                eprintln!("{index} {:<16} army {army:>5} {style:?}: won {wins}/2, {seconds:>5.1} s, lost {:>3.0}%, recalls {recalls:.0}", def.name, lost * 100.0);
            }
        }
    }
}

//the Insane ladder against Brutal (5.4 of the plan): each boss on each difficulty against the late run's army and relics (see
//`the_boss_balance_of_a_late_run`), dodging, over two seeds: how often it is won, how long it takes and how much of the army is lost. LADDER_TIERS=0,1,5
//picks the tiers (0 is Brutal), BOSS_ONLY the bosses, ARMY the army (default 10000) and TIER the round (default 4)
#[test]
#[ignore]
fn the_insane_ladder_table() {
    let tiers: Vec<u8> = std::env::var("LADDER_TIERS").ok().map_or(vec![0, 1, 3, 5, 8, 10], |text| text.split(',').filter_map(|part| part.trim().parse().ok()).collect());
    let army: usize = std::env::var("ARMY").ok().and_then(|v| v.parse().ok()).unwrap_or(10_000);
    let round: u32 = std::env::var("TIER").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let relics = late_run_upgrades();
    eprintln!("{army} units, {} relics, round {round}, dodging", relics.taken.len());
    for (index, def) in BOSSES.iter().enumerate() {
        if let Ok(only) = std::env::var("BOSS_ONLY") {
            if !only.split(',').any(|part| part.trim().parse::<usize>() == Ok(index)) {
                continue;
            }
        }
        for &tier in &tiers {
            let difficulty = if tier == 0 { Difficulty::Brutal } else { Difficulty::Insane(tier) };
            let runs: Vec<Fought> = (0..2).map(|seed| fight_on(def, army, round, difficulty, &relics, 300 + seed)).collect();
            eprintln!(
                "{index} {:<16} {:<9} +{:>2} rounds, answer {:.2}: won {}/2, {:>5.1} s, lost {:>3.0}%",
                def.name,
                difficulty.name(),
                difficulty.boss_rounds(),
                difficulty.relic_answer(),
                runs.iter().filter(|run| run.won).count(),
                runs.iter().map(|run| run.seconds).sum::<f32>() / 2.0,
                runs.iter().map(|run| run.lost as f32 / run.army as f32 * 100.0).sum::<f32>() / 2.0
            );
        }
    }
}

pub(super) fn army_health(units: &[Unit]) -> f32 {
    units.iter().map(|unit| unit.health.max(0.0)).sum()
}

//one fight told second by second: the boss's health and phase, the army, and what has hurt it (contact, hazards, shots)
#[test]
#[ignore]
fn one_boss_fight_in_detail() {
    let spec = std::env::var("BOSS_DETAIL").unwrap_or_else(|_| "1,600,tank".to_owned());
    let parts: Vec<&str> = spec.split(',').collect();
    let def = &BOSSES[parts[0].parse::<usize>().unwrap()];
    let army: usize = parts[1].parse().unwrap();
    let upgrades = Upgrades::default();
    rand::srand(7);
    let dt = 1.0 / 60.0;
    let walls = Walls::default();
    let player = MIDDLE;
    let side = (army as f32).sqrt().ceil() as usize;
    let mut units: Vec<Unit> = (0..army)
        .map(|i| {
            let at = player + Vec2::new((i % side) as f32 - side as f32 / 2.0, (i / side) as f32 - side as f32 / 2.0) * 34.0;
            Unit::new(at.x, at.y, rand::gen_range(UNIT_MIN_SPEED, UNIT_MAX_SPEED))
        })
        .collect();
    let mut enemies = vec![Enemy::boss(def, army, 0, player + Vec2::new(BOSS_SPAWN_MIN_TILES * TILE_SIZE, 0.0))];
    let (mut babies, mut shots, mut hazards, mut squads) = (Vec::new(), Vec::new(), Vec::<Hazard>::new(), Squads::new());
    let mut fx = Effects::default();
    let field = field_for(player, &walls);
    let (mut contact, mut zones, mut bullets, mut touching_max) = (0.0f32, 0.0f32, 0.0f32, 0usize);
    let mut seconds = 0.0f32;
    let mut next_print = 0.0;
    eprintln!("{} army {army}: size {:.0}, health {:.0}, attack {:.0}/s", def.name, enemies[0].size, enemies[0].max_health, enemies[0].attack);
    while seconds < 90.0 && !units.is_empty() && current_boss(&enemies).is_some() {
        seconds += dt;
        let enemy_boost = speed_multiplier(units.len());
        if (seconds * 2.0).fract() < dt * 2.0 {
            if let Some(boss) = current_boss(&enemies) {
                send_squad(&mut units, &mut squads, boss, &mut fx);
            }
        }
        update_squads(&mut squads, &enemies, &mut units, &walls, false);
        update_units(&mut units, player.x, player.y, dt, &walls, &field, enemy_boost, &squads);
        patrol_free(&mut enemies, player, dt, enemy_boost, &mut fx);
        enemy_abilities(&mut enemies, &units, player, dt, &mut fx);
        boss_act(&mut enemies, &units, player, dt, &mut shots, &mut fx);
        shoot_enemies(&mut enemies, &units, &mut shots, dt, &mut fx);
        hazards.append(&mut fx.hazards);
        let before = army_health(&units);
        update_shots(&mut shots, &mut units, &walls, dt, &upgrades, &mut fx);
        let after_shots = army_health(&units);
        update_hazards(&mut hazards, &mut units, &upgrades, dt, &mut fx);
        let after_zones = army_health(&units);
        heal_units(&mut units, dt, &upgrades);
        let healed = army_health(&units);
        fight(&mut enemies, &mut units, &mut babies, dt, &upgrades, &mut fx);
        let after_fight = army_health(&units);
        bullets += before - after_shots;
        zones += after_shots - after_zones;
        contact += healed - after_fight;
        hazards.append(&mut fx.hazards);
        tick_enemy_numbers(&mut enemies, dt, &mut fx);
        bury_units(&mut units, &mut fx);
        if let Some(boss) = current_boss(&enemies) {
            touching_max = touching_max.max(units.iter().filter(|unit| boss.overlaps_circle(unit.x, unit.y, RADIUS)).count());
            if seconds >= next_print {
                next_print += 2.0;
                let phase = boss.boss.as_ref().unwrap().phase;
                let minions = enemies.iter().filter(|enemy| enemy.owner == Some(boss.id)).count();
                let touching = units.iter().filter(|unit| boss.overlaps_circle(unit.x, unit.y, RADIUS)).count();
                eprintln!(
                    "{seconds:>5.1}s boss {:>3.0}% shield {:>5.0} phase {phase} minions {minions:>2} at {:>5.0} px | army {:>5} touching {touching:>4} | hurt by contact {contact:>7.0}, zones {zones:>7.0}, shots {bullets:>7.0}",
                    boss.health / boss.max_health * 100.0,
                    boss.shield,
                    boss.position.distance(player),
                    units.len()
                );
            }
        }
        fx = Effects::default();
    }
    eprintln!("ended at {seconds:.1}s: army {} left, boss {}", units.len(), if current_boss(&enemies).is_some() { "alive" } else { "dead" });
}

//later rounds against an army that has taken a few relics
#[test]
#[ignore]
fn the_boss_balance_table_for_later_rounds() {
    for (tier, relics) in [(1u32, vec![Relic::Whetstone, Relic::IronResolve, Relic::Regeneration]), (2, vec![Relic::Whetstone, Relic::IronResolve, Relic::Regeneration, Relic::Whetstone, Relic::IronResolve, Relic::Bounty, Relic::FleetFeet])] {
        let mut upgrades = Upgrades::default();
        for relic in relics {
            upgrades.take(relic);
        }
        for (index, def) in BOSSES.iter().enumerate() {
            for factor in [2usize, 6] {
                let army = def.min_army * factor;
                let runs: Vec<Fought> = (0..2).map(|seed| fight_a_boss(def, army, tier, Style::Tank, &upgrades, 200 + seed)).collect();
                let wins = runs.iter().filter(|run| run.won).count();
                let seconds = runs.iter().map(|run| run.seconds).sum::<f32>() / 2.0;
                let lost = runs.iter().map(|run| run.lost as f32 / run.army as f32).sum::<f32>() / 2.0;
                eprintln!("tier {tier} {index} {:<16} army {army:>5} Tank: won {wins}/2, {seconds:>5.1} s, lost {:>3.0}% ({} relics)", def.name, lost * 100.0, upgrades.taken.len());
            }
        }
    }
}

//the bosses a simple dodger can beat in these fights (it takes 0.7 s to react to a circle, like a person); the Singularity and the
//Bastion are played by hand with the table above, the bot is no good at breaking barriers or at the pull
const DODGEABLE: [usize; 5] = [0, 1, 2, 3, 4];

//how many of a few fights are won, and the share of the army lost on average
fn fights(def: &'static BossDef, army: usize, tier: u32, style: Style, upgrades: &Upgrades, first_seed: u64) -> (usize, f32) {
    let runs: Vec<Fought> = (0..3).map(|k| fight_a_boss(def, army, tier, style, upgrades, first_seed + k)).collect();
    (runs.iter().filter(|run| run.won).count(), runs.iter().map(|run| run.lost as f32 / run.army as f32).sum::<f32>() / 3.0)
}

#[test]
fn an_army_the_size_a_boss_asks_for_beats_it_by_keeping_out_of_the_circles_but_not_for_free() {
    for index in DODGEABLE {
        let def = &BOSSES[index];
        let (wins, lost) = fights(def, def.min_army, 0, Style::Dodge, &Upgrades::default(), 1 + 10 * index as u64);
        assert!(wins >= 2, "{}: won {wins} of 3", def.name);
        assert!(lost > 0.03 && lost < 0.7, "{}: lost {lost}", def.name);
    }
}

#[test]
fn standing_in_the_circles_of_a_boss_loses_the_army_where_getting_out_of_the_way_does_not() {
    //a boss is sized for the army, so a bigger army does not make standing there any safer
    for index in [0, 2, 3, 4] {
        let def = &BOSSES[index];
        let army = def.min_army * 4;
        let (tank_wins, tank_lost) = fights(def, army, 0, Style::Tank, &Upgrades::default(), 40 + 10 * index as u64);
        let (dodge_wins, dodge_lost) = fights(def, army, 0, Style::Dodge, &Upgrades::default(), 40 + 10 * index as u64);
        assert!(dodge_wins >= 2 && dodge_lost < 0.7, "{}: dodging won {dodge_wins} of 3 and lost {dodge_lost}", def.name);
        assert!(tank_wins <= 1 && tank_lost > 0.85, "{}: standing there won {tank_wins} of 3 and lost {tank_lost}", def.name);
    }
}

//how much one boss's fights vary: the same fight over many seeds. BOSS_ONLY=4 picks the boss (default 0), SEEDS the number of
//fights (default 30), FACTOR how many times the army it asks for (default 1), or ARMY an army of exactly that size
#[test]
#[ignore]
fn how_much_a_boss_fight_varies_over_many_seeds() {
    let index: usize = std::env::var("BOSS_ONLY").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let seeds: u64 = std::env::var("SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
    let factor: usize = std::env::var("FACTOR").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let def = &BOSSES[index];
    let army: usize = std::env::var("ARMY").ok().and_then(|v| v.parse().ok()).unwrap_or(def.min_army * factor);
    let mut lost: Vec<f32> = (0..seeds)
        .map(|seed| {
            let run = fight_a_boss(def, army, 0, Style::Tank, &Upgrades::default(), 1000 + seed);
            if run.won { run.lost as f32 / run.army as f32 } else { 9.9 }
        })
        .collect();
    for (seed, share) in lost.iter().enumerate().filter(|(_, share)| **share > 0.6) {
        eprintln!("seed {} lost {:.0}%", 1000 + seed, share * 100.0);
    }
    lost.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let losses = lost.iter().filter(|share| **share > 9.0).count();
    let won: Vec<f32> = lost.iter().copied().filter(|share| *share < 9.0).collect();
    eprintln!(
        "{} at {} units over {seeds} seeds: lost the fight {losses} times, won ones lost {:.0}% / {:.0}% / {:.0}% (best / median / worst)",
        def.name, army, won.first().copied().unwrap_or(0.0) * 100.0, won.get(won.len() / 2).copied().unwrap_or(0.0) * 100.0, won.last().copied().unwrap_or(0.0) * 100.0
    );
}

//what a late run looks like, with the relics of one that was played: BOSS_ONLY=3 picks the boss, ARMY the army (default 10000),
//TIER the round (default 4). the three difficulties against the same army, standing still and dodging
#[test]
#[ignore]
fn the_boss_balance_of_a_late_run() {
    let index: usize = std::env::var("BOSS_ONLY").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    let army: usize = std::env::var("ARMY").ok().and_then(|v| v.parse().ok()).unwrap_or(10_000);
    let tier: u32 = std::env::var("TIER").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let def = &BOSSES[index];
    let relics = late_run_upgrades();
    eprintln!("{} at round {tier}, {army} units, {} relics, damage x{:.1}", def.name, relics.taken.len(), relics.damage);
    for difficulty in DIFFICULTIES {
        let mut upgrades = late_run_upgrades();
        upgrades.foe_answer = difficulty.relic_answer();
        for style in [Style::Tank, Style::Dodge] {
            let rounds = difficulty.boss_rounds();
            let run = fight_a_hardened_boss(def, army, tier, style, &upgrades, 300, rounds);
            eprintln!(
                "{:<7} {style:?}: damage x{:.1} +{rounds} rounds: {} in {:>5.1} s, lost {:>3.0}%, {} of {} changes",
                difficulty.name(),
                upgrades.hit(),
                if run.won { "won " } else { "LOST" },
                run.seconds,
                run.lost as f32 / run.army as f32 * 100.0,
                run.changes,
                def.phases.len() - 1 + if def.splits { 2 } else { 0 } + 1
            );
        }
    }
}

//the relics of a run that was played a long way: 35 bosses slain, 40 relics, the damage 39 times what it starts at
pub(super) fn late_run_upgrades() -> Upgrades {
    let mut upgrades = Upgrades::default();
    for (relic, count) in [(Relic::Whetstone, 8), (Relic::GlassCannon, 4), (Relic::IronResolve, 4), (Relic::Regeneration, 6), (Relic::Overclock, 3), (Relic::FleetFeet, 3), (Relic::Aegis, 3), (Relic::Martyr, 2), (Relic::Thorns, 2), (Relic::Tailwind, 3), (Relic::OpeningSalvo, 1), (Relic::Freezing, 1)] {
        for _ in 0..count {
            upgrades.take(relic);
        }
    }
    upgrades
}

//the late run's army against bosses made `rounds` further on, with a few answers to the damage relics. BOSS_ONLY, ARMY and TIER as above
#[test]
#[ignore]
fn the_boss_balance_of_a_late_run_for_a_few_answers() {
    let index: usize = std::env::var("BOSS_ONLY").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    let army: usize = std::env::var("ARMY").ok().and_then(|v| v.parse().ok()).unwrap_or(10_000);
    let tier: u32 = std::env::var("TIER").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let def = &BOSSES[index];
    for rounds in [0u32, 1, 2] {
        for answer in [0.0f32, 0.5, 0.6, 0.7, 0.8, 0.9] {
            let mut upgrades = late_run_upgrades();
            upgrades.foe_answer = answer;
            let runs: Vec<Fought> = (0..2).map(|seed| fight_a_hardened_boss(def, army, tier, Style::Dodge, &upgrades, 300 + seed, rounds)).collect();
            eprintln!(
                "{} +{rounds} rounds, answer {answer:.1} (damage x{:>4.1}): won {}/2, {:>5.1} s, lost {:>3.0}%",
                def.name,
                upgrades.hit(),
                runs.iter().filter(|run| run.won).count(),
                runs.iter().map(|run| run.seconds).sum::<f32>() / 2.0,
                runs.iter().map(|run| run.lost as f32 / run.army as f32 * 100.0).sum::<f32>() / 2.0
            );
        }
    }
}

//the same over many seeds for an army that gets out of the way: BOSS_ONLY picks the boss (default 0), SEEDS the number of fights (default 10) and
//FACTOR how many times the army it asks for (default 1)
#[test]
#[ignore]
fn how_a_dodging_army_fares_over_many_seeds() {
    let index: usize = std::env::var("BOSS_ONLY").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let seeds: u64 = std::env::var("SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    let factor: usize = std::env::var("FACTOR").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let def = &BOSSES[index];
    let army = def.min_army * factor;
    let runs: Vec<Fought> = (0..seeds).map(|seed| fight_a_boss(def, army, 0, Style::Dodge, &Upgrades::default(), 2000 + seed)).collect();
    let won: Vec<&Fought> = runs.iter().filter(|run| run.won).collect();
    let mean = |values: Vec<f32>| values.iter().sum::<f32>() / values.len().max(1) as f32;
    eprintln!(
        "{} at {army} units dodging over {seeds} seeds: won {}, the won ones {:.1} s and {:.0}% of the army lost on average, {:.0} recalls",
        def.name,
        won.len(),
        mean(won.iter().map(|run| run.seconds).collect()),
        mean(won.iter().map(|run| run.lost as f32 / run.army as f32 * 100.0).collect()),
        mean(runs.iter().map(|run| run.recalls as f32).collect())
    );
}
