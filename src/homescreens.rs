//! the title screen comes in several looks: a backdrop and colours for the words (dark or light), where the words stand, which boss looms over the rest, which enemies
//! stand in the army's way, and where the player is and how big their army is. this is only what each one is made of, as numbers; titlescene.rs draws it and screens.rs puts
//! the words on it. pressing Tab on the title screen goes to the next, and which one comes up first is not always the same

use crate::prelude::*;
use std::sync::atomic::AtomicUsize;

//where something is along one side of the window: from the near edge (left or top), from the middle, or from the far edge (right or bottom)
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Edge {
    Near,
    Middle,
    Far,
}

//a place in the window, in pixels of the size the pictures are made for (a window of another size draws them bigger or smaller, see `Frame`): across from one edge and
//down from another. the offsets are always towards the inside, from the near and far edges, and to the right and down from the middle
#[derive(Clone, Copy, Debug)]
pub(crate) struct Spot {
    pub(crate) across: Edge,
    pub(crate) x: f32,
    pub(crate) down: Edge,
    pub(crate) y: f32,
}

const fn spot(across: Edge, x: f32, down: Edge, y: f32) -> Spot {
    Spot { across, x, down, y }
}

impl Spot {
    //where it is in a window of a size, with everything `scale` times as big as it was made
    pub(crate) fn at(self, screen: Vec2, scale: f32) -> Vec2 {
        let along = |edge: Edge, offset: f32, length: f32| match edge {
            Edge::Near => offset * scale,
            Edge::Middle => length / 2.0 + offset * scale,
            Edge::Far => length - offset * scale,
        };
        Vec2::new(along(self.across, self.x, screen.x), along(self.down, self.y, screen.y))
    }

    //which way it is from the middle of the window: -1 for the left or the top half, 1 for the other
    pub(crate) fn side(self, screen: Vec2, scale: f32) -> Vec2 {
        let at = self.at(screen, scale);
        Vec2::new(if at.x < screen.x / 2.0 { -1.0 } else { 1.0 }, if at.y < screen.y / 2.0 { -1.0 } else { 1.0 })
    }
}

//the colours of a screen: the backdrop from the top of the window to the bottom, the faint grid on it, the title and its glow, and the words under it
#[derive(Clone, Copy, Debug)]
pub(crate) struct Theme {
    pub(crate) light: bool,
    pub(crate) top: Color,
    pub(crate) bottom: Color,
    pub(crate) grid: Color,
    pub(crate) heading: Color, //the letters of the title
    pub(crate) glow: Color,    //the haze of light round them
    pub(crate) shadow: Color,  //and the shadow under them
    pub(crate) accent: Color,  //the line under the title, the prompt and the speed lines of the ground
    pub(crate) ink: Color,     //the words that say most
    pub(crate) muted: Color,   //the ones that say least
    pub(crate) trophy: Color,  //the best run
    pub(crate) alert: Color,   //what has gone wrong, and the button that closes the game
}

//where the column of words stands: the title, the line under it, and the prompts, one under the other
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Words {
    pub(crate) align: Align,
    pub(crate) margin: f32,   //how far from the edge, in pixels of the design, of a column that is on the left or the right
    pub(crate) size: f32,     //how big the words are next to what they would be if they filled the window, 1 for that
    pub(crate) title_y: f32,  //how far down the window the title stands, as a share of its height
    pub(crate) prompts_y: f32, //and the first of the prompts
}

impl Words {
    //how wide the widest line of the words is, at most, in a window of a size
    pub(crate) fn width(&self, screen: Vec2) -> f32 {
        (1.2 * screen.y * self.size).min(0.9 * screen.x)
    }

    //where the middle of the column of words is across the window, when the design is drawn `scale` times as big as it was made
    pub(crate) fn center(&self, screen: Vec2, scale: f32) -> f32 {
        let half = self.width(screen) / 2.0;
        let center = match self.align {
            Align::Center => screen.x / 2.0,
            Align::Left => self.margin * scale + half,
            Align::Right => screen.x - self.margin * scale - half,
        };
        center.clamp(half.min(screen.x / 2.0), (screen.x - half).max(screen.x / 2.0))
    }
}

//an enemy where it stands, and how big (1 for the size of its kind), whether it is running at the army and how much health it has left
#[derive(Clone, Copy, Debug)]
pub(crate) struct Placed {
    pub(crate) name: &'static str,
    pub(crate) at: Spot,
    pub(crate) scale: f32,
    pub(crate) chasing: bool,
    pub(crate) health: f32,
}

