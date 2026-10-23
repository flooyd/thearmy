//! champions: special units won as loot from bosses, elite enemies and barracks. each is a unit in the army (it follows the
//! player, is pushed about and hurt like the rest) with a body of its own, much more might and one power: some fight as many
//! units, some strengthen or mend the units round them, some strike the enemies from where they stand, and some change what
//! the whole army gets from the run. there are fourteen kinds, in three grades of how rare they are

use crate::prelude::*;

//what a champion does besides fighting by touch. the amounts are in the table of kinds
#[derive(Clone, Copy, Debug)]
pub(crate) enum Power {
    Brute,                                //nothing else: it just fights as many units and takes little damage
    Rage { extra: f32 },                  //its might grows by this many times itself as it is hurt
    Banner { radius: f32, might: f32 },   //the units near it do this many times the damage
    Bulwark { radius: f32, ward: f32 },   //the units near it take this share of the damage
    Mend { radius: f32, per_second: f32 }, //the units near it heal this much health a second
    //hurls a fireball every so often at the thickest cluster of enemies in range, that hurts all of them. a hit does `flat`
    //units' worth of a second's damage and `share` of each enemy's health
    Fireball { every: f32, range: f32, radius: f32, flat: f32, share: f32 },
    Snipe { every: f32, range: f32, flat: f32, share: f32 }, //a shot at the toughest enemy in range
    Lightning { every: f32, range: f32, jumps: usize, flat: f32, share: f32 }, //a bolt that jumps from enemy to enemy
    Frost { radius: f32, slow: f32, every: f32, freeze: f32 }, //enemies near it keep this share of their speed, and every so often are frozen for a moment
    Quake { every: f32, radius: f32, flat: f32, share: f32 }, //slams the ground, hurting every enemy round it
    Muster { every: f32, count: usize },  //calls recruits (babies) round itself
    Rebirth { recharge: f32, radius: f32, flat: f32, share: f32 }, //when it dies it rises again in a blaze, and can't again for a while
    Scout { speed: f32, pickup: f32 },    //the army moves this many times faster and picks babies up from this many times further
    Quartermaster { points: f32, bounty: f32 }, //wall points a second, and enemies drop this many times the babies
}

impl Power {
    //how far its aura reaches, for the ones that have one
    pub(crate) fn aura(&self) -> Option<f32> {
        match *self {
            Power::Banner { radius, .. } | Power::Bulwark { radius, .. } | Power::Mend { radius, .. } | Power::Frost { radius, .. } => Some(radius),
            _ => None,
        }
    }

    //the same power for a champion of a level: more of what it does (damage, healing, recruits), done more often and reaching
    //further. level 1 is the power as the table has it
    pub(crate) fn at_level(&self, level: u8) -> Power {
        let steps = level.max(1).saturating_sub(1) as f32;
        let more = 1.0 + CHAMPION_LEVEL_POWER * steps;
        let quicker = 1.0 + CHAMPION_LEVEL_PACE * steps;
        let further = 1.0 + CHAMPION_LEVEL_REACH * steps;
        match *self {
            Power::Brute => Power::Brute,
            Power::Rage { extra } => Power::Rage { extra: extra * more },
            Power::Banner { radius, might } => Power::Banner { radius: radius * further, might: 1.0 + (might - 1.0) * more },
            Power::Bulwark { radius, ward } => Power::Bulwark { radius: radius * further, ward: ward.powf(more) },
            Power::Mend { radius, per_second } => Power::Mend { radius: radius * further, per_second: per_second * more },
            Power::Fireball { every, range, radius, flat, share } => Power::Fireball { every: every / quicker, range: range * further, radius: radius * further, flat: flat * more, share: (share * more).min(0.9) },
            Power::Snipe { every, range, flat, share } => Power::Snipe { every: every / quicker, range: range * further, flat: flat * more, share: (share * more).min(0.9) },
            Power::Lightning { every, range, jumps, flat, share } => Power::Lightning { every: every / quicker, range: range * further, jumps: jumps + steps as usize, flat: flat * more, share: (share * more).min(0.9) },
            Power::Frost { radius, slow, every, freeze } => Power::Frost { radius: radius * further, slow: slow / (1.0 + 0.1 * steps), every: every / quicker, freeze: freeze * (1.0 + 0.1 * steps) },
            Power::Quake { every, radius, flat, share } => Power::Quake { every: every / quicker, radius: radius * further, flat: flat * more, share: (share * more).min(0.9) },
            Power::Muster { every, count } => Power::Muster { every: every / quicker, count: (count as f32 * more).round() as usize },
            Power::Rebirth { recharge, radius, flat, share } => Power::Rebirth { recharge: recharge / (1.0 + 0.15 * steps), radius: radius * further, flat: flat * more, share: (share * more).min(0.9) },
            Power::Scout { speed, pickup } => Power::Scout { speed: 1.0 + (speed - 1.0) * more, pickup: 1.0 + (pickup - 1.0) * more },
            Power::Quartermaster { points, bounty } => Power::Quartermaster { points: points * more, bounty: 1.0 + (bounty - 1.0) * more },
        }
    }

    //how long it waits after it joins before it first goes off, half its time: the one that has just joined doesn't wait
    //longest, and a second one isn't in step with the first
    fn first_wait(&self) -> f32 {
        match *self {
            Power::Fireball { every, .. } | Power::Snipe { every, .. } | Power::Lightning { every, .. } | Power::Quake { every, .. } | Power::Muster { every, .. } => every * 0.5,
            Power::Frost { every, .. } => every * 0.6,
            _ => 0.0,
        }
    }
}

