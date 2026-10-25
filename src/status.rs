//! what is being done to an enemy that comes and goes (`Statuses`): chill that builds up to a freeze, the freeze that holds it and makes it brittle, burning
//! that catches on what it touches, a shock that makes the next hit worse, poison that stacks, and how much of its speed the ground and the auras leave it.
//! what puts each on is here (`chill`, `freeze`, `ignite`, `shock`, `poison`), and `tick_statuses` runs them all, once a frame. a boss resists: it takes
//! BOSS_CHILL of the chill, burning and poison put on it, and it is never held frozen, only slowed, to FROZEN_BOSS_SLOW at the most. a frozen enemy that dies,
//! or takes a big hit, shatters (see `resolve_deaths_among` and `strike`)

use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Statuses {
    pub(crate) slow: f32,      //the share of its speed the ground (tar) and the auras leave it this frame: the ground puts it back each frame
    pub(crate) chill: f32,     //0 to CHILL_FULL, from frost: it ebbs, it slows the enemy the more there is, and at full the enemy freezes
    pub(crate) frozen: f32,    //seconds left held solid: it can't move, attack or shoot, and it is brittle
    pub(crate) burning: f32,   //seconds left alight...
    pub(crate) burn: f32,      //...for this much damage a second; it catches on the enemies touching it
    pub(crate) spread_in: f32, //seconds until it next does
    pub(crate) shocked: f32,   //seconds left in which the next hit from a power does SHOCK_BONUS times as much
    pub(crate) poison: f32,    //stacks, each taking POISON_SHARE of its most health a second, ebbing
}

impl Default for Statuses {
    fn default() -> Statuses {
        Statuses { slow: 1.0, chill: 0.0, frozen: 0.0, burning: 0.0, burn: 0.0, spread_in: 0.0, shocked: 0.0, poison: 0.0 }
    }
}

//how much an enemy takes of what is put on it: all of it, or BOSS_CHILL for a boss
fn taken(enemy: &Enemy) -> f32 {
    if enemy.boss.is_some() { BOSS_CHILL } else { 1.0 }
}

//chill put on an enemy (nothing more on one that is frozen already): at full, one that can be frozen freezes for `hold` seconds
pub(crate) fn chill(enemy: &mut Enemy, amount: f32, hold: f32) {
    if enemy.status.frozen > 0.0 || amount <= 0.0 {
        return;
    }
    enemy.status.chill = (enemy.status.chill + amount * taken(enemy)).min(CHILL_FULL);
    if enemy.status.chill >= CHILL_FULL {
        freeze(enemy, hold);
    }
}

//an enemy held frozen for `seconds` at full chill, if it can be: not a boss, a nest or one lying in wait
pub(crate) fn freeze(enemy: &mut Enemy, seconds: f32) {
    if !enemy.can_freeze() || seconds <= 0.0 {
        return;
    }
    enemy.status.frozen = enemy.status.frozen.max(seconds);
    enemy.status.chill = CHILL_FULL;
}

//an enemy set alight for `seconds`, burning for `damage_a_second` (the more of either, of this and what it had)
pub(crate) fn ignite(enemy: &mut Enemy, seconds: f32, damage_a_second: f32) {
    if seconds <= 0.0 || damage_a_second <= 0.0 {
        return;
    }
    let status = &mut enemy.status;
    status.burning = status.burning.max(seconds);
    status.burn = status.burn.max(damage_a_second);
}

//an enemy shocked for `seconds`: the next hit from a power does SHOCK_BONUS times as much (see `strike`)
pub(crate) fn shock(enemy: &mut Enemy, seconds: f32) {
    enemy.status.shocked = enemy.status.shocked.max(seconds);
}

//stacks of poison on an enemy, up to POISON_MOST (a venomancer's cloud)
pub(crate) fn poison(enemy: &mut Enemy, stacks: f32) {
    enemy.status.poison = (enemy.status.poison + stacks.max(0.0)).min(POISON_MOST);
}

//the share of its speed what is done to it leaves an enemy: the ground's and the auras' slow, and chill on top, down to CHILL_SLOW at full (FROZEN_BOSS_SLOW for
//a boss). one that is frozen does not move at all, which the moving sees to
pub(crate) fn speed_kept(enemy: &Enemy) -> f32 {
    let most = if enemy.boss.is_some() { FROZEN_BOSS_SLOW } else { CHILL_SLOW };
    enemy.status.slow * (1.0 - (1.0 - most) * enemy.status.chill / CHILL_FULL)
}

//damage that is not a hit (burning, poison): it shows as a number, and the credit stays with whoever hurt the enemy last
fn wear(enemy: &mut Enemy, damage: f32) {
    if damage > 0.0 && enemy.health > 0.0 {
        let dealt = enemy.take_damage(damage);
        enemy.hurt += dealt;
    }
}

//a frame of what is done to the enemies: a freeze runs down (and a thaw takes the chill with it), chill ebbs, a shock wears off, burning hurts and, every
//BURN_SPREAD_EVERY, catches on the enemies touching it, at BURN_SPREAD of its strength and for no longer than it has left, so a fire dies out, and poison hurts and ebbs
pub(crate) fn tick_statuses(enemies: &mut [Enemy], dt: f32) {
    if dt <= 0.0 {
        return;
    }
    let mut catching: Vec<(u32, Vec2, f32, f32, f32)> = Vec::new(); //(which, where, how far it reaches, the burn it passes on, for how long)
    for enemy in enemies.iter_mut() {
        let share = taken(enemy);
        let most = enemy.max_health;
        let status = &mut enemy.status;
        if status.frozen > 0.0 {
            status.frozen = (status.frozen - dt).max(0.0);
            if status.frozen == 0.0 {
                status.chill = 0.0;
            }
        } else {
            status.chill = (status.chill - CHILL_EBB * dt).max(0.0);
        }
        status.shocked = (status.shocked - dt).max(0.0);
        let mut harm = 0.0;
        if status.burning > 0.0 {
            harm += status.burn * share * dt.min(status.burning);
            status.burning = (status.burning - dt).max(0.0);
            status.spread_in -= dt;
            if status.burning == 0.0 {
                status.burn = 0.0;
            } else if status.spread_in <= 0.0 {
                status.spread_in = BURN_SPREAD_EVERY;
                catching.push((enemy.id, enemy.position, enemy.size / 2.0, status.burn * BURN_SPREAD, status.burning * BURN_SPREAD));
            }
        }
        if status.poison > 0.0 {
            harm += status.poison * POISON_SHARE * most * share * dt;
            status.poison = (status.poison - POISON_EBB * dt).max(0.0);
        }
        wear(enemy, harm);
    }
    for (from, at, reach, burn, seconds) in catching {
        for enemy in enemies.iter_mut().filter(|enemy| enemy.id != from && enemy.health > 0.0 && !enemy.lurking()) {
            if enemy.position.distance(at) <= reach + enemy.size / 2.0 + BURN_TOUCH {
                ignite(enemy, seconds, burn);
            }
        }
    }
}
