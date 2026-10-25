//! the army: how a unit moves, crowds, heals and dies, and the babies that grow into units

use crate::prelude::*;

//everything the units do in a frame: walk towards the player, shove each other apart and get pushed out of walls.
//the walls are dealt with from where each unit started the frame, so however hard a crowd shoves a bunched up unit
//it slides along the wall, rather than ending up past the middle of it and being pushed out the other side
#[cfg(test)]
pub(crate) fn update_units(units: &mut [Unit], player_x: f32, player_y: f32, dt: f32, walls: &Walls, field: &FlowField, speed_multiplier: f32, squads: &Squads) {
    update_units_in(units, player_x, player_y, dt, walls, field, speed_multiplier, squads, &Drill::FREE);
}

//the same, with the army standing in a formation if the drill says so: each unit walks to its place in the shape and stays there,
//so they have no need to be shoved apart, and each can be moved and settled in one go
pub(crate) fn update_units_in(units: &mut [Unit], player_x: f32, player_y: f32, dt: f32, walls: &Walls, field: &FlowField, speed_multiplier: f32, squads: &Squads, drill: &Drill) {
    if drill.formation != Formation::Free {
        let (player, army, near) = (Vec2::new(player_x, player_y), units.len(), NearWalls::new(walls));
        share_out(units, |_, units| {
            for unit in units.iter_mut().filter(|unit| unit.garrison.is_none()) {
                let from = Vec2::new(unit.x, unit.y);
                move_unit(unit, player, army, dt, walls, field, speed_multiplier, squads, drill);
                settle_unit(unit, from, dt, walls, &near);
            }
        });
        return;
    }
    let started: Vec<Vec2> = units.iter().map(|unit| Vec2::new(unit.x, unit.y)).collect();
    move_units_in(units, player_x, player_y, dt, walls, field, speed_multiplier, squads, drill);
    spread_units(units);
    settle_units(units, &started, dt, walls);
}

//push the units out of the walls, from where each one started the frame (`started`, in the same order), keep them in the
//world and work out how each has actually moved
pub(crate) fn settle_units(units: &mut [Unit], started: &[Vec2], dt: f32, walls: &Walls) {
    let near = NearWalls::new(walls);
    share_out(units, |first, units| {
        for (unit, &from) in units.iter_mut().zip(&started[first..]) {
            if unit.garrison.is_none() {
                settle_unit(unit, from, dt, walls, &near);
            }
        }
    });
}

//one unit pushed out of the walls from where it started the frame, kept in the world, and its velocity worked out. a unit that
//has no wall anywhere near has nothing to be pushed out of
fn settle_unit(unit: &mut Unit, from: Vec2, dt: f32, walls: &Walls, near: &NearWalls) {
    let to = Vec2::new(unit.x, unit.y);
    let clear = from.distance_squared(to) < TILE_SIZE * TILE_SIZE * 4.0 && !near.may_be_near(from) && !near.may_be_near(to);
    let settled = if clear { to } else { move_clear_of_walls(from, to, walls) };
    unit.x = settled.x;
    unit.y = settled.y;
    clamp_to_world(&mut unit.x, &mut unit.y);
    unit.velocity = smoothed_velocity(unit.velocity, from, Vec2::new(unit.x, unit.y), dt);
    unit.nose = turned_nose(unit.nose, unit.velocity, dt);
}

//the way a unit's nose points after a frame: turned a little towards the way the unit is going, or as it was if it is going slowly (so a unit standing in a
//crowd keeps the way it was pointing, and the shoving does not make it twitch). no trig, since every unit does this every frame
pub(crate) fn turned_nose(nose: Vec2, velocity: Vec2, dt: f32) -> Vec2 {
    let speed_squared = velocity.length_squared();
    if dt <= 0.0 || speed_squared < UNIT_TURN_MIN_SPEED * UNIT_TURN_MIN_SPEED {
        return nose;
    }
    let wanted = velocity / speed_squared.sqrt();
    let turn = (UNIT_TURN * dt).min(1.0);
    (nose * (1.0 - turn) + wanted * turn).try_normalize().unwrap_or(wanted)
}

//the velocity of something that moved from one position to another in dt seconds, blended into its last velocity so
//jitter (a crowd shoving, a wall pushing back) averages out and only steady movement points anywhere
pub(crate) fn smoothed_velocity(last: Vec2, from: Vec2, to: Vec2, dt: f32) -> Vec2 {
    if dt <= 0.0 {
        return last;
    }
    let blend = 1.0 - (-dt / VELOCITY_SMOOTHING).exp();
    last.lerp((to - from) / dt, blend)
}