//a kind of champion
pub(crate) struct Champion {
    pub(crate) name: &'static str,
    pub(crate) tag: &'static str,   //two letters for the middle of its body, and the roster
    pub(crate) blurb: &'static str, //what it does, in a line
    pub(crate) color: Color,
    pub(crate) grade: Rarity,
    pub(crate) sides: u8,  //its body is a polygon with this many sides, so each kind can be told apart
    pub(crate) turn: f32,  //degrees the polygon is turned
    pub(crate) size: f32,  //its radius as a multiple of a unit's
    pub(crate) might: f32, //how many units' damage it does by touch, before the army's size adds to it
    pub(crate) ward: f32,  //the share of the damage that reaches it, 1 for none shrugged off
    pub(crate) power: Power,
}

pub(crate) static CHAMPIONS: [Champion; 14] = [
    Champion { name: "Juggernaut", tag: "Ju", blurb: "fights as 14 units and takes a fifth of the damage", color: Color::new(0.82, 0.6, 0.35, 1.0), grade: Rarity::Common, sides: 4, turn: 45.0, size: 2.1, might: 14.0, ward: 0.22, power: Power::Brute },
    Champion { name: "Berserker", tag: "Bk", blurb: "fights as 10 units, and as up to 30 the more it is hurt", color: Color::new(0.9, 0.14, 0.2, 1.0), grade: Rarity::Common, sides: 3, turn: -90.0, size: 1.6, might: 10.0, ward: 0.5, power: Power::Rage { extra: 2.0 } },
    Champion { name: "Bulwark", tag: "Bw", blurb: "the units near it take a quarter less damage", color: Color::new(0.45, 0.6, 0.92, 1.0), grade: Rarity::Common, sides: 6, turn: 0.0, size: 1.8, might: 3.0, ward: 0.3, power: Power::Bulwark { radius: 420.0, ward: 0.75 } },
    Champion { name: "Medic", tag: "Md", blurb: "mends the units near it, 14 health a second each", color: Color::new(0.4, 0.95, 0.6, 1.0), grade: Rarity::Common, sides: 8, turn: 22.5, size: 1.4, might: 1.0, ward: 0.6, power: Power::Mend { radius: 380.0, per_second: 14.0 } },
    Champion { name: "Drummer", tag: "Dr", blurb: "calls 4 recruits every 9 seconds", color: Color::new(0.95, 0.72, 0.28, 1.0), grade: Rarity::Common, sides: 5, turn: -90.0, size: 1.4, might: 2.0, ward: 0.6, power: Power::Muster { every: 9.0, count: 4 } },
    Champion { name: "Pyromancer", tag: "Py", blurb: "hurls a fireball every 4 seconds at the thickest crowd of enemies", color: Color::new(1.0, 0.5, 0.08, 1.0), grade: Rarity::Uncommon, sides: 3, turn: 90.0, size: 1.4, might: 2.0, ward: 0.7, power: Power::Fireball { every: 4.2, range: 850.0, radius: 190.0, flat: 8.0, share: 0.2 } },
    Champion { name: "Sharpshooter", tag: "Sh", blurb: "picks off the toughest enemy in sight, twice a second", color: Color::new(0.75, 0.95, 0.3, 1.0), grade: Rarity::Uncommon, sides: 4, turn: 0.0, size: 1.3, might: 1.0, ward: 0.7, power: Power::Snipe { every: 0.5, range: 1000.0, flat: 3.0, share: 0.025 } },
    Champion { name: "Pathfinder", tag: "Pf", blurb: "the army moves 12% faster and picks babies up from 35% further", color: Color::new(0.2, 0.85, 0.8, 1.0), grade: Rarity::Uncommon, sides: 5, turn: 0.0, size: 1.3, might: 1.0, ward: 0.7, power: Power::Scout { speed: 1.12, pickup: 1.35 } },
    Champion { name: "Quartermaster", tag: "Qm", blurb: "earns 1.5 wall points a second, and enemies drop 25% more babies", color: Color::new(0.8, 0.72, 0.5, 1.0), grade: Rarity::Uncommon, sides: 6, turn: 30.0, size: 1.4, might: 1.0, ward: 0.7, power: Power::Quartermaster { points: 1.5, bounty: 1.25 } },
    Champion { name: "Warlord", tag: "Wl", blurb: "the units round its banner hit 35% harder", color: Color::new(0.85, 0.35, 0.95, 1.0), grade: Rarity::Uncommon, sides: 5, turn: 90.0, size: 1.7, might: 6.0, ward: 0.4, power: Power::Banner { radius: 520.0, might: 1.35 } },
    Champion { name: "Frostweaver", tag: "Fw", blurb: "slows enemies near it by 45%, and every 10 seconds freezes them for 1.5", color: Color::new(0.6, 0.86, 1.0, 1.0), grade: Rarity::Rare, sides: 6, turn: 0.0, size: 1.4, might: 1.0, ward: 0.7, power: Power::Frost { radius: 540.0, slow: 0.55, every: 10.0, freeze: 1.5 } },
    Champion { name: "Stormcaller", tag: "St", blurb: "a bolt of lightning every 3 seconds that jumps through 6 enemies", color: Color::new(0.62, 0.5, 1.0, 1.0), grade: Rarity::Rare, sides: 4, turn: 45.0, size: 1.5, might: 2.0, ward: 0.6, power: Power::Lightning { every: 3.0, range: 750.0, jumps: 6, flat: 5.0, share: 0.12 } },
    Champion { name: "Earthshaker", tag: "Es", blurb: "slams the ground every 6 seconds, crushing everything round it", color: Color::new(0.72, 0.34, 0.2, 1.0), grade: Rarity::Rare, sides: 8, turn: 0.0, size: 2.0, might: 8.0, ward: 0.35, power: Power::Quake { every: 6.5, radius: 300.0, flat: 12.0, share: 0.3 } },
    Champion { name: "Phoenix", tag: "Ph", blurb: "rises from its ashes when it dies, burning every enemy near it. once every 90 seconds", color: Color::new(1.0, 0.82, 0.25, 1.0), grade: Rarity::Rare, sides: 3, turn: -90.0, size: 1.6, might: 5.0, ward: 0.5, power: Power::Rebirth { recharge: 90.0, radius: 340.0, flat: 30.0, share: 0.5 } },
];

