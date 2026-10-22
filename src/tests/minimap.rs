//the minimap

use super::*;

// ---------- the minimap ----------

pub(super) const SCREEN: Vec2 = Vec2::new(1600.0, 900.0);

pub(super) fn enemy_on_map(map: &Minimap, at_screen: Vec2) -> Enemy {
    let mut enemy = still_enemy(map.center + (at_screen - map.rect.center()) / map.scale(), 100.0);
    enemy.size = 44.0;
    enemy
}

#[test]
fn the_minimap_sits_in_the_bottom_right_corner_and_is_square() {
    for screen in [SCREEN, Vec2::new(800.0, 600.0), Vec2::new(3840.0, 2160.0), Vec2::new(300.0, 200.0)] {
        let map = Minimap::new(screen, MapView::Near, Vec2::new(16000.0, 16000.0));
        assert_eq!(map.rect.w, map.rect.h);
        assert!(map.rect.x >= 0.0 && map.rect.y >= 0.0, "{screen}: {:?}", map.rect);
        assert!((map.rect.right() - (screen.x - MINIMAP_MARGIN)).abs() < 0.01);
        assert!((map.rect.bottom() - (screen.y - MINIMAP_MARGIN)).abs() < 0.01);
        assert!(map.rect.w <= MINIMAP_MAX_SIDE);
    }
    let big = Minimap::new(Vec2::new(3840.0, 2160.0), MapView::Near, Vec2::ZERO);
    assert_eq!(big.rect.w, MINIMAP_MAX_SIDE);
    let ordinary = Minimap::new(SCREEN, MapView::Near, Vec2::ZERO);
    assert!((ordinary.rect.w - 270.0).abs() < 0.01);
}

#[test]
fn the_world_view_shows_the_whole_world_in_the_box() {
    let map = Minimap::new(SCREEN, MapView::World, Vec2::new(123.0, 456.0));
    assert!((map.to_screen(Vec2::splat(WORLD_SIZE / 2.0)) - map.rect.center()).length() < 0.01);
    assert!((map.to_screen(Vec2::ZERO) - map.rect.point()).length() < 0.01);
    assert!((map.to_screen(Vec2::splat(WORLD_SIZE)) - (map.rect.point() + map.rect.size())).length() < 0.01);
}

#[test]
fn the_near_view_is_centred_on_the_player_and_shows_closer_detail() {
    let player = Vec2::new(16000.0, 16000.0);
    let near = Minimap::new(SCREEN, MapView::Near, player);
    let world = Minimap::new(SCREEN, MapView::World, player);
    assert!((near.to_screen(player) - near.rect.center()).length() < 0.01);
    assert!(near.scale() > world.scale() * 4.0);
    //a point one grid square away is TILE_SIZE x scale away on the map
    assert!(((near.to_screen(player + Vec2::new(TILE_SIZE, 0.0)) - near.to_screen(player)).x - TILE_SIZE * near.scale()).abs() < 0.001);
    //60 squares out is the edge
    let edge = near.to_screen(player + Vec2::new(MINIMAP_NEAR_TILES * TILE_SIZE, 0.0));
    assert!((edge.x - near.rect.right()).abs() < 0.01);
}

#[test]
fn the_near_view_never_shows_past_the_edge_of_the_world() {
    for player in [Vec2::ZERO, Vec2::new(WORLD_SIZE, WORLD_SIZE), Vec2::new(100.0, 16000.0), Vec2::new(31900.0, 200.0)] {
        let map = Minimap::new(SCREEN, MapView::Near, player);
        let top_left = map.to_screen(Vec2::ZERO);
        let bottom_right = map.to_screen(Vec2::splat(WORLD_SIZE));
        assert!(top_left.x <= map.rect.x + 0.01 && top_left.y <= map.rect.y + 0.01, "{player}");
        assert!(bottom_right.x >= map.rect.right() - 0.01 && bottom_right.y >= map.rect.bottom() - 0.01, "{player}");
        //the player is still on the map, if not in the middle
        assert!(map.contains(map.to_screen(player)));
    }
}

#[test]
fn a_click_on_a_dot_finds_the_enemy_and_the_closest_one_when_dots_overlap() {
    let map = Minimap::new(SCREEN, MapView::World, Vec2::ZERO);
    let a = map.rect.center() + Vec2::new(-30.0, 10.0);
    let b = a + Vec2::new(5.0, 0.0);
    let far = map.rect.center() + Vec2::new(60.0, 60.0);
    let enemies = vec![enemy_on_map(&map, a), enemy_on_map(&map, b), enemy_on_map(&map, far)];
    assert_eq!(minimap_enemy_at(&map, a, &enemies), Some(0));
    assert_eq!(minimap_enemy_at(&map, b + Vec2::new(1.0, 0.0), &enemies), Some(1));
    assert_eq!(minimap_enemy_at(&map, far, &enemies), Some(2));
    //near enough counts, a bit further doesn't
    let near_miss = far + Vec2::new(MINIMAP_MIN_DOT / 2.0 + MINIMAP_CLICK_MARGIN - 0.5, 0.0);
    assert_eq!(minimap_enemy_at(&map, near_miss, &enemies), Some(2));
    let miss = far + Vec2::new(MINIMAP_MIN_DOT / 2.0 + MINIMAP_CLICK_MARGIN + 1.0, 0.0);
    assert_eq!(minimap_enemy_at(&map, miss, &enemies), None);
    assert_eq!(minimap_enemy_at(&map, map.rect.center() + Vec2::new(-80.0, -80.0), &enemies), None);
}

