//! building: spending wall points to place walls by dragging, mending the hurt ones, and deleting for a refund

use crate::prelude::*;

//the squares from one square to another, if they are in the same row or column. a diagonal gives nothing
pub(crate) fn drag_line(start: (i32, i32), end: (i32, i32)) -> Vec<(i32, i32)> {
    if start.0 == end.0 {
        (start.1.min(end.1)..=start.1.max(end.1)).map(|row| (start.0, row)).collect()
    } else if start.1 == end.1 {
        (start.0.min(end.0)..=start.0.max(end.0)).map(|col| (col, start.1)).collect()
    } else {
        Vec::new()
    }
}

//a mouse button being held down while it is moved over the grid
pub(crate) struct Drag {
    pub(crate) last: (i32, i32),         //the last square it reached
    pub(crate) horizontal: Option<bool>, //whether it is going along a row (true) or a column, None until it has moved
    pub(crate) run: i32,                 //how many squares it has gone since it last turned
}

impl Drag {
    pub(crate) fn new(start: (i32, i32)) -> Drag {
        Drag { last: start, horizontal: None, run: 0 }
    }

    //move the drag on to the square under the mouse and give every square it went over. a drag goes along the row or
    //column of the square it last reached, so it can turn from one to the other without letting go of the button.
    //squares that are not in line with it, which includes diagonal ones, are ignored until the mouse is back in line
    pub(crate) fn reach(&mut self, square: (i32, i32)) -> Vec<(i32, i32)> {
        if square == self.last {
            return vec![square];
        }
        let squares = drag_line(self.last, square);
        if squares.is_empty() {
            return squares;
        }
        let horizontal = square.1 == self.last.1;
        if self.horizontal.is_some_and(|going| going != horizontal) {
            //turning a corner. a drag has to go a few squares in a line before it can turn, otherwise a diagonal drag
            //would still build a staircase, turning after every square
            if self.run < MIN_RUN_BEFORE_TURN {
                return Vec::new();
            }
            self.run = 0;
        }
        self.horizontal = Some(horizontal);
        self.run += (square.0 - self.last.0).abs() + (square.1 - self.last.1).abs();
        self.last = square;
        squares
    }
}

//the squares a mouse button is dragging over. pressing the button starts a drag on the square under the mouse and
//holding it while moving along a row or column gives every square in between
pub(crate) fn dragged_squares(button: MouseButton, drag: &mut Option<Drag>, hovered: Option<(i32, i32)>) -> Vec<(i32, i32)> {
    if is_mouse_button_pressed(button) {
        //None when the press was outside the world, so dragging in from outside gives nothing
        *drag = hovered.map(Drag::new);
    } else if !is_mouse_button_down(button) {
        *drag = None;
    }
    match (drag.as_mut(), hovered) {
        (Some(drag), Some(hovered)) => drag.reach(hovered),
        _ => Vec::new(),
    }
}

//the player's building: what is chosen to build, the wall points saved up, and the two drags in progress
pub(crate) struct Mason {
    pub(crate) selected: WallKind,
    pub(crate) points: f32,
    pub(crate) place_drag: Option<Drag>,
    pub(crate) delete_drag: Option<Drag>,
    pub(crate) nag_in: f32, //seconds until it can say again that there aren't enough points
}

impl Mason {
    pub(crate) fn new() -> Mason {
        Mason { selected: WallKind::Wall, points: START_WALL_POINTS, place_drag: None, delete_drag: None, nag_in: 0.0 }
    }

    //points earned, up to the most that can be saved
    pub(crate) fn earn(&mut self, points: f32) {
        self.points = (self.points + points).min(MAX_WALL_POINTS);
    }

    //the number keys choose what a left click builds
    pub(crate) fn choose(&mut self) {
        for (key, kind) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5].into_iter().zip(BUILDABLE) {
            if is_key_pressed(key) {
                self.selected = kind;
            }
        }
    }
}