//the index of the kind with this name
pub(crate) fn champion_named(name: &str) -> Option<usize> {
    CHAMPIONS.iter().position(|champion| champion.name == name)
}

impl Unit {
    //which champion this unit is, if it is one
    pub(crate) fn champion_kind(&self) -> Option<&'static Champion> {
        (self.champion as usize).checked_sub(1).and_then(|kind| CHAMPIONS.get(kind))
    }

    //how big it is: a champion is bigger than the rest, and a little bigger for every level
    pub(crate) fn radius(&self) -> f32 {
        match self.champion_kind() {
            Some(kind) => RADIUS * kind.size * (1.0 + 0.05 * self.level.saturating_sub(1) as f32),
            None => RADIUS,
        }
    }

    //the most health it can have: an ordinary unit's, or a champion's, which a level adds to
    pub(crate) fn max_health(&self) -> f32 {
        if self.champion == 0 { UNIT_HEALTH } else { champion_health(self.level) }
    }
}

//the health of a champion of a level
pub(crate) fn champion_health(level: u8) -> f32 {
    CHAMPION_HEALTH * (1.0 + CHAMPION_LEVEL_HEALTH * level.max(1).saturating_sub(1) as f32)
}

//the experience a champion of this level needs to reach the next
pub(crate) fn champion_xp_needed(level: u8) -> f32 {
    CHAMPION_XP_FIRST * CHAMPION_XP_GROWTH.powi(level.max(1) as i32 - 1)
}

//what an enemy destroyed teaches the champions: a boss most, an elite more than an ordinary one
pub(crate) fn kill_xp(enemy: &Enemy) -> f32 {
    if enemy.boss.is_some() {
        CHAMPION_XP_BOSS
    } else if enemy.modifier != Modifier::None {
        CHAMPION_XP_ELITE
    } else {
        1.0
    }
}

//a new champion of a kind, which joins the army where it is put
pub(crate) fn champion_unit(kind: usize, at: Vec2) -> Unit {
    let mut unit = Unit::new(at.x, at.y, UNIT_MAX_SPEED);
    unit.champion = kind as u8 + 1;
    unit.level = 1;
    unit.health = champion_health(1);
    unit.age = 0.0;
    unit.power = CHAMPIONS[kind].power.first_wait();
    unit
}

//a champion gains a level: its health is mended in full, there is more room for it, and it flares so the player sees
pub(crate) fn level_up(unit: &mut Unit, fx: &mut Effects) {
    let Some(kind) = unit.champion_kind() else {
        return;
    };
    if unit.level >= CHAMPION_MAX_LEVEL {
        return;
    }
    unit.level += 1;
    unit.health = unit.max_health();
    unit.guard = 0.0;
    let at = Vec2::new(unit.x, unit.y);
    fx.ring(at, 220.0, kind.color);
    fx.burst(at, kind.color, 30, 320.0);
    fx.popup(at, unit.radius() + 8.0, 0.0, format!("LEVEL {}", unit.level), kind.color);
    fx.cue_at(Cue::Champion, at);
}

//what the enemies destroyed in a frame teach the champions, each of which gets all of it and rises a level when it has enough
pub(crate) fn train_champions(units: &mut [Unit], xp: f32, fx: &mut Effects) {
    if xp <= 0.0 {
        return;
    }
    for unit in units.iter_mut().filter(|unit| unit.champion != 0 && unit.level < CHAMPION_MAX_LEVEL) {
        unit.xp += xp;
        while unit.level < CHAMPION_MAX_LEVEL && unit.xp >= champion_xp_needed(unit.level) {
            unit.xp -= champion_xp_needed(unit.level);
            level_up(unit, fx);
            if let Some(kind) = unit.champion_kind() {
                fx.announce(format!("{} REACHES LEVEL {}", kind.name.to_uppercase(), unit.level), kind.color);
            }
        }
        if unit.level >= CHAMPION_MAX_LEVEL {
            unit.xp = 0.0;
        }
    }
}

//how many champions there are in the army
pub(crate) fn champions_held<'a>(units: impl IntoIterator<Item = &'a Unit>) -> usize {
    units.into_iter().filter(|unit| unit.champion != 0).count()
}

//how many of each kind
pub(crate) fn champions_by_kind<'a>(units: impl IntoIterator<Item = &'a Unit>) -> [usize; CHAMPIONS.len()] {
    let mut held = [0; CHAMPIONS.len()];
    for unit in units {
        if let Some(kind) = (unit.champion as usize).checked_sub(1).filter(|&kind| kind < CHAMPIONS.len()) {
            held[kind] += 1;
        }
    }
    held
}

// ---------- winning them ----------

//where a champion came from, which says how good it is likely to be
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Odds {
    Barracks,
    Elite,
    Boss(u32), //the tier of the boss, each time round the bosses have been the better the chances
    Horde(u32), //a chest from a broken horde, of this tier (2 silver, 3 gold): a gold one never holds a common champion
    Citadel,   //always one of the best
}

impl Odds {
    //how likely each grade is, as weights: common, uncommon, rare
    pub(crate) fn weights(self) -> [f32; 3] {
        match self {
            Odds::Barracks | Odds::Elite => [6.0, 3.0, 1.0],
            Odds::Boss(tier) => [3.0, 3.0, 1.5 + 0.5 * tier as f32],
            Odds::Horde(tier) => [(6.0 - 2.0 * tier as f32).max(0.0), 2.0 + tier as f32, (tier as f32 - 1.0).max(0.0)],
            Odds::Citadel => [0.0, 0.0, 1.0],
        }
    }
}

