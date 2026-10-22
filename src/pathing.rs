//! the flow field: the cheapest way to the player round the walls, built a little each frame

use crate::prelude::*;

//for every grid square near the player, what it costs to walk from there to the player. units follow it downhill.
//walls can't be walked through. a square with no way to the player (the player or the unit is walled in) instead leads
//to the squares of its own walled-off area that are closest to the player, the cost there counts distance from the
//player first and the walk second, so units that can't get to the player get as close as they can
pub(crate) struct FlowField {
    pub(crate) target: (i32, i32), //the grid square the player is on, which the costs lead to
    pub(crate) player: Vec2,       //where the player was when the field was started
    pub(crate) first_col: i32,     //the field covers a square of grid squares, this is its top left one
    pub(crate) first_row: i32,
    pub(crate) size: i32,
    pub(crate) cost: Vec<u32>,
    pub(crate) wall: Vec<bool>,  //squares that can't be walked through
    pub(crate) extra: Vec<u32>,  //what each other square costs to enter on top of the walk: a wall to chew through, spikes to avoid
    pub(crate) stage: Stage,
    pub(crate) open: BinaryHeap<Reverse<(u32, usize)>>, //squares still to be worked through while the field is being built
}

//the steps of building a field, in order
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Stage {
    Reaching,       //working outwards from the player over every square that has a way to the player
    Seeding(usize), //giving the squares that were not reached a cost for their distance from the player, from this one on
    Spreading,      //working outwards from those squares so the rest of their area leads to the closest of them
    Finished,
}

impl FlowField {
    pub(crate) fn new() -> FlowField {
        FlowField {
            target: (0, 0),
            player: Vec2::ZERO,
            first_col: 0,
            first_row: 0,
            size: 0,
            cost: Vec::new(),
            wall: Vec::new(),
            extra: Vec::new(),
            stage: Stage::Finished,
            open: BinaryHeap::new(),
        }
    }

    pub(crate) fn index(&self, col: i32, row: i32) -> Option<usize> {
        let (col, row) = (col - self.first_col, row - self.first_row);
        if (0..self.size).contains(&col) && (0..self.size).contains(&row) {
            Some((row * self.size + col) as usize)
        } else {
            None
        }
    }

    //whether a step from a grid square in the direction (dc, dr) is allowed. a diagonal step may not cut the corner of a wall
    pub(crate) fn can_step(&self, col: i32, row: i32, dc: i32, dr: i32) -> bool {
        if dc == 0 || dr == 0 {
            return true;
        }
        let open = |col: i32, row: i32| self.index(col, row).is_some_and(|i| !self.wall[i] && self.extra[i] == 0);
        open(col + dc, row) && open(col, row + dr)
    }

    //build the whole field at once
    #[cfg(test)]
    pub(crate) fn compute(&mut self, player: Vec2, walls: &Walls) {
        self.start(player, walls);
        self.work(Duration::MAX);
    }

    //whether the field is part way through being built, it can't be used until work has finished it
    pub(crate) fn is_building(&self) -> bool {
        self.stage != Stage::Finished
    }

    //begin building the costs towards the player for the army. building is split into steps so that a frame doesn't have
    //to wait for the whole field, call work until it says the field is finished
    pub(crate) fn start(&mut self, player: Vec2, walls: &Walls) {
        self.start_for(player, walls, Mover::Ally);
    }

    //begin building the costs towards a point for a mover. a wall that stops it but that it can break (an enemy's way
    //through the player's walls, the army's through a lair's) costs a lot to go through instead of being in the way, so
    //the way leads round a short wall and through a long one. a wall it can't break is in the way
    pub(crate) fn start_for(&mut self, player: Vec2, walls: &Walls, mover: Mover) {
        let target = tile_of(player.x, player.y);
        self.target = target;
        self.player = player;
        self.size = (FIELD_RADIUS * 2 + 1).min(WORLD_TILES);
        self.first_col = (target.0 - FIELD_RADIUS).clamp(0, WORLD_TILES - self.size);
        self.first_row = (target.1 - FIELD_RADIUS).clamp(0, WORLD_TILES - self.size);
        let cells = (self.size * self.size) as usize;
        self.wall.clear();
        self.wall.resize(cells, false);
        self.extra.clear();
        self.extra.resize(cells, 0);
        for (&(col, row), wall) in walls {
            let Some(i) = self.index(col, row) else {
                continue;
            };
            match wall.route_cost(mover) {
                None => self.wall[i] = true,
                Some(cost) => self.extra[i] = cost,
            }
        }
        self.cost.clear();
        self.cost.resize(cells, u32::MAX);

        let start = self.index(target.0, target.1).unwrap();
        self.cost[start] = 0;
        self.open.clear();
        self.open.push(Reverse((0u32, start)));
        self.stage = Stage::Reaching;
    }

