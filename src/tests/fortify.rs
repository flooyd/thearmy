//the kinds of wall and who they stop, building with wall points, wall damage, towers, spikes and tar

use super::*;

pub(super) fn made(kind: WallKind, side: Side, col: i32, row: i32) -> Wall {
    Wall::made(col, row, kind, side, 500.0)
}

pub(super) fn walls_with(list: Vec<Wall>) -> Walls {
    list.into_iter().map(|wall| ((wall.col, wall.row), wall)).collect()
}

//a spot well away from the starting house
pub(super) const FAR: Vec2 = Vec2::new(20000.0, 20000.0);

#[test]
fn each_kind_of_wall_stops_the_right_movers() {
    use Mover::*;
    use Side::*;
    use WallKind::*;
    //(kind, side, stops the army, stops enemies)
    let table = [
        (Wall, Friendly, true, true),
        (Wall, Hostile, true, false),
        (Gate, Friendly, false, true),
        (Tower, Friendly, true, true),
        (Spikes, Friendly, false, false),
        (Tar, Friendly, false, false),
        (Barrier, Hostile, true, false),
    ];
    for (kind, side, army, enemy) in table {
        let wall = made(kind, side, 5, 5);
        assert_eq!(wall.blocks(Ally), army, "{kind:?} {side:?} for the army");
        assert_eq!(wall.blocks(Enemy), enemy, "{kind:?} {side:?} for enemies");
    }
}

#[test]
fn a_gate_lets_the_army_through_and_keeps_enemies_out() {
    let walls = walls_with((0..5).map(|k| made(WallKind::Gate, Side::Friendly, 300, 300 + k)).collect());
    let from = Vec2::new(300.0 * TILE_SIZE - 100.0, 302.5 * TILE_SIZE);
    let to = from + Vec2::new(300.0, 0.0);
    let army = move_clear(from, to, RADIUS, Mover::Ally, &walls);
    assert!(army.x > from.x + 290.0, "the army was stopped at {army}");
    let enemy = move_clear(from, to, RADIUS, Mover::Enemy, &walls);
    assert!(enemy.x < 300.0 * TILE_SIZE - RADIUS + 0.1, "an enemy got to {enemy}");
}

#[test]
fn a_hostile_wall_stops_the_army_and_not_enemies() {
    let walls = walls_with((0..5).map(|k| made(WallKind::Wall, Side::Hostile, 300, 300 + k)).collect());
    let from = Vec2::new(300.0 * TILE_SIZE - 100.0, 302.5 * TILE_SIZE);
    let to = from + Vec2::new(300.0, 0.0);
    assert!(move_clear(from, to, RADIUS, Mover::Ally, &walls).x < 300.0 * TILE_SIZE);
    assert!(move_clear(from, to, RADIUS, Mover::Enemy, &walls).x > from.x + 290.0);
}

#[test]
fn spikes_and_tar_can_be_walked_over_by_anyone() {
    for kind in [WallKind::Spikes, WallKind::Tar] {
        let walls = walls_with((0..5).map(|k| made(kind, Side::Friendly, 300, 300 + k)).collect());
        let from = Vec2::new(300.0 * TILE_SIZE - 100.0, 302.5 * TILE_SIZE);
        for mover in [Mover::Ally, Mover::Enemy] {
            assert!(move_clear(from, from + Vec2::new(300.0, 0.0), RADIUS, mover, &walls).x > from.x + 290.0, "{kind:?} {mover:?}");
        }
    }
}

#[test]
fn what_a_route_through_a_wall_costs_depends_on_who_is_going() {
    let wall = made(WallKind::Wall, Side::Friendly, 1, 1);
    assert_eq!(wall.route_cost(Mover::Ally), None);
    assert_eq!(wall.route_cost(Mover::Enemy), Some(ENEMY_CHEW_COST));
    let lair = made(WallKind::Wall, Side::Hostile, 1, 1);
    assert_eq!(lair.route_cost(Mover::Ally), Some(UNIT_CHEW_COST));
    assert_eq!(lair.route_cost(Mover::Enemy), Some(0));
    assert_eq!(made(WallKind::Gate, Side::Friendly, 1, 1).route_cost(Mover::Enemy), Some(GATE_CHEW_COST));
    assert_eq!(made(WallKind::Gate, Side::Friendly, 1, 1).route_cost(Mover::Ally), Some(0));
    assert_eq!(made(WallKind::Tower, Side::Friendly, 1, 1).route_cost(Mover::Enemy), Some(TOWER_CHEW_COST));
    assert_eq!(made(WallKind::Spikes, Side::Friendly, 1, 1).route_cost(Mover::Enemy), Some(FLOOR_AVOID_COST));
    assert_eq!(made(WallKind::Spikes, Side::Friendly, 1, 1).route_cost(Mover::Ally), Some(0));
}

