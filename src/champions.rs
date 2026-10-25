//! champions: special units won as loot from bosses, elite enemies and barracks. each is a unit in the army (it follows the
//! player, is pushed about and hurt like the rest) with a body of its own, much more might and one power: some fight as many
//! units, some strengthen or mend the units round them, some strike the enemies from where they stand, and some change what
//! the whole army gets from the run. there are fourteen kinds, in three grades of how rare they are

use crate::prelude::*;
use std::cell::RefCell;
use std::sync::OnceLock;

//what a champion does besides fighting by touch. the amounts are in the table of kinds
#[derive(Clone, Copy, Debug)]
pub(crate) enum Power {
    Brute,                                //nothing else: it just fights as many units and takes little damage
    Rage { extra: f32 },                  //its might grows by this many times itself as it is hurt
    Banner { radius: f32, might: f32 },   //the units near it do this many times the damage
    Bulwark { radius: f32, ward: f32 },   //the units near it take this share of the damage
    Mend { radius: f32, per_second: f32 }, //the units near it heal this much health a second
    //the powers that hurt do a share of what the whole army does by touch (see `power_damage`): `weight` is that share, as damage a
    //second, so a hit every `every` seconds does `weight * every` seconds' worth of the army's touch to each enemy it reaches
    Fireball { every: f32, range: f32, radius: f32, weight: f32 }, //hurled every so often at the thickest cluster of enemies in range, it hurts all of them
    Snipe { every: f32, range: f32, weight: f32 }, //a shot at the toughest enemy in range
    Lightning { every: f32, range: f32, jumps: usize, weight: f32 }, //a bolt that jumps from enemy to enemy
    Frost { radius: f32, slow: f32, every: f32, freeze: f32 }, //enemies near it keep this share of their speed, and every so often are frozen for a moment
    Quake { every: f32, radius: f32, weight: f32 }, //slams the ground, hurting every enemy round it
    Muster { every: f32, count: usize },  //calls recruits (babies) round itself
    Rebirth { recharge: f32, radius: f32, burst: f32 }, //when it dies it rises again in a blaze that does `burst` seconds' worth of the army's touch to every enemy near, and can't again for a while
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
    #[cfg(test)]
    pub(crate) fn at_level(&self, level: u8) -> Power {
        self.grown(level, &Growth::NONE)
    }

