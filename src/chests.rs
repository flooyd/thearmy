//! the chest a broken horde leaves behind: it lies where the last of the horde fell, and the player or a unit opens it by getting
//! there, and stays there for a few seconds if it is a good one (see CHEST_HOLD_SECONDS), which the enemies near it stop (it is
//! contested, see CHEST_CONTEST_TILES). what is in it depends on how tough the horde was: wall points and recruits for any, a champion
//! as well for a good one, and a choice of relics too for the best. if nobody gets to it in time it is gone

use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Chest {
    pub(crate) at: Vec2,
    pub(crate) tier: u32,   //1 bronze, 2 silver, 3 gold
    pub(crate) points: f32, //the wall points in it
    pub(crate) left: f32,   //seconds until it is gone
    pub(crate) held: f32,   //seconds it has been held for, so far (not kept in a save: a chest that is loaded has to be held again)
    pub(crate) contested: bool, //there are enemies near enough to keep it from being held (only a chest that has to be held can be). worked out every frame, not kept
    pub(crate) stalled: bool,   //and the player or a unit is at it, so the hold is standing still
}

//what a chest gives
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Loot {
    pub(crate) points: f32,
    pub(crate) recruits: usize,
    pub(crate) champion: bool,
    pub(crate) relic: bool,
}

//what a tier of chest is called and its colour
pub(crate) fn tier_name(tier: u32) -> &'static str {
    match tier {
        1 => "BRONZE",
        2 => "SILVER",
        _ => "GOLD",
    }
}

pub(crate) fn tier_color(tier: u32) -> Color {
    match tier {
        1 => Color::new(0.85, 0.55, 0.3, 1.0),
        2 => Color::new(0.78, 0.86, 1.0, 1.0),
        _ => Color::new(1.0, 0.82, 0.25, 1.0),
    }
}

impl Chest {
    //a chest for a horde of this tier that was worth `worth` wall points, put down at `at`
    pub(crate) fn new(at: Vec2, tier: u32, worth: f32) -> Chest {
        let tier = tier.clamp(1, 3);
        Chest { at, tier, points: worth * CHEST_POINTS_PER_TIER * tier as f32, left: CHEST_SECONDS, held: 0.0, contested: false, stalled: false }
    }

    //how long the player or a unit has to stay at it before it opens, 0 if it opens as soon as it is reached
    pub(crate) fn hold(&self) -> f32 {
        CHEST_HOLD_SECONDS[(self.tier.clamp(1, 3) - 1) as usize]
    }

    //what is in it for an army this big: the recruits are a share of the army, more for a better chest
    pub(crate) fn loot(&self, army: usize) -> Loot {
        let recruits = ((army as f32 * CHEST_RECRUIT_SHARE) as usize).clamp(CHEST_RECRUITS_MIN, CHEST_RECRUITS_MAX) * self.tier as usize;
        Loot { points: self.points, recruits, champion: self.tier >= CHEST_CHAMPION_TIER, relic: self.tier >= CHEST_RELIC_TIER }
    }

    pub(crate) fn name(&self) -> &'static str {
        tier_name(self.tier)
    }

    pub(crate) fn color(&self) -> Color {
        tier_color(self.tier)
    }

    //whether it is on show, which it is not for some of every second when it is about to go
    pub(crate) fn shown(&self, time: f32) -> bool {
        self.left > CHEST_BLINK_SECONDS || (time * 5.0).fract() < 0.65
    }
}

//whether an enemy is near enough to a chest to contest it: CHEST_CONTEST_TILES from the edge of one that is awake
pub(crate) fn contested_by(at: Vec2, enemies: &[Enemy]) -> bool {
    let reach = CHEST_CONTEST_TILES * TILE_SIZE;
    enemies.iter().any(|enemy| enemy.contests() && enemy.position.distance(at) <= reach + enemy.size / 2.0)
}

//the same for one player, whose army is `units`: what the tests that have just the one player use
#[cfg(test)]
pub(crate) fn take_reached(chests: &mut Vec<Chest>, player: Vec2, units: &[Unit], enemies: &[Enemy], dt: f32) -> Vec<Chest> {
    let field = FlowField::new();
    let prey = [Prey { position: player, units, team: units.len(), pressure: units.len(), boost: 1.0, cap: DEFAULT_ARMY_LIMIT, field: &field }];
    take_reached_among(chests, &prey, enemies, dt).into_iter().map(|(chest, _)| chest).collect()
}

