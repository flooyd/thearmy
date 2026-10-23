//! the multiplayer screens: the form where a player says who they are and where to, the lobby a host waits in while others join, and the one a player
//! waits in once they have

use crate::prelude::*;

const LOG_LINES: usize = 5; //how much of what has happened a lobby remembers
const MAX_EARLY_MESSAGES: usize = 64; //how many messages of the world a player waiting in a lobby keeps (the game takes them as soon as it opens)

// ---------- the form ----------

//what a player does in multiplayer. nobody is ever asked for, or shown, an address: a game is hosted on the lobby list and joined from it, and the one other
//way in is a game on this very computer
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Role {
    Host,  //a game that goes on the lobby list
    Join,  //one of the games on the list
    Local, //a game that is hosted on this computer, in another window
}

const ROLES: [Role; 3] = [Role::Host, Role::Join, Role::Local];

impl Role {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Role::Host => "host",
            Role::Join => "join",
            Role::Local => "local",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Role> {
        ROLES.into_iter().find(|role| role.name() == name)
    }

    //the next one along (round to the first), the one after it (stopping at the last) or the one before it (stopping at the first)
    pub(crate) fn next(self) -> Role {
        ROLES[(ROLES.iter().position(|&role| role == self).unwrap_or(0) + 1) % ROLES.len()]
    }

    pub(crate) fn before(self) -> Role {
        ROLES[ROLES.iter().position(|&role| role == self).unwrap_or(0).saturating_sub(1)]
    }

    pub(crate) fn after(self) -> Role {
        ROLES[(ROLES.iter().position(|&role| role == self).unwrap_or(0) + 1).min(ROLES.len() - 1)]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Row {
    Role,
    Name,
    Password,
    Port, //the port to host on, or the port of the game on this computer
}

//(someone joining a game from the list is asked for the password when they have picked one, and has no port to give)
const HOST_ROWS: [Row; 4] = [Row::Role, Row::Name, Row::Password, Row::Port];
const JOIN_ROWS: [Row; 2] = [Row::Role, Row::Name];

//the rows of the form for what the player is doing
pub(crate) fn rows(role: Role) -> &'static [Row] {
    match role {
        Role::Host | Role::Local => &HOST_ROWS,
        Role::Join => &JOIN_ROWS,
    }
}

//what the form asks for, once it is filled in right
pub(crate) enum Submit {
    Host { config: HostConfig, list_on: String }, //and the lobby server to put the game on
    Join { name: String },                        //to see the games on the list: the password is asked for once one is picked
    Local { target: String, name: String, password: String },
}

pub(crate) struct Form {
    pub(crate) role: Role,
    pub(crate) name: TextField,
    pub(crate) password: TextField,
    pub(crate) port: TextField,
    pub(crate) server: String, //the address of the lobby server (not asked for: see directory.rs)
    pub(crate) focus: Row,
    pub(crate) reveal: bool, //whether the password is shown as it is typed
    pub(crate) problem: Option<String>,
}

impl Form {
    //filled in with what the player used last time (except the password, which is never kept)
    pub(crate) fn new(settings: &Settings) -> Form {
        let role = settings.multiplayer_role;
        Form {
            role,
            name: TextField::new(&settings.player_name, MAX_NAME_LENGTH, Allowed::Name),
            password: TextField::new("", MAX_PASSWORD, Allowed::Password),
            port: TextField::new(&settings.host_port.to_string(), 5, Allowed::Port),
            server: lobby_server(),
            focus: if settings.player_name.is_empty() || role == Role::Join { Row::Name } else { Row::Password },
            reveal: false,
            problem: None,
        }
    }

    pub(crate) fn field(&mut self, row: Row) -> Option<&mut TextField> {
        match row {
            Row::Role => None,
            Row::Name => Some(&mut self.name),
            Row::Password => Some(&mut self.password),
            Row::Port => Some(&mut self.port),
        }
    }

    //move the focus down (1) or up (-1) the rows, round to the other end
    pub(crate) fn step(&mut self, by: i32) {
        let rows = rows(self.role);
        let at = rows.iter().position(|&row| row == self.focus).unwrap_or(0) as i32;
        self.focus = rows[(at + by).rem_euclid(rows.len() as i32) as usize];
    }

    pub(crate) fn switch(&mut self, role: Role) {
        self.role = role;
        self.problem = None;
        //(a player joining from the list has no use for the password or the port, and is not left on them)
        if !rows(role).contains(&self.focus) {
            self.focus = Row::Name;
        }
    }

    //Enter: on the last row it asks to go ahead (true), on the others it moves on to the next
    pub(crate) fn enter(&mut self) -> bool {
        if rows(self.role).last() == Some(&self.focus) {
            return true;
        }
        self.step(1);
        false
    }

    //what is asked for, or what is wrong with it in words the player can act on
    pub(crate) fn submit(&self) -> Result<Submit, String> {
        let name = clean_name(self.name.text());
        if name.is_empty() {
            return Err("type the name you want to be called".to_owned());
        }
        if self.role == Role::Join {
            return Ok(Submit::Join { name });
        }
        if let Some(problem) = password_problem(self.password.text()) {
            return Err(problem);
        }
        let password = self.password.text().to_owned();
        let port = parse_port(self.port.text())?;
        match self.role {
            Role::Local => Ok(Submit::Local { target: format!("127.0.0.1:{port}"), name, password }),
            _ => Ok(Submit::Host { config: HostConfig::new(&name, &password, port), list_on: self.listing()? }),
        }
    }

    //the lobby server a game that is hosted goes on. one that is not an address is a problem to tell the player of, as the game would never be on the list
    fn listing(&self) -> Result<String, String> {
        let server = self.server.trim();
        games_url(server)?;
        Ok(server.to_owned())
    }

    //what is worth keeping for next time, once it has been used
    pub(crate) fn remember(&self, settings: &mut Settings) {
        settings.player_name = clean_name(self.name.text());
        settings.multiplayer_role = self.role;
        if self.role != Role::Join {
            settings.host_port = parse_port(self.port.text()).unwrap_or(settings.host_port);
        }
    }
}

// ---------- what a lobby is told ----------

//a line for the log of a host's lobby (the player who was turned away is not named by where they came from)
pub(crate) fn log_line(event: &HostEvent) -> (String, Color) {
    match event {
        HostEvent::Joined { name, .. } => (format!("{name} joined"), CYAN),
        HostEvent::Left { name, why, .. } => (format!("{name} left ({why})"), MUTED),
        HostEvent::TurnedAway { why, .. } => (format!("someone was turned away: {why}"), ALERT),
    }
}

//a host's lobby is a roster and what has happened to it
struct HostPage {
    host: NetHost,
    log: Vec<(String, Color)>,
    listing: Option<Result<(), String>>, //how the game is doing on the lobby list: whether it is on it, or what is wrong. None while it waits to hear
}

impl HostPage {
    fn new(host: NetHost) -> HostPage {
        HostPage { host, log: Vec::new(), listing: None }
    }

