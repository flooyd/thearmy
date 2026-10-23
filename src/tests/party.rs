//a game with others that goes on and is kept: a player who goes has their army held for them (out of the world's reach), a saved game has everyone in it, away until they
//join it again by their names, and the ways the stay in a game of another ends are told apart by whether it is worth trying to get back into it

use super::*;

const RULES: Rules = Rules { army_limit: DEFAULT_ARMY_LIMIT, difficulty: Difficulty::Normal };
const DT: f32 = 1.0 / 60.0;

fn idle() -> PlayerInput {
    PlayerInput::idle(Formation::Free, Vec2::new(1920.0, 1080.0))
}

//a world with Ada (the host), Bo and Cy in it, each with an army of their own
fn trio() -> (Sim, Table) {
    let mut sim = Sim::from_snapshot(new_run(), RULES);
    sim.name_host("Ada");
    let mut table = Table::new();
    assert_eq!(table.seat(&mut sim, 1, "Bo"), Ok(1));
    assert_eq!(table.seat(&mut sim, 2, "Cy"), Ok(2));
    (sim, table)
}

fn army_of(sim: &Sim, player: usize) -> usize {
    sim.units.iter().filter(|unit| unit.owner as usize == player).count()
}

fn held_by(sim: &Sim, player: usize) -> usize {
    sim.players.list[player].held.as_ref().map_or(0, |held| held.len())
}

fn steps(sim: &mut Sim, count: usize) {
    for _ in 0..count {
        sim.step(vec![idle(); sim.players.len()], Pace::steady(DT), RULES);
    }
}

// ---------- held armies ----------

#[test]
fn a_player_who_goes_has_their_army_held_and_the_others_are_not_touched() {
    let (mut sim, mut table) = trio();
    let (ada, bo, cy) = (army_of(&sim, 0), army_of(&sim, 1), army_of(&sim, 2));
    assert!(ada > 0 && bo > 0 && cy > 0);
    table.unseat(&mut sim, 1);
    assert_eq!(sim.players.away_names(), ["Bo"]);
    assert!(sim.players.list[1].is_away() && !sim.players.list[2].is_away());
    assert_eq!(army_of(&sim, 1), 0, "none of it is in the world");
    assert_eq!(held_by(&sim, 1), bo);
    assert_eq!((army_of(&sim, 0), army_of(&sim, 2)), (ada, cy));
    //(going twice is going once)
    sim.hold(1);
    assert_eq!(held_by(&sim, 1), bo);
}

#[test]
fn the_host_is_the_game_and_is_never_held() {
    let (mut sim, _) = trio();
    let before = army_of(&sim, 0);
    sim.hold(0);
    assert!(!sim.players.list[0].is_away());
    assert_eq!(army_of(&sim, 0), before);
    sim.hold(9);
}

#[test]
fn what_is_held_comes_back_to_where_it_was_when_the_player_does() {
    let (mut sim, mut table) = trio();
    let before: Vec<(f32, f32)> = sim.units.iter().filter(|unit| unit.owner == 1).map(|unit| (unit.x, unit.y)).collect();
    table.unseat(&mut sim, 1);
    steps(&mut sim, 120);
    assert_eq!(table.seat(&mut sim, 1, "bo"), Ok(1));
    assert!(!sim.players.list[1].is_away());
    assert_eq!(sim.players.away_names().len(), 0);
    let after: Vec<(f32, f32)> = sim.units.iter().filter(|unit| unit.owner == 1).map(|unit| (unit.x, unit.y)).collect();
    assert_eq!(after.len(), before.len());
    for (was, is) in before.iter().zip(&after) {
        assert_eq!(was, is, "where each unit stood, as nothing in the world moved it");
    }
}

#[test]
fn nothing_in_the_world_can_hurt_a_held_army() {
    let (mut sim, mut table) = trio();
    let at = sim.players.list[1].position;
    table.unseat(&mut sim, 1);
    //a crowd of enemies right on where the army stood
    sim.enemies.clear();
    for i in 0..20 {
        let mut enemy = kind_at("Grunt", at + Vec2::new(i as f32 * 3.0, 0.0));
        enemy.attack = 500.0;
        sim.enemies.push(enemy);
    }
    let held = held_by(&sim, 1);
    let healths: Vec<f32> = sim.players.list[1].held.as_ref().unwrap().iter().map(|unit| unit.health).collect();
    steps(&mut sim, 300);
    assert_eq!(held_by(&sim, 1), held, "none of them was lost");
    let after: Vec<f32> = sim.players.list[1].held.as_ref().unwrap().iter().map(|unit| unit.health).collect();
    assert_eq!(after, healths, "nor hurt");
    assert_eq!(army_of(&sim, 1), 0);
}