fn grade_index(grade: Rarity) -> usize {
    match grade {
        Rarity::Common => 0,
        Rarity::Uncommon => 1,
        Rarity::Rare => 2,
    }
}

//the kind to give out, from a roll between 0 and 1: a grade is picked by the weights and then one of its kinds, and a kind the
//army has already is less likely for each it has, so the roster doesn't fill up with the same one
#[cfg(test)]
pub(crate) fn pick_champion(units: &[Unit], weights: [f32; 3], roll: f32) -> usize {
    pick_champion_among(champions_by_kind(units), weights, roll)
}

//the same when the champions the army has are already counted, by kind
pub(crate) fn pick_champion_among(held: [usize; CHAMPIONS.len()], weights: [f32; 3], roll: f32) -> usize {
    let in_grade = |grade: Rarity| CHAMPIONS.iter().filter(|champion| champion.grade == grade).count() as f32;
    let weight = |kind: usize| {
        let champion = &CHAMPIONS[kind];
        weights[grade_index(champion.grade)] / in_grade(champion.grade) * CHAMPION_REPEAT_WEIGHT.powi(held[kind] as i32)
    };
    let total: f32 = (0..CHAMPIONS.len()).map(weight).sum();
    let mut pick = roll.clamp(0.0, 0.999_999) * total;
    for kind in 0..CHAMPIONS.len() {
        if pick < weight(kind) {
            return kind;
        }
        pick -= weight(kind);
    }
    (0..CHAMPIONS.len()).rev().find(|&kind| weight(kind) > 0.0).unwrap_or(0)
}

//whether a boss that has just been slain drops a champion. the first always does, so every run has one early
pub(crate) fn boss_drops_champion(bosses_slain: usize, roll: f32) -> bool {
    bosses_slain <= 1 || roll < CHAMPION_BOSS_CHANCE
}

//a champion joins a player's army where `at` is, in a flare of its colour. one of a kind that army already has merges into it
//instead, raising its level, and takes no room. the roster has room for MAX_CHAMPIONS kinds, and a new one won past that is turned
//into recruits. `units` is everyone's, and only the units of that player are looked at. returns the kind that was won
pub(crate) fn award_champion_for(owner: usize, units: &mut Vec<Unit>, babies: &mut Vec<Baby>, at: Vec2, odds: Odds, cap: usize, fx: &mut Effects) -> Option<usize> {
    let mine = |unit: &Unit| unit.owner as usize == owner;
    let kind = pick_champion_among(champions_by_kind(units.iter().filter(|unit| mine(unit))), odds.weights(), rand::gen_range(0.0, 1.0));
    let champion = &CHAMPIONS[kind];
    //one of a kind the army has already merges into it, which rises a level (the highest one that can still rise), and takes up no room
    if let Some(veteran) = units.iter_mut().filter(|unit| mine(unit) && unit.champion as usize == kind + 1 && unit.level < CHAMPION_MAX_LEVEL).max_by_key(|unit| unit.level) {
        level_up(veteran, fx);
        let spot = Vec2::new(veteran.x, veteran.y);
        fx.announce(format!("{} MERGES: LEVEL {}", champion.name.to_uppercase(), veteran.level), champion.color);
        fx.ring(spot, 320.0, TROPHY);
        fx.add_punch(0.4);
        return Some(kind);
    }
    if champions_held(units.iter().filter(|unit| mine(unit))) >= MAX_CHAMPIONS {
        let army = units.iter().filter(|unit| mine(unit)).count();
        spread_babies(babies, at, CHAMPION_RECRUITS, army, cap, TILE_SIZE * 3.0);
        fx.announce(format!("ROSTER FULL: {CHAMPION_RECRUITS} RECRUITS INSTEAD"), TROPHY);
        fx.cue(Cue::Liberate);
        return None;
    }
    let spot = at + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(0.0, RADIUS * 2.0);
    units.push(champion_unit(kind, spot).owned_by(owner));
    fx.announce(format!("{} JOINS YOUR ARMY", champion.name.to_uppercase()), champion.color);
    fx.popup(spot, RADIUS * 3.0, 0.0, format!("{}: {}", champion.grade.name(), champion.name), champion.color);
    fx.ring(spot, 200.0, champion.color);
    fx.burst(spot, champion.color, 40, 360.0);
    fx.add_punch(0.5);
    fx.cue(Cue::Champion);
    Some(kind)
}

//hand out the champions the frame has won, each where it was won, to the player who won it, with the room their relics leave their army
pub(crate) fn award_recruits<R: Relics + ?Sized>(units: &mut Vec<Unit>, babies: &mut Vec<Baby>, relics: &R, fx: &mut Effects) {
    for (at, odds, owner) in std::mem::take(&mut fx.recruits) {
        award_champion_for(owner, units, babies, at, odds, relics.of(owner as u8).unit_cap(), fx);
    }
}

// ---------- what they do ----------

//the damage a champion's power does to an enemy: `flat` units' worth of a second, and a share of its health, a boss taking only
//a part of that share. the Whetstone relic and the like strengthen it as they do the units
fn power_damage(enemy: &Enemy, flat: f32, share: f32, upgrades: &Upgrades) -> f32 {
    let share = if enemy.boss.is_some() { share * CHAMPION_BOSS_SHARE } else { share };
    (UNIT_DAMAGE * flat + enemy.max_health * share) * upgrades.hit()
}

