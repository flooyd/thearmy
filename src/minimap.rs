//! the minimap and clicking on it

use crate::prelude::*;

pub(crate) const MINIMAP_HEIGHT: f32 = 0.3; //the minimap is a square this share of the screen's height along each side...
pub(crate) const MINIMAP_MIN_SIDE: f32 = 180.0; //...but never smaller than this many pixels...
pub(crate) const MINIMAP_MAX_SIDE: f32 = 380.0; //...or bigger than this
pub(crate) const MINIMAP_MARGIN: f32 = 12.0; //pixels from the corner of the screen it sits in
pub(crate) const MINIMAP_NEAR_TILES: f32 = 60.0; //the near view shows this many grid squares each way from the player
pub(crate) const MINIMAP_CLICK_MARGIN: f32 = 7.0; //pixels, a click this close to an enemy's dot is on it
pub(crate) const MINIMAP_MIN_DOT: f32 = 3.0; //pixels along a side, the smallest an enemy's dot is, so the little ones can be seen
pub(crate) const MINIMAP_MAX_DOT: f32 = 10.0; //and the biggest

//how much of the world the minimap shows
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum MapView {
    Near,  //the squares around the player
    World, //all of it
}

impl MapView {
    pub(crate) fn other(self) -> MapView {
        match self {
            MapView::Near => MapView::World,
            MapView::World => MapView::Near,
        }
    }
}

//where the minimap is on the screen and which part of the world it shows
pub(crate) struct Minimap {
    pub(crate) rect: Rect, //on the screen
    pub(crate) center: Vec2, //the world position at its middle
    pub(crate) half: f32,    //world pixels from the middle to each edge
}

impl Minimap {
    //in the bottom right corner of a screen. the near view is centred on the player, but slides back from the edge of the
    //world so that it never shows anything outside it
    pub(crate) fn new(screen: Vec2, view: MapView, player: Vec2) -> Minimap {
        let side = (screen.y * MINIMAP_HEIGHT).clamp(MINIMAP_MIN_SIDE, MINIMAP_MAX_SIDE).min(screen.min_element() - MINIMAP_MARGIN * 2.0).max(1.0);
        let rect = Rect::new(screen.x - MINIMAP_MARGIN - side, screen.y - MINIMAP_MARGIN - side, side, side);
        match view {
            MapView::World => Minimap { rect, center: Vec2::splat(WORLD_SIZE / 2.0), half: WORLD_SIZE / 2.0 },
            MapView::Near => {
                let half = MINIMAP_NEAR_TILES * TILE_SIZE;
                Minimap { rect, center: player.clamp(Vec2::splat(half), Vec2::splat(WORLD_SIZE - half)), half }
            }
        }
    }

    //screen pixels for each world pixel
    pub(crate) fn scale(&self) -> f32 {
        self.rect.w / (self.half * 2.0)
    }

    pub(crate) fn to_screen(&self, world: Vec2) -> Vec2 {
        self.rect.center() + (world - self.center) * self.scale()
    }

    pub(crate) fn contains(&self, screen: Vec2) -> bool {
        self.rect.contains(screen)
    }

    //the length of the side of an enemy's dot: its size on the map, but not so small it can't be seen or so big it hides the rest
    pub(crate) fn dot_size(&self, enemy: &Enemy) -> f32 {
        //a boss is always as big as a dot gets
        let smallest = if enemy.boss.is_some() { MINIMAP_MAX_DOT } else { MINIMAP_MIN_DOT };
        (enemy.size * self.scale()).clamp(smallest, MINIMAP_MAX_DOT)
    }
}