//a unit sent at an enemy walks to where its squad says the enemy is, the rest follow the player. each unit works out where
//to head and steps on its own, only reading the walls and the routes, so the army is shared out over the cores
#[cfg(test)]
pub(crate) fn move_units(units: &mut [Unit], player_x: f32, player_y: f32, dt: f32, walls: &Walls, field: &FlowField, speed_multiplier: f32, squads: &Squads) {
    move_units_in(units, player_x, player_y, dt, walls, field, speed_multiplier, squads, &Drill::FREE);
}

//the same for an army that may be standing in a formation
pub(crate) fn move_units_in(units: &mut [Unit], player_x: f32, player_y: f32, dt: f32, walls: &Walls, field: &FlowField, speed_multiplier: f32, squads: &Squads, drill: &Drill) {
    let player = Vec2::new(player_x, player_y);
    let army = units.len();
    share_out(units, |_, units| {
        for unit in units.iter_mut().filter(|unit| unit.garrison.is_none()) {
            move_unit(unit, player, army, dt, walls, field, speed_multiplier, squads, drill);
        }
    });
}

//one unit's step of a frame (`army` is how many units there are, which the size of a formation depends on)
fn move_unit(unit: &mut Unit, player: Vec2, army: usize, dt: f32, walls: &Walls, field: &FlowField, speed_multiplier: f32, squads: &Squads, drill: &Drill) {
    match unit.target.and_then(|enemy| squads.get(&enemy)) {
        Some(squad) => step_unit(unit, squad.position, dt, walls, &squad.field, speed_multiplier),
        None => {
            unit.target = None;
            match drill.goal(unit, player, army) {
                Some(place) => step_unit_to(unit, place, FORMATION_REACH, dt, walls, field, speed_multiplier, true),
                None => step_unit(unit, player, dt, walls, field, speed_multiplier),
            }
        }
    }
}

//move a unit for one frame of dt seconds towards the player, walking round walls, but make it stop UNIT_FOLLOW_DISTANCE away.
//one that was sent at an enemy doesn't stop short, it walks right in to the middle of it, where the goal is the enemy.
//working out where to head is the most expensive thing a unit does, so it only does it every STEERING_INTERVAL, or
//when it gets to the point it was heading for, and walks the same way in between. its speed is its own times the
//multiplier the army's size gives
pub(crate) fn step_unit(unit: &mut Unit, player: Vec2, dt: f32, walls: &Walls, field: &FlowField, speed_multiplier: f32) {
    let stop = if unit.target.is_some() { 0.0 } else { UNIT_FOLLOW_DISTANCE };
    step_unit_to(unit, player, stop, dt, walls, field, speed_multiplier, false);
}

//the same for a unit going to any spot (`goal`), which it stops `stop` pixels short of. a unit in a formation runs faster the
//further behind its place it is (`catch_up`), otherwise the army would be left strung out behind a player who is on the move
pub(crate) fn step_unit_to(unit: &mut Unit, player: Vec2, stop: f32, dt: f32, walls: &Walls, field: &FlowField, speed_multiplier: f32, catch_up: bool) {
    let position = Vec2::new(unit.x, unit.y);
    let distance = position.distance(player);
    if distance <= stop {
        return;
    }
    let hurry = if catch_up { 1.0 + ((distance - stop) / FORMATION_CATCH_UP).min(FORMATION_CATCH_UP_MAX) } else { 1.0 };
    let step_length = unit.speed * speed_multiplier * hurry * dt;
    unit.steer_in -= dt;
    let arrived = matches!(unit.steer, Steer::Point(point) if position.distance(point) <= step_length);
    if unit.steer_in <= 0.0 || arrived {
        unit.steer = unit_steer(position, player, walls, field);
        unit.steer_in = STEERING_INTERVAL;
    }
    let target = match unit.steer {
        Steer::Player => player,
        Steer::Point(point) => point,
    };
    let to_target = target - position;
    //don't overshoot a target that is closer than one step, that would make the unit jitter. one heading for the player
    //stops UNIT_FOLLOW_DISTANCE short, so it can't go past that when its steps are long. it aims half a pixel inside
    //so that rounding can't leave it a hair outside, and never quite stopping
    let reach = match unit.steer {
        Steer::Player => to_target.length() - (stop - 0.5).max(0.0),
        Steer::Point(_) => to_target.length(),
    };
    let step = to_target.normalize_or_zero() * step_length.min(reach);
    unit.x += step.x;
    unit.y += step.y;
}

