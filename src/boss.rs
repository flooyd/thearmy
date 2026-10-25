//! bosses: their definitions, phases and attack patterns, and the director that sends them

use crate::prelude::*;

//the things a boss does to the army, one after another on timers
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Pattern {
    Slam,    //rears up and stomps where it stands, a circle just wider than itself
    Meteors, //a scatter of small circles on the army, landing one after another
    Nova,    //a huge, slow circle round itself, that only running away from it avoids
    Brood,   //calls up its minions
    Ring,    //fires a ring of shots out in every direction, which walls stop
    Burn,    //sets the ground on fire in a few places, which burns for a while
    Charge,  //stands still to show where it is going, then dashes that way
    Pull,    //drags the army towards it for a moment, and then goes off where they end up
    Fortify, //raises a ring of barriers round itself, with gaps, and holds its ground inside it
    Volley,  //a fan of heavy shots at the army
}

//a pattern and how many seconds between one use and the next
pub(crate) struct Cast {
    pub(crate) pattern: Pattern,
    pub(crate) every: f32,
}

//a stretch of a boss's health, from `below` of it (1.0 for the first) down to where the next begins
pub(crate) struct Phase {
    pub(crate) name: &'static str,
    pub(crate) below: f32,
    pub(crate) shield: f32, //a shield this many times its health, new when the phase begins
    pub(crate) speed: f32,  //times as fast as usual
    pub(crate) casts: &'static [Cast],
}

pub(crate) struct BossDef {
    pub(crate) name: &'static str,
    pub(crate) color: Color,
    pub(crate) size: f32,         //pixels along each side for the smallest army...
    pub(crate) growth: f32,       //...and this much more for each square root of the army, so the army can fit round it
    pub(crate) kill_seconds: f32, //how long the whole army, all touching it, would take to destroy it
    pub(crate) speed: f32,        //pixels per second, before the multiplier the army's size gives
    pub(crate) attack_share: f32, //units it kills a second as a share of the army, while it touches them
    pub(crate) babies_share: f32, //babies it drops as a share of the army
    pub(crate) min_army: usize,   //it only comes once the army is this big, and is sized for at least this many
    pub(crate) minion: &'static str,
    pub(crate) brood: usize,      //minions it calls up at a time for the smallest army
    pub(crate) guarded: bool,     //it takes less damage the more minions it has alive
    pub(crate) splits: bool,      //it tears in two when it is hurt, and again, each half with half the health
    pub(crate) keeps_away: f32,   //pixels it keeps from the player, backing off when they come closer, 0 for one that runs at them
    pub(crate) flank: bool,       //it takes much less damage from the units in front of it, which is where the player is
    pub(crate) phases: &'static [Phase],
}

