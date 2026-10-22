//! relics a slain boss offers, the upgrades they add up to and the choice screen

use crate::prelude::*;

//how often a relic is offered: a common one is six times as likely as a rare one
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Rarity {
    Common,
    Uncommon,
    Rare,
}

impl Rarity {
    pub(crate) fn weight(self) -> f32 {
        match self {
            Rarity::Common => 6.0,
            Rarity::Uncommon => 3.0,
            Rarity::Rare => 1.0,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Rarity::Common => "COMMON",
            Rarity::Uncommon => "UNCOMMON",
            Rarity::Rare => "RARE",
        }
    }

    pub(crate) fn color(self) -> Color {
        match self {
            Rarity::Common => Color::new(0.78, 0.8, 0.86, 1.0),
            Rarity::Uncommon => CYAN,
            Rarity::Rare => Color::new(1.0, 0.8, 0.25, 1.0),
        }
    }
}

//rewards for slaying a boss, one of three of which the player chooses. each can be chosen again, and stacks
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Relic {
    Whetstone,    //every unit hits harder
    IronResolve,  //every unit takes less damage
    Regeneration, //hurt units heal faster
    FleetFeet,    //the player and the army move faster
    Magnet,       //babies are picked up from further away
    Bounty,       //enemies drop more babies
    Masonry,      //walls are built tougher
    Quarry,       //enemies are worth more wall points
    Archers,      //towers shoot harder
    Thorns,       //an enemy takes back some of what it does to the units
    Bloodlust,    //every unit heals a little for each enemy destroyed
    Aegis,        //every unit has a shield that takes the first hits, and comes back when it is left alone
    Muster,       //volunteers join the army when it is chosen
    OpeningSalvo, //a shockwave when a fight begins
    Martyr,       //a unit that dies in an enemy hurts it
    GlassCannon,  //units hit much harder and take more damage
    Overclock,    //the player and the army are much faster, and heal slower
    Barracks,     //the army can have more units than it could
    Tailwind,     //the speed an army's size gives can go higher than it could
}

pub(crate) const RELICS: [Relic; 19] = [
    Relic::Whetstone,
    Relic::IronResolve,
    Relic::Regeneration,
    Relic::FleetFeet,
    Relic::Magnet,
    Relic::Bounty,
    Relic::Masonry,
    Relic::Quarry,
    Relic::Archers,
    Relic::Thorns,
    Relic::Bloodlust,
    Relic::Aegis,
    Relic::Muster,
    Relic::OpeningSalvo,
    Relic::Martyr,
    Relic::GlassCannon,
    Relic::Overclock,
    Relic::Barracks,
    Relic::Tailwind,
];

