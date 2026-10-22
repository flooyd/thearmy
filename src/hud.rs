//! the text on screen: the status panel, banners, tooltips and the boss bar

use crate::prelude::*;

//the army, what the run has done so far and what is coming, in the top left of the screen, drawn with the default camera so it
//stays fixed. frame_time is the smoothed time of a frame in seconds
pub(crate) fn draw_hud(x: f32, y: f32, frame_time: f32, units: usize, run: &Run, boost: f32, pickup_range: f32, muted: bool, music_on: bool, formation: Formation, director: &Director, sites: &[Site], cap: usize, horde_line: Option<(String, Color)>) {
    let (left, top) = (12.0, 12.0);
    let pad = 14.0;
    //what goes in the panel, so it can be as wide as its widest line
    let size = 19.0;
    let small = 15.0;
    let rows: Vec<(&str, String)> = vec![
        ("destroyed", grouped(run.destroyed)),
        ("lost", grouped(run.lost)),
        ("bosses slain", format!("{}", run.bosses)),
        ("speed", format!("x{:.1}", boost)),
        ("pickup range", format!("{:.0}", pickup_range)),
    ];
    let (villages, nests, barracks) = site_counts(sites);
    let objectives = [("villages freed", villages), ("nests destroyed", nests), ("barracks taken", barracks)];
    let boss_line = boss_status(director, units);
    let citadel_line = citadel_status(sites);
    let debug = format!("fps {:.0}  {:.1} ms  tile {}, {}", 1.0 / frame_time, frame_time * 1000.0, (x / TILE_SIZE) as i32, (y / TILE_SIZE) as i32);
    let controls = format!("[M] sound {}   [R] recall", if muted { "off" } else { "on" });
    //a fixed width, so the panel doesn't change size with the numbers and the countdown in it
    let width = 410.0;
    let height = pad * 2.0 + 80.0 + 22.0 * ((rows.len() + 1) / 2) as f32 + 36.0 + 26.0 + 22.0 + 18.0 + 22.0 + if citadel_line.is_some() { 22.0 } else { 0.0 } + if horde_line.is_some() { 22.0 } else { 0.0 };
    panel(left, top, width, height);

    //the army: the number that matters, big
    let mut at = top + pad;
    draw_label("UNITS", left + pad, at + 14.0, small, CYAN, Face::Bold);
    draw_label_right(&format!("of {}", grouped(cap)), left + width - pad, at + 14.0, small, MUTED, Face::Body);
    draw_label(&grouped(units), left + pad, at + 52.0, 38.0, INK, Face::Header);
    at += 60.0;
    draw_meter(left + pad, at + 2.0, width - pad * 2.0, 5.0, units as f32 / cap as f32, CYAN);
    at += 20.0;
    //what the run has done, in two columns
    let column = (width - pad * 2.0) / 2.0;
    for (i, (label, value)) in rows.iter().enumerate() {
        let (cx, cy) = (left + pad + column * (i % 2) as f32, at + 22.0 * (i / 2) as f32);
        draw_label(label, cx, cy + 14.0, small, MUTED, Face::Body);
        draw_label(value, cx + text_width(label, small, Face::Body) + 7.0, cy + 14.0, size, INK, Face::Bold);
    }
    at += 22.0 * ((rows.len() + 1) / 2) as f32 + 12.0;
    //what there is to do
    for (i, (label, (done, all))) in objectives.iter().enumerate() {
        let (cx, cy) = (left + pad + column * (i % 2) as f32, at + 22.0 * (i / 2) as f32);
        draw_label(label, cx, cy + 14.0, small, MUTED, Face::Body);
        let value = format!("{done}/{all}");
        draw_label(&value, cx + text_width(label, small, Face::Body) + 7.0, cy + 14.0, size, if done == all && *all > 0 { TROPHY } else { CYAN }, Face::Bold);
    }
    at += 22.0 * ((objectives.len() + 1) / 2) as f32 + 6.0;
    let boss_color = if director.boss.is_some() { ALERT } else { INK };
    draw_label(&boss_line, left + pad, at + 14.0, fit_size(&boss_line, size, width - pad * 2.0, Face::Body), boss_color, Face::Body);
    at += 22.0;
    if let Some(line) = &citadel_line {
        draw_label(line, left + pad, at + 14.0, fit_size(line, size, width - pad * 2.0, Face::Body), TROPHY, Face::Body);
        at += 22.0;
    }
    if let Some((line, color)) = &horde_line {
        draw_label(line, left + pad, at + 14.0, fit_size(line, size, width - pad * 2.0, Face::Body), *color, Face::Body);
        at += 22.0;
    }
    at += 6.0;
    let dim = Color::new(MUTED.r, MUTED.g, MUTED.b, 0.65);
    draw_label(&debug, left + pad, at + 12.0, small, dim, Face::Body);
    draw_label_right(&controls, left + width - pad, at + 12.0, small, dim, Face::Body);
    draw_label(&format!("[F] formation: {}", formation.name().to_lowercase()), left + pad, at + 30.0, small, if formation == Formation::Free { dim } else { CYAN }, Face::Body);
    draw_label_right(&format!("[N] music {}   [Esc] menu", if music_on { "on" } else { "off" }), left + width - pad, at + 30.0, small, dim, Face::Body);
}