pub(crate) static BOSSES: [BossDef; 7] = [
    BossDef {
        name: "The Hive Mother",
        color: Color::new(0.78, 0.85, 0.2, 1.0),
        size: 150.0,
        growth: 15.0,
        kill_seconds: 6.0,
        speed: 95.0,
        attack_share: 0.012,
        babies_share: 0.6,
        min_army: 100,
        minion: "Imp",
        brood: 6,
        guarded: true,
        splits: false,
        keeps_away: 0.0,
        flank: false,
        phases: &[
            Phase { name: "the brood stirs", below: 1.0, shield: 0.0, speed: 1.0, casts: &[Cast { pattern: Pattern::Brood, every: 7.0 }, Cast { pattern: Pattern::Ring, every: 9.0 }] },
            Phase { name: "acid rain", below: 0.6, shield: 0.0, speed: 1.0, casts: &[Cast { pattern: Pattern::Brood, every: 5.0 }, Cast { pattern: Pattern::Burn, every: 8.0 }, Cast { pattern: Pattern::Ring, every: 7.0 }] },
            Phase { name: "frenzy", below: 0.25, shield: 0.0, speed: 1.4, casts: &[Cast { pattern: Pattern::Brood, every: 3.5 }, Cast { pattern: Pattern::Burn, every: 6.0 }, Cast { pattern: Pattern::Meteors, every: 7.0 }] },
        ],
    },
    BossDef {
        name: "The Iron Warden",
        color: Color::new(0.45, 0.6, 0.85, 1.0),
        size: 200.0,
        growth: 18.0,
        kill_seconds: 6.0,
        speed: 60.0,
        attack_share: 0.012,
        babies_share: 0.7,
        min_army: 200,
        minion: "Bulwark",
        brood: 2,
        guarded: false,
        splits: false,
        keeps_away: 0.0,
        flank: false,
        phases: &[
            Phase { name: "shields up", below: 1.0, shield: 0.2, speed: 1.0, casts: &[Cast { pattern: Pattern::Slam, every: 6.0 }, Cast { pattern: Pattern::Meteors, every: 9.0 }] },
            Phase { name: "reinforced", below: 0.65, shield: 0.3, speed: 1.0, casts: &[Cast { pattern: Pattern::Slam, every: 5.0 }, Cast { pattern: Pattern::Ring, every: 6.0 }, Cast { pattern: Pattern::Brood, every: 10.0 }] },
            Phase { name: "overload", below: 0.3, shield: 0.0, speed: 1.3, casts: &[Cast { pattern: Pattern::Nova, every: 8.0 }, Cast { pattern: Pattern::Slam, every: 4.0 }, Cast { pattern: Pattern::Meteors, every: 6.0 }] },
        ],
    },
    BossDef {
        name: "Warlord Grakk",
        color: Color::new(0.85, 0.3, 0.15, 1.0),
        size: 130.0,
        growth: 13.0,
        kill_seconds: 6.0,
        speed: 120.0,
        attack_share: 0.02,
        babies_share: 0.7,
        min_army: 350,
        minion: "Swarmer",
        brood: 10,
        guarded: false,
        splits: false,
        keeps_away: 0.0,
        flank: false,
        phases: &[
            Phase { name: "the charge", below: 1.0, shield: 0.0, speed: 1.0, casts: &[Cast { pattern: Pattern::Charge, every: 5.0 }, Cast { pattern: Pattern::Brood, every: 9.0 }] },
            Phase { name: "stampede", below: 0.6, shield: 0.0, speed: 1.15, casts: &[Cast { pattern: Pattern::Charge, every: 3.5 }, Cast { pattern: Pattern::Slam, every: 7.0 }, Cast { pattern: Pattern::Brood, every: 8.0 }] },
            Phase { name: "berserk", below: 0.25, shield: 0.0, speed: 1.5, casts: &[Cast { pattern: Pattern::Charge, every: 2.5 }, Cast { pattern: Pattern::Ring, every: 6.0 }, Cast { pattern: Pattern::Slam, every: 6.0 }] },
        ],
    },
    BossDef {
        name: "The Eclipse",
        color: Color::new(0.6, 0.3, 0.95, 1.0),
        size: 220.0,
        growth: 17.0,
        kill_seconds: 7.0,
        speed: 80.0,
        attack_share: 0.012,
        babies_share: 0.8,
        min_army: 600,
        minion: "Drone",
        brood: 4,
        guarded: false,
        splits: false,
        keeps_away: 0.0,
        flank: false,
        phases: &[
            Phase { name: "the light fails", below: 1.0, shield: 0.0, speed: 1.0, casts: &[Cast { pattern: Pattern::Meteors, every: 5.0 }, Cast { pattern: Pattern::Nova, every: 11.0 }] },
            Phase { name: "totality", below: 0.66, shield: 0.15, speed: 1.0, casts: &[Cast { pattern: Pattern::Meteors, every: 4.5 }, Cast { pattern: Pattern::Brood, every: 8.0 }, Cast { pattern: Pattern::Burn, every: 8.0 }] },
            Phase { name: "oblivion", below: 0.33, shield: 0.0, speed: 1.25, casts: &[Cast { pattern: Pattern::Slam, every: 5.0 }, Cast { pattern: Pattern::Nova, every: 7.0 }, Cast { pattern: Pattern::Meteors, every: 4.0 }, Cast { pattern: Pattern::Ring, every: 5.0 }] },
        ],
    },
    //tears in two when it is hurt, and each half does it again: four queens, each with a quarter of the health
    BossDef {
        name: "The Swarm Queen",
        color: Color::new(0.92, 0.35, 0.65, 1.0),
        size: 120.0,
        growth: 12.5,
        kill_seconds: 6.5,
        speed: 90.0,
        attack_share: 0.008,
        babies_share: 0.9,
        min_army: 700,
        minion: "Swarmer",
        brood: 6,
        guarded: false,
        splits: true,
        keeps_away: 0.0,
        flank: false,
        phases: &[
            Phase { name: "the swarm wakes", below: 1.0, shield: 0.0, speed: 1.0, casts: &[Cast { pattern: Pattern::Brood, every: 6.0 }, Cast { pattern: Pattern::Ring, every: 10.0 }] },
            Phase { name: "she divides", below: 0.6, shield: 0.0, speed: 1.0, casts: &[Cast { pattern: Pattern::Brood, every: 5.0 }, Cast { pattern: Pattern::Burn, every: 9.0 }, Cast { pattern: Pattern::Slam, every: 9.0 }] },
            Phase { name: "a thousand wings", below: 0.3, shield: 0.0, speed: 1.2, casts: &[Cast { pattern: Pattern::Brood, every: 4.0 }, Cast { pattern: Pattern::Meteors, every: 8.0 }, Cast { pattern: Pattern::Ring, every: 7.0 }] },
        ],
    },
    //drags the army in and goes off where it lands: walls hold the units back, and the player can outrun it
    BossDef {
        name: "The Singularity",
        color: Color::new(0.3, 0.35, 0.9, 1.0),
        size: 170.0,
        growth: 14.0,
        kill_seconds: 7.0,
        speed: 70.0,
        attack_share: 0.012,
        babies_share: 0.9,
        min_army: 900,
        minion: "Drone",
        brood: 4,
        guarded: false,
        splits: false,
        keeps_away: 0.0,
        flank: false,
        phases: &[
            Phase { name: "gravity gathers", below: 1.0, shield: 0.0, speed: 1.0, casts: &[Cast { pattern: Pattern::Pull, every: 10.0 }, Cast { pattern: Pattern::Meteors, every: 8.0 }] },
            Phase { name: "event horizon", below: 0.62, shield: 0.15, speed: 1.0, casts: &[Cast { pattern: Pattern::Pull, every: 7.5 }, Cast { pattern: Pattern::Nova, every: 10.0 }, Cast { pattern: Pattern::Ring, every: 8.0 }] },
            Phase { name: "collapse", below: 0.3, shield: 0.0, speed: 1.2, casts: &[Cast { pattern: Pattern::Pull, every: 6.0 }, Cast { pattern: Pattern::Slam, every: 5.0 }, Cast { pattern: Pattern::Meteors, every: 6.0 }, Cast { pattern: Pattern::Brood, every: 9.0 }] },
        ],
    },
    //fights from a distance and behind barriers, and its front is armoured: it turns to face the player, so the army has to
    //come at it from where the player isn't
    BossDef {
        name: "The Bastion",
        color: Color::new(0.4, 0.75, 0.7, 1.0),
        size: 150.0,
        growth: 13.0,
        kill_seconds: 8.0,
        speed: 75.0,
        attack_share: 0.01,
        babies_share: 1.0,
        min_army: 1200,
        minion: "Spitter",
        brood: 2,
        guarded: false,
        splits: false,
        keeps_away: 760.0,
        flank: true,
        phases: &[
            Phase { name: "hold the line", below: 1.0, shield: 0.0, speed: 1.0, casts: &[Cast { pattern: Pattern::Volley, every: 3.6 }, Cast { pattern: Pattern::Fortify, every: 15.0 }] },
            Phase { name: "crossfire", below: 0.6, shield: 0.1, speed: 1.0, casts: &[Cast { pattern: Pattern::Volley, every: 3.0 }, Cast { pattern: Pattern::Brood, every: 10.0 }, Cast { pattern: Pattern::Fortify, every: 12.0 }, Cast { pattern: Pattern::Meteors, every: 9.0 }] },
            Phase { name: "no retreat", below: 0.3, shield: 0.0, speed: 1.25, casts: &[Cast { pattern: Pattern::Volley, every: 2.4 }, Cast { pattern: Pattern::Ring, every: 7.0 }, Cast { pattern: Pattern::Fortify, every: 10.0 }, Cast { pattern: Pattern::Slam, every: 7.0 }] },
        ],
    },
];