impl Relic {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Relic::Whetstone => "Whetstone",
            Relic::IronResolve => "Iron Resolve",
            Relic::Regeneration => "Regeneration",
            Relic::FleetFeet => "Fleet Feet",
            Relic::Magnet => "Magnet",
            Relic::Bounty => "Bounty",
            Relic::Masonry => "Masonry",
            Relic::Quarry => "Quarry",
            Relic::Archers => "Archers",
            Relic::Thorns => "Thorns",
            Relic::Bloodlust => "Bloodlust",
            Relic::Aegis => "Aegis",
            Relic::Muster => "Muster",
            Relic::OpeningSalvo => "Opening Salvo",
            Relic::Martyr => "Martyr",
            Relic::GlassCannon => "Glass Cannon",
            Relic::Overclock => "Overclock",
            Relic::Barracks => "Barracks",
            Relic::Tailwind => "Tailwind",
        }
    }

    pub(crate) fn blurb(self) -> &'static str {
        match self {
            Relic::Whetstone => "units hit 25% harder",
            Relic::IronResolve => "units take 20% less damage",
            Relic::Regeneration => "hurt units heal twice as fast",
            Relic::FleetFeet => "you and the army move 12% faster",
            Relic::Magnet => "babies are picked up from 50% further",
            Relic::Bounty => "enemies drop 40% more babies",
            Relic::Masonry => "walls are built with 30% more health",
            Relic::Quarry => "enemies are worth 50% more wall points",
            Relic::Archers => "units in towers shoot 40% harder",
            Relic::Thorns => "an enemy takes back 20% of the damage it does to your units",
            Relic::Bloodlust => "every unit heals 1.5 for each enemy destroyed",
            Relic::Aegis => "every unit has a 30 point shield that comes back after 10 quiet seconds",
            Relic::Muster => "volunteers join you: an eighth of the army, at least ten",
            Relic::OpeningSalvo => "when a fight begins a shockwave hurts every enemy near you",
            Relic::Martyr => "a unit that dies inside an enemy blows up for 40 damage",
            Relic::GlassCannon => "units hit 60% harder but take 40% more damage",
            Relic::Overclock => "you and the army are 35% faster, but heal at half the rate",
            Relic::Barracks => "the army can grow 1,000 units bigger than it could",
            Relic::Tailwind => "the speed a big army gets can go 0.5x higher than the limit",
        }
    }

    //two letters to know it by on the relic bar
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Relic::Whetstone => "Wh",
            Relic::IronResolve => "IR",
            Relic::Regeneration => "Rg",
            Relic::FleetFeet => "FF",
            Relic::Magnet => "Mg",
            Relic::Bounty => "Bo",
            Relic::Masonry => "Ms",
            Relic::Quarry => "Qu",
            Relic::Archers => "Ar",
            Relic::Thorns => "Th",
            Relic::Bloodlust => "Bl",
            Relic::Aegis => "Ae",
            Relic::Muster => "Mu",
            Relic::OpeningSalvo => "OS",
            Relic::Martyr => "My",
            Relic::GlassCannon => "GC",
            Relic::Overclock => "Ov",
            Relic::Barracks => "Br",
            Relic::Tailwind => "Tw",
        }
    }

    pub(crate) fn color(self) -> Color {
        match self {
            Relic::Whetstone => Color::new(1.0, 0.7, 0.3, 1.0),
            Relic::IronResolve => Color::new(0.6, 0.75, 1.0, 1.0),
            Relic::Regeneration => Color::new(0.5, 1.0, 0.6, 1.0),
            Relic::FleetFeet => Color::new(1.0, 1.0, 0.5, 1.0),
            Relic::Magnet => Color::new(0.8, 0.5, 1.0, 1.0),
            Relic::Bounty => Color::new(1.0, 0.85, 0.2, 1.0),
            Relic::Masonry => Color::new(0.75, 0.7, 0.62, 1.0),
            Relic::Quarry => Color::new(0.7, 0.55, 0.4, 1.0),
            Relic::Archers => Color::new(0.55, 0.9, 0.5, 1.0),
            Relic::Thorns => Color::new(0.4, 0.85, 0.35, 1.0),
            Relic::Bloodlust => Color::new(0.95, 0.25, 0.3, 1.0),
            Relic::Aegis => Color::new(0.4, 0.85, 1.0, 1.0),
            Relic::Muster => Color::new(0.9, 0.9, 0.9, 1.0),
            Relic::OpeningSalvo => Color::new(1.0, 0.55, 0.2, 1.0),
            Relic::Martyr => Color::new(1.0, 0.4, 0.15, 1.0),
            Relic::GlassCannon => Color::new(0.9, 0.95, 1.0, 1.0),
            Relic::Overclock => Color::new(0.5, 1.0, 0.9, 1.0),
            Relic::Barracks => Color::new(0.95, 0.72, 0.85, 1.0),
            Relic::Tailwind => Color::new(0.6, 0.95, 1.0, 1.0),
        }
    }

    pub(crate) fn rarity(self) -> Rarity {
        match self {
            Relic::Whetstone | Relic::IronResolve | Relic::Regeneration | Relic::Magnet | Relic::Masonry | Relic::Quarry => Rarity::Common,
            Relic::FleetFeet | Relic::Bounty | Relic::Archers | Relic::Thorns | Relic::Bloodlust | Relic::Aegis | Relic::Muster | Relic::Barracks | Relic::Tailwind => Rarity::Uncommon,
            Relic::OpeningSalvo | Relic::Martyr | Relic::GlassCannon | Relic::Overclock => Rarity::Rare,
        }
    }
}