//the relics taken, as a row of small tiles above the build bar: the relic's colour and initial, how many of it are held, and
//what the one under the mouse does
pub(crate) fn draw_relic_bar(upgrades: &Upgrades, mouse: Vec2) {
    let held: Vec<Relic> = RELICS.iter().copied().filter(|&relic| upgrades.count(relic) > 0).collect();
    if held.is_empty() {
        return;
    }
    let tile = 34.0;
    let gap = 5.0;
    let left = 12.0;
    let top = relic_bar_top();
    let mut hovered = None;
    for (i, relic) in held.iter().enumerate() {
        let x = left + (tile + gap) * i as f32;
        let rect = Rect::new(x, top, tile, tile);
        let color = relic.color();
        draw_rectangle(x, top, tile, tile, PANEL);
        draw_rectangle(x, top, tile, 4.0, color);
        draw_rectangle_lines(x, top, tile, tile, 1.5, Color::new(color.r, color.g, color.b, 0.8));
        draw_label_centered(relic.tag(), x + tile / 2.0, top + tile * 0.62, 15.0, color, Face::Header);
        let count = upgrades.count(*relic);
        if count > 1 {
            draw_label_right(&format!("x{count}"), x + tile - 3.0, top + tile - 3.0, 12.0, INK, Face::Bold);
        }
        if rect.contains(mouse) {
            hovered = Some((*relic, x));
        }
    }
    if let Some((relic, x)) = hovered {
        let count = upgrades.count(relic);
        let title = if count > 1 { format!("{} x{}", relic.name(), count) } else { relic.name().to_owned() };
        let width = text_width(&title, 20.0, Face::Header).max(text_width(relic.blurb(), 16.0, Face::Body)) + 24.0;
        let (tx, ty) = (x.min(screen_width() - width - 8.0), top - 62.0);
        panel_edged(tx, ty, width, 54.0, relic.color());
        draw_label(&title, tx + 12.0, ty + 24.0, 20.0, relic.color(), Face::Header);
        draw_label(relic.blurb(), tx + 12.0, ty + 45.0, 16.0, INK, Face::Body);
    }
}

//where the row of relics is: over the build bar
pub(crate) fn relic_bar_top() -> f32 {
    screen_height() - BUILD_BAR_HEIGHT - 44.0
}

//what the HUD says about the boss: which is alive, or which is next and when
pub(crate) fn boss_status(director: &Director, army: usize) -> String {
    let def = director.next_def();
    if director.boss.is_some() {
        format!("BOSS: {}", def.name)
    } else if army < def.min_army {
        format!("next boss: {} (from {} units)", def.name, def.min_army)
    } else {
        let seconds = director.next_boss_in.ceil() as usize;
        format!("next boss: {} in {}:{:02}", def.name, seconds / 60, seconds % 60)
    }
}

