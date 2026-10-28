//! the ground away from the meadow: patches of ice, lava and swamp laid over it (see `Patch`). each is round with a ragged edge, drawn by the ground's
//! shader (see ground.rs) and felt by whatever stands on it: the army slides on ice and the enemies on it are chilled, lava burns the units and sets the
//! enemies alight, and a swamp bogs down the player, the army and the enemies alike. the patches are laid out from the world's sites (`lay_patches`),
//! with a seed made of where the sites are, so a run loaded from a save, and a guest who has been told the sites, have the same ground without anyone
//! having to be told it

use crate::prelude::*;
use macroquad::rand::RandGenerator;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Biome {
    Ice,
    Lava,
    Swamp,
}

pub(crate) const BIOMES: [Biome; 3] = [Biome::Ice, Biome::Lava, Biome::Swamp];

impl Biome {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Biome::Ice => "Ice",
            Biome::Lava => "Lava",
            Biome::Swamp => "Swamp",
        }
    }

    //the number the ground's shader knows it by
    pub(crate) fn code(self) -> f32 {
        match self {
            Biome::Ice => 1.0,
            Biome::Lava => 2.0,
            Biome::Swamp => 3.0,
        }
    }

    //its colour on the minimap
    pub(crate) fn color(self) -> Color {
        match self {
            Biome::Ice => Color::new(0.62, 0.8, 0.95, 1.0),
            Biome::Lava => Color::new(1.0, 0.42, 0.1, 1.0),
            Biome::Swamp => Color::new(0.38, 0.52, 0.28, 1.0),
        }
    }

    //how many patches of it a world has
    fn count(self) -> usize {
        match self {
            Biome::Ice => ICE_PATCHES,
            Biome::Lava => LAVA_PATCHES,
            Biome::Swamp => SWAMP_PATCHES,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Patch {
    pub(crate) biome: Biome,
    pub(crate) position: Vec2,
    pub(crate) radius: f32,
}

impl Patch {
    //how far into it a place is, as a share of its radius: under 1 is on it. near the edge a noise moves it in or out by up to PATCH_WOBBLE of the
    //radius, so the edge is ragged; the shader works it out the same way (see ground.rs), so what is drawn is what is felt
    pub(crate) fn depth(&self, at: Vec2) -> f32 {
        let depth = at.distance(self.position) / self.radius;
        if (depth - 1.0).abs() > PATCH_WOBBLE {
            return depth;
        }
        depth + (edge_noise(at) - 0.5) * 2.0 * PATCH_WOBBLE
    }

    pub(crate) fn holds(&self, at: Vec2) -> bool {
        self.depth(at) < 1.0
    }

    //the furthest from its middle that is on it
    pub(crate) fn reach(&self) -> f32 {
        self.radius * (1.0 + PATCH_WOBBLE)
    }
}

//the ground at a place: the biome of the patch it is on, if any
pub(crate) fn biome_at(patches: &[Patch], at: Vec2) -> Option<Biome> {
    patches.iter().find(|patch| patch.holds(at)).map(|patch| patch.biome)
}

// ---------- the ragged edge, the same sums as the shader's ----------

fn fract(x: f32) -> f32 {
    x - x.floor()
}

//a number from 0 to 1 for a point, as `hash12` in the shader
fn hash(p: Vec2) -> f32 {
    let mut p3 = Vec3::new(fract(p.x * 0.1031), fract(p.y * 0.1031), fract(p.x * 0.1031));
    let d = p3.dot(Vec3::new(p3.y, p3.z, p3.x) + Vec3::splat(33.33));
    p3 += Vec3::splat(d);
    fract((p3.x + p3.y) * p3.z)
}

//smooth noise from 0 to 1, as `noise` in the shader
fn value_noise(p: Vec2) -> f32 {
    let i = p.floor();
    let f = p - i;
    let u = f * f * (Vec2::splat(3.0) - 2.0 * f);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    lerp(lerp(hash(i), hash(i + Vec2::X), u.x), lerp(hash(i + Vec2::Y), hash(i + Vec2::ONE), u.x), u.y)
}

//what moves the edge of a patch at a place, from 0 to 1, as `edgeNoise` in the shader
pub(crate) fn edge_noise(at: Vec2) -> f32 {
    let p = at * PATCH_EDGE_SCALE;
    0.65 * value_noise(p) + 0.35 * value_noise(p * 2.03 + Vec2::new(17.1, 9.3))
}

// ---------- where they are ----------

//the patches of a world with these sites: each kind's count, of sizes between PATCH_RADII, each clear of the start, the edge, the sites and the other
//patches (one that can't find room is left out). the seed is made of where the sites are, so the same sites always have the same patches
pub(crate) fn lay_patches(sites: &[Site]) -> Vec<Patch> {
    let dice = RandGenerator::new();
    dice.srand(sites.iter().fold(0x5EED_6A12_u64, |seed, site| (seed.rotate_left(13) ^ ((site.position.x.to_bits() as u64) << 32 | site.position.y.to_bits() as u64)).wrapping_mul(0x9E37_79B9_7F4A_7C15)));
    let centre = Vec2::splat(WORLD_SIZE / 2.0);
    let mut patches: Vec<Patch> = Vec::new();
    for biome in BIOMES {
        for _ in 0..biome.count() {
            for _ in 0..PATCH_TRIES {
                let radius = dice.gen_range(PATCH_RADII.0, PATCH_RADII.1);
                let patch = Patch { biome, position: Vec2::ZERO, radius };
                let edge = patch.reach() + PATCH_GAP;
                let position = Vec2::new(dice.gen_range(edge, WORLD_SIZE - edge), dice.gen_range(edge, WORLD_SIZE - edge));
                let clear = position.distance(centre) >= patch.reach() + SITE_START_CLEAR_TILES * TILE_SIZE
                    && sites.iter().all(|site| site.position.distance(position) >= patch.reach() + PATCH_SITE_GAP)
                    && patches.iter().all(|other| other.position.distance(position) >= patch.reach() + other.reach() + PATCH_GAP);
                if clear {
                    patches.push(Patch { position, ..patch });
                    break;
                }
            }
        }
    }
    patches
}

//the patches whose ground could be in `view`, nearest its middle first, no more than the shader takes
pub(crate) fn patches_in(patches: &[Patch], view: Rect) -> Vec<Patch> {
    let middle = view.center();
    let mut near: Vec<Patch> = patches
        .iter()
        .filter(|patch| {
            let closest = patch.position.clamp(view.point(), view.point() + view.size());
            closest.distance(patch.position) <= patch.reach()
        })
        .copied()
        .collect();
    near.sort_by(|a, b| a.position.distance_squared(middle).total_cmp(&b.position.distance_squared(middle)));
    near.truncate(MAX_PATCHES_DRAWN);
    near
}

// ---------- what they do ----------

//how fast the player walks at a place, as a share of their speed: slower in a swamp
pub(crate) fn ground_pace(patches: &[Patch], at: Vec2) -> f32 {
    if biome_at(patches, at) == Some(Biome::Swamp) { SWAMP_KEPT } else { 1.0 }
}

//what the ground does to the enemies standing on it, after the walls have had their say (see `terrain_effects`, which puts their speed back each
//frame): a swamp bogs them down, ice chills them (so one left on it long enough freezes), and lava sets them alight
pub(crate) fn ground_effects_on_enemies(enemies: &mut [Enemy], patches: &[Patch], dt: f32) {
    if patches.is_empty() || dt <= 0.0 {
        return;
    }
    for enemy in enemies.iter_mut().filter(|enemy| enemy.health > 0.0 && !enemy.lurking()) {
        match biome_at(patches, enemy.position) {
            Some(Biome::Swamp) => enemy.status.slow *= SWAMP_KEPT,
            Some(Biome::Ice) => chill(enemy, ICE_CHILL * dt, FREEZE_SECONDS),
            Some(Biome::Lava) => ignite(enemy, LAVA_BURN_SECONDS, enemy.max_health * LAVA_SHARE),
            None => {}
        }
    }
}

//where a unit on a patch stood and how it was moving at the start of a frame, before the armies walk
pub(crate) struct Footing {
    index: usize,
    biome: Biome,
    from: Vec2,
    velocity: Vec2,
}

//the units standing on a patch at the start of a frame (only the patches near a player are looked at, as the armies are round them). none in a
//tower, which stands above it
pub(crate) fn footing_of(units: &[Unit], patches: &[Patch], players: &[Vec2]) -> Vec<Footing> {
    let near: Vec<Patch> = patches.iter().filter(|patch| players.iter().any(|player| player.distance(patch.position) <= patch.reach() + GROUND_ARMY_REACH)).copied().collect();
    if near.is_empty() {
        return Vec::new();
    }
    units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.garrison.is_none())
        .filter_map(|(index, unit)| {
            let at = Vec2::new(unit.x, unit.y);
            biome_at(&near, at).map(|biome| Footing { index, biome, from: at, velocity: unit.velocity })
        })
        .collect()
}

//what the ground did to the units on it, once the armies have walked: in a swamp they got SWAMP_KEPT as far as they meant to, on ice they carried on
//the way they were going, letting go of it at ICE_GRIP a second (and so slide, and take a while to turn), and on lava they burn, LAVA_BURN of a
//unit's health a second (a champion as much less as it takes less, and the relics as they say)
pub(crate) fn ground_the_army<R: Relics + ?Sized>(units: &mut [Unit], footing: &[Footing], relics: &R, dt: f32) {
    if dt <= 0.0 {
        return;
    }
    let lava = cause_of(Biome::Lava.name());
    let glide = (-ICE_GRIP * dt).exp();
    for foot in footing {
        let Some(unit) = units.get_mut(foot.index) else {
            continue;
        };
        let moved = Vec2::new(unit.x, unit.y) - foot.from;
        let went = match foot.biome {
            Biome::Swamp => moved * SWAMP_KEPT,
            Biome::Ice => foot.velocity * dt * glide + moved * (1.0 - glide),
            Biome::Lava => {
                unit.hurt_by(UNIT_HEALTH * LAVA_BURN * dt * relics.of(unit.owner).toughness * unit.ward, lava);
                moved
            }
        };
        if went != moved {
            let to = foot.from + went;
            unit.x = to.x.clamp(RADIUS, WORLD_SIZE - RADIUS);
            unit.y = to.y.clamp(RADIUS, WORLD_SIZE - RADIUS);
            unit.velocity = went / dt;
        }
    }
}

//the patches on the minimap, under everything else on it: a disc of each one's colour, dimmed, cut off at the edge of the map
pub(crate) fn draw_patch_marks(map: &Minimap, patches: &[Patch]) {
    const SIDES: usize = 28;
    let rect = map.rect;
    let inside = |point: Vec2| point.clamp(rect.point(), rect.point() + rect.size());
    for patch in patches {
        let point = map.to_screen(patch.position);
        let radius = (patch.radius * map.scale()).max(2.0);
        if point.x + radius < rect.x || point.x - radius > rect.right() || point.y + radius < rect.y || point.y - radius > rect.bottom() {
            continue;
        }
        let color = patch.biome.color();
        let color = Color::new(color.r * 0.4, color.g * 0.4, color.b * 0.4, 0.9);
        for side in 0..SIDES {
            let turn = |k: usize| Vec2::from_angle(k as f32 * std::f32::consts::TAU / SIDES as f32) * radius;
            draw_triangle(inside(point), inside(point + turn(side)), inside(point + turn(side + 1)), color);
        }
    }
}