//what the relics taken so far add up to: multipliers that start at 1, and amounts that start at 0
#[derive(Clone, Debug)]
pub(crate) struct Upgrades {
    pub(crate) damage: f32,    //of the damage every unit does
    pub(crate) toughness: f32, //of the damage every unit takes, so less than 1 is tougher
    pub(crate) healing: f32,   //of how quickly hurt units heal
    pub(crate) speed: f32,     //of the speed of the player and the army
    pub(crate) pickup: f32,    //of how far away babies are picked up from
    pub(crate) bounty: f32,    //of the babies enemies drop
    pub(crate) masonry: f32,   //of the health walls are built with
    pub(crate) archery: f32,   //of the damage towers do
    pub(crate) quarry: f32,    //of the wall points enemies are worth
    pub(crate) thorns: f32,    //share of the damage an enemy does to units that it takes itself
    pub(crate) bloodlust: f32, //health every unit gets for each enemy destroyed
    pub(crate) guard: f32,     //points of shield every unit has
    pub(crate) salvo: f32,     //shockwaves' strength when a fight begins, 0 for none
    pub(crate) martyr: f32,    //of the damage a unit does to the enemy it dies in
    pub(crate) extra_units: usize, //the army can have this many more units than MAX_UNITS
    pub(crate) extra_boost: f32,   //the most the army's size can speed everyone up is this many times normal higher than MAX_SPEED_MULTIPLIER
    pub(crate) taken: Vec<Relic>,
}

impl Default for Upgrades {
    fn default() -> Upgrades {
        Upgrades { damage: 1.0, toughness: 1.0, healing: 1.0, speed: 1.0, pickup: 1.0, bounty: 1.0, masonry: 1.0, archery: 1.0, quarry: 1.0, thorns: 0.0, bloodlust: 0.0, guard: 0.0, salvo: 0.0, martyr: 0.0, extra_units: 0, extra_boost: 0.0, taken: Vec::new() }
    }
}

impl Upgrades {
    pub(crate) fn take(&mut self, relic: Relic) {
        match relic {
            Relic::Whetstone => self.damage *= 1.25,
            Relic::IronResolve => self.toughness *= 0.8,
            Relic::Regeneration => self.healing *= 2.0,
            Relic::FleetFeet => self.speed *= 1.12,
            Relic::Magnet => self.pickup *= 1.5,
            Relic::Bounty => self.bounty *= 1.4,
            Relic::Masonry => self.masonry *= 1.3,
            Relic::Quarry => self.quarry *= 1.5,
            Relic::Archers => self.archery *= 1.4,
            Relic::Thorns => self.thorns += 0.2,
            Relic::Bloodlust => self.bloodlust += 1.5,
            Relic::Aegis => self.guard += 30.0,
            Relic::Muster => {} //the game brings the volunteers
            Relic::OpeningSalvo => self.salvo += 1.0,
            Relic::Martyr => self.martyr += 1.0,
            Relic::GlassCannon => {
                self.damage *= 1.6;
                self.toughness *= 1.4;
            }
            Relic::Overclock => {
                self.speed *= 1.35;
                self.healing *= 0.5;
            }
            Relic::Barracks => self.extra_units += BARRACKS_UNITS,
            Relic::Tailwind => self.extra_boost += TAILWIND_BOOST,
        }
        self.taken.push(relic);
    }

    pub(crate) fn count(&self, relic: Relic) -> usize {
        self.taken.iter().filter(|&&taken| taken == relic).count()
    }