//the enemy whose dot is under a point on the screen, if the map shows one there: the closest, if several are
pub(crate) fn minimap_enemy_at(map: &Minimap, point: Vec2, enemies: &[Enemy]) -> Option<usize> {
    enemies
        .iter()
        .enumerate()
        .map(|(i, enemy)| (i, map.to_screen(enemy.position), map.dot_size(enemy)))
        .filter(|&(_, dot, size)| map.contains(dot) && dot.distance(point) <= size / 2.0 + MINIMAP_CLICK_MARGIN)
        .min_by(|a, b| a.1.distance_squared(point).total_cmp(&b.1.distance_squared(point)))
        .map(|(i, _, _)| i)
}

//the minimap, drawn with the default camera: a dot for everything in the part of the world it shows, a box for the part the
//camera shows, rings round the enemies that are after the player and round the ones a squad has been sent at, and the
//enemy under the mouse outlined. `time` is for the rings to pulse
pub(crate) fn draw_minimap(
    map: &Minimap,
    view: MapView,
    player: Vec2,
    camera: Rect,
    hovered: Option<usize>,
    units: &[Unit],
    enemies: &[Enemy],
    babies: &[Baby],
    squads: &Squads,
    walls: &Walls,
    hazards: &[Hazard],
    sites: &[Site],
    time: f32,
) {
    let rect = map.rect;
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.0, 0.0, 0.0, 0.75));
    //a dot is drawn only if all of it is on the map
    let on_map = |point: Vec2, margin: f32| point.x >= rect.x + margin && point.x <= rect.right() - margin && point.y >= rect.y + margin && point.y <= rect.bottom() - margin;

    //the walls are too small to see when all the world is on the map
    if view == MapView::Near {
        let side = (TILE_SIZE * map.scale()).max(1.0);
        let wall_color = Color::new(0.55, 0.55, 0.6, 1.0);
        for &(col, row) in walls.keys() {
            let point = map.to_screen(Vec2::new(col as f32 * TILE_SIZE, row as f32 * TILE_SIZE));
            if on_map(point, 0.0) && on_map(point + Vec2::splat(side), 0.0) {
                draw_rectangle(point.x, point.y, side, side, wall_color);
            }
        }
    }
    draw_site_marks(map, sites, time);
    let baby_color = pickup_color();
    for baby in babies {
        let point = map.to_screen(baby.position);
        if on_map(point, 1.0) {
            draw_rectangle(point.x - 1.0, point.y - 1.0, 2.0, 2.0, baby_color);
        }
    }
    //(thousands of units sit on a few hundred pixels of the map, so each pixel is drawn once, however many are on it)
    let (columns, rows) = (rect.w.ceil() as usize + 1, rect.h.ceil() as usize + 1);
    let mut seen = vec![0u8; columns * rows];
    for unit in units {
        let point = map.to_screen(Vec2::new(unit.x, unit.y));
        if on_map(point, 1.0) {
            let at = (point.x - rect.x) as usize + columns * (point.y - rect.y) as usize;
            //(a unit of a squad shows over the ones that aren't)
            seen[at] = seen[at].max(if unit.target.is_some() { 2 } else { 1 });
        }
    }
    for (at, &mark) in seen.iter().enumerate() {
        if mark != 0 {
            let (px, py) = (rect.x + (at % columns) as f32, rect.y + (at / columns) as f32);
            draw_rectangle(px - 0.5, py - 0.5, 2.0, 2.0, if mark == 2 { SQUAD_COLOR } else { WHITE });
        }
    }
    //the champions are bigger dots, ringed, in their own colours
    for unit in units.iter().filter(|unit| unit.champion != 0) {
        let point = map.to_screen(Vec2::new(unit.x, unit.y));
        if let Some(kind) = unit.champion_kind().filter(|_| on_map(point, 4.0)) {
            draw_circle(point.x, point.y, 3.5, kind.color);
            draw_circle_lines(point.x, point.y, 3.5, 1.2, WHITE);
        }
    }
    //what is about to go off, as the circle it will hurt
    for hazard in hazards.iter().filter(|hazard| hazard.armed) {
        let point = map.to_screen(hazard.position);
        let radius = (hazard.radius * map.scale()).max(2.0).min(rect.w / 3.0);
        if on_map(point, radius) {
            draw_circle_lines(point.x, point.y, radius, 1.5, danger_color(0.5 + 0.4 * (0.5 + 0.5 * (time * 12.0).sin())));
        }
    }
    for (i, enemy) in enemies.iter().enumerate() {
        let point = map.to_screen(enemy.position);
        let size = map.dot_size(enemy);
        if !on_map(point, size / 2.0) {
            continue;
        }
        draw_rectangle(point.x - size / 2.0, point.y - size / 2.0, size, size, enemy.color);
        if enemy.boss.is_some() {
            let pulse = 0.5 + 0.5 * (time * 6.0).sin();
            draw_circle_lines(point.x, point.y, size / 2.0 + 5.0 + 3.0 * pulse, 2.0, Color::new(1.0, 1.0, 1.0, 0.6 + 0.4 * pulse));
        } else if enemy.modifier != Modifier::None {
            let mark = enemy.modifier.color();
            draw_rectangle_lines(point.x - size / 2.0 - 1.0, point.y - size / 2.0 - 1.0, size + 2.0, size + 2.0, 1.5, mark);
        }
        if squads.contains_key(&enemy.id) {
            draw_rectangle_lines(point.x - size / 2.0 - 2.0, point.y - size / 2.0 - 2.0, size + 4.0, size + 4.0, 1.5, SQUAD_COLOR);
        }
        if enemy.chasing {
            let pulse = 0.5 + 0.5 * (time * 8.0).sin();
            //(the ones of a horde are ringed in orange, so they can be told from the rest)
            let ring = if in_a_horde(enemy) { Color::new(1.0, 0.55, 0.1, 0.7 + 0.3 * pulse) } else { Color::new(1.0, 0.3, 0.3, 0.6 + 0.4 * pulse) };
            draw_circle_lines(point.x, point.y, size / 2.0 + 4.0 + 3.0 * pulse, 1.5, ring);
        }
        if hovered == Some(i) {
            draw_rectangle_lines(point.x - size / 2.0 - 3.0, point.y - size / 2.0 - 3.0, size + 6.0, size + 6.0, 2.0, WHITE);
        }
    }

    //the part of the world the camera shows, kept to at least a few pixels so it can be found when all the world is on the map
    let corner_a = map.to_screen(camera.point());
    let corner_b = map.to_screen(camera.point() + camera.size());
    let middle = (corner_a + corner_b) / 2.0;
    let half_box = ((corner_b - corner_a) / 2.0).max(Vec2::splat(3.0));
    let top_left = (middle - half_box).max(rect.point());
    let bottom_right = (middle + half_box).min(rect.point() + rect.size());
    if bottom_right.x > top_left.x && bottom_right.y > top_left.y {
        draw_rectangle_lines(top_left.x, top_left.y, bottom_right.x - top_left.x, bottom_right.y - top_left.y, 1.5, Color::new(1.0, 1.0, 1.0, 0.8));
    }

    let point = map.to_screen(player).max(rect.point() + Vec2::splat(4.0)).min(rect.point() + rect.size() - Vec2::splat(4.0));
    draw_circle(point.x, point.y, 4.0, BLUE);
    draw_circle_lines(point.x, point.y, 4.0, 1.5, WHITE);

    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.75));
    let label = if view == MapView::Near { "NEAR" } else { "WORLD" };
    let size = 15.0;
    let key_width = text_width("Tab", size, Face::Bold) + 10.0;
    let plate = text_width(label, size, Face::Bold) + key_width + 22.0;
    draw_rectangle(rect.x, rect.y, plate, size + 12.0, Color::new(0.0, 0.0, 0.0, 0.6));
    draw_label(label, rect.x + 7.0, rect.y + size + 3.0, size, CYAN, Face::Bold);
    draw_key("Tab", rect.x + 15.0 + text_width(label, size, Face::Bold), rect.y + size + 3.0, size - 2.0);
}