    fn absorb(&mut self) {
        for event in self.host.poll() {
            self.log.push(log_line(&event));
        }
        self.host.announce(false);
        for news in self.host.listing_news() {
            match news {
                DirectoryEvent::Listed(_) => self.listing = Some(Ok(())),
                DirectoryEvent::Unlisted(problem) => self.listing = Some(Err(problem)),
                DirectoryEvent::Games(_) | DirectoryEvent::Unreachable(_) => {}
            }
        }
        let extra = self.log.len().saturating_sub(LOG_LINES);
        self.log.drain(..extra);
    }
}

//what a player who has joined knows of the game they are in
#[derive(Default)]
pub(crate) struct Lobby {
    pub(crate) joined: bool,
    pub(crate) slot: u8,
    pub(crate) members: Vec<Member>,
    pub(crate) started: bool, //the host has started the run
    pub(crate) world: Vec<Vec<u8>>, //what the host has said of the world, which is kept for the game that follows
}

impl Lobby {
    //an event from the host. what ends the stay comes back as the words to tell the player
    pub(crate) fn apply(&mut self, event: ClientEvent) -> Option<String> {
        match event {
            ClientEvent::Joined { slot, members } => {
                self.joined = true;
                self.slot = slot;
                self.members = members;
                None
            }
            ClientEvent::Roster(members) => {
                self.members = members;
                None
            }
            ClientEvent::Started => {
                self.started = true;
                None
            }
            ClientEvent::World(message) => {
                //(the first of them can come with the news that the run has started, and every one is needed)
                if self.world.len() < MAX_EARLY_MESSAGES {
                    self.world.push(message);
                }
                None
            }
            ClientEvent::Refused(refusal) => Some(refusal.text()),
            ClientEvent::Failed(problem) => Some(problem),
            ClientEvent::Closed(why) => Some(why),
        }
    }
}

struct JoinPage {
    client: NetClient,
    to: String,                     //who is being joined, as the screen says it
    asked: Option<Box<PasswordPage>>, //the password page it came from, when a game on the list was picked, to go back to if it does not work out
    lobby: Lobby,
}

enum Screen {
    Form,
    Browse(Box<BrowsePage>),
    Password(Box<PasswordPage>),
    Host(Box<HostPage>),
    Join(Box<JoinPage>),
}

// ---------- drawing ----------

//the room the screens are drawn in: centred, and no wider than it reads well
struct Frame {
    left: f32,
    width: f32,
    row: f32,  //how tall a line of the form is
    size: f32, //the text in it
}

fn frame_of(screen: Vec2) -> Frame {
    let width = (screen.x * 0.92).min(680.0);
    let row = (screen.y * 0.075).clamp(34.0, 60.0);
    Frame { left: (screen.x - width) / 2.0, width, row, size: (row * 0.46).round() }
}

fn baseline(rect: Rect, size: f32) -> f32 {
    rect.y + rect.h / 2.0 + size * 0.35
}

fn draw_button(rect: Rect, label: &str, key: &str, mouse: Vec2, tone: Color, enabled: bool) {
    let hot = enabled && rect.contains(mouse);
    let tone = if enabled { tone } else { Color::new(MUTED.r, MUTED.g, MUTED.b, 0.45) };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(tone.r, tone.g, tone.b, if hot { 0.3 } else { 0.13 }));
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.5, Color::new(tone.r, tone.g, tone.b, if hot { 1.0 } else { 0.65 }));
    let size = (rect.h * 0.4).round().max(13.0);
    let key_room = if key.is_empty() { 0.0 } else { text_width(key, size * 0.8, Face::Bold) + 34.0 };
    let label_size = fit_size(label, size, rect.w - key_room - 32.0, Face::Bold);
    let y = baseline(rect, label_size);
    draw_label(label, rect.x + 16.0, y, label_size, if enabled { INK } else { MUTED }, Face::Bold);
    if !key.is_empty() {
        draw_key(key, rect.right() - text_width(key, size * 0.8, Face::Bold) - 24.0, y, size * 0.8);
    }
}

//text broken at spaces into lines no wider than `width`
fn wrap(text: &str, size: f32, width: f32, face: Face) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if text_width(&format!("{line} {word}"), size, face) <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_owned()),
        }
    }
    lines
}

const LINE_SPACING: f32 = 1.4;

//wrapped text with the first baseline at y. returns how tall it came to
fn draw_wrapped(text: &str, x: f32, y: f32, size: f32, width: f32, color: Color, face: Face) -> f32 {
    let lines = wrap(text, size, width, face);
    for (i, line) in lines.iter().enumerate() {
        draw_label(line, x, y + size * LINE_SPACING * i as f32, size, color, face);
    }
    size * LINE_SPACING * lines.len() as f32
}

//the part of a field's text that fits in `room`, with the caret in view, and how far in from its start the caret is
fn visible_part(shown: &str, caret: usize, room: f32, size: f32) -> (String, f32) {
    let letters: Vec<char> = shown.chars().collect();
    let caret = caret.min(letters.len());
    let part = |from: usize, to: usize| letters[from..to].iter().collect::<String>();
    let mut from = 0;
    while from < caret && text_width(&part(from, caret), size, Face::Body) > room {
        from += 1;
    }
    let mut to = letters.len();
    while to > caret && text_width(&part(from, to), size, Face::Body) > room {
        to -= 1;
    }
    (part(from, to), text_width(&part(from, caret), size, Face::Body))
}

fn draw_field(rect: Rect, field: &TextField, secret: bool, focused: bool, size: f32, time: f32) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.0, 0.0, 0.0, 0.38));
    let edge = if focused { CYAN } else { Color::new(MUTED.r, MUTED.g, MUTED.b, 0.5) };
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, if focused { 2.0 } else { 1.0 }, edge);
    let (text, caret_x) = visible_part(&field.shown(secret), field.cursor(), rect.w - 24.0, size);
    let y = baseline(rect, size);
    draw_label(&text, rect.x + 12.0, y, size, INK, Face::Body);
    if focused && (time * 1.7) % 1.0 < 0.6 {
        let x = rect.x + 12.0 + caret_x;
        draw_line(x, rect.y + rect.h * 0.2, x, rect.y + rect.h * 0.8, 2.0, CYAN);
    }
}

//where everything on the form is, which the drawing and the mouse both go by
struct FormRects {
    rows: Vec<(Row, Rect)>, //each row's control, which takes the room right of its label
    roles: [Rect; 3],       //inside the first of them
    reveal: Option<Rect>,   //in the password field, if there is one
    go: Rect,
    back: Rect,
    note_y: f32,
    top: f32,
}