    //the most units the army can have
    pub(crate) fn unit_cap(&self) -> usize {
        MAX_UNITS + self.extra_units
    }

    //how many times faster than normal the player and the army are for an army this big, from its size and the relics that
    //speed it up. the enemies' own speed-up comes from `speed_multiplier`, which the relics don't change
    pub(crate) fn army_boost(&self, army: usize) -> f32 {
        speed_multiplier_with(army, self.extra_boost) * self.speed
    }

    //how much of its health a shockwave takes off an enemy: each one is a share of what is left, so any number of them
    //hurt more and none of them kills outright
    pub(crate) fn salvo_share(&self) -> f32 {
        1.0 - (1.0 - SALVO_SHARE).powf(self.salvo)
    }
}

//how likely a relic is to be offered. a Barracks is much likelier while the army is full, which is when it is the only way to grow it
fn offer_weight(relic: Relic, army_full: bool) -> f32 {
    let weight = relic.rarity().weight();
    if army_full && relic == Relic::Barracks { weight * BARRACKS_FULL_ARMY_WEIGHT } else { weight }
}

//three different relics out of a pool, at random, the common ones more often than the rare ones
pub(crate) fn weighted_choices(pool: &[Relic], army_full: bool) -> [Relic; 3] {
    let mut left = pool.to_vec();
    let mut choices = [pool[0]; 3];
    for choice in &mut choices {
        let mut pick = rand::gen_range(0.0, left.iter().map(|&relic| offer_weight(relic, army_full)).sum::<f32>());
        let mut index = left.len() - 1;
        for (i, relic) in left.iter().enumerate() {
            if pick < offer_weight(*relic, army_full) {
                index = i;
                break;
            }
            pick -= offer_weight(*relic, army_full);
        }
        *choice = left.remove(index);
    }
    choices
}

//three different relics to choose from, at random. `army_full` is whether the army is as big as it can be
pub(crate) fn relic_choices(army_full: bool) -> [Relic; 3] {
    weighted_choices(&RELICS, army_full)
}

//three of the rare ones, the reward for winning
pub(crate) fn rare_choices() -> [Relic; 3] {
    let rares: Vec<Relic> = RELICS.iter().copied().filter(|relic| relic.rarity() == Rarity::Rare).collect();
    weighted_choices(&rares, false)
}

//the relics on offer after a boss, while the world stands still
#[derive(Clone)]
pub(crate) struct RelicChoice {
    pub(crate) options: [Relic; 3],
    pub(crate) age: f32,       //seconds it has been up
    pub(crate) rare_only: bool, //the relics are all rare, and so are the ones a reroll brings
    pub(crate) army_full: bool, //the army was as big as it can be when this came up, which makes a Barracks likelier
    pub(crate) chest: u32,      //the tier of the chest of a horde that it came from, 0 if it came from a boss
}

impl RelicChoice {
    pub(crate) fn new(army_full: bool) -> RelicChoice {
        RelicChoice { options: relic_choices(army_full), age: 0.0, rare_only: false, army_full, chest: 0 }
    }

    //what a gold chest holds
    pub(crate) fn from_chest(tier: u32, army_full: bool) -> RelicChoice {
        RelicChoice { chest: tier, ..RelicChoice::new(army_full) }
    }

    pub(crate) fn rare() -> RelicChoice {
        RelicChoice { options: rare_choices(), age: 0.0, rare_only: true, army_full: false, chest: 0 }
    }

    //three others, if there are any
    pub(crate) fn reroll(&mut self) {
        let before = self.options;
        for _ in 0..8 {
            self.options = if self.rare_only { rare_choices() } else { relic_choices(self.army_full) };
            if self.options.iter().all(|relic| !before.contains(relic)) || self.rare_only {
                break;
            }
        }
    }
}