#[test]
fn a_player_who_is_away_is_not_given_a_new_army_in_place_of_the_held_one() {
    //(the game revives a player with none, but one who is away has an army: it is held)
    let (mut sim, mut table) = trio();
    table.unseat(&mut sim, 1);
    assert_eq!(army_of(&sim, 1), 0);
    assert!(sim.players.list[1].is_away(), "which is what tells the two apart");
    assert!(held_by(&sim, 1) > 0);
}

#[test]
fn units_in_a_tower_stay_in_it_when_their_player_goes() {
    let (mut sim, mut table) = trio();
    let tile = (600, 600);
    let at = sim.players.list[1].position;
    let mut inside = unit_at(at.x, at.y).owned_by(1);
    inside.garrison = Some(tile);
    sim.units.push(inside);
    let ungarrisoned = army_of(&sim, 1) - 1;
    table.unseat(&mut sim, 1);
    assert_eq!(army_of(&sim, 1), 1, "the one in the tower");
    assert_eq!(held_by(&sim, 1), ungarrisoned);
}

#[test]
fn whatever_comes_to_a_player_who_is_away_is_held_with_the_rest() {
    let (mut sim, mut table) = trio();
    table.unseat(&mut sim, 1);
    let held = held_by(&sim, 1);
    let at = sim.players.list[1].position;
    sim.units.push(unit_at(at.x, at.y).owned_by(1));
    steps(&mut sim, 1);
    assert_eq!(army_of(&sim, 1), 0, "it is not left out in the world");
    assert_eq!(held_by(&sim, 1), held + 1);
}

#[test]
fn a_held_player_keeps_what_they_were_choosing_from() {
    let (mut sim, mut table) = trio();
    sim.players.list[1].offer(RelicChoice::new());
    table.unseat(&mut sim, 1);
    assert!(sim.players.list[1].choice.is_some());
    table.seat(&mut sim, 1, "Bo").unwrap();
    assert!(sim.players.list[1].choice.is_some(), "and is still choosing when they are back");
}

#[test]
fn the_squads_of_a_player_who_goes_are_called_off() {
    let (mut sim, mut table) = trio();
    for unit in sim.units.iter_mut().filter(|unit| unit.owner == 1) {
        unit.target = Some(5);
    }
    table.unseat(&mut sim, 1);
    assert!(sim.players.list[1].held.as_ref().unwrap().iter().all(|unit| unit.target.is_none()));
}

// ---------- a saved game with others in it ----------

//a party of three that has been played a while: Bo has gone, Cy is there, and each has relics and a choice of their own
fn played() -> Sim {
    let (mut sim, mut table) = trio();
    sim.players.list[1].upgrades.take(Relic::Muster);
    sim.players.list[1].rerolls = 3;
    sim.players.list[1].mason.points = 123.5;
    sim.players.list[2].offer(RelicChoice::new());
    sim.players.list[2].position += Vec2::new(300.0, 40.0);
    for unit in sim.units.iter_mut().filter(|unit| unit.owner == 2) {
        unit.x += 300.0;
        unit.y += 40.0;
    }
    let cy_unit = sim.units.iter().position(|unit| unit.owner == 2).unwrap();
    sim.units[cy_unit].champion = champion_named("Phoenix").unwrap() as u8 + 1;
    sim.units[cy_unit].power = 12.5;
    sim.units[cy_unit].level = 2;
    steps(&mut sim, 30);
    table.unseat(&mut sim, 1);
    sim
}

#[test]
fn a_game_with_others_is_kept_with_all_of_them_in_it() {
    let sim = played();
    let text = sim.capture(0, 1.0, false).to_text();
    let loaded = Snapshot::from_text(&text).expect("it reads back");
    assert_eq!(loaded.company.len(), 2);
    assert_eq!((loaded.company[0].name.as_str(), loaded.company[1].name.as_str()), ("Bo", "Cy"));
    assert_eq!(loaded.company[0].rerolls, 3);
    assert_eq!(loaded.company[0].wall_points, 123.5);
    assert_eq!(loaded.company[0].upgrades.taken.len(), 1);
    assert!(loaded.company[1].choice.is_some() && loaded.company[0].choice.is_none());
    assert_eq!(loaded.company[1].position, sim.players.list[2].position);
    //every army is there, whether it was held or in the world
    let kept: Vec<usize> = (0..3).map(|owner| loaded.units.iter().filter(|unit| unit.owner as usize == owner).count()).collect();
    let was: Vec<usize> = (0..3).map(|owner| army_of(&sim, owner) + held_by(&sim, owner)).collect();
    assert_eq!(kept, was);
    assert_eq!(loaded.units.len(), was.iter().sum::<usize>());
}