//where a unit at a position should head
pub(crate) fn unit_steer(position: Vec2, player: Vec2, walls: &Walls, field: &FlowField) -> Steer {
    steer_along(position, player, PATH_CLEARANCE, Mover::Ally, walls, field)
}

//spread units away from each other if they get too close to avoid them all being on top of each other.
//comparing every unit with every other gets slow with a lot of them, so they are sorted into a grid of cells as big as
//the minimum distance and each is only compared with the units in its own and the cells around it
pub(crate) fn spread_units(units: &mut [Unit]) {
    const NONE: u32 = u32::MAX;
    let min_distance = 32.0; // minimum distance between units
    if units.len() < 2 {
        return;
    }
    //the grid only covers the units
    let mut low = Vec2::new(units[0].x, units[0].y);
    let mut high = low;
    for unit in units.iter() {
        low = low.min(Vec2::new(unit.x, unit.y));
        high = high.max(Vec2::new(unit.x, unit.y));
    }
    let columns = ((high.x - low.x) / min_distance) as usize + 1;
    let rows = ((high.y - low.y) / min_distance) as usize + 1;
    let cells: Vec<(usize, usize)> = units
        .iter()
        .map(|unit| (((unit.x - low.x) / min_distance) as usize, ((unit.y - low.y) / min_distance) as usize))
        .collect();
    //each cell has a chain of the units in it: first[cell] is one of them and next[i] the one after unit i
    let mut first = vec![NONE; columns * rows];
    let mut next = vec![NONE; units.len()];
    for (i, &(column, row)) in cells.iter().enumerate() {
        next[i] = first[row * columns + column];
        first[row * columns + column] = i as u32;
    }

    for i in 0..units.len() {
        //a unit inside a tower stays where it is
        if units[i].garrison.is_some() {
            continue;
        }
        let (column, row) = cells[i];
        let mut looked = 0;
        'cells: for cell_row in row.saturating_sub(1)..=(row + 1).min(rows - 1) {
            for cell_column in column.saturating_sub(1)..=(column + 1).min(columns - 1) {
                let mut j = first[cell_row * columns + cell_column];
                while j != NONE {
                    //every pair is looked at once, from the unit that comes first
                    if j as usize > i && units[j as usize].garrison.is_none() {
                        //a big army crowded onto the player is dozens of units to a cell, and looking at every one of the
                        //hundreds round each unit took 20 ms a frame. a unit that is that crowded gets pushed by some of
                        //them, the rest do it next frame
                        looked += 1;
                        if looked > SPREAD_LOOKS {
                            break 'cells;
                        }
                        let j = j as usize;
                        let dx = units[j].x - units[i].x;
                        let dy = units[j].y - units[i].y;
                        let distance = (dx * dx + dy * dy).sqrt();
                        if distance < min_distance {
                            let overlap = (min_distance - distance) / 2.0;
                            //units exactly on top of each other have no direction to be pushed in, any will do
                            let (nx, ny) = if distance == 0.0 { (1.0, 0.0) } else { (dx / distance, dy / distance) };
                            units[i].x -= nx * overlap;
                            units[i].y -= ny * overlap;
                            units[j].x += nx * overlap;
                            units[j].y += ny * overlap;
                        }
                    }
                    j = next[j as usize];
                }
            }
        }
    }
}

//the units sorted into cells, for what has to find the ones near an enemy without looking at all of them: with hundreds of
//enemies and thousands of units, comparing every enemy with every unit was most of a frame. it remembers where the units were
//when it was made, so it is for work that doesn't move them
pub(crate) struct UnitGrid {
    low: Vec2,
    columns: usize,
    rows: usize,
    first: Vec<u32>, //first[cell] is one of the units in it, or NONE
    next: Vec<u32>,  //and next[i] the one after unit i
}

impl UnitGrid {
    const NONE: u32 = u32::MAX;