//the lord of the citadel in the world's corner: not one of the bosses the director sends, it waits in its walls for the army and
//uses what the others do
pub(crate) static FINAL_BOSS: BossDef = BossDef {
    name: "The Sovereign",
    color: Color::new(0.92, 0.2, 0.3, 1.0),
    size: 240.0,
    growth: 17.0,
    kill_seconds: 10.0,
    speed: 85.0,
    attack_share: 0.013,
    babies_share: 1.2,
    min_army: CITADEL_MIN_ARMY,
    minion: "Bulwark",
    brood: 3,
    guarded: false,
    splits: false,
    keeps_away: 0.0,
    flank: false,
    phases: &[
        Phase { name: "the court assembles", below: 1.0, shield: 0.0, speed: 1.0, casts: &[Cast { pattern: Pattern::Brood, every: 8.0 }, Cast { pattern: Pattern::Slam, every: 6.0 }, Cast { pattern: Pattern::Meteors, every: 9.0 }] },
        Phase { name: "the crown burns", below: 0.75, shield: 0.0, speed: 1.0, casts: &[Cast { pattern: Pattern::Pull, every: 10.0 }, Cast { pattern: Pattern::Burn, every: 8.0 }, Cast { pattern: Pattern::Ring, every: 7.0 }] },
        Phase { name: "no mercy", below: 0.5, shield: 0.2, speed: 1.0, casts: &[Cast { pattern: Pattern::Fortify, every: 14.0 }, Cast { pattern: Pattern::Volley, every: 4.0 }, Cast { pattern: Pattern::Nova, every: 9.0 }, Cast { pattern: Pattern::Meteors, every: 6.0 }] },
        Phase { name: "the sovereign falls", below: 0.25, shield: 0.0, speed: 1.3, casts: &[Cast { pattern: Pattern::Pull, every: 7.0 }, Cast { pattern: Pattern::Nova, every: 7.0 }, Cast { pattern: Pattern::Slam, every: 5.0 }, Cast { pattern: Pattern::Meteors, every: 5.0 }, Cast { pattern: Pattern::Charge, every: 6.0 }] },
    ],
};

//where a boss is in its fight
#[derive(Clone)]
pub(crate) struct BossState {
    pub(crate) def: &'static BossDef,
    pub(crate) tier: u32,          //how many times round the bosses have been, each time they are tougher and quicker
    pub(crate) phase: usize,
    pub(crate) timers: Vec<f32>,   //seconds until each cast of the phase is used again
    pub(crate) shifting: f32,      //seconds left of the pause between phases, in which it can't be hurt
    pub(crate) last_stand: bool,
    pub(crate) family: u32,        //the id of the first of the bosses that came from one, it is the family the director waits for
    pub(crate) family_max: f32,    //the health of the whole family when it came
    pub(crate) generation: u32,    //how many times it has split
    pub(crate) facing: Vec2,       //the way it is looking, which a boss with an armoured front turns to the player
    pub(crate) anchor_left: f32,   //seconds it holds its ground, inside the barriers it raised
}

impl Enemy {
    //a boss, sized for an army of this many units: as tall as the army can fit round and with as much health as the army
    //would take def.kill_seconds to cut through if all of it touched. it runs at the player from the start and never gives up
    pub(crate) fn boss(def: &'static BossDef, army: usize, tier: u32, start: Vec2) -> Enemy {
        let sized_for = army.max(def.min_army) as f32;
        let size = def.size + def.growth * sized_for.sqrt();
        let toughness = 1.0 + BOSS_TIER_HEALTH * tier as f32;
        let health = sized_for * UNIT_DAMAGE * def.kill_seconds * toughness;
        let phase = &def.phases[0];
        let id = NEXT_ENEMY_ID.fetch_add(1, Ordering::Relaxed);
        let edge = size / 2.0;
        let start = start.clamp(Vec2::splat(edge), Vec2::splat(WORLD_SIZE - edge));
        Enemy {
            id,
            name: def.name,
            position: start,
            size,
            color: def.color,
            max_health: health,
            health,
            babies: (sized_for * def.babies_share).round() as usize,
            //the chase multiplies it again, so this is what makes it def.speed on the chase
            speed: def.speed / CHASE_SPEED * phase.speed * tier_pace(tier),
            route: vec![start],
            heading_for: 0,
            attack: sized_for * UNIT_HEALTH * def.attack_share * BOSS_ATTACK_FACTOR * (1.0 + BOSS_TIER_ATTACK * tier as f32),
            aggro: WORLD_SIZE * 4.0,
            chasing: true,
            range: 0.0,
            volley: 0,
            shot_in: 99.0,
            hurt: 0.0,
            killed: 0,
            popup_in: rand::gen_range(0.0, POPUP_INTERVAL),
            behavior: Behavior::Walker,
            modifier: Modifier::None,
            scale: army_scale(army),
            owner: None,
            armor: 0.0,
            shield: health * phase.shield,
            max_shield: health * phase.shield,
            shield_idle: 0.0,
            mode: Mode::Roam,
            timer: 0.0,
            flash: 10.0,
            blew_up: false,
            boss: Some(BossState {
                def,
                tier,
                phase: 0,
                timers: phase.casts.iter().map(|cast| cast.every * rand::gen_range(0.6, 1.0)).collect(),
                shifting: 0.0,
                last_stand: false,
                family: id,
                family_max: health,
                generation: 0,
                facing: Vec2::X,
                anchor_left: 0.0,
            }),
            site: None,
            steer: Steer::Player,
            steer_in: rand::gen_range(0.0, STEERING_INTERVAL),
            stuck: 0.0,
            pressing: None,
            status: Statuses::default(),
            provoked: 0.0,
            grudge: Vec2::ZERO,
            rout: 0.0,
            rallied: false,
            credit: 0,
            struck: 0.0,
        }
    }
}