#[test]
fn enemies_outside_the_part_of_the_world_on_the_map_cannot_be_clicked() {
    let player = Vec2::new(16000.0, 16000.0);
    let map = Minimap::new(SCREEN, MapView::Near, player);
    let outside = {
        let mut enemy = still_enemy(player + Vec2::new(MINIMAP_NEAR_TILES * TILE_SIZE + 100.0, 0.0), 100.0);
        enemy.size = 44.0;
        enemy
    };
    let inside = {
        let mut enemy = still_enemy(player + Vec2::new(500.0, -400.0), 100.0);
        enemy.size = 44.0;
        enemy
    };
    let enemies = vec![outside, inside];
    //right at the map's edge, where the one outside would be, there's nothing
    let edge = Vec2::new(map.rect.right() - 1.0, map.rect.center().y);
    assert_eq!(minimap_enemy_at(&map, edge, &enemies), None);
    assert_eq!(minimap_enemy_at(&map, map.to_screen(enemies[1].position), &enemies), Some(1));
    //but the world view has both
    let world = Minimap::new(SCREEN, MapView::World, player);
    assert_eq!(minimap_enemy_at(&world, world.to_screen(enemies[0].position), &enemies), Some(0));
}

#[test]
fn bigger_enemies_get_bigger_dots_within_limits() {
    let map = Minimap::new(SCREEN, MapView::World, Vec2::ZERO);
    let mut enemy = plain_enemy(Vec2::new(16000.0, 16000.0));
    enemy.size = 10.0;
    assert_eq!(map.dot_size(&enemy), MINIMAP_MIN_DOT);
    enemy.size = 100000.0;
    assert_eq!(map.dot_size(&enemy), MINIMAP_MAX_DOT);
    let near = Minimap::new(SCREEN, MapView::Near, Vec2::new(16000.0, 16000.0));
    enemy.size = 130.0;
    assert!(near.dot_size(&enemy) > map.dot_size(&enemy));
}

#[test]
fn the_view_toggles_between_near_and_the_whole_world() {
    assert_eq!(MapView::Near.other(), MapView::World);
    assert_eq!(MapView::World.other().other(), MapView::World);
}

#[test]
fn a_click_on_a_dot_sends_a_squad_at_that_enemy_just_as_a_click_in_the_world_does() {
    let at = Vec2::new(16000.0, 16000.0);
    let map = Minimap::new(SCREEN, MapView::World, at);
    let enemies = vec![still_enemy(at, 1000.0), still_enemy(at + Vec2::new(4000.0, 0.0), 1000.0)];
    let mut units: Vec<Unit> = (0..50).map(|i| unit_on(at + Vec2::new(-200.0 - 10.0 * i as f32, 0.0))).collect();
    let mut squads = Squads::new();
    let clicked = minimap_enemy_at(&map, map.to_screen(enemies[1].position), &enemies).expect("a dot there");
    let sent = send_squad(&mut units, &mut squads, &enemies[clicked], &mut Effects::default());
    assert_eq!(sent, squad_size(&enemies[1]));
    assert!(squads.contains_key(&enemies[1].id) && !squads.contains_key(&enemies[0].id));
    assert_eq!(units.iter().filter(|unit| unit.target == Some(enemies[1].id)).count(), sent);
}

#[test]
fn mapping_a_full_army_and_a_lot_of_enemies_onto_the_minimap_is_cheap() {
    //the part of drawing that isn't the GPU: mapping and culling a full army, a lot of enemies and babies, and walls
    let player = Vec2::new(16000.0, 16000.0);
    let map = Minimap::new(SCREEN, MapView::Near, player);
    let units: Vec<Unit> = (0..MAX_UNITS).map(|i| unit_on(player + Vec2::new((i % 80) as f32 * 40.0 - 1600.0, (i / 80) as f32 * 40.0 - 1000.0))).collect();
    let enemies: Vec<Enemy> = (0..ENEMY_COUNT).map(|i| still_enemy(player + Vec2::new(i as f32 * 90.0 - 2000.0, 700.0), 10.0)).collect();
    let started = Instant::now();
    let mut visible = 0;
    for _ in 0..200 {
        for unit in &units {
            visible += map.contains(map.to_screen(Vec2::new(unit.x, unit.y))) as usize;
        }
        for i in 0..enemies.len() {
            visible += minimap_enemy_at(&map, map.rect.center(), &enemies[i..i + 1]).is_some() as usize;
        }
    }
    let per_frame = started.elapsed().as_secs_f32() / 200.0 * 1000.0;
    assert!(visible > 0);
    assert!(per_frame < 1.0, "{per_frame} ms");
}

