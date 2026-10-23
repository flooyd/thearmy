//! formations: where the army stands round the player. left alone the units pile up on the player in a heap, so deep that most
//! of them can't be seen and thousands of them are on top of each other. in a formation each has a place of its own in a shape
//! (a ring, a wedge, a line) that turns with the way the player is going, and it walks to that place and stays there, which is
//! easier to read, doesn't need the units shoving each other apart, and has them fighting where the shape meets the enemy

use crate::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Formation {
    Free,  //the units pile up round the player
    Ring,  //a ring round the player, with the player safe in the middle
    Wedge, //a spearhead, the player at its point
    Line,  //a wall across the way the player is going, the player behind it
}

pub(crate) const FORMATIONS: [Formation; 4] = [Formation::Free, Formation::Ring, Formation::Wedge, Formation::Line];

impl Formation {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Formation::Free => "Free",
            Formation::Ring => "Ring",
            Formation::Wedge => "Wedge",
            Formation::Line => "Line",
        }
    }

    pub(crate) fn blurb(self) -> &'static str {
        match self {
            Formation::Free => "no shape: the units pile up on the player",
            Formation::Ring => "a ring round the player, hit from every side at once",
            Formation::Wedge => "a spearhead in the way you are going, everything meets the point first",
            Formation::Line => "a wide wall in front of the player, to catch a lot of enemies at once",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Formation> {
        FORMATIONS.into_iter().find(|formation| formation.name() == name)
    }

    pub(crate) fn next(self) -> Formation {
        let at = FORMATIONS.iter().position(|&formation| formation == self).unwrap_or(0);
        FORMATIONS[(at + 1) % FORMATIONS.len()]
    }
}

//the two numbers between 0 and 1 a unit's slot stands for, which is where in the shape it stands (see `formation_offset`)
pub(crate) fn slot_coords(slot: u32) -> (f32, f32) {
    ((slot & 0xFFFF) as f32 / 65536.0, (slot >> 16) as f32 / 65536.0)
}

//the area a formation of this many units covers. every unit has room of its own up to FORMATION_ROOMY_UNITS, and past that the shape
//grows more slowly than the army: the enemies stop growing there, and one that has run at the player and is standing on them has to
//be inside the shape, or the units can do it no harm (they hurt what they overlap, and nothing else)
pub(crate) fn formation_area(army: usize) -> f32 {
    let army = army.max(1) as f32;
    let roomy = army.min(FORMATION_ROOMY_UNITS) * 0.87 * FORMATION_SPACING * FORMATION_SPACING;
    roomy * (army / FORMATION_ROOMY_UNITS).max(1.0).powf(FORMATION_PACKING)
}

//where in the formation the unit with this slot stands, as pixels from the player, for an army this big and the player facing
//a way. every unit has a place that is the same from one frame to the next however many others there are, so the shape
//grows and shrinks smoothly as the army does, and units that die leave gaps instead of the ones behind them shuffling up
pub(crate) fn formation_offset(formation: Formation, slot: u32, army: usize, facing: Vec2) -> Vec2 {
    let (a, b) = slot_coords(slot);
    let area = formation_area(army);
    let local = match formation {
        Formation::Free => Vec2::ZERO,
        Formation::Ring => {
            let hole = FORMATION_RING_HOLE;
            let outer = (area / (std::f32::consts::PI * (1.0 - hole * hole))).sqrt();
            let inner = (outer * hole).max(FORMATION_MIN_HOLE);
            let outer = (inner * inner + area / std::f32::consts::PI).sqrt();
            //(picking the distance by area spreads the units evenly over the ring)
            let radius = (inner * inner + a * (outer * outer - inner * inner)).sqrt();
            Vec2::from_angle(b * std::f32::consts::TAU) * radius
        }
        Formation::Wedge => {
            let length = (area / FORMATION_WEDGE_WIDTH).sqrt();
            let along = a.sqrt();
            Vec2::new(FORMATION_LEAD - length * along, (2.0 * b - 1.0) * along * length * FORMATION_WEDGE_WIDTH)
        }
        Formation::Line => {
            let depth = (area / FORMATION_LINE_RATIO).sqrt();
            Vec2::new(FORMATION_LEAD + depth * (1.0 - a), (2.0 * b - 1.0) * depth * FORMATION_LINE_RATIO / 2.0)
        }
    };
    facing * local.x + facing.perp() * local.y
}

//where a champion stands in any formation: close to the player, among the others like it, where the army shields it
pub(crate) fn guard_offset(slot: u32) -> Vec2 {
    let (a, b) = slot_coords(slot);
    Vec2::from_angle(b * std::f32::consts::TAU) * FORMATION_GUARD * a.sqrt()
}

//how the army is drilled for a frame: the shape, and the way it faces
#[derive(Clone, Copy, Debug)]
pub(crate) struct Drill {
    pub(crate) formation: Formation,
    pub(crate) facing: Vec2, //a unit vector
}

impl Drill {
    #[cfg(test)]
    pub(crate) const FREE: Drill = Drill { formation: Formation::Free, facing: Vec2::new(0.0, -1.0) };

    //the place a unit is to be at, if the army has a shape. a champion stays by the player
    pub(crate) fn goal(&self, unit: &Unit, player: Vec2, army: usize) -> Option<Vec2> {
        if self.formation == Formation::Free {
            return None;
        }
        if unit.champion != 0 {
            return Some(player + guard_offset(unit.slot));
        }
        Some(player + formation_offset(self.formation, unit.slot, army, self.facing))
    }
}

//turn the way the army faces towards the way the player is going, smoothly. a player standing still leaves it as it is
pub(crate) fn turn_to_face(facing: Vec2, velocity: Vec2, dt: f32) -> Vec2 {
    if velocity.length() < FORMATION_TURN_MIN_SPEED || dt <= 0.0 {
        return facing;
    }
    let tau = std::f32::consts::TAU;
    let now = facing.to_angle();
    let wanted = velocity.to_angle();
    let difference = (wanted - now + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI;
    Vec2::from_angle(now + difference * (1.0 - (-FORMATION_TURN * dt).exp()))
}

