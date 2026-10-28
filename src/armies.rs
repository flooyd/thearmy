//! whose units are whose. every unit is in the one list, marked with the player it fights for; to deal with an army the list is
//! kept in order of owner, so each player's units are one stretch of it, and what is done to an army is done to that stretch

use crate::prelude::*;
use std::ops::Range;

//the place a unit's owner has in a list of MAX_PLAYERS: one that is past the end counts as the last
pub(crate) fn owner_index(owner: u8) -> usize {
    (owner as usize).min(MAX_PLAYERS - 1)
}

//how many units each player has
pub(crate) fn army_sizes(units: &[Unit]) -> [usize; MAX_PLAYERS] {
    let mut sizes = [0; MAX_PLAYERS];
    for unit in units {
        sizes[owner_index(unit.owner)] += 1;
    }
    sizes
}

//put the units in order of their owner. the sort is stable, so within an army nothing moves, and a list that is in order already
//(which it is when nobody has been added since it was last done) is left alone
pub(crate) fn sort_by_owner(units: &mut [Unit]) {
    if !units.is_sorted_by_key(|unit| unit.owner) {
        units.sort_by_key(|unit| unit.owner);
    }
}

//where each of `count` armies is in a list that is in order of owner. the units of an owner past the last belong to the last
pub(crate) fn army_ranges(units: &[Unit], count: usize) -> Vec<Range<usize>> {
    let mut ranges = Vec::with_capacity(count);
    let mut start = 0;
    for owner in 0..count {
        let end = if owner + 1 == count { units.len() } else { units.partition_point(|unit| unit.owner as usize <= owner) };
        ranges.push(start..end);
        start = end;
    }
    ranges
}

//the list put in order of owner if there is more than one player, and where each army is. with one player the whole list is the army,
//whatever the units are marked with, and there is nothing to do
pub(crate) fn arrange(units: &mut [Unit], players: usize) -> Vec<Range<usize>> {
    if players > 1 {
        sort_by_owner(units);
    }
    army_ranges(units, players.max(1))
}

//the list as one slice for each army (`ranges` are from `army_ranges`: side by side, from the start)
pub(crate) fn split_armies<'a>(mut units: &'a mut [Unit], ranges: &[Range<usize>]) -> Vec<&'a mut [Unit]> {
    let mut armies = Vec::with_capacity(ranges.len());
    for range in ranges {
        let (army, rest) = std::mem::take(&mut units).split_at_mut(range.len());
        armies.push(army);
        units = rest;
    }
    armies
}

//what each army is for a frame: where it is in the list, what its champions add for it, the relics it has with that on top, and how
//many times faster than normal it and its player are
pub(crate) struct Armies {
    pub(crate) ranges: Vec<Range<usize>>,
    pub(crate) perks: Vec<Perks>,
    pub(crate) perked: Vec<Upgrades>,
    pub(crate) boosts: Vec<f32>,
}

impl Armies {
    //the units are put in order of owner first
    pub(crate) fn of(units: &mut [Unit], players: &Players) -> Armies {
        let ranges = arrange(units, players.len());
        let perks: Vec<Perks> = ranges.iter().map(|range| Perks::of(&units[range.clone()])).collect();
        let perked: Vec<Upgrades> = perks.iter().zip(&players.list).map(|(perks, player)| perks.applied(&player.upgrades)).collect();
        let boosts = perked.iter().zip(&ranges).map(|(upgrades, range)| upgrades.army_boost(range.len())).collect();
        Armies { ranges, perks, perked, boosts }
    }

    pub(crate) fn size(&self, player: usize) -> usize {
        self.ranges[player].len()
    }

    //the players as the enemies see them this frame (see `prey_of`)
    pub(crate) fn prey<'a>(&self, players: &'a Players, units: &'a [Unit], pressure_of: impl Fn(usize) -> usize) -> Vec<Prey<'a>> {
        prey_of(players, units, &self.ranges, pressure_of)
    }
}