impl Enemy {
    //a boss as it should be for a harder difficulty: `rounds` further on than it was made for (tougher, quicker and harder hitting,
    //as each time round the bosses are). only for one that has just come, as it goes back to its full health
    pub(crate) fn harden(&mut self, rounds: u32) {
        let Some(tier) = self.boss.as_ref().map(|state| state.tier) else {
            return;
        };
        let round = |tier: u32, per_round: f32| 1.0 + per_round * tier as f32;
        let tougher = round(tier + rounds, BOSS_TIER_HEALTH) / round(tier, BOSS_TIER_HEALTH);
        self.empower_by(tougher, round(tier + rounds, BOSS_TIER_ATTACK) / round(tier, BOSS_TIER_ATTACK), 1.0);
        self.speed *= tier_pace(tier + rounds) / tier_pace(tier);
        if let Some(state) = self.boss.as_mut() {
            state.tier += rounds;
            state.family_max = self.max_health;
        }
    }
}

//how much quicker a boss is for the number of times round the bosses have been, and no more than BOSS_TIER_PACE_MAX
pub(crate) fn tier_pace(tier: u32) -> f32 {
    1.0 + (BOSS_TIER_PACE * tier as f32).min(BOSS_TIER_PACE_MAX)
}

//one of the attacks a boss warns of beforehand, as the circle on the ground: it does `weight` times a unit's health, times
//BOSS_BLAST_FACTOR and more each time round, to each unit in it, up to `share` of the army (and that more each time round)
//of them. that kills outright what it reaches, whatever shields and toughness the army has, so the way to meet it is to be
//somewhere else. walls take what they always did from it
pub(crate) fn blast(position: Vec2, radius: f32, warn: f32, weight: f32, share: f32, army: usize, tier: u32, color: Color, boom: Cue) -> Hazard {
    let damage = UNIT_HEALTH * weight * BOSS_BLAST_FACTOR * (1.0 + BOSS_TIER_BLAST * tier as f32);
    let victims = victims_of(army, (share * (1.0 + BOSS_TIER_REACH * tier as f32)).min(1.0));
    let mut hazard = Hazard::new(position, radius, warn, damage, victims, color, boom);
    hazard.wall_damage = UNIT_HEALTH * weight * BLAST_WALL_FACTOR;
    hazard
}

//move a boss into its next phase: a pause in which it can't be hurt, a pulse that clears the army from round it, a new
//set of attacks and a new shield
pub(crate) fn enter_phase(enemy: &mut Enemy, state: &mut BossState, phase: usize, army: usize, fx: &mut Effects) {
    let def = state.def;
    let next = &def.phases[phase];
    state.phase = phase;
    state.timers = next.casts.iter().map(|cast| cast.every * rand::gen_range(0.6, 1.0)).collect();
    state.shifting = PHASE_SHIFT_SECONDS;
    enemy.speed = def.speed / CHASE_SPEED * next.speed * tier_pace(state.tier) * if state.last_stand { 1.3 } else { 1.0 };
    if next.shield > 0.0 {
        enemy.max_shield = enemy.max_health * next.shield;
        enemy.shield = enemy.max_shield;
        enemy.shield_idle = 0.0;
    }
    enemy.mode = Mode::Cast { left: PHASE_SHIFT_SECONDS };
    //the pulse goes off as the pause ends, so the army has the pause to get away
    fx.hazards.push(blast(enemy.position, enemy.size * 0.5 + 260.0, PHASE_SHIFT_SECONDS, 1.0, 0.4, army, state.tier, enemy.color, Cue::Slam).vs_walls(enemy.scale).circle(BossCircle::Shift));
    fx.announce(format!("{}: {}", def.name, next.name.to_uppercase()), enemy.color);
    fx.add_shake(0.6);
    //(it cracks: pieces of it fly)
    fx.shatter(enemy.position, enemy.size * 0.5, enemy.color, enemy.id ^ 0x9E37_79B9, false);
    fx.cue(Cue::PhaseShift);
}