//what the army is about to be hit by: the worst of the circles on the ground that are counting down to kill what is in them
pub(crate) struct BlastWarning {
    pub(crate) seconds: f32,
    pub(crate) units: usize,       //the most units it will kill, which is the ones inside it or its limit
    pub(crate) player_inside: bool,
}

//the circle that is due to go off that would cost the most units, if it would cost enough (a few, or a hundredth of the army) or
//the player is in it. the quick ones, like a shell, and the ones that do no harm by themselves are not worth a warning
pub(crate) fn blast_warning(hazards: &[Hazard], player: Vec2, units: &[Unit]) -> Option<BlastWarning> {
    let mut worst: Option<BlastWarning> = None;
    for hazard in hazards.iter().filter(|hazard| hazard.armed && hazard.damage >= UNIT_HEALTH && hazard.warn_total >= BLAST_WARNING_MIN_SECONDS) {
        let reach = hazard.radius * hazard.radius;
        let inside = units.iter().filter(|unit| unit.health > 0.0 && unit.garrison.is_none() && Vec2::new(unit.x, unit.y).distance_squared(hazard.position) <= reach).count();
        let at_stake = inside.min(hazard.victims);
        let player_inside = player.distance_squared(hazard.position) <= reach;
        let worth_it = player_inside || at_stake >= (units.len() / 100).max(3);
        let worse = worst.as_ref().is_none_or(|other| (player_inside, at_stake) > (other.player_inside, other.units));
        if worth_it && worse {
            worst = Some(BlastWarning { seconds: hazard.warn.max(0.0), units: at_stake, player_inside });
        }
    }
    worst
}

//a message under the top of the screen telling the player to get out of the circle, and the army with them. it keeps clear of
//the panel at the left, and flashes between white and red so it can be read over the red of the circle
pub(crate) fn draw_blast_warning(warning: &BlastWarning, time: f32) {
    let (w, h) = (screen_width(), screen_height());
    let flash = 0.5 + 0.5 * (time * 10.0).sin();
    let title = if warning.player_inside { "GET OUT OF THE RED!" } else { "THE ARMY IS IN THE RED!" };
    let room = w - HUD_PANEL_RIGHT - 20.0;
    let size = fit_size(title, (h * 0.06).round(), room, Face::Header);
    let left = ((w - text_width(title, size, Face::Header)) / 2.0).max(HUD_PANEL_RIGHT);
    draw_label_shadowed(title, left, h * 0.14, size, Color::new(1.0, 0.45 + 0.55 * flash, 0.4 + 0.5 * flash, 1.0), Face::Header);
    let line = format!("{} units will die in {:.1}s - walk clear, [R] recalls squads", grouped(warning.units), warning.seconds);
    let small = fit_size(&line, (h * 0.036).round(), room, Face::Bold);
    let left = ((w - text_width(&line, small, Face::Bold)) / 2.0).max(HUD_PANEL_RIGHT);
    let baseline = h * 0.14 + size * 0.85;
    let width = text_width(&line, small, Face::Bold);
    draw_rectangle(left - 14.0, baseline - small * 1.05, width + 28.0, small * 1.5, Color::new(0.0, 0.02, 0.06, 0.7));
    draw_label_shadowed(&line, left, baseline, small, INK, Face::Bold);
}

//what the HUD says while the citadel is out of the way: when its lord is back
pub(crate) fn citadel_status(sites: &[Site]) -> Option<String> {
    let site = sites.iter().find(|site| site.kind == SiteKind::Citadel && site.state == SiteState::Cleared)?;
    Some(format!("the Sovereign returns in {}", clock(site.clock.ceil())))
}

pub(crate) const MILESTONES: [usize; 7] = [100, 250, 500, 1000, 2000, 3000, MAX_UNITS]; //army sizes the player is congratulated on

//the next army size to congratulate the player on once the army is `size`: the round ones, and then each time the army gets
//BARRACKS_UNITS past the usual cap, which only Barracks relics make room for
pub(crate) fn next_milestone(size: usize) -> usize {
    MILESTONES.iter().copied().find(|&milestone| milestone > size).unwrap_or_else(|| (size - MAX_UNITS) / BARRACKS_UNITS * BARRACKS_UNITS + MAX_UNITS + BARRACKS_UNITS)
}
pub(crate) const BANNER_SECONDS: f32 = 3.0; //how long a message stays up
pub(crate) const MAX_WAITING_BANNERS: usize = 4; //messages that can be waiting for the one on screen to go, any more are dropped

