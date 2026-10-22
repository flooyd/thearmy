//saves of busy moments, to load in the real game and look at (for screenshots). nothing here checks anything:
//LOOK_SCENES_DIR=<folder> cargo test --release write_scenes_to_look_at -- --ignored
//it writes horde.sav, boss.sav, champions.sav, chest.sav, freeze.sav, pincer.sav, hold.sav, rout.sav and contest.sav there

use super::*;

//an army of `count` units crowded round `at`, where the player is, in a world with the enemies of the wilds about
fn army_at(count: usize, at: Vec2, zoom: f32) -> Snapshot {
    let mut world = new_run();
    world.player = at;
    world.zoom = zoom;
    let reach = 60.0 + (count as f32).sqrt() * 11.0;
    world.units = (0..count)
        .map(|_| {
            let at = random_position_around(at.x, at.y, 30.0, reach);
            Unit::new(at.x, at.y, 400.0)
        })
        .collect();
    world.enemies.clear();
    respawn_enemies(&mut world.enemies, at.x, at.y, count * 2);
    world.run = Run { destroyed: 4120, lost: 380, peak_army: count + 300, bosses: 2, nests: 7, villages: 4, seconds: 1480.0, citadels: 0 };
    world.wall_points = 340.0;
    for relic in [Relic::Whetstone, Relic::Whetstone, Relic::IronResolve, Relic::FleetFeet, Relic::Magnet, Relic::Bounty, Relic::Thorns, Relic::Bloodlust, Relic::Freezing, Relic::Freezing, Relic::Aegis, Relic::Archers] {
        world.upgrades.take(relic);
    }
    //a line of walls with towers in it, behind the army
    let (col, row) = ((at.x / TILE_SIZE) as i32, (at.y / TILE_SIZE) as i32);
    for c in col - 16..=col + 16 {
        let kind = if (c - col) % 8 == 0 { WallKind::Tower } else { WallKind::Wall };
        let wall = Wall::made(c, row + 11, kind, Side::Friendly, 500.0);
        world.walls.insert((wall.col, wall.row), wall);
    }
    world
}

fn horde_of(name: &str, from: Vec2, army: usize) -> Vec<Enemy> {
    make_horde(horde_named(name).unwrap(), from, army, army * 2)
}