const fn foe(name: &'static str, at: Spot, scale: f32, chasing: bool, health: f32) -> Placed {
    Placed { name, at, scale, chasing, health }
}

//the player at the head of their army: where they are, how many ships there are in the two rings round them and strung out behind (each ring as how many and how far
//out, in the world of the scene), how big all of it is next to the rest of the picture, how many times bigger than a ship the player is drawn, and which way they run
#[derive(Clone, Copy, Debug)]
pub(crate) struct Army {
    pub(crate) at: Spot,
    pub(crate) inner: (usize, f32),
    pub(crate) outer: (usize, f32),
    pub(crate) tail: usize,
    pub(crate) zoom: f32,
    pub(crate) hero: f32,
    pub(crate) faces_left: bool,
}

impl Army {
    #[cfg(test)]
    pub(crate) fn ships(&self) -> usize {
        self.inner.0 + self.outer.0 + self.tail
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Homescreen {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) name: &'static str,
    pub(crate) theme: Theme,
    pub(crate) words: Words,
    pub(crate) boss: &'static str, //which boss stands over it
    pub(crate) boss_at: Spot,
    pub(crate) boss_size: f32, //how many pixels of the design its picture takes up, along each side
    pub(crate) foes: &'static [Placed],
    pub(crate) army: Army,
}

//what the screen has been the last time one was picked for it (so that the next is not the same)
static LAST_SHOWN: AtomicUsize = AtomicUsize::new(usize::MAX);

//a look for the screen when it comes up: any but the one it had last, unless `THE_ARMY_HOMESCREEN` says which (its number, from 0)
pub(crate) fn random_homescreen() -> usize {
    if let Some(chosen) = std::env::var("THE_ARMY_HOMESCREEN").ok().and_then(|number| number.trim().parse::<usize>().ok()) {
        LAST_SHOWN.store(chosen % HOMESCREENS.len(), Ordering::Relaxed);
        return chosen % HOMESCREENS.len();
    }
    let last = LAST_SHOWN.load(Ordering::Relaxed);
    let mut pick = (miniquad::date::now() * 1000.0) as usize % HOMESCREENS.len();
    if pick == last {
        pick = (pick + 1) % HOMESCREENS.len();
    }
    LAST_SHOWN.store(pick, Ordering::Relaxed);
    pick
}

//the look after one, which Tab goes to
pub(crate) fn next_homescreen(current: usize) -> usize {
    let next = (current + 1) % HOMESCREENS.len();
    LAST_SHOWN.store(next, Ordering::Relaxed);
    next
}

//the boss of a name
pub(crate) fn boss_color(name: &str) -> Color {
    BOSSES.iter().chain(std::iter::once(&FINAL_BOSS)).find(|def| def.name == name).map_or(WHITE, |def| def.color)
}

const fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::new(r, g, b, 1.0)
}

const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::new(r, g, b, a)
}