//do one of a boss's attacks. `owned` is how many minions it has alive. the ones it calls go into `born`
pub(crate) fn cast_pattern(enemy: &mut Enemy, state: &mut BossState, pattern: Pattern, units: &[Unit], player: Vec2, owned: usize, born: &mut Vec<Enemy>, shots: &mut Vec<Shot>, fx: &mut Effects) {
    let army = units.len();
    let root = (army as f32).sqrt();
    let tier = state.tier as usize;
    let color = enemy.color;
    //a spot in the army (or on the player when there is none), up to `spread` pixels from one of its units
    let in_army = |spread: f32| -> Vec2 {
        let base = if units.is_empty() {
            player
        } else {
            let unit = &units[rand::gen_range(0, units.len())];
            Vec2::new(unit.x, unit.y)
        };
        base + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * rand::gen_range(0.0, spread)
    };
    match pattern {
        Pattern::Slam => {
            let radius = enemy.size * 0.75 + 140.0;
            let warn = blast_warn(BOSS_WARN_SLAM, radius);
            fx.hazards.push(blast(enemy.position, radius, warn, 1.0, 0.4, army, state.tier, color, Cue::Slam).vs_walls(enemy.scale).circle(BossCircle::Slam));
            enemy.mode = Mode::Cast { left: warn + 0.5 };
            fx.cue(Cue::Warning);
        }
        Pattern::Meteors => {
            let count = 5 + 2 * tier + (army / 400).min(6);
            for k in 0..count {
                fx.hazards.push(blast(in_army(120.0), 90.0 + root * 2.0, BOSS_WARN_METEOR + 0.2 * k as f32, 1.0, 0.04, army, state.tier, color, Cue::Explosion).vs_walls(enemy.scale).circle(BossCircle::Meteor));
            }
            enemy.mode = Mode::Cast { left: 0.6 };
            fx.cue(Cue::Warning);
        }
        Pattern::Nova => {
            let radius = 650.0 + enemy.size * 0.6;
            let warn = blast_warn(BOSS_WARN_NOVA, radius);
            fx.hazards.push(blast(enemy.position, radius, warn, 1.0, 0.55, army, state.tier, color, Cue::Slam).vs_walls(enemy.scale).circle(BossCircle::Nova));
            enemy.mode = Mode::Cast { left: warn + 0.4 };
            fx.cue(Cue::Warning);
        }
        Pattern::Brood => {
            let want = state.def.brood + (root / 4.0) as usize + tier;
            let count = want.min(MAX_BOSS_MINIONS.saturating_sub(owned));
            for _ in 0..count {
                let around = enemy.position + Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * (enemy.size / 2.0 + rand::gen_range(20.0, 120.0));
                born.push(Enemy::minion(around, kind_named(state.def.minion), enemy.scale, enemy.id));
            }
            if count > 0 {
                fx.burst(enemy.position, color, 40, 300.0);
                fx.cue_at(Cue::Summon, enemy.position);
            }
            enemy.mode = Mode::Cast { left: 0.7 };
        }
        Pattern::Ring => {
            let count = 16 + 4 * tier + (army / 300).min(16);
            let turn = rand::gen_range(0.0, std::f32::consts::TAU);
            for k in 0..count {
                let direction = Vec2::from_angle(turn + std::f32::consts::TAU * k as f32 / count as f32);
                shots.push(Shot::heavy(enemy.position + direction * enemy.size / 2.0, direction * 340.0, color));
            }
            fx.cue_at(Cue::Shoot, enemy.position);
            enemy.mode = Mode::Cast { left: 0.5 };
        }
        Pattern::Burn => {
            for _ in 0..3 + tier {
                fx.hazards.push(Hazard::new(in_army(200.0), 150.0 + root * 1.5, 0.9, 0.0, 0, Color::new(1.0, 0.5, 0.1, 1.0), Cue::Explosion).burning(5.0, 45.0));
            }
            enemy.mode = Mode::Cast { left: 0.5 };
            fx.cue(Cue::Warning);
        }
        Pattern::Charge => {
            enemy.mode = Mode::Windup { dir: (player - enemy.position).normalize_or_zero(), left: CHARGE_WINDUP * 0.8 };
            fx.cue(Cue::Warning);
        }
        Pattern::Pull => {
            //the army is dragged in for a moment, and where it ends up it is blown up
            let warn = BOSS_WARN_PULL;
            let reach = 1000.0 + enemy.size;
            fx.pulls.push(Pull { position: enemy.position, radius: reach, left: warn, total: warn });
            fx.hazards.push(blast(enemy.position, enemy.size * 0.5 + 200.0, warn, 1.2, 0.45, army, state.tier, color, Cue::Slam).vs_walls(enemy.scale).circle(BossCircle::Pull));
            enemy.mode = Mode::Cast { left: warn + 0.4 };
            fx.cue_at(Cue::Pull, enemy.position);
        }
        Pattern::Fortify => {
            //barriers go up round it, with two gaps, and it stays put inside them until they are down
            let half = (enemy.size / 2.0 / TILE_SIZE).ceil() as i32 + 3;
            let middle = tile_of(enemy.position.x, enemy.position.y);
            let first = rand::gen_range(0, 4);
            let second = (first + rand::gen_range(1, 4)) % 4;
            let reach = (half - 3).max(1);
            let health = WALL_HEALTH * 1.5 * army_scale(army) * (1.0 + 0.3 * tier as f32);
            for (col, row) in lair_tiles(middle, half, [first, second], [rand::gen_range(-reach, reach), rand::gen_range(-reach, reach)]) {
                let mut barrier = Wall::made(col, row, WallKind::Barrier, Side::Hostile, health);
                barrier.life = BARRIER_SECONDS;
                fx.raise.push(barrier);
            }
            state.anchor_left = BARRIER_SECONDS * 0.8;
            enemy.mode = Mode::Cast { left: 0.8 };
            fx.burst(enemy.position, color, 30, 260.0);
            fx.cue(Cue::Warning);
        }
        Pattern::Volley => {
            //a fan of heavy shots at somewhere in the army
            let count = 5 + tier + (army / 400).min(6);
            let aim = (in_army(60.0) - enemy.position).normalize_or_zero();
            for k in 0..count {
                let direction = aim.rotate(Vec2::from_angle((k as f32 - (count as f32 - 1.0) / 2.0) * 0.11));
                shots.push(Shot::heavy(enemy.position + direction * enemy.size / 2.0, direction * 460.0, color));
            }
            fx.cue_at(Cue::Shoot, enemy.position);
            enemy.mode = Mode::Cast { left: 0.45 };
        }
    }
}

//the same for one player, whose army is `units`: what the tests that have just the one player use
#[cfg(test)]
pub(crate) fn boss_act(enemies: &mut Vec<Enemy>, units: &[Unit], player: Vec2, dt: f32, shots: &mut Vec<Shot>, fx: &mut Effects) {
    let field = FlowField::new();
    let prey = [Prey { position: player, units, team: units.len(), pressure: units.len(), boost: 1.0, cap: DEFAULT_ARMY_LIMIT, field: &field }];
    boss_act_among(enemies, &prey, dt, shots, fx);
}