#[test]
#[ignore]
fn write_scenes_to_look_at() {
    let Ok(dir) = std::env::var("LOOK_SCENES_DIR") else {
        return;
    };
    let write = |name: &str, world: &Snapshot| std::fs::write(format!("{dir}/{name}.sav"), world.to_text()).unwrap();
    let field = Vec2::splat(WORLD_SIZE / 2.0) + Vec2::new(3200.0, 2400.0);

    //three hordes closing in on a big army at once
    rand::srand(101);
    let mut world = army_at(2800, field, 0.5);
    world.enemies.extend(horde_of("Swarm Tide", field + Vec2::new(1000.0, -150.0), 2800));
    world.enemies.extend(horde_of("Iron Column", field + Vec2::new(-950.0, -500.0), 2800));
    world.enemies.extend(horde_of("Wildfire", field + Vec2::new(150.0, -1050.0), 2800));
    world.hordes.next_in = 40.0;
    write("horde", &world);

    //a boss on top of the army
    rand::srand(102);
    let mut world = army_at(1800, field, 0.42);
    let boss = Enemy::boss(&BOSSES[3], 1800, 1, field + Vec2::new(60.0, -820.0));
    world.director = Director { next_boss_in: 100.0, slain: 3, boss: Some(boss.id), warned: true };
    world.enemies.push(boss);
    world.run.bosses = 3;
    write("boss", &world);

    //champions of every kind, chests in the field, a horde about to arrive
    rand::srand(103);
    let mut world = army_at(1100, field, 0.6);
    for kind in 0..CHAMPIONS.len() {
        let spot = field + Vec2::from_angle(kind as f32 * 0.45) * (90.0 + (kind % 3) as f32 * 35.0);
        let mut champion = champion_unit(kind, spot);
        champion.level = 1 + (kind as u8 % 4);
        champion.health = champion.max_health();
        world.units.push(champion);
    }
    world.hordes.chests = vec![
        Chest::new(field + Vec2::new(520.0, -260.0), 3, 60.0),
        Chest::new(field + Vec2::new(-560.0, -120.0), 2, 60.0),
        Chest::new(field + Vec2::new(250.0, 420.0), 1, 60.0),
    ];
    world.hordes.coming = Some(Coming { def: horde_named("Stampede").unwrap(), at: field + Vec2::new(-2800.0, -1200.0), left: 4.0, other: None });
    write("champions", &world);

    //the relics from a gold chest, over a fight that has stopped
    rand::srand(104);
    let mut world = army_at(2200, field, 0.55);
    world.enemies.extend(horde_of("Titan March", field + Vec2::new(700.0, -200.0), 2200));
    world.enemies.extend(horde_of("Stampede", field + Vec2::new(-650.0, 150.0), 2200));
    world.choice = Some(RelicChoice::from_chest(3));
    write("chest", &world);

    //a small army at close range with a lot of Freezing relics, so most of what it fights is frozen and breaks into shards
    rand::srand(105);
    let mut world = army_at(450, field, 0.9);
    for _ in 0..4 {
        world.upgrades.take(Relic::Freezing);
    }
    world.enemies.clear();
    for (i, name) in ["Brute", "Titan", "Grunt", "Brute", "Charger", "Grunt", "Titan", "Brute", "Grunt", "Brute"].into_iter().enumerate() {
        let at = field + Vec2::from_angle(i as f32 * 0.63) * (330.0 + (i % 3) as f32 * 90.0);
        let mut enemy = Enemy::new(at, kind_named(name), 1.4);
        enemy.chasing = true;
        world.enemies.push(enemy);
    }
    write("freeze", &world);

    //a pincer: one side, behind its captain, in view, and the other far off the other way (its captain only an arrow at the edge)
    rand::srand(106);
    let mut world = army_at(1500, field, 0.4);
    let def = horde_named("Pincer Host").unwrap();
    let (near, far) = (field + Vec2::new(1350.0, 120.0), field + Vec2::new(-3300.0, -250.0));
    let pincer = make_pincer(def, near, far, 1500, 3000);
    let worth = pincer.iter().map(kill_points).sum();
    world.hordes.fight = Some(Fight { def, total: pincer.len(), lost: 0, worth, last_at: near });
    world.enemies.extend(pincer);
    write("pincer", &world);

    //a gold chest at the army's feet, to be held, and a silver one off to the side
    rand::srand(107);
    let mut world = army_at(500, field, 0.9);
    world.enemies.clear();
    world.hordes.chests = vec![Chest::new(field + Vec2::new(60.0, 10.0), 3, 80.0), Chest::new(field + Vec2::new(-600.0, 200.0), 2, 80.0)];
    write("hold", &world);

    //a captain with a sliver of health left, in the army: it falls at once, and the crowd round it breaks and runs
    rand::srand(108);
    let mut world = army_at(1500, field, 0.4);
    let def = horde_named("Pincer Host").unwrap();
    let (near, far) = (field + Vec2::new(760.0, 0.0), field + Vec2::new(-3300.0, -250.0));
    let mut pincer = make_pincer(def, near, far, 1500, 3000);
    if let Some(captain) = pincer.iter_mut().find(|enemy| enemy.is_captain() && enemy.position.distance(near) < 1.0) {
        captain.health = 1.0;
        captain.shield = 0.0;
    }
    let worth = pincer.iter().map(kill_points).sum();
    world.hordes.fight = Some(Fight { def, total: pincer.len(), lost: 0, worth, last_at: near });
    world.enemies.extend(pincer);
    write("rout", &world);

    //a gold chest under the army with tough enemies beside it, so the hold is stood still
    rand::srand(109);
    let mut world = army_at(500, field, 0.9);
    world.enemies.clear();
    world.hordes.chests = vec![Chest::new(field + Vec2::new(40.0, 10.0), 3, 80.0)];
    for (k, name) in ["Brute", "Titan", "Brute"].into_iter().enumerate() {
        let mut enemy = Enemy::new(field + Vec2::from_angle(k as f32 * 2.1 + 0.4) * 300.0, kind_named(name), 1.0);
        enemy.max_health = 1.0e7;
        enemy.health = enemy.max_health;
        enemy.chasing = true;
        world.enemies.push(enemy);
    }
    write("contest", &world);
}
