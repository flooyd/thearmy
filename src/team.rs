//! what a game with others in it shows of them: whose army is whose (each in the colour of its player), their names over them, the ones who are off the screen
//! as arrows at its edge, a list of everyone with how big their army is, and what happens to a player whose army has fallen

use crate::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

//how far an army's colour goes towards the colour of its player (units are white, and a player's own colour is what they would be if all of it went)
pub(crate) const TEAM_TINT: f32 = 0.5;
const PANEL_WIDTH: f32 = 290.0;
const PANEL_ROW: f32 = 27.0;

//an army's colour in a game with others: the units' own, a way towards their player's
pub(crate) fn team_color(color: Color, owner: Color) -> Color {
    lerp_color(color, owner, TEAM_TINT)
}

//a game with others in it, as what draws the armies and the map is told of it
pub(crate) struct Team<'a> {
    pub(crate) colors: &'a [Color], //the colour of each player
    pub(crate) mates: &'a [Mate],
}

//everything the others are shown of a player
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Mate {
    pub(crate) name: String,
    pub(crate) color: Color,
    pub(crate) position: Vec2,
    pub(crate) army: usize,
    pub(crate) you: bool,
    pub(crate) choosing: bool,       //they have relics to choose from
    pub(crate) ping_ms: Option<u16>, //how long a message takes to get to them and back, if they are at another machine
    pub(crate) away: bool,           //they have gone from the game, and their army is held where it was for them to come back to
}

//the players, as they are shown to each other. `sizes` is how big each army is, `members` who is on a connection (it says how far away each is)
pub(crate) fn mates_of(players: &Players, sizes: &[usize], members: &[Member], me: usize) -> Vec<Mate> {
    players
        .list
        .iter()
        .enumerate()
        .map(|(i, player)| {
            let member = members.iter().find(|member| member.name.eq_ignore_ascii_case(&player.name));
            Mate {
                name: player.name.clone(),
                color: player.color,
                position: player.position,
                army: sizes.get(i).copied().unwrap_or(0),
                you: i == me,
                choosing: player.choice.is_some(),
                //(the host is the one that has no ping: there is nothing between it and the world)
                ping_ms: member.filter(|member| member.slot != 0 && i != me).map(|member| member.ping_ms),
                away: i != 0 && member.is_none(),
            }
        })
        .collect()
}

//what the army of each player is drawn in, for a game with more than one
pub(crate) fn team_colors(players: &Players) -> Vec<Color> {
    players.list.iter().map(|player| player.color).collect()
}

//a lighter shade of a colour, for words over the game
fn light(color: Color) -> Color {
    Color::new((color.r + 1.0) / 2.0, (color.g + 1.0) / 2.0, (color.b + 1.0) / 2.0, 1.0)
}

//the names of the others over them, and what they are doing if it is worth knowing (the arrows at the edge of the screen are for the ones that are off it).
//drawn with the default camera, `camera` being where in the world the middle of the screen is
pub(crate) fn draw_tags(mates: &[Mate], camera: Vec2, zoom: f32) {
    let half = Vec2::new(screen_width(), screen_height()) / 2.0;
    for mate in mates.iter().filter(|mate| !mate.you && !mate.away) {
        let at = half + (mate.position - camera) * zoom;
        if at.x < -120.0 || at.x > screen_width() + 120.0 || at.y < -60.0 || at.y > screen_height() + 60.0 {
            continue;
        }
        let size = 18.0;
        let width = text_width(&mate.name, size, Face::Bold);
        let y = at.y - RUNNER_TOP * zoom - 8.0;
        draw_label_shadowed(&mate.name, at.x - width / 2.0, y, size, light(mate.color), Face::Bold);
        if mate.choosing {
            let line = "CHOOSING A RELIC";
            draw_label_shadowed(line, at.x - text_width(line, 13.0, Face::Bold) / 2.0, y - 19.0, 13.0, TROPHY, Face::Bold);
        }
    }
}

//how close (in pixels) two arrows at the edge of the screen can be before they share one, or their names would be written over each other
const SHARED_ARROW: f32 = 110.0;

//the others who are off the screen, as the arrows that point at them: those whose arrows would be one on top of another are one arrow with all their names, at the first of them
pub(crate) fn arrow_groups(mates: &[Mate], camera: Vec2, zoom: f32, screen: Vec2) -> Vec<(Vec2, Color, String)> {
    let mut groups: Vec<(Vec2, Vec2, Color, Vec<String>)> = Vec::new();
    for mate in mates.iter().filter(|mate| !mate.you && !mate.away) {
        let Some((spot, _)) = edge_spot(mate.position, camera, zoom, screen) else { continue };
        let name = mate.name.to_uppercase();
        match groups.iter_mut().find(|(_, first, ..)| first.distance(spot) < SHARED_ARROW) {
            Some((.., names)) => names.push(name),
            None => groups.push((mate.position, spot, mate.color, vec![name])),
        }
    }
    groups.into_iter().map(|(at, _, color, names)| (at, color, names.join(", "))).collect()
}

//an arrow at the edge of the screen to each of the others who is off it, with their name
pub(crate) fn draw_mate_arrows(mates: &[Mate], camera: Vec2, zoom: f32) {
    for (at, color, label) in arrow_groups(mates, camera, zoom, Vec2::new(screen_width(), screen_height())) {
        draw_edge_arrow(at, camera, zoom, color, &label);
    }
}

//where the list of players is, in the top right
pub(crate) fn panel_rect(count: usize, screen: Vec2) -> Rect {
    let width = PANEL_WIDTH.min(screen.x * 0.4);
    Rect::new(screen.x - width - 12.0, 12.0, width, 34.0 + PANEL_ROW * count as f32)
}

