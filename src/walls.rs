//! walls: the kinds there are and who each stops, collision against them, wearing them down and drawing them

use crate::prelude::*;

//who is trying to get past a wall
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mover {
    Ally,  //the player and the army
    Enemy,
}

//who raised a wall
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Friendly, //the player's: enemies chew through it
    Hostile,  //a lair's or a boss's: the army has to break it
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WallKind {
    Wall,    //stops everyone
    Gate,    //stops enemies, the player and the army walk through it
    Tower,   //stops everyone, and units can man it to shoot from behind it
    Spikes,  //stops nobody, hurts the enemies standing on it, and wears out
    Tar,     //stops nobody, slows the enemies standing in it
    Barrier, //what a boss raises: stops the army, not enemies, and crumbles by itself
}

//what the player can build, in the order of the keys that choose them
pub(crate) const BUILDABLE: [WallKind; 5] = [WallKind::Wall, WallKind::Gate, WallKind::Tower, WallKind::Spikes, WallKind::Tar];

impl WallKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            WallKind::Wall => "Wall",
            WallKind::Gate => "Gate",
            WallKind::Tower => "Tower",
            WallKind::Spikes => "Spikes",
            WallKind::Tar => "Tar",
            WallKind::Barrier => "Barrier",
        }
    }

    pub(crate) fn blurb(self) -> &'static str {
        match self {
            WallKind::Wall => "stops everyone",
            WallKind::Gate => "stops enemies, not you",
            WallKind::Tower => "[G] man it to shoot",
            WallKind::Spikes => "hurts enemies on it",
            WallKind::Tar => "slows enemies in it",
            WallKind::Barrier => "raised by a boss",
        }
    }

    //wall points to build one
    pub(crate) fn cost(self) -> f32 {
        match self {
            WallKind::Wall => 1.0,
            WallKind::Gate => 2.0,
            WallKind::Tower => 8.0,
            WallKind::Spikes => 2.0,
            WallKind::Tar => 1.0,
            WallKind::Barrier => 0.0,
        }
    }

    //how many times WALL_HEALTH it has
    pub(crate) fn health_factor(self) -> f32 {
        match self {
            WallKind::Wall => 1.0,
            WallKind::Gate => 0.6,
            WallKind::Tower => 3.0,
            WallKind::Spikes | WallKind::Tar => 0.0,
            WallKind::Barrier => 1.2,
        }
    }

    pub(crate) fn color(self) -> Color {
        match self {
            WallKind::Wall => BLUE,
            WallKind::Gate => Color::new(0.85, 0.65, 0.2, 1.0),
            WallKind::Tower => Color::new(0.55, 0.6, 0.72, 1.0),
            WallKind::Spikes => Color::new(0.7, 0.72, 0.78, 1.0),
            WallKind::Tar => Color::new(0.25, 0.1, 0.35, 1.0),
            WallKind::Barrier => Color::new(0.95, 0.2, 0.45, 1.0),
        }
    }

    //spikes and tar are on the floor: things walk over them, so they are drawn under everything
    pub(crate) fn is_floor(self) -> bool {
        matches!(self, WallKind::Spikes | WallKind::Tar)
    }
}

//the health a wall of a kind is built with for an army of this many units. walls get tougher as the enemies do, and a
//wall built for a small army is weak against a big one until it is repaired
pub(crate) fn wall_health(kind: WallKind, army: usize) -> f32 {
    match kind {
        WallKind::Spikes | WallKind::Tar => SPIKE_HEALTH,
        _ => WALL_HEALTH * kind.health_factor() * army_scale(army),
    }
}

//a wall fills one grid square, keyed by that square's (col, row) in Walls
#[derive(Clone)]
pub(crate) struct Wall {
    pub(crate) col: i32,
    pub(crate) row: i32,
    pub(crate) size: f32,
    pub(crate) color: Color,
    pub(crate) kind: WallKind,
    pub(crate) side: Side,
    pub(crate) health: f32,
    pub(crate) max_health: f32,
    pub(crate) life: f32,     //seconds before it crumbles by itself, infinite for the ones that stay
    pub(crate) hurt: f32,     //seconds since it was hit, for the flash
    pub(crate) garrison: usize, //units inside a tower
    pub(crate) fire_in: f32,  //seconds until a tower shoots
}