    pub(crate) fn new(units: &[Unit]) -> UnitGrid {
        //the grid only covers the units
        let mut low = units.first().map_or(Vec2::ZERO, |unit| Vec2::new(unit.x, unit.y));
        let mut high = low;
        for unit in units {
            low = low.min(Vec2::new(unit.x, unit.y));
            high = high.max(Vec2::new(unit.x, unit.y));
        }
        let columns = ((high.x - low.x) / GRID_CELL) as usize + 1;
        let rows = ((high.y - low.y) / GRID_CELL) as usize + 1;
        let mut first = vec![UnitGrid::NONE; columns * rows];
        let mut next = vec![UnitGrid::NONE; units.len()];
        for (i, unit) in units.iter().enumerate() {
            let cell = (((unit.y - low.y) / GRID_CELL) as usize) * columns + ((unit.x - low.x) / GRID_CELL) as usize;
            next[i] = first[cell];
            first[cell] = i as u32;
        }
        UnitGrid { low, columns, rows, first, next }
    }

    //adds to `found` the units that may be inside the box (every one that is, and some of the ones round it), in no order
    pub(crate) fn near(&self, from: Vec2, to: Vec2, found: &mut Vec<usize>) {
        let cell = |value: f32, low: f32, count: usize| (((value - low) / GRID_CELL).floor() as i64).clamp(-1, count as i64);
        let (first_column, last_column) = (cell(from.x, self.low.x, self.columns), cell(to.x, self.low.x, self.columns));
        let (first_row, last_row) = (cell(from.y, self.low.y, self.rows), cell(to.y, self.low.y, self.rows));
        //a box that is wholly to one side of the units has nothing in it
        if last_column < 0 || last_row < 0 || first_column >= self.columns as i64 || first_row >= self.rows as i64 {
            return;
        }
        for row in first_row.max(0)..=last_row.min(self.rows as i64 - 1) {
            for column in first_column.max(0)..=last_column.min(self.columns as i64 - 1) {
                let mut i = self.first[row as usize * self.columns + column as usize];
                while i != UnitGrid::NONE {
                    found.push(i as usize);
                    i = self.next[i as usize];
                }
            }
        }
    }
}

//white, or SQUAD_COLOR for a unit that has been sent at an enemy, or a champion's own colour
pub(crate) fn unit_base_color(unit: &Unit) -> Color {
    if let Some(kind) = unit.champion_kind() {
        kind.color
    } else if unit.target.is_some() {
        SQUAD_COLOR
    } else {
        WHITE
    }
}

//a unit's colour, going red the more it is hurt
pub(crate) fn unit_color(unit: &Unit) -> Color {
    unit_color_in(unit, None)
}

//a unit's colour in a game with others, where the army of each player is tinted towards the colour of the player (a champion is not: it has its own, and a ring
//in its player's, see team.rs)
pub(crate) fn unit_color_in(unit: &Unit, team: Option<&[Color]>) -> Color {
    let mut base = unit_base_color(unit);
    if let Some(colors) = team.filter(|_| unit.champion == 0) {
        base = team_color(base, colors[owner_index(unit.owner).min(colors.len() - 1)]);
    }
    if unit.guard > 0.0 {
        //a shield tints it towards the accent colour
        base = Color::new(base.r + (CYAN.r - base.r) * 0.45, base.g + (CYAN.g - base.g) * 0.45, base.b + (CYAN.b - base.b) * 0.45, 1.0);
    }
    let most = unit.max_health();
    if unit.health >= most {
        return base;
    }
    let hurt = (1.0 - unit.health / most).clamp(0.0, 1.0) * 0.75;
    let wound = wound_color();
    Color::new(base.r + (wound.r - base.r) * hurt, base.g + (wound.g - base.g) * hurt, base.b + (wound.b - base.b) * hurt, 1.0)
}

//a unit with a random speed, which is what the player starts with and what a baby unit grows into
pub(crate) fn new_unit(position: Vec2) -> Unit {
    Unit { age: 0.0, ..Unit::new(position.x, position.y, rand::gen_range(UNIT_MIN_SPEED, UNIT_MAX_SPEED)) }
}

//the units the player starts with, close to them but not as close as a unit following them gets
pub(crate) fn starting_units(player_x: f32, player_y: f32) -> Vec<Unit> {
    let count = rand::gen_range(START_UNITS_MIN, START_UNITS_MAX + 1);
    (0..count)
        .map(|_| new_unit(random_position_around(player_x, player_y, UNIT_SPAWN_MIN_DISTANCE, UNIT_SPAWN_MAX_DISTANCE)))
        .collect()
}