//how many players the list has in it this frame, which the arrows at the edge of the screen need to know to keep off it
static LISTED: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn list_players(count: usize) {
    LISTED.store(count, Ordering::Relaxed);
}

//the list of players, if there is one on the screen
pub(crate) fn team_panel(screen: Vec2) -> Option<Rect> {
    let count = LISTED.load(Ordering::Relaxed);
    (count >= 2).then(|| panel_rect(count, screen))
}

//the list: a row for each player, with their colour, their name, how far away they are, and how big their army is
pub(crate) fn draw_team_panel(mates: &[Mate]) {
    if mates.len() < 2 {
        return;
    }
    let screen = Vec2::new(screen_width(), screen_height());
    let rect = panel_rect(mates.len(), screen);
    panel(rect.x, rect.y, rect.w, rect.h);
    draw_label("PLAYERS", rect.x + 14.0, rect.y + 22.0, 14.0, CYAN, Face::Bold);
    for (i, mate) in mates.iter().enumerate() {
        let top = rect.y + 30.0 + PANEL_ROW * i as f32;
        let middle = top + PANEL_ROW / 2.0;
        let dimmed = if mate.away { 0.45 } else { 1.0 };
        let color = Color::new(mate.color.r, mate.color.g, mate.color.b, dimmed);
        draw_circle(rect.x + 22.0, middle, 7.0, color);
        draw_circle_lines(rect.x + 22.0, middle, 7.0, 1.5, Color::new(0.0, 0.0, 0.0, 0.7));
        let right = rect.x + rect.w - 14.0;
        let (count, tone) = if mate.away {
            ("AWAY".to_owned(), MUTED)
        } else if mate.army == 0 {
            ("FALLEN".to_owned(), ALERT)
        } else {
            (grouped(mate.army), INK)
        };
        draw_label_right(&count, right, middle + 6.0, 17.0, tone, Face::Bold);
        let room = rect.w - 40.0 - text_width(&count, 17.0, Face::Bold) - 24.0;
        let name = if mate.you { format!("{} (you)", mate.name) } else { mate.name.clone() };
        let size = fit_size(&name, 17.0, room, Face::Bold);
        draw_label(&name, rect.x + 38.0, middle + 6.0, size, if mate.away { MUTED } else { INK }, Face::Bold);
        let after = rect.x + 38.0 + text_width(&name, size, Face::Bold) + 8.0;
        let note = match (mate.choosing, mate.ping_ms) {
            (true, _) => Some(("relic".to_owned(), TROPHY)),
            (false, Some(ping)) if !mate.away => Some((format!("{ping} ms"), Color::new(MUTED.r, MUTED.g, MUTED.b, 0.8))),
            _ => None,
        };
        if let Some((text, tone)) = note {
            if after + text_width(&text, 12.0, Face::Body) < right - text_width(&count, 17.0, Face::Bold) - 8.0 {
                draw_label(&text, after, middle + 5.0, 12.0, tone, Face::Body);
            }
        }
    }
}

//a ring round each champion in the colour of the player it is with, so that whose it is can be told at a glance
pub(crate) fn draw_champion_rings(units: &[Unit], colors: &[Color], view: Rect, zoom: f32) {
    let pixel = 1.0 / zoom;
    for unit in units.iter().filter(|unit| unit.champion != 0 && unit.garrison.is_none()) {
        let at = Vec2::new(unit.x, unit.y);
        if !view.contains(at) {
            continue;
        }
        let color = colors[owner_index(unit.owner).min(colors.len() - 1)];
        draw_circle_lines(unit.x, unit.y, unit.radius() + 10.0 * pixel.max(0.6), 3.0 * pixel.max(0.6), color);
    }
}

//how long the host can be silent before the player is told of it (it says something several times a second)
pub(crate) const QUIET_SECONDS: f32 = 1.5;

//the warning that nothing is coming from the host, at the top of the screen
pub(crate) fn draw_trouble(quiet: f32) {
    let w = screen_width();
    let line = format!("NO WORD FROM THE HOST FOR {:.0} SECONDS", quiet.floor());
    let size = (screen_height() * 0.03).round().max(16.0);
    let size = fit_size(&line, size, w * 0.9, Face::Bold);
    let width = text_width(&line, size, Face::Bold) + 40.0;
    let y = screen_height() * 0.12;
    draw_rectangle(w / 2.0 - width / 2.0, y - size * 1.2, width, size * 1.9, Color::new(0.0, 0.0, 0.0, 0.7));
    let pulse = 0.75 + 0.25 * (get_time() as f32 * 6.0).sin();
    draw_label_centered(&line, w / 2.0, y, size, Color::new(ALERT.r, ALERT.g, ALERT.b, pulse), Face::Bold);
}

//what is said over the game while a player has no army: the world goes on, and a few units come to their side in a moment
pub(crate) fn draw_fallen(seconds_left: f32, since: f32) {
    let fade = (since / 0.4).clamp(0.0, 1.0);
    let (w, h) = (screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.45 * fade));
    let size = (h * 0.07).round();
    let title = "YOUR ARMY HAS FALLEN";
    draw_label_centered(title, w / 2.0, h * 0.42, fit_size(title, size, w * 0.9, Face::Header), Color::new(ALERT.r, ALERT.g, ALERT.b, fade), Face::Header);
    let line = format!("a few units come to your side in {}", seconds_left.ceil().max(0.0) as u32);
    let small = (h * 0.032).round();
    draw_label_centered(&line, w / 2.0, h * 0.42 + small * 2.2, fit_size(&line, small, w * 0.9, Face::Bold), Color::new(INK.r, INK.g, INK.b, fade), Face::Bold);
}