//hurt an enemy with a power of a player's champion, and keep count so a number floats up from it. the player has the credit for it
fn strike(enemy: &mut Enemy, damage: f32, owner: u8) {
    let dealt = enemy.take_damage(damage);
    enemy.hurt += dealt;
    enemy.credit = owner;
}

fn in_reach(enemy: &Enemy, from: Vec2, reach: f32) -> bool {
    enemy.health > 0.0 && !enemy.lurking() && enemy.position.distance(from) <= reach + enemy.size / 2.0
}

//a streak of light between two points
fn streak(fx: &mut Effects, from: Vec2, to: Vec2, color: Color) {
    fx.beams.push(Beam { from, to, color, age: 0.0 });
}

//what every unit does to the others by what it is, each frame: it is given the might and the ward its kind has, and what grows
//from them (a berserker's rage, a big army's), the auras of the champions that have them reach the units near, and a champion
//heals on its own. `dt` is how long the frame was
pub(crate) fn apply_champions(units: &mut [Unit], dt: f32) {
    let growth = units.len() as f32 / CHAMPION_MIGHT_UNITS;
    let mut auras: Vec<(Vec2, Power)> = Vec::new();
    for unit in units.iter_mut() {
        let Some(kind) = unit.champion_kind() else {
            unit.might = 1.0;
            unit.ward = 1.0;
            continue;
        };
        let steps = unit.level.max(1) as i32 - 1;
        let power = kind.power.at_level(unit.level);
        let most = unit.max_health();
        //its might is one unit's and then what it has over that, which grows with the army and with its level
        let mut might = (1.0 + (kind.might - 1.0) * (1.0 + growth)) * (1.0 + CHAMPION_LEVEL_MIGHT * steps as f32);
        if let Power::Rage { extra } = power {
            might *= 1.0 + extra * (1.0 - unit.health / most).clamp(0.0, 1.0);
        }
        unit.might = might;
        unit.ward = (kind.ward * CHAMPION_LEVEL_WARD.powi(steps)).max(MIN_WARD);
        if unit.health < most {
            unit.health = (unit.health + (CHAMPION_REGEN + most * CHAMPION_REGEN_SHARE) * dt).min(most);
        }
        if power.aura().is_some() {
            auras.push((Vec2::new(unit.x, unit.y), power));
        }
    }
    if auras.is_empty() {
        return;
    }
    let grid = UnitGrid::new(units);
    let mut near = Vec::new();
    for (at, power) in auras {
        let Some(radius) = power.aura() else {
            continue;
        };
        near.clear();
        grid.near(at - Vec2::splat(radius), at + Vec2::splat(radius), &mut near);
        for &i in &near {
            let unit = &mut units[i];
            if unit.garrison.is_some() || Vec2::new(unit.x, unit.y).distance(at) > radius {
                continue;
            }
            match power {
                Power::Banner { might, .. } => unit.might *= might,
                Power::Bulwark { ward, .. } => unit.ward = (unit.ward * ward).max(MIN_WARD),
                Power::Mend { per_second, .. } if unit.health < unit.max_health() => unit.health = (unit.health + per_second * dt).min(unit.max_health()),
                _ => {}
            }
        }
    }
}

pub(crate) const MIN_WARD: f32 = 0.1; //however many shields are stacked, at least this share of the damage gets through