#[test]
fn a_game_with_others_comes_back_with_everyone_but_the_host_away() {
    let sim = played();
    let was: Vec<usize> = (0..3).map(|owner| army_of(&sim, owner) + held_by(&sim, owner)).collect();
    let text = sim.capture(0, 1.0, false).to_text();
    let back = Sim::from_snapshot(Snapshot::from_text(&text).unwrap(), RULES);
    assert_eq!(back.players.len(), 3);
    assert_eq!(back.players.away_names(), ["Bo", "Cy"]);
    assert_eq!(army_of(&back, 0), was[0], "the host\'s army is in the world");
    assert_eq!((army_of(&back, 1), army_of(&back, 2)), (0, 0), "and the others\' are held");
    assert_eq!((held_by(&back, 1), held_by(&back, 2)), (was[1], was[2]));
    assert_eq!(back.players.list[1].upgrades.taken.len(), 1);
    assert_eq!(back.players.list[1].rerolls, 3);
    assert_eq!(back.players.list[1].mason.points, 123.5);
    assert!(back.players.list[2].choice.is_some());
    assert_eq!(back.players.list[2].position, sim.players.list[2].position);
    //the champion is still a champion, at its level
    let champion = back.players.list[2].held.as_ref().unwrap().iter().find(|unit| unit.champion != 0).expect("the champion was kept");
    let champion_then = sim.units.iter().find(|unit| unit.owner == 2 && unit.champion != 0).unwrap();
    assert_eq!((champion.champion_kind().map(|kind| kind.name), champion.power, champion.level), (Some("Phoenix"), champion_then.power, 2));
    //a second trip changes nothing more
    let (again, first) = (back.capture(0, 1.0, false).body(), Snapshot::from_text(&text).unwrap().body());
    let at = again.bytes().zip(first.bytes()).position(|(a, b)| a != b).unwrap_or(again.len().min(first.len()));
    assert!(again == first, "they differ at {at}: {:?} and {:?}", &again[at.saturating_sub(60)..(at + 60).min(again.len())], &first[at.saturating_sub(60)..(at + 60).min(first.len())]);
}

#[test]
fn the_players_of_a_saved_game_take_their_armies_up_again_by_joining_with_their_names() {
    let sim = played();
    let was = army_of(&sim, 2) + held_by(&sim, 2);
    let text = sim.capture(0, 1.0, false).to_text();
    let mut back = Sim::from_snapshot(Snapshot::from_text(&text).unwrap(), RULES);
    let mut table = Table::new();
    assert_eq!(table.seat(&mut back, 3, "CY"), Ok(2), "in the place they had, whatever their capitals");
    assert_eq!(army_of(&back, 2), was);
    assert_eq!(back.players.away_names(), ["Bo"]);
    assert_eq!(back.players.len(), 3, "no one new");
    //(and they can go and come back as any player can)
    table.unseat(&mut back, 3);
    assert_eq!(army_of(&back, 2), 0);
    assert_eq!(table.seat(&mut back, 1, "Cy"), Ok(2));
    assert_eq!(army_of(&back, 2), was);
}

#[test]
fn a_saved_game_with_room_left_takes_a_new_player_and_one_that_is_full_does_not() {
    let sim = played();
    let text = sim.capture(0, 1.0, false).to_text();
    let mut back = Sim::from_snapshot(Snapshot::from_text(&text).unwrap(), RULES);
    let mut table = Table::new();
    assert_eq!(table.seat(&mut back, 3, "Di"), Ok(3), "the place that is left");
    assert!(table.seat(&mut back, 4, "Ed").is_err(), "none is");
    assert_eq!(back.players.away_names(), ["Bo", "Cy"], "and the ones who are away are still away");
}

#[test]
fn a_player_of_a_saved_game_who_has_the_hosts_name_is_made_unlike_them() {
    let sim = played();
    let text = sim.capture(0, 1.0, false).to_text();
    let mut back = Sim::from_snapshot(Snapshot::from_text(&text).unwrap(), RULES);
    back.name_host("bo");
    assert_eq!(back.players.list[0].name, "bo");
    assert_ne!(back.players.list[1].name.to_lowercase(), "bo");
    assert!(back.players.list[1].name.starts_with("Bo"));
    let names: Vec<String> = back.players.list.iter().map(|player| player.name.to_lowercase()).collect();
    assert!(names.iter().enumerate().all(|(i, name)| !names[..i].contains(name)), "{names:?}");
}