    //the same for a champion that has evolved, whose evolutions add to what its level gives (see `Growth`). the powers that hurt do more a
    //second for going off more often: their weight, which is damage a second, grows with the pace as well as with the strength
    pub(crate) fn grown(&self, level: u8, growth: &Growth) -> Power {
        let steps = level.max(1).saturating_sub(1) as f32;
        let more = (1.0 + CHAMPION_LEVEL_POWER * steps) * growth.power;
        let quicker = (1.0 + CHAMPION_LEVEL_PACE * steps) * growth.pace;
        let further = (1.0 + CHAMPION_LEVEL_REACH * steps) * growth.reach;
        let chill = (1.0 + 0.1 * steps) * growth.power;
        match *self {
            Power::Brute => Power::Brute,
            Power::Rage { extra } => Power::Rage { extra: extra * more },
            Power::Banner { radius, might } => Power::Banner { radius: radius * further, might: 1.0 + (might - 1.0) * more },
            Power::Bulwark { radius, ward } => Power::Bulwark { radius: radius * further, ward: ward.powf(more) },
            Power::Mend { radius, per_second } => Power::Mend { radius: radius * further, per_second: per_second * more },
            Power::Fireball { every, range, radius, weight } => Power::Fireball { every: every / quicker, range: range * further, radius: radius * further, weight: weight * more * quicker },
            Power::Snipe { every, range, weight } => Power::Snipe { every: every / quicker, range: range * further, weight: weight * more * quicker },
            Power::Lightning { every, range, jumps, weight } => Power::Lightning { every: every / quicker, range: range * further, jumps: jumps + steps as usize + growth.jumps, weight: weight * more * quicker },
            Power::Frost { radius, slow, every, freeze } => Power::Frost { radius: radius * further, slow: (slow / chill).max(FROST_MIN_SLOW), every: every / quicker, freeze: (freeze * chill).min(every / quicker * 0.5) },
            Power::Quake { every, radius, weight } => Power::Quake { every: every / quicker, radius: radius * further, weight: weight * more * quicker },
            Power::Muster { every, count } => Power::Muster { every: every / quicker, count: (count as f32 * more).round() as usize },
            Power::Rebirth { recharge, radius, burst } => Power::Rebirth { recharge: recharge / ((1.0 + 0.15 * steps) * growth.pace), radius: radius * further, burst: burst * more },
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
    Champion { name: "Juggernaut", tag: "Ju", blurb: "fights as 14 units and takes a fifth of the damage", color: Color::new(0.82, 0.6, 0.35, 1.0), grade: Rarity::Common, sides: 4, turn: 45.0, size: 3.0, might: 14.0, ward: 0.22, power: Power::Brute },
    Champion { name: "Berserker", tag: "Bk", blurb: "fights as 10 units, and as up to 30 the more it is hurt", color: Color::new(0.9, 0.14, 0.2, 1.0), grade: Rarity::Common, sides: 3, turn: -90.0, size: 2.4, might: 10.0, ward: 0.5, power: Power::Rage { extra: 2.0 } },
    Champion { name: "Bulwark", tag: "Bw", blurb: "the units near it take a quarter less damage", color: Color::new(0.45, 0.6, 0.92, 1.0), grade: Rarity::Common, sides: 6, turn: 0.0, size: 2.6, might: 3.0, ward: 0.3, power: Power::Bulwark { radius: 420.0, ward: 0.75 } },
    Champion { name: "Medic", tag: "Md", blurb: "mends the units near it, 14 health a second each", color: Color::new(0.4, 0.95, 0.6, 1.0), grade: Rarity::Common, sides: 8, turn: 22.5, size: 2.0, might: 1.0, ward: 0.6, power: Power::Mend { radius: 380.0, per_second: 14.0 } },
    Champion { name: "Drummer", tag: "Dr", blurb: "calls 4 recruits every 9 seconds", color: Color::new(0.95, 0.72, 0.28, 1.0), grade: Rarity::Common, sides: 5, turn: -90.0, size: 2.0, might: 2.0, ward: 0.6, power: Power::Muster { every: 9.0, count: 4 } },
    Champion { name: "Pyromancer", tag: "Py", blurb: "hurls a fireball every 4 seconds at the thickest crowd of enemies", color: Color::new(1.0, 0.5, 0.08, 1.0), grade: Rarity::Uncommon, sides: 3, turn: 90.0, size: 2.1, might: 2.0, ward: 0.7, power: Power::Fireball { every: 4.2, range: 850.0, radius: 190.0, weight: 0.07 } },
    Champion { name: "Sharpshooter", tag: "Sh", blurb: "picks off the toughest enemy in sight, twice a second", color: Color::new(0.75, 0.95, 0.3, 1.0), grade: Rarity::Uncommon, sides: 4, turn: 0.0, size: 2.0, might: 1.0, ward: 0.7, power: Power::Snipe { every: 0.5, range: 1000.0, weight: 0.1 } },
    Champion { name: "Pathfinder", tag: "Pf", blurb: "the army moves 12% faster and picks babies up from 35% further", color: Color::new(0.2, 0.85, 0.8, 1.0), grade: Rarity::Uncommon, sides: 5, turn: 0.0, size: 2.0, might: 1.0, ward: 0.7, power: Power::Scout { speed: 1.12, pickup: 1.35 } },
    Champion { name: "Quartermaster", tag: "Qm", blurb: "earns 1.5 wall points a second, and enemies drop 25% more babies", color: Color::new(0.8, 0.72, 0.5, 1.0), grade: Rarity::Uncommon, sides: 6, turn: 30.0, size: 2.1, might: 1.0, ward: 0.7, power: Power::Quartermaster { points: 1.5, bounty: 1.25 } },
    Champion { name: "Warlord", tag: "Wl", blurb: "the units round its banner hit 35% harder", color: Color::new(0.85, 0.35, 0.95, 1.0), grade: Rarity::Uncommon, sides: 5, turn: 90.0, size: 2.5, might: 6.0, ward: 0.4, power: Power::Banner { radius: 520.0, might: 1.35 } },
    Champion { name: "Frostweaver", tag: "Fw", blurb: "slows enemies near it by 45%, and every 10 seconds freezes them for 1.5", color: Color::new(0.6, 0.86, 1.0, 1.0), grade: Rarity::Rare, sides: 6, turn: 0.0, size: 2.1, might: 1.0, ward: 0.7, power: Power::Frost { radius: 540.0, slow: 0.55, every: 10.0, freeze: 1.5 } },
    Champion { name: "Stormcaller", tag: "St", blurb: "a bolt of lightning every 3 seconds that jumps through 6 enemies", color: Color::new(0.62, 0.5, 1.0, 1.0), grade: Rarity::Rare, sides: 4, turn: 45.0, size: 2.2, might: 2.0, ward: 0.6, power: Power::Lightning { every: 3.0, range: 750.0, jumps: 6, weight: 0.06 } },
    Champion { name: "Earthshaker", tag: "Es", blurb: "slams the ground every 6 seconds, crushing everything round it", color: Color::new(0.72, 0.34, 0.2, 1.0), grade: Rarity::Rare, sides: 8, turn: 0.0, size: 2.8, might: 8.0, ward: 0.35, power: Power::Quake { every: 6.5, radius: 300.0, weight: 0.08 } },
    Champion { name: "Phoenix", tag: "Ph", blurb: "rises from its ashes when it dies, burning every enemy near it. once every 90 seconds", color: Color::new(1.0, 0.82, 0.25, 1.0), grade: Rarity::Rare, sides: 3, turn: -90.0, size: 2.4, might: 5.0, ward: 0.5, power: Power::Rebirth { recharge: 90.0, radius: 340.0, burst: 4.0 } },
];

impl Champion {
    //whether it is a tank that draws the enemies onto itself: the ones chasing the player that come near it go for it instead (see
    //`patrol_enemies_among`), which is what its toughness is for
    pub(crate) fn taunts(&self) -> bool {
        matches!(self.power, Power::Brute | Power::Bulwark { .. })
    }
}

//the index of the kind with this name
pub(crate) fn champion_named(name: &str) -> Option<usize> {
    CHAMPIONS.iter().position(|champion| champion.name == name)
}

// ---------- evolutions ----------

//one change an evolution makes to a champion, as times what it was (`Jumps` is a number more)
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Change {
    Strength(f32), //what its power does: damage, healing, recruits, the strength of its aura
    Pace(f32),     //how often its power goes off (and how soon a phoenix can rise again)
    Reach(f32),    //how far its power reaches: its aura, its range, its blast
    Might(f32),    //the might it fights with by touch
    Ward(f32),     //the share of the damage that reaches it, so less is tougher
    Health(f32),   //the most health it can have
    Jumps(usize),  //more enemies a bolt of lightning jumps through
}

//what a champion's evolutions add up to, on top of what its level gives: all 1, and no more jumps, for none
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Growth {
    pub(crate) power: f32,
    pub(crate) pace: f32,
    pub(crate) reach: f32,
    pub(crate) might: f32,
    pub(crate) ward: f32,
    pub(crate) health: f32,
    pub(crate) jumps: usize,
}

impl Growth {
    pub(crate) const NONE: Growth = Growth { power: 1.0, pace: 1.0, reach: 1.0, might: 1.0, ward: 1.0, health: 1.0, jumps: 0 };

    fn with(mut self, change: Change) -> Growth {
        match change {
            Change::Strength(times) => self.power *= times,
            Change::Pace(times) => self.pace *= times,
            Change::Reach(times) => self.reach *= times,
            Change::Might(times) => self.might *= times,
            Change::Ward(times) => self.ward *= times,
            Change::Health(times) => self.health *= times,
            Change::Jumps(more) => self.jumps += more,
        }
        self
    }
}

//one of the two ways a champion can grow at a level of evolution: a name, and the changes it makes. (some names, a sharpshooter's piercing
//rounds say, are for what later steps of the upgrade plan give them; for now each is the numbers here)
pub(crate) struct Evolution {
    pub(crate) name: &'static str,
    pub(crate) changes: &'static [Change],
}

const fn evolution(name: &'static str, changes: &'static [Change]) -> Evolution {
    Evolution { name, changes }
}

//the evolutions of each kind of champion, in the order of CHAMPIONS (with its name, to be sure of that): for each of EVOLUTION_LEVELS, two to
//choose between. there is the one, at the top level, as the owner wants it a choice made once rather than one every few levels, so each way is
//a big step. the first of a pair is mostly more of its power, the second mostly its pace, its reach or its own toughness
pub(crate) static EVOLUTIONS: [(&str, [[Evolution; 2]; EVOLUTION_LEVELS.len()]); 14] = [
    ("Juggernaut", [[evolution("Unstoppable", &[Change::Might(1.6)]), evolution("Immovable", &[Change::Health(1.8)])]]),
    ("Berserker", [[evolution("Rampage", &[Change::Strength(1.45), Change::Might(1.25)]), evolution("Undying Rage", &[Change::Health(1.6), Change::Ward(0.8)])]]),
    ("Bulwark", [[evolution("Aegis", &[Change::Strength(1.6)]), evolution("Fortress", &[Change::Reach(1.35), Change::Health(1.4)])]]),
    ("Medic", [[evolution("Miracle Worker", &[Change::Strength(1.6)]), evolution("Sanctuary", &[Change::Reach(1.4)])]]),
    ("Drummer", [[evolution("Call to Arms", &[Change::Strength(1.6)]), evolution("Endless Column", &[Change::Pace(1.5)])]]),
    ("Pyromancer", [[evolution("Sunfire", &[Change::Strength(1.5), Change::Reach(1.15)]), evolution("Rain of Fire", &[Change::Pace(1.5)])]]),
    ("Sharpshooter", [[evolution("Headhunter", &[Change::Strength(1.6)]), evolution("Bullet Storm", &[Change::Pace(1.5)])]]),
    ("Pathfinder", [[evolution("Windrunner", &[Change::Strength(1.6)]), evolution("Pack Mule", &[Change::Health(1.6), Change::Ward(0.75)])]]),
    ("Quartermaster", [[evolution("War Chest", &[Change::Strength(1.6)]), evolution("Iron Stores", &[Change::Health(1.6), Change::Ward(0.75)])]]),
    ("Warlord", [[evolution("Conqueror", &[Change::Strength(1.6)]), evolution("Grand Standard", &[Change::Reach(1.4)])]]),
    ("Frostweaver", [[evolution("Winter's Heart", &[Change::Strength(1.6)]), evolution("Endless Winter", &[Change::Reach(1.3), Change::Pace(1.2)])]]),
    ("Stormcaller", [[evolution("Thunderlord", &[Change::Strength(1.6)]), evolution("Tempest", &[Change::Jumps(6), Change::Reach(1.2)])]]),
    ("Earthshaker", [[evolution("Cataclysm", &[Change::Strength(1.5), Change::Reach(1.15)]), evolution("Seismic Pulse", &[Change::Pace(1.5)])]]),
    ("Phoenix", [[evolution("Supernova", &[Change::Strength(1.5), Change::Reach(1.2)]), evolution("Undying", &[Change::Pace(1.6)])]]),
];

//the two ways of each level of evolution for a kind of champion (its place in CHAMPIONS)
pub(crate) fn evolutions_of(kind: usize) -> Option<&'static [[Evolution; 2]; EVOLUTION_LEVELS.len()]> {
    EVOLUTIONS.get(kind).map(|(_, tiers)| tiers)
}

//which of the two ways was chosen at a level of evolution, if one was. `evolutions` is two bits for each of EVOLUTION_LEVELS, from the
//lowest: 0 for none chosen yet, 1 for the first way and 2 for the second
pub(crate) fn chosen_at(evolutions: u8, tier: usize) -> Option<usize> {
    match (evolutions >> (tier * 2)) & 3 {
        pick @ (1 | 2) => Some(pick as usize - 1),
        _ => None,
    }
}

//what the evolutions a champion of a kind (plus one, as a unit has it) has chosen add up to
pub(crate) fn growth_of(champion: u8, evolutions: u8) -> Growth {
    let Some(tiers) = (champion as usize).checked_sub(1).and_then(evolutions_of) else {
        return Growth::NONE;
    };
    let mut growth = Growth::NONE;
    for (tier, pair) in tiers.iter().enumerate() {
        if let Some(chosen) = chosen_at(evolutions, tier) {
            growth = pair[chosen].changes.iter().fold(growth, |growth, &change| growth.with(change));
        }
    }
    growth
}

//the evolutions a champion of a level can have, from what a file or a message says: a choice that is neither of the two, or at a level it has
//not reached, is dropped
pub(crate) fn tidy_evolutions(evolutions: u8, level: u8) -> u8 {
    EVOLUTION_LEVELS.iter().enumerate().fold(0u8, |kept, (tier, &at)| match chosen_at(evolutions, tier) {
        Some(chosen) if level >= at => kept | (chosen as u8 + 1) << (tier * 2),
        _ => kept,
    })
}

impl Unit {
    //which champion this unit is, if it is one
    pub(crate) fn champion_kind(&self) -> Option<&'static Champion> {
        (self.champion as usize).checked_sub(1).and_then(|kind| CHAMPIONS.get(kind))
    }