//rerolls the player has: one to begin with and another for every three bosses slain
pub(crate) fn rerolls_earned(bosses: usize) -> u32 {
    1 + bosses as u32 / 3
}

//where the three cards are on a screen
pub(crate) fn relic_cards(screen: Vec2) -> [Rect; 3] {
    let gap = 24.0;
    let width = (screen.x * 0.26).clamp(200.0, 360.0).min((screen.x - gap * 2.0 - 32.0) / 3.0);
    let height = (screen.y * 0.44).clamp(240.0, 340.0);
    let left = (screen.x - width * 3.0 - gap * 2.0) / 2.0;
    let top = (screen.y * 0.3).min(screen.y - height - 16.0).max(0.0);
    [0, 1, 2].map(|i| Rect::new(left + (width + gap) * i as f32, top, width, height))
}

//the words broken into lines that are no wider than `max_width` pixels when drawn at `size`
pub(crate) fn wrap_text(text: &str, size: f32, max_width: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if text_width(&format!("{line} {word}"), size, Face::Body) <= max_width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_owned()),
        }
    }
    lines
}

//the choice of relics over a darkened world: three cards with a number, a name, what it does and how many are held
pub(crate) fn draw_relic_choice(choice: &RelicChoice, upgrades: &Upgrades, rerolls: u32) {
    let screen = Vec2::new(screen_width(), screen_height());
    let ready = choice.age >= RELIC_LOCKOUT;
    draw_rectangle(0.0, 0.0, screen.x, screen.y, Color::new(0.0, 0.01, 0.03, 0.78));
    //the heading, with a line of the accent colour under it
    let heading = match (choice.rare_only, choice.chest) {
        (true, _) => "VICTORY",
        (false, 0) => "BOSS SLAIN",
        (false, 1) => "BRONZE CHEST",
        (false, 2) => "SILVER CHEST",
        (false, _) => "GOLD CHEST",
    };
    let heading_size = fit_size(heading, (screen.y * 0.09).round(), screen.x * 0.9, Face::Header);
    draw_heading(heading, screen.x / 2.0, screen.y * 0.16, heading_size, TROPHY);
    let line = (text_width(heading, heading_size, Face::Header) * 0.5).min(screen.x * 0.3);
    draw_line(screen.x / 2.0 - line, screen.y * 0.185, screen.x / 2.0 + line, screen.y * 0.185, 3.0, CYAN);
    draw_label_centered("CHOOSE A RELIC", screen.x / 2.0, screen.y * 0.235, (screen.y * 0.032).round(), CYAN, Face::Bold);
    let mouse = Vec2::from(mouse_position());
    for (i, card) in relic_cards(screen).iter().enumerate() {
        let relic = choice.options[i];
        let hovered = ready && card.contains(mouse);
        let lift = if hovered { 8.0 } else { 0.0 };
        let (x, y) = (card.x, card.y - lift);
        let color = relic.color();
        if hovered {
            draw_rectangle(x - 6.0, y - 6.0, card.w + 12.0, card.h + 12.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.14));
        }
        draw_rectangle(x, y, card.w, card.h, Color::new(0.04, 0.06, 0.1, 0.97));
        draw_rectangle(x, y, card.w, 6.0, color);
        draw_rectangle_lines(x, y, card.w, card.h, if hovered { 3.0 } else { 2.0 }, if hovered { CYAN } else { Color::new(color.r, color.g, color.b, 0.7) });
        draw_key(&format!("{}", i + 1), x + 14.0, y + 38.0, 22.0);
        let rarity = relic.rarity();
        draw_label_right(&rarity.name().to_uppercase(), x + card.w - 14.0, y + 36.0, 16.0, rarity.color(), Face::Bold);
        let name_size = fit_size(relic.name(), (card.h * 0.1).min(30.0).round(), card.w - 24.0, Face::Header);
        draw_label_centered(relic.name(), x + card.w / 2.0, y + card.h * 0.33, name_size, color, Face::Header);
        draw_line(x + card.w * 0.3, y + card.h * 0.38, x + card.w * 0.7, y + card.h * 0.38, 1.5, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.5));
        let blurb_size = (card.h * 0.066).clamp(16.0, 22.0).round();
        for (k, line) in wrap_text(relic.blurb(), blurb_size, card.w - 32.0).iter().enumerate() {
            draw_label_centered(line, x + card.w / 2.0, y + card.h * 0.5 + blurb_size * 1.4 * k as f32, blurb_size, INK, Face::Body);
        }
        let held = upgrades.count(relic);
        if held > 0 {
            draw_label_centered(&format!("you have {held}"), x + card.w / 2.0, y + card.h - 18.0, 16.0, MUTED, Face::Body);
        }
    }
    if ready {
        let fade = 0.6 + 0.4 * (get_time() as f32 * 3.0).sin();
        draw_label_centered("press 1, 2 or 3, or click one", screen.x / 2.0, screen.y * 0.9, (screen.y * 0.036).round(), Color::new(INK.r, INK.g, INK.b, fade), Face::Body);
        let reroll = if rerolls > 0 { format!("[R] reroll ({rerolls} left)") } else { "no rerolls left".to_owned() };
        draw_label_centered(&reroll, screen.x / 2.0, screen.y * 0.95, (screen.y * 0.032).round(), if rerolls > 0 { CYAN } else { MUTED }, Face::Bold);
    }
}