//place a wall of a kind on a grid square unless it already has one or, for a kind the army can't walk through, the player,
//a unit or a baby unit is standing in the way. returns whether it was placed
pub(crate) fn place_wall(walls: &mut Walls, kind: WallKind, health: f32, col: i32, row: i32, player_x: f32, player_y: f32, units: &[Unit], babies: &[Baby]) -> bool {
    let wall = Wall::made(col, row, kind, Side::Friendly, health);
    let in_the_way = wall.blocks(Mover::Ally)
        && (wall.overlaps_circle(player_x, player_y, RADIUS)
            || units.iter().any(|unit| unit.garrison.is_none() && wall.overlaps_circle(unit.x, unit.y, RADIUS))
            || babies.iter().any(|baby| wall.overlaps_circle(baby.position.x, baby.position.y, BABY_RADIUS)));
    if walls.contains_key(&(col, row)) || in_the_way {
        return false;
    }
    walls.insert((col, row), wall);
    true
}

//what a click on a square did
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Built {
    Placed,
    Repaired,
    Nothing, //something was in the way, or the wall was already as good as new
    Poor,    //not enough wall points
}

//a left click on a square: a hurt wall of the player's is mended and made as strong as a new one, an empty square gets the
//chosen kind. both cost wall points
pub(crate) fn build(walls: &mut Walls, kind: WallKind, col: i32, row: i32, player: Vec2, units: &[Unit], babies: &[Baby], points: &mut f32, army: usize, upgrades: &Upgrades) -> Built {
    if let Some(wall) = walls.get_mut(&(col, row)) {
        if wall.side != Side::Friendly || wall.health >= wall.max_health {
            return Built::Nothing;
        }
        if *points < REPAIR_COST {
            return Built::Poor;
        }
        *points -= REPAIR_COST;
        let fresh = wall_health(wall.kind, army, upgrades).max(wall.max_health);
        wall.health = fresh;
        wall.max_health = fresh;
        wall.hurt = 0.0;
        return Built::Repaired;
    }
    if *points < kind.cost() {
        return Built::Poor;
    }
    if !place_wall(walls, kind, wall_health(kind, army, upgrades), col, row, player.x, player.y, units, babies) {
        return Built::Nothing;
    }
    *points -= kind.cost();
    Built::Placed
}

//delete one of the player's walls: part of what it cost comes back as wall points, less the more it was hurt, and the units inside a tower come
//out. returns whether there was a wall of the player's to delete
pub(crate) fn delete_wall(mason: &mut Mason, walls: &mut Walls, units: &mut [Unit], tile: (i32, i32)) -> bool {
    if !walls.get(&tile).is_some_and(|wall| wall.side == Side::Friendly) {
        return false;
    }
    let wall = walls.remove(&tile).unwrap();
    mason.earn(wall.kind.cost() * WALL_REFUND_SHARE * wall.share_left());
    release_garrison(units, tile, wall.center());
    true
}

//left click (or drag) builds and right click (or drag) deletes the player's walls, giving back part of what they cost. a
//left click that is on an enemy (the caller has sent a squad at it) builds nothing, and the drag it would have started
//doesn't happen. nor does anything happen while the mouse is over the minimap, which isn't the world. returns whether any
//wall changed
pub(crate) fn edit_walls(
    mason: &mut Mason,
    walls: &mut Walls,
    units: &mut [Unit],
    babies: &[Baby],
    player: Vec2,
    zoom: f32,
    clicked_enemy: bool,
    over_minimap: bool,
    dt: f32,
    army: usize,
    upgrades: &Upgrades,
    fx: &mut Effects,
) -> bool {
    mason.nag_in -= dt;
    mason.choose();
    let hovered = if over_minimap { None } else { hovered_tile(player.x, player.y, zoom) };
    let mut changed = false;
    let place_at = if clicked_enemy { None } else { hovered };
    let kind = mason.selected;
    for (col, row) in dragged_squares(MouseButton::Left, &mut mason.place_drag, place_at) {
        match build(walls, kind, col, row, player, units, babies, &mut mason.points, army, upgrades) {
            Built::Placed | Built::Repaired => {
                changed = true;
            }
            Built::Poor if mason.nag_in <= 0.0 => {
                mason.nag_in = 1.0;
                fx.popup(player, RADIUS * 2.0, 26.0, "not enough wall points".to_owned(), Color::new(1.0, 0.55, 0.4, 1.0));
            }
            _ => {}
        }
    }
    for (col, row) in dragged_squares(MouseButton::Right, &mut mason.delete_drag, hovered) {
        changed |= delete_wall(mason, walls, units, (col, row));
    }
    changed
}