#[test]
fn the_army_s_route_goes_through_a_lair_s_wall_when_going_round_is_longer_and_stops_at_its_own() {
    //a long wall between a unit and the player, with the way round far longer than breaking through
    let (col, row) = tile_of(FAR.x, FAR.y);
    let line: Vec<(i32, i32)> = (row - 30..=row + 30).map(|r| (col, r)).collect();
    let player = tile_center((col + 2, row));
    let from = tile_center((col - 2, row));
    for (side, passable) in [(Side::Hostile, true), (Side::Friendly, false)] {
        let walls = walls_with(line.iter().map(|&(c, r)| made(WallKind::Wall, side, c, r)).collect());
        let mut field = FlowField::new();
        field.compute(player, &walls);
        let i = field.index(col - 2, row).unwrap();
        assert!(field.cost[i] != u32::MAX);
        let route_crosses = {
            let mut tile = (col - 2, row);
            let mut crossed = false;
            while let Some(next) = field.next_tile(tile) {
                crossed |= next.0 == col && next.1 == row;
                tile = next;
            }
            crossed
        };
        assert_eq!(route_crosses, passable, "{side:?}: the route {} the wall", if route_crosses { "crosses" } else { "avoids" });
        let _ = from;
    }
}

#[test]
fn shots_stop_at_the_player_s_walls_and_wear_them_and_fly_through_a_lair_s() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let start = tile_center((col - 3, row));
    for (side, stopped) in [(Side::Friendly, true), (Side::Hostile, false)] {
        let walls = walls_with(vec![made(WallKind::Wall, side, col, row)]);
        let mut shots = vec![Shot::new(start, Vec2::new(SHOT_SPEED, 0.0))];
        let mut fx = Effects::default();
        for _ in 0..60 {
            update_shots(&mut shots, &mut [], &walls, 1.0 / 60.0, &Upgrades::default(), &mut fx);
        }
        assert_eq!(shots.is_empty(), stopped, "{side:?}");
        assert_eq!(fx.wall_hits.len(), stopped as usize);
        if stopped {
            assert_eq!(fx.wall_hits[0].0, (col, row));
            assert!((fx.wall_hits[0].1 - SHOT_DAMAGE * SHOT_WALL_SHARE).abs() < 0.01);
        }
    }
}

#[test]
fn a_blast_hurts_the_player_s_walls_in_its_reach_and_no_others() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let at = tile_center((col, row));
    let mut walls = walls_with(vec![
        made(WallKind::Wall, Side::Friendly, col, row),
        made(WallKind::Wall, Side::Friendly, col + 1, row),
        made(WallKind::Wall, Side::Friendly, col + 8, row),
        made(WallKind::Wall, Side::Hostile, col, row + 1),
    ]);
    let mut hazards = vec![hazard_at(at, 120.0, 0.0, 100.0, 5)];
    let mut fx = Effects::default();
    update_hazards(&mut hazards, &mut [], &Upgrades::default(), 0.016, &mut fx);
    assert_eq!(fx.blasts.len(), 1);
    update_walls(&mut walls, &mut [], 0.016, &mut fx);
    let expected = 500.0 - 100.0 * BLAST_WALL_FACTOR;
    assert!((walls[&(col, row)].health - expected).abs() < 0.01, "{}", walls[&(col, row)].health);
    assert!((walls[&(col + 1, row)].health - expected).abs() < 0.01);
    assert_eq!(walls[&(col + 8, row)].health, 500.0, "too far");
    assert_eq!(walls[&(col, row + 1)].health, 500.0, "a lair's wall is not the player's");
}

#[test]
fn a_boss_s_blast_breaks_walls_built_for_the_army_it_is_scaled_to() {
    let hazard = hazard_at(FAR, 100.0, 0.0, 100.0, 3);
    let plain = hazard.wall_damage;
    assert_eq!(hazard.vs_walls(4.0).wall_damage, plain * 4.0);
}