//the champions' powers that act on the world, each frame. the timers count down and each power goes off when its time is up and
//there is something for it to do. a rebirth that happened last frame blazes now. it has to come after the terrain has set the
//enemies' speeds for the frame, as the ones that slow them take from that
pub(crate) fn champion_acts<R: Relics + ?Sized>(units: &mut [Unit], enemies: &mut [Enemy], babies: &mut Vec<Baby>, relics: &R, dt: f32, fx: &mut Effects) {
    if dt <= 0.0 {
        return;
    }
    for (at, kind, level, owner) in std::mem::take(&mut fx.rebirths) {
        let upgrades = relics.of(owner);
        let Some(champion) = (kind as usize).checked_sub(1).and_then(|kind| CHAMPIONS.get(kind)) else {
            continue;
        };
        let Power::Rebirth { radius, flat, share, .. } = champion.power.at_level(level) else {
            continue;
        };
        for enemy in enemies.iter_mut().filter(|enemy| in_reach(enemy, at, radius)) {
            let damage = power_damage(enemy, flat, share, upgrades);
            strike(enemy, damage, owner);
        }
        fx.ring(at, radius, champion.color);
        fx.burst(at, champion.color, 60, 420.0);
        fx.mark(at, radius * 0.6, champion.color, MarkKind::Scorch);
        fx.add_shake(0.3);
        fx.cue_at(Cue::Explosion, at);
    }
    //the powers of a champion are the relics of the player it fights for, and a muster is held to the size of that player's army
    let sizes = army_sizes(units);
    for unit in units.iter_mut().filter(|unit| unit.champion != 0 && unit.garrison.is_none()) {
        let Some(kind) = unit.champion_kind() else {
            continue;
        };
        let at = Vec2::new(unit.x, unit.y);
        let owner = unit.owner;
        let upgrades = relics.of(owner);
        let (army, cap) = (sizes[owner_index(owner)], upgrades.unit_cap());
        let mut timer = unit.power;
        match kind.power.at_level(unit.level) {
            Power::Fireball { every, range, radius, flat, share } => {
                timer -= dt;
                if timer <= 0.0 {
                    timer = match fireball_target(enemies, at, range, radius) {
                        Some(target) => {
                            for enemy in enemies.iter_mut().filter(|enemy| in_reach(enemy, target, radius)) {
                                let damage = power_damage(enemy, flat, share, upgrades);
                                strike(enemy, damage, owner);
                            }
                            streak(fx, at, target, kind.color);
                            fx.ring(target, radius, kind.color);
                            fx.burst(target, kind.color, 36, 340.0);
                            fx.mark(target, radius * 0.8, kind.color, MarkKind::Scorch);
                            fx.add_shake(0.1);
                            fx.cue_at(Cue::Explosion, target);
                            every
                        }
                        None => 0.25, //nothing to burn, look again soon
                    };
                }
            }
            Power::Snipe { every, range, flat, share } => {
                timer -= dt;
                if timer <= 0.0 {
                    let toughest = enemies
                        .iter()
                        .enumerate()
                        .filter(|(_, enemy)| in_reach(enemy, at, range))
                        .max_by(|a, b| (a.1.health + a.1.shield).total_cmp(&(b.1.health + b.1.shield)))
                        .map(|(i, _)| i);
                    timer = match toughest {
                        Some(i) => {
                            let damage = power_damage(&enemies[i], flat, share, upgrades);
                            strike(&mut enemies[i], damage, owner);
                            streak(fx, at, enemies[i].position, kind.color);
                            fx.burst(enemies[i].position, kind.color, 8, 180.0);
                            fx.cue_at(Cue::Arrow, at);
                            every
                        }
                        None => 0.2,
                    };
                }
            }
            Power::Lightning { every, range, jumps, flat, share } => {
                timer -= dt;
                if timer <= 0.0 {
                    let chain = lightning_chain(enemies, at, range, jumps);
                    timer = if chain.is_empty() {
                        0.25
                    } else {
                        let mut from = at;
                        for &i in &chain {
                            let damage = power_damage(&enemies[i], flat, share, upgrades);
                            strike(&mut enemies[i], damage, owner);
                            streak(fx, from, enemies[i].position, kind.color);
                            fx.burst(enemies[i].position, kind.color, 10, 220.0);
                            from = enemies[i].position;
                        }
                        fx.add_shake(0.05);
                        fx.cue_at(Cue::Zap, at);
                        every
                    };
                }
            }
            Power::Quake { every, radius, flat, share } => {
                timer -= dt;
                if timer <= 0.0 {
                    timer = if enemies.iter().any(|enemy| in_reach(enemy, at, radius)) {
                        for enemy in enemies.iter_mut().filter(|enemy| in_reach(enemy, at, radius)) {
                            let damage = power_damage(enemy, flat, share, upgrades);
                            strike(enemy, damage, owner);
                        }
                        fx.ring(at, radius, kind.color);
                        fx.burst(at, kind.color, 50, 360.0);
                        fx.mark(at, radius * 0.7, kind.color, MarkKind::Scorch);
                        fx.add_shake(0.35);
                        fx.cue_at(Cue::Slam, at);
                        every
                    } else {
                        0.25
                    };
                }
            }
            Power::Frost { radius, slow, every, freeze } => {
                timer -= dt;
                if timer <= 0.0 {
                    timer = every;
                    fx.ring(at, radius, kind.color);
                    fx.cue_at(Cue::Frost, at);
                }
                let freezing = timer > every - freeze;
                for enemy in enemies.iter_mut().filter(|enemy| in_reach(enemy, at, radius)) {
                    let keeps = if freezing { if enemy.boss.is_some() { FROZEN_BOSS_SLOW } else { FROZEN_SLOW } } else { slow };
                    enemy.slow = enemy.slow.min(keeps);
                }
            }
            Power::Muster { every, count } => {
                timer -= dt;
                if timer <= 0.0 {
                    timer = if units_room(army, babies.len(), cap) {
                        spread_babies(babies, at, count, army, cap, RADIUS * 5.0);
                        fx.burst(at, GREEN, 12, 200.0);
                        fx.ring(at, RADIUS * 5.0, kind.color);
                        fx.cue_at(Cue::Dispatch, at);
                        every
                    } else {
                        1.0
                    };
                }
            }
            Power::Rebirth { .. } => timer = (timer - dt).max(0.0),
            Power::Brute | Power::Rage { .. } | Power::Banner { .. } | Power::Bulwark { .. } | Power::Mend { .. } | Power::Scout { .. } | Power::Quartermaster { .. } => {}
        }
        unit.power = timer;
    }
}

fn units_room(army: usize, waiting: usize, cap: usize) -> bool {
    army + waiting < cap
}

//where a fireball should land: the enemy in range with the most others round it, the toughest of those if it is a tie
fn fireball_target(enemies: &[Enemy], from: Vec2, range: f32, radius: f32) -> Option<Vec2> {
    let candidates: Vec<&Enemy> = enemies.iter().filter(|enemy| in_reach(enemy, from, range)).collect();
    candidates
        .iter()
        .map(|enemy| {
            let crowd = candidates.iter().filter(|other| other.position.distance(enemy.position) <= radius + other.size / 2.0).count();
            (crowd, enemy.health, enemy.position)
        })
        .max_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)))
        .map(|(_, _, position)| position)
}

//the enemies a bolt of lightning goes through, as indexes: the nearest in range of the champion, and then the nearest not yet hit
//that is within CHAMPION_JUMP_RANGE of the last, up to `jumps` of them
fn lightning_chain(enemies: &[Enemy], from: Vec2, range: f32, jumps: usize) -> Vec<usize> {
    let nearest = |to: Vec2, reach: f32, hit: &[usize]| {
        enemies
            .iter()
            .enumerate()
            .filter(|(i, enemy)| !hit.contains(i) && in_reach(enemy, to, reach))
            .min_by(|a, b| a.1.position.distance_squared(to).total_cmp(&b.1.position.distance_squared(to)))
            .map(|(i, _)| i)
    };
    let mut chain = Vec::new();
    let mut last = from;
    let mut reach = range;
    while chain.len() < jumps {
        let Some(next) = nearest(last, reach, &chain) else {
            break;
        };
        chain.push(next);
        last = enemies[next].position;
        reach = CHAMPION_JUMP_RANGE;
    }
    chain
}

