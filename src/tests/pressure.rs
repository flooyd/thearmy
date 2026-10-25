//how hard the wild enemies are on the army, and that they stay so as the army and the run grow. the table is only run by hand:
//cargo test --release how_long_enemies_last -- --ignored --nocapture

use super::*;

const DT: f32 = 1.0 / 120.0;

//an army of this many units settled round the player the way the game leaves them
fn settled_army(army: usize, player: Vec2) -> Vec<Unit> {
    let walls = Walls::default();
    let field = FlowField::new();
    let squads = Squads::new();
    let mut units: Vec<Unit> = (0..army)
        .map(|_| {
            let at = random_position_around(player.x, player.y, 60.0, 700.0);
            Unit::new(at.x, at.y, 400.0)
        })
        .collect();
    let boost = speed_multiplier(army);
    for _ in 0..360 {
        update_units(&mut units, player.x, player.y, DT, &walls, &field, boost, &squads);
    }
    units
}

//a kind of enemy made for an army this big, the way the game makes it
fn made_for(kind: &'static EnemyKind, army: usize, at: Vec2) -> Enemy {
    let mut enemy = Enemy::new(at, kind, army_scale(army));
    enemy.empower(army_might(army));
    enemy
}

//how long the army takes to destroy an enemy that the player has walked into (its middle is `offset` pixels from the player), and
//how many of the units it kills first, or None for the seconds if it is still there after a minute. the units stand still
fn lasts(units: &[Unit], enemy: Enemy, upgrades: &Upgrades) -> (Option<f32>, usize) {
    let mut units = units.to_vec();
    let mut enemies = vec![enemy];
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    for frame in 0..60 * 120 {
        fight(&mut enemies, &mut units, &mut babies, DT, upgrades, &mut fx);
        if enemies.is_empty() {
            return (Some((frame + 1) as f32 * DT), units.iter().filter(|unit| unit.health <= 0.0).count());
        }
    }
    (None, units.iter().filter(|unit| unit.health <= 0.0).count())
}

#[test]
#[ignore]
fn how_long_enemies_last_against_a_big_army() {
    let player = Vec2::splat(WORLD_SIZE / 2.0);
    for army in [100usize, 300, 1000, 4000, 8000, 15000] {
        rand::srand(11);
        let units = settled_army(army, player);
        let spread = units.iter().map(|unit| Vec2::new(unit.x, unit.y).distance(player)).sum::<f32>() / army as f32;
        println!("{army} units, on average {spread:.0} px from the player, enemies {:.1}x tougher, {:.1}x harder hitting", army_might(army).powf(ENEMY_MIGHT_HEALTH), army_might(army).powf(ENEMY_MIGHT_ATTACK));
        for name in ["Grunt", "Scout", "Brute", "Titan", "Colossus", "Bulwark", "Charger"] {
            let kind = kind_named(name);
            let mut row = format!("    {name:9}");
            for (label, along) in [("edge", 1.0), ("middle", 0.0)] {
                let enemy = made_for(kind, army, player + Vec2::new(kind.size * army_scale(army) * 0.5 * along, 0.0));
                let (seconds, lost) = lasts(&units, enemy, &Upgrades::default());
                row += &match seconds {
                    Some(seconds) => format!("  {label}: {seconds:5.2}s, {lost:5} lost"),
                    None => format!("  {label}: over a minute, {lost:5} lost"),
                };
            }
            println!("{row}");
        }
    }
}

#[test]
fn past_the_army_that_makes_enemies_as_big_as_they_get_they_keep_getting_tougher() {
    assert_eq!((army_might(0), army_might(1000), army_might(15000)), (1.0, 1.0, 15.0));
    let at = MIDDLE;
    for army in [500usize, 1000, 4000, 15000, 60000] {
        let (scale, might) = (army_scale(army), army_might(army));
        for kind in ENEMY_KINDS.iter().filter(|kind| kind.weight > 0.0) {
            let enemy = enemy_for(at, kind, army);
            assert!(enemy.size <= kind.size * MAX_ENEMY_SCALE * 1.2 + 0.01, "{} is {} wide at {army}", kind.name, enemy.size);
            let health = enemy.max_health / (kind.health * scale * scale);
            let (low, high) = (might.powf(ENEMY_MIGHT_HEALTH) * 0.79, might.powf(ENEMY_MIGHT_HEALTH) * 1.31);
            assert!(health >= low && health <= high, "{} has {health}x the health at {army}", kind.name);
            let attack = enemy.attack / (kind.attack * scale);
            let (low, high) = (might.powf(ENEMY_MIGHT_ATTACK) * 0.84, might.powf(ENEMY_MIGHT_ATTACK) * 1.16);
            assert!(attack >= low && attack <= high, "{} hits {attack}x as hard at {army}", kind.name);
            assert!((enemy.max_shield / enemy.max_health - kind.shield).abs() < 1e-3, "the shield goes with the health");
        }
    }
}

#[test]
fn enemies_at_fifteen_thousand_units_are_several_times_tougher_than_at_a_thousand_but_fall_quicker_and_not_in_a_blink() {
    //the same few kinds, averaged over many so their random nudges cancel out
    let mean = |army: usize, name: &str| (0..200).map(|_| enemy_for(MIDDLE, kind_named(name), army).max_health).sum::<f32>() / 200.0;
    for name in ["Grunt", "Brute", "Titan"] {
        let (small, big) = (mean(1000, name), mean(15000, name));
        assert!(big > small * 4.0 && big < small * 8.0, "{name}: {big} against {small}");
    }
    //what a bigger army does to one: it is destroyed sooner, because there are more units to do it, but not a fifth of the time
    let seconds = |army: usize| mean(army, "Titan") / (army as f32 * UNIT_DAMAGE);
    assert!(seconds(15000) < seconds(1000) && seconds(15000) > seconds(1000) * 0.2, "{} against {}", seconds(15000), seconds(1000));
}