//the button in a password field that shows or hides what is typed in it
fn reveal_rect(password: Rect, size: f32) -> Rect {
    let width = text_width("SHOW", size * 0.7, Face::Bold) + 20.0;
    Rect::new(password.right() - width - 6.0, password.y + 6.0, width, password.h - 12.0)
}

fn draw_reveal(rect: Rect, revealed: bool, size: f32, mouse: Vec2) {
    let size = (size * 0.7).round();
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(1.0, 1.0, 1.0, if rect.contains(mouse) { 0.14 } else { 0.05 }));
    draw_label_centered(if revealed { "HIDE" } else { "SHOW" }, rect.x + rect.w / 2.0, baseline(rect, size), size, MUTED, Face::Bold);
}

fn form_rects(screen: Vec2, role: Role) -> FormRects {
    let frame = frame_of(screen);
    let top = (screen.y * 0.2).max(frame.row * 1.7);
    let button_height = frame.row * 1.1;
    let buttons_y = screen.y - button_height - (screen.y * 0.04).max(14.0);
    //(room is kept below the rows for what is said of them and for what is wrong, and the rows are as far apart as they can be in what is left)
    let reserve = frame.size * 0.85 * LINE_SPACING * 5.0;
    let rows = rows(role);
    let step = ((buttons_y - top - reserve - 10.0) / rows.len() as f32).min(frame.row * 1.3);
    let height = frame.row.min(step * 0.82);
    let field_left = frame.left + frame.width * 0.3;
    let field_width = frame.width * 0.7;
    let gap = 8.0;
    let third = (field_width - gap * 2.0) / 3.0;
    let rects: Vec<(Row, Rect)> = rows.iter().enumerate().map(|(i, &row)| (row, Rect::new(field_left, top + step * i as f32, field_width, height))).collect();
    let rect_of = |wanted: Row| rects.iter().find(|(row, _)| *row == wanted).map(|(_, rect)| *rect);
    let first = rect_of(Row::Role).expect("the form starts with the role");
    let reveal = rect_of(Row::Password).map(|password| reveal_rect(password, frame.size));
    let after = rects.last().map_or(top, |(_, rect)| rect.bottom());
    FormRects {
        roles: std::array::from_fn(|i| Rect::new(first.x + (third + gap) * i as f32, first.y, third, first.h)),
        rows: rects,
        reveal,
        go: Rect::new(frame.left, buttons_y, frame.width * 0.62, button_height),
        back: Rect::new(frame.left + frame.width * 0.66, buttons_y, frame.width * 0.34, button_height),
        note_y: after + frame.size * 1.1,
        top,
    }
}

fn draw_form(form: &Form, time: f32, mouse: Vec2) {
    let screen = Vec2::new(screen_width(), screen_height());
    let frame = frame_of(screen);
    let rects = form_rects(screen, form.role);
    let heading_size = fit_size("MULTIPLAYER", (screen.y * 0.085).round(), screen.x * 0.9, Face::Header);
    draw_glowing_heading("MULTIPLAYER", screen.x / 2.0, rects.top - frame.row * 1.05, heading_size, WHITE, 1.0);
    for &(row, rect) in &rects.rows {
        let focused = form.focus == row;
        let name = match row {
            Row::Role => "GAME",
            Row::Name => "NAME",
            Row::Password => "PASSWORD",
            Row::Port => "PORT",
        };
        let room = rect.x - frame.left - 12.0;
        let label_size = fit_size(name, frame.size, room, Face::Bold);
        draw_label(name, frame.left, baseline(rect, label_size), label_size, if focused { CYAN } else { MUTED }, Face::Bold);
        match row {
            //host, join or join local
            Row::Role => {
                for (role_rect, (role, name)) in rects.roles.iter().zip([(Role::Host, "HOST A GAME"), (Role::Join, "JOIN A GAME"), (Role::Local, "JOIN LOCAL")]) {
                    let chosen = form.role == role;
                    let tone = if chosen { CYAN } else { MUTED };
                    draw_rectangle(role_rect.x, role_rect.y, role_rect.w, role_rect.h, Color::new(tone.r, tone.g, tone.b, if chosen { 0.22 } else { 0.05 }));
                    draw_rectangle_lines(role_rect.x, role_rect.y, role_rect.w, role_rect.h, if chosen && focused { 2.0 } else { 1.0 }, Color::new(tone.r, tone.g, tone.b, if chosen { 1.0 } else { 0.4 }));
                    let size = fit_size(name, frame.size, role_rect.w - 16.0, Face::Bold);
                    draw_label_centered(name, role_rect.x + role_rect.w / 2.0, baseline(*role_rect, size), size, if chosen { INK } else { MUTED }, Face::Bold);
                }
            }
            _ => {
                let field = match row {
                    Row::Name => &form.name,
                    Row::Password => &form.password,
                    _ => &form.port,
                };
                draw_field(rect, field, row == Row::Password && !form.reveal, focused, frame.size, time);
            }
        }
    }
    if let Some(reveal) = rects.reveal {
        draw_reveal(reveal, form.reveal, frame.size, mouse);
    }
    //what the player is doing
    let note = match form.role {
        Role::Host => "your game goes on the lobby list, where players find it and join it with the password you choose here. for those outside your home the port (UDP) must be forwarded on your router to this computer",
        Role::Join => "see the games that are open on the lobby list, pick one, and give the password its host chose",
        Role::Local => "joins a game that is hosted on this computer, in another window, by its port (UDP): for trying the game out, or two players at one computer",
    };
    let note_size = (frame.size * 0.78).round();
    let note_height = draw_wrapped(note, frame.left, rects.note_y, note_size, frame.width, MUTED, Face::Body);
    draw_button(rects.go, match form.role { Role::Host => "START HOSTING", Role::Join => "FIND GAMES", Role::Local => "JOIN THE GAME" }, "Enter", mouse, CYAN, true);
    draw_button(rects.back, "BACK", "Esc", mouse, MUTED, true);
    if let Some(problem) = &form.problem {
        draw_wrapped(problem, frame.left, rects.note_y + note_height + note_size * 0.6, (frame.size * 0.85).round(), frame.width, ALERT, Face::Bold);
    }
}