//how many times faster than normal the player, the units and the enemies move for an army of this many units: not at
//all for the smallest, then a little more for each unit, up to MAX_SPEED_MULTIPLIER at SPEED_BOOST_UNITS
pub(crate) fn speed_multiplier(army: usize) -> f32 {
    speed_multiplier_with(army, 0.0)
}

//the same with the most it can reach raised by `extra`, which an army of SPEED_BOOST_UNITS gets to and a smaller one gets
//its share of, so the relics that raise it help at any size
pub(crate) fn speed_multiplier_with(army: usize, extra: f32) -> f32 {
    1.0 + (MAX_SPEED_MULTIPLIER + extra - 1.0) * (army as f32 / SPEED_BOOST_UNITS as f32).min(1.0)
}

//how close the player has to be to a baby unit to make it grow. it starts at BABY_PICKUP_RANGE and grows by the same
//factor for every unit, so it keeps feeling bigger, until at PICKUP_FULL_UNITS it reaches from the middle of the screen
//to its corner when zoomed all the way out, which is every baby that can be seen. the screen size is in pixels
pub(crate) fn baby_pickup_range(army: usize, screen: Vec2) -> f32 {
    let screen_reach = (screen.length() / 2.0 / MIN_ZOOM).max(BABY_PICKUP_RANGE);
    let progress = (army.saturating_sub(1) as f32 / (PICKUP_FULL_UNITS - 1) as f32).min(1.0);
    BABY_PICKUP_RANGE * (screen_reach / BABY_PICKUP_RANGE).powf(progress)
}

pub(crate) static NEXT_UNIT_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Clone)]
pub(crate) struct Unit {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) speed: f32,         //pixels per second
    pub(crate) steer: Steer,       //where it is heading
    pub(crate) steer_in: f32,      //seconds until it works out where to head again
    pub(crate) velocity: Vec2,     //pixels per second, how it has actually been moving, slower than speed when something is in the way
    pub(crate) nose: Vec2,         //a unit vector, the way it points when it is drawn as a ship: it turns to the way the unit is going and is left alone while it is hardly moving
    pub(crate) target: Option<u32>, //the enemy (its id) it was sent at, None when it is following the player
    pub(crate) health: f32,        //it dies at 0, and heals up to UNIT_HEALTH
    pub(crate) garrison: Option<(i32, i32)>, //the tower (its grid square) it is inside, where it stays safe and shoots
    pub(crate) guard: f32,         //points of shield (from the Aegis relic) that take hits before its health does
    pub(crate) calm: f32,          //seconds since it was last hurt, a shield comes back after a quiet while
    pub(crate) age: f32,           //seconds since it grew from a baby, for the swell and the flash it starts with
    pub(crate) champion: u8,       //0 for an ordinary unit, otherwise which of the CHAMPIONS it is, plus one
    pub(crate) power: f32,         //a champion's timer: seconds until its power goes off, or until it can rise again
    pub(crate) might: f32,         //how many units' damage it does by touch, worked out each frame by apply_champions
    pub(crate) ward: f32,          //the share of the damage that reaches it, likewise, 1 for an ordinary unit
    pub(crate) hexed: f32,         //seconds left of a hexer's curse on it, while which it takes HEX_EXTRA more harm
    pub(crate) level: u8,          //0 for an ordinary unit, otherwise the champion's level, from 1
    pub(crate) xp: f32,            //a champion's experience towards its next level
    pub(crate) evolutions: u8,     //a champion's evolutions: two bits for each of EVOLUTION_LEVELS, 0 for none chosen yet, 1 or 2 for the way chosen
    pub(crate) slot: u32,          //where it stands in a formation, as two random numbers (see `slot_coords`) that it keeps for good
    pub(crate) id: u32,           //different for every unit ever made, which is how a machine that is shown the army (see `Viewer`) tells one from another as they come and go
    pub(crate) owner: u8,          //which player's army it is in (their index in `Players`), 0 when there is only one
}

//where a unit is heading
#[derive(Clone, Copy)]
pub(crate) enum Steer {
    Player,      //straight at the player, wherever they are now
    Point(Vec2), //a point in the world
}

