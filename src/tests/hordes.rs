//hordes: announced, then there all at once and running at the player from one direction

use super::*;

const CENTRE: Vec2 = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);

#[test]
fn a_horde_is_as_big_as_the_army_calls_for_and_never_bigger_than_it_can_be() {
    for def in HORDES.iter() {
        assert!(def.count(0) >= def.base.min(def.most) && def.count(10_000_000) == def.most, "{}", def.name);
        let mut last = 0;
        for army in [0, 100, 500, 2000, 8000, 20_000] {
            let count = def.count(army);
            assert!(count >= last && count <= def.most, "{} at {army}", def.name);
            last = count;
        }
        for (kind, share) in def.kinds {
            assert!(*share > 0.0 && ENEMY_KINDS.iter().any(|known| known.name == *kind), "{kind} is an enemy");
        }
    }
    //the first one a small army meets is the easy one, and the dangerous ones wait for an army that can take them
    assert!(HORDES.iter().any(|def| def.from_army == 0));
    assert!(random_horde(10).from_army <= 10);
    for _ in 0..100 {
        assert!(random_horde(60).from_army <= 60);
    }
}

#[test]
fn the_compass_names_the_way_a_horde_comes_from_with_north_up_the_screen() {
    for (direction, name) in [(Vec2::X, "EAST"), (Vec2::Y, "SOUTH"), (-Vec2::X, "WEST"), (-Vec2::Y, "NORTH"), (Vec2::new(1.0, -1.0), "NORTH-EAST"), (Vec2::new(-1.0, 1.0), "SOUTH-WEST")] {
        assert_eq!(compass(direction), name);
    }
}

#[test]
fn a_horde_is_announced_with_its_direction_and_arrives_after_the_warning() {
    rand::srand(3);
    let mut hordes = Hordes::new();
    let mut enemies = Vec::new();
    let army = 400;
    //nothing before the first is due
    assert!(hordes.update(&mut enemies, CENTRE, army, army, false, HORDE_FIRST_SECONDS - 1.0).is_none() && hordes.coming.is_none());
    let beat = hordes.update(&mut enemies, CENTRE, army, army, false, 2.0);
    let (name, from) = match beat {
        Some(HordeBeat::Warned { name, from }) => (name, from),
        _ => panic!("a horde is announced when its time comes"),
    };
    let coming = hordes.coming.expect("and it is on its way");
    assert_eq!(coming.def.name, name);
    assert_eq!(compass(coming.at - CENTRE), from);
    assert!((coming.at.distance(CENTRE) - HORDE_DISTANCE_TILES * TILE_SIZE).abs() < 1.0, "it comes from far off");
    assert!(enemies.is_empty(), "nobody yet");
    assert_eq!(hordes.heading(&enemies, CENTRE), Some(coming.at), "the arrow points where they will come from");
    //nothing happens until the warning is up
    assert!(hordes.update(&mut enemies, CENTRE, army, army, false, HORDE_WARNING_SECONDS - 1.0).is_none() && enemies.is_empty());
    let beat = hordes.update(&mut enemies, CENTRE, army, army, false, 1.5);
    let count = match beat {
        Some(HordeBeat::Arrived { name: arrived, count }) => {
            assert_eq!(arrived, name);
            count
        }
        _ => panic!("and then they are there"),
    };
    assert_eq!(count, coming.def.count(army));
    assert_eq!(enemies.len(), count);
    assert!(hordes.coming.is_none() && hordes.next_in >= HORDE_GAP_SECONDS.0 && hordes.next_in <= HORDE_GAP_SECONDS.1, "{}", hordes.next_in);
    let spread = HORDE_SPREAD + (count as f32).sqrt() * HORDE_SPREAD_PER_ROOT;
    for enemy in &enemies {
        assert!(in_a_horde(enemy) && !is_wild(enemy), "a horde is nobody's wild enemy, so the ones kept up don't count it");
        assert!(enemy.chasing && (enemy.aggro >= HORDE_AGGRO_TILES * TILE_SIZE || enemy.behavior == Behavior::Charger), "they run at the player from the start");
        assert!(enemy.position.distance(coming.at) <= spread + 1.0);
        assert!(coming.def.kinds.iter().any(|(kind, _)| *kind == enemy.name), "{}", enemy.name);
    }
    //the arrow keeps pointing at them until they are on top of the army
    let middle = enemies.iter().map(|enemy| enemy.position).sum::<Vec2>() / enemies.len() as f32;
    assert!(hordes.heading(&enemies, CENTRE).is_some_and(|at| at.distance(middle) < 1.0));
    let mut arrived = enemies.clone();
    for enemy in &mut arrived {
        enemy.position = CENTRE + Vec2::new(100.0, 0.0);
    }
    assert!(hordes.heading(&arrived, CENTRE).is_none(), "no arrow for a horde that is here");
}