#[test]
fn a_game_of_one_is_kept_exactly_as_it_always_was() {
    let sim = Sim::from_snapshot(new_run(), RULES);
    let snapshot = sim.capture(0, 1.0, false);
    assert!(snapshot.company.is_empty());
    let text = snapshot.to_text();
    assert!(!text.contains("party") && !text.contains("owners"));
    let header = text.lines().next().unwrap();
    assert_eq!(header.split_whitespace().count(), 6, "the same words as before in the first line: {header}");
    assert_eq!(SaveInfo::from_header(header).unwrap().party, 1);
    assert_eq!(Sim::from_snapshot(Snapshot::from_text(&text).unwrap(), RULES).players.len(), 1);
}

#[test]
fn the_first_line_of_a_saved_game_says_how_many_were_in_it() {
    let text = played().capture(0, 1.0, false).to_text();
    let info = SaveInfo::from_header(text.lines().next().unwrap()).unwrap();
    assert_eq!(info.party, 3);
    assert!(info.army > 0);
}

#[test]
fn a_saved_game_that_is_damaged_is_refused_and_not_half_loaded() {
    let good = played().capture(0, 1.0, false).to_text();
    assert!(Snapshot::from_text(&good).is_ok());
    let damage = |from: &str, to: &str| {
        assert_eq!(good.matches(from).count(), 1, "{from:?}");
        Snapshot::from_text(&good.replacen(from, to, 1))
    };
    assert!(damage("party 2", "party 0").is_err(), "no one in a party");
    assert!(damage("party 2", "party 9").is_err(), "more than a game holds");
    //a name that is not hexadecimal, or not a name a player can have
    let line = |name: &str| format!("\n{} ", name_word(name));
    assert!(damage(&line("Bo"), "\nzz ").is_err());
    assert!(damage(&line("Bo"), "\n2f2f ").is_err(), "// is not a name");
    //two players with one name
    assert!(damage(&line("Cy"), &line("bO")).is_err());
    //armies that do not add up to the units there are
    let cut = good.find("owners ").unwrap();
    assert!(Snapshot::from_text(&good[..cut]).is_err(), "a party with no owners");
    let line_end = good[cut..].find('\n').map_or(good.len(), |end| cut + end);
    assert!(Snapshot::from_text(&format!("{}owners 1 0 1\n", &good[..cut])).is_err(), "one unit of an owner and the rest of none");
    assert!(Snapshot::from_text(&format!("{}owners 1 7 99999999{}", &good[..cut], &good[line_end..])).is_err(), "an owner that is not in the game, and more units than there are");
    //and a save that stops part way, wherever it stops
    for at in (cut..good.len()).step_by(7) {
        assert!(Snapshot::from_text(&good[..at]).is_err() || at + 1 >= good.len(), "cut at {at}");
    }
}

#[test]
fn a_game_with_others_has_a_save_of_its_own_that_a_game_of_one_does_not_go_on_from() {
    let dir = std::env::temp_dir().join(format!("the-army-test-{}-party", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let saves = Saves::in_dir(&dir);
    assert_ne!(saves.path(Slot::Party), saves.path(Slot::Auto));
    assert_ne!(saves.path(Slot::Party), saves.path(Slot::Quick));
    assert!(saves.info(Slot::Party).is_none());
    let party = played().capture(0, 1.0, false);
    saves.write(Slot::Party, &party.to_text()).unwrap();
    let info = saves.info(Slot::Party).expect("it can be told about");
    assert_eq!(info.party, 3);
    assert_eq!(saves.read(Slot::Party).unwrap().company.len(), 2);
    assert!(saves.newest().is_none(), "the title screen\'s own continue is of a run of one");
    saves.write(Slot::Quick, &new_run().to_text()).unwrap();
    assert_eq!(saves.newest().map(|(slot, _)| slot), Some(Slot::Quick));
    saves.delete(Slot::Party);
    assert!(saves.info(Slot::Party).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------- getting back into a game ----------

#[test]
fn the_host_ending_the_game_is_the_end_of_it_and_the_connection_going_is_not() {
    let ended = Lost::by(ClientEvent::Refused(Refusal::HostClosed));
    assert!(!ended.retry && ended.why.contains("host"), "{ended:?}");
    assert!(!Lost::by(ClientEvent::Refused(Refusal::Kicked)).retry, "put out is put out");
    assert!(!Lost::by(ClientEvent::Refused(Refusal::WrongPassword)).retry);
    assert!(Lost::by(ClientEvent::Closed("the connection ended (timed out)".to_owned())).retry);
    assert!(Lost::by(ClientEvent::Failed("no answer".to_owned())).retry);
    assert!(!Lost::by(ClientEvent::Closed("you left the game".to_owned())).retry, "leaving is the player\'s own doing");
}