    //how big it is: a champion is two to three times a unit, and a little bigger for every level, up to MAX_CHAMPION_SIZE
    pub(crate) fn radius(&self) -> f32 {
        match self.champion_kind() {
            Some(kind) => RADIUS * (kind.size * (1.0 + CHAMPION_LEVEL_SIZE * self.level.saturating_sub(1) as f32)).min(MAX_CHAMPION_SIZE),
            None => RADIUS,
        }
    }

    //the most health it can have: an ordinary unit's, or a champion's, which a level and its evolutions add to
    pub(crate) fn max_health(&self) -> f32 {
        if self.champion == 0 { UNIT_HEALTH } else { champion_health(self.level) * self.growth().health }
    }

    //what its evolutions add to it
    pub(crate) fn growth(&self) -> Growth {
        growth_of(self.champion, self.evolutions)
    }

    //its power as it is now: its kind's, at its level and grown by its evolutions
    pub(crate) fn champion_power(&self) -> Option<Power> {
        self.champion_kind().map(|kind| kind.power.grown(self.level, &self.growth()))
    }

    //the first level of evolution it has reached that its owner has not chosen at
    pub(crate) fn evolution_due(&self) -> Option<usize> {
        if self.champion == 0 {
            return None;
        }
        (0..EVOLUTION_LEVELS.len()).find(|&tier| self.level >= EVOLUTION_LEVELS[tier] && chosen_at(self.evolutions, tier).is_none())
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

//a champion grows the way its owner chose at a level of evolution it has reached and not chosen at yet, is mended in full and flares. false,
//with nothing changed, for a choice it can't make
pub(crate) fn evolve(unit: &mut Unit, tier: usize, choice: usize, fx: &mut Effects) -> bool {
    let (Some(kind), Some(tiers)) = (unit.champion_kind(), (unit.champion as usize).checked_sub(1).and_then(evolutions_of)) else {
        return false;
    };
    let Some(&level) = EVOLUTION_LEVELS.get(tier) else {
        return false;
    };
    if choice >= 2 || unit.level < level || chosen_at(unit.evolutions, tier).is_some() {
        return false;
    }
    unit.evolutions |= (choice as u8 + 1) << (tier * 2);
    unit.health = unit.max_health();
    let at = Vec2::new(unit.x, unit.y);
    fx.ring(at, 260.0, kind.color);
    fx.ring(at, 160.0, TROPHY);
    fx.burst(at, kind.color, 40, 360.0);
    fx.popup(at, unit.radius() + 8.0, 0.0, format!("{}!", tiers[tier][choice].name.to_uppercase()), kind.color);
    fx.cue_at(Cue::Champion, at);
    true
}

//the champion of a player's (by its id) that waits for them to choose how it grows, and at which level of evolution: the first in the
//roster's order, if there are several
pub(crate) fn evolution_due_for(units: &[Unit], owner: usize) -> Option<(u32, usize)> {
    units
        .iter()
        .filter(|unit| owner_index(unit.owner) == owner)
        .filter_map(|unit| unit.evolution_due().map(|tier| (unit.champion, unit.id, tier)))
        .min()
        .map(|(_, id, tier)| (id, tier))
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

//the damage a champion's power does to an enemy it hits: `seconds` worth of what the whole army (`army` units, with the player's relics
//on their touch, as `Upgrades::hit` has it) does by touch in a second, so a champion keeps up with the army it fights for however big that
//grows. an army smaller than CHAMPION_MIN_ARMY counts as that big, and a boss takes only CHAMPION_BOSS_SHARE of it
fn power_damage(army: usize, upgrades: &Upgrades, enemy: &Enemy, seconds: f32) -> f32 {
    army_touch(army, upgrades) * seconds * boss_share(enemy)
}

//what the whole of an army of `army` units does by touch in a second, as a champion's power counts it: no fewer than CHAMPION_MIN_ARMY, with the
//player's relics on their touch
pub(crate) fn army_touch(army: usize, upgrades: &Upgrades) -> f32 {
    army.max(CHAMPION_MIN_ARMY) as f32 * UNIT_DAMAGE * upgrades.hit()
}

//the share of a champion's power an enemy takes: all of it, or CHAMPION_BOSS_SHARE for a boss
pub(crate) fn boss_share(enemy: &Enemy) -> f32 {
    if enemy.boss.is_some() { CHAMPION_BOSS_SHARE } else { 1.0 }
}

//hurt an enemy with a power of a player's champion, and keep count so a number floats up from it. the player has the credit for it
pub(crate) fn strike(enemy: &mut Enemy, damage: f32, owner: u8) {
    //a shocked one takes the next hit the harder, and the shock is spent
    let damage = if enemy.status.shocked > 0.0 {
        enemy.status.shocked = 0.0;
        damage * SHOCK_BONUS
    } else {
        damage
    };
    let dealt = enemy.take_damage(damage);
    enemy.hurt += dealt;
    enemy.credit = owner;
    enemy.struck = HIT_FLASH;
    //a frozen one is brittle: a big enough hit breaks it, and it shatters (see resolve_deaths_among)
    if enemy.status.frozen > 0.0 && dealt >= SHATTER_HIT * enemy.max_health {
        enemy.health = 0.0;
    }
}

pub(crate) fn in_reach(enemy: &Enemy, from: Vec2, reach: f32) -> bool {
    enemy.health > 0.0 && !enemy.lurking() && enemy.position.distance(from) <= reach + enemy.size / 2.0
}

//a streak of light between two points
fn streak(fx: &mut Effects, from: Vec2, to: Vec2, color: Color) {
    fx.beams.push(Beam { from, to, color, age: 0.0 });
}

//a champion's place in the lists of how its volleys grow (SNIPE_ROUNDS and the rest), which go level by level
fn level_step(level: u8) -> usize {
    (level.clamp(1, CHAMPION_MAX_LEVEL) - 1) as usize
}

//a ring of a stormcaller's sparks, which turn towards the enemies near from SPARK_HOMING_LEVEL on
fn spark_volley(from: Vec2, aim: Vec2, count: u8, step: usize, champion: u8) -> Volley {
    let homing = if step + 1 >= SPARK_HOMING_LEVEL as usize { SPARK_HOMING } else { 0.0 };
    Volley { from, aim, kind: BoltKind::Spark, shape: VolleyShape::Ring, count, scatter: 0.0, speed: SPARK_SPEED, reach: SPARK_REACH, pierce: 0, bounce: 0, homing, splash: 0.0, embers: 0, champion, seed: volley_seed() }
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
        let grown = unit.growth();
        let power = kind.power.grown(unit.level, &grown);
        let most = unit.max_health();
        //its might is one unit's and then what it has over that, which grows with the army, with its level and with its evolutions
        let mut might = (1.0 + (kind.might - 1.0) * (1.0 + growth)) * (1.0 + CHAMPION_LEVEL_MIGHT * steps as f32) * grown.might;
        if let Power::Rage { extra } = power {
            might *= 1.0 + extra * (1.0 - unit.health / most).clamp(0.0, 1.0);
        }
        unit.might = might;
        unit.ward = (kind.ward * CHAMPION_LEVEL_WARD.powi(steps) * grown.ward).max(MIN_WARD);
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
    //the powers of a champion are the relics of the player it fights for, and a share of what that player's army does
    let sizes = army_sizes(units);
    for (at, kind, level, evolutions, owner) in std::mem::take(&mut fx.rebirths) {
        let upgrades = relics.of(owner);
        let army = sizes[owner_index(owner)];
        let Some(champion) = (kind as usize).checked_sub(1).and_then(|kind| CHAMPIONS.get(kind)) else {
            continue;
        };
        let Power::Rebirth { radius, burst, .. } = champion.power.grown(level, &growth_of(kind, evolutions)) else {
            continue;
        };
        for enemy in enemies.iter_mut().filter(|enemy| in_reach(enemy, at, radius)) {
            let damage = power_damage(army, upgrades, enemy, burst);
            strike(enemy, damage, owner);
        }
        fx.ring(at, radius, champion.color);
        fx.burst(at, champion.color, 60, 420.0);
        fx.mark(at, radius * 0.6, champion.color, MarkKind::Scorch);
        fx.add_shake(0.3);
        fx.cue_at(Cue::Explosion, at);
    }
    //(a muster is held to the size of that player's army)
    for unit in units.iter_mut().filter(|unit| unit.champion != 0 && unit.garrison.is_none()) {
        let Some(kind) = unit.champion_kind() else {
            continue;
        };
        let at = Vec2::new(unit.x, unit.y);
        let owner = unit.owner;
        let upgrades = relics.of(owner);
        let (army, cap) = (sizes[owner_index(owner)], upgrades.unit_cap());
        let mut timer = unit.power;
        match kind.power.grown(unit.level, &unit.growth()) {
            Power::Fireball { every, range, radius, weight } => {
                timer -= dt;
                if timer <= 0.0 {
                    timer = match fireball_target(enemies, at, range, radius) {
                        Some(target) => {
                            //shells lobbed at the thickest crowd, which go off where they land (see `fly_bolts`): more of them as it rises, and
                            //from the middle levels each throws out a ring of embers. they share what one fireball did, and the embers EMBER_SHARE more
                            let step = level_step(unit.level);
                            let (shells, embers) = (FIREBALL_SHELLS[step], FIREBALL_EMBERS[step]);
                            let total = army_touch(army, upgrades) * weight * every;
                            let volley = Volley {
                                from: at,
                                aim: target - at,
                                kind: BoltKind::Shell,
                                shape: VolleyShape::Burst,
                                count: shells,
                                scatter: radius * 1.2,
                                speed: SHELL_SPEED,
                                reach: at.distance(target),
                                pierce: 0,
                                bounce: 0,
                                homing: 0.0,
                                splash: radius,
                                embers,
                                champion: unit.champion,
                                seed: volley_seed(),
                            };
                            let hurt = Hurt { each: total / shells as f32, ember: total * EMBER_SHARE / (shells as f32 * embers.max(1) as f32), owner };
                            fx.volleys.push((volley, hurt));
                            every
                        }
                        None => 0.25, //nothing to burn, look again soon
                    };
                }
            }
            Power::Snipe { every, range, weight } => {
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
                            //a burst of rounds at it, longer as it rises, which from the middle levels go through the enemies they hit and then
                            //glance off to the next. the rounds share what one shot did
                            let step = level_step(unit.level);
                            let rounds = SNIPE_ROUNDS[step];
                            let volley = Volley {
                                from: at,
                                aim: enemies[i].position - at,
                                kind: BoltKind::Round,
                                shape: VolleyShape::Burst,
                                count: rounds,
                                scatter: ROUND_SCATTER,
                                speed: ROUND_SPEED,
                                reach: range * 1.2,
                                pierce: SNIPE_PIERCE[step],
                                bounce: SNIPE_BOUNCE[step],
                                homing: 0.0,
                                splash: 0.0,
                                embers: 0,
                                champion: unit.champion,
                                seed: volley_seed(),
                            };
                            fx.volleys.push((volley, Hurt { each: army_touch(army, upgrades) * weight * every / rounds as f32, ember: 0.0, owner }));
                            fx.cue_at(Cue::Arrow, at);
                            every
                        }
                        None => 0.2,
                    };
                }
            }
            Power::Lightning { every, range, jumps, weight } => {
                let before = timer;
                timer -= dt;
                let step = level_step(unit.level);
                let touch = army_touch(army, upgrades) * weight;
                //at the top levels a spiral of sparks turns round it all the while there are enemies near (its arms go out as the timer
                //passes each SPIRAL_EVERY, at an angle the timer turns)
                let arms = SPIRAL_ARMS[step];
                if arms > 0 && (before / SPIRAL_EVERY).floor() != (timer / SPIRAL_EVERY).floor() && enemies.iter().any(|enemy| in_reach(enemy, at, SPARK_REACH)) {
                    let aim = Vec2::from_angle(timer * SPIRAL_TURN + (unit.id % 64) as f32);
                    fx.volleys.push((spark_volley(at, aim, arms, step, unit.champion), Hurt { each: touch * SPIRAL_SHARE * SPIRAL_EVERY / arms as f32, ember: 0.0, owner }));
                }
                if timer <= 0.0 {
                    let chain = lightning_chain(enemies, at, range, jumps);
                    timer = if chain.is_empty() {
                        0.25
                    } else {
                        let mut from = at;
                        for &i in &chain {
                            let damage = power_damage(army, upgrades, &enemies[i], weight * every);
                            strike(&mut enemies[i], damage, owner);
                            streak(fx, from, enemies[i].position, kind.color);
                            fx.burst(enemies[i].position, kind.color, 10, 220.0);
                            from = enemies[i].position;
                        }
                        //what it went through is left shocked: the next power to hit it does more
                        for &i in &chain {
                            shock(&mut enemies[i], SHOCK_SECONDS);
                        }
                        //and from the middle levels a ring of sparks goes out with each bolt
                        let ring = SPARK_RING[step];
                        if ring > 0 {
                            let volley = spark_volley(at, enemies[chain[0]].position - at, ring, step, unit.champion);
                            fx.volleys.push((volley, Hurt { each: touch * every * SPARK_SHARE / ring as f32, ember: 0.0, owner }));
                        }
                        fx.add_shake(0.05);
                        fx.cue_at(Cue::Zap, at);
                        every
                    };
                }
            }
            Power::Quake { every, radius, weight } => {
                timer -= dt;
                if timer <= 0.0 {
                    timer = if enemies.iter().any(|enemy| in_reach(enemy, at, radius)) {
                        for enemy in enemies.iter_mut().filter(|enemy| in_reach(enemy, at, radius)) {
                            let damage = power_damage(army, upgrades, enemy, weight * every);
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
                let pulse = timer <= 0.0;
                if pulse {
                    timer = every;
                    fx.ring(at, radius, kind.color);
                    fx.cue_at(Cue::Frost, at);
                }
                //its aura slows and chills what is in it, and its pulse chills it all at once, which freezes (for `freeze` seconds) what was chilled
                //enough; a boss is only slowed (see status.rs)
                for enemy in enemies.iter_mut().filter(|enemy| in_reach(enemy, at, radius)) {
                    enemy.status.slow = enemy.status.slow.min(slow);
                    chill(enemy, FROST_CHILL * dt + if pulse { FROST_PULSE_CHILL } else { 0.0 }, freeze);
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
            //the best few of a kind each add their share, at their level and with their evolutions, the rest nothing
            let mut best: Vec<&Unit> = champions.iter().copied().filter(|unit| unit.champion as usize == kind + 1).collect();
            best.sort_by_key(|unit| Reverse((unit.level, unit.evolutions)));
            best.truncate(CHAMPION_STACK);
            let (mut speed, mut pickup, mut bounty, mut points) = (0.0, 0.0, 0.0, 0.0);
            for unit in best {
                match champion.power.grown(unit.level, &unit.growth()) {
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

//the reach of the champions' auras, each a faint circle in its colour: drawn under the army, so the units in it are not tinted by it.
//`zoom` keeps the line the same width on screen, `time` makes the circles pulse
pub(crate) fn draw_champion_auras(units: &[Unit], view: Rect, zoom: f32, time: f32) {
    let pixel = 1.0 / zoom;
    for unit in units.iter().filter(|unit| unit.champion != 0 && unit.garrison.is_none()) {
        let Some(kind) = unit.champion_kind() else {
            continue;
        };
        let Some(reach) = kind.power.grown(unit.level, &unit.growth()).aura() else {
            continue;
        };
        let (x, y) = (unit.x, unit.y);
        let seen = Rect::new(view.x - reach, view.y - reach, view.w + reach * 2.0, view.h + reach * 2.0);
        if !seen.contains(Vec2::new(x, y)) {
            continue;
        }
        let pulse = 0.5 + 0.5 * (time * 2.0 + x * 0.01).sin();
        draw_circle(x, y, reach, Color::new(kind.color.r, kind.color.g, kind.color.b, 0.05));
        draw_circle_lines(x, y, reach, 2.0 * pixel, Color::new(kind.color.r, kind.color.g, kind.color.b, 0.22 + 0.12 * pulse));
    }
}

//the drawing of a kind of champion (its place in CHAMPIONS), if it has one yet: the one called "Champion" and its name (see art/champions.py)
pub(crate) fn champion_shape(kind: usize) -> Option<usize> {
    static SHAPES: OnceLock<Vec<Option<usize>>> = OnceLock::new();
    SHAPES.get_or_init(|| CHAMPIONS.iter().map(|champion| shape_named(&format!("Champion {}", champion.name))).collect()).get(kind).copied().flatten()
}

//what a champion is drawn as at a zoom: its picture, unless its kind has none yet, the setting Simple shapes is on, or it is too small on the screen to make out
//(under CHAMPION_ART_PIXELS across), when it is its polygon
pub(crate) fn champion_art(unit: &Unit, zoom: f32, simple: bool) -> Option<usize> {
    if simple || unit.radius() * 2.0 * zoom < CHAMPION_ART_PIXELS {
        return None;
    }
    champion_shape((unit.champion as usize).checked_sub(1)?)
}

thread_local! {
    //which way each champion (by its id) is drawn facing, kept from frame to frame so that it turns rather than jumps
    static CHAMPION_HEADINGS: RefCell<HashMap<u32, Heading>> = RefCell::new(HashMap::new());
}

//its rank, under it: a gold diamond for every three levels above the first, and a chevron for each level over that
fn draw_rank(x: f32, y: f32, level: u8, pixel: f32) {
    let steps = level.saturating_sub(1);
    let (diamonds, chevrons) = (steps / 3, steps % 3);
    let size = pixel.max(0.6);
    let gap = 8.0 * size;
    let mut at = x - (diamonds + chevrons).saturating_sub(1) as f32 * gap / 2.0;
    for _ in 0..diamonds {
        draw_poly(at, y, 4, 4.6 * size, 0.0, BLACK);
        draw_poly(at, y, 4, 3.4 * size, 0.0, TROPHY);
        at += gap;
    }
    for _ in 0..chevrons {
        let (wide, high) = (3.6 * size, 2.4 * size);
        for (width, color) in [(3.6 * size, BLACK), (1.8 * size, TROPHY)] {
            draw_line(at - wide, y + high, at, y - high, width, color);
            draw_line(at, y - high, at + wide, y + high, width, color);
        }
        at += gap;
    }
}

//the champions in the world, over the army and everything else on the ground. one whose kind has a picture is drawn as it (see art/champions.py), turned to the
//nearest enemy near it or else the way it is going, and swelling and brightening a moment when its power goes off; one that has none yet, or is too small to make out,
//or all of them with `simple` (the setting Simple shapes), is a polygon of its kind with its two letters in the middle. under each is the ring of its grade, under it
//its rank, and over it a bar when it is hurt. `zoom` keeps the lines and the letters the same size on screen; `camera` is the one the world is drawn with. the
//auras are drawn apart, under the army (see `draw_champion_auras`)
pub(crate) fn draw_champions(units: &[Unit], enemies: &[Enemy], view: Rect, zoom: f32, camera: &Camera2D, simple: bool) {
    let pixel = 1.0 / zoom;
    let dt = get_frame_time().min(0.1);
    let shown: Vec<&Unit> = units
        .iter()
        .filter(|unit| unit.champion != 0 && unit.garrison.is_none() && unit.champion_kind().is_some())
        .filter(|unit| {
            let reach = unit.radius() * PICTURE;
            Rect::new(view.x - reach, view.y - reach, view.w + reach * 2.0, view.h + reach * 2.0).contains(Vec2::new(unit.x, unit.y))
        })
        .collect();
    //under each, the ring of its grade, and a ring going out from one that has just arrived
    for unit in &shown {
        let Some(kind) = unit.champion_kind() else {
            continue;
        };
        let (x, y, radius) = (unit.x, unit.y, unit.radius());
        let grade = kind.grade.color();
        draw_circle_lines(x, y, radius + 5.0, 2.5 * pixel.max(0.6), Color::new(grade.r, grade.g, grade.b, 0.85));
        if unit.age < 1.5 {
            let t = unit.age / 1.5;
            draw_circle_lines(x, y, radius + 90.0 * t, 3.0 * pixel, Color::new(kind.color.r, kind.color.g, kind.color.b, 1.0 - t));
        }
    }
    let mut creatures = Vec::new();
    CHAMPION_HEADINGS.with_borrow_mut(|headings| {
        let mut kept = HashMap::with_capacity(shown.len());
        for unit in &shown {
            let Some(shape) = champion_art(unit, zoom, simple) else {
                continue;
            };
            let at = Vec2::new(unit.x, unit.y);
            let want = enemies
                .iter()
                .filter(|enemy| enemy.health > 0.0 && !enemy.lurking())
                .map(|enemy| enemy.position - at)
                .filter(|way| way.length() < CHAMPION_LOOK)
                .min_by(|a, b| a.length_squared().total_cmp(&b.length_squared()));
            let mut heading = headings.remove(&unit.id).unwrap_or_else(|| Heading::new(at, unit.id, want));
            let facing = heading.turn(at, want, CHAMPION_TURN, dt);
            kept.insert(unit.id, heading);
            let act = power_cycle(unit).filter(|cycle| cycle.period >= 1.0).map_or(0.0, |cycle| (1.0 - cycle.since / ROSTER_PULSE).max(0.0));
            let color = lerp_color(unit_color(unit), WHITE, 0.35 * act);
            creatures.push(Creature::new(shape, at, unit.radius() * 2.0 * (1.0 + CHAMPION_ACT_SWELL * act), facing, color).seeded(unit.id));
        }
        *headings = kept;
    });
    draw_creatures(&creatures, camera);
    for unit in shown.iter().filter(|unit| champion_art(unit, zoom, simple).is_none()) {
        let Some(kind) = unit.champion_kind() else {
            continue;
        };
        let (x, y, radius) = (unit.x, unit.y, unit.radius());
        let body = unit_color(unit);
        draw_poly(x, y, kind.sides, radius + UNIT_BORDER, kind.turn, BLACK);
        draw_poly(x, y, kind.sides, radius, kind.turn, body);
        draw_poly(x, y, kind.sides, radius * 0.58, kind.turn, lerp_color(body, WHITE, 0.5));
        let size = (radius * 0.5).clamp(15.0, 28.0); //the letters grow with the body
        let width = text_width(kind.tag, size, Face::Header) * pixel;
        draw_label_scaled(kind.tag, x - width / 2.0, y + size * 0.36 * pixel, size, pixel, Color::new(0.04, 0.06, 0.1, 1.0), Face::Header);
    }
    for unit in &shown {
        let (x, y, radius) = (unit.x, unit.y, unit.radius());
        draw_rank(x, y + radius + 10.0 * pixel.max(0.6), unit.level, pixel);
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
pub(crate) const ROSTER_ROW: f32 = 32.0; //a row at full size, with the health and the way to the next level under the name...
pub(crate) const ROSTER_ROW_COMPACT: f32 = 22.0; //...and squeezed, with only a thin line of health, when the rows would not all fit above the minimap
pub(crate) const ROSTER_GAP: f32 = 4.0;
pub(crate) const ROSTER_TOP: f32 = 34.0; //room for the heading over the first row
pub(crate) const ROSTER_PULSE: f32 = 0.45; //seconds a row glows after its champion's power goes off
pub(crate) const PING_SECONDS: f32 = 1.6; //how long a champion picked out from the roster is marked in the world
pub(crate) const ROSTER_CLEAR: f32 = 8.0; //pixels between the roster and the list of players above it, and the minimap below

//how many champions the roster had in it when it was last drawn, which the arrows at the edge of the screen need to know to keep off it
static ROSTERED: AtomicUsize = AtomicUsize::new(0);

//where the roster starts down the screen: under the list of players when there is one (`panel`, see `team_panel`), else at the top.
//the heading goes in the first ROSTER_TOP pixels of it
pub(crate) fn roster_origin(panel: Option<Rect>) -> f32 {
    panel.map_or(12.0, |panel| panel.bottom() + ROSTER_CLEAR)
}

//where each champion's row is, down the right edge of the screen: under the list of players if there is one, and above the minimap.
//the rows are at full size when they fit, squeezed when they don't, and in a second column to the left when even that is not enough
pub(crate) fn roster_rows(count: usize, screen: Vec2, panel: Option<Rect>) -> Vec<Rect> {
    let top = roster_origin(panel) + ROSTER_TOP;
    let floor = Minimap::new(screen, MapView::Near, Vec2::ZERO).rect.y - ROSTER_CLEAR;
    let room = (floor - top).max(0.0);
    let right = screen.x - 12.0 - ROSTER_WIDTH;
    let fits = |row: f32, gap: f32| count as f32 * (row + gap) - gap <= room;
    let (row, gap) = if fits(ROSTER_ROW, ROSTER_GAP) { (ROSTER_ROW, ROSTER_GAP) } else { (ROSTER_ROW_COMPACT, 2.0) };
    let per_column = if fits(row, gap) { count.max(1) } else { (((room + gap) / (row + gap)).floor() as usize).max(1) };
    (0..count)
        .map(|i| {
            let (column, place) = (i / per_column, i % per_column);
            Rect::new(right - column as f32 * (ROSTER_WIDTH + ROSTER_CLEAR), top + (row + gap) * place as f32, ROSTER_WIDTH, row)
        })
        .collect()
}

//the roster as it was last drawn, if there is one on the screen, as one rectangle from its heading to its last row: what the arrows at the
//edge of the screen keep off (see `edge_spot`)
pub(crate) fn roster_panel(screen: Vec2) -> Option<Rect> {
    let count = ROSTERED.load(Ordering::Relaxed);
    if count == 0 {
        return None;
    }
    let panel = team_panel(screen);
    let rows = roster_rows(count, screen, panel);
    let left = rows.iter().map(|row| row.x).fold(f32::MAX, f32::min);
    let bottom = rows.iter().map(|row| row.bottom()).fold(0.0, f32::max);
    let top = roster_origin(panel);
    Some(Rect::new(left, top, screen.x - 12.0 - left, bottom - top))
}

//the champions, by kind and then in the order they joined, which is what the roster lists
pub(crate) fn roster_of(units: &[Unit]) -> Vec<&Unit> {
    let mut roster: Vec<&Unit> = units.iter().filter(|unit| unit.champion != 0).collect();
    roster.sort_by_key(|unit| unit.champion);
    roster
}

//a champion's picture as an icon on the screen (in the roster, on a card), `radius` from its middle to the edge of the picture: the polygon of its kind when it has
//no picture, or with `simple` (the setting Simple shapes)
pub(crate) fn draw_champion_icon(kind: usize, at: Vec2, radius: f32, simple: bool) {
    let Some(champion) = CHAMPIONS.get(kind) else {
        return;
    };
    match champion_shape(kind).filter(|_| !simple) {
        Some(shape) => {
            let screen = follow_camera(screen_width() / 2.0, screen_height() / 2.0, 1.0);
            draw_creatures(&[Creature::new(shape, at, radius * 2.0 / PICTURE, Vec2::new(0.0, -1.0), champion.color)], &screen);
        }
        None => draw_icon(champion, at, radius * 0.65),
    }
}

//a champion's polygon as a small icon, centred on a point
fn draw_icon(kind: &Champion, at: Vec2, radius: f32) {
    draw_poly(at.x, at.y, kind.sides, radius + 1.5, kind.turn, BLACK);
    draw_poly(at.x, at.y, kind.sides, radius, kind.turn, kind.color);
}

//the champions along the right edge of the screen, each with its health, and what the one under the mouse is and does
pub(crate) fn draw_roster(units: &[Unit], mouse: Vec2, ping: Option<u32>, simple: bool) {
    let roster = roster_of(units);
    ROSTERED.store(roster.len(), Ordering::Relaxed);
    if roster.is_empty() {
        return;
    }
    let screen = Vec2::new(screen_width(), screen_height());
    let panel = team_panel(screen);
    let rows = roster_rows(roster.len(), screen, panel);
    draw_label_right(&format!("CHAMPIONS {}/{}", roster.len(), MAX_CHAMPIONS), screen.x - 14.0, roster_origin(panel) + ROSTER_TOP - 2.0, 15.0, CYAN, Face::Bold);
    let mut hovered = None;
    for (unit, row) in roster.iter().zip(&rows) {
        let Some(kind) = unit.champion_kind() else {
            continue;
        };
        let squeezed = row.h < ROSTER_ROW;
        let cycle = power_cycle(unit);
        //the row glows for a moment when the champion's power goes off (not for one that goes off all the time), and its edge is lit while the
        //mouse is on it or the champion is picked out in the world
        let glow = cycle.filter(|cycle| cycle.period >= 1.0).map_or(0.0, |cycle| (1.0 - cycle.since / ROSTER_PULSE).max(0.0));
        draw_rectangle(row.x, row.y, row.w, row.h, PANEL);
        if glow > 0.0 {
            draw_rectangle(row.x, row.y, row.w, row.h, Color::new(kind.color.r, kind.color.g, kind.color.b, 0.22 * glow));
        }
        draw_rectangle(row.x, row.y, 4.0, row.h, kind.color);
        let grade = kind.grade.color();
        let edge = if ping == Some(unit.id) {
            Color::new(TROPHY.r, TROPHY.g, TROPHY.b, 0.6 + 0.4 * (get_time() as f32 * 12.0).sin())
        } else if row.contains(mouse) {
            CYAN
        } else {
            Color::new(grade.r, grade.g, grade.b, 0.6)
        };
        draw_rectangle_lines(row.x, row.y, row.w, row.h, if edge == CYAN { 2.0 } else { 1.5 }, edge);
        let icon = Vec2::new(row.x + 22.0, row.y + row.h / 2.0);
        let size = if squeezed { 7.0 } else { 9.0 };
        //(a full row has the champion's picture, a squeezed one its polygon, which is all that can be made out that small)
        if squeezed {
            draw_icon(kind, icon, size);
        } else {
            draw_champion_icon(unit.champion as usize - 1, icon, ROSTER_ICON, simple);
        }
        //a ring round the icon fills as its power comes round again, and is whole and bright when it is ready
        if let Some(cycle) = cycle {
            let ring = size + 3.5;
            draw_circle_lines(icon.x, icon.y, ring, 1.5, Color::new(1.0, 1.0, 1.0, 0.14));
            let lit = if cycle.ready >= 1.0 { kind.color } else { Color::new(kind.color.r, kind.color.g, kind.color.b, 0.7) };
            draw_arc(icon.x, icon.y, 40, ring - 1.25, -90.0, 2.5, cycle.ready.clamp(0.0, 1.0) * 360.0, lit);
            if glow > 0.0 {
                draw_circle_lines(icon.x, icon.y, ring + 7.0 * (1.0 - glow), 2.0, Color::new(kind.color.r, kind.color.g, kind.color.b, glow));
            }
        }
        draw_label(kind.name, row.x + 38.0, row.y + 15.0, if squeezed { 14.0 } else { 16.0 }, INK, Face::Bold);
        draw_label_right(&format!("Lv {}", unit.level), row.x + row.w - 8.0, row.y + 15.0, if squeezed { 13.0 } else { 14.0 }, if unit.level >= CHAMPION_MAX_LEVEL { TROPHY } else { MUTED }, Face::Bold);
        let share = (unit.health / unit.max_health()).clamp(0.0, 1.0);
        let health = lerp_color(wound_color(), pickup_color(), share);
        if squeezed {
            //only a thin line of health, there is no room for more
            draw_meter(row.x + 38.0, row.y + row.h - 5.0, row.w - 48.0, 3.0, share, health);
        } else {
            draw_meter(row.x + 38.0, row.y + 20.0, row.w - 48.0, 5.0, share, health);
            //how far it is to the next level, a thin line under its health
            if unit.level < CHAMPION_MAX_LEVEL {
                let learned = (unit.xp / champion_xp_needed(unit.level)).clamp(0.0, 1.0);
                draw_rectangle(row.x + 38.0, row.y + 27.0, (row.w - 48.0) * learned, 2.0, TROPHY);
            }
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
        //what its power does now, with its level and its evolutions
        if let Some(numbers) = unit.champion_power().as_ref().and_then(power_numbers) {
            lines.push((format!("now: {numbers}"), CYAN));
        }
        if unit.level > 1 {
            let more = (CHAMPION_LEVEL_POWER * (unit.level - 1) as f32 * 100.0).round();
            lines.push((format!("level {}: its power is {more:.0}% stronger and it goes off {:.0}% faster", unit.level, CHAMPION_LEVEL_PACE * (unit.level - 1) as f32 * 100.0), TROPHY));
        }
        //the ways it has grown, and what each does
        if let Some(tiers) = (unit.champion as usize).checked_sub(1).and_then(evolutions_of) {
            for (tier, pair) in tiers.iter().enumerate() {
                if let Some(chosen) = chosen_at(unit.evolutions, tier) {
                    let does: Vec<String> = pair[chosen].changes.iter().map(|&change| change_text(&kind.power, change)).collect();
                    lines.push((format!("{}: {}", pair[chosen].name, does.join(", ")), kind.color));
                }
            }
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
        lines.push(("click to find it in the world".to_owned(), MUTED));
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

//what a change does, in words, for a champion with this power
pub(crate) fn change_text(power: &Power, change: Change) -> String {
    let more = |times: f32| ((times - 1.0) * 100.0).round();
    match change {
        Change::Strength(times) => {
            let p = more(times);
            match power {
                Power::Brute => format!("{p:.0}% stronger"),
                Power::Rage { .. } => format!("its rage is {p:.0}% fiercer"),
                Power::Banner { .. } => format!("its banner's boost is {p:.0}% stronger"),
                Power::Bulwark { .. } => format!("its shield is {p:.0}% stronger"),
                Power::Mend { .. } => format!("it mends {p:.0}% more"),
                Power::Fireball { .. } => format!("its fireballs do {p:.0}% more damage"),
                Power::Snipe { .. } => format!("its shots do {p:.0}% more damage"),
                Power::Lightning { .. } => format!("its bolts do {p:.0}% more damage"),
                Power::Frost { .. } => format!("it slows enemies more and freezes them {p:.0}% longer"),
                Power::Quake { .. } => format!("its quakes do {p:.0}% more damage"),
                Power::Muster { .. } => format!("it calls {p:.0}% more recruits"),
                Power::Rebirth { .. } => format!("its blaze does {p:.0}% more damage"),
                Power::Scout { .. } => format!("what it adds to speed and pickup is {p:.0}% more"),
                Power::Quartermaster { .. } => format!("{p:.0}% more wall points and babies"),
            }
        }
        Change::Pace(times) => match power {
            Power::Rebirth { .. } => format!("it can rise again {:.0}% sooner", ((1.0 - 1.0 / times) * 100.0).round()),
            Power::Muster { .. } => format!("it calls recruits {:.0}% more often", more(times)),
            Power::Frost { .. } => format!("it freezes {:.0}% more often", more(times)),
            Power::Snipe { .. } => format!("it shoots {:.0}% more often", more(times)),
            _ => format!("its power goes off {:.0}% more often", more(times)),
        },
        Change::Reach(times) => match power {
            Power::Fireball { .. } => format!("it throws {0:.0}% further, with a {0:.0}% wider blast", more(times)),
            Power::Quake { .. } | Power::Rebirth { .. } => format!("its blast is {:.0}% wider", more(times)),
            _ if power.aura().is_some() => format!("its aura reaches {:.0}% further", more(times)),
            _ => format!("it reaches {:.0}% further", more(times)),
        },
        Change::Might(times) => format!("it fights {:.0}% harder by touch", more(times)),
        Change::Ward(times) => format!("it takes {:.0}% less damage", ((1.0 - times) * 100.0).round()),
        Change::Health(times) => format!("{:.0}% more health", more(times)),
        Change::Jumps(count) => format!("its bolts jump through {count} more enemies"),
    }
}

//how a champion is to grow, chosen over a darkened world as the relics are: two cards, each with its number, its name and what it changes.
//`ready` is once it has been up long enough to be chosen from (see RELIC_LOCKOUT)
pub(crate) fn draw_evolution_choice(unit: &Unit, tier: usize, ready: bool, simple: bool) {
    let (Some(kind), Some(tiers)) = (unit.champion_kind(), (unit.champion as usize).checked_sub(1).and_then(evolutions_of)) else {
        return;
    };
    let Some(pair) = tiers.get(tier) else {
        return;
    };
    let screen = Vec2::new(screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, screen.x, screen.y, Color::new(0.0, 0.01, 0.03, 0.78));
    let heading = format!("{} EVOLVES", kind.name.to_uppercase());
    let heading_size = fit_size(&heading, (screen.y * 0.09).round(), screen.x * 0.9, Face::Header);
    draw_heading(&heading, screen.x / 2.0, screen.y * 0.16, heading_size, kind.color);
    let line = (text_width(&heading, heading_size, Face::Header) * 0.5).min(screen.x * 0.3);
    draw_line(screen.x / 2.0 - line, screen.y * 0.185, screen.x / 2.0 + line, screen.y * 0.185, 3.0, CYAN);
    draw_label_centered(&format!("LEVEL {}: CHOOSE HOW IT GROWS, ONCE", EVOLUTION_LEVELS[tier]), screen.x / 2.0, screen.y * 0.235, (screen.y * 0.032).round(), CYAN, Face::Bold);
    let mouse = Vec2::from(mouse_position());
    for (i, card) in choice_cards(screen, 2).iter().enumerate() {
        let evolution = &pair[i];
        let hovered = ready && card.contains(mouse);
        let lift = if hovered { 8.0 } else { 0.0 };
        let (x, y) = (card.x, card.y - lift);
        if hovered {
            draw_rectangle(x - 6.0, y - 6.0, card.w + 12.0, card.h + 12.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.14));
        }
        draw_rectangle(x, y, card.w, card.h, Color::new(0.04, 0.06, 0.1, 0.97));
        draw_rectangle(x, y, card.w, 6.0, kind.color);
        draw_rectangle_lines(x, y, card.w, card.h, if hovered { 3.0 } else { 2.0 }, if hovered { CYAN } else { Color::new(kind.color.r, kind.color.g, kind.color.b, 0.7) });
        draw_key(&format!("{}", i + 1), x + 14.0, y + 38.0, 22.0);
        draw_label_right("FINAL FORM", x + card.w - 14.0, y + 36.0, 16.0, TROPHY, Face::Bold);
        draw_champion_icon(unit.champion as usize - 1, Vec2::new(x + card.w / 2.0, y + card.h * 0.19), 24.0, simple);
        let name_size = fit_size(evolution.name, (card.h * 0.1).min(30.0).round(), card.w - 24.0, Face::Header);
        draw_label_centered(evolution.name, x + card.w / 2.0, y + card.h * 0.33, name_size, kind.color, Face::Header);
        draw_line(x + card.w * 0.3, y + card.h * 0.38, x + card.w * 0.7, y + card.h * 0.38, 1.5, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.5));
        let size = (card.h * 0.062).clamp(15.0, 21.0).round();
        let mut row = 0;
        for change in evolution.changes {
            for line in crate::relics::wrap_text(&change_text(&kind.power, *change), size, card.w - 32.0) {
                draw_label_centered(&line, x + card.w / 2.0, y + card.h * 0.5 + size * 1.4 * row as f32, size, INK, Face::Body);
                row += 1;
            }
        }
    }
    if ready {
        let fade = 0.6 + 0.4 * (get_time() as f32 * 3.0).sin();
        draw_label_centered("press 1 or 2, or click one", screen.x / 2.0, screen.y * 0.9, (screen.y * 0.036).round(), Color::new(INK.r, INK.g, INK.b, fade), Face::Body);
    }
}

//where a champion's power is in its round: `ready` from 0, just gone off, to 1, ready again, and `since` the seconds since it went off, of a
//round of `period` seconds. a phoenix's round is the wait before it can rise again
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cycle {
    pub(crate) ready: f32,
    pub(crate) since: f32,
    pub(crate) period: f32,
}

//none for the champions whose power doesn't go off (the brutes and the auras). a timer longer than the round, as a level leaves it, is a
//power that has not just gone off
pub(crate) fn power_cycle(unit: &Unit) -> Option<Cycle> {
    let period = match unit.champion_power()? {
        Power::Fireball { every, .. } | Power::Snipe { every, .. } | Power::Lightning { every, .. } | Power::Frost { every, .. } | Power::Quake { every, .. } | Power::Muster { every, .. } => every,
        Power::Rebirth { recharge, .. } => recharge,
        _ => return None,
    };
    let left = unit.power.clamp(0.0, period);
    Some(Cycle { ready: 1.0 - left / period, since: if unit.power > period { period } else { period - left }, period })
}

//what a power does, in numbers, for the roster's tooltip. a share is of the damage the whole army does by touch, a second
pub(crate) fn power_numbers(power: &Power) -> Option<String> {
    let share = |weight: f32| (weight * 100.0).round();
    Some(match *power {
        Power::Brute => return None,
        Power::Rage { extra } => format!("fights up to {:.1} times as hard as it is hurt", 1.0 + extra),
        Power::Banner { radius, might } => format!("units within {radius:.0} hit {:.0}% harder", (might - 1.0) * 100.0),
        Power::Bulwark { radius, ward } => format!("units within {radius:.0} take {:.0}% less damage", (1.0 - ward) * 100.0),
        Power::Mend { radius, per_second } => format!("mends {per_second:.0} health a second to each unit within {radius:.0}"),
        Power::Fireball { every, range, radius, weight } => format!("a fireball every {every:.1}s, {:.0}% of the army's damage, {radius:.0} wide, reach {range:.0}", share(weight)),
        Power::Snipe { every, range, weight } => format!("{:.1} shots a second, {:.0}% of the army's damage, reach {range:.0}", 1.0 / every, share(weight)),
        Power::Lightning { every, range, jumps, weight } => format!("a bolt every {every:.1}s through {jumps} enemies, {:.0}% of the army's damage to each, reach {range:.0}", share(weight)),
        Power::Frost { radius, slow, every, freeze } => format!("enemies within {radius:.0} keep {:.0}% of their speed, frozen {freeze:.1}s every {every:.1}s", slow * 100.0),
        Power::Quake { every, radius, weight } => format!("a quake every {every:.1}s, {:.0}% of the army's damage, {radius:.0} wide", share(weight)),
        Power::Muster { every, count } => format!("{count} recruits every {every:.1}s"),
        Power::Rebirth { recharge, radius, burst } => format!("rises in a blaze worth {burst:.1}s of the army's damage, {radius:.0} wide, once every {recharge:.0}s"),
        Power::Scout { speed, pickup } => format!("the army is {:.0}% faster and picks babies up from {:.0}% further", (speed - 1.0) * 100.0, (pickup - 1.0) * 100.0),
        Power::Quartermaster { points, bounty } => format!("{points:.1} wall points a second, and enemies drop {:.0}% more babies", (bounty - 1.0) * 100.0),
    })
}

//the champion (by its id) of a player's whose row in their roster is under a point on the screen, if any. `panel` is the list of players, which
//the roster goes under (see `team_panel`)
pub(crate) fn roster_row_at(units: &[Unit], owner: usize, screen: Vec2, panel: Option<Rect>, point: Vec2) -> Option<u32> {
    let mine: Vec<Unit> = units.iter().filter(|unit| unit.champion != 0 && owner_index(unit.owner) == owner).cloned().collect();
    let roster = roster_of(&mine);
    let rows = roster_rows(roster.len(), screen, panel);
    roster.iter().zip(&rows).find(|(_, row)| row.contains(point)).map(|(unit, _)| unit.id)
}

//a champion picked out from the roster, in the world: rings close in on it and a ring round it flashes, fading over PING_SECONDS. `age` is how
//long since it was picked out, `zoom` keeps the lines the same width on the screen
pub(crate) fn draw_champion_ping(units: &[Unit], id: u32, age: f32, zoom: f32) {
    let Some((unit, kind)) = units.iter().find(|unit| unit.id == id).and_then(|unit| unit.champion_kind().map(|kind| (unit, kind))) else {
        return;
    };
    let pixel = 1.0 / zoom;
    let fade = (1.0 - age / PING_SECONDS).clamp(0.0, 1.0);
    let radius = unit.radius();
    for k in 0..3 {
        let phase = (age * 1.6 + k as f32 / 3.0).fract();
        draw_circle_lines(unit.x, unit.y, radius * (1.3 + 3.5 * (1.0 - phase)), 3.0 * pixel, Color::new(kind.color.r, kind.color.g, kind.color.b, 0.9 * fade * phase));
    }
    let flash = 0.5 + 0.5 * (age * 14.0).sin();
    draw_circle_lines(unit.x, unit.y, radius * 1.2, 4.0 * pixel, Color::new(TROPHY.r, TROPHY.g, TROPHY.b, fade * flash));
}