    //carry on building the field for up to the given time. returns true once it is finished.
    //reaching and spreading are dijkstra outwards from the squares with a known cost, a step costs 10, or 14 diagonally
    pub(crate) fn work(&mut self, budget: Duration) -> bool {
        let started = Instant::now();
        let mut worked = 0;
        loop {
            match self.stage {
                Stage::Finished => break,
                Stage::Seeding(i) if i == self.cost.len() => self.stage = Stage::Spreading,
                Stage::Seeding(i) => {
                    //(a square that wasn't reached, but isn't a wall, has no way to the player)
                    if self.cost[i] == u32::MAX && !self.wall[i] {
                        self.cost[i] = self.distance_cost(i);
                        self.open.push(Reverse((self.cost[i], i)));
                    }
                    self.stage = Stage::Seeding(i + 1);
                }
                Stage::Reaching | Stage::Spreading => match self.open.pop() {
                    None if self.stage == Stage::Reaching => self.stage = Stage::Seeding(0),
                    None => self.stage = Stage::Finished,
                    //(otherwise this is an outdated entry, the square was reached more cheaply since it was added)
                    Some(Reverse((cost, i))) if cost <= self.cost[i] => self.reach_from(i, cost),
                    Some(_) => {}
                },
            }
            //asking the clock every time would cost more than it saves
            worked += 1;
            if worked % 64 == 0 && started.elapsed() >= budget {
                break;
            }
        }
        !self.is_building()
    }

    //give the squares around a square, which has the given cost, a cost if walking from them to it is cheaper
    //than what they had
    pub(crate) fn reach_from(&mut self, i: usize, cost: u32) {
        let col = (i % self.size as usize) as i32 + self.first_col;
        let row = (i / self.size as usize) as i32 + self.first_row;
        for (dc, dr) in NEIGHBORS {
            let Some(n) = self.index(col + dc, row + dr) else {
                continue;
            };
            if self.wall[n] || !self.can_step(col, row, dc, dr) {
                continue;
            }
            let step = (if dc != 0 && dr != 0 { 14 } else { 10 }) + self.extra[n];
            if cost + step < self.cost[n] {
                self.cost[n] = cost + step;
                self.open.push(Reverse((cost + step, n)));
            }
        }
    }

    //the cost of a square that can't reach the player: how far its closest point is from the player, in bands of
    //CLOSE_ENOUGH, each of which costs more than any walk through the field. so the squares a unit would walk to are
    //the ones in the closest band, and the walking only decides between those
    pub(crate) fn distance_cost(&self, i: usize) -> u32 {
        let left = ((i % self.size as usize) as i32 + self.first_col) as f32 * TILE_SIZE;
        let top = ((i / self.size as usize) as i32 + self.first_row) as f32 * TILE_SIZE;
        let closest = Vec2::new(self.player.x.clamp(left, left + TILE_SIZE), self.player.y.clamp(top, top + TILE_SIZE));
        (closest.distance(self.player) / CLOSE_ENOUGH) as u32 * CUT_OFF_STEP
    }

    //the neighbouring grid square that is the cheapest way on towards the target, None at the target itself, at the
    //closest square to the player of an area the player can't be reached from, or outside the field
    pub(crate) fn next_tile(&self, tile: (i32, i32)) -> Option<(i32, i32)> {
        let here = self.index(tile.0, tile.1)?;
        let mut best = None;
        let mut best_total = u32::MAX;
        for (dc, dr) in NEIGHBORS {
            let Some(n) = self.index(tile.0 + dc, tile.1 + dr) else {
                continue;
            };
            //only squares nearer the target, which also keeps the target itself from being walked away from. walls
            //never have a cost so they are never nearer
            if self.cost[n] >= self.cost[here] || !self.can_step(tile.0, tile.1, dc, dr) {
                continue;
            }
            //compare what the whole step costs, not just the cost beyond it, so a diagonal step has to be worth its length
            let step = if dc != 0 && dr != 0 { 14 } else { 10 };
            let total = self.cost[n] + step;
            if total < best_total {
                best_total = total;
                best = Some((tile.0 + dc, tile.1 + dr));
            }
        }
        best
    }
}