//walls (and other things kept by grid square) are looked up many thousands of times a frame by the units and the
//enemies, so they use a hash that is quick to work out instead of one that is hard to attack, which nothing here needs
#[derive(Default, Clone, Copy)]
pub(crate) struct TileHasher(u64);

impl std::hash::Hasher for TileHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_u64(byte as u64);
        }
    }

    fn write_i32(&mut self, number: i32) {
        self.write_u64(number as u32 as u64);
    }

    fn write_u64(&mut self, number: u64) {
        self.0 = (self.0.rotate_left(5) ^ number).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

pub(crate) type TileMap<V> = HashMap<(i32, i32), V, std::hash::BuildHasherDefault<TileHasher>>;
pub(crate) type Walls = TileMap<Wall>;

impl Wall {
    //an ordinary wall of the player's
    pub(crate) fn new(col: i32, row: i32) -> Wall {
        Wall::made(col, row, WallKind::Wall, Side::Friendly, WALL_HEALTH)
    }

    pub(crate) fn made(col: i32, row: i32, kind: WallKind, side: Side, health: f32) -> Wall {
        let color = match side {
            Side::Friendly => kind.color(),
            Side::Hostile if kind == WallKind::Barrier => kind.color(),
            Side::Hostile => Color::new(0.42, 0.1, 0.2, 1.0),
        };
        Wall { col, row, size: TILE_SIZE, color, kind, side, health, max_health: health, life: f32::INFINITY, hurt: WALL_FLASH, garrison: 0, fire_in: TOWER_INTERVAL }
    }

    pub(crate) fn rect(&self) -> Rect {
        Rect::new(self.col as f32 * TILE_SIZE, self.row as f32 * TILE_SIZE, self.size, self.size)
    }

    pub(crate) fn center(&self) -> Vec2 {
        tile_center((self.col, self.row))
    }

    //whether it stops a mover
    pub(crate) fn blocks(&self, mover: Mover) -> bool {
        match (self.kind, self.side, mover) {
            (WallKind::Spikes | WallKind::Tar, _, _) => false,
            (WallKind::Gate, _, mover) => mover == Mover::Enemy,
            (_, Side::Friendly, _) => true,
            (_, Side::Hostile, mover) => mover == Mover::Ally,
        }
    }

    //whether it stops what enemies shoot
    pub(crate) fn stops_shots(&self) -> bool {
        self.side == Side::Friendly && !self.kind.is_floor()
    }

    //what a route through it costs a mover: None if it can't be done, Some(0) for an open square, otherwise how much the
    //chewing through or the avoiding of it counts for
    pub(crate) fn route_cost(&self, mover: Mover) -> Option<u32> {
        match (mover, self.side) {
            (Mover::Ally, Side::Friendly) if self.blocks(mover) => None,
            (Mover::Ally, Side::Hostile) if self.blocks(mover) => Some(UNIT_CHEW_COST),
            (Mover::Enemy, Side::Friendly) if self.blocks(mover) => Some(match self.kind {
                WallKind::Gate => GATE_CHEW_COST,
                WallKind::Tower => TOWER_CHEW_COST,
                _ => ENEMY_CHEW_COST,
            }),
            (Mover::Enemy, Side::Friendly) if self.kind.is_floor() => Some(FLOOR_AVOID_COST),
            _ => Some(0),
        }
    }

    pub(crate) fn share_left(&self) -> f32 {
        (self.health / self.max_health.max(1.0)).clamp(0.0, 1.0)
    }

    //the point on the wall that is closest to the given position
    pub(crate) fn nearest_point(&self, x: f32, y: f32) -> Vec2 {
        let rect = self.rect();
        Vec2::new(x.clamp(rect.x, rect.x + rect.w), y.clamp(rect.y, rect.y + rect.h))
    }

    //whether the point of this wall closest to a position is a corner with another wall right up against it, which
    //makes it not a corner of the walls as a whole. a circle pushed away from such a corner catches where two walls of
    //a row meet instead of sliding past, the flat side of the other wall is what should push it
    pub(crate) fn is_hidden_corner(&self, x: f32, y: f32, mover: Mover, walls: &Walls) -> bool {
        let rect = self.rect();
        let sideways = if x < rect.x {
            -1
        } else if x > rect.x + rect.w {
            1
        } else {
            return false;
        };
        let vertical = if y < rect.y {
            -1
        } else if y > rect.y + rect.h {
            1
        } else {
            return false;
        };
        blocks_at(walls, self.col + sideways, self.row, mover) || blocks_at(walls, self.col, self.row + vertical, mover)
    }

    pub(crate) fn overlaps_circle(&self, x: f32, y: f32, radius: f32) -> bool {
        (Vec2::new(x, y) - self.nearest_point(x, y)).length() < radius
    }

    //how far a circle of a radius at the given position has to move to stop overlapping the wall, zero if it doesn't
    pub(crate) fn push_out(&self, x: f32, y: f32, radius: f32, mover: Mover, walls: &Walls) -> Vec2 {
        let offset = Vec2::new(x, y) - self.nearest_point(x, y);
        let distance = offset.length();
        if distance >= radius {
            return Vec2::ZERO;
        }
        if distance > 0.0 {
            return offset / distance * (radius - distance);
        }
        //the center is inside the wall, leave through the nearest side. a side with another wall against it doesn't
        //lead out, only into that wall, so those are only used when every side has one
        let rect = self.rect();
        let sides = [
            (x - rect.x, (-1, 0)),
            (rect.x + rect.w - x, (1, 0)),
            (y - rect.y, (0, -1)),
            (rect.y + rect.h - y, (0, 1)),
        ];
        let mut exits: Vec<_> = sides.into_iter().filter(|&(_, (dc, dr))| !blocks_at(walls, self.col + dc, self.row + dr, mover)).collect();
        if exits.is_empty() {
            exits = sides.to_vec();
        }
        let (depth, (dc, dr)) = exits.into_iter().min_by(|a, b| a.0.total_cmp(&b.0)).unwrap();
        Vec2::new(dc as f32, dr as f32) * (depth + radius)
    }

    //take damage, returns whether it is destroyed
    pub(crate) fn damage(&mut self, amount: f32) -> bool {
        self.health -= amount;
        self.hurt = 0.0;
        self.health <= 0.0
    }
}

//whether there is a wall on a grid square that stops a mover
pub(crate) fn blocks_at(walls: &Walls, col: i32, row: i32, mover: Mover) -> bool {
    walls.get(&(col, row)).is_some_and(|wall| wall.blocks(mover))
}

//walls block the mover: push a circle out of any wall it overlaps, which makes it slide along the wall
pub(crate) fn push_out_of(x: &mut f32, y: &mut f32, radius: f32, mover: Mover, walls: &Walls) {
    if walls.is_empty() {
        return;
    }
    //a circle is smaller than a grid square so only the squares around it can have a wall touching it
    let (col, row) = tile_of(*x, *y);
    //two passes so being pushed out of one wall into its neighbour (like in a corner) is resolved too, though the
    //second is only needed if the first pushed anything
    for _ in 0..2 {
        let mut pushed = false;
        for wall_col in (col - 1)..=(col + 1) {
            for wall_row in (row - 1)..=(row + 1) {
                let Some(wall) = walls.get(&(wall_col, wall_row)).filter(|wall| wall.blocks(mover)) else {
                    continue;
                };
                if wall.is_hidden_corner(*x, *y, mover, walls) {
                    continue;
                }
                let push = wall.push_out(*x, *y, radius, mover, walls);
                *x += push.x;
                *y += push.y;
                pushed |= push != Vec2::ZERO;
            }
        }
        if !pushed {
            break;
        }
    }
}

//the player and the units are pushed out of the walls that stop them
#[cfg(test)]
pub(crate) fn push_out_of_walls(x: &mut f32, y: &mut f32, walls: &Walls) {
    push_out_of(x, y, RADIUS, Mover::Ally, walls);
}

//move a circle in a straight line from one position to another, being pushed out of walls on the way. it moves in small
//steps because a circle that gets more than half way into a wall is pushed out of the far side, which is how fast
//movement, or a crowd shoving, would otherwise carry it through
pub(crate) fn move_clear(from: Vec2, to: Vec2, radius: f32, mover: Mover, walls: &Walls) -> Vec2 {
    let steps = (from.distance(to) / MAX_STEP).ceil().max(1.0);
    let step = (to - from) / steps;
    let mut position = from;
    for _ in 0..steps as i32 {
        position += step;
        push_out_of(&mut position.x, &mut position.y, radius, mover, walls);
    }
    position
}

pub(crate) fn move_clear_of_walls(from: Vec2, to: Vec2, walls: &Walls) -> Vec2 {
    move_clear(from, to, RADIUS, Mover::Ally, walls)
}

//whether a circle touches any wall that stops a mover
pub(crate) fn touches_blocking(x: f32, y: f32, radius: f32, mover: Mover, walls: &Walls) -> bool {
    let (first_col, first_row) = tile_of(x - radius, y - radius);
    let (last_col, last_row) = tile_of(x + radius, y + radius);
    for col in first_col..=last_col {
        for row in first_row..=last_row {
            if walls.get(&(col, row)).is_some_and(|wall| wall.blocks(mover) && wall.overlaps_circle(x, y, radius)) {
                return true;
            }
        }
    }
    false
}

//whether a circle touches any wall that stops the army
#[cfg(test)]
pub(crate) fn touches_wall(x: f32, y: f32, radius: f32, walls: &Walls) -> bool {
    touches_blocking(x, y, radius, Mover::Ally, walls)
}

//whether a circle can move in a straight line from one point to another without touching a wall that stops it
pub(crate) fn path_clear(from: Vec2, to: Vec2, radius: f32, mover: Mover, walls: &Walls) -> bool {
    //checking every few pixels is enough because the circle is bigger than the gaps between the checks
    let steps = (from.distance(to) / 8.0).ceil().max(1.0) as i32;
    (0..=steps).all(|step| {
        let point = from.lerp(to, step as f32 / steps as f32);
        !touches_blocking(point.x, point.y, radius, mover, walls)
    })
}

//the edge of a square of walls START_HOUSE_TILES grid squares along each side, with the middle of the world, where the
//player starts, in the middle of it
pub(crate) fn starting_house() -> Walls {
    let first = WORLD_TILES / 2 - START_HOUSE_TILES / 2;
    let last = first + START_HOUSE_TILES - 1;
    let mut walls = Walls::default();
    for col in first..=last {
        for row in first..=last {
            if col == first || col == last || row == first || row == last {
                walls.insert((col, row), Wall::new(col, row));
            }
        }
    }
    walls
}

//the hits walls took in a frame become damage: the ones that gave way are gone, with what was in them let out, and so are
//the ones whose time is up. blasts are turned into hits on the player's walls in their reach first. returns whether any
//wall went, which makes every route out of date
pub(crate) fn update_walls(walls: &mut Walls, units: &mut [Unit], dt: f32, fx: &mut Effects) -> bool {
    let mut broken: Vec<(i32, i32)> = Vec::new();
    for blast in std::mem::take(&mut fx.blasts) {
        let (first_col, first_row) = tile_of(blast.position.x - blast.radius, blast.position.y - blast.radius);
        let (last_col, last_row) = tile_of(blast.position.x + blast.radius, blast.position.y + blast.radius);
        for col in first_col..=last_col {
            for row in first_row..=last_row {
                if let Some(wall) = walls.get(&(col, row)).filter(|wall| wall.side == Side::Friendly) {
                    let hit = match blast.shape {
                        Shape::Circle => wall.overlaps_circle(blast.position.x, blast.position.y, blast.radius),
                        //(another shape hits the walls its edge comes within half a tile of the middle of)
                        shape => outside_by(blast.position, blast.radius, shape, tile_center((col, row))) <= TILE_SIZE * 0.5,
                    };
                    if hit {
                        fx.wall_hits.push(((col, row), blast.damage));
                    }
                }
            }
        }
    }
    for (tile, damage) in std::mem::take(&mut fx.wall_hits) {
        if let Some(wall) = walls.get_mut(&tile) {
            if wall.damage(damage) {
                broken.push(tile);
            }
        }
    }
    for (&tile, wall) in walls.iter_mut() {
        wall.hurt = (wall.hurt + dt).min(WALL_FLASH);
        if wall.life.is_finite() {
            wall.life -= dt;
            if wall.life <= 0.0 && !broken.contains(&tile) {
                broken.push(tile);
            }
        }
    }
    for &tile in &broken {
        if let Some(wall) = walls.remove(&tile) {
            let at = wall.center();
            fx.burst(at, wall.color, 10, 220.0);
            if wall.kind != WallKind::Spikes && wall.kind != WallKind::Tar {
                fx.add_shake(0.08);
                fx.cue_at(Cue::WallBreak, at);
            }
            release_garrison(units, tile, at);
        }
    }
    !broken.is_empty()
}

//the walls drawn under everything: spikes and tar, which things walk over
pub(crate) fn draw_floor(center_x: f32, center_y: f32, zoom: f32, walls: &Walls, time: f32) {
    let (first_col, last_col, first_row, last_row) = visible_tiles(center_x, center_y, zoom);
    for col in first_col..=last_col {
        for row in first_row..=last_row {
            if let Some(wall) = walls.get(&(col, row)).filter(|wall| wall.kind.is_floor()) {
                wall.draw(time);
            }
        }
    }
}

//the walls, drawn after everything else in the world so they are on top of all of it. the hovered square is drawn
//under them, so one with a wall on it gets an outline on top instead, to show which one a right click would delete. an
//empty hovered square shows what a left click would build there
pub(crate) fn draw_walls(center_x: f32, center_y: f32, zoom: f32, hovered: Option<(i32, i32)>, walls: &Walls, selected: Option<WallKind>, time: f32) {
    let (first_col, last_col, first_row, last_row) = visible_tiles(center_x, center_y, zoom);
    for col in first_col..=last_col {
        for row in first_row..=last_row {
            if let Some(wall) = walls.get(&(col, row)).filter(|wall| !wall.kind.is_floor()) {
                wall.draw(time);
            }
        }
    }
    let Some((col, row)) = hovered else {
        return;
    };
    if walls.contains_key(&(col, row)) {
        draw_rectangle_lines(col as f32 * TILE_SIZE, row as f32 * TILE_SIZE, TILE_SIZE, TILE_SIZE, 4.0 / zoom, WHITE);
    } else if let Some(kind) = selected {
        let color = kind.color();
        draw_rectangle(col as f32 * TILE_SIZE, row as f32 * TILE_SIZE, TILE_SIZE, TILE_SIZE, Color::new(color.r, color.g, color.b, 0.45));
        if kind == WallKind::Tower {
            let at = tile_center((col, row));
            draw_circle_lines(at.x, at.y, TOWER_RANGE, 2.0 / zoom, Color::new(color.r, color.g, color.b, 0.5));
        }
    }
}

impl Wall {
    //drawn in its kind's look, cracked and lit when it has been hit
    pub(crate) fn draw(&self, time: f32) {
        let rect = self.rect();
        self.draw_in(rect.x, rect.y, rect.w, time);
    }

    //drawn in a square of the given size with its corner at a point, which is how the build bar shows one
    pub(crate) fn draw_in(&self, x: f32, y: f32, size: f32, time: f32) {
        let left = self.share_left();
        let lit = (1.0 - self.hurt / WALL_FLASH).clamp(0.0, 1.0);
        let shade = 0.6 + 0.4 * left;
        let base = lighten(Color::new(self.color.r * shade, self.color.g * shade, self.color.b * shade, self.color.a), lit * 0.7);
        match self.kind {
            WallKind::Wall | WallKind::Tower => {
                draw_rectangle(x, y, size, size, base);
                draw_rectangle(x, y, size, 3.0, lighten(base, 0.35));
                draw_rectangle(x, y, 3.0, size, lighten(base, 0.2));
                draw_rectangle(x, y + size - 3.0, size, 3.0, Color::new(0.0, 0.0, 0.0, 0.3));
                draw_rectangle(x + size - 3.0, y, 3.0, size, Color::new(0.0, 0.0, 0.0, 0.2));
                if self.side == Side::Hostile {
                    for k in 0..3 {
                        let stud = Vec2::new(x + size * (0.2 + 0.3 * k as f32), y + size * 0.5);
                        draw_circle(stud.x, stud.y, 3.5, Color::new(0.85, 0.3, 0.3, 0.8));
                    }
                }
                if self.kind == WallKind::Tower {
                    let notch = size * 0.22;
                    for (cx, cy) in [(x, y), (x + size - notch, y), (x, y + size - notch), (x + size - notch, y + size - notch)] {
                        draw_rectangle(cx, cy, notch, notch, lighten(base, 0.45));
                    }
                    draw_rectangle(x + size * 0.4, y + size * 0.3, size * 0.2, size * 0.4, Color::new(0.05, 0.05, 0.1, 0.9));
                    for k in 0..self.garrison.min(TOWER_CAPACITY) {
                        let pip = Vec2::new(x + size * 0.12 + (k % 6) as f32 * size * 0.15, y + size * (0.78 + 0.12 * (k / 6) as f32));
                        draw_circle(pip.x, pip.y, 3.0, WHITE);
                    }
                }
            }
            WallKind::Gate => {
                draw_rectangle(x, y, size, size, Color::new(0.1, 0.09, 0.05, 1.0));
                for k in 0..5 {
                    draw_rectangle(x + size * (0.08 + 0.2 * k as f32), y + 2.0, size * 0.1, size - 4.0, base);
                }
                draw_rectangle(x, y + size * 0.45, size, size * 0.1, lighten(base, 0.3));
                draw_rectangle_lines(x, y, size, size, 3.0, lighten(base, 0.2));
            }
            WallKind::Spikes => {
                draw_rectangle(x, y, size, size, Color::new(0.12, 0.1, 0.12, 1.0));
                for i in 0..3 {
                    for j in 0..3 {
                        let at = Vec2::new(x + size * (0.2 + 0.3 * i as f32), y + size * (0.2 + 0.3 * j as f32));
                        let half = size * 0.1;
                        draw_triangle(Vec2::new(at.x, at.y - half * 1.4), Vec2::new(at.x - half, at.y + half), Vec2::new(at.x + half, at.y + half), base);
                    }
                }
            }
            WallKind::Tar => {
                draw_rectangle(x, y, size, size, Color::new(0.08, 0.03, 0.12, 0.92));
                for k in 0..3 {
                    let wobble = (time * 1.5 + (self.col * 7 + self.row * 3 + k) as f32).sin();
                    draw_circle(x + size * (0.25 + 0.25 * k as f32), y + size * (0.35 + 0.15 * wobble), size * 0.1, Color::new(0.4, 0.2, 0.55, 0.5));
                }
            }
            WallKind::Barrier => {
                let fade = (self.life / 3.0).clamp(0.0, 1.0);
                let pulse = 0.5 + 0.5 * (time * 6.0 + self.row as f32).sin();
                draw_rectangle(x, y, size, size, Color::new(base.r, base.g, base.b, (0.45 + 0.2 * pulse) * fade));
                draw_rectangle_lines(x, y, size, size, 3.0, Color::new(1.0, 0.7, 0.8, 0.9 * fade));
            }
        }
        if left < 1.0 && !self.kind.is_floor() {
            self.draw_damage(x, y, size, left);
        }
    }

    //cracks, as many as the wall is hurt, and a bar along the bottom
    fn draw_damage(&self, x: f32, y: f32, size: f32, left: f32) {
        let seed = (self.col.wrapping_mul(73856093) ^ self.row.wrapping_mul(19349663)) as u32;
        let cracks = if left < 0.25 { 4 } else if left < 0.5 { 3 } else if left < 0.75 { 2 } else { 1 };
        for k in 0..cracks {
            let bits = seed.rotate_left(k * 7);
            let start = Vec2::new(x + size * ((bits & 0xff) as f32 / 255.0), y + size * (((bits >> 8) & 0x3) as f32 / 3.0));
            let mut at = start;
            for step in 0..3u32 {
                let turn = ((bits >> (10 + step * 3)) & 0x7) as f32 / 7.0 - 0.5;
                let next = Vec2::new((at.x + size * turn * 0.7).clamp(x, x + size), (at.y + size * 0.3).clamp(y, y + size));
                draw_line(at.x, at.y, next.x, next.y, 2.0, Color::new(0.0, 0.0, 0.0, 0.7));
                at = next;
            }
        }
        draw_rectangle(x + 4.0, y + size - 9.0, size - 8.0, 5.0, Color::new(0.0, 0.0, 0.0, 0.6));
        draw_rectangle(x + 4.0, y + size - 9.0, (size - 8.0) * left, 5.0, Color::new(1.0 - left, left, 0.1, 1.0));
    }
}

//put walls a boss has raised in the world, on the squares that have nothing on them. returns whether any went up
pub(crate) fn raise_walls(walls: &mut Walls, raised: &mut Vec<Wall>) -> bool {
    let mut any = false;
    for wall in raised.drain(..) {
        if let std::collections::hash_map::Entry::Vacant(spot) = walls.entry((wall.col, wall.row)) {
            spot.insert(wall);
            any = true;
        }
    }
    any
}

//which parts of the world have a wall in them or close by, so that the thousands of units that are far from any wall don't look for
//walls one grid square at a time. the world is cut into chunks of CHUNK_TILES x CHUNK_TILES grid squares, and a chunk is marked if
//a wall is in it or in one next to it, which leaves everything in an unmarked chunk at least CHUNK_TILES grid squares from a wall
pub(crate) struct NearWalls {
    marked: Vec<bool>,
}

const CHUNK_TILES: i32 = 8;
const CHUNKS: i32 = WORLD_TILES / CHUNK_TILES + 1;

impl NearWalls {
    pub(crate) fn new(walls: &Walls) -> NearWalls {
        let mut marked = vec![false; (CHUNKS * CHUNKS) as usize];
        for &(col, row) in walls.keys() {
            let (chunk_col, chunk_row) = (col.div_euclid(CHUNK_TILES), row.div_euclid(CHUNK_TILES));
            for r in (chunk_row - 1).max(0)..=(chunk_row + 1).min(CHUNKS - 1) {
                for c in (chunk_col - 1).max(0)..=(chunk_col + 1).min(CHUNKS - 1) {
                    marked[(r * CHUNKS + c) as usize] = true;
                }
            }
        }
        NearWalls { marked }
    }

    //whether a wall may be within CHUNK_TILES grid squares of a point. a point outside the world could be near anything
    pub(crate) fn may_be_near(&self, at: Vec2) -> bool {
        let (col, row) = tile_of(at.x, at.y);
        let (chunk_col, chunk_row) = (col.div_euclid(CHUNK_TILES), row.div_euclid(CHUNK_TILES));
        if !(0..CHUNKS).contains(&chunk_col) || !(0..CHUNKS).contains(&chunk_row) {
            return true;
        }
        self.marked[(chunk_row * CHUNKS + chunk_col) as usize]
    }
}