impl Unit {
    //take damage (already made less by a tough unit), and less again if a champion or a shield from one is looking after it: the
    //shield from the Aegis relic takes it first
    pub(crate) fn hurt(&mut self, damage: f32) {
        self.calm = 0.0;
        let damage = damage * self.ward * if self.hexed > 0.0 { 1.0 + HEX_EXTRA } else { 1.0 };
        let soaked = damage.min(self.guard);
        self.guard -= soaked;
        self.health -= damage - soaked;
    }

    pub(crate) fn new(x: f32, y: f32, speed: f32) -> Unit {
        //each unit starts its steering timer at a different point so they don't all work out where to head in the same frame
        Unit {
            x,
            y,
            speed,
            steer: Steer::Player,
            steer_in: rand::gen_range(0.0, STEERING_INTERVAL),
            velocity: Vec2::ZERO,
            nose: Vec2::new(0.0, -1.0),
            target: None,
            health: UNIT_HEALTH,
            garrison: None,
            guard: 0.0,
            calm: 99.0,
            age: 99.0,
            champion: 0,
            power: 0.0,
            might: 1.0,
            ward: 1.0,
            hexed: 0.0,
            level: 0,
            xp: 0.0,
            evolutions: 0,
            slot: rand::rand(),
            id: NEXT_UNIT_ID.fetch_add(1, Ordering::Relaxed),
            owner: 0,
        }
    }

    //the same unit in another player's army
    pub(crate) fn owned_by(self, owner: usize) -> Unit {
        Unit { owner: owner as u8, ..self }
    }
}

//hurt units get better over time, slowly
pub(crate) fn heal_units(units: &mut [Unit], dt: f32, upgrades: &Upgrades) {
    for unit in units.iter_mut() {
        unit.calm += dt;
        unit.age += dt;
        unit.hexed = (unit.hexed - dt).max(0.0);
        if upgrades.guard > 0.0 && unit.calm >= AEGIS_CALM_SECONDS && unit.guard < upgrades.guard {
            unit.guard = upgrades.guard;
        }
        let most = unit.max_health();
        if unit.health < most {
            unit.health = (unit.health + UNIT_HEAL_RATE * upgrades.healing * dt).min(most);
        }
    }
}

//units that have run out of health die, in a puff of their colour. returns how many. a champion that can rise again does, in the
//place where it fell, and one that can't is mourned
pub(crate) fn bury_units(units: &mut Vec<Unit>, fx: &mut Effects) -> usize {
    let before = units.len();
    let mut somewhere = None;
    units.retain_mut(|unit| {
        if unit.health > 0.0 {
            return true;
        }
        if let Some(kind) = unit.champion_kind() {
            if let Power::Rebirth { recharge, .. } = kind.power.grown(unit.level, &unit.growth()) {
                if unit.power <= 0.0 {
                    unit.health = unit.max_health();
                    unit.guard = 0.0;
                    unit.power = recharge;
                    fx.rebirths.push((Vec2::new(unit.x, unit.y), unit.champion, unit.level, unit.evolutions, unit.owner));
                    return true;
                }
            }
            fx.announce(format!("{} HAS FALLEN", kind.name.to_uppercase()), ALERT);
            fx.cue(Cue::Fall);
            fx.add_shake(0.3);
        }
        fx.burst(Vec2::new(unit.x, unit.y), unit_base_color(unit), 6, 200.0);
        fx.fallen.push((Vec2::new(unit.x, unit.y), unit.owner));
        somewhere = Some(Vec2::new(unit.x, unit.y));
        false
    });
    let lost = before - units.len();
    if let Some(at) = somewhere {
        fx.cue_at(Cue::Death, at);
    }
    lost
}

//a green baby unit, dropped by a destroyed enemy, that stays where it is until the player comes near
#[derive(Clone)]
pub(crate) struct Baby {
    pub(crate) position: Vec2,
    pub(crate) age: f32, //seconds since it dropped
}

//a player who babies can grow for: where they are, how far they reach, and how big their army is now and can be
pub(crate) struct Picker {
    pub(crate) position: Vec2,
    pub(crate) range: f32,
    pub(crate) army: usize,
    pub(crate) cap: usize,
}