//where something that is `clearance` wide at a position should head to get to a goal: straight at it when it is near and nothing
//is in the way, otherwise along the flow field for a few grid squares, to the furthest of them that can be reached in a
//straight line, which cuts corners instead of following the centers of the squares
pub(crate) fn steer_along(position: Vec2, goal: Vec2, clearance: f32, mover: Mover, walls: &Walls, field: &FlowField) -> Steer {
    if position.distance(goal) <= DIRECT_CHASE_TILES * TILE_SIZE && path_clear(position, goal, clearance, mover, walls) {
        return Steer::Player;
    }
    let mut route = Vec::new();
    let mut tile = tile_of(position.x, position.y);
    while route.len() < LOOKAHEAD {
        let Some(next) = field.next_tile(tile) else {
            break;
        };
        route.push(next);
        tile = next;
    }
    for &tile in route.iter().rev() {
        let (steer, point) = if tile == field.target {
            (Steer::Player, goal)
        } else {
            (Steer::Point(tile_center(tile)), tile_center(tile))
        };
        if path_clear(position, point, clearance, mover, walls) {
            return steer;
        }
    }
    //nowhere left to go on the route: head for its first square. with no route at all the mover is outside the flow
    //field, or it is as close to a goal it can't reach as its area gets, either way it heads straight for the goal.
    //when a wall is in the way it slides along it to the closest spot
    route.first().map_or(Steer::Player, |&tile| Steer::Point(tile_center(tile)))
}

//the routes everything follows to the player, kept up to date a bit each frame: one for the army and one for the enemies,
//which differ in the walls they can't get through. each is built in `building` while the last finished one is followed
pub(crate) struct Routes {
    pub(crate) army: FlowField,
    pub(crate) foes: FlowField,
    building_army: FlowField,
    building_foes: FlowField,
}

impl Routes {
    //both routes built in full, for the player where they are
    pub(crate) fn new(player: Vec2, walls: &Walls) -> Routes {
        let mut routes = Routes { army: FlowField::new(), foes: FlowField::new(), building_army: FlowField::new(), building_foes: FlowField::new() };
        routes.army.start_for(player, walls, Mover::Ally);
        routes.army.work(Duration::MAX);
        routes.foes.start_for(player, walls, Mover::Enemy);
        routes.foes.work(Duration::MAX);
        routes
    }

    //a route starts again when walls changed, which makes the one being built out of date, or when the player has moved
    //to another grid square (two, for the enemies, who can do with less exact directions) and the last one is done
    pub(crate) fn update(&mut self, player: Vec2, walls: &Walls, walls_changed: bool) {
        let tile = tile_of(player.x, player.y);
        let far = |from: (i32, i32), limit: i32| (from.0 - tile.0).abs().max((from.1 - tile.1).abs()) >= limit;
        if walls_changed || (!self.building_army.is_building() && far(self.army.target, 1)) {
            self.building_army.start_for(player, walls, Mover::Ally);
        }
        if self.building_army.is_building() && self.building_army.work(FIELD_BUDGET) {
            std::mem::swap(&mut self.army, &mut self.building_army);
        }
        if walls_changed || (!self.building_foes.is_building() && far(self.foes.target, 2)) {
            self.building_foes.start_for(player, walls, Mover::Enemy);
        }
        if self.building_foes.is_building() && self.building_foes.work(FIELD_BUDGET / 2) {
            std::mem::swap(&mut self.foes, &mut self.building_foes);
        }
    }
}

#[cfg(test)]
impl Routes {
    //finish whatever is being built, so a test doesn't depend on how fast the machine is
    pub(crate) fn finish(&mut self) {
        if self.building_army.is_building() {
            self.building_army.work(Duration::MAX);
            std::mem::swap(&mut self.army, &mut self.building_army);
        }
        if self.building_foes.is_building() {
            self.building_foes.work(Duration::MAX);
            std::mem::swap(&mut self.foes, &mut self.building_foes);
        }
    }
}