//who is in the game: a row for each place, with the colour they will be, their name and how far away they are. returns the place of the player whose
//kick button was clicked, if `kickable` and one was
fn draw_roster(frame: &Frame, top: f32, members: &[Member], mine: u8, kickable: bool, mouse: Vec2, click: bool) -> (f32, Option<u8>) {
    let size = frame.size;
    let height = frame.row * (MAX_PLAYERS as f32 + 0.9);
    panel(frame.left, top, frame.width, height);
    draw_label("PLAYERS", frame.left + 18.0, top + frame.row * 0.7, (size * 0.8).round(), CYAN, Face::Bold);
    let mut kicked = None;
    for slot in 0..MAX_PLAYERS {
        let rect = Rect::new(frame.left, top + frame.row * (slot as f32 + 0.9), frame.width, frame.row);
        let y = baseline(rect, size);
        let color = PLAYER_COLORS[slot];
        match members.iter().find(|member| member.slot as usize == slot) {
            Some(member) => {
                draw_circle(rect.x + 30.0, rect.y + rect.h / 2.0, size * 0.32, color);
                let you = member.slot == mine;
                draw_label(&member.name, rect.x + 52.0, y, size, INK, Face::Bold);
                if you {
                    draw_label("you", rect.x + 62.0 + text_width(&member.name, size, Face::Bold), y, size * 0.8, MUTED, Face::Body);
                }
                let mut right = rect.right() - 18.0;
                if kickable && slot != 0 {
                    let kick = Rect::new(right - 78.0, rect.y + rect.h * 0.16, 78.0, rect.h * 0.68);
                    let hot = kick.contains(mouse);
                    draw_rectangle(kick.x, kick.y, kick.w, kick.h, Color::new(ALERT.r, ALERT.g, ALERT.b, if hot { 0.3 } else { 0.1 }));
                    draw_rectangle_lines(kick.x, kick.y, kick.w, kick.h, 1.0, ALERT);
                    draw_label_centered("KICK", kick.x + kick.w / 2.0, baseline(kick, size * 0.7), size * 0.7, INK, Face::Bold);
                    if hot && click {
                        kicked = Some(member.slot);
                    }
                    right = kick.x - 14.0;
                }
                let ping = if member.slot == 0 { "host".to_owned() } else { format!("{} ms", member.ping_ms) };
                draw_label_right(&ping, right, y, size * 0.8, MUTED, Face::Body);
            }
            None => {
                draw_circle_lines(rect.x + 30.0, rect.y + rect.h / 2.0, size * 0.32, 1.5, Color::new(color.r, color.g, color.b, 0.4));
                draw_label("waiting for a player", rect.x + 52.0, y, size * 0.9, Color::new(MUTED.r, MUTED.g, MUTED.b, 0.55), Face::Body);
            }
        }
        if slot + 1 < MAX_PLAYERS {
            draw_line(frame.left + 14.0, rect.bottom(), frame.left + frame.width - 14.0, rect.bottom(), 1.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.1));
        }
    }
    (height, kicked)
}

//what has happened, as many of the latest as there is room for between `top` and `bottom`
fn draw_log(frame: &Frame, top: f32, bottom: f32, log: &[(String, Color)]) {
    let size = (frame.size * 0.78).round();
    let room = ((bottom - top) / (size * LINE_SPACING)).floor().max(0.0) as usize;
    let shown = &log[log.len().saturating_sub(room)..];
    for (i, (line, color)) in shown.iter().enumerate() {
        let fitted = fit_size(line, size, frame.width, Face::Body);
        draw_label(line, frame.left, top + size * LINE_SPACING * (i as f32 + 0.8), fitted, *color, Face::Body);
    }
}

// ---------- the screens ----------

enum FormOutcome {
    Stay,
    Back,
    Go,
}

fn form_frame(form: &mut Form, typing: &mut Typing, time: f32, dt: f32, mouse: Vec2, click: bool) -> FormOutcome {
    let screen = Vec2::new(screen_width(), screen_height());
    let rects = form_rects(screen, form.role);
    let edits = typing.edits(dt);
    if is_key_pressed(KeyCode::Escape) || (click && rects.back.contains(mouse)) {
        return FormOutcome::Back;
    }
    //(the mouse picks a row, or a button)
    if click {
        for &(row, rect) in &rects.rows {
            match row {
                Row::Role => {
                    for (role_rect, role) in rects.roles.iter().zip(ROLES) {
                        if role_rect.contains(mouse) {
                            form.switch(role);
                            form.focus = Row::Role;
                        }
                    }
                }
                _ if rect.contains(mouse) => form.focus = row,
                _ => {}
            }
        }
        if rects.reveal.is_some_and(|reveal| reveal.contains(mouse)) {
            form.reveal = !form.reveal;
        }
        if rects.go.contains(mouse) {
            return FormOutcome::Go;
        }
    }
    let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    if is_key_pressed(KeyCode::Tab) {
        form.step(if shift { -1 } else { 1 });
    }
    if is_key_pressed(KeyCode::Down) {
        form.step(1);
    }
    if is_key_pressed(KeyCode::Up) {
        form.step(-1);
    }
    let focus = form.focus;
    if focus == Row::Role {
        if is_key_pressed(KeyCode::Left) {
            form.switch(form.role.before());
        }
        if is_key_pressed(KeyCode::Right) {
            form.switch(form.role.after());
        }
        if is_key_pressed(KeyCode::Space) {
            form.switch(form.role.next());
        }
    } else if let Some(field) = form.field(focus) {
        for edit in edits {
            field.apply(edit);
        }
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
        if form.enter() {
            return FormOutcome::Go;
        }
    }
    draw_form(form, time, mouse);
    FormOutcome::Stay
}

// ---------- the lobby list ----------

const BROWSE_REFRESH_SECONDS: f32 = 8.0; //how often the list is read again while it is on the screen

//the games on a lobby server, for a player to pick from
struct BrowsePage {
    directory: Directory,
    name: String, //what the player who is looking is called, for when they join one
    games: Option<Vec<ListedGame>>, //None until the first answer
    problem: Option<String>,
    selected: usize,
    scroll: usize, //the first game shown
    asked_at: f32, //when the list was last asked for
}

impl BrowsePage {
    fn new(directory: Directory, name: String, time: f32) -> BrowsePage {
        directory.refresh();
        BrowsePage { directory, name, games: None, problem: None, selected: 0, scroll: 0, asked_at: time }
    }

    fn ask(&mut self, time: f32) {
        self.directory.refresh();
        self.asked_at = time;
    }

    fn absorb(&mut self) {
        for event in self.directory.poll() {
            match event {
                DirectoryEvent::Games(games) => {
                    //(the same game stays picked when the list is read again, if it is still there)
                    let before = self.games.as_ref().and_then(|games| games.get(self.selected)).map(|game| (game.address, game.port));
                    self.selected = before.and_then(|at| games.iter().position(|game| (game.address, game.port) == at)).unwrap_or(0);
                    self.games = Some(games);
                    self.problem = None;
                }
                DirectoryEvent::Unreachable(problem) => self.problem = Some(problem),
                DirectoryEvent::Listed(_) | DirectoryEvent::Unlisted(_) => {}
            }
        }
    }
}

enum BrowseOutcome {
    Stay,
    Back,
    Pick(ListedGame),
}

//why a game on the list cannot be joined, if it can't
fn why_not(game: &ListedGame) -> Option<String> {
    if !game.same_version() {
        return Some(format!("{}'s game is version {}, and this is {}", game.name, game.version, env!("CARGO_PKG_VERSION")));
    }
    game.is_full().then(|| format!("{}'s game is full", game.name))
}