//babies grow into units of the army of the nearest player they are within range of (and whose army has room), once they have been
//there a moment. a baby that no one can take, because it is out of reach or the armies in reach are full, waits, and one that has waited
//BABY_LIFE seconds is gone. the new units are put at the end of the list, so the units are out of order of owner until they are
//sorted again (see `sort_by_owner`)
pub(crate) fn grow_babies_among(babies: &mut Vec<Baby>, units: &mut Vec<Unit>, pickers: &mut [Picker], dt: f32, fx: &mut Effects) {
    babies.retain_mut(|baby| {
        baby.age += dt;
        if baby.age >= BABY_LIFE {
            return false;
        }
        if baby.age < BABY_PICKUP_DELAY {
            return true;
        }
        let taker = pickers
            .iter()
            .enumerate()
            .filter(|(_, picker)| picker.army < picker.cap && baby.position.distance(picker.position) <= picker.range)
            .min_by(|a, b| baby.position.distance_squared(a.1.position).total_cmp(&baby.position.distance_squared(b.1.position)))
            .map(|(i, _)| i);
        let Some(owner) = taker else {
            return true;
        };
        units.push(new_unit(baby.position).owned_by(owner));
        pickers[owner].army += 1;
        fx.grown[owner.min(MAX_PLAYERS - 1)] += 1;
        fx.burst(baby.position, pickup_color(), 5, 140.0);
        fx.cue(Cue::Pop);
        false
    });
}

//the same for one player, whose army is all the units there are
#[cfg(test)]
pub(crate) fn grow_babies(babies: &mut Vec<Baby>, units: &mut Vec<Unit>, player_x: f32, player_y: f32, dt: f32, range: f32, cap: usize, fx: &mut Effects) {
    let mut picker = [Picker { position: Vec2::new(player_x, player_y), range, army: units.len(), cap }];
    grow_babies_among(babies, units, &mut picker, dt, fx);
}

//babies are small circles (green, unless the colours are changed) with a ring round them. they are drawn over the units, which are standing on the
//enemy that dropped them, so they can be seen. only the ones in view (with a ring's width to spare) are made into discs
pub(crate) fn baby_discs(babies: &[Baby], view: Rect) -> Vec<Disc> {
    let mut discs = Vec::new();
    let pickup = pickup_color();
    for baby in babies.iter().filter(|baby| view.contains(baby.position)) {
        //pops out from nothing when it drops
        let grown = (baby.age / 0.15).min(1.0);
        let color = Color { a: pickup.a * baby_blink(baby.age), ..pickup };
        discs.push(Disc::new(baby.position.x, baby.position.y, BABY_RADIUS * grown, color));
        discs.push(Disc::ring(baby.position.x, baby.position.y, (BABY_RADIUS + 4.0) * grown, 2.0, color));
    }
    discs
}

//how much of its colour a baby shows: all of it, until the last BABY_BLINK_SECONDS of its life, when it flickers between
//full and faint, quicker and fainter as the time runs out
pub(crate) fn baby_blink(age: f32) -> f32 {
    let left = BABY_LIFE - age;
    if left >= BABY_BLINK_SECONDS {
        return 1.0;
    }
    let closing = (1.0 - left / BABY_BLINK_SECONDS).clamp(0.0, 1.0);
    //the blinks a second climb from 1.5 to 5 over the time, so the phase is that rate added up
    let blinking = closing * BABY_BLINK_SECONDS;
    let phase = 1.5 * blinking + 0.5 * (3.5 / BABY_BLINK_SECONDS) * blinking * blinking;
    let beat = (phase * std::f32::consts::TAU).cos() * 0.5 + 0.5;
    1.0 - (1.0 - beat) * (0.5 + 0.4 * closing)
}


impl Unit {
    //a unit as another machine shows it: what it is and where, and nothing of what it is doing (which only the host's world needs). it takes nothing from
    //the random numbers or the ids, so showing an army does not change what the host's world does
    pub(crate) fn replica(id: u32, owner: usize, at: Vec2) -> Unit {
        Unit {
            x: at.x,
            y: at.y,
            speed: (UNIT_MIN_SPEED + UNIT_MAX_SPEED) / 2.0,
            steer: Steer::Player,
            steer_in: 0.0,
            velocity: Vec2::ZERO,
            nose: Vec2::new(0.0, -1.0),
            target: None,
            health: UNIT_HEALTH,
            garrison: None,
            guard: 0.0,
            calm: 99.0,
            age: 99.0,
            champion: 0,
            power: 0.0,
            might: 1.0,
            ward: 1.0,
            hexed: 0.0,
            level: 0,
            xp: 0.0,
            evolutions: 0,
            slot: 0,
            id,
            owner: owner as u8,
        }
    }
}