pub(crate) const BUILD_BAR_HEIGHT: f32 = 138.0; //how far up from the bottom of the screen the build bar reaches

//the build bar along the bottom left: the wall points, a slot for each kind with its key and cost, the chosen one outlined in
//the accent colour, and what the chosen one is. a kind that can't be afforded is dim
pub(crate) fn draw_build_bar(mason: &Mason, time: f32) {
    let slot = 58.0;
    let gap = 8.0;
    let left = 12.0;
    let top = screen_height() - slot - 39.0;
    let bar_width = (slot + gap) * BUILDABLE.len() as f32 - gap;
    //the wall points, with how full the store is along the bottom of the panel
    let points_top = top - 38.0;
    panel(left - 3.0, points_top, bar_width + 6.0, 28.0);
    draw_label("WALL POINTS", left + 8.0, points_top + 19.0, 14.0, MUTED, Face::Bold);
    draw_label_right(&format!("{:.0}", mason.points), left + bar_width - 5.0, points_top + 21.0, 20.0, INK, Face::Header);
    draw_meter(left + 2.0, points_top + 25.0, bar_width - 4.0, 2.0, mason.points / MAX_WALL_POINTS, CYAN);
    for (i, kind) in BUILDABLE.into_iter().enumerate() {
        let x = left + (slot + gap) * i as f32;
        let affordable = mason.points >= kind.cost();
        let selected = mason.selected == kind;
        draw_rectangle(x - 3.0, top - 3.0, slot + 6.0, slot + 6.0, PANEL);
        Wall::made(0, 0, kind, Side::Friendly, WALL_HEALTH).draw_in(x + 6.0, top + 4.0, slot - 12.0, time);
        if !affordable {
            draw_rectangle(x, top, slot, slot, Color::new(0.0, 0.0, 0.0, 0.55));
        }
        if selected {
            draw_rectangle(x - 3.0, top - 3.0, slot + 6.0, slot + 6.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.12));
            draw_rectangle_lines(x - 5.0, top - 5.0, slot + 10.0, slot + 10.0, 1.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.35));
        }
        draw_rectangle_lines(x - 3.0, top - 3.0, slot + 6.0, slot + 6.0, if selected { 3.0 } else { 1.5 }, if selected { CYAN } else { Color::new(1.0, 1.0, 1.0, 0.25) });
        draw_key(&format!("{}", i + 1), x + 3.0, top + 16.0, 14.0);
        let cost = format!("{:.0}", kind.cost());
        let cost_color = if !affordable { ALERT } else if selected { CYAN } else { INK };
        draw_label_shadowed(&cost, x + slot - text_width(&cost, 20.0, Face::Bold) - 4.0, top + slot - 5.0, 20.0, cost_color, Face::Bold);
    }
    //what the chosen kind is
    let name = mason.selected.name().to_uppercase();
    draw_label_shadowed(&name, left, top + slot + 26.0, 16.0, CYAN, Face::Bold);
    draw_label_shadowed(mason.selected.blurb(), left + text_width(&name, 16.0, Face::Bold) + 10.0, top + slot + 26.0, 16.0, INK, Face::Body);
}