#[test]
fn a_wall_that_takes_enough_damage_is_gone_and_a_tower_lets_its_garrison_out() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let mut tower = made(WallKind::Tower, Side::Friendly, col, row);
    tower.garrison = 2;
    let mut walls = walls_with(vec![tower, made(WallKind::Wall, Side::Friendly, col + 3, row)]);
    let at = tile_center((col, row));
    let mut units = vec![Unit::new(at.x, at.y, 100.0), Unit::new(at.x, at.y, 100.0), Unit::new(at.x + 500.0, at.y, 100.0)];
    units[0].garrison = Some((col, row));
    units[1].garrison = Some((col, row));
    let mut fx = Effects::default();
    fx.wall_hits.push(((col, row), 1.0e6));
    fx.wall_hits.push(((col + 3, row), 100.0));
    assert!(update_walls(&mut walls, &mut units, 0.016, &mut fx));
    assert!(!walls.contains_key(&(col, row)));
    assert_eq!(walls[&(col + 3, row)].health, 400.0, "the other wall only took a hit");
    assert!(units.iter().all(|unit| unit.garrison.is_none()));
    assert!(units[0].x != at.x || units[0].y != at.y, "it came out");
    assert!(fx.cues.contains(&Cue::WallBreak));
    assert!(fx.wall_hits.is_empty() && fx.blasts.is_empty());
    //and nothing broke the next time
    assert!(!update_walls(&mut walls, &mut units, 0.016, &mut fx));
}

#[test]
fn a_barrier_crumbles_by_itself_when_its_time_is_up() {
    let mut barrier = made(WallKind::Barrier, Side::Hostile, 5, 5);
    barrier.life = 1.0;
    let mut walls = walls_with(vec![barrier]);
    let mut fx = Effects::default();
    assert!(!update_walls(&mut walls, &mut [], 0.6, &mut fx));
    assert!(update_walls(&mut walls, &mut [], 0.6, &mut fx));
    assert!(walls.is_empty());
}

#[test]
fn walls_are_tougher_for_a_bigger_army_and_masonry_makes_them_tougher_still() {
    let up = Upgrades::default();
    let small = wall_health(WallKind::Wall, 5, &up);
    let big = wall_health(WallKind::Wall, 1000, &up);
    assert_eq!(small, WALL_HEALTH);
    assert!(big > small * 5.0, "{big} next to {small}");
    let mut masonry = Upgrades::default();
    masonry.masonry = 1.6;
    assert!((wall_health(WallKind::Wall, 1000, &masonry) - big * 1.6).abs() < 0.01);
    assert!(wall_health(WallKind::Tower, 100, &up) > wall_health(WallKind::Wall, 100, &up));
    assert!(wall_health(WallKind::Gate, 100, &up) < wall_health(WallKind::Wall, 100, &up));
    assert_eq!(wall_health(WallKind::Spikes, 1, &up), wall_health(WallKind::Spikes, 6000, &up));
}

#[test]
fn building_spends_wall_points_and_stops_when_there_are_not_enough() {
    let mut walls = Walls::default();
    let mut points = 3.0;
    let up = Upgrades::default();
    let player = FAR;
    let (col, row) = tile_of(FAR.x + 500.0, FAR.y);
    assert_eq!(build(&mut walls, WallKind::Wall, col, row, player, &[], &[], &mut points, 10, &up), Built::Placed);
    assert_eq!(points, 2.0);
    assert_eq!(build(&mut walls, WallKind::Gate, col + 1, row, player, &[], &[], &mut points, 10, &up), Built::Placed);
    assert_eq!(points, 0.0);
    assert_eq!(build(&mut walls, WallKind::Wall, col + 2, row, player, &[], &[], &mut points, 10, &up), Built::Poor);
    assert_eq!(walls.len(), 2);
    //a tower is expensive
    let mut points = 5.0;
    assert_eq!(build(&mut walls, WallKind::Tower, col + 2, row, player, &[], &[], &mut points, 10, &up), Built::Poor);
    points = 8.0;
    assert_eq!(build(&mut walls, WallKind::Tower, col + 2, row, player, &[], &[], &mut points, 10, &up), Built::Placed);
    assert_eq!(walls[&(col + 2, row)].kind, WallKind::Tower);
    assert_eq!(walls[&(col + 2, row)].health, wall_health(WallKind::Tower, 10, &up));
}