#[test]
fn what_a_nest_a_garrison_and_a_splitter_make_is_as_tough_as_the_rest() {
    let site = Site::new(SiteKind::Nest(1), MIDDLE);
    let baseline = |army: usize| nest_core(&NESTS[1], &site, army).max_health;
    assert!(baseline(15000) > baseline(1000) * 4.0);
    for guard in village_guards(&Site::new(SiteKind::Village, MIDDLE), 15000) {
        let kind = kind_named(guard.name);
        assert!(guard.max_health / (kind.health * MAX_ENEMY_SCALE * MAX_ENEMY_SCALE) > 4.0, "{} has {}", guard.name, guard.max_health);
    }
    let mut splitter = enemy_for(MIDDLE, kind_named("Splitter"), 15000);
    splitter.health = 0.0;
    let (mut enemies, mut babies, mut fx) = (vec![splitter], Vec::new(), Effects::default());
    resolve_deaths(&mut enemies, &mut babies, 15000, &Upgrades::default(), &mut fx);
    assert_eq!(enemies.len(), 3);
    let plain = kind_named("Splitling").health * minion_scale(MAX_ENEMY_SCALE).powi(2);
    assert!(enemies.iter().all(|splitling| splitling.max_health > plain * 4.0), "{} against {plain}", enemies[0].max_health);
}

#[test]
fn the_wild_enemies_are_made_for_a_bigger_army_for_every_boss_destroyed() {
    assert_eq!(enemy_pressure(1000, 0), 1000);
    assert_eq!(enemy_pressure(1000, 10), (1000.0 * (1.0 + 10.0 * ENEMY_ROUND_GROWTH)).round() as usize);
    assert!(enemy_pressure(100, 14) > enemy_pressure(100, 7) && enemy_pressure(100, 7) > 100);
    let mut enemies = Vec::new();
    respawn_enemies(&mut enemies, MIDDLE.x, MIDDLE.y, enemy_pressure(1000, 25));
    let tougher = (1.0 + 25.0 * ENEMY_ROUND_GROWTH).powf(ENEMY_MIGHT_HEALTH);
    assert!(!enemies.is_empty());
    for enemy in &enemies {
        let kind = kind_named(enemy.name);
        let ratio = enemy.max_health / (kind.health * MAX_ENEMY_SCALE * MAX_ENEMY_SCALE);
        assert!(ratio > tougher * 0.79, "{} has {ratio}x, wanted at least {}", enemy.name, tougher * 0.79);
    }
}

//how far into a big army's fire an enemy gets: it starts at the edge of what the front of the army sees, straight ahead, and runs at the player, with
//the vanguard firing at it as in the game and the units fighting it when it gets there. what is printed is how long it lasted, and whether it reached
//the army (and how many units it killed) or how far from the player it fell. made to see whether the late game is still a fight with the army firing
#[test]
#[ignore]
fn how_far_enemies_get_into_a_big_armys_fire() {
    let player = Vec2::splat(WORLD_SIZE / 2.0);
    for army in [1000usize, 4000, 15000, 50000] {
        rand::srand(11);
        let units = settled_army(army, player);
        let reach = units.iter().map(|unit| Vec2::new(unit.x, unit.y).distance(player)).fold(0.0f32, f32::max);
        println!("{army} units, reaching {reach:.0} px, firing {:.0} px, enemies {:.1}x tougher", gunnery_range(army), army_might(army).powf(ENEMY_MIGHT_HEALTH));
        for name in ["Grunt", "Brute", "Titan", "Colossus"] {
            let kind = kind_named(name);
            let start = player + Vec2::new(reach + gunnery_range(army) * 0.95, 0.0);
            let mut enemy = made_for(kind, army, start);
            enemy.chasing = true;
            enemy.aggro = 1.0e6;
            let (mut enemies, mut units, mut bolts, mut babies) = (vec![enemy], units.clone(), Vec::new(), Vec::new());
            let mut fx = Effects::default();
            let front = [Front { position: player, facing: -Vec2::X, formation: Formation::Ring, upgrades: &Upgrades::default() }];
            let (mut seconds, mut nearest) = (0.0f32, f32::MAX);
            let mut touched = false;
            while seconds < 60.0 && !enemies.is_empty() {
                seconds += DT;
                patrol_free(&mut enemies, player, DT, speed_multiplier(army), &mut fx);
                if let Some(enemy) = enemies.first() {
                    nearest = nearest.min(enemy.position.distance(player));
                }
                //(the army faces the enemy coming at it: the player has turned to it)
                let facing = enemies.first().map_or(Vec2::X, |enemy| (enemy.position - player).normalize_or_zero());
                let front = [Front { facing, ..front[0] }];
                fire_vanguard(&front, &units, &enemies, seconds, DT, 1.0, true, &mut bolts, Some(&mut fx));
                fly_bolts(&mut bolts, &mut enemies, DT, Some(&mut fx));
                let before = units.iter().filter(|unit| unit.health > 0.0).count();
                fight(&mut enemies, &mut units, &mut babies, DT, &Upgrades::default(), &mut fx);
                touched |= units.iter().filter(|unit| unit.health > 0.0).count() < before || nearest <= reach;
                fx = Effects::default();
            }
            let lost = units.iter().filter(|unit| unit.health <= 0.0).count();
            let outcome = if !enemies.is_empty() { "still there after a minute".to_owned() } else if touched { format!("reached the army, killed {lost}") } else { format!("fell {nearest:.0} px from the player") };
            println!("    {name:9} lasted {seconds:5.2}s, {outcome}");
        }
    }
}
