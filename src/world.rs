//! the world as a grid: coordinates, the player's movement, the camera, the mouse and drawing the ground

use crate::prelude::*;

//the way the movement keys say to go: a unit vector, or zero when none is held (or they cancel out)
pub(crate) fn keyboard_direction() -> Vec2 {
    let mut direction = Vec2::ZERO;
    if is_key_down(KeyCode::W) {
        direction.y -= 1.0;
    }
    if is_key_down(KeyCode::D) {
        direction.x += 1.0;
    }
    if is_key_down(KeyCode::A) {
        direction.x -= 1.0;
    }
    if is_key_down(KeyCode::S) {
        direction.y += 1.0;
    }
    direction.normalize_or_zero()
}

//speed is in pixels per second and dt is the frame time in seconds so the game plays the same at any frame rate. where someone walking from a position in a direction (a unit vector, or zero) gets to in dt seconds: round the walls, and not out of
//the world
pub(crate) fn walk_player(from: Vec2, direction: Vec2, speed: f32, dt: f32, walls: &Walls) -> Vec2 {
    let moved = move_clear_of_walls(from, from + direction * speed * dt, walls);
    let (mut x, mut y) = (moved.x, moved.y);
    clamp_to_world(&mut x, &mut y);
    Vec2::new(x, y)
}

pub(crate) fn clamp_to_world(x: &mut f32, y: &mut f32) {
    //keep the whole circle inside the world
    *x = (*x).clamp(RADIUS, WORLD_SIZE - RADIUS);
    *y = (*y).clamp(RADIUS, WORLD_SIZE - RADIUS);
}

//the grid square a position is in
pub(crate) fn tile_of(x: f32, y: f32) -> (i32, i32) {
    ((x / TILE_SIZE).floor() as i32, (y / TILE_SIZE).floor() as i32)
}

pub(crate) fn tile_center(tile: (i32, i32)) -> Vec2 {
    Vec2::new((tile.0 as f32 + 0.5) * TILE_SIZE, (tile.1 as f32 + 0.5) * TILE_SIZE)
}

//scroll wheel zooms, up zooms in and down zooms out. only the direction is used because the size of a wheel step
//depends on the platform and mouse
pub(crate) fn handle_zoom(zoom: &mut f32) {
    let (_, wheel) = mouse_wheel();
    if wheel != 0.0 {
        *zoom = (*zoom * ZOOM_STEP.powf(wheel.signum())).clamp(MIN_ZOOM, MAX_ZOOM);
    }
}

//camera that keeps the player centered on screen, the world scrolls instead of the player moving across the screen
pub(crate) fn follow_camera(player_x: f32, player_y: f32, zoom: f32) -> Camera2D {
    Camera2D {
        target: Vec2::new(player_x, player_y),
        zoom: Vec2::new(2.0 * zoom / screen_width(), 2.0 * zoom / screen_height()),
        ..Default::default()
    }
}

//where the mouse is in the world. the camera is centered on the player so the screen center is the player
pub(crate) fn mouse_world(player_x: f32, player_y: f32, zoom: f32) -> Vec2 {
    let (mouse_x, mouse_y) = mouse_position();
    Vec2::new(player_x + (mouse_x - screen_width() / 2.0) / zoom, player_y + (mouse_y - screen_height() / 2.0) / zoom)
}

//the grid square under the mouse, if any
pub(crate) fn hovered_tile(player_x: f32, player_y: f32, zoom: f32) -> Option<(i32, i32)> {
    let world = mouse_world(player_x, player_y, zoom);
    let col = (world.x / TILE_SIZE).floor() as i32;
    let row = (world.y / TILE_SIZE).floor() as i32;
    if (0..WORLD_TILES).contains(&col) && (0..WORLD_TILES).contains(&row) {
        Some((col, row))
    } else {
        None
    }
}