#[test]
fn building_where_the_player_or_a_unit_stands_is_refused_but_a_gate_goes_anywhere() {
    let up = Upgrades::default();
    let mut walls = Walls::default();
    let mut points = 50.0;
    let tile = tile_of(FAR.x, FAR.y);
    let units = [Unit::new(FAR.x, FAR.y, 100.0)];
    assert_eq!(build(&mut walls, WallKind::Wall, tile.0, tile.1, FAR + Vec2::new(900.0, 0.0), &units, &[], &mut points, 10, &up), Built::Nothing);
    assert_eq!(build(&mut walls, WallKind::Wall, tile.0, tile.1, FAR, &[], &[], &mut points, 10, &up), Built::Nothing);
    assert_eq!(points, 50.0, "nothing was spent");
    assert_eq!(build(&mut walls, WallKind::Gate, tile.0, tile.1, FAR, &units, &[], &mut points, 10, &up), Built::Placed);
    //a unit inside a tower is not in the way
    let mut inside = Unit::new(FAR.x + 300.0, FAR.y, 100.0);
    inside.garrison = Some((1, 1));
    let tile = tile_of(inside.x, inside.y);
    assert_eq!(build(&mut walls, WallKind::Wall, tile.0, tile.1, FAR + Vec2::new(900.0, 0.0), &[inside], &[], &mut points, 10, &up), Built::Placed);
}

#[test]
fn a_hurt_wall_is_mended_for_a_point_and_made_as_strong_as_a_new_one() {
    let up = Upgrades::default();
    let (col, row) = tile_of(FAR.x, FAR.y);
    let mut walls = walls_with(vec![Wall::new(col, row)]);
    walls.get_mut(&(col, row)).unwrap().health = 100.0;
    let mut points = 5.0;
    //built for a small army, mended for a big one
    assert_eq!(build(&mut walls, WallKind::Wall, col, row, FAR + Vec2::new(900.0, 0.0), &[], &[], &mut points, 1000, &up), Built::Repaired);
    assert_eq!(points, 5.0 - REPAIR_COST);
    assert_eq!(walls[&(col, row)].health, wall_health(WallKind::Wall, 1000, &up));
    assert_eq!(walls[&(col, row)].max_health, walls[&(col, row)].health);
    //a wall that isn't hurt is left alone and costs nothing
    assert_eq!(build(&mut walls, WallKind::Wall, col, row, FAR + Vec2::new(900.0, 0.0), &[], &[], &mut points, 1000, &up), Built::Nothing);
    assert_eq!(points, 5.0 - REPAIR_COST);
    //and one that is hurt can't be mended with no points
    walls.get_mut(&(col, row)).unwrap().health = 1.0;
    points = 0.5;
    assert_eq!(build(&mut walls, WallKind::Wall, col, row, FAR + Vec2::new(900.0, 0.0), &[], &[], &mut points, 1000, &up), Built::Poor);
    //a lair's wall is not the player's to mend
    let mut lair = walls_with(vec![made(WallKind::Wall, Side::Hostile, col, row)]);
    lair.get_mut(&(col, row)).unwrap().health = 1.0;
    points = 5.0;
    assert_eq!(build(&mut lair, WallKind::Wall, col, row, FAR + Vec2::new(900.0, 0.0), &[], &[], &mut points, 1000, &up), Built::Nothing);
}

#[test]
fn deleting_a_wall_gives_back_half_of_what_it_cost_less_for_a_hurt_one() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let mut mason = Mason::new();
    mason.points = 0.0;
    let mut walls = walls_with(vec![made(WallKind::Tower, Side::Friendly, col, row), made(WallKind::Tower, Side::Friendly, col + 1, row), made(WallKind::Wall, Side::Hostile, col + 2, row)]);
    walls.get_mut(&(col + 1, row)).unwrap().health = 250.0;
    assert!(delete_wall(&mut mason, &mut walls, &mut [], (col, row)));
    assert!((mason.points - WallKind::Tower.cost() * WALL_REFUND_SHARE).abs() < 0.001);
    assert!(delete_wall(&mut mason, &mut walls, &mut [], (col + 1, row)));
    assert!((mason.points - WallKind::Tower.cost() * WALL_REFUND_SHARE * 1.5).abs() < 0.001, "{}", mason.points);
    //a lair's wall is not the player's, and an empty square has nothing to delete
    assert!(!delete_wall(&mut mason, &mut walls, &mut [], (col + 2, row)));
    assert!(!delete_wall(&mut mason, &mut walls, &mut [], (col + 5, row)));
    assert_eq!(walls.len(), 1);
}

