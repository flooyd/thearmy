//! the vanguard: the front of each army fires bolts ahead of it, the way it is heading, at the enemies there, and holds its fire when there are none.
//! only the units in the front GUNNERY_FRONT_SHARE of the army's depth along its heading fire, never champions (who have powers of their own) or units
//! in towers (who shoot from there), and never at an enemy behind them. the bigger the army the further they see (`gunnery_range`). which of them fire in a frame, and at what, comes from where everything is
//! and the clock alone, so a guest's machine works it out for itself from what it is shown and flies the same bolts, which hurt nothing there (see
//! `fly_bolts`): no message carries them. the cost of a frame is the units once over and the bolts fired, never the units times the enemies

use crate::prelude::*;

//what one player's army fires with: where the player is, the way the army faces, its shape, and the relics that count for it
pub(crate) struct Front<'a> {
    pub(crate) position: Vec2,
    pub(crate) facing: Vec2,
    pub(crate) formation: Formation,
    pub(crate) upgrades: &'a Upgrades,
}

//the fronts of the players' armies, in the order of the players, with the relics each has
pub(crate) fn fronts_of<'a, R: Relics + ?Sized>(players: &Players, relics: &'a R) -> Vec<Front<'a>> {
    players.list.iter().enumerate().map(|(i, player)| Front { position: player.position, facing: player.facing, formation: player.formation, upgrades: relics.of(i as u8) }).collect()
}

//how often each gunner of an army fires, a second
pub(crate) fn gunner_rate(front: &Front) -> f32 {
    GUNNER_RATE * front.upgrades.gunnery * if front.formation == Formation::Wedge { WEDGE_GUNNERY } else { 1.0 }
}

//how far the front of an army of `army` units sees and fires: GUNNERY_RANGE for a small one, growing (as the square root of its size, as the army's
//reach does) to GUNNERY_WIDE_RANGE, about the width of the screen zoomed out to see it, at GUNNERY_WIDE_ARMY units, and no further
pub(crate) fn gunnery_range(army: usize) -> f32 {
    let grown = (army as f32 / GUNNERY_WIDE_ARMY as f32).clamp(0.0, 1.0).sqrt();
    GUNNERY_RANGE + (GUNNERY_WIDE_RANGE - GUNNERY_RANGE) * grown
}

//the enemy a gunner at a place fires at: the nearest within `range` in the narrow cone round the heading, or failing that the nearest in the wide one.
//nothing outside the wide cone, which never takes in what is behind
fn target_for(at: Vec2, facing: Vec2, range: f32, targets: &[(Vec2, f32)], aim_cos: f32, search_cos: f32) -> Option<Vec2> {
    let mut narrow: Option<(f32, Vec2)> = None;
    let mut wide: Option<(f32, Vec2)> = None;
    for &(position, half) in targets {
        let way = position - at;
        let distance = way.length();
        if distance <= 0.0 || distance - half > range {
            continue;
        }
        let cos = way.dot(facing) / distance;
        let best = if cos >= aim_cos {
            &mut narrow
        } else if cos >= search_cos {
            &mut wide
        } else {
            continue;
        };
        if best.is_none_or(|(nearest, _)| distance < nearest) {
            *best = Some((distance, position));
        }
    }
    narrow.or(wide).map(|(_, position)| position)
}