//every boss: it changes phase as its health runs down, makes a last stand at the end, and uses the attacks of its phase
//when their timers run out (one at a time, never while it is dashing or already in the middle of one). it turns on the player nearest
//to it, and what it throws is thrown at that player's army, and sized for it
pub(crate) fn boss_act_among(enemies: &mut Vec<Enemy>, prey: &[Prey], dt: f32, shots: &mut Vec<Shot>, fx: &mut Effects) {
    let mut born: Vec<Enemy> = Vec::new();
    for i in 0..enemies.len() {
        let Some(mut state) = enemies[i].boss.take() else {
            continue;
        };
        let id = enemies[i].id;
        let hunted = nearest(prey, enemies[i].position);
        let (player, units) = (hunted.position, hunted.units);
        let army = units.len();
        let owned = enemies.iter().filter(|other| other.owner == Some(id)).count() + born.len();
        let enemy = &mut enemies[i];
        let def = state.def;
        if def.guarded {
            enemy.armor = (owned as f32 * 0.04).min(0.6);
        }
        //it turns to look at the player, not quite as fast as they can run round it
        let wanted = (player - enemy.position).normalize_or_zero();
        if wanted != Vec2::ZERO {
            let angle = state.facing.perp_dot(wanted).atan2(state.facing.dot(wanted));
            let turn = angle.clamp(-FLANK_TURN * dt, FLANK_TURN * dt);
            state.facing = state.facing.rotate(Vec2::from_angle(turn)).normalize_or_zero();
        }
        state.anchor_left = (state.anchor_left - dt).max(0.0);
        if state.shifting > 0.0 {
            state.shifting = (state.shifting - dt).max(0.0);
            enemy.boss = Some(state);
            continue;
        }
        let health_share = enemy.health / enemy.max_health;
        if let Some(phase) = def.phases.iter().rposition(|phase| health_share <= phase.below) {
            if phase > state.phase {
                enter_phase(enemy, &mut state, phase, army, fx);
                enemy.boss = Some(state);
                continue;
            }
        }
        if def.splits && (state.generation as usize) < SPLIT_AT.len() && health_share <= SPLIT_AT[state.generation as usize] {
            let twin = split_in_two(enemy, &mut state, army, fx);
            born.push(twin);
            enemy.boss = Some(state);
            continue;
        }
        if !state.last_stand && health_share < LAST_STAND_AT {
            state.last_stand = true;
            enemy.speed *= 1.3;
            fx.announce(format!("{}: LAST STAND!", def.name), RED);
            fx.add_shake(0.5);
            fx.shatter(enemy.position, enemy.size * 0.6, enemy.color, enemy.id ^ 0x85EB_CA6B, false);
            fx.cue(Cue::PhaseShift);
        }
        //the lord of a citadel waits in it for someone to come. if it is hurt before then it still changes as it should, as it
        //can't be hurt past that, but it uses none of its attacks
        if enemy.site.is_some() && !enemy.chasing {
            enemy.boss = Some(state);
            continue;
        }
        let rate = if state.last_stand { 1.6 } else { 1.0 };
        let mut due = None;
        for (k, timer) in state.timers.iter_mut().enumerate() {
            *timer -= dt * rate;
            if *timer <= 0.0 && due.is_none() && enemy.mode == Mode::Roam {
                due = Some(k);
            }
        }
        if let Some(k) = due {
            let cast = &def.phases[state.phase].casts[k];
            state.timers[k] = cast.every * 0.92f32.powi(state.tier as i32) * rand::gen_range(0.85, 1.15);
            //the others wait their turn, and after an attack that kills what it lands on the next one waits until the army has had
            //time to get back to the boss
            let blast = is_blast(cast.pattern);
            for (timer, other) in state.timers.iter_mut().zip(def.phases[state.phase].casts) {
                *timer = timer.max(if blast && is_blast(other.pattern) { BOSS_BLAST_HOLD } else { 0.8 });
            }
            cast_pattern(enemy, &mut state, cast.pattern, units, player, owned, &mut born, shots, fx);
        }
        enemy.boss = Some(state);
    }
    enemies.extend(born);
}

//the seconds a circle on the ground shows before it goes off: `base`, and a little longer the bigger it is, because the army has
//further to go to get out of it
pub(crate) fn blast_warn(base: f32, radius: f32) -> f32 {
    base + BOSS_WARN_PER_1000_PX * radius / 1000.0
}

//whether an attack shows a circle on the ground that kills what it lands on, the ones to get out of the way of
pub(crate) fn is_blast(pattern: Pattern) -> bool {
    matches!(pattern, Pattern::Slam | Pattern::Meteors | Pattern::Nova | Pattern::Pull)
}

//the kinds of circle a boss puts on the ground that kill what they land on. each has a colour of its own, so the army can tell at a
//glance which one it is looking at and what to do about it
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum BossCircle {
    Slam,   //round the boss, where it stands
    Meteor, //small ones on the army, one after another
    Nova,   //a huge one round the boss
    Pull,   //the well that drags the army in, and goes off where it ends up
    Shift,  //the pulse as a boss changes phase
    Split,  //the pulse as a boss tears in two
}

#[cfg(test)]
pub(crate) const CIRCLES: [BossCircle; 6] = [BossCircle::Slam, BossCircle::Meteor, BossCircle::Nova, BossCircle::Pull, BossCircle::Shift, BossCircle::Split];

impl BossCircle {
    pub(crate) fn name(self) -> &'static str {
        match self {
            BossCircle::Slam => "SLAM",
            BossCircle::Meteor => "METEORS",
            BossCircle::Nova => "NOVA",
            BossCircle::Pull => "GRAVITY WELL",
            BossCircle::Shift => "PHASE PULSE",
            BossCircle::Split => "SPLIT PULSE",
        }
    }

    pub(crate) fn color(self) -> Color {
        match self {
            BossCircle::Slam => Color::new(1.0, 0.42, 0.12, 1.0),   //orange
            BossCircle::Meteor => Color::new(1.0, 0.86, 0.2, 1.0),  //gold
            BossCircle::Nova => Color::new(0.82, 0.38, 1.0, 1.0),   //violet
            BossCircle::Pull => Color::new(0.4, 0.58, 1.0, 1.0),    //blue
            BossCircle::Shift => Color::new(0.25, 1.0, 0.8, 1.0),   //teal
            BossCircle::Split => Color::new(1.0, 0.5, 0.78, 1.0),   //pink
        }
    }

    //the colour it is drawn in, as a word
    pub(crate) fn colour_name(self) -> &'static str {
        match self {
            BossCircle::Slam => "ORANGE",
            BossCircle::Meteor => "GOLD",
            BossCircle::Nova => "VIOLET",
            BossCircle::Pull => "BLUE",
            BossCircle::Shift => "TEAL",
            BossCircle::Split => "PINK",
        }
    }

    //what to do about it, in a few words
    pub(crate) fn advice(self) -> &'static str {
        match self {
            BossCircle::Slam | BossCircle::Shift | BossCircle::Split => "walk clear of the boss",
            BossCircle::Meteor => "keep moving",
            BossCircle::Nova => "run out of the huge circle",
            BossCircle::Pull => "run from the well",
        }
    }
}