#[test]
fn wall_points_are_capped_and_kills_earn_them() {
    let mut mason = Mason::new();
    assert_eq!(mason.points, START_WALL_POINTS);
    mason.earn(5000.0);
    assert_eq!(mason.points, MAX_WALL_POINTS);
    //a plain enemy, a rich one and a boss, but not a minion
    let up = Upgrades::default();
    let mut fx = Effects::default();
    let mut babies = Vec::new();
    let mut plain = plain_enemy(FAR);
    plain.health = 0.0;
    let mut rich = plain_enemy(FAR);
    rich.health = 0.0;
    rich.babies = 20;
    let mut minion = plain_enemy(FAR);
    minion.health = 0.0;
    minion.owner = Some(999_999);
    minion.babies = 0;
    let mut enemies = vec![plain, rich, minion];
    resolve_deaths(&mut enemies, &mut babies, 10, &up, &mut fx);
    assert!((fx.spoils[0].points - (POINTS_PER_KILL + POINTS_PER_BABY) - (POINTS_PER_KILL + 20.0 * POINTS_PER_BABY)).abs() < 0.001, "{}", fx.spoils[0].points);
    let mut boss = Enemy::boss(&BOSSES[0], 100, 0, FAR);
    boss.health = 0.0;
    let babies_of_boss = boss.babies as f32;
    let mut fx = Effects::default();
    resolve_deaths(&mut vec![boss], &mut babies, 100, &up, &mut fx);
    assert!((fx.spoils[0].points - (POINTS_PER_KILL + babies_of_boss * POINTS_PER_BABY + BOSS_POINTS)).abs() < 0.001);
}

#[test]
fn a_unit_inside_a_tower_is_safe_stays_put_and_is_not_drawn_into_a_crowd() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let at = tile_center((col, row));
    let mut walls = walls_with(vec![made(WallKind::Tower, Side::Friendly, col, row)]);
    walls.get_mut(&(col, row)).unwrap().garrison = 1;
    let mut inside = Unit::new(at.x, at.y, 300.0);
    inside.garrison = Some((col, row));
    let mut units = vec![inside, Unit::new(at.x + 5.0, at.y, 300.0)];
    //an enemy sitting on both of them hurts only the one outside
    let mut enemies = vec![striker(at, 1000.0)];
    let mut fx = Effects::default();
    let mut babies = Vec::new();
    fight(&mut enemies, &mut units, &mut babies, 0.1, &Upgrades::default(), &mut fx);
    assert_eq!(units[0].health, UNIT_HEALTH);
    assert!(units[1].health < UNIT_HEALTH);
    //it doesn't walk to the player, nor get pushed out of the tower or by its neighbour
    let field = FlowField::new();
    for _ in 0..60 {
        update_units(&mut units, at.x + 900.0, at.y, 1.0 / 60.0, &walls, &field, 1.0, &Squads::new());
    }
    assert_eq!((units[0].x, units[0].y), (at.x, at.y));
    assert!(units[1].x > at.x + 100.0);
}

#[test]
fn manning_a_tower_takes_the_closest_free_units_in_reach_and_the_key_again_lets_them_out() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let at = tile_center((col, row));
    let mut walls = walls_with(vec![made(WallKind::Tower, Side::Friendly, col, row), made(WallKind::Wall, Side::Friendly, col + 5, row)]);
    let mut units: Vec<Unit> = (0..20).map(|k| Unit::new(at.x + 100.0 + k as f32 * 10.0, at.y + 80.0, 100.0)).collect();
    units.push(Unit::new(at.x + TOWER_REACH_TILES * TILE_SIZE + 400.0, at.y, 100.0)); //too far
    units.push(Unit::new(at.x + 50.0, at.y + 80.0, 100.0)); //closest, but sent at an enemy
    let last = units.len() - 1;
    units[last].target = Some(7);
    let mut fx = Effects::default();
    assert_eq!(man_tower(&mut walls, &mut units, (col, row), &mut fx), TOWER_CAPACITY);
    assert_eq!(walls[&(col, row)].garrison, TOWER_CAPACITY);
    let inside: Vec<usize> = (0..units.len()).filter(|&i| units[i].garrison == Some((col, row))).collect();
    assert_eq!(inside.len(), TOWER_CAPACITY);
    assert!(inside.iter().all(|&i| i < 20 && (units[i].x, units[i].y) == (at.x, at.y)));
    assert!(inside.iter().all(|&i| i < TOWER_CAPACITY), "the closest ones went in: {inside:?}");
    assert!(units[last].garrison.is_none() && units[last - 1].garrison.is_none());
    //not a tower, nothing there
    assert_eq!(man_tower(&mut walls, &mut units, (col + 5, row), &mut fx), 0);
    assert_eq!(man_tower(&mut walls, &mut units, (col + 9, row), &mut fx), 0);
    //again
    assert_eq!(man_tower(&mut walls, &mut units, (col, row), &mut fx), TOWER_CAPACITY);
    assert_eq!(walls[&(col, row)].garrison, 0);
    assert!(units.iter().all(|unit| unit.garrison.is_none()));
}