//the chests that a player or a unit has held for long enough, taken out of the world, each with the player who opened it: of the players
//who are at one, the one whose avatar is nearest. one that is being held fills up for the `dt` seconds that have gone by (by any of
//them), and one that is not gives its hold up faster than it was made. while enemies are near one that has to be held it is
//contested, and the hold stands still: it does not fill, but it is not given up either, as long as someone stays
pub(crate) fn take_reached_among(chests: &mut Vec<Chest>, prey: &[Prey], enemies: &[Enemy], dt: f32) -> Vec<(Chest, usize)> {
    if chests.is_empty() {
        return Vec::new();
    }
    let reach = CHEST_REACH * CHEST_REACH;
    let mut opened = Vec::new();
    chests.retain_mut(|chest| {
        let at_it = |player: &Prey| player.position.distance_squared(chest.at) <= reach || player.units.iter().any(|unit| Vec2::new(unit.x, unit.y).distance_squared(chest.at) <= reach);
        let holder = prey.iter().enumerate().filter(|(_, player)| at_it(player)).min_by(|a, b| a.1.position.distance_squared(chest.at).total_cmp(&b.1.position.distance_squared(chest.at))).map(|(i, _)| i);
        let reached = holder.is_some();
        chest.contested = chest.hold() > 0.0 && contested_by(chest.at, enemies);
        chest.stalled = reached && chest.contested;
        if reached && !chest.stalled {
            chest.held += dt;
        } else if !reached {
            chest.held = (chest.held - dt * CHEST_HOLD_DRAIN).max(0.0);
        }
        let open = reached && !chest.stalled && chest.held >= chest.hold();
        if let (true, Some(holder)) = (open, holder) {
            opened.push((*chest, holder));
        }
        !open
    });
    opened
}

//open a chest for the player who held it: its points are earned, its recruits turn up round it, and the champion, if there is one, is
//won where it lay. `army` and `cap` are how big their army is and can be. returns whether a choice of relics is to be offered as well
pub(crate) fn open_chest(owner: usize, chest: &Chest, army: usize, cap: usize, babies: &mut Vec<Baby>, fx: &mut Effects) -> bool {
    let loot = chest.loot(army);
    let color = chest.color();
    fx.earn(owner, loot.points);
    let before = babies.len();
    spread_babies(babies, chest.at, loot.recruits, army, cap, TILE_SIZE * 2.5);
    let recruits = babies.len() - before;
    if loot.champion {
        fx.recruits.push((chest.at, Odds::Horde(chest.tier), owner));
    }
    let mut what = format!("+{} POINTS", loot.points.round() as i64);
    if recruits > 0 {
        what += &format!("  +{recruits} RECRUITS");
    }
    fx.popup(chest.at, RADIUS * 2.5, 0.0, format!("{} CHEST: {what}", chest.name()), color);
    fx.announce(format!("{} CHEST OPENED", chest.name()), color);
    fx.ring(chest.at, 300.0, color);
    fx.burst(chest.at, color, 70, 420.0);
    fx.add_punch(0.3);
    fx.cue(if loot.relic { Cue::Relic } else { Cue::Liberate });
    loot.relic
}