//which card a click at a point is on, if any
pub(crate) fn relic_card_at(screen: Vec2, point: Vec2) -> Option<usize> {
    relic_cards(screen).iter().position(|card| card.contains(point))
}

//the first time an enemy that has noticed the player gets close, a shockwave goes out from them that hurts every enemy near,
//and can't go off again for a while. the cooldown is `wait`, in seconds
pub(crate) fn opening_salvo(enemies: &mut [Enemy], player: Vec2, wait: &mut f32, dt: f32, upgrades: &Upgrades, fx: &mut Effects) {
    *wait = (*wait - dt).max(0.0);
    if upgrades.salvo <= 0.0 || *wait > 0.0 {
        return;
    }
    let started = enemies.iter().any(|enemy| (enemy.chasing || enemy.boss.is_some()) && enemy.behavior != Behavior::Nest && enemy.position.distance(player) - enemy.size / 2.0 <= SALVO_TRIGGER);
    if !started {
        return;
    }
    *wait = SALVO_COOLDOWN;
    for enemy in enemies.iter_mut().filter(|enemy| enemy.position.distance(player) - enemy.size / 2.0 <= SALVO_RADIUS) {
        let share = upgrades.salvo_share() * if enemy.boss.is_some() { 0.1 } else { 1.0 };
        enemy.hurt += enemy.take_damage(enemy.max_health * share);
    }
    fx.ring(player, SALVO_RADIUS, Color::new(1.0, 0.55, 0.2, 1.0));
    fx.add_shake(0.3);
    fx.cue(Cue::Slam);
}

//a unit that died inside an enemy hurts it
pub(crate) fn martyr_blasts(enemies: &mut [Enemy], fallen: &[Vec2], upgrades: &Upgrades) {
    if upgrades.martyr <= 0.0 {
        return;
    }
    for at in fallen.iter().take(400) {
        if let Some(enemy) = enemies.iter_mut().find(|enemy| enemy.health > 0.0 && enemy.overlaps_circle(at.x, at.y, RADIUS)) {
            enemy.hurt += enemy.take_damage(MARTYR_DAMAGE * upgrades.martyr);
        }
    }
}

//every unit gets some health back
pub(crate) fn heal_everyone(units: &mut [Unit], amount: f32) {
    if amount <= 0.0 {
        return;
    }
    for unit in units.iter_mut() {
        let most = unit.max_health();
        if unit.health < most {
            unit.health = (unit.health + amount).min(most);
        }
    }
}