#[test]
fn manning_a_tower_with_nobody_near_says_so() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let mut walls = walls_with(vec![made(WallKind::Tower, Side::Friendly, col, row)]);
    let mut units = vec![Unit::new(FAR.x + 9000.0, FAR.y, 100.0)];
    let mut fx = Effects::default();
    assert_eq!(man_tower(&mut walls, &mut units, (col, row), &mut fx), 0);
    assert!(fx.popups.iter().any(|popup| popup.text.contains("no units")));
    assert_eq!(walls[&(col, row)].garrison, 0);
}

#[test]
fn a_manned_tower_shoots_the_nearest_enemy_in_range_and_an_empty_one_does_not() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let at = tile_center((col, row));
    let mut walls = walls_with(vec![made(WallKind::Tower, Side::Friendly, col, row)]);
    let mut enemies = vec![still_enemy(at + Vec2::new(300.0, 0.0), 1.0e6), still_enemy(at + Vec2::new(200.0, 0.0), 1.0e6), still_enemy(at + Vec2::new(TOWER_RANGE + 400.0, 0.0), 1.0e6)];
    let up = Upgrades::default();
    let mut fx = Effects::default();
    for _ in 0..120 {
        fire_towers(&mut walls, &mut enemies, &up, 1.0 / 60.0, &mut fx);
    }
    assert!(enemies.iter().all(|enemy| enemy.health == 1.0e6), "an empty tower fired");
    walls.get_mut(&(col, row)).unwrap().garrison = 10;
    walls.get_mut(&(col, row)).unwrap().fire_in = 0.0;
    fire_towers(&mut walls, &mut enemies, &up, 1.0 / 60.0, &mut fx);
    let damage = 10.0 * TOWER_DAMAGE * TOWER_INTERVAL;
    assert!((1.0e6 - enemies[1].health - damage).abs() < 1.0, "the nearest took {}", 1.0e6 - enemies[1].health);
    assert_eq!(enemies[0].health, 1.0e6);
    assert_eq!(enemies[2].health, 1.0e6, "out of range");
    assert_eq!(fx.beams.len(), 1);
    assert!(fx.cues.contains(&Cue::Arrow));
    //then it waits its turn
    fire_towers(&mut walls, &mut enemies, &up, 1.0 / 60.0, &mut fx);
    assert_eq!(fx.beams.len(), 1);
    //archery and whetstones add to it
    let mut strong = Upgrades::default();
    strong.archery = 2.0;
    strong.damage = 1.5;
    walls.get_mut(&(col, row)).unwrap().fire_in = 0.0;
    let before = enemies[1].health;
    fire_towers(&mut walls, &mut enemies, &strong, 1.0 / 60.0, &mut fx);
    assert!((before - enemies[1].health - damage * 3.0).abs() < 1.0);
}