//a frame of the vanguard's fire. `clock` is the world's seconds, `density` the share of the bolts flown (see `Density`: fewer are heavier, so what they
//do is the same), and `hurts` whether they do damage (the host's world) or not (a guest's machine, where `fx` is None too). each gunner fires
//`gunner_rate` times a second at a moment of its own, made from its id; past a frame's share of GUNNERY_BOLTS_A_SECOND, every so many of those due fire, each
//the heavier for the ones that didn't
pub(crate) fn fire_vanguard(fronts: &[Front], units: &[Unit], enemies: &[Enemy], clock: f32, dt: f32, density: f32, hurts: bool, bolts: &mut Vec<Bolt>, mut fx: Option<&mut Effects>) {
    if dt <= 0.0 || fronts.is_empty() || units.is_empty() || enemies.is_empty() {
        return;
    }
    let count = fronts.len();
    let facing: Vec<Vec2> = fronts.iter().map(|front| front.facing.try_normalize().unwrap_or(Vec2::NEG_Y)).collect();
    let fights = |unit: &Unit| unit.champion == 0 && unit.garrison.is_none() && unit.health > 0.0 && owner_index(unit.owner) < count;
    //how far forward of its player each army reaches along its heading, how far back, and how far out
    let (mut front_most, mut back_most, mut reach) = (vec![f32::MIN; count], vec![f32::MAX; count], vec![0.0f32; count]);
    let mut size = vec![0usize; count];
    for unit in units.iter().filter(|unit| fights(unit)) {
        let owner = owner_index(unit.owner);
        let offset = Vec2::new(unit.x, unit.y) - fronts[owner].position;
        let ahead = offset.dot(facing[owner]);
        front_most[owner] = front_most[owner].max(ahead);
        back_most[owner] = back_most[owner].min(ahead);
        reach[owner] = reach[owner].max(offset.length());
        size[owner] += 1;
    }
    let (aim_cos, search_cos) = (GUNNERY_AIM_ARC.to_radians().cos(), GUNNERY_SEARCH_ARC.to_radians().cos());
    let wrapped = clock.rem_euclid(1000.0);
    let density = density.clamp(0.05, 1.0);
    for (owner, front) in fronts.iter().enumerate() {
        if front_most[owner] < back_most[owner] {
            continue;
        }
        let range = gunnery_range(size[owner]) * front.upgrades.sight;
        let line = front_most[owner] - GUNNERY_FRONT_SHARE * (front_most[owner] - back_most[owner]);
        //the enemies there are to fire at, in reach of some part of the army
        let targets: Vec<(Vec2, f32)> = enemies
            .iter()
            .filter(|enemy| enemy.health > 0.0 && !enemy.lurking() && enemy.position.distance(front.position) <= reach[owner] + range + enemy.size / 2.0)
            .map(|enemy| (enemy.position, enemy.size / 2.0))
            .collect();
        if targets.is_empty() {
            continue;
        }
        let rate = gunner_rate(front);
        let due: Vec<&Unit> = units
            .iter()
            .filter(|unit| fights(unit) && owner_index(unit.owner) == owner)
            .filter(|unit| (Vec2::new(unit.x, unit.y) - front.position).dot(facing[owner]) >= line)
            .filter(|unit| hash_noise(unit.id, 1) < density)
            .filter(|unit| {
                let phase = hash_noise(unit.id, 2) + wrapped * rate;
                phase.floor() != (phase - dt * rate).floor()
            })
            .collect();
        if due.is_empty() {
            continue;
        }
        let taken = due.len().min(((GUNNERY_BOLTS_A_SECOND * dt).round() as usize).max(1));
        let stride = due.len() as f32 / taken as f32;
        let heavier = stride / density;
        let mut fired = 0;
        for k in 0..taken {
            let unit = due[(k as f32 * stride) as usize];
            let at = Vec2::new(unit.x, unit.y);
            let Some(target) = target_for(at, facing[owner], range, &targets, aim_cos, search_cos) else {
                continue;
            };
            let from = at + facing[owner] * RADIUS;
            let volley = Volley {
                from,
                aim: target - from,
                kind: BoltKind::Tracer,
                shape: VolleyShape::Burst,
                count: 1,
                scatter: TRACER_SCATTER,
                speed: TRACER_SPEED * (range / GUNNERY_RANGE).sqrt(),
                reach: range * 1.15,
                pierce: front.upgrades.pierce,
                bounce: 0,
                homing: 0.0,
                splash: 0.0,
                embers: 0,
                champion: 0,
                seed: unit.id ^ (wrapped * 120.0) as u32,
            };
            //each does what the gunner does by touch in a second, times GUNNER_POWER, over the shots it fires in a second at the usual rate: so firing
            //more often (a wedge, a Volley Drill) is more damage, and a lower density or a full frame only fewer, heavier bolts
            let hurt = if hurts { Hurt { each: UNIT_DAMAGE * front.upgrades.hit() * unit.might * GUNNER_POWER / GUNNER_RATE * heavier, ember: 0.0, owner: owner as u8 } } else { Hurt::default() };
            launch_volley(bolts, &volley, hurt);
            fired += 1;
        }
        if fired > 0 {
            if let Some(fx) = fx.as_deref_mut() {
                fx.cue_at(Cue::Volley, front.position);
            }
        }
    }
}
