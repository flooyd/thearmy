//! sending groups of units at an enemy and calling them back

use crate::prelude::*;

//the units sent at one enemy. they walk to where it is, round any walls on the way, by a flow field that leads to it
pub(crate) struct Squad {
    pub(crate) position: Vec2,        //where the enemy is now
    pub(crate) field: FlowField,      //leads to where the enemy was when it was built, near enough to get there from far away
    pub(crate) next_field: FlowField, //being built for where the enemy is now
}

//every squad, by the id of the enemy it was sent at
pub(crate) type Squads = HashMap<u32, Squad>;

//the enemy under a point in the world, if there is one: each counts as bigger all round by the margin, which makes small
//ones easier to hit. with several, the one whose middle is nearest. an ambusher lying in wait is not there to be found
pub(crate) fn enemy_at(point: Vec2, enemies: &[Enemy], margin: f32) -> Option<usize> {
    enemies
        .iter()
        .enumerate()
        .filter(|(_, enemy)| !enemy.lurking() && (point - enemy.position).abs().max_element() <= enemy.size / 2.0 + margin)
        .min_by(|a, b| a.1.position.distance_squared(point).total_cmp(&b.1.position.distance_squared(point)))
        .map(|(i, _)| i)
}

//how many units destroy an enemy in SQUAD_KILL_SECONDS, but never fewer than SQUAD_MIN_UNITS. a bigger, tougher enemy is
//sent a bigger squad, and one that has been hurt a smaller one
pub(crate) fn squad_size(enemy: &Enemy) -> usize {
    //a shield has to be worn down too, and armor means every hit counts for less
    let to_get_through = (enemy.health + enemy.shield) / (1.0 - enemy.armor).max(0.1);
    ((to_get_through / (UNIT_DAMAGE * SQUAD_KILL_SECONDS)).ceil() as usize).max(SQUAD_MIN_UNITS)
}

//send the units closest to an enemy, that haven't been sent at one already (and aren't champions, who stay with the player), to destroy it: as many as squad_size says, or
//every one there is if that is fewer. they join the squad already sent at it if there is one. returns how many were sent
pub(crate) fn send_squad(units: &mut [Unit], squads: &mut Squads, enemy: &Enemy, fx: &mut Effects) -> usize {
    let mut free: Vec<(f32, usize)> = units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.target.is_none() && unit.champion == 0)
        .map(|(i, unit)| (Vec2::new(unit.x, unit.y).distance_squared(enemy.position), i))
        .collect();
    let count = squad_size(enemy).min(free.len());
    if count == 0 {
        return 0;
    }
    //only the closest few need finding, so the rest don't need sorting
    if count < free.len() {
        free.select_nth_unstable_by(count - 1, |a, b| a.0.total_cmp(&b.0));
        free.truncate(count);
    }
    for &(_, i) in &free {
        units[i].target = Some(enemy.id);
        //head the right way now, not up to STEERING_INTERVAL from now
        units[i].steer_in = 0.0;
    }
    squads.entry(enemy.id).or_insert_with(|| Squad { position: enemy.position, field: FlowField::new(), next_field: FlowField::new() });
    fx.popup(enemy.position, enemy.size / 2.0, 0.0, format!("{count} sent"), SQUAD_COLOR);
    fx.cue(Cue::Dispatch);
    count
}

//a player's click on an enemy, which they name by its id: sends a squad at it. nothing happens if there is no such enemy any more, or it is
//an ambusher lying in wait, which can't be clicked on. returns how many were sent
pub(crate) fn send_squad_at(units: &mut [Unit], squads: &mut Squads, enemies: &[Enemy], id: u32, fx: &mut Effects) -> usize {
    match enemies.iter().find(|enemy| enemy.id == id && !enemy.lurking()) {
        Some(enemy) => send_squad(units, squads, enemy, fx),
        None => 0,
    }
}

//call every squad back: the units sent at enemies go back to following the player, which is how an army gets out of the way
//of something a boss is about to do. returns how many units were called back
pub(crate) fn recall_squads(units: &mut [Unit], squads: &mut Squads, player: Vec2, fx: &mut Effects) -> usize {
    let mut recalled = 0;
    for unit in units.iter_mut().filter(|unit| unit.target.is_some()) {
        unit.target = None;
        unit.steer_in = 0.0;
        recalled += 1;
    }
    squads.clear();
    if recalled > 0 {
        fx.popup(player, RADIUS * 2.0, 26.0, format!("{recalled} recalled"), SQUAD_COLOR);
        fx.cue(Cue::Dispatch);
    }
    recalled
}

//keep the squads up to date. each follows its enemy, and works out its route again once the enemy has moved on a few grid
//squares. one whose enemy has been destroyed is disbanded, and its units go back to following the player
#[cfg(test)]
pub(crate) fn update_squads(squads: &mut Squads, enemies: &[Enemy], units: &mut [Unit], walls: &Walls, walls_changed: bool) -> Vec<Vec2> {
    update_squads_within(squads, enemies, units, walls, walls_changed, SQUAD_ROUTE_BUDGET)
}

//the same with a limit on the time spent working out the squads' routes (a frame has SQUAD_ROUTE_BUDGET for the squads of all the players)
pub(crate) fn update_squads_within(squads: &mut Squads, enemies: &[Enemy], units: &mut [Unit], walls: &Walls, walls_changed: bool, budget: Duration) -> Vec<Vec2> {
    let started = Instant::now();
    //a squad with nobody left in it (they all died) is no more, either. where the enemy that wiped one out is gets returned
    let manned: HashSet<u32> = units.iter().filter_map(|unit| unit.target).collect();
    let mut wiped_out = Vec::new();
    squads.retain(|id, squad| {
        let Some(enemy) = enemies.iter().find(|enemy| enemy.id == *id) else {
            return false;
        };
        if !manned.contains(id) {
            wiped_out.push(enemy.position);
            return false;
        }
        squad.position = enemy.position;
        let tile = tile_of(enemy.position.x, enemy.position.y);
        let target = squad.field.target;
        let moved = squad.field.size == 0 || (tile.0 - target.0).abs().max((tile.1 - target.1).abs()) >= SQUAD_ROUTE_TILES;
        //changed walls make the route being built out of date so it starts again, an enemy that moved on waits for it to finish
        if walls_changed || (!squad.next_field.is_building() && moved) {
            squad.next_field.start(enemy.position, walls);
        }
        if squad.next_field.is_building() && squad.next_field.work(budget.saturating_sub(started.elapsed())) {
            std::mem::swap(&mut squad.field, &mut squad.next_field);
        }
        true
    });
    for unit in units.iter_mut() {
        if unit.target.is_some_and(|enemy| !squads.contains_key(&enemy)) {
            unit.target = None;
            unit.steer_in = 0.0;
        }
    }
    wiped_out
}