#[test]
fn spikes_hurt_the_enemy_standing_on_them_and_wear_out_and_tar_slows_it() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let on = |dx: i32| tile_center((col + dx, row));
    let walls = walls_with(vec![made(WallKind::Spikes, Side::Friendly, col, row), made(WallKind::Tar, Side::Friendly, col + 1, row), made(WallKind::Spikes, Side::Hostile, col + 2, row)]);
    let mut enemies = vec![still_enemy(on(0), 1000.0), still_enemy(on(1), 1000.0), still_enemy(on(2), 1000.0), still_enemy(on(5), 1000.0)];
    let mut boss = Enemy::boss(&BOSSES[0], 100, 0, on(0));
    boss.position = on(0);
    let boss_health = boss.max_health;
    enemies.push(boss);
    let mut fx = Effects::default();
    terrain_effects(&mut enemies, &walls, 1.0, &mut fx);
    assert!((enemies[0].health - (1000.0 - 1000.0 * SPIKE_DAMAGE_SHARE)).abs() < 0.01, "{}", enemies[0].health);
    assert_eq!(enemies[0].status.slow, 1.0);
    assert_eq!(enemies[1].health, 1000.0);
    assert_eq!(enemies[1].status.slow, TAR_SLOW);
    assert_eq!(enemies[2].health, 1000.0, "a lair's spikes are not the player's");
    assert_eq!(enemies[3].status.slow, 1.0);
    let boss_took = boss_health - enemies[4].health;
    assert!((boss_took - boss_health * SPIKE_DAMAGE_SHARE * SPIKE_BOSS_FACTOR).abs() < 1.0, "{boss_took}");
    //two enemies on it wear it down twice as fast, and it goes after enough of that
    assert_eq!(fx.wall_hits.len(), 2);
    assert!(fx.wall_hits.iter().all(|&(tile, wear)| tile == (col, row) && (wear - SPIKE_WEAR).abs() < 0.001));
    //walking off the tar gives the speed back
    enemies[1].position = on(5);
    terrain_effects(&mut enemies, &walls, 1.0, &mut fx);
    assert_eq!(enemies[1].status.slow, 1.0);
}

#[test]
fn units_pressed_against_a_lair_wall_wear_it_down_in_proportion_to_how_many() {
    let (col, row) = tile_of(FAR.x, FAR.y);
    let wall = made(WallKind::Wall, Side::Hostile, col, row);
    let rect = wall.rect();
    let own = made(WallKind::Wall, Side::Friendly, col + 4, row);
    let own_rect = own.rect();
    let walls = walls_with(vec![wall, own]);
    let touching = |count: usize| -> Vec<Unit> { (0..count).map(|k| Unit::new(rect.x - RADIUS, rect.y + 10.0 + k as f32, 100.0)).collect() };
    let up = Upgrades::default();
    let hits = |units: &[Unit]| {
        let mut fx = Effects::default();
        chew_hostile_walls(units, &walls, &up, 0.5, &mut fx);
        fx.wall_hits
    };
    assert!(hits(&[]).is_empty());
    let one = hits(&touching(1));
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].0, (col, row));
    assert!((one[0].1 - UNIT_DAMAGE * UNIT_CHEW_SHARE * 0.5).abs() < 0.01);
    assert!((hits(&touching(10))[0].1 - 10.0 * one[0].1).abs() < 0.1);
    //too far away, or at the player's own wall, or inside a tower, does nothing
    let far = vec![Unit::new(rect.x - 200.0, rect.y + 10.0, 100.0), Unit::new(own_rect.x - RADIUS, own_rect.y + 10.0, 100.0)];
    assert!(hits(&far).is_empty());
    let mut inside = touching(3);
    inside.iter_mut().for_each(|unit| unit.garrison = Some((1, 1)));
    assert!(hits(&inside).is_empty());
    //it needs no hostile walls to be anywhere
    let mut fx = Effects::default();
    chew_hostile_walls(&touching(5), &Walls::default(), &up, 0.5, &mut fx);
    assert!(fx.wall_hits.is_empty());
}

#[test]
fn wall_points_a_kill_earns_are_spent_by_main_with_the_quarry_bonus() {
    let mut mason = Mason::new();
    let mut fx = Effects::default();
    fx.spoils[0].points = 10.0;
    let mut up = Upgrades::default();
    up.quarry = 1.5;
    mason.earn(std::mem::take(&mut fx.spoils[0].points) * up.quarry);
    assert_eq!(mason.points, START_WALL_POINTS + 15.0);
    assert_eq!(fx.spoils[0].points, 0.0);
}

#[test]
fn a_wall_s_share_of_health_left_stays_between_zero_and_one() {
    for kind in [WallKind::Wall, WallKind::Gate, WallKind::Tower, WallKind::Spikes, WallKind::Tar, WallKind::Barrier] {
        for side in [Side::Friendly, Side::Hostile] {
            let mut wall = made(kind, side, 3, 4);
            for health in [500.0, 300.0, 100.0, 10.0, -5.0] {
                wall.health = health;
                assert!((0.0..=1.0).contains(&wall.share_left()));
            }
        }
    }
}