fn browse_frame(page: &mut BrowsePage, time: f32, mouse: Vec2, click: bool) -> BrowseOutcome {
    page.absorb();
    if time - page.asked_at >= BROWSE_REFRESH_SECONDS {
        page.ask(time);
    }
    let screen = Vec2::new(screen_width(), screen_height());
    let frame = frame_of(screen);
    let heading_size = fit_size("GAMES", (screen.y * 0.085).round(), screen.x * 0.9, Face::Header);
    let top = (screen.y * 0.2).max(frame.row * 1.8);
    draw_glowing_heading("GAMES", screen.x / 2.0, top - frame.row * 0.4, heading_size, WHITE, 1.0);
    let small = (frame.size * 0.72).round();
    //the buttons along the bottom
    let button_height = frame.row * 1.1;
    let y = screen.y - button_height - (screen.y * 0.04).max(14.0);
    let gap = 10.0;
    let half = (frame.width - gap) / 2.0;
    let refresh = Rect::new(frame.left, y, half, button_height);
    let back = Rect::new(frame.left + half + gap, y, half, button_height);
    draw_button(refresh, "REFRESH", "F5", mouse, CYAN, true);
    draw_button(back, "BACK", "Esc", mouse, MUTED, true);
    //the games
    let list_top = top + frame.row * 0.6;
    let list_bottom = y - frame.row * 0.9;
    panel(frame.left, list_top, frame.width, list_bottom - list_top);
    let row_height = frame.row * 0.95;
    let room = (((list_bottom - list_top) - frame.row * 0.3) / row_height).floor().max(1.0) as usize;
    let count = page.games.as_ref().map_or(0, Vec::len);
    let (wheel, keys) = (mouse_wheel().1, (is_key_pressed(KeyCode::Down) as i32) - (is_key_pressed(KeyCode::Up) as i32));
    if count > 0 {
        let moved = keys + if is_key_pressed(KeyCode::PageDown) { room as i32 } else { 0 } - if is_key_pressed(KeyCode::PageUp) { room as i32 } else { 0 };
        page.selected = (page.selected as i32 + moved).clamp(0, count as i32 - 1) as usize;
        if wheel != 0.0 {
            page.scroll = (page.scroll as i32 - wheel.signum() as i32 * 2).clamp(0, count.saturating_sub(room) as i32) as usize;
        }
        //(the one picked is kept in view)
        if moved != 0 {
            page.scroll = page.scroll.clamp((page.selected + 1).saturating_sub(room), page.selected);
        }
        page.scroll = page.scroll.min(count.saturating_sub(room));
    }
    let mut pick = None;
    match &page.games {
        None if page.problem.is_none() => {
            let dots = ".".repeat(1 + (time * 2.0) as usize % 3);
            let line = format!("LOOKING FOR GAMES{dots}");
            draw_label_centered(&line, screen.x / 2.0, list_top + frame.row * 1.4, fit_size(&line, frame.size, frame.width, Face::Bold), Color::new(CYAN.r, CYAN.g, CYAN.b, 0.6 + 0.4 * (time * 3.0).sin()), Face::Bold);
        }
        Some(games) if games.is_empty() && page.problem.is_none() => {
            let line = "NOBODY IS HOSTING RIGHT NOW";
            draw_label_centered(line, screen.x / 2.0, list_top + frame.row * 1.4, fit_size(line, frame.size, frame.width, Face::Bold), MUTED, Face::Bold);
        }
        Some(games) => {
            for (i, game) in games.iter().enumerate().skip(page.scroll).take(room) {
                let rect = Rect::new(frame.left + 6.0, list_top + frame.row * 0.15 + row_height * (i - page.scroll) as f32, frame.width - 12.0, row_height);
                let joinable = why_not(game).is_none();
                let hot = rect.contains(mouse);
                if hot && click {
                    page.selected = i;
                    pick = Some(i);
                }
                if i == page.selected || hot {
                    draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(CYAN.r, CYAN.g, CYAN.b, if i == page.selected { 0.16 } else { 0.07 }));
                }
                let text_y = baseline(rect, frame.size);
                draw_circle(rect.x + 20.0, rect.y + rect.h / 2.0, frame.size * 0.28, if joinable { CYAN } else { MUTED });
                draw_label(&game.name, rect.x + 42.0, text_y, fit_size(&game.name, frame.size, rect.w * 0.34, Face::Bold), if joinable { INK } else { MUTED }, Face::Bold);
                //(from the right: how many are in it, then what is to know of it)
                let count_text = format!("{}/{}", game.players, game.max);
                let mut right = rect.right() - 16.0;
                draw_label_right(&count_text, right, text_y, frame.size * 0.9, if game.is_full() { ALERT } else { INK }, Face::Bold);
                right -= text_width(&count_text, frame.size * 0.9, Face::Bold) + 22.0;
                let (note, tone) = if !game.same_version() { (format!("v{}", game.version), ALERT) } else if game.is_full() { ("FULL".to_owned(), ALERT) } else if game.started { ("IN PROGRESS".to_owned(), TROPHY) } else { ("WAITING".to_owned(), pickup_color()) };
                draw_label_right(&note, right, text_y, small, tone, Face::Bold);
            }
            if count > room {
                let bar_h = (list_bottom - list_top - 16.0) * room as f32 / count as f32;
                let bar_y = list_top + 8.0 + (list_bottom - list_top - 16.0 - bar_h) * page.scroll as f32 / (count - room) as f32;
                draw_rectangle(frame.left + frame.width - 7.0, bar_y, 3.0, bar_h, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.5));
            }
        }
        None => {}
    }
    if let Some(problem) = &page.problem {
        let at = if count == 0 { list_top + frame.row * 1.2 } else { list_bottom + frame.row * 0.2 };
        draw_wrapped(problem, frame.left + 18.0, at, small, frame.width - 36.0, ALERT, Face::Bold);
    }
    if is_key_pressed(KeyCode::Escape) || (click && back.contains(mouse)) {
        return BrowseOutcome::Back;
    }
    if is_key_pressed(KeyCode::F5) || is_key_pressed(KeyCode::R) || (click && refresh.contains(mouse)) {
        page.ask(time);
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
        pick = Some(page.selected);
    }
    if let Some(game) = pick.and_then(|i| page.games.as_ref()?.get(i)) {
        match why_not(game) {
            Some(why) => page.problem = Some(why),
            None => return BrowseOutcome::Pick(game.clone()),
        }
    }
    BrowseOutcome::Stay
}

//a game that has been picked from the list, and what is asked for to get into it: its password
struct PasswordPage {
    browse: Box<BrowsePage>, //the list it was picked from, which Back goes back to
    game: ListedGame,
    password: TextField,
    reveal: bool,
    problem: Option<String>,
}