//the chests in the world, under the army: a box in the colour of its tier with a glow that pulses
pub(crate) fn draw_chests(chests: &[Chest], view: Rect, time: f32) {
    for chest in chests.iter().filter(|chest| chest.shown(time) && view.contains(chest.at)) {
        let color = chest.color();
        let pulse = 0.5 + 0.5 * (time * 3.0).sin();
        draw_circle(chest.at.x, chest.at.y, 62.0 + 8.0 * pulse, Color::new(color.r, color.g, color.b, 0.10 + 0.07 * pulse));
        if chest.hold() > 0.0 {
            //it has to be held: a heavier ring to stand in, and an arc that fills round it as it is, with a glow inside that fills too
            let share = (chest.held / chest.hold()).clamp(0.0, 1.0);
            draw_circle_lines(chest.at.x, chest.at.y, CHEST_REACH, 5.0, Color::new(color.r, color.g, color.b, 0.45 + 0.2 * pulse));
            if chest.contested {
                //enemies near enough to contest it: a red ring as far out as they count from, and the ring to stand in goes red too
                let fast = 0.5 + 0.5 * (time * 9.0).sin();
                draw_circle(chest.at.x, chest.at.y, CHEST_CONTEST_TILES * TILE_SIZE, danger_color(0.05 + 0.04 * fast));
                draw_circle_lines(chest.at.x, chest.at.y, CHEST_CONTEST_TILES * TILE_SIZE, 3.0, danger_color(0.35 + 0.3 * fast));
                draw_circle_lines(chest.at.x, chest.at.y, CHEST_REACH, 5.0, danger_color(0.5 + 0.4 * fast));
            }
            if share > 0.0 {
                draw_circle(chest.at.x, chest.at.y, CHEST_REACH * share, Color::new(color.r, color.g, color.b, 0.12 + 0.12 * share));
            }
        } else {
            draw_circle_lines(chest.at.x, chest.at.y, CHEST_REACH, 2.0, Color::new(color.r, color.g, color.b, 0.35));
        }
        let (w, h) = (54.0, 38.0);
        let (left, top) = (chest.at.x - w / 2.0, chest.at.y - h / 2.0);
        let dark = Color::new(color.r * 0.45, color.g * 0.45, color.b * 0.45, 1.0);
        draw_rectangle(left - 3.0, top - 3.0, w + 6.0, h + 6.0, BLACK);
        draw_rectangle(left, top, w, h, dark);
        draw_rectangle(left, top, w, h * 0.38, color);
        draw_rectangle(left, top + h * 0.38 - 2.0, w, 4.0, BLACK);
        draw_rectangle(chest.at.x - 6.0, top + h * 0.38 - 7.0, 12.0, 15.0, Color::new(1.0, 0.95, 0.7, 1.0));
        draw_rectangle_lines(chest.at.x - 6.0, top + h * 0.38 - 7.0, 12.0, 15.0, 2.0, BLACK);
        let glint = (time * 2.0 + chest.at.x * 0.01).fract();
        draw_circle(left + w * glint, top + 5.0, 2.5, Color::new(1.0, 1.0, 1.0, 1.0 - glint));
    }
}

//how far a chest that is being held has got, on top of the army standing on it (which would hide it): a ring that fills clockwise
//from the top, in a dark track so it reads over the units
pub(crate) fn draw_chest_holds(chests: &[Chest], view: Rect) {
    for chest in chests.iter().filter(|chest| (chest.held > 0.0 || chest.stalled) && chest.hold() > 0.0 && view.contains(chest.at)) {
        let share = (chest.held / chest.hold()).clamp(0.0, 1.0);
        let color = chest.color();
        draw_circle_lines(chest.at.x, chest.at.y, CHEST_REACH, 18.0, Color::new(0.0, 0.0, 0.0, 0.5));
        draw_circle_lines(chest.at.x, chest.at.y, CHEST_REACH, 10.0, Color::new(color.r, color.g, color.b, 0.5));
        let start = -std::f32::consts::FRAC_PI_2;
        //a hold that the enemies have stopped flashes red instead of cream
        let fill = if chest.stalled { danger_color(0.55 + 0.45 * (get_time() as f32 * 9.0).sin().abs()) } else { Color::new(1.0, 1.0, 0.85, 1.0) };
        if share > 0.0 {
            draw_sweep(chest.at, CHEST_REACH, start, std::f32::consts::TAU * share, 18.0, BLACK);
            draw_sweep(chest.at, CHEST_REACH, start, std::f32::consts::TAU * share, 11.0, fill);
        } else if chest.stalled {
            //(none of it made yet: the whole track flashes instead)
            draw_circle_lines(chest.at.x, chest.at.y, CHEST_REACH, 11.0, fill);
        }
    }
}

//a mark for each chest on the minimap: a diamond in its colour, held at the edge of the map when the chest is off it
pub(crate) fn draw_chest_marks(map: &Minimap, chests: &[Chest], time: f32) {
    let rect = map.rect;
    for chest in chests.iter().filter(|chest| chest.shown(time)) {
        let point = map.to_screen(chest.at).max(rect.point() + Vec2::splat(7.0)).min(rect.point() + rect.size() - Vec2::splat(7.0));
        draw_poly(point.x, point.y, 4, 7.0, 0.0, BLACK);
        draw_poly(point.x, point.y, 4, 5.5, 0.0, chest.color());
    }
}

//an arrow at the edge of the screen to the nearest chest, with how many grid squares away it is and how long it has left
pub(crate) fn draw_chest_arrow(chests: &[Chest], player: Vec2, camera: Vec2, zoom: f32) {
    let Some(chest) = chests.iter().min_by(|a, b| a.at.distance_squared(player).total_cmp(&b.at.distance_squared(player))) else {
        return;
    };
    let label = format!("{} CHEST {}  {}s", chest.name(), (chest.at.distance(player) / TILE_SIZE) as i32, chest.left.ceil() as i32);
    draw_edge_arrow(chest.at, camera, zoom, chest.color(), &label);
}