//a message for the top middle of the screen
pub(crate) struct Banner {
    pub(crate) text: String,
    pub(crate) color: Color,
    pub(crate) age: f32,  //seconds it has been up
    pub(crate) life: f32, //seconds it stays up
}

//the messages to show, one at a time. with more waiting, each is up for a shorter time
pub(crate) struct Banners {
    pub(crate) shown: Option<Banner>,
    pub(crate) waiting: Vec<Banner>,
}

impl Banners {
    pub(crate) fn new() -> Banners {
        Banners { shown: None, waiting: Vec::new() }
    }

    pub(crate) fn push(&mut self, text: String, color: Color) {
        if self.waiting.len() < MAX_WAITING_BANNERS {
            self.waiting.push(Banner { text, color, age: 0.0, life: BANNER_SECONDS });
        }
    }

    pub(crate) fn update(&mut self, dt: f32) {
        if let Some(banner) = self.shown.as_mut() {
            banner.age += dt;
        }
        if self.shown.as_ref().is_some_and(|banner| banner.age > banner.life) {
            self.shown = None;
        }
        if self.shown.is_none() && !self.waiting.is_empty() {
            let mut next = self.waiting.remove(0);
            if !self.waiting.is_empty() {
                next.life = BANNER_SECONDS * 0.55;
            }
            self.shown = Some(next);
        }
    }
}