//the players as the enemies see them: each with their army (`ranges` are where the armies are in the list, which has to be in order of
//owner: see `arrange`), what is made for the enemies round them made for `pressure_of` their team (their army and those of the players
//near them), and the route the enemies take to them
pub(crate) fn prey_of<'a>(players: &'a Players, units: &'a [Unit], ranges: &[Range<usize>], pressure_of: impl Fn(usize) -> usize) -> Vec<Prey<'a>> {
    let sizes: Vec<usize> = ranges.iter().map(|range| range.len()).collect();
    let teams = team_armies(&players.positions(), &sizes);
    players
        .list
        .iter()
        .enumerate()
        .map(|(i, player)| Prey {
            position: player.position,
            units: &units[ranges[i].clone()],
            team: teams[i],
            pressure: pressure_of(teams[i]),
            boost: speed_multiplier(sizes[i]),
            cap: player.upgrades.unit_cap(),
            field: &player.foes.field,
        })
        .collect()
}

//the army side of a frame, for every player in turn: the route their army follows is kept up to date, their squads follow the enemies
//they were sent at, their army walks to them (or to its place in the formation) and the champions in it do what they do for the units
//round them. the work the routes and the squads may take in a frame is shared out between the players
pub(crate) fn step_armies(players: &mut Players, armies: &Armies, units: &mut [Unit], enemies: &[Enemy], walls: &Walls, walls_changed: bool, dt: f32, fx: &mut Effects) {
    let count = players.len() as u32;
    for (i, army) in split_armies(units, &armies.ranges).into_iter().enumerate() {
        let player = &mut players.list[i];
        player.route.update(player.position, walls, walls_changed, FIELD_BUDGET / count);
        for lost_at in update_squads_within(&mut player.squads, enemies, army, walls, walls_changed, SQUAD_ROUTE_BUDGET / count) {
            fx.cue(Cue::SquadLost);
            fx.popup(lost_at, RADIUS * 3.0, 0.0, "SQUAD LOST".to_owned(), ALERT);
        }
        //(the champions stand on a ring round the player that is big enough for all of them)
        let drill = player.drill().guarding(guard_radius(army.iter()));
        update_units_in(army, player.position.x, player.position.y, dt, walls, &player.route.field, armies.boosts[i], &player.squads, &drill);
        apply_champions(army, dt);
    }
}

//hurt units get better over time, each player's with their own relics
pub(crate) fn heal_armies(players: &Players, units: &mut [Unit], dt: f32) {
    let ranges = arrange(units, players.len());
    for (player, army) in players.list.iter().zip(split_armies(units, &ranges)) {
        heal_units(army, dt, &player.upgrades);
    }
}

//what the enemies a player's army destroyed this frame teach their champions
pub(crate) fn train_armies(players: &Players, units: &mut [Unit], fx: &mut Effects) {
    let ranges = arrange(units, players.len());
    for (i, army) in split_armies(units, &ranges).into_iter().enumerate() {
        train_champions(army, std::mem::take(&mut fx.spoils[i].xp), fx);
    }
}

//the health the enemies a player's army destroyed this frame give every unit in it
pub(crate) fn refresh_armies(players: &Players, units: &mut [Unit], fx: &mut Effects) {
    let ranges = arrange(units, players.len());
    for (i, army) in split_armies(units, &ranges).into_iter().enumerate() {
        heal_everyone(army, std::mem::take(&mut fx.spoils[i].heal_all));
    }
}

//the wall points each player has earned this frame: what their army's kills and the world gave them were worth (more with the Quarry relic),
//and what their quartermasters make
pub(crate) fn earn_wall_points(players: &mut Players, armies: &Armies, dt: f32, fx: &mut Effects) {
    for (i, player) in players.list.iter_mut().enumerate() {
        player.mason.earn(std::mem::take(&mut fx.spoils[i].points));
        player.mason.earn(armies.perks[i].points * dt);
    }
}

//babies grow into units for the player whose army they are in reach of. returns how far each player reaches (the screens are how big
//each player's window is, in pixels, which a big army reaches across)
pub(crate) fn grow_armies(players: &Players, armies: &Armies, units: &mut Vec<Unit>, babies: &mut Vec<Baby>, screens: &[Vec2], dt: f32, fx: &mut Effects) -> Vec<f32> {
    let ranges = arrange(units, players.len());
    let mut pickers: Vec<Picker> = players
        .list
        .iter()
        .zip(&ranges)
        .enumerate()
        .map(|(i, (player, range))| Picker {
            position: player.position,
            range: baby_pickup_range(range.len(), screens.get(i).copied().unwrap_or(DEFAULT_SCREEN)) * armies.perked[i].pickup,
            army: range.len(),
            cap: player.upgrades.unit_cap(),
        })
        .collect();
    grow_babies_among(babies, units, &mut pickers, dt, fx);
    pickers.iter().map(|picker| picker.range).collect()
}