// ---------- what they give the whole army ----------

//what the champions alive add up to for the army as a whole
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Perks {
    pub(crate) speed: f32,  //times the speed of the player and the army
    pub(crate) pickup: f32, //times how far babies are picked up from
    pub(crate) bounty: f32, //times the babies enemies drop
    pub(crate) points: f32, //wall points a second
}

impl Perks {
    pub(crate) fn of(units: &[Unit]) -> Perks {
        let mut perks = Perks { speed: 1.0, pickup: 1.0, bounty: 1.0, points: 0.0 };
        let champions: Vec<&Unit> = units.iter().filter(|unit| unit.champion != 0).collect();
        if champions.is_empty() {
            return perks;
        }
        for (kind, champion) in CHAMPIONS.iter().enumerate() {
            if !matches!(champion.power, Power::Scout { .. } | Power::Quartermaster { .. }) {
                continue;
            }
            //the best few of a kind each add their share, at their level, the rest nothing
            let mut levels: Vec<u8> = champions.iter().filter(|unit| unit.champion as usize == kind + 1).map(|unit| unit.level).collect();
            levels.sort_unstable_by(|a, b| b.cmp(a));
            levels.truncate(CHAMPION_STACK);
            let (mut speed, mut pickup, mut bounty, mut points) = (0.0, 0.0, 0.0, 0.0);
            for level in levels {
                match champion.power.at_level(level) {
                    Power::Scout { speed: faster, pickup: further } => {
                        speed += faster - 1.0;
                        pickup += further - 1.0;
                    }
                    Power::Quartermaster { points: wall_points, bounty: more } => {
                        bounty += more - 1.0;
                        points += wall_points;
                    }
                    _ => {}
                }
            }
            perks.speed *= 1.0 + speed;
            perks.pickup *= 1.0 + pickup;
            perks.bounty *= 1.0 + bounty;
            perks.points += points;
        }
        perks
    }

    //the relics' numbers with the perks on top
    pub(crate) fn applied(&self, upgrades: &Upgrades) -> Upgrades {
        let mut perked = upgrades.clone();
        perked.speed *= self.speed;
        perked.pickup *= self.pickup;
        perked.bounty *= self.bounty;
        perked
    }
}

// ---------- seeing them ----------

//the champions in the world: the reach of the auras they have as a faint circle, then each one's body, a polygon of its kind with
//its two letters in the middle, ringed in the colour of its grade, and a bar when it is hurt. `zoom` is for keeping the lines and
//the letters the same size on screen, `time` for the circles to pulse
pub(crate) fn draw_champions(units: &[Unit], view: Rect, zoom: f32, time: f32) {
    let pixel = 1.0 / zoom;
    for unit in units.iter().filter(|unit| unit.champion != 0 && unit.garrison.is_none()) {
        let Some(kind) = unit.champion_kind() else {
            continue;
        };
        let (x, y) = (unit.x, unit.y);
        let reach = kind.power.at_level(unit.level).aura().unwrap_or(0.0);
        let seen = Rect::new(view.x - reach, view.y - reach, view.w + reach * 2.0, view.h + reach * 2.0);
        if !seen.contains(Vec2::new(x, y)) {
            continue;
        }
        if reach > 0.0 {
            let pulse = 0.5 + 0.5 * (time * 2.0 + x * 0.01).sin();
            draw_circle(x, y, reach, Color::new(kind.color.r, kind.color.g, kind.color.b, 0.05));
            draw_circle_lines(x, y, reach, 2.0 * pixel, Color::new(kind.color.r, kind.color.g, kind.color.b, 0.22 + 0.12 * pulse));
        }
        let radius = unit.radius();
        let body = unit_color(unit);
        draw_poly(x, y, kind.sides, radius + UNIT_BORDER, kind.turn, BLACK);
        draw_poly(x, y, kind.sides, radius, kind.turn, body);
        draw_poly(x, y, kind.sides, radius * 0.58, kind.turn, lerp_color(body, WHITE, 0.5));
        let grade = kind.grade.color();
        draw_circle_lines(x, y, radius + 5.0, 2.5 * pixel.max(0.6), Color::new(grade.r, grade.g, grade.b, 0.85));
        let size = 15.0;
        let width = text_width(kind.tag, size, Face::Header) * pixel;
        draw_label_scaled(kind.tag, x - width / 2.0, y + size * 0.36 * pixel, size, pixel, Color::new(0.04, 0.06, 0.1, 1.0), Face::Header);
        //a champion that has just arrived sends a ring out
        if unit.age < 1.5 {
            let t = unit.age / 1.5;
            draw_circle_lines(x, y, radius + 90.0 * t, 3.0 * pixel, Color::new(kind.color.r, kind.color.g, kind.color.b, 1.0 - t));
        }
        //a pip for each level it has above the first, under it
        for pip in 1..unit.level {
            let across = (pip as f32 - unit.level as f32 / 2.0) * 7.0 * pixel.max(0.6);
            draw_circle(x + across, y + radius + 9.0 * pixel.max(0.6), 2.4 * pixel.max(0.6), TROPHY);
        }
        if unit.health < unit.max_health() {
            let (width, height) = (radius * 2.0, 5.0 * pixel.max(0.5));
            let share = (unit.health / unit.max_health()).clamp(0.0, 1.0);
            let top = y - radius - height - 6.0 * pixel;
            draw_rectangle(x - width / 2.0 - pixel, top - pixel, width + 2.0 * pixel, height + 2.0 * pixel, Color::new(0.0, 0.0, 0.0, 0.8));
            draw_rectangle(x - width / 2.0, top, width * share, height, lerp_color(wound_color(), pickup_color(), share));
        }
    }
}