//a big message in the top middle of the screen that fades in, and out again at the end of its time, with a line of the accent
//colour that grows under it. a message too long for the screen is drawn smaller
pub(crate) fn draw_banner(banner: &Banner) {
    let fade_in = (banner.age / 0.3).min(1.0);
    let alpha = (fade_in * ((banner.life - banner.age) / 0.8).clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let size = fit_size(&banner.text, (screen_height() * 0.07).round(), screen_width() * 0.92, Face::Header);
    let width = text_width(&banner.text, size, Face::Header);
    //it drops into place as it appears
    let y = screen_height() * 0.3 - (1.0 - fade_in) * 24.0;
    let color = Color::new(banner.color.r, banner.color.g, banner.color.b, alpha);
    draw_label_shadowed(&banner.text, (screen_width() - width) / 2.0, y, size, color, Face::Header);
    let line = width * 0.5 * fade_in.powi(2);
    let middle = screen_width() / 2.0;
    draw_line(middle - line, y + size * 0.3, middle + line, y + size * 0.3, 3.0, Color::new(CYAN.r, CYAN.g, CYAN.b, alpha * 0.9));
}

//what is notable about an enemy, in a few words each: what it does, and what it takes less damage from
pub(crate) fn traits(enemy: &Enemy) -> Vec<String> {
    let mut traits = Vec::new();
    if enemy.boss.is_some() {
        traits.push("boss".to_owned());
    }
    if let Some(state) = enemy.boss.as_ref() {
        if state.def.flank {
            traits.push("armoured front".to_owned());
        }
        if state.def.splits {
            traits.push("splits in two".to_owned());
        }
    }
    if enemy.is_minion() {
        traits.push("minion, worth nothing".to_owned());
    }
    match enemy.behavior {
        Behavior::Walker => {}
        Behavior::Shooter => traits.push("shoots".to_owned()),
        Behavior::Splitter => traits.push("splits when destroyed".to_owned()),
        Behavior::Charger => traits.push("charges".to_owned()),
        Behavior::Bomber => traits.push("explodes on contact".to_owned()),
        Behavior::Healer => traits.push("mends other enemies".to_owned()),
        Behavior::Summoner => traits.push("calls up imps".to_owned()),
        Behavior::Mortar => traits.push("lobs shells".to_owned()),
        Behavior::Nest => traits.push("sends out more of its own".to_owned()),
    }
    if enemy.aggro > 0.0 && enemy.behavior != Behavior::Charger && enemy.boss.is_none() {
        traits.push("chases".to_owned());
    }
    if enemy.armor > 0.0 {
        traits.push(format!("armor {:.0}%", enemy.armor * 100.0));
    }
    if enemy.max_shield > 0.0 {
        traits.push("shielded".to_owned());
    }
    match enemy.modifier {
        Modifier::None | Modifier::Armored => {}
        Modifier::Swift => traits.push("fast".to_owned()),
        Modifier::Volatile => traits.push("explodes when destroyed".to_owned()),
        Modifier::Frenzied => traits.push("hits twice as hard".to_owned()),
    }
    traits
}

//what the mouse is over, when it is an enemy: its kind, health and attack, what it does, and the squad a click would send.
//free_units is how many units are not already sent at something. it keeps clear of `avoid`, so it doesn't cover what it is about
pub(crate) fn draw_enemy_tooltip(enemy: &Enemy, free_units: usize, avoid: Rect) {
    let size = 18.0;
    let wanted = squad_size(enemy);
    let (send, send_color) = if wanted <= free_units {
        (format!("click: send {wanted} units"), SQUAD_COLOR)
    } else {
        (format!("click: send {free_units} units (wants {wanted})"), ALERT)
    };
    let shield = if enemy.shield > 0.0 { format!("  shield {}", compact_number(enemy.shield)) } else { String::new() };
    let title_color = if enemy.modifier == Modifier::None { INK } else { enemy.modifier.color() };
    let mut lines = vec![
        (format!("health {}/{}{}", compact_number(enemy.health.max(0.0)), compact_number(enemy.max_health), shield), INK),
        (format!("hits for {}/s", compact_number(enemy.attack)), MUTED),
    ];
    let traits = traits(enemy);
    if !traits.is_empty() {
        lines.push((traits.join(", "), Color::new(1.0, 0.85, 0.5, 1.0)));
    }
    lines.push((send, send_color));
    let title = enemy.title();
    let width = lines.iter().map(|(line, _)| text_width(line, size, Face::Body)).fold(text_width(&title, 21.0, Face::Header), f32::max);
    let line_height = size * 1.35;
    let height = 38.0 + line_height * lines.len() as f32 + 8.0;
    let (mouse_x, mouse_y) = mouse_position();
    let panel_width = width + 28.0;
    let mut x = (mouse_x + 18.0).min(screen_width() - panel_width - 8.0).max(0.0);
    let y = (mouse_y + 18.0).min(screen_height() - height - 4.0).max(0.0);
    if Rect::new(x, y, panel_width, height).overlaps(&avoid) {
        x = (avoid.x - panel_width - 8.0).max(0.0);
    }
    panel_edged(x, y, panel_width, height, if enemy.boss.is_some() { enemy.color } else { CYAN });
    draw_label(&title, x + 14.0, y + 28.0, 21.0, title_color, Face::Header);
    for (i, (line, color)) in lines.iter().enumerate() {
        draw_label(line, x + 14.0, y + 38.0 + line_height * (i as f32 + 0.85), size, *color, Face::Body);
    }
}

//the boss's health along the bottom of the screen: its name and phase, the bar with a tick where each phase begins, its
//shield over the top in blue, and a note while it is between phases. `family_share` is how much of it, and of whatever it split
//into, is left, and `alive` how many of those there are
pub(crate) fn draw_boss_bar(boss: &Enemy, family_share: f32, alive: usize) {
    let Some(state) = boss.boss.as_ref() else {
        return;
    };
    let width = (screen_width() * 0.45).clamp(300.0, 680.0);
    let height = 20.0;
    let x = (screen_width() - width) / 2.0;
    let y = screen_height() - 38.0;
    let phase = &state.def.phases[state.phase];
    let name = if alive > 1 { format!("{}  x{}", state.def.name, alive) } else { state.def.name.to_owned() };
    let name_size = fit_size(&name, 26.0, width * 0.6, Face::Header);
    draw_label_shadowed(&name, x, y - 12.0, name_size, boss.color, Face::Header);
    draw_label_right(&phase.name.to_uppercase(), x + width, y - 12.0, 16.0, CYAN, Face::Bold);
    draw_rectangle(x - 3.0, y - 3.0, width + 6.0, height + 6.0, Color::new(0.0, 0.0, 0.0, 0.8));
    draw_rectangle(x, y, width * family_share, height, boss.color);
    draw_rectangle(x, y, width * family_share, height * 0.35, Color::new(1.0, 1.0, 1.0, 0.18));
    if boss.max_shield > 0.0 && boss.shield > 0.0 {
        let shield_share = (boss.shield / boss.max_health).min(1.0);
        draw_rectangle(x, y, width * shield_share, height * 0.4, Color::new(0.4, 0.85, 1.0, 0.95));
    }
    for phase in state.def.phases.iter().skip(1).filter(|_| !state.def.splits) {
        draw_rectangle(x + width * phase.below - 1.0, y, 2.0, height, Color::new(1.0, 1.0, 1.0, 0.7));
    }
    draw_rectangle_lines(x - 3.0, y - 3.0, width + 6.0, height + 6.0, 2.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.8));
    let note = if state.shifting > 0.0 {
        Some("SHIFTING - CAN'T BE HURT".to_owned())
    } else if state.def.flank {
        Some("FRONT ARMOURED - HIT IT FROM BEHIND".to_owned())
    } else if boss.armor > 0.0 {
        Some(format!("ARMOR {:.0}%", boss.armor * 100.0))
    } else {
        None
    };
    if let Some(note) = note {
        draw_label_centered(&note, screen_width() / 2.0, y + height - 5.0, 15.0, INK, Face::Bold);
    }
}