//the circle an attack puts on the ground, if it puts one
pub(crate) fn circle_of(pattern: Pattern) -> Option<BossCircle> {
    match pattern {
        Pattern::Slam => Some(BossCircle::Slam),
        Pattern::Meteors => Some(BossCircle::Meteor),
        Pattern::Nova => Some(BossCircle::Nova),
        Pattern::Pull => Some(BossCircle::Pull),
        _ => None,
    }
}

//the circle a boss will put down next
pub(crate) struct NextCircle {
    pub(crate) circle: BossCircle,
    pub(crate) seconds: f32, //until it can, a boss that is busy with something else puts it down as soon as it is done
    pub(crate) share: f32,   //0 just after the last one, 1 when it is due
}

impl BossState {
    //the circle that is due next, which one it is and how soon, from the timers of the attacks of this phase. none while it is between
    //phases, or if the phase has none
    pub(crate) fn next_circle(&self) -> Option<NextCircle> {
        if self.shifting > 0.0 {
            return None;
        }
        let rate = if self.last_stand { 1.6 } else { 1.0 };
        let whole = |every: f32| (every * 0.92f32.powi(self.tier as i32)).max(BOSS_BLAST_HOLD) / rate;
        self.def.phases[self.phase]
            .casts
            .iter()
            .zip(&self.timers)
            .filter_map(|(cast, &timer)| circle_of(cast.pattern).map(|circle| (circle, (timer / rate).max(0.0), whole(cast.every))))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(circle, seconds, whole)| NextCircle { circle, seconds, share: (1.0 - seconds / whole.max(seconds).max(0.001)).clamp(0.0, 1.0) })
    }
}

//the share of its health a boss that splits is at when it does, for the first time and the second
pub(crate) const SPLIT_AT: [f32; 2] = [0.55, 0.3];

//a boss that splits tears in two: the other half stands beside it, each with half the health and a little less of everything
//else. returns the other half
pub(crate) fn split_in_two(enemy: &mut Enemy, state: &mut BossState, army: usize, fx: &mut Effects) -> Enemy {
    state.generation += 1;
    enemy.size *= 0.8;
    enemy.max_health /= 2.0;
    enemy.health /= 2.0;
    enemy.max_shield /= 2.0;
    enemy.shield /= 2.0;
    enemy.attack *= 0.8;
    enemy.speed *= 1.1;
    enemy.babies = enemy.babies.div_ceil(2);
    state.shifting = SPLIT_PAUSE;
    enemy.mode = Mode::Cast { left: SPLIT_PAUSE };
    let mut twin = enemy.clone();
    twin.id = NEXT_ENEMY_ID.fetch_add(1, Ordering::Relaxed);
    //the twin's attacks come half a round after its sibling's, otherwise every one of them goes off at the army at once
    let mut twin_state = state.clone();
    for (timer, cast) in twin_state.timers.iter_mut().zip(state.def.phases[state.phase].casts) {
        *timer += cast.every * 0.5;
    }
    twin.boss = Some(twin_state);
    let apart = Vec2::from_angle(rand::gen_range(0.0, std::f32::consts::TAU)) * enemy.size * 0.7;
    let middle = enemy.position;
    let edge = Vec2::splat(enemy.size / 2.0);
    enemy.position = (middle + apart).clamp(edge, Vec2::splat(WORLD_SIZE) - edge);
    twin.position = (middle - apart).clamp(edge, Vec2::splat(WORLD_SIZE) - edge);
    //a pulse that clears the army from between them
    fx.hazards.push(blast(middle, enemy.size * 0.8 + 160.0, SPLIT_PAUSE, 1.0, 0.3, army, state.tier, enemy.color, Cue::Slam).vs_walls(enemy.scale).circle(BossCircle::Split));
    fx.announce(format!("{} DIVIDES", state.def.name.to_uppercase()), enemy.color);
    fx.burst(middle, enemy.color, 60, 340.0);
    fx.add_shake(0.5);
    fx.cue_at(Cue::Split, middle);
    twin
}

//how much of the damage a unit at a point does to a boss: all of it, unless the boss has an armoured front and the unit is in it
pub(crate) fn flank_factor(enemy: &Enemy, at: Vec2) -> f32 {
    let Some(state) = enemy.boss.as_ref().filter(|state| state.def.flank) else {
        return 1.0;
    };
    let to = (at - enemy.position).normalize_or_zero();
    if to == Vec2::ZERO || state.facing.dot(to) >= FLANK_ARC.cos() {
        FLANK_FRONT
    } else {
        1.0
    }
}

//a gravity well that is dragging the army in
pub(crate) struct Pull {
    pub(crate) position: Vec2,
    pub(crate) radius: f32, //pixels, what it reaches
    pub(crate) left: f32,   //seconds
    pub(crate) total: f32,
}

//drag the units, and the players less, within each pull towards its middle. a wall in the way stops them, and one near the
//middle is as far as they get. `players` are where the players are, which are moved
pub(crate) fn update_pulls_among(pulls: &mut Vec<Pull>, units: &mut [Unit], players: &mut [Vec2], walls: &Walls, dt: f32) {
    for pull in pulls.iter() {
        let step = PULL_SPEED * dt;
        for unit in units.iter_mut().filter(|unit| unit.garrison.is_none()) {
            let from = Vec2::new(unit.x, unit.y);
            let to_middle = pull.position - from;
            let distance = to_middle.length();
            if distance > pull.radius || distance < RADIUS {
                continue;
            }
            let moved = move_clear(from, from + to_middle / distance * step.min(distance), RADIUS, Mover::Ally, walls);
            unit.x = moved.x;
            unit.y = moved.y;
        }
        for player in players.iter_mut() {
            let to_middle = pull.position - *player;
            let distance = to_middle.length();
            if distance <= pull.radius && distance >= RADIUS {
                *player = move_clear(*player, *player + to_middle / distance * (step * PULL_PLAYER_SHARE).min(distance), RADIUS, Mover::Ally, walls);
            }
        }
    }
    pulls.retain_mut(|pull| {
        pull.left -= dt;
        pull.left > 0.0
    });
}

