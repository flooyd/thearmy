//the picture in the corner of the window: three sizes of the player with some ships round them, on a tile with round corners

use super::*;

fn pixel(picture: &[u8], size: usize, column: usize, row: usize) -> [u8; 4] {
    let at = (row * size + column) * 4;
    [picture[at], picture[at + 1], picture[at + 2], picture[at + 3]]
}

#[test]
fn the_icon_comes_in_the_three_sizes_the_window_asks_for_and_is_not_the_default_one() {
    let icon = window_icon();
    assert_eq!((icon.small.len(), icon.medium.len(), icon.big.len()), (16 * 16 * 4, 32 * 32 * 4, 64 * 64 * 4));
    let logo = macroquad::miniquad::conf::Icon::miniquad_logo();
    assert!(icon.small != logo.small && icon.medium != logo.medium && icon.big != logo.big);
}

#[test]
fn the_icon_is_a_tile_with_round_corners_that_is_filled_everywhere_else() {
    for size in [16, 32, 64] {
        let picture = picture(size);
        let last = size - 1;
        for (column, row) in [(0, 0), (last, 0), (0, last), (last, last)] {
            assert_eq!(pixel(&picture, size, column, row)[3], 0, "the corner at {column},{row} of {size} shows what is behind it");
        }
        //(the middle of each side is in the tile)
        for (column, row) in [(size / 2, 0), (size / 2, last), (0, size / 2), (last, size / 2)] {
            assert!(pixel(&picture, size, column, row)[3] > 100, "the edge at {column},{row} of {size} is empty");
        }
    }
}

#[test]
fn the_player_is_in_the_middle_in_their_colour_with_the_ships_to_each_side_in_theirs() {
    let blue = PLAYER_COLORS[0];
    let near = |pixel: [u8; 4]| {
        let (r, g, b) = (pixel[0] as f32 / 255.0, pixel[1] as f32 / 255.0, pixel[2] as f32 / 255.0);
        pixel[3] == 255 && (r - blue.r).abs() < 0.2 && (g - blue.g).abs() < 0.2 && (b - blue.b).abs() < 0.2
    };
    for size in [32, 64] {
        let picture = picture(size);
        let count = |from: usize, to: usize| (0..size).flat_map(|row| (from..to).map(move |column| (column, row))).filter(|&(column, row)| near(pixel(&picture, size, column, row))).count();
        let (left, middle, right) = (count(0, size * 3 / 10), count(size * 3 / 10, size * 7 / 10), count(size * 7 / 10, size));
        assert!(middle > 0 && left > 0 && right > 0, "{size}: {left} {middle} {right}");
        //(the ships to each side are as many as each other)
        assert!((left as f32 - right as f32).abs() <= left.max(right) as f32 * 0.25, "{size}: {left} {right}");
    }
}

#[test]
fn the_icon_is_the_same_every_time() {
    assert_eq!(picture(32), picture(32));
}