#[test]
fn no_horde_comes_while_a_boss_is_about_or_for_an_army_too_small_to_fear_one() {
    rand::srand(4);
    let mut enemies = Vec::new();
    let mut hordes = Hordes::new();
    assert!(hordes.update(&mut enemies, CENTRE, HORDE_MIN_ARMY - 1, 50, false, 1000.0).is_none() && hordes.coming.is_none());
    assert!(hordes.update(&mut enemies, CENTRE, 500, 500, true, 1000.0).is_none() && hordes.coming.is_none(), "a boss is enough to deal with");
    assert_eq!(hordes.next_in, HORDE_FIRST_SECONDS, "the time to the next doesn't run down while it can't come");
    hordes.next_in = 0.0;
    assert!(matches!(hordes.update(&mut enemies, CENTRE, 500, 500, false, 0.1), Some(HordeBeat::Warned { .. })), "as soon as there is neither");
}

#[test]
fn a_horde_that_is_left_far_behind_is_let_go_and_the_rest_are_kept() {
    let mut hordes = Hordes::new();
    let mut enemies: Vec<Enemy> = (0..5).map(|i| kind_at("Grunt", CENTRE + Vec2::new(200.0 * i as f32, 0.0))).collect();
    for enemy in &mut enemies[..3] {
        enemy.site = Some(HORDE_SITE);
    }
    enemies[0].position = CENTRE + Vec2::new(ENEMY_LEASH_TILES * TILE_SIZE + 500.0, 0.0);
    enemies[3].position = CENTRE + Vec2::new(ENEMY_LEASH_TILES * TILE_SIZE + 500.0, 100.0); //(a wild one, which has its own leash)
    hordes.update(&mut enemies, CENTRE, 500, 500, true, 0.016);
    assert_eq!(enemies.len(), 4);
    assert!(enemies.iter().filter(|enemy| in_a_horde(enemy)).count() == 2);
    assert!(enemies.iter().any(|enemy| enemy.position.y == CENTRE.y + 100.0), "wild enemies are left to the code that keeps them");
}

#[test]
fn a_horde_is_made_for_the_pressure_it_is_given_and_a_bigger_army_means_stronger_enemies() {
    rand::srand(6);
    let def = &HORDES[1];
    let health = |pressure: usize| {
        let horde = make_horde(def, CENTRE, 300, pressure);
        horde.iter().filter(|enemy| enemy.modifier == Modifier::None).map(|enemy| enemy.max_health / ENEMY_KINDS.iter().find(|kind| kind.name == enemy.name).unwrap().health).sum::<f32>() / horde.len() as f32
    };
    assert!(health(6000) > health(60) * 2.0, "{} against {}", health(6000), health(60));
}

#[test]
fn the_places_hordes_come_from_are_all_inside_the_world() {
    rand::srand(8);
    for corner in [Vec2::splat(1500.0), Vec2::new(WORLD_SIZE - 1500.0, 1500.0), CENTRE] {
        for _ in 0..50 {
            if let Some(at) = horde_spot(corner) {
                let edge = HORDE_EDGE_TILES * TILE_SIZE;
                assert!(at.x > edge && at.y > edge && at.x < WORLD_SIZE - edge && at.y < WORLD_SIZE - edge);
                assert!((at.distance(corner) - HORDE_DISTANCE_TILES * TILE_SIZE).abs() < 1.0);
            }
        }
    }
    assert!(horde_spot(CENTRE).is_some());
}
