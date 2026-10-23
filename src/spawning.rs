//! putting enemies in the world: single ones, packs, warbands and respawns

use crate::prelude::*;

//a random enemy, from the player's position out to ENEMY_SPAWN_MAX_TILES grid squares away but not closer than
//ENEMY_SPAWN_MIN_TILES, as big and tough as the size of the army calls for. (the game picks the kind itself, to know
//whether it comes alone, so this is for the tests)
#[cfg(test)]
pub(crate) fn spawn_enemy(player_x: f32, player_y: f32, army: usize) -> Enemy {
    spawn_of_kind(player_x, player_y, random_kind(army), army)
}

//an enemy of a kind made for an army this big: the size and the numbers the army calls for, and tougher still past the point where size stops growing
pub(crate) fn enemy_for(at: Vec2, kind: &EnemyKind, army: usize) -> Enemy {
    let mut enemy = Enemy::new(at, kind, army_scale(army));
    enemy.empower(army_might(army));
    enemy
}

//a random enemy of a kind that has been picked already, placed and made like spawn_enemy does it
pub(crate) fn spawn_of_kind(player_x: f32, player_y: f32, kind: &'static EnemyKind, army: usize) -> Enemy {
    let mut enemy = enemy_for(spawn_point(player_x, player_y, kind, army_scale(army)), kind, army);
    if rand::gen_range(0.0, 1.0) < elite_chance(army) {
        enemy.make_elite(MODIFIERS[rand::gen_range(0, MODIFIERS.len())]);
    }
    enemy
}

//where a kind of enemy starts: ENEMY_SPAWN_MIN_TILES to ENEMY_SPAWN_MAX_TILES from the player. an enemy can be huge, so
//that much further out keeps it from landing on top of the army
pub(crate) fn spawn_point(player_x: f32, player_y: f32, kind: &EnemyKind, scale: f32) -> Vec2 {
    let reach = kind.size * scale * 0.6;
    random_position_around(player_x, player_y, ENEMY_SPAWN_MIN_TILES * TILE_SIZE + reach, ENEMY_SPAWN_MAX_TILES * TILE_SIZE + reach)
}

//the next thing to turn up: a pack of the kinds that come in packs, once in a while a warband, otherwise one enemy
pub(crate) fn spawn_group(player_x: f32, player_y: f32, army: usize) -> Vec<Enemy> {
    let scale = army_scale(army);
    let kind = random_kind(army);
    if kind.pack > 0 {
        let most = if kind.behavior == Behavior::Ambusher { MAX_AMBUSH_PACK } else { 30 };
        let count = (kind.pack + army / 100).min(most);
        let middle = spawn_point(player_x, player_y, kind, scale);
        let mut pack: Vec<Enemy> = (0..count)
            .map(|_| enemy_for(middle + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(0.0, PACK_SPREAD), kind, army))
            .collect();
        //a pack travels together
        let route = pack[0].route.clone();
        for member in &mut pack {
            member.route = route.clone();
        }
        return pack;
    }
    if army >= WARBAND_FROM_UNITS && rand::gen_range(0.0, 1.0) < WARBAND_CHANCE {
        return spawn_warband(player_x, player_y, army);
    }
    vec![spawn_of_kind(player_x, player_y, kind, army)]
}

//a captain with escorts round it, and a healer if the army is big enough for one, that all patrol the same route
pub(crate) fn spawn_warband(player_x: f32, player_y: f32, army: usize) -> Vec<Enemy> {
    let scale = army_scale(army);
    let on_their_own = |kind: &&EnemyKind| kind.pack == 0 && kind.behavior != Behavior::Healer;
    let captain_kind = (0..20).map(|_| random_kind(army)).find(|kind| on_their_own(&kind) && kind.health >= 200.0).unwrap_or(&ENEMY_KINDS[0]);
    let middle = spawn_point(player_x, player_y, captain_kind, scale);
    let mut band = vec![enemy_for(middle, captain_kind, army)];
    for _ in 0..rand::gen_range(3, 7) {
        let kind = (0..20).map(|_| random_kind(army)).find(|kind| on_their_own(&kind)).unwrap_or(&ENEMY_KINDS[0]);
        band.push(enemy_for(middle + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(60.0, PACK_SPREAD), kind, army));
    }
    let healer = kind_named("Healer");
    if army >= healer.from_army {
        band.push(enemy_for(middle + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(60.0, PACK_SPREAD), healer, army));
    }
    let route = band[0].route.clone();
    for member in &mut band {
        member.route = route.clone();
    }
    band
}

//the same for one player, at a place, with enemies made for an army of `army`
pub(crate) fn respawn_enemies(enemies: &mut Vec<Enemy>, player_x: f32, player_y: f32, army: usize) {
    respawn_enemies_among(enemies, &[(Vec2::new(player_x, player_y), army)]);
}

//enemies are never used up: whenever there are fewer than ENEMY_COUNT round a player, new ones turn up in a random place, alone or in a
//group, made for the army the second number of a player says (what their fight is for). the world is far bigger than they can be spread
//over, so the ones that have been left more than ENEMY_LEASH_TILES behind every player are let go: the population keeps up with the
//players instead of staying where the run began.
//a group starts at the emptiest of a few places in the ring, otherwise the ones left behind would pile up at the player's back while the
//ground ahead stayed bare. players who are near each other have the same enemies round them, so there are no more of them than for one
pub(crate) fn respawn_enemies_among(enemies: &mut Vec<Enemy>, players: &[(Vec2, usize)]) {
    let leash = ENEMY_LEASH_TILES * TILE_SIZE;
    enemies.retain(|enemy| !is_wild(enemy) || players.iter().any(|(player, _)| enemy.position.distance_squared(*player) <= leash * leash));
    let mut crowd: Vec<Vec2> = enemies.iter().filter(|enemy| is_wild(enemy)).map(|enemy| enemy.position).collect();
    //ambushers stay put until the player comes by, so they would pile up if every group that turned up could be one
    let mut lurkers = enemies.iter().filter(|enemy| enemy.lurking() && is_wild(enemy)).count();
    let most_lurkers = ENEMY_COUNT / MAX_LURKERS_SHARE * players.len();
    for &(player, army) in players {
        let mut round_them = crowd.iter().filter(|at| at.distance_squared(player) <= leash * leash).count();
        while round_them < ENEMY_COUNT {
            let mut group = spawn_group(player.x, player.y, army);
            for _ in 0..8 {
                if group[0].lurking() && lurkers + group.len() > most_lurkers {
                    group = spawn_group(player.x, player.y, army);
                } else {
                    break;
                }
            }
            lurkers += group.iter().filter(|member| member.lurking()).count();
            let reach = group[0].size * 0.6;
            let room = |at: Vec2| crowd.iter().map(|other| other.distance_squared(at)).fold(f32::MAX, f32::min);
            let mut best = group[0].position;
            let mut most_room = room(best);
            for _ in 1..SPAWN_CANDIDATES {
                let at = random_position_around(player.x, player.y, ENEMY_SPAWN_MIN_TILES * TILE_SIZE + reach, ENEMY_SPAWN_MAX_TILES * TILE_SIZE + reach);
                if room(at) > most_room {
                    best = at;
                    most_room = room(at);
                }
            }
            let step = best - group[0].position;
            for member in &mut group {
                member.shift(step);
            }
            round_them += group.len();
            crowd.extend(group.iter().map(|member| member.position));
            enemies.extend(group);
        }
    }
}