pub(crate) const ROSTER_WIDTH: f32 = 204.0;
pub(crate) const ROSTER_ROW: f32 = 32.0;
pub(crate) const ROSTER_GAP: f32 = 4.0;
pub(crate) const ROSTER_TOP: f32 = 34.0; //room for the heading over the first row

//where each champion's row is, down the right edge of the screen from the top
pub(crate) fn roster_rows(count: usize, screen: Vec2) -> Vec<Rect> {
    (0..count).map(|i| Rect::new(screen.x - 12.0 - ROSTER_WIDTH, ROSTER_TOP + 12.0 + (ROSTER_ROW + ROSTER_GAP) * i as f32, ROSTER_WIDTH, ROSTER_ROW)).collect()
}

//the champions, by kind and then in the order they joined, which is what the roster lists
pub(crate) fn roster_of(units: &[Unit]) -> Vec<&Unit> {
    let mut roster: Vec<&Unit> = units.iter().filter(|unit| unit.champion != 0).collect();
    roster.sort_by_key(|unit| unit.champion);
    roster
}

//a champion's polygon as a small icon, centred on a point
fn draw_icon(kind: &Champion, at: Vec2, radius: f32) {
    draw_poly(at.x, at.y, kind.sides, radius + 1.5, kind.turn, BLACK);
    draw_poly(at.x, at.y, kind.sides, radius, kind.turn, kind.color);
}

//the champions along the right edge of the screen, each with its health, and what the one under the mouse is and does
pub(crate) fn draw_roster(units: &[Unit], mouse: Vec2) {
    let roster = roster_of(units);
    if roster.is_empty() {
        return;
    }
    let screen = Vec2::new(screen_width(), screen_height());
    let rows = roster_rows(roster.len(), screen);
    draw_label_right(&format!("CHAMPIONS {}/{}", roster.len(), MAX_CHAMPIONS), screen.x - 14.0, ROSTER_TOP - 2.0 + 12.0, 15.0, CYAN, Face::Bold);
    let mut hovered = None;
    for (unit, row) in roster.iter().zip(&rows) {
        let Some(kind) = unit.champion_kind() else {
            continue;
        };
        draw_rectangle(row.x, row.y, row.w, row.h, PANEL);
        draw_rectangle(row.x, row.y, 4.0, row.h, kind.color);
        let grade = kind.grade.color();
        draw_rectangle_lines(row.x, row.y, row.w, row.h, 1.5, Color::new(grade.r, grade.g, grade.b, 0.6));
        draw_icon(kind, Vec2::new(row.x + 22.0, row.y + row.h / 2.0), 9.0);
        draw_label(kind.name, row.x + 38.0, row.y + 15.0, 16.0, INK, Face::Bold);
        draw_label_right(&format!("Lv {}", unit.level), row.x + row.w - 8.0, row.y + 15.0, 14.0, if unit.level >= CHAMPION_MAX_LEVEL { TROPHY } else { MUTED }, Face::Bold);
        let share = (unit.health / unit.max_health()).clamp(0.0, 1.0);
        draw_meter(row.x + 38.0, row.y + 20.0, row.w - 48.0, 5.0, share, lerp_color(wound_color(), pickup_color(), share));
        //how far it is to the next level, a thin line under its health
        if unit.level < CHAMPION_MAX_LEVEL {
            let learned = (unit.xp / champion_xp_needed(unit.level)).clamp(0.0, 1.0);
            draw_rectangle(row.x + 38.0, row.y + 27.0, (row.w - 48.0) * learned, 2.0, TROPHY);
        }
        if row.contains(mouse) {
            hovered = Some((*unit, *row));
        }
    }
    if let Some((unit, row)) = hovered {
        let Some(kind) = unit.champion_kind() else {
            return;
        };
        let mut lines = vec![(kind.blurb.to_owned(), INK)];
        if unit.level > 1 {
            let more = (CHAMPION_LEVEL_POWER * (unit.level - 1) as f32 * 100.0).round();
            lines.push((format!("level {}: its power is {more:.0}% stronger and it goes off {:.0}% faster", unit.level, CHAMPION_LEVEL_PACE * (unit.level - 1) as f32 * 100.0), TROPHY));
        }
        lines.push((format!("health {:.0}/{:.0}   takes {:.0}% of the damage", unit.health.max(0.0), unit.max_health(), (unit.ward * 100.0).round()), MUTED));
        if unit.level < CHAMPION_MAX_LEVEL {
            lines.push((format!("{:.0}/{:.0} to level {}   (a second {} merges into it)", unit.xp, champion_xp_needed(unit.level), unit.level + 1, kind.name), MUTED));
        } else {
            lines.push(("highest level".to_owned(), TROPHY));
        }
        if let Power::Rebirth { .. } = kind.power {
            lines.push((if unit.power > 0.0 { format!("reborn again in {}", clock(unit.power)) } else { "ready to rise again".to_owned() }, MUTED));
        }
        let title = format!("{}  {}  Lv {}", kind.name, kind.grade.name(), unit.level);
        let width = lines.iter().map(|(line, _)| text_width(line, 16.0, Face::Body)).fold(text_width(&title, 20.0, Face::Header), f32::max) + 28.0;
        let height = 40.0 + 22.0 * lines.len() as f32;
        let x = (row.x - width - 8.0).max(4.0);
        let y = row.y.min(screen_height() - height - 4.0);
        panel_edged(x, y, width, height, kind.color);
        draw_label(&title, x + 14.0, y + 26.0, 20.0, kind.color, Face::Header);
        for (i, (line, color)) in lines.iter().enumerate() {
            draw_label(line, x + 14.0, y + 48.0 + 22.0 * i as f32, 16.0, *color, Face::Body);
        }
    }
}
