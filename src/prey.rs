//! the players as the enemies see them. each enemy goes after the player nearest to it, along the route to them, and what the enemies near a
//! player are made for is the army that player has together with the armies of the players near enough to be in the same fight

use crate::prelude::*;

//one player, for what the enemies do about them
pub(crate) struct Prey<'a> {
    pub(crate) position: Vec2,
    pub(crate) units: &'a [Unit],  //their army
    pub(crate) team: usize,        //how big a fight they are in: their army and those of the players near them
    pub(crate) pressure: usize,    //the size of army what is made for them is made for: the team, and more for the bosses that have been slain
    pub(crate) boost: f32,         //how many times faster than normal the enemies near them are, for the size of their army
    pub(crate) cap: usize,         //the most units their army can have
    pub(crate) field: &'a FlowField, //the route to them round the walls
}

impl Prey<'_> {
    pub(crate) fn army(&self) -> usize {
        self.units.len()
    }
}

//the place in the list of the player nearest to a place
pub(crate) fn nearest_to(prey: &[Prey], at: Vec2) -> usize {
    let mut best = 0;
    let mut best_distance = f32::MAX;
    for (i, other) in prey.iter().enumerate() {
        let distance = other.position.distance_squared(at);
        if distance < best_distance {
            best = i;
            best_distance = distance;
        }
    }
    best
}

pub(crate) fn nearest<'b, 'a>(prey: &'b [Prey<'a>], at: Vec2) -> &'b Prey<'a> {
    &prey[nearest_to(prey, at)]
}

//how big a fight each player is in: their army, and the armies of the players within ALLY_TILES of them. one alone is just their own
pub(crate) fn team_armies(positions: &[Vec2], armies: &[usize]) -> Vec<usize> {
    let reach = ALLY_TILES * TILE_SIZE;
    positions.iter().map(|&at| positions.iter().zip(armies).filter(|(other, _)| other.distance_squared(at) <= reach * reach).map(|(_, size)| size).sum()).collect()
}

//where each player is and the army the enemies made for them are made for
pub(crate) fn pressures(prey: &[Prey]) -> Vec<(Vec2, usize)> {
    prey.iter().map(|player| (player.position, player.pressure)).collect()
}