pub(crate) static HOMESCREENS: [Homescreen; 5] = [
    //the first: the army at the foot of the screen on the left, its enemies on the right, and the Sovereign over them all. very dark blue, and the cyan of the game
    Homescreen {
        name: "Vanguard",
        theme: Theme {
            light: false,
            top: rgb(0.015, 0.025, 0.05),
            bottom: rgb(0.015, 0.025, 0.05),
            grid: rgba(0.22, 0.89, 1.0, 0.045),
            heading: rgb(1.0, 1.0, 1.0),
            glow: rgba(0.22, 0.89, 1.0, 0.22),
            shadow: rgba(0.0, 0.0, 0.0, 0.75),
            accent: rgb(0.22, 0.89, 1.0),
            ink: rgb(0.93, 0.97, 1.0),
            muted: rgb(0.58, 0.69, 0.8),
            trophy: rgb(1.0, 0.85, 0.3),
            alert: rgb(1.0, 0.42, 0.35),
        },
        words: Words { align: Align::Center, margin: 0.0, size: 1.0, title_y: 0.46, prompts_y: 0.76 },
        boss: "The Sovereign",
        boss_at: spot(Edge::Far, 150.0, Edge::Near, 112.0),
        boss_size: 187.0,
        foes: &[
            foe("Bulwark", spot(Edge::Far, 110.0, Edge::Far, 235.0), 1.0, false, 1.0),
            foe("Splitter", spot(Edge::Far, 215.0, Edge::Far, 255.0), 1.0, false, 1.0),
            foe("Brute", spot(Edge::Far, 230.0, Edge::Far, 150.0), 1.0, true, 0.6),
            foe("Grunt", spot(Edge::Far, 130.0, Edge::Far, 105.0), 1.0, true, 1.0),
            foe("Grunt", spot(Edge::Far, 235.0, Edge::Far, 62.0), 1.0, true, 0.7),
            foe("Scout", spot(Edge::Far, 55.0, Edge::Far, 160.0), 1.0, true, 1.0),
        ],
        army: Army { at: spot(Edge::Near, 195.0, Edge::Far, 190.0), inner: (6, 36.0), outer: (13, 70.0), tail: 9, zoom: 1.0, hero: 1.7, faces_left: false },
    },
    //warm paper and navy ink, the words in a column on the left and the Hive Mother on the right, and a small army with a big runner at the foot of the screen
    Homescreen {
        name: "Daybreak",
        theme: Theme {
            light: true,
            top: rgb(0.99, 0.95, 0.86),
            bottom: rgb(0.88, 0.93, 0.98),
            grid: rgba(0.1, 0.16, 0.35, 0.06),
            heading: rgb(0.07, 0.11, 0.26),
            glow: rgba(1.0, 0.6, 0.25, 0.30),
            shadow: rgba(1.0, 1.0, 1.0, 0.6),
            accent: rgb(0.92, 0.4, 0.08),
            ink: rgb(0.07, 0.1, 0.22),
            muted: rgb(0.32, 0.38, 0.52),
            trophy: rgb(0.78, 0.48, 0.0),
            alert: rgb(0.8, 0.17, 0.14),
        },
        words: Words { align: Align::Left, margin: 64.0, size: 0.62, title_y: 0.27, prompts_y: 0.55 },
        boss: "The Hive Mother",
        boss_at: spot(Edge::Far, 215.0, Edge::Middle, -25.0),
        boss_size: 330.0,
        foes: &[
            foe("Brute", spot(Edge::Far, 420.0, Edge::Far, 110.0), 1.0, true, 0.7),
            foe("Grunt", spot(Edge::Far, 120.0, Edge::Far, 95.0), 1.0, false, 1.0),
            foe("Scout", spot(Edge::Far, 300.0, Edge::Far, 70.0), 1.0, true, 1.0),
            foe("Healer", spot(Edge::Far, 60.0, Edge::Far, 230.0), 1.0, false, 1.0),
        ],
        army: Army { at: spot(Edge::Near, 175.0, Edge::Far, 105.0), inner: (5, 36.0), outer: (10, 66.0), tail: 6, zoom: 0.8, hero: 2.2, faces_left: false },
    },
    //black and violet: the words on the right, the Eclipse over the left, and a big army coming from the right to meet a cloud of small enemies
    Homescreen {
        name: "Eclipse",
        theme: Theme {
            light: false,
            top: rgb(0.03, 0.01, 0.08),
            bottom: rgb(0.11, 0.02, 0.16),
            grid: rgba(0.85, 0.35, 1.0, 0.05),
            heading: rgb(0.98, 0.94, 1.0),
            glow: rgba(0.9, 0.3, 1.0, 0.28),
            shadow: rgba(0.0, 0.0, 0.0, 0.8),
            accent: rgb(0.95, 0.4, 0.95),
            ink: rgb(0.96, 0.93, 1.0),
            muted: rgb(0.68, 0.58, 0.82),
            trophy: rgb(1.0, 0.82, 0.35),
            alert: rgb(1.0, 0.45, 0.4),
        },
        words: Words { align: Align::Right, margin: 56.0, size: 0.8, title_y: 0.32, prompts_y: 0.52 },
        boss: "The Eclipse",
        boss_at: spot(Edge::Near, 215.0, Edge::Near, 200.0),
        boss_size: 300.0,
        foes: &[
            foe("Seeker", spot(Edge::Near, 90.0, Edge::Far, 100.0), 1.0, true, 1.0),
            foe("Drone", spot(Edge::Near, 190.0, Edge::Far, 190.0), 1.0, false, 1.0),
            foe("Imp", spot(Edge::Near, 70.0, Edge::Far, 215.0), 1.0, true, 1.0),
            foe("Imp", spot(Edge::Near, 290.0, Edge::Far, 95.0), 1.0, true, 1.0),
            foe("Summoner", spot(Edge::Near, 400.0, Edge::Far, 160.0), 1.0, false, 0.8),
            foe("Splitling", spot(Edge::Near, 175.0, Edge::Far, 70.0), 1.0, true, 1.0),
        ],
        army: Army { at: spot(Edge::Far, 250.0, Edge::Far, 112.0), inner: (8, 38.0), outer: (16, 78.0), tail: 12, zoom: 1.0, hero: 1.8, faces_left: true },
    },
    //deep green: the words across the top and the swarm below them, the Swarm Queen in the middle of it and the army at one side
    Homescreen {
        name: "Hive",
        theme: Theme {
            light: false,
            top: rgb(0.01, 0.045, 0.03),
            bottom: rgb(0.05, 0.11, 0.05),
            grid: rgba(0.7, 0.95, 0.25, 0.045),
            heading: rgb(0.93, 1.0, 0.82),
            glow: rgba(0.7, 0.95, 0.2, 0.26),
            shadow: rgba(0.0, 0.0, 0.0, 0.8),
            accent: rgb(0.72, 0.96, 0.22),
            ink: rgb(0.93, 1.0, 0.9),
            muted: rgb(0.58, 0.75, 0.58),
            trophy: rgb(1.0, 0.8, 0.25),
            alert: rgb(1.0, 0.5, 0.35),
        },
        words: Words { align: Align::Center, margin: 0.0, size: 0.8, title_y: 0.24, prompts_y: 0.52 },
        boss: "The Swarm Queen",
        boss_at: spot(Edge::Middle, 40.0, Edge::Far, 125.0),
        boss_size: 215.0,
        foes: &[
            foe("Swarmer", spot(Edge::Middle, -150.0, Edge::Far, 70.0), 1.0, true, 1.0),
            foe("Swarmer", spot(Edge::Middle, -105.0, Edge::Far, 140.0), 1.0, true, 1.0),
            foe("Swarmer", spot(Edge::Middle, 200.0, Edge::Far, 60.0), 1.0, true, 1.0),
            foe("Swarmer", spot(Edge::Middle, 250.0, Edge::Far, 150.0), 1.0, true, 1.0),
            foe("Scout", spot(Edge::Far, 120.0, Edge::Far, 110.0), 1.0, true, 1.0),
            foe("Spitter", spot(Edge::Far, 70.0, Edge::Far, 235.0), 1.0, false, 1.0),
            foe("Swarmer", spot(Edge::Middle, 330.0, Edge::Far, 85.0), 1.0, true, 1.0),
        ],
        army: Army { at: spot(Edge::Near, 120.0, Edge::Far, 120.0), inner: (5, 34.0), outer: (9, 62.0), tail: 5, zoom: 0.9, hero: 2.0, faces_left: false },
    },
    //pale steel and dark ink with a red edge: the Iron Warden over a short title, a big army on the left and the heavy enemies on the right
    Homescreen {
        name: "Ironclad",
        theme: Theme {
            light: true,
            top: rgb(0.9, 0.92, 0.95),
            bottom: rgb(0.72, 0.77, 0.84),
            grid: rgba(0.1, 0.12, 0.2, 0.07),
            heading: rgb(0.09, 0.11, 0.17),
            glow: rgba(0.85, 0.2, 0.25, 0.22),
            shadow: rgba(1.0, 1.0, 1.0, 0.55),
            accent: rgb(0.78, 0.12, 0.18),
            ink: rgb(0.08, 0.1, 0.15),
            muted: rgb(0.33, 0.38, 0.47),
            trophy: rgb(0.7, 0.42, 0.0),
            alert: rgb(0.78, 0.15, 0.12),
        },
        words: Words { align: Align::Center, margin: 0.0, size: 0.72, title_y: 0.5, prompts_y: 0.76 },
        boss: "The Iron Warden",
        boss_at: spot(Edge::Middle, 0.0, Edge::Near, 120.0),
        boss_size: 215.0,
        foes: &[
            foe("Titan", spot(Edge::Far, 170.0, Edge::Far, 190.0), 0.9, true, 0.8),
            foe("Charger", spot(Edge::Far, 330.0, Edge::Far, 100.0), 1.0, true, 1.0),
            foe("Mortar", spot(Edge::Far, 95.0, Edge::Far, 85.0), 1.0, false, 1.0),
            foe("Lancer", spot(Edge::Far, 60.0, Edge::Far, 330.0), 1.0, false, 1.0),
            foe("Bomber", spot(Edge::Far, 250.0, Edge::Far, 60.0), 1.0, true, 1.0),
        ],
        army: Army { at: spot(Edge::Near, 215.0, Edge::Far, 170.0), inner: (7, 36.0), outer: (14, 72.0), tail: 10, zoom: 1.2, hero: 1.6, faces_left: false },
    },
];
