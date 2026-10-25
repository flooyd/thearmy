//! enemy kinds and the Enemy itself: stats, modifiers, modes and drawing

use crate::prelude::*;
use std::cell::RefCell;

//the chance that a new enemy is an elite: none for a small army, more the bigger it gets
pub(crate) fn elite_chance(army: usize) -> f32 {
    if army < ELITE_FROM_UNITS {
        return 0.0;
    }
    ELITE_MAX_CHANCE * ((army - ELITE_FROM_UNITS) as f32 / (ELITE_FULL_UNITS - ELITE_FROM_UNITS) as f32).min(1.0)
}

//whether an enemy counts towards the ENEMY_COUNT that are kept alive: the ones that are not made by something else
pub(crate) fn is_wild(enemy: &Enemy) -> bool {
    enemy.owner.is_none() && enemy.boss.is_none() && enemy.site.is_none()
}

//how much bigger than the starting size enemies are made for an army of this many units. the bigger the army the more
//there is to fight with, so size grows with this, health with its square (a bigger enemy has more units inside it
//at once), and the babies with it
pub(crate) fn army_scale(army: usize) -> f32 {
    (army as f32 / SCALE_FROM_UNITS).max(1.0).sqrt().min(MAX_ENEMY_SCALE)
}

//how many times tougher than size alone makes them enemies are for an army this big: 1 up to ENEMY_MIGHT_FROM_UNITS units, where they are as big as they get
pub(crate) fn army_might(army: usize) -> f32 {
    (army as f32 / ENEMY_MIGHT_FROM_UNITS).max(1.0)
}

//the size of army the wild enemies are made for: the army's, and more for every boss it has destroyed, so a run that goes on gets harder
pub(crate) fn enemy_pressure(army: usize, bosses_slain: usize) -> usize {
    (army as f32 * (1.0 + ENEMY_ROUND_GROWTH * bosses_slain as f32)).round() as usize
}

//what an enemy does besides walking about and fighting by touch
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Behavior {
    Walker,   //walks its route, and runs at the player when it notices them
    Shooter,  //fires volleys at the units from where it is
    Splitter, //breaks into smaller ones when it is destroyed
    Charger,  //stops to show where it is going, then dashes
    Bomber,   //runs at the army and blows up
    Healer,   //stays with the others and mends them
    Summoner, //calls up imps while the player is near
    Mortar,   //lobs shells that land a moment later
    Nest,     //stays where it is and sends out more of its own, until it is destroyed
    Ambusher, //lies hidden where it is until the player comes close, then springs
    Executioner, //winds up a cleave at the units in front of it
    Sniper,      //shows a long line at the army and fires down it
    Hexer,       //lays a curse on the ground that makes the units in it take more harm
    Blinker,     //flickers, and is behind the army in a moment
    Sapper,      //lays mines where the army will walk
    Overseer,    //marks a champion, and a heavy blow lands where it was
}

//what makes an elite different from the rest of its kind
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Modifier {
    None,
    Swift,    //much faster
    Armored,  //shrugs off half the damage
    Volatile, //goes off when it is destroyed
    Frenzied, //hits twice as hard, and runs faster
    Captain,  //boss sized, leads one side of a pincer horde. not one of MODIFIERS: no ordinary enemy is ever made one
}

pub(crate) const MODIFIERS: [Modifier; 4] = [Modifier::Swift, Modifier::Armored, Modifier::Volatile, Modifier::Frenzied];

impl Modifier {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Modifier::None => "",
            Modifier::Swift => "Swift",
            Modifier::Armored => "Armored",
            Modifier::Volatile => "Volatile",
            Modifier::Frenzied => "Frenzied",
            Modifier::Captain => "Captain",
        }
    }

    pub(crate) fn color(self) -> Color {
        match self {
            Modifier::None => WHITE,
            Modifier::Swift => Color::new(0.4, 1.0, 1.0, 1.0),
            Modifier::Armored => Color::new(0.75, 0.78, 0.85, 1.0),
            Modifier::Volatile => Color::new(1.0, 0.55, 0.1, 1.0),
            Modifier::Frenzied => Color::new(1.0, 0.25, 0.3, 1.0),
            Modifier::Captain => Color::new(1.0, 0.8, 0.2, 1.0),
        }
    }
}

//what an enemy is in the middle of, when it is not just walking its route or running at the player
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Mode {
    Roam,
    Windup { dir: Vec2, left: f32 }, //standing still, showing where it is about to dash
    Dash { dir: Vec2, left: f32 },   //dashing that way, hitting much harder
    Recover { left: f32 },           //winded after a dash
    Fuse { left: f32 },              //a bomber that has touched the army, about to go off
    Cast { left: f32 },              //a boss rearing up for an attack, or between phases
    Lurk,                            //an ambusher lying hidden where it is, until the player comes close or a unit finds it
}

//a kind of enemy. the numbers are for an army too small for enemies to be scaled up
pub(crate) struct EnemyKind {
    pub(crate) name: &'static str,
    pub(crate) color: Color,
    pub(crate) size: f32,        //pixels along each side
    pub(crate) health: f32,
    pub(crate) babies: f32,      //baby units it drops when it is destroyed
    pub(crate) min_speed: f32,   //pixels per second, each enemy is given a random speed between these two
    pub(crate) max_speed: f32,
    pub(crate) weight: f32,      //how common it is next to the others
    pub(crate) from_army: usize, //it only turns up once the army has this many units
    pub(crate) attack: f32,      //damage per second it does to the units it touches, one after another
    pub(crate) aggro_tiles: f32, //it runs at the player once they are this many grid squares from its edge, 0 for one that never does
    pub(crate) range: f32,       //pixels, 0 for one that fights by touch, otherwise how far away it shoots or lobs shells at the units
    pub(crate) behavior: Behavior,
    pub(crate) shield: f32, //a shield this many times its health that soaks up damage first, and grows back when it is left alone
    pub(crate) pack: usize, //how many turn up together, 0 for one on its own
    pub(crate) shot: ShotKind, //what a shooter fires
}