//the same with one player. returns their new position
#[cfg(test)]
pub(crate) fn update_pulls(pulls: &mut Vec<Pull>, units: &mut [Unit], player: Vec2, walls: &Walls, dt: f32) -> Vec2 {
    let mut players = [player];
    update_pulls_among(pulls, units, &mut players, walls, dt);
    players[0]
}

//rings that shrink towards the middle of each pull, faster as it goes
pub(crate) fn draw_pulls(pulls: &[Pull], time: f32) {
    for pull in pulls {
        let (x, y) = (pull.position.x, pull.position.y);
        let strength = (pull.left / pull.total).clamp(0.0, 1.0);
        let edge = Color::new(0.45, 0.5, 1.0, 1.0);
        draw_circle(x, y, pull.radius, Color::new(edge.r, edge.g, edge.b, 0.05));
        draw_circle_lines(x, y, pull.radius, 5.0, Color::new(edge.r, edge.g, edge.b, 0.5));
        for k in 0..5 {
            let phase = (time * 0.9 + k as f32 / 5.0).fract();
            let radius = pull.radius * (1.0 - phase * phase);
            draw_circle_lines(x, y, radius, 3.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.5 * phase * (0.4 + 0.6 * strength)));
        }
    }
}

//what the director found out in a frame
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Beat {
    Warning,                                    //a boss is about to come
    Arrived(&'static str),                      //it has
    Slain { name: &'static str, tier: u32 },    //the boss that was alive is gone
}

//sends a boss every so often: the first FIRST_BOSS_SECONDS into a run, the others BOSS_GAP_SECONDS after the last was
//slain, the four of them in turn and then round again, tougher
#[derive(Clone)]
pub(crate) struct Director {
    pub(crate) next_boss_in: f32,
    pub(crate) slain: u32,         //bosses slain
    pub(crate) boss: Option<u32>,  //the family of the boss that is alive: the id of the first of it, as a boss that splits is more than one
    pub(crate) warned: bool,
}

impl Director {
    pub(crate) fn new() -> Director {
        Director { next_boss_in: FIRST_BOSS_SECONDS, slain: 0, boss: None, warned: false }
    }

    pub(crate) fn next_def(&self) -> &'static BossDef {
        &BOSSES[self.slain as usize % BOSSES.len()]
    }

    pub(crate) fn next_tier(&self) -> u32 {
        self.slain / BOSSES.len() as u32
    }

    //the same for one player, whose army is `army` big: what the tests that have just the one player use
    #[cfg(test)]
    pub(crate) fn update(&mut self, enemies: &mut Vec<Enemy>, player: Vec2, army: usize, dt: f32) -> Option<Beat> {
        let field = FlowField::new();
        let prey = [Prey { position: player, units: &[], team: army, pressure: army, boost: 1.0, cap: DEFAULT_ARMY_LIMIT, field: &field }];
        self.update_among(enemies, &prey, dt)
    }

    //count down, and when it is time put a boss in `enemies`, coming from far out at one of the players, made for the fight they are in.
    //a boss only comes once a fight is big enough for it, and the warning with it. returns what happened, if anything
    pub(crate) fn update_among(&mut self, enemies: &mut Vec<Enemy>, prey: &[Prey], dt: f32) -> Option<Beat> {
        if let Some(id) = self.boss {
            if enemies.iter().any(|enemy| enemy.boss.as_ref().is_some_and(|state| state.family == id)) {
                return None;
            }
            let def = &BOSSES[(self.slain as usize) % BOSSES.len()];
            let tier = self.next_tier();
            self.boss = None;
            self.slain += 1;
            self.next_boss_in = BOSS_GAP_SECONDS;
            self.warned = false;
            return Some(Beat::Slain { name: def.name, tier });
        }
        //the lord of a citadel is a fight to itself
        if enemies.iter().any(|enemy| enemy.boss.is_some() && enemy.site.is_some()) {
            return None;
        }
        let def = self.next_def();
        //the countdown waits for an army big enough to be worth a boss, and it is sent at one of the players who have one
        let worth_it: Vec<&Prey> = prey.iter().filter(|player| player.team >= def.min_army).collect();
        if worth_it.is_empty() {
            return None;
        }
        self.next_boss_in = (self.next_boss_in - dt).max(0.0);
        if self.next_boss_in <= 0.0 {
            let target = worth_it[rand::gen_range(0, worth_it.len())];
            let boss = spawn_boss(target.position, target.team, def, self.next_tier());
            self.boss = Some(boss.id);
            enemies.push(boss);
            return Some(Beat::Arrived(def.name));
        }
        if self.next_boss_in <= BOSS_WARNING_SECONDS && !self.warned {
            self.warned = true;
            return Some(Beat::Warning);
        }
        None
    }
}

//a boss, BOSS_SPAWN_MIN_TILES to BOSS_SPAWN_MAX_TILES from the player
pub(crate) fn spawn_boss(player: Vec2, army: usize, def: &'static BossDef, tier: u32) -> Enemy {
    let start = random_position_around(player.x, player.y, BOSS_SPAWN_MIN_TILES * TILE_SIZE, BOSS_SPAWN_MAX_TILES * TILE_SIZE);
    Enemy::boss(def, army, tier, start)
}

//the boss that is alive, if there is one. of a family that has split, the oldest of it
pub(crate) fn current_boss(enemies: &[Enemy]) -> Option<&Enemy> {
    enemies.iter().find(|enemy| enemy.boss.is_some())
}

//how much is left of the boss and everything that came of it, as a share of what it started with
pub(crate) fn family_health(enemies: &[Enemy], boss: &Enemy) -> f32 {
    let Some(state) = boss.boss.as_ref() else {
        return 0.0;
    };
    let left: f32 = enemies.iter().filter(|other| other.boss.as_ref().is_some_and(|other| other.family == state.family)).map(|other| other.health.max(0.0)).sum();
    (left / state.family_max.max(1.0)).clamp(0.0, 1.0)
}