//the range of grid squares on screen when the camera is centered on a position, clipped to the world, as
//(first column, last column, first row, last row). zooming in shows fewer of them
pub(crate) fn visible_tiles(center_x: f32, center_y: f32, zoom: f32) -> (i32, i32, i32, i32) {
    let half_width = screen_width() / zoom / 2.0;
    let half_height = screen_height() / zoom / 2.0;
    let first_col = (((center_x - half_width) / TILE_SIZE).floor() as i32).max(0);
    let last_col = (((center_x + half_width) / TILE_SIZE).floor() as i32).min(WORLD_TILES - 1);
    let first_row = (((center_y - half_height) / TILE_SIZE).floor() as i32).max(0);
    let last_row = (((center_y + half_height) / TILE_SIZE).floor() as i32).min(WORLD_TILES - 1);
    (first_col, last_col, first_row, last_row)
}

//how plainly the grid lines show at a zoom: as they always have when close in, fading from GRID_FADE_ZOOM out and gone at half of it
pub(crate) fn grid_alpha(zoom: f32) -> f32 {
    0.1 * ((zoom - GRID_FADE_ZOOM / 2.0) / (GRID_FADE_ZOOM / 2.0)).clamp(0.0, 1.0)
}

//draw the world floor, hovered square, grid lines and border, which everything else is drawn over. the walls are drawn by
//draw_walls, over everything. only the visible squares are drawn, and the grid is left out once it has faded to nothing
pub(crate) fn draw_world(center_x: f32, center_y: f32, zoom: f32, hovered: Option<(i32, i32)>) {
    let line_color = Color::new(1.0, 1.0, 1.0, grid_alpha(zoom));
    let line_width = 1.0 / zoom; //so the grid lines stay one pixel wide on screen
    let (first_col, last_col, first_row, last_row) = visible_tiles(center_x, center_y, zoom);

    draw_rectangle(0.0, 0.0, WORLD_SIZE, WORLD_SIZE, Color::new(0.08, 0.08, 0.1, 1.0));
    if let Some((col, row)) = hovered {
        draw_rectangle(col as f32 * TILE_SIZE, row as f32 * TILE_SIZE, TILE_SIZE, TILE_SIZE, WHITE);
    }

    if line_color.a > 0.0 {
        let grid_top = first_row as f32 * TILE_SIZE;
        let grid_bottom = (last_row + 1) as f32 * TILE_SIZE;
        for col in first_col..=(last_col + 1) {
            let line_x = col as f32 * TILE_SIZE;
            draw_line(line_x, grid_top, line_x, grid_bottom, line_width, line_color);
        }
        let grid_left = first_col as f32 * TILE_SIZE;
        let grid_right = (last_col + 1) as f32 * TILE_SIZE;
        for row in first_row..=(last_row + 1) {
            let line_y = row as f32 * TILE_SIZE;
            draw_line(grid_left, line_y, grid_right, line_y, line_width, line_color);
        }
    }

    draw_rectangle_lines(0.0, 0.0, WORLD_SIZE, WORLD_SIZE, 4.0, RED);
}

//a random position between two distances in pixels away from a point
pub(crate) fn random_position_around(center_x: f32, center_y: f32, min_distance: f32, max_distance: f32) -> Vec2 {
    let bounds = RADIUS..=WORLD_SIZE - RADIUS;
    let mut position = Vec2::new(center_x, center_y);
    //try again when the position lands outside the world, the point can be close to an edge
    for _ in 0..100 {
        //picking the distance by area spreads the positions evenly instead of bunching them up near the point
        let distance = rand::gen_range(min_distance * min_distance, max_distance * max_distance).sqrt();
        let angle = rand::gen_range(0.0, std::f32::consts::TAU);
        position = Vec2::new(center_x + angle.cos() * distance, center_y + angle.sin() * distance);
        if bounds.contains(&position.x) && bounds.contains(&position.y) {
            break;
        }
    }
    position
}

//the part of the world on screen when the camera is centred on a position, a circle bigger all round
pub(crate) fn visible_area(center_x: f32, center_y: f32, zoom: f32) -> Rect {
    let half_width = screen_width() / zoom / 2.0 + RADIUS;
    let half_height = screen_height() / zoom / 2.0 + RADIUS;
    Rect::new(center_x - half_width, center_y - half_height, half_width * 2.0, half_height * 2.0)
}