impl PasswordPage {
    fn new(browse: Box<BrowsePage>, game: ListedGame) -> PasswordPage {
        PasswordPage { browse, game, password: TextField::new("", MAX_PASSWORD, Allowed::Password), reveal: false, problem: None }
    }
}

enum PasswordOutcome {
    Stay,
    Back,
    Join, //with the password as it was typed
}

fn password_frame(page: &mut PasswordPage, typing: &mut Typing, time: f32, dt: f32, mouse: Vec2, click: bool) -> PasswordOutcome {
    let screen = Vec2::new(screen_width(), screen_height());
    let frame = frame_of(screen);
    let edits = typing.edits(dt);
    let top = (screen.y * 0.2).max(frame.row * 1.7);
    let heading_size = fit_size("JOIN A GAME", (screen.y * 0.085).round(), screen.x * 0.9, Face::Header);
    draw_glowing_heading("JOIN A GAME", screen.x / 2.0, top - frame.row * 1.05, heading_size, WHITE, 1.0);
    //whose game it is
    let game = &page.game;
    let title = format!("{}'S GAME", game.name.to_uppercase());
    draw_label_centered(&title, screen.x / 2.0, top + frame.row * 0.2, fit_size(&title, (frame.size * 1.25).round(), frame.width, Face::Bold), CYAN, Face::Bold);
    let state = format!("{} of {} players, {}", game.players, game.max, if game.started { "the run is under way" } else { "waiting to start" });
    let small = (frame.size * 0.78).round();
    draw_label_centered(&state, screen.x / 2.0, top + frame.row * 0.2 + small * 1.7, fit_size(&state, small, frame.width, Face::Body), MUTED, Face::Body);
    //the password
    let field = Rect::new(frame.left + frame.width * 0.3, top + frame.row * 1.6, frame.width * 0.7, frame.row);
    let label_size = fit_size("PASSWORD", frame.size, field.x - frame.left - 12.0, Face::Bold);
    draw_label("PASSWORD", frame.left, baseline(field, label_size), label_size, CYAN, Face::Bold);
    draw_field(field, &page.password, !page.reveal, true, frame.size, time);
    let reveal = reveal_rect(field, frame.size);
    draw_reveal(reveal, page.reveal, frame.size, mouse);
    let note = format!("the password {} chose for the game", game.name);
    let below = field.bottom() + small * 1.9;
    let note_height = draw_wrapped(&note, frame.left, below, small, frame.width, MUTED, Face::Body);
    if let Some(problem) = &page.problem {
        draw_wrapped(problem, frame.left, below + note_height + small * 0.6, (frame.size * 0.85).round(), frame.width, ALERT, Face::Bold);
    }
    //the buttons along the bottom
    let button_height = frame.row * 1.1;
    let y = screen.y - button_height - (screen.y * 0.04).max(14.0);
    let join = Rect::new(frame.left, y, frame.width * 0.62, button_height);
    let back = Rect::new(frame.left + frame.width * 0.66, y, frame.width * 0.34, button_height);
    draw_button(join, "JOIN THE GAME", "Enter", mouse, CYAN, true);
    draw_button(back, "BACK", "Esc", mouse, MUTED, true);
    if is_key_pressed(KeyCode::Escape) || (click && back.contains(mouse)) {
        return PasswordOutcome::Back;
    }
    if click && reveal.contains(mouse) {
        page.reveal = !page.reveal;
    }
    for edit in edits {
        page.password.apply(edit);
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || (click && join.contains(mouse)) {
        page.problem = password_problem(page.password.text());
        if page.problem.is_none() {
            return PasswordOutcome::Join;
        }
    }
    PasswordOutcome::Stay
}

enum HostOutcome {
    Stay,
    Back,
    Start { saved_run: bool },
}

fn host_frame(page: &mut HostPage, saved: Option<&SaveInfo>, mouse: Vec2, click: bool) -> HostOutcome {
    page.absorb();
    let screen = Vec2::new(screen_width(), screen_height());
    let frame = frame_of(screen);
    let heading_size = fit_size("HOSTING", (screen.y * 0.085).round(), screen.x * 0.9, Face::Header);
    let top = (screen.y * 0.2).max(frame.row * 1.8);
    draw_glowing_heading("HOSTING", screen.x / 2.0, top - frame.row * 0.4, heading_size, WHITE, 1.0);
    //where the others go: the lobby list, and what it takes for those outside the home to get to the game
    let port = page.host.port;
    let size = frame.size * 0.85;
    let note = format!("players find your game on the lobby list and join it with its password. from outside your home, port {port} (UDP) must be forwarded on your router to this computer. a second window here joins with JOIN LOCAL, on port {port}");
    let note_size = (frame.size * 0.72).round();
    let note_lines = wrap(&note, note_size, frame.width - 36.0, Face::Body).len() as f32;
    //(what is wrong with the listing is said on lines of its own, over the rest)
    let (status, tone, trouble) = match &page.listing {
        None => ("asking the lobby server...".to_owned(), MUTED, None),
        Some(Ok(())) => ("on the lobby list".to_owned(), pickup_color(), None),
        Some(Err(problem)) => ("not on the lobby list".to_owned(), ALERT, Some(problem.as_str())),
    };
    let trouble_height = trouble.map_or(0.0, |problem| note_size * LINE_SPACING * wrap(problem, note_size, frame.width - 36.0, Face::Body).len() as f32 + frame.row * 0.15);
    let address_panel = frame.row * 1.4 + trouble_height + note_size * LINE_SPACING * note_lines + frame.row * 0.1;
    panel(frame.left, top, frame.width, address_panel);
    draw_label("LOBBY LIST", frame.left + 18.0, top + frame.row * 0.8, size, MUTED, Face::Bold);
    let room = frame.width - 36.0 - text_width("LOBBY LIST", size, Face::Bold) - 16.0;
    draw_label_right(&status, frame.left + frame.width - 18.0, top + frame.row * 0.8, fit_size(&status, size, room, Face::Body), tone, Face::Body);
    if let Some(problem) = trouble {
        draw_wrapped(problem, frame.left + 18.0, top + frame.row * 1.4, note_size, frame.width - 36.0, ALERT, Face::Body);
    }
    draw_wrapped(&note, frame.left + 18.0, top + frame.row * 1.4 + trouble_height, note_size, frame.width - 36.0, MUTED, Face::Body);
    //who is in
    let members = page.host.members();
    let roster_top = top + address_panel + frame.row * 0.35;
    let (roster_height, kicked) = draw_roster(&frame, roster_top, &members, 0, true, mouse, click);
    if let Some(slot) = kicked {
        page.host.kick(slot);
    }
    //the buttons along the bottom
    let button_height = frame.row * 1.1;
    let y = screen.y - button_height - (screen.y * 0.04).max(14.0);
    draw_log(&frame, roster_top + roster_height, y - 8.0, &page.log);
    let gap = 10.0;
    let third = (frame.width - gap * 2.0) / 3.0;
    let start = Rect::new(frame.left, y, third, button_height);
    let carry_on = Rect::new(frame.left + third + gap, y, third, button_height);
    let back = Rect::new(frame.left + (third + gap) * 2.0, y, third, button_height);
    draw_button(start, "NEW RUN", "Enter", mouse, CYAN, true);
    //(a game that was saved with others in it is carried on with them: they join it again by their names)
    draw_button(carry_on, if saved.is_some_and(|info| info.party > 1) { "CONTINUE PARTY" } else { "CONTINUE" }, "C", mouse, TROPHY, saved.is_some());
    draw_button(back, "BACK", "Esc", mouse, MUTED, true);
    if is_key_pressed(KeyCode::Escape) || (click && back.contains(mouse)) {
        return HostOutcome::Back;
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || is_key_pressed(KeyCode::N) || (click && start.contains(mouse)) {
        return HostOutcome::Start { saved_run: false };
    }
    if saved.is_some() && (is_key_pressed(KeyCode::C) || (click && carry_on.contains(mouse))) {
        return HostOutcome::Start { saved_run: true };
    }
    HostOutcome::Stay
}

enum JoinOutcome {
    Stay,
    Back,
    Go,           //the host has started the run
    Over(String), //it did not work out, or it ended, and why
}

fn join_frame(page: &mut JoinPage, time: f32, mouse: Vec2, click: bool) -> JoinOutcome {
    for event in page.client.poll() {
        if let Some(why) = page.lobby.apply(event) {
            return JoinOutcome::Over(why);
        }
    }
    let screen = Vec2::new(screen_width(), screen_height());
    let frame = frame_of(screen);
    let lobby = &page.lobby;
    let title = if lobby.joined { "IN THE GAME" } else { "JOINING" };
    let heading_size = fit_size(title, (screen.y * 0.085).round(), screen.x * 0.9, Face::Header);
    let top = (screen.y * 0.2).max(frame.row * 1.8);
    draw_glowing_heading(title, screen.x / 2.0, top - frame.row * 0.4, heading_size, WHITE, 1.0);
    let pulse = Color::new(CYAN.r, CYAN.g, CYAN.b, 0.6 + 0.4 * (time * 3.0).sin());
    let size = frame.size;
    if !lobby.joined {
        let dots = ".".repeat(1 + (time * 2.0) as usize % 3);
        let line = format!("CONNECTING TO {}{dots}", page.to.to_uppercase());
        draw_label_centered(&line, screen.x / 2.0, top + frame.row * 1.2, fit_size(&line, size, frame.width, Face::Bold), pulse, Face::Bold);
    } else {
        let (_, _) = draw_roster(&frame, top, &lobby.members, lobby.slot, false, mouse, false);
        let below = top + frame.row * (MAX_PLAYERS as f32 + 0.9) + frame.row * 0.9;
        if lobby.started {
            draw_label_centered("THE HOST HAS STARTED THE GAME", screen.x / 2.0, below, fit_size("THE HOST HAS STARTED THE GAME", size, frame.width, Face::Bold), TROPHY, Face::Bold);
        } else {
            draw_label_centered("WAITING FOR THE HOST TO START", screen.x / 2.0, below, fit_size("WAITING FOR THE HOST TO START", size, frame.width, Face::Bold), pulse, Face::Bold);
        }
    }
    let button_height = frame.row * 1.1;
    let back = Rect::new(frame.left, screen.y - button_height - (screen.y * 0.04).max(14.0), frame.width * 0.34, button_height);
    draw_button(back, "LEAVE", "Esc", mouse, MUTED, true);
    if is_key_pressed(KeyCode::Escape) || (click && back.contains(mouse)) {
        return JoinOutcome::Back;
    }
    if page.lobby.started {
        return JoinOutcome::Go;
    }
    JoinOutcome::Stay
}

//where the multiplayer screens lead
pub(crate) enum Multi {
    Host(NetHost, Option<Slot>), //a game the player is hosting, and the save it is to carry on from, if it is to
    Guest(NetClient, Lobby), //the game of another that the player joined, now it has started
}

//the multiplayer screens, from the form to the lobby. returns the game the player is hosting when they start it, or the game they joined when its host
//starts it; None when they back out. a player who joins another's game stays in its lobby until it starts or they leave. what a host can carry on from is
//the game they last had with others in it, if there is one, or else the run they saved last
pub(crate) async fn show_multiplayer(settings: &mut Settings, saves: &Saves) -> Option<Multi> {
    let carry = saves.info(Slot::Party).map(|info| (Slot::Party, info)).or_else(|| saves.newest());
    let mut form = Form::new(settings);
    let mut typing = Typing::new();
    let mut page = Screen::Form;
    let mut time = 0.0;
    let outcome = loop {
        quit_if_asked();
        let dt = get_frame_time();
        let mouse = Vec2::from(mouse_position());
        let click = is_mouse_button_pressed(MouseButton::Left);
        draw_backdrop(time);
        match &mut page {
            Screen::Form => match form_frame(&mut form, &mut typing, time, dt, mouse, click) {
                FormOutcome::Stay => {}
                FormOutcome::Back => break None,
                FormOutcome::Go => match form.submit() {
                    Err(problem) => form.problem = Some(problem),
                    Ok(Submit::Host { config, list_on }) => match NetHost::start(config) {
                        Ok(mut host) => match host.list_on(&list_on) {
                            Ok(()) => {
                                form.remember(settings);
                                saves.save_settings(settings);
                                page = Screen::Host(Box::new(HostPage::new(host)));
                            }
                            Err(problem) => form.problem = Some(problem),
                        },
                        Err(problem) => form.problem = Some(problem),
                    },
                    Ok(Submit::Join { name }) => match Directory::connect(&form.server) {
                        Ok(directory) => {
                            form.remember(settings);
                            saves.save_settings(settings);
                            form.problem = None;
                            page = Screen::Browse(Box::new(BrowsePage::new(directory, name, time)));
                        }
                        Err(problem) => form.problem = Some(problem),
                    },
                    Ok(Submit::Local { target, name, password }) => {
                        form.remember(settings);
                        saves.save_settings(settings);
                        form.problem = None;
                        let client = NetClient::connect(&target, &name, &password);
                        page = Screen::Join(Box::new(JoinPage { client, to: "this computer".to_owned(), asked: None, lobby: Lobby::default() }));
                    }
                },
            },
            Screen::Browse(browse) => match browse_frame(browse, time, mouse, click) {
                BrowseOutcome::Stay => {}
                BrowseOutcome::Back => page = Screen::Form,
                BrowseOutcome::Pick(game) => {
                    let Screen::Browse(browse) = std::mem::replace(&mut page, Screen::Form) else { unreachable!("it was the list") };
                    page = Screen::Password(Box::new(PasswordPage::new(browse, game)));
                }
            },
            Screen::Password(ask) => match password_frame(ask, &mut typing, time, dt, mouse, click) {
                PasswordOutcome::Stay => {}
                PasswordOutcome::Back => {
                    let Screen::Password(ask) = std::mem::replace(&mut page, Screen::Form) else { unreachable!("it was the password page") };
                    page = Screen::Browse(ask.browse);
                }
                PasswordOutcome::Join => {
                    let Screen::Password(ask) = std::mem::replace(&mut page, Screen::Form) else { unreachable!("it was the password page") };
                    let client = NetClient::connect(&ask.game.target(), &ask.browse.name, ask.password.text());
                    page = Screen::Join(Box::new(JoinPage { client, to: ask.game.name.clone(), asked: Some(ask), lobby: Lobby::default() }));
                }
            },
            Screen::Host(host) => match host_frame(host, carry.as_ref().map(|(_, info)| info), mouse, click) {
                HostOutcome::Stay => {}
                HostOutcome::Back => page = Screen::Form,
                HostOutcome::Start { saved_run } => {
                    let Screen::Host(host) = std::mem::replace(&mut page, Screen::Form) else { unreachable!("it was a host page") };
                    host.host.start_run();
                    break Some(Multi::Host(host.host, carry.as_ref().filter(|_| saved_run).map(|(slot, _)| *slot)));
                }
            },
            Screen::Join(join) => match join_frame(join, time, mouse, click) {
                JoinOutcome::Stay => {}
                //(a player who picked the game from the list goes back to its password, to try again or to leave from there)
                JoinOutcome::Back => {
                    let Screen::Join(join) = std::mem::replace(&mut page, Screen::Form) else { unreachable!("it was a join page") };
                    if let Some(ask) = join.asked {
                        page = Screen::Password(ask);
                    }
                }
                JoinOutcome::Go => {
                    let Screen::Join(join) = std::mem::replace(&mut page, Screen::Form) else { unreachable!("it was a join page") };
                    let JoinPage { client, lobby, .. } = *join;
                    break Some(Multi::Guest(client, lobby));
                }
                JoinOutcome::Over(why) => {
                    let Screen::Join(join) = std::mem::replace(&mut page, Screen::Form) else { unreachable!("it was a join page") };
                    match join.asked {
                        //(it did not work out, which may be the password: it can be typed again)
                        Some(mut ask) if !join.lobby.joined => {
                            ask.problem = Some(why);
                            page = Screen::Password(ask);
                        }
                        _ => form.problem = Some(why),
                    }
                }
            },
        }
        time += dt;
        next_frame().await;
    };
    //(the key that left is still down for the rest of this frame, and would be taken by whatever is shown next)
    next_frame().await;
    outcome
}

//how long a player who has lost the game they were in is tried to be got back into it
const REJOIN_SECONDS: f32 = 60.0;
//how long between one try and the next
const REJOIN_EVERY: f32 = 1.5;

//the screen while a player is put back in the game they were in, as the same player, which can take a few tries (the host may not know yet that the old connection has
//gone, and the network may be gone). it ends with the new connection when the host has started the game for them, or with why it can't be done (Esc gives up)
pub(crate) async fn reconnect_to(reconnect: &Reconnect, why: &str) -> Result<(NetClient, Lobby), String> {
    let mut time = 0.0;
    let mut tries = 0;
    let mut client: Option<NetClient> = None;
    let mut lobby = Lobby::default();
    let mut next_try = 0.0;
    let mut last_problem = why.to_owned();
    let outcome = loop {
        quit_if_asked();
        let dt = get_frame_time();
        if client.is_none() && time >= next_try {
            tries += 1;
            client = Some(NetClient::connect(&reconnect.target, &reconnect.name, &reconnect.password));
            lobby = Lobby::default();
        }
        let mut gone = false;
        if let Some(connection) = client.as_mut() {
            for event in connection.poll() {
                match event {
                    //(still on the books, as far as the host knows: it will let go of the old one in a few seconds)
                    ClientEvent::Refused(Refusal::NameTaken) => {
                        last_problem = "the host has not noticed the connection is gone yet".to_owned();
                        gone = true;
                    }
                    ClientEvent::Refused(refusal) => return Err(refusal.text()),
                    ClientEvent::Failed(problem) => {
                        last_problem = problem;
                        gone = true;
                    }
                    ClientEvent::Closed(problem) => {
                        last_problem = problem;
                        gone = true;
                    }
                    other => {
                        lobby.apply(other);
                    }
                }
            }
        }
        if gone {
            client = None;
            next_try = time + REJOIN_EVERY;
        } else if lobby.started {
            if let Some(client) = client.take() {
                break Ok((client, lobby));
            }
        }
        if is_key_pressed(KeyCode::Escape) {
            break Err(why.to_owned());
        }
        if time > REJOIN_SECONDS {
            break Err(format!("could not get back into the game: {last_problem}"));
        }
        draw_backdrop(time);
        let (w, h) = (screen_width(), screen_height());
        let heading = "CONNECTION LOST";
        draw_glowing_heading(heading, w / 2.0, h * 0.34, fit_size(heading, (h * 0.085).round(), w * 0.9, Face::Header), WHITE, 1.0);
        let small = (h * 0.03).round().max(14.0);
        draw_label_centered(&why.to_uppercase(), w / 2.0, h * 0.34 + small * 2.4, fit_size(&why.to_uppercase(), small, w * 0.9, Face::Body), MUTED, Face::Body);
        let line = format!("TRYING TO GET BACK IN{}  (TRY {tries})", ".".repeat(1 + (time * 2.0) as usize % 3));
        draw_label_centered(&line, w / 2.0, h * 0.5, fit_size(&line, (h * 0.04).round(), w * 0.9, Face::Bold), CYAN, Face::Bold);
        draw_label_centered("your army is held for you", w / 2.0, h * 0.5 + small * 2.0, small, MUTED, Face::Body);
        draw_label_centered("Esc to give up", w / 2.0, h * 0.5 + small * 3.4, small, MUTED, Face::Body);
        time += dt;
        next_frame().await;
    };
    //(the key that left is still down for the rest of this frame, and would be taken by whatever is shown next)
    next_frame().await;
    outcome
}

//what a hosted game says of itself while it is being played: where it is listening and who is in it
pub(crate) fn hosting_text(port: u16, members: &[Member]) -> String {
    let names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
    format!("HOSTING ON PORT {port}: {}", names.join(", "))
}

pub(crate) fn hosting_line(host: &NetHost) -> String {
    hosting_text(host.port, &host.members())
}

//what a joined game says of itself while it is being played: who is in it
pub(crate) fn joined_line(members: &[Member]) -> String {
    let names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
    format!("IN A GAME WITH: {}", names.join(", "))
}