pub(crate) static ENEMY_KINDS: [EnemyKind; 29] = [
    //the ordinary one
    EnemyKind { name: "Grunt", color: RED, size: 44.0, health: 100.0, babies: 1.0, min_speed: 60.0, max_speed: 120.0, weight: 6.0, from_army: 0, attack: 25.0, aggro_tiles: 7.0, range: 0.0, behavior: Behavior::Walker, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //small, fast and frail
    EnemyKind { name: "Scout", color: YELLOW, size: 30.0, health: 50.0, babies: 1.0, min_speed: 160.0, max_speed: 240.0, weight: 3.0, from_army: 0, attack: 12.0, aggro_tiles: 10.0, range: 0.0, behavior: Behavior::Walker, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //slow and tough, worth several babies
    EnemyKind { name: "Brute", color: Color::new(0.65, 0.25, 0.95, 1.0), size: 72.0, health: 450.0, babies: 4.0, min_speed: 40.0, max_speed: 80.0, weight: 3.0, from_army: 6, attack: 60.0, aggro_tiles: 6.0, range: 0.0, behavior: Behavior::Walker, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    EnemyKind { name: "Titan", color: Color::new(0.95, 0.5, 0.1, 1.0), size: 130.0, health: 2000.0, babies: 15.0, min_speed: 25.0, max_speed: 50.0, weight: 2.0, from_army: 25, attack: 160.0, aggro_tiles: 5.0, range: 0.0, behavior: Behavior::Walker, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    EnemyKind { name: "Colossus", color: Color::new(0.95, 0.2, 0.6, 1.0), size: 220.0, health: 9000.0, babies: 60.0, min_speed: 15.0, max_speed: 30.0, weight: 1.0, from_army: 100, attack: 420.0, aggro_tiles: 8.0, range: 0.0, behavior: Behavior::Walker, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //never runs at anyone, but shoots volleys at the units from a distance, shots that walls stop
    EnemyKind { name: "Spitter", color: Color::new(0.62, 0.42, 0.25, 1.0), size: 56.0, health: 150.0, babies: 2.0, min_speed: 50.0, max_speed: 90.0, weight: 3.0, from_army: 15, attack: 10.0, aggro_tiles: 0.0, range: 640.0, behavior: Behavior::Shooter, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //tiny, quick and everywhere at once: they come in packs, and a pack in a crowd is a lot of small bites
    EnemyKind { name: "Swarmer", color: Color::new(0.6, 0.9, 0.2, 1.0), size: 20.0, health: 20.0, babies: 0.5, min_speed: 170.0, max_speed: 260.0, weight: 2.0, from_army: 0, attack: 6.0, aggro_tiles: 12.0, range: 0.0, behavior: Behavior::Walker, shield: 0.0, pack: 10, shot: ShotKind::Plain },
    //hides behind a shield that has to be worn down first, and that grows back if the units let up
    EnemyKind { name: "Bulwark", color: Color::new(0.3, 0.55, 0.8, 1.0), size: 78.0, health: 260.0, babies: 4.0, min_speed: 35.0, max_speed: 65.0, weight: 2.0, from_army: 12, attack: 40.0, aggro_tiles: 5.0, range: 0.0, behavior: Behavior::Walker, shield: 1.6, pack: 0, shot: ShotKind::Plain },
    //destroyed, it breaks into three smaller ones that run at the player
    EnemyKind { name: "Splitter", color: Color::new(0.1, 0.75, 0.6, 1.0), size: 66.0, health: 220.0, babies: 2.0, min_speed: 50.0, max_speed: 90.0, weight: 2.0, from_army: 8, attack: 30.0, aggro_tiles: 7.0, range: 0.0, behavior: Behavior::Splitter, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //stands still and shows where it is going, then dashes that way and hits much harder for a moment
    EnemyKind { name: "Charger", color: Color::new(0.6, 0.18, 0.12, 1.0), size: 62.0, health: 320.0, babies: 5.0, min_speed: 40.0, max_speed: 70.0, weight: 3.0, from_army: 20, attack: 80.0, aggro_tiles: 12.0, range: 0.0, behavior: Behavior::Charger, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //runs at the army and blows up, taking some of it with it
    EnemyKind { name: "Bomber", color: Color::new(0.4, 0.4, 0.46, 1.0), size: 34.0, health: 70.0, babies: 1.0, min_speed: 150.0, max_speed: 220.0, weight: 3.0, from_army: 30, attack: 5.0, aggro_tiles: 16.0, range: 0.0, behavior: Behavior::Bomber, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //mends the enemies round it. only found with a warband, never on its own
    EnemyKind { name: "Healer", color: Color::new(0.6, 1.0, 0.7, 1.0), size: 40.0, health: 140.0, babies: 3.0, min_speed: 45.0, max_speed: 75.0, weight: 0.0, from_army: 25, attack: 4.0, aggro_tiles: 0.0, range: 0.0, behavior: Behavior::Healer, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //calls up imps while the player is near, and they die with it
    EnemyKind { name: "Summoner", color: Color::new(0.35, 0.25, 0.75, 1.0), size: 54.0, health: 200.0, babies: 4.0, min_speed: 35.0, max_speed: 60.0, weight: 2.0, from_army: 40, attack: 15.0, aggro_tiles: 0.0, range: 0.0, behavior: Behavior::Summoner, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //lobs shells at the units from far away, that land a moment later and hurt everything round where they land
    EnemyKind { name: "Mortar", color: Color::new(0.55, 0.6, 0.2, 1.0), size: 58.0, health: 180.0, babies: 3.0, min_speed: 30.0, max_speed: 55.0, weight: 2.0, from_army: 50, attack: 10.0, aggro_tiles: 0.0, range: 900.0, behavior: Behavior::Mortar, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //fires one fast bolt that goes straight through the units in its way, a lane to keep out of
    EnemyKind { name: "Lancer", color: Color::new(0.85, 0.92, 1.0, 1.0), size: 52.0, health: 170.0, babies: 3.0, min_speed: 45.0, max_speed: 80.0, weight: 2.0, from_army: 55, attack: 10.0, aggro_tiles: 0.0, range: 880.0, behavior: Behavior::Shooter, shield: 0.0, pack: 0, shot: ShotKind::Piercing },
    //lobs slow grenades that go off where they land and hurt everything round them
    EnemyKind { name: "Grenadier", color: Color::new(0.9, 0.55, 0.1, 1.0), size: 60.0, health: 200.0, babies: 3.0, min_speed: 40.0, max_speed: 70.0, weight: 2.0, from_army: 45, attack: 10.0, aggro_tiles: 0.0, range: 700.0, behavior: Behavior::Shooter, shield: 0.0, pack: 0, shot: ShotKind::Splash },
    //its orbs bend towards the nearest unit as they fly, so running does not shake them off
    EnemyKind { name: "Seeker", color: Color::new(0.92, 0.3, 0.72, 1.0), size: 36.0, health: 90.0, babies: 2.0, min_speed: 70.0, max_speed: 110.0, weight: 2.0, from_army: 70, attack: 8.0, aggro_tiles: 0.0, range: 620.0, behavior: Behavior::Shooter, shield: 0.0, pack: 0, shot: ShotKind::Homing },
    //lies hidden until the player comes close, then springs on them with a rush, together with the rest of its pack
    EnemyKind { name: "Ambusher", color: Color::new(0.36, 0.26, 0.46, 1.0), size: 40.0, health: 150.0, babies: 4.0, min_speed: 120.0, max_speed: 190.0, weight: 2.0, from_army: 30, attack: 36.0, aggro_tiles: 14.0, range: 0.0, behavior: Behavior::Ambusher, shield: 0.0, pack: 3, shot: ShotKind::Plain },
    //what the others make, never found by themselves
    EnemyKind { name: "Imp", color: Color::new(0.8, 0.3, 0.9, 1.0), size: 22.0, health: 30.0, babies: 0.0, min_speed: 150.0, max_speed: 230.0, weight: 0.0, from_army: 0, attack: 8.0, aggro_tiles: 20.0, range: 0.0, behavior: Behavior::Walker, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    EnemyKind { name: "Splitling", color: Color::new(0.35, 0.9, 0.75, 1.0), size: 34.0, health: 60.0, babies: 1.0, min_speed: 90.0, max_speed: 150.0, weight: 0.0, from_army: 0, attack: 18.0, aggro_tiles: 12.0, range: 0.0, behavior: Behavior::Walker, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    EnemyKind { name: "Drone", color: Color::new(0.5, 0.8, 1.0, 1.0), size: 28.0, health: 60.0, babies: 0.0, min_speed: 60.0, max_speed: 100.0, weight: 0.0, from_army: 0, attack: 6.0, aggro_tiles: 0.0, range: 520.0, behavior: Behavior::Shooter, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //what a frozen enemy breaks into when it is destroyed
    EnemyKind { name: "Shard", color: Color::new(0.75, 0.93, 1.0, 1.0), size: 26.0, health: 45.0, babies: 0.5, min_speed: 110.0, max_speed: 190.0, weight: 0.0, from_army: 0, attack: 14.0, aggro_tiles: 14.0, range: 0.0, behavior: Behavior::Walker, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //what a lair is built round: it never moves, and is only ever put in the world by a site
    EnemyKind { name: "Nest", color: Color::new(0.8, 0.25, 0.3, 1.0), size: 96.0, health: 700.0, babies: 12.0, min_speed: 0.0, max_speed: 0.0, weight: 0.0, from_army: 0, attack: 30.0, aggro_tiles: 0.0, range: 0.0, behavior: Behavior::Nest, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //the ones that teach a rule each (see enemy_ai.rs). a hooded brute that winds up a cleave that cuts down the units in front of it: step out of it
    EnemyKind { name: "Executioner", color: Color::new(0.55, 0.12, 0.15, 1.0), size: 70.0, health: 600.0, babies: 6.0, min_speed: 50.0, max_speed: 80.0, weight: 1.2, from_army: 300, attack: 30.0, aggro_tiles: 12.0, range: 0.0, behavior: Behavior::Executioner, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //shows a long thin line at the army, and fires down it
    EnemyKind { name: "Sniper", color: Color::new(0.45, 0.55, 0.45, 1.0), size: 46.0, health: 140.0, babies: 3.0, min_speed: 30.0, max_speed: 50.0, weight: 1.2, from_army: 400, attack: 8.0, aggro_tiles: 0.0, range: 1500.0, behavior: Behavior::Sniper, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //curses a patch of the ground, and the units in it take more harm: leave it
    EnemyKind { name: "Hexer", color: Color::new(0.55, 0.2, 0.75, 1.0), size: 48.0, health: 180.0, babies: 3.0, min_speed: 40.0, max_speed: 60.0, weight: 1.2, from_army: 500, attack: 8.0, aggro_tiles: 0.0, range: 1000.0, behavior: Behavior::Hexer, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //flickers, and is behind the army: turn and fight it
    EnemyKind { name: "Blinker", color: Color::new(0.45, 0.4, 1.0, 1.0), size: 40.0, health: 160.0, babies: 3.0, min_speed: 120.0, max_speed: 170.0, weight: 1.2, from_army: 800, attack: 30.0, aggro_tiles: 12.0, range: 0.0, behavior: Behavior::Blinker, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //lays faint mines where the army walks, that go off a moment after it steps on them: walk round them
    EnemyKind { name: "Sapper", color: Color::new(0.6, 0.5, 0.3, 1.0), size: 44.0, health: 150.0, babies: 3.0, min_speed: 70.0, max_speed: 100.0, weight: 1.2, from_army: 250, attack: 10.0, aggro_tiles: 10.0, range: 0.0, behavior: Behavior::Sapper, shield: 0.0, pack: 0, shot: ShotKind::Plain },
    //marks a champion, and a heavy blow lands where it was: move the champion
    EnemyKind { name: "Overseer", color: Color::new(0.15, 0.45, 0.5, 1.0), size: 60.0, health: 300.0, babies: 5.0, min_speed: 30.0, max_speed: 50.0, weight: 1.0, from_army: 1000, attack: 8.0, aggro_tiles: 0.0, range: 1300.0, behavior: Behavior::Overseer, shield: 0.0, pack: 0, shot: ShotKind::Plain },
];

//the kind with this name
pub(crate) fn kind_named(name: &str) -> &'static EnemyKind {
    ENEMY_KINDS.iter().find(|kind| kind.name == name).unwrap_or(&ENEMY_KINDS[0])
}

//a random kind of enemy out of those that turn up for an army this big, the common ones more often. the ones with no
//weight are never picked, they only come with something else
pub(crate) fn random_kind(army: usize) -> &'static EnemyKind {
    let kinds: Vec<&EnemyKind> = ENEMY_KINDS.iter().filter(|kind| army >= kind.from_army && kind.weight > 0.0).collect();
    let mut pick = rand::gen_range(0.0, kinds.iter().map(|kind| kind.weight).sum::<f32>());
    for kind in &kinds {
        if pick < kind.weight {
            return kind;
        }
        pick -= kind.weight;
    }
    kinds[kinds.len() - 1]
}

pub(crate) static NEXT_ENEMY_ID: AtomicU32 = AtomicU32::new(0);

//a square that patrols a random route near where it started. the player and the units walk straight through it, but
//a unit inside it hurts it, and when it is destroyed it drops baby units
#[derive(Clone)]
pub(crate) struct Enemy {
    pub(crate) id: u32,        //different for every enemy ever made, so a squad can keep track of the one it was sent at
    pub(crate) name: &'static str,
    pub(crate) position: Vec2, //its middle
    pub(crate) size: f32,      //pixels along each side
    pub(crate) color: Color,
    pub(crate) max_health: f32,
    pub(crate) health: f32,
    pub(crate) babies: usize,      //how many baby units it drops
    pub(crate) speed: f32,         //pixels per second
    pub(crate) route: Vec<Vec2>,   //the points it walks to in turn, over and over
    pub(crate) heading_for: usize, //which of them it is walking to
    pub(crate) attack: f32,        //damage per second it does to the units it touches, one after another
    pub(crate) aggro: f32,         //pixels from it the player has to be to be noticed, 0 if it never notices them
    pub(crate) chasing: bool,      //whether it has noticed the player and is running at them
    pub(crate) range: f32,         //pixels it shoots or lobs shells from, 0 if it fights by touch
    pub(crate) volley: usize,      //shots it fires at once
    pub(crate) shot_in: f32,       //seconds until it shoots again
    pub(crate) hurt: f32,          //damage it has taken since it last showed a number for it
    pub(crate) killed: usize,      //units it has killed since it last showed a number for it
    pub(crate) popup_in: f32,      //seconds until it shows them
    pub(crate) behavior: Behavior,
    pub(crate) modifier: Modifier,
    pub(crate) scale: f32,         //how many times bigger than its kind it was made, what it makes is scaled from this
    pub(crate) owner: Option<u32>, //the enemy that made it, which it dies with. what is made is no kill and drops no babies
    pub(crate) armor: f32,         //the share of the damage it takes that it shrugs off
    pub(crate) shield: f32,        //damage it soaks up before its health is touched
    pub(crate) max_shield: f32,
    pub(crate) shield_idle: f32,   //seconds since it was last hurt, a shield grows back after being left alone
    pub(crate) mode: Mode,
    pub(crate) timer: f32,         //seconds until its ability goes off next
    pub(crate) flash: f32,         //seconds since its ability last went off, for drawing
    pub(crate) blew_up: bool,      //it set itself off, which is neither a kill nor a drop
    pub(crate) boss: Option<BossState>,
    pub(crate) site: Option<u32>,  //the site (a lair, a village) it belongs to
    pub(crate) steer: Steer,       //where it is heading, worked out every STEERING_INTERVAL
    pub(crate) steer_in: f32,
    pub(crate) stuck: f32,         //seconds it has been held up by a wall
    pub(crate) pressing: Option<(i32, i32)>, //the wall it is pushing against and chewing at
    pub(crate) status: Statuses,   //what is being done to it: chill, a freeze, burning, a shock, poison, and the share of its speed the ground leaves it (see status.rs)
    pub(crate) provoked: f32,      //seconds it is still fighting back at the units that hurt it, not kept in a save
    pub(crate) grudge: Vec2,       //where those units were, which it goes to and fights them at
    pub(crate) rout: f32,          //seconds it is still running from the army, after its captain fell: it does nothing else. not kept in a save
    pub(crate) rallied: bool,      //a living captain is rallying it: it is faster and hits harder. worked out every frame, not kept in a save
    pub(crate) credit: u8,         //the player whose army has hurt it most lately, who is given the babies and the rewards when it is destroyed. not kept in a save
    pub(crate) struck: f32,        //seconds left of the white it shows after a bolt or a champion's power hit it, for drawing. not kept in a save
}

impl Enemy {
    //an enemy of a kind, with every number nudged a little so no two are the same, and everything but the speed
    //scaled up for a big army
    pub(crate) fn new(start: Vec2, kind: &EnemyKind, scale: f32) -> Enemy {
        let size = kind.size * scale * rand::gen_range(0.85, 1.2);
        let health = kind.health * scale * scale * rand::gen_range(0.8, 1.3);
        let shade = rand::gen_range(0.8, 1.0);
        let edge = size / 2.0;
        let start = start.clamp(Vec2::splat(edge), Vec2::splat(WORLD_SIZE - edge));
        let expected_babies = kind.babies * scale * rand::gen_range(0.8, 1.25);
        //one that is worth a fraction of a baby drops one now and then, so a pack of them is worth the sum
        let babies = if kind.babies >= 1.0 {
            expected_babies.round().max(1.0) as usize
        } else {
            expected_babies.floor() as usize + (rand::gen_range(0.0, 1.0) < expected_babies.fract()) as usize
        };
        Enemy {
            id: NEXT_ENEMY_ID.fetch_add(1, Ordering::Relaxed),
            name: kind.name,
            position: start,
            size,
            color: Color::new(kind.color.r * shade, kind.color.g * shade, kind.color.b * shade, 1.0),
            max_health: health,
            health,
            babies,
            speed: rand::gen_range(kind.min_speed, kind.max_speed),
            route: patrol_route(start, size),
            heading_for: 0,
            //hits harder the bigger the army, but only as fast as its size grows and not its health, which grows as the
            //square of that. a squad sized to destroy it in a couple of seconds loses fewer units, as a share of the squad,
            //the bigger the army is, while the babies it drops grow with its size
            attack: kind.attack * scale * rand::gen_range(0.85, 1.15),
            aggro: if kind.aggro_tiles > 0.0 { kind.aggro_tiles * TILE_SIZE + size / 2.0 } else { 0.0 },
            chasing: false,
            range: kind.range,
            volley: if kind.behavior == Behavior::Shooter { (scale.ceil() as usize).clamp(1, MAX_VOLLEY) } else { 0 },
            shot_in: rand::gen_range(0.3, SHOT_INTERVAL),
            hurt: 0.0,
            killed: 0,
            popup_in: rand::gen_range(0.0, POPUP_INTERVAL),
            behavior: kind.behavior,
            modifier: Modifier::None,
            scale,
            owner: None,
            armor: 0.0,
            shield: health * kind.shield,
            max_shield: health * kind.shield,
            shield_idle: 0.0,
            mode: if kind.behavior == Behavior::Ambusher { Mode::Lurk } else { Mode::Roam },
            timer: rand::gen_range(0.3, 1.5),
            flash: 10.0,
            blew_up: false,
            boss: None,
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

    //something another enemy made, a smaller one, that dies with it and is worth nothing
    pub(crate) fn minion(start: Vec2, kind: &EnemyKind, scale: f32, owner: u32) -> Enemy {
        let mut minion = Enemy::new(start, kind, minion_scale(scale));
        minion.owner = Some(owner);
        minion.babies = 0;
        minion
    }

    pub(crate) fn is_minion(&self) -> bool {
        self.owner.is_some()
    }

    //its kind's name with the modifier in front of it
    pub(crate) fn title(&self) -> String {
        if self.modifier == Modifier::None {
            self.name.to_owned()
        } else {
            format!("{} {}", self.modifier.label(), self.name)
        }
    }

    //make it tougher and harder hitting, and worth more, for an army past the size where enemies stop growing
    pub(crate) fn empower(&mut self, might: f32) {
        self.empower_by(might.powf(ENEMY_MIGHT_HEALTH), might.powf(ENEMY_MIGHT_ATTACK), might.powf(ENEMY_MIGHT_BABIES));
    }

    //the same by the numbers it is given: for health (and the shield, which goes with it), attack, and babies
    pub(crate) fn empower_by(&mut self, health: f32, attack: f32, babies: f32) {
        self.max_health *= health;
        self.health = self.max_health;
        self.max_shield *= health;
        self.shield = self.max_shield;
        self.attack *= attack;
        self.babies = (self.babies as f32 * babies).round() as usize;
    }

    //make it an elite: bigger, tougher, worth more babies, and different in the way the modifier says
    pub(crate) fn make_elite(&mut self, modifier: Modifier) {
        self.modifier = modifier;
        self.size *= 1.12;
        self.max_health *= 1.5;
        self.health = self.max_health;
        self.max_shield *= 1.5;
        self.shield = self.max_shield;
        self.babies = (self.babies as f32 * 2.5).round() as usize;
        match modifier {
            Modifier::None | Modifier::Volatile | Modifier::Captain => {}
            Modifier::Swift => self.speed *= 1.7,
            Modifier::Armored => self.armor = 0.5,
            Modifier::Frenzied => {
                self.attack *= 2.0;
                self.speed *= 1.25;
            }
        }
    }

    //make it the captain of a horde made for an army this big: as big as a boss is for it, much tougher, harder hitting and worth a lot,
    //and never left behind by the crowd it leads
    pub(crate) fn make_captain(&mut self, army: usize) {
        self.modifier = Modifier::Captain;
        self.size = CAPTAIN_SIZE_BASE + CAPTAIN_SIZE_GROWTH * (army.max(1) as f32).sqrt();
        self.max_health *= CAPTAIN_HEALTH;
        self.health = self.max_health;
        self.max_shield *= CAPTAIN_HEALTH;
        self.shield = self.max_shield;
        self.attack *= CAPTAIN_ATTACK;
        self.babies = (self.babies as f32 * CAPTAIN_BABIES).round() as usize;
        self.armor = self.armor.max(CAPTAIN_ARMOR);
        self.speed = self.speed.max(CAPTAIN_MIN_SPEED);
    }

    pub(crate) fn is_captain(&self) -> bool {
        self.modifier == Modifier::Captain
    }

    //take damage: armor shrugs off a share of it, a shield soaks up the rest before health is touched, and a boss between
    //phases takes none. returns how much it took
    pub(crate) fn take_damage(&mut self, damage: f32) -> f32 {
        if self.boss.as_ref().is_some_and(|boss| boss.shifting > 0.0) || self.lurking() {
            return 0.0;
        }
        let mut damage = damage * (1.0 - self.armor);
        if damage <= 0.0 {
            return 0.0;
        }
        self.shield_idle = 0.0;
        let mut taken = 0.0;
        if self.shield > 0.0 {
            let soaked = damage.min(self.shield);
            self.shield -= soaked;
            damage -= soaked;
            taken = soaked;
        }
        //a boss stops at the health of a change it has yet to make, however hard it is hit: the change is made a moment later,
        //and what it was held from is not damage done
        let held = self.boss_gate().min(self.health);
        if self.health - damage < held {
            taken += self.health - held;
            self.health = held;
        } else {
            taken += damage;
            self.health -= damage;
        }
        taken
    }

    //the least health a boss can be brought to for now: the health at which it makes its next change (enter a phase, split in two,
    //make its last stand), a little below the share that sets the change off, so that it does. nothing for one with none left to make
    pub(crate) fn boss_gate(&self) -> f32 {
        let Some(state) = self.boss.as_ref() else {
            return f32::NEG_INFINITY;
        };
        let mut next = state.def.phases.get(state.phase + 1..).unwrap_or_default().iter().map(|phase| phase.below).fold(0.0f32, f32::max);
        if state.def.splits && (state.generation as usize) < SPLIT_AT.len() {
            next = next.max(SPLIT_AT[state.generation as usize]);
        }
        if !state.last_stand {
            next = next.max(LAST_STAND_AT);
        }
        if next <= 0.0 {
            return f32::NEG_INFINITY;
        }
        self.max_health * next * BOSS_GATE_MARGIN
    }

    //whether it is an ambusher still lying in wait: it can't be seen, clicked, hurt or frozen, and does nothing until it springs
    pub(crate) fn lurking(&self) -> bool {
        self.mode == Mode::Lurk
    }

    //an ambusher springs: it is out in the open, after the player, and its first rush at them hits as hard as a dash
    pub(crate) fn spring(&mut self, toward: Vec2, fx: &mut Effects) {
        if !self.lurking() {
            return;
        }
        let dir = (toward - self.position).normalize_or_zero();
        self.mode = Mode::Dash { dir: if dir == Vec2::ZERO { Vec2::X } else { dir }, left: AMBUSH_LUNGE_TIME };
        self.chasing = true;
        self.flash = 0.0;
        fx.popup(self.position, self.size / 2.0, 0.0, "AMBUSH!".to_owned(), danger_color(1.0));
        fx.burst(self.position, self.color, 10, 240.0);
        fx.cue_at(Cue::Alert, self.position);
    }

    //hurt by units: it is provoked, and fights them where they are for a while. returns whether this is the first it has heard of
    //them (it was not provoked already), which is when it calls the enemies near it too
    pub(crate) fn provoke(&mut self, at: Vec2) -> bool {
        let fresh = self.provoked <= 0.0;
        self.provoked = RETALIATE_SECONDS;
        self.grudge = at;
        fresh
    }

    //whether it can be provoked: the ones that move and have a mind of their own, not a boss (it is after the army anyway), a nest
    //or something that is held frozen or lying in wait
    pub(crate) fn can_be_provoked(&self) -> bool {
        self.boss.is_none() && self.behavior != Behavior::Nest && !self.lurking() && self.status.frozen <= 0.0
    }

    //whether a Freezing relic can hold it: not a boss, and not a nest that never moves
    pub(crate) fn can_freeze(&self) -> bool {
        self.boss.is_none() && self.behavior != Behavior::Nest && !self.lurking()
    }

    //whether it breaks into shards when it is destroyed: it is frozen, and was not itself made by another enemy or broken off one
    pub(crate) fn shatters(&self) -> bool {
        self.status.frozen > 0.0 && self.can_freeze() && !self.is_minion() && self.name != "Shard"
    }

    //damage per second to the units it touches: a dash hits much harder, a rallied one hits harder, and a frozen or routed one does none
    pub(crate) fn attack_now(&self) -> f32 {
        if self.status.frozen > 0.0 || self.routed() {
            return 0.0;
        }
        let attack = if self.rallied { self.attack * RALLY_ATTACK } else { self.attack };
        match self.mode {
            Mode::Dash { .. } => attack * CHARGE_DAMAGE,
            _ => attack,
        }
    }

    //how much of its speed it is going at: tar, an aura and chill slow it, and a captain's rally quickens it
    pub(crate) fn pace(&self) -> f32 {
        speed_kept(self) * if self.rallied { RALLY_SPEED } else { 1.0 }
    }

    //whether it is running from the army, its captain having fallen
    pub(crate) fn routed(&self) -> bool {
        self.rout > 0.0
    }

    //its captain has fallen: it runs from the army for a while. a dash or a wind up it was in the middle of comes to nothing
    pub(crate) fn break_and_run(&mut self) {
        self.rout = ROUT_SECONDS;
        self.rallied = false;
        if matches!(self.mode, Mode::Windup { .. } | Mode::Dash { .. } | Mode::Recover { .. } | Mode::Cast { .. }) {
            self.mode = Mode::Roam;
        }
    }

    //whether it is awake and a danger to be near, which is what keeps a chest from being held: not one that is hiding, held solid or running away
    pub(crate) fn contests(&self) -> bool {
        self.health > 0.0 && !self.lurking() && self.status.frozen <= 0.0 && !self.routed()
    }

    //show the damage it has taken and the units it has killed since it last did as numbers floating up from it
    pub(crate) fn show_numbers(&mut self, fx: &mut Effects) {
        if self.hurt >= 1.0 {
            fx.popup(self.position, self.size / 2.0, 0.0, format!("-{}", compact_number(self.hurt)), Color::new(1.0, 0.9, 0.4, 1.0));
        }
        if self.killed > 0 {
            fx.popup(self.position, self.size / 2.0, 26.0, format!("-{} units", self.killed), Color::new(1.0, 0.35, 0.35, 1.0));
        }
        self.hurt = 0.0;
        self.killed = 0;
    }

    //move it, and the route it patrols, by a step. for a group that has just been made and is put somewhere else
    pub(crate) fn shift(&mut self, step: Vec2) {
        let edge = Vec2::splat(self.size / 2.0);
        let inside = |point: Vec2| (point + step).clamp(edge, Vec2::splat(WORLD_SIZE) - edge);
        self.position = inside(self.position);
        for point in &mut self.route {
            *point = inside(*point);
        }
    }

    //whether a circle touches the square
    pub(crate) fn overlaps_circle(&self, x: f32, y: f32, radius: f32) -> bool {
        let half = self.size / 2.0;
        let center = Vec2::new(x, y);
        let nearest = center.clamp(self.position - Vec2::splat(half), self.position + Vec2::splat(half));
        nearest.distance_squared(center) < radius * radius
    }

    //how big a circle round its middle keeps it out of walls: as big as it is, up to a limit, a giant crushes walls instead
    pub(crate) fn wall_radius(&self) -> f32 {
        (self.size / 2.0).clamp(6.0, ENEMY_WALL_RADIUS_MAX)
    }

    //move by a step, kept out of the walls that stop enemies. returns how much of the step it managed, 0 to 1. one that
    //`chew`s and gets held up is left pressing on the wall in its way, which `chew_walls` wears down
    pub(crate) fn advance(&mut self, step: Vec2, dt: f32, chew: bool, walls: &Walls) -> f32 {
        let wanted = step.length();
        if wanted <= 0.0 {
            self.stuck = 0.0;
            return 1.0;
        }
        let edge = self.size / 2.0;
        let before = self.position;
        let moved = move_clear(before, before + step, self.wall_radius(), Mover::Enemy, walls);
        self.position = moved.clamp(Vec2::splat(edge), Vec2::splat(WORLD_SIZE - edge));
        let direction = step / wanted;
        let got = ((self.position - before).dot(direction) / wanted).clamp(0.0, 1.0);
        if got < 0.4 {
            self.stuck += dt;
        } else {
            self.stuck = 0.0;
        }
        self.pressing = if chew && got < 0.5 { self.wall_in_the_way(direction, walls) } else { None };
        got
    }

    //the wall that stops it going a direction: the one closest to a point just in front of it that it is touching
    pub(crate) fn wall_in_the_way(&self, direction: Vec2, walls: &Walls) -> Option<(i32, i32)> {
        let radius = self.wall_radius();
        let front = self.position + direction * (radius + 4.0);
        let (col, row) = tile_of(front.x, front.y);
        let mut best = None;
        let mut best_distance = f32::MAX;
        for wall_col in (col - 1)..=(col + 1) {
            for wall_row in (row - 1)..=(row + 1) {
                let Some(wall) = walls.get(&(wall_col, wall_row)).filter(|wall| wall.blocks(Mover::Enemy)) else {
                    continue;
                };
                let distance = wall.nearest_point(front.x, front.y).distance(front);
                if distance < best_distance && wall.overlaps_circle(self.position.x, self.position.y, radius + 6.0) {
                    best_distance = distance;
                    best = Some((wall_col, wall_row));
                }
            }
        }
        best
    }

    //carry on with a dash, a wind up or a cast that has begun. returns whether it is busy with one, which keeps it from
    //walking or running at anyone. a dash that runs into a wall is over: it hits the wall hard and is dazed
    pub(crate) fn carry_on(&mut self, dt: f32, speed_multiplier: f32, walls: &Walls, fx: &mut Effects) -> bool {
        match self.mode {
            Mode::Roam => return false,
            Mode::Windup { dir, left } => {
                self.mode = if left - dt > 0.0 {
                    Mode::Windup { dir, left: left - dt }
                } else {
                    fx.cue_at(Cue::Charge, self.position);
                    Mode::Dash { dir, left: CHARGE_TIME }
                };
            }
            Mode::Dash { dir, left } => {
                let got = self.advance(dir * CHARGE_SPEED * speed_multiplier * dt, dt, true, walls);
                self.mode = if got < 0.5 {
                    if let Some(tile) = self.pressing {
                        fx.wall_hits.push((tile, self.attack * CHARGE_DAMAGE * CHARGE_CRASH_SHARE));
                    }
                    fx.burst(self.position + dir * self.size * 0.5, self.color, 12, 260.0);
                    fx.add_shake(0.15);
                    fx.cue_at(Cue::Thud, self.position);
                    Mode::Recover { left: CHARGE_RECOVER * 1.4 }
                } else if left - dt > 0.0 {
                    Mode::Dash { dir, left: left - dt }
                } else {
                    Mode::Recover { left: CHARGE_RECOVER }
                };
            }
            Mode::Recover { left } => self.mode = if left - dt > 0.0 { Mode::Recover { left: left - dt } } else { Mode::Roam },
            Mode::Cast { left } => self.mode = if left - dt > 0.0 { Mode::Cast { left: left - dt } } else { Mode::Roam },
            //a bomber stands still while its fuse burns, enemy_abilities counts it down
            Mode::Fuse { .. } | Mode::Lurk => {}
        }
        true
    }
}

//what an enemy makes is smaller than it is: its scale to the power MINION_SCALE_POWER, never less than 1
pub(crate) fn minion_scale(scale: f32) -> f32 {
    scale.powf(MINION_SCALE_POWER).max(1.0)
}

//how much further a bomb or a shell reaches for an enemy of a scale
pub(crate) fn blast_scale(scale: f32) -> f32 {
    scale.powf(0.35).max(1.0)
}

//how many units something that hurts a share of the army hurts at the most, but never fewer than three
pub(crate) fn victims_of(army: usize, share: f32) -> usize {
    ((army as f32 * share).ceil() as usize).max(3)
}

//every so often each enemy shows the damage it has taken and the units it has killed as numbers
pub(crate) fn tick_enemy_numbers(enemies: &mut [Enemy], dt: f32, fx: &mut Effects) {
    for enemy in enemies {
        enemy.struck = (enemy.struck - dt).max(0.0);
        enemy.popup_in -= dt;
        if enemy.popup_in <= 0.0 {
            enemy.show_numbers(fx);
            enemy.popup_in = POPUP_INTERVAL;
        }
    }
}

//a colour moved part of the way to white
pub(crate) fn lighten(color: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color::new(color.r + (1.0 - color.r) * amount, color.g + (1.0 - color.g) * amount, color.b + (1.0 - color.b) * amount, color.a)
}

//the plain way to draw enemies, for the ones who would rather not have the pictures (the setting Simple shapes): squares in their colour with a bar over them once
//they are hurt, thicker on a big one. one that a
//squad has been sent at has a pulsing outline. what sets one apart is drawn on it: a charger's lane while it winds up, a
//shield, armor, an elite's mark, a bomber's fuse and blast, the ring of a healer's pulse or a summoner's call, and a boss's
//aura and eyes. `time` is in seconds, and `speed_multiplier` is the army's, which the length of a dash depends on
pub(crate) fn draw_enemies_plain(enemies: &[Enemy], squads: &Squads, time: f32, speed_multiplier: f32) {
    for enemy in enemies {
        let half = enemy.size / 2.0;
        let (x, y) = (enemy.position.x - half, enemy.position.y - half);
        if enemy.lurking() {
            //nothing to see but a patch of disturbed ground, for whoever looks
            let pulse = 0.65 + 0.35 * (time * 2.0 + enemy.position.x * 0.013).sin();
            draw_circle(enemy.position.x, enemy.position.y, half * 1.3, Color::new(0.04, 0.02, 0.07, AMBUSH_TELL * pulse));
            draw_circle_lines(enemy.position.x, enemy.position.y, half * 1.3, 2.0, Color::new(0.55, 0.4, 0.7, AMBUSH_TELL * 0.8 * pulse));
            continue;
        }
        if let Mode::Windup { dir, left } = enemy.mode {
            let urgency = (1.0 - left / CHARGE_WINDUP).clamp(0.0, 1.0);
            let end = enemy.position + dir * CHARGE_SPEED * speed_multiplier * CHARGE_TIME;
            draw_line(enemy.position.x, enemy.position.y, end.x, end.y, enemy.size, Color::new(1.0, 0.2, 0.15, 0.10 + 0.22 * urgency));
            draw_line(enemy.position.x, enemy.position.y, end.x, end.y, 4.0, Color::new(1.0, 0.45, 0.3, 0.7));
        }
        let mut color = lighten(enemy.color, ((1.0 - enemy.flash / 0.15) * 0.6).max(enemy.struck / HIT_FLASH * 0.7));
        if matches!(enemy.mode, Mode::Fuse { .. }) && (time * 24.0).sin() > 0.0 {
            color = WHITE;
        }
        if enemy.status.frozen > 0.0 {
            color = Color::new(color.r * 0.3 + ICE.r * 0.7, color.g * 0.3 + ICE.g * 0.7, color.b * 0.3 + ICE.b * 0.7, 1.0);
        }
        if enemy.boss.as_ref().is_some_and(|boss| boss.shifting > 0.0) && (time * 30.0).sin() > 0.0 {
            color = lighten(color, 0.7);
        }
        //a blinker about to go flickers
        if enemy.behavior == Behavior::Blinker && enemy.timer > 0.0 && enemy.timer < BLINK_TELL && (time * 34.0).sin() > 0.0 {
            color = lighten(color, 0.8);
        }
        //a captain's rally reaches this far, drawn under it so the crowd can be seen to be inside it
        if enemy.is_captain() {
            let reach = RALLY_TILES * TILE_SIZE + half;
            let pulse = 0.5 + 0.5 * (time * 2.5).sin();
            let gold = enemy.modifier.color();
            draw_circle(enemy.position.x, enemy.position.y, reach, Color::new(gold.r, gold.g, gold.b, 0.03 + 0.02 * pulse));
            draw_circle_lines(enemy.position.x, enemy.position.y, reach, 3.0, Color::new(gold.r, gold.g, gold.b, 0.18 + 0.14 * pulse));
        }
        //running from the army: washed out
        if enemy.routed() {
            color = lighten(color, 0.55);
        }
        draw_rectangle(x, y, enemy.size, enemy.size, color);
        if enemy.status.frozen > 0.0 {
            //frosted over: a white edge and a glint across it
            let width = (enemy.size * 0.06).clamp(3.0, 16.0);
            draw_rectangle_lines(x, y, enemy.size, enemy.size, width, WHITE);
            draw_line(x + enemy.size * 0.2, y + enemy.size * 0.8, x + enemy.size * 0.8, y + enemy.size * 0.2, width * 0.6, Color::new(1.0, 1.0, 1.0, 0.7));
        }
        if enemy.armor > 0.0 {
            let width = (enemy.size * 0.05).clamp(3.0, 14.0);
            draw_rectangle_lines(x + width * 0.5, y + width * 0.5, enemy.size - width, enemy.size - width, width, Color::new(0.8, 0.82, 0.9, 0.9));
        }
        if enemy.routed() {
            //running away: a white outline in place of the red one
            let width = (enemy.size * 0.03).clamp(2.0, 12.0);
            draw_rectangle_lines(x, y, enemy.size, enemy.size, width, WHITE);
        } else if enemy.chasing && enemy.boss.is_none() {
            //running at the player: a red outline, thinner than the squad's
            let width = (enemy.size * 0.03).clamp(2.0, 12.0);
            draw_rectangle_lines(x, y, enemy.size, enemy.size, width, RED);
        }
        if enemy.rallied {
            //rallied by a captain: a gold edge inside the red one
            let width = (enemy.size * 0.025).clamp(2.0, 10.0);
            let gold = Modifier::Captain.color();
            let inset = (enemy.size * 0.03).clamp(2.0, 12.0) + width * 0.5;
            draw_rectangle_lines(x + inset, y + inset, enemy.size - inset * 2.0, enemy.size - inset * 2.0, width, Color::new(gold.r, gold.g, gold.b, 0.95));
        }
        if enemy.provoked > 0.0 && !enemy.chasing && enemy.boss.is_none() {
            //fighting back at the units that hurt it: an orange outline, and the shorter it has left the fainter
            let width = (enemy.size * 0.03).clamp(2.0, 12.0);
            let fade = (enemy.provoked / RETALIATE_SECONDS).clamp(0.2, 1.0);
            draw_rectangle_lines(x, y, enemy.size, enemy.size, width, Color::new(1.0, 0.62, 0.15, 0.9 * fade));
        }
        if enemy.modifier != Modifier::None {
            let width = (enemy.size * 0.04).clamp(3.0, 14.0);
            let pulse = 0.7 + 0.3 * (time * 6.0).sin();
            let mark = enemy.modifier.color();
            draw_rectangle_lines(x - width, y - width, enemy.size + width * 2.0, enemy.size + width * 2.0, width, Color::new(mark.r, mark.g, mark.b, pulse));
        }
        if enemy.is_captain() {
            //a crown of three points on its top edge, so it can be told from far off in a crowd
            let crown = enemy.modifier.color();
            let spike = enemy.size * 0.09;
            for k in 0..3 {
                let base = x + enemy.size * (0.2 + 0.3 * k as f32);
                let tip = if k == 1 { 2.4 } else { 1.7 };
                draw_triangle(Vec2::new(base - spike, y), Vec2::new(base + spike, y), Vec2::new(base, y - spike * tip), BLACK);
                draw_triangle(Vec2::new(base - spike * 0.8, y), Vec2::new(base + spike * 0.8, y), Vec2::new(base, y - spike * (tip - 0.3)), crown);
            }
        }
        if enemy.shield > 0.0 {
            let share = (enemy.shield / enemy.max_shield.max(1.0)).clamp(0.0, 1.0);
            draw_rectangle_lines(x - 10.0, y - 10.0, enemy.size + 20.0, enemy.size + 20.0, 4.0, Color::new(0.4, 0.85, 1.0, 0.3 + 0.6 * share));
        }
        match enemy.behavior {
            Behavior::Healer if enemy.flash < 0.6 => {
                let age = enemy.flash / 0.6;
                draw_circle_lines(enemy.position.x, enemy.position.y, HEAL_RANGE * age, 4.0, Color::new(0.4, 1.0, 0.5, 0.6 * (1.0 - age)));
            }
            Behavior::Summoner if enemy.flash < 0.5 => {
                let age = enemy.flash / 0.5;
                draw_circle_lines(enemy.position.x, enemy.position.y, half + 20.0 + 160.0 * age, 4.0, Color::new(0.7, 0.4, 1.0, 0.7 * (1.0 - age)));
            }
            Behavior::Bomber if matches!(enemy.mode, Mode::Fuse { .. }) => {
                let pulse = 0.5 + 0.5 * (time * 24.0).sin();
                draw_circle_lines(enemy.position.x, enemy.position.y, BLAST_RADIUS * blast_scale(enemy.scale) + half, 3.0, danger_color(0.4 + 0.5 * pulse));
            }
            _ => {}
        }
        if let Some(state) = &enemy.boss {
            let pulse = 0.5 + 0.5 * (time * if state.last_stand { 10.0 } else { 3.0 }).sin();
            let width = (enemy.size * 0.02).clamp(6.0, 30.0);
            let aura = if state.last_stand { RED } else { enemy.color };
            draw_rectangle_lines(x - width, y - width, enemy.size + width * 2.0, enemy.size + width * 2.0, width, Color::new(aura.r, aura.g, aura.b, 0.3 + 0.35 * pulse));
            //a ring round it that fills in the colour of the circle it will put down next, and flashes when that is close
            if let Some(next) = state.next_circle() {
                let tint = next.circle.color();
                let radius = enemy.size * 0.5 * std::f32::consts::SQRT_2 + width * 3.0 + 14.0;
                let thickness = (enemy.size * 0.025).clamp(4.0, 20.0);
                let soon = if next.seconds <= 1.5 { 0.5 + 0.5 * (time * 14.0).sin() } else { 0.0 };
                draw_circle_lines(enemy.position.x, enemy.position.y, radius, thickness * 0.5, Color::new(tint.r, tint.g, tint.b, 0.2));
                draw_sweep(enemy.position, radius, -std::f32::consts::FRAC_PI_2, std::f32::consts::TAU * next.share, thickness, Color::new(tint.r, tint.g, tint.b, 0.75 + 0.25 * soon));
            }
            //an armoured front is a bright arc on the side it is facing
            if state.def.flank {
                let radius = enemy.size * 0.62;
                let width = (enemy.size * 0.04).clamp(5.0, 20.0);
                let steps = 16;
                let point = |k: i32| enemy.position + state.facing.rotate(Vec2::from_angle(-FLANK_ARC + 2.0 * FLANK_ARC * k as f32 / steps as f32)) * radius;
                for k in 0..steps {
                    let (from, to) = (point(k), point(k + 1));
                    draw_line(from.x, from.y, to.x, to.y, width, Color::new(0.75, 0.9, 1.0, 0.9));
                }
            }
            //two eyes, which glow red when it makes its last stand
            let eye = enemy.size * 0.14;
            for side in [-1.0, 1.0] {
                let at = Vec2::new(enemy.position.x + side * enemy.size * 0.22, enemy.position.y - enemy.size * 0.1);
                draw_rectangle(at.x - eye / 2.0, at.y - eye / 2.0, eye, eye, WHITE);
                draw_rectangle(at.x - eye / 4.0, at.y - eye / 4.0, eye / 2.0, eye / 2.0, if state.last_stand { RED } else { BLACK });
            }
        }
        if squads.contains_key(&enemy.id) {
            let pulse = 0.5 + 0.5 * (get_time() as f32 * 8.0).sin();
            let width = (enemy.size * 0.06).clamp(4.0, 24.0);
            let outline = Color::new(SQUAD_COLOR.r, SQUAD_COLOR.g, SQUAD_COLOR.b, 0.5 + 0.5 * pulse);
            draw_rectangle_lines(x - width, y - width, enemy.size + width * 2.0, enemy.size + width * 2.0, width, outline);
        }
        if enemy.health < enemy.max_health || enemy.shield > 0.0 {
            let bar_height = (enemy.size * 0.1).clamp(6.0, 40.0);
            let bar_y = enemy.position.y - half - bar_height * 2.0;
            draw_rectangle(x, bar_y, enemy.size, bar_height, Color::new(0.0, 0.0, 0.0, 0.7));
            draw_rectangle(x, bar_y, enemy.size * (enemy.health / enemy.max_health).clamp(0.0, 1.0), bar_height, pickup_color());
            if enemy.shield > 0.0 {
                draw_rectangle(x, bar_y - bar_height * 0.5, enemy.size * (enemy.shield / enemy.max_health).min(1.0), bar_height * 0.4, Color::new(0.4, 0.85, 1.0, 1.0));
            }
        }
    }
}

//the rings that show what an enemy is up to go round the hitbox, this many half-sides of it from its middle (a bit past the corners)
const CUE_RING: f32 = 1.45;

//which way each enemy is facing, kept from frame to frame: it is only drawn, so it is worked out here, the same in a game of one and in one that is joined
thread_local! {
    static HEADINGS: RefCell<HashMap<u32, Heading>> = RefCell::new(HashMap::new());
}

//the way an enemy has to face, if it has to face anywhere: a charger faces the way it dashes, a boss, an enemy that is running at the army and one that shoots
//from where it can reach face the nearest player, and the rest face the way they are going
pub(crate) fn wanted_direction(enemy: &Enemy, watch: &[Vec2]) -> Option<Vec2> {
    if let Mode::Windup { dir, .. } | Mode::Dash { dir, .. } = enemy.mode {
        return Some(dir);
    }
    if enemy.status.frozen > 0.0 || enemy.routed() {
        return None;
    }
    let to_player = watch.iter().map(|&player| player - enemy.position).min_by(|a, b| a.length_squared().total_cmp(&b.length_squared()))?;
    let engaged = enemy.boss.is_some() || enemy.chasing || (enemy.range > 0.0 && to_player.length() < enemy.range + enemy.size);
    engaged.then_some(to_player)
}

//how fast an enemy turns, in radians a second: a big one slowly, a charger fast when it is showing where it is going
pub(crate) fn turn_rate(enemy: &Enemy) -> f32 {
    if matches!(enemy.mode, Mode::Windup { .. }) {
        14.0
    } else if enemy.boss.is_some() {
        FLANK_TURN
    } else {
        (9.0 - enemy.size / 40.0).clamp(3.0, 9.0)
    }
}

//the way an enemy first seen is facing, before it has gone anywhere: the way it has to face, or towards where it is walking
pub(crate) fn first_look(enemy: &Enemy, watch: &[Vec2]) -> Option<Vec2> {
    wanted_direction(enemy, watch).or_else(|| enemy.route.get(enemy.heading_for).map(|&point| point - enemy.position))
}

//what is laid over an enemy to show what has been done to it, as the shapes to draw and the colour of each: an elite's mark, armor, ice and a shield
pub(crate) fn marks_of(enemy: &Enemy) -> Vec<(&'static str, Color)> {
    let mut marks = Vec::new();
    if enemy.modifier != Modifier::None {
        marks.push((enemy.modifier.label(), enemy.modifier.color()));
    }
    if enemy.armor > 0.0 && enemy.boss.is_none() && enemy.modifier != Modifier::Armored {
        marks.push((Modifier::Armored.label(), Modifier::Armored.color()));
    }
    //a boss cracks as it goes from phase to phase, the more the further on, and the most in its last stand
    if let Some(state) = enemy.boss.as_ref() {
        let stage = (state.phase + state.last_stand as usize).min(3);
        if stage > 0 {
            marks.push((["Cracked I", "Cracked II", "Cracked III"][stage - 1], lighten(enemy.color, 0.5)));
        }
    }
    if enemy.status.frozen > 0.0 {
        marks.push(("Frozen", ICE));
    } else if enemy.status.chill >= CHILL_FULL * CHILL_SEEN {
        marks.push(("Chilled", Color::new(ICE.r, ICE.g, ICE.b, (enemy.status.chill / CHILL_FULL).clamp(0.3, 1.0))));
    }
    if enemy.status.slow < SLOW_SEEN && enemy.status.frozen <= 0.0 {
        marks.push(("Slowed", Color::new(0.6, 0.7, 0.85, 1.0)));
    }
    if enemy.status.burning > 0.0 {
        marks.push(("Burning", Color::new(1.0, 0.55, 0.1, 1.0)));
    }
    if enemy.status.shocked > 0.0 {
        marks.push(("Shocked", Color::new(0.85, 0.65, 1.0, 1.0)));
    }
    if enemy.status.poison > 0.0 {
        marks.push(("Poisoned", Color::new(0.55, 1.0, 0.4, 1.0)));
    }
    if enemy.shield > 0.0 {
        let share = (enemy.shield / enemy.max_shield.max(1.0)).clamp(0.0, 1.0);
        marks.push(("Shield", Color::new(0.4, 0.85, 1.0, 0.35 + 0.65 * share)));
    }
    marks
}

//the enemies, drawn as the creatures they are (see creatures.rs), turned to face the nearest of `watch`, the places the players are, when they are going for one.
//with `simple` they are the plain squares instead
pub(crate) fn draw_enemies(enemies: &[Enemy], squads: &Squads, time: f32, speed_multiplier: f32, camera: &Camera2D, watch: &[Vec2], simple: bool) {
    if simple {
        draw_enemies_plain(enemies, squads, time, speed_multiplier);
        return;
    }
    let dt = get_frame_time().min(0.1);
    HEADINGS.with_borrow_mut(|headings| {
        //(the ones that are not here any more are left behind)
        let mut kept = HashMap::with_capacity(enemies.len());
        draw_enemy_art(enemies, squads, time, speed_multiplier, camera, &mut |enemy| {
            if let Some(state) = enemy.boss.as_ref().filter(|state| state.def.flank) {
                //(an armoured front is where the boss is facing, which the game keeps)
                return state.facing;
            }
            let mut heading = headings.remove(&enemy.id).unwrap_or_else(|| Heading::new(enemy.position, enemy.id, first_look(enemy, watch)));
            let facing = heading.turn(enemy.position, wanted_direction(enemy, watch), turn_rate(enemy), dt);
            kept.insert(enemy.id, heading);
            facing
        });
        *headings = kept;
    });
}

//the same with the way each one faces worked out by `facing`. what is under an enemy (a charger's lane, a captain's reach) is drawn first, then the enemies, with
//what sets one apart laid over each, and then what is over them: the rings round the ones that are going for the army or have been sent at, a boss's aura, a
//healer's pulse, a bomber's fuse and a bar over the ones that are hurt. `time` is in seconds, and `speed_multiplier` is the army's, which the length of a dash
//depends on
pub(crate) fn draw_enemy_art(enemies: &[Enemy], squads: &Squads, time: f32, speed_multiplier: f32, camera: &Camera2D, facing: &mut dyn FnMut(&Enemy) -> Vec2) {
    let view = camera_view(camera);
    //(the picture of an enemy reaches well past its hitbox, and its rings past that)
    let near = |enemy: &Enemy| {
        let reach = enemy.size * 1.5;
        view.overlaps(&Rect::new(enemy.position.x - reach, enemy.position.y - reach, reach * 2.0, reach * 2.0))
    };
    let mut creatures = Vec::with_capacity(enemies.len());
    for enemy in enemies {
        let half = enemy.size / 2.0;
        if enemy.lurking() {
            //nothing to see but a patch of disturbed ground, for whoever looks
            let pulse = 0.65 + 0.35 * (time * 2.0 + enemy.position.x * 0.013).sin();
            draw_circle(enemy.position.x, enemy.position.y, half * 1.3, Color::new(0.04, 0.02, 0.07, AMBUSH_TELL * pulse));
            draw_circle_lines(enemy.position.x, enemy.position.y, half * 1.3, 2.0, Color::new(0.55, 0.4, 0.7, AMBUSH_TELL * 0.8 * pulse));
            continue;
        }
        let way = facing(enemy);
        if let Mode::Windup { dir, left } = enemy.mode {
            let urgency = (1.0 - left / CHARGE_WINDUP).clamp(0.0, 1.0);
            let end = enemy.position + dir * CHARGE_SPEED * speed_multiplier * CHARGE_TIME;
            draw_line(enemy.position.x, enemy.position.y, end.x, end.y, enemy.size, Color::new(1.0, 0.2, 0.15, 0.10 + 0.22 * urgency));
            draw_line(enemy.position.x, enemy.position.y, end.x, end.y, 4.0, Color::new(1.0, 0.45, 0.3, 0.7));
        }
        //a captain's rally reaches this far, drawn under it so the crowd can be seen to be inside it
        if enemy.is_captain() {
            let reach = RALLY_TILES * TILE_SIZE + half;
            let pulse = 0.5 + 0.5 * (time * 2.5).sin();
            let gold = enemy.modifier.color();
            draw_circle(enemy.position.x, enemy.position.y, reach, Color::new(gold.r, gold.g, gold.b, 0.03 + 0.02 * pulse));
            draw_circle_lines(enemy.position.x, enemy.position.y, reach, 3.0, Color::new(gold.r, gold.g, gold.b, 0.18 + 0.14 * pulse));
        }
        if !near(enemy) {
            continue;
        }
        let mut color = lighten(enemy.color, ((1.0 - enemy.flash / 0.15) * 0.6).max(enemy.struck / HIT_FLASH * 0.7));
        if matches!(enemy.mode, Mode::Fuse { .. }) && (time * 24.0).sin() > 0.0 {
            color = WHITE;
        }
        if enemy.status.frozen > 0.0 {
            color = Color::new(color.r * 0.3 + ICE.r * 0.7, color.g * 0.3 + ICE.g * 0.7, color.b * 0.3 + ICE.b * 0.7, 1.0);
        }
        if enemy.boss.as_ref().is_some_and(|boss| boss.shifting > 0.0) && (time * 30.0).sin() > 0.0 {
            color = lighten(color, 0.7);
        }
        //a blinker about to go flickers
        if enemy.behavior == Behavior::Blinker && enemy.timer > 0.0 && enemy.timer < BLINK_TELL && (time * 34.0).sin() > 0.0 {
            color = lighten(color, 0.8);
        }
        //running from the army: washed out
        if enemy.routed() {
            color = lighten(color, 0.55);
        }
        if let Some(shape) = shape_named(enemy.name) {
            let last_stand = enemy.boss.as_ref().is_some_and(|boss| boss.last_stand);
            creatures.push(Creature::new(shape, enemy.position, enemy.size, way, color).seeded(enemy.id).hurt(last_stand));
        }
        for (name, tint) in marks_of(enemy) {
            if let Some(shape) = shape_named(name) {
                creatures.push(Creature::new(shape, enemy.position, enemy.size, way, tint).seeded(enemy.id));
            }
        }
    }
    draw_creatures(&creatures, camera);
    for enemy in enemies.iter().filter(|enemy| !enemy.lurking() && near(enemy)) {
        let half = enemy.size / 2.0;
        let at = enemy.position;
        let ring = (half * CUE_RING).max(half + 2.0);
        let width = (enemy.size * 0.03).clamp(2.0, 12.0);
        if enemy.routed() {
            //running away: a white ring in place of the red one
            draw_circle_lines(at.x, at.y, ring, width, WHITE);
        } else if enemy.chasing && enemy.boss.is_none() {
            //running at the player: a red ring, thinner than the squad's
            draw_circle_lines(at.x, at.y, ring, width, RED);
        }
        if enemy.rallied {
            //rallied by a captain: a gold ring inside the red one
            let gold = Modifier::Captain.color();
            draw_circle_lines(at.x, at.y, ring - width * 1.6, width * 0.8, Color::new(gold.r, gold.g, gold.b, 0.95));
        }
        if enemy.provoked > 0.0 && !enemy.chasing && enemy.boss.is_none() {
            //fighting back at the units that hurt it: an orange ring, and the shorter it has left the fainter
            let fade = (enemy.provoked / RETALIATE_SECONDS).clamp(0.2, 1.0);
            draw_circle_lines(at.x, at.y, ring, width, Color::new(1.0, 0.62, 0.15, 0.9 * fade));
        }
        match enemy.behavior {
            Behavior::Healer if enemy.flash < 0.6 => {
                let age = enemy.flash / 0.6;
                draw_circle_lines(at.x, at.y, HEAL_RANGE * age, 4.0, Color::new(0.4, 1.0, 0.5, 0.6 * (1.0 - age)));
            }
            Behavior::Summoner if enemy.flash < 0.5 => {
                let age = enemy.flash / 0.5;
                draw_circle_lines(at.x, at.y, half + 20.0 + 160.0 * age, 4.0, Color::new(0.7, 0.4, 1.0, 0.7 * (1.0 - age)));
            }
            Behavior::Bomber if matches!(enemy.mode, Mode::Fuse { .. }) => {
                let pulse = 0.5 + 0.5 * (time * 24.0).sin();
                draw_circle_lines(at.x, at.y, BLAST_RADIUS * blast_scale(enemy.scale) + half, 3.0, danger_color(0.4 + 0.5 * pulse));
            }
            _ => {}
        }
        //how far out the outermost ring is, which the squad's goes outside
        let mut outer = ring + width * 0.5;
        if let Some(state) = &enemy.boss {
            let pulse = 0.5 + 0.5 * (time * if state.last_stand { 10.0 } else { 3.0 }).sin();
            let aura_width = (enemy.size * 0.02).clamp(6.0, 30.0);
            let aura = if state.last_stand { RED } else { enemy.color };
            outer = ring + aura_width * 0.5;
            draw_circle_lines(at.x, at.y, ring, aura_width, Color::new(aura.r, aura.g, aura.b, 0.3 + 0.35 * pulse));
            //a ring round it that fills in the colour of the circle it will put down next, and flashes when that is close
            if let Some(next) = state.next_circle() {
                let tint = next.circle.color();
                let radius = enemy.size * 0.5 * std::f32::consts::SQRT_2 + aura_width * 3.0 + 14.0;
                let thickness = (enemy.size * 0.025).clamp(4.0, 20.0);
                let soon = if next.seconds <= 1.5 { 0.5 + 0.5 * (time * 14.0).sin() } else { 0.0 };
                draw_circle_lines(at.x, at.y, radius, thickness * 0.5, Color::new(tint.r, tint.g, tint.b, 0.2));
                draw_sweep(at, radius, -std::f32::consts::FRAC_PI_2, std::f32::consts::TAU * next.share, thickness, Color::new(tint.r, tint.g, tint.b, 0.75 + 0.25 * soon));
                outer = outer.max(radius + thickness * 0.5);
            }
            //an armoured front is a bright arc on the side it is facing
            if state.def.flank {
                let radius = enemy.size * 0.62;
                let arc_width = (enemy.size * 0.04).clamp(5.0, 20.0);
                let steps = 16;
                let point = |k: i32| at + state.facing.rotate(Vec2::from_angle(-FLANK_ARC + 2.0 * FLANK_ARC * k as f32 / steps as f32)) * radius;
                for k in 0..steps {
                    let (from, to) = (point(k), point(k + 1));
                    draw_line(from.x, from.y, to.x, to.y, arc_width, Color::new(0.75, 0.9, 1.0, 0.9));
                }
            }
        }
        if squads.contains_key(&enemy.id) {
            let pulse = 0.5 + 0.5 * (get_time() as f32 * 8.0).sin();
            let squad_width = (enemy.size * 0.06).clamp(4.0, 24.0);
            let outline = Color::new(SQUAD_COLOR.r, SQUAD_COLOR.g, SQUAD_COLOR.b, 0.5 + 0.5 * pulse);
            draw_circle_lines(at.x, at.y, outer + squad_width * 0.5 + 2.0, squad_width, outline);
        }
        if enemy.health < enemy.max_health || enemy.shield > 0.0 {
            let bar_height = (enemy.size * 0.1).clamp(6.0, 40.0);
            let (x, bar_y) = (at.x - half, at.y - half * PICTURE - bar_height * 2.0);
            draw_rectangle(x, bar_y, enemy.size, bar_height, Color::new(0.0, 0.0, 0.0, 0.7));
            draw_rectangle(x, bar_y, enemy.size * (enemy.health / enemy.max_health).clamp(0.0, 1.0), bar_height, pickup_color());
            if enemy.shield > 0.0 {
                draw_rectangle(x, bar_y - bar_height * 0.5, enemy.size * (enemy.shield / enemy.max_health).min(1.0), bar_height * 0.4, Color::new(0.4, 0.85, 1.0, 1.0));
            }
        }
    }
}


//an arc of a circle: from `start` radians round by `sweep` radians, drawn as short lines
pub(crate) fn draw_sweep(center: Vec2, radius: f32, start: f32, sweep: f32, thickness: f32, color: Color) {
    let steps = ((sweep.abs() * radius / 24.0).ceil() as usize).clamp(2, 160);
    let point = |k: usize| center + Vec2::from_angle(start + sweep * k as f32 / steps as f32) * radius;
    for k in 0..steps {
        let (from, to) = (point(k), point(k + 1));
        draw_line(from.x, from.y, to.x, to.y, thickness, color);
    }
}