//an arrow at the edge of the screen pointing at the boss when it is off screen, with how many grid squares away it is
pub(crate) fn draw_boss_arrow(boss: &Enemy, player: Vec2, zoom: f32) {
    let half = Vec2::new(screen_width(), screen_height()) / 2.0;
    let offset = (boss.position - player) * zoom;
    let reach = boss.size / 2.0 * zoom;
    if offset.x.abs() - reach < half.x && offset.y.abs() - reach < half.y {
        return;
    }
    let margin = 44.0;
    let direction = offset.normalize_or_zero();
    let along = ((half.x - margin) / direction.x.abs().max(0.0001)).min((half.y - margin) / direction.y.abs().max(0.0001));
    let at = half + direction * along;
    let side = direction.perp();
    let pulse = 0.7 + 0.3 * (get_time() as f32 * 6.0).sin();
    let color = Color::new(boss.color.r, boss.color.g, boss.color.b, pulse);
    draw_triangle(at + direction * 22.0, at - direction * 12.0 + side * 16.0, at - direction * 12.0 - side * 16.0, Color::new(0.0, 0.0, 0.0, 0.6));
    draw_triangle(at + direction * 20.0, at - direction * 11.0 + side * 14.0, at - direction * 11.0 - side * 14.0, color);
    let label = format!("{}", (boss.position.distance(player) / TILE_SIZE) as i32);
    let width = text_width(&label, 20.0, Face::Bold);
    let label_at = (at - direction * 36.0 - Vec2::new(width / 2.0, -8.0)).max(Vec2::splat(4.0)).min(Vec2::new(screen_width() - width - 4.0, screen_height() - 4.0));
    draw_label_shadowed(&label, label_at.x, label_at.y, 20.0, CYAN, Face::Bold);
}

//how many villages are free, nests destroyed and barracks taken, out of those the world has: (done, all) of each
pub(crate) fn site_counts(sites: &[Site]) -> ((usize, usize), (usize, usize), (usize, usize)) {
    let count = |wanted: fn(&SiteKind) -> bool| {
        let all = sites.iter().filter(|site| wanted(&site.kind)).count();
        let done = sites.iter().filter(|site| wanted(&site.kind) && site.state == SiteState::Cleared).count();
        (done, all)
    };
    (count(|kind| *kind == SiteKind::Village), count(|kind| matches!(kind, SiteKind::Nest(_))), count(|kind| *kind == SiteKind::Barracks))
}
