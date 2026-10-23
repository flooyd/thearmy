//! the lobby list: a small web service (lobby-server/ in the repository) where a host says that they are open and players look for a game to join, so that
//! nobody has to type an address. the game itself never goes through it, a player still connects to the host directly and the service only says where.
//! the service takes the host's address from the request it is told in, so what is listed is always where the host really is.
//! all of it is done on a thread of its own, so a slow or missing service never costs a frame

use crate::prelude::*;
use serde_json::{Value, json};
use std::net::IpAddr;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};

pub(crate) const MAX_SERVER_LENGTH: usize = 120;
//where the list of games is kept unless the player says otherwise: empty, until there is a service to name here (see lobby-server/README.md)
pub(crate) const DEFAULT_LOBBY_SERVER: &str = "";
const REQUEST_SECONDS: u64 = 6; //how long the service has to answer
const MAX_ANSWER_BYTES: u64 = 256 * 1024;
const MAX_LISTED: usize = 100; //how many games of the ones it names are looked at
const MAX_VERSION_LENGTH: usize = 24;
const MIN_PAUSE: Duration = if cfg!(test) { Duration::from_millis(60) } else { Duration::from_secs(3) }; //the least between two times a host tells it something
const MIN_BEAT: Duration = if cfg!(test) { Duration::from_millis(100) } else { Duration::from_secs(10) }; //the least and
const MAX_BEAT: Duration = Duration::from_secs(60); //most between times a host says that its game is still there
const DEFAULT_BEAT: Duration = Duration::from_secs(30);
const RETRY: Duration = if cfg!(test) { Duration::from_millis(150) } else { Duration::from_secs(10) }; //how long a host waits before trying again after it did not work
const CLOSE_WAIT: Duration = Duration::from_millis(1500); //how long closing waits for the game to be taken off the list

// ---------- what is said, and what is heard ----------

//what a host says of their game
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Announcement {
    pub(crate) name: String, //the host's
    pub(crate) port: u16,
    pub(crate) players: usize,
    pub(crate) max: usize,
    pub(crate) started: bool, //whether the run is going already (players can still join it)
}

//a game that is on the list
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ListedGame {
    pub(crate) name: String, //the host's
    pub(crate) address: IpAddr,
    pub(crate) port: u16,
    pub(crate) players: u8,
    pub(crate) max: u8,
    pub(crate) started: bool,
    pub(crate) version: String,
}

impl ListedGame {
    //what to connect to: `1.2.3.4:47821`, or `[::1]:47821`
    pub(crate) fn target(&self) -> String {
        std::net::SocketAddr::new(self.address, self.port).to_string()
    }

    pub(crate) fn is_full(&self) -> bool {
        self.players >= self.max
    }

    //a host with another version of the game would turn the player away
    pub(crate) fn same_version(&self) -> bool {
        self.version == env!("CARGO_PKG_VERSION")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DirectoryEvent {
    Games(Vec<ListedGame>),
    Unreachable(String), //the list could not be read, and why
    Listed(String),      //a game is on the list, at this address
    Unlisted(String),    //a game could not be put on the list (or has not stayed on it), and why. said once, not each time it fails
}

//the address of the service's list, from what a player typed: `https://army.example.com`, `army.example.com`, `http://127.0.0.1:8787/` and the
//whole of `https://army.example.com/api/games` all work
pub(crate) fn games_url(server: &str) -> Result<String, String> {
    let server = server.trim();
    if server.is_empty() {
        return Err("type the address of the lobby server, like https://the-army-lobby.vercel.app".to_owned());
    }
    if server.chars().any(|letter| letter.is_whitespace() || letter.is_control() || matches!(letter, '@' | '?' | '#')) {
        return Err("that is not the address of a lobby server".to_owned());
    }
    let (scheme, rest) = match server.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") => (scheme.to_ascii_lowercase(), rest),
        Some(_) => return Err("the lobby server's address starts with http:// or https://".to_owned()),
        None => ("https".to_owned(), server),
    };
    let rest = rest.trim_end_matches('/');
    let rest = rest.strip_suffix("/api/games").unwrap_or(rest);
    if rest.is_empty() || rest.starts_with('/') || rest.starts_with(':') {
        return Err("there is no name in front of the path of the lobby server's address".to_owned());
    }
    Ok(format!("{scheme}://{rest}/api/games"))
}

//the games in the service's answer. one that does not make sense is left out, and so is anything past MAX_LISTED
pub(crate) fn parse_games(answer: &Value) -> Vec<ListedGame> {
    let Some(games) = answer.get("games").and_then(Value::as_array) else {
        return Vec::new();
    };
    games
        .iter()
        .take(MAX_LISTED)
        .filter_map(|game| {
            let count = |key: &str, least: u64, most: u64| game.get(key)?.as_u64().filter(|&number| (least..=most).contains(&number));
            let name = clean_name(game.get("name")?.as_str()?);
            let version = game.get("version")?.as_str()?;
            let port = count("port", 1, u16::MAX as u64)?;
            let max = count("max", 1, 16)?;
            let players = count("players", 0, max)?;
            (!name.is_empty() && version.len() <= MAX_VERSION_LENGTH && !version.chars().any(char::is_control)).then_some(())?;
            Some(ListedGame {
                name,
                address: game.get("address")?.as_str()?.parse().ok()?,
                port: port as u16,
                players: players as u8,
                max: max as u8,
                started: game.get("started")?.as_bool()?,
                version: version.to_owned(),
            })
        })
        .collect()
}

// ---------- the service ----------

struct Service {
    agent: ureq::Agent,
    url: String,
}

impl Service {
    fn new(url: String) -> Service {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(REQUEST_SECONDS)))
            .http_status_as_error(false)
            .user_agent(format!("the-army/{}", env!("CARGO_PKG_VERSION")))
            .build();
        Service { agent: config.into(), url }
    }

    fn games(&self) -> Result<Vec<ListedGame>, String> {
        answer(self.agent.get(&self.url).call()).map(|answer| parse_games(&answer))
    }

    //tell it a game is there. gives the address it is listed at and how often to say so again
    fn announce(&self, said: &Announcement) -> Result<(String, Duration), String> {
        let body = json!({ "name": said.name, "port": said.port, "players": said.players, "max": said.max, "locked": true, "started": said.started, "version": env!("CARGO_PKG_VERSION") });
        let answer = answer(self.agent.post(&self.url).header("Content-Type", "application/json").send(body.to_string()))?;
        let address = answer.get("address").and_then(Value::as_str).unwrap_or("this machine").to_owned();
        let ttl = answer.get("ttl").and_then(Value::as_u64).map_or(DEFAULT_BEAT, |seconds| Duration::from_secs(seconds) / 3);
        Ok((address, ttl.clamp(MIN_BEAT, MAX_BEAT)))
    }

    fn withdraw(&self, port: u16) -> Result<(), String> {
        answer(self.agent.delete(format!("{}?port={port}", self.url)).call()).map(|_| ())
    }
}

//what the service answered, or what is wrong in words for the player
fn answer(result: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Value, String> {
    let mut response = result.map_err(|error| reach_problem(&error))?;
    let status = response.status();
    let text = response.body_mut().with_config().limit(MAX_ANSWER_BYTES).read_to_string().unwrap_or_default();
    let value: Option<Value> = serde_json::from_str(&text).ok();
    if status.is_success() {
        return value.ok_or_else(|| "the lobby server did not answer in a way that makes sense (is that its address?)".to_owned());
    }
    //(its own words if it had any, otherwise what the status says)
    let said = value.as_ref().and_then(|value| value.get("error")?.as_str()).map(|error| short(error).to_owned());
    Err(match said {
        Some(said) => format!("the lobby server said: {said}"),
        None => format!("the lobby server answered {} (is that its address?)", status.as_u16()),
    })
}

//why the service could not be spoken to, short enough to read
fn reach_problem(error: &ureq::Error) -> String {
    match error {
        ureq::Error::HostNotFound => "could not find the lobby server (is its address right?)".to_owned(),
        ureq::Error::Timeout(_) => "the lobby server took too long to answer".to_owned(),
        ureq::Error::ConnectionFailed | ureq::Error::Io(_) => "could not reach the lobby server (is it running, and is the address right?)".to_owned(),
        other => format!("could not talk to the lobby server ({})", short(&other.to_string())),
    }
}

//the start of a long message
fn short(text: &str) -> &str {
    let mut end = text.len().min(120);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

// ---------- the thread ----------

enum Command {
    Refresh,
    List(Announcement),
    Close,
}

//the game as the thread knows it: what it is to say, and when
struct Posting {
    said: Option<Announcement>,
    posted_at: Option<Instant>,
    due: Option<Instant>,
    every: Duration,
    working: Option<bool>, //whether the last time worked, if there has been one
}

fn work(service: Service, commands: Receiver<Command>, events: Sender<DirectoryEvent>) {
    let mut post = Posting { said: None, posted_at: None, due: None, every: DEFAULT_BEAT, working: None };
    loop {
        let wait = post.due.map_or(Duration::from_secs(3600), |at| at.saturating_duration_since(Instant::now()));
        match commands.recv_timeout(wait) {
            Ok(Command::Refresh) => {
                let event = match service.games() {
                    Ok(games) => DirectoryEvent::Games(games),
                    Err(problem) => DirectoryEvent::Unreachable(problem),
                };
                let _ = events.send(event);
            }
            Ok(Command::List(said)) => {
                if post.said.as_ref() != Some(&said) {
                    post.said = Some(said);
                    //(what changes is told at once, but not more often than the service would put up with)
                    let now = Instant::now();
                    post.due = Some(post.posted_at.map_or(now, |at| (at + MIN_PAUSE).max(now)));
                }
            }
            Ok(Command::Close) | Err(RecvTimeoutError::Disconnected) => {
                if let Some(said) = post.said.take() {
                    let _ = service.withdraw(said.port);
                }
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                let Some(said) = post.said.clone() else {
                    post.due = None;
                    continue;
                };
                let now = Instant::now();
                post.posted_at = Some(now);
                match service.announce(&said) {
                    Ok((address, every)) => {
                        post.every = every;
                        post.due = Some(now + every);
                        if post.working != Some(true) {
                            let _ = events.send(DirectoryEvent::Listed(address));
                        }
                        post.working = Some(true);
                    }
                    Err(problem) => {
                        post.due = Some(now + RETRY);
                        if post.working != Some(false) {
                            let _ = events.send(DirectoryEvent::Unlisted(problem));
                        }
                        post.working = Some(false);
                    }
                }
            }
        }
    }
}

// ---------- what the game holds ----------

//a connection to a lobby list service, for reading the list or for keeping one game on it. dropping it takes the game off
pub(crate) struct Directory {
    commands: Sender<Command>,
    events: Receiver<DirectoryEvent>,
    finished: Receiver<()>,
    said: Mutex<Option<Announcement>>, //what was last passed on, so that saying the same again each frame costs nothing
}

impl Directory {
    pub(crate) fn connect(server: &str) -> Result<Directory, String> {
        let service = Service::new(games_url(server)?);
        let (command_tx, commands) = mpsc::channel();
        let (event_tx, events) = mpsc::channel();
        let (finished_tx, finished) = mpsc::channel();
        std::thread::Builder::new()
            .name("lobby-list".to_owned())
            .spawn(move || {
                work(service, commands, event_tx);
                let _ = finished_tx.send(());
            })
            .map_err(|error| format!("could not start the lobby list: {error}"))?;
        Ok(Directory { commands: command_tx, events, finished, said: Mutex::new(None) })
    }

    //ask for the list again. the answer comes with `poll`
    pub(crate) fn refresh(&self) {
        let _ = self.commands.send(Command::Refresh);
    }

    //say that a game is there, as it is now. says it again by itself every so often until this is dropped
    pub(crate) fn list(&self, said: Announcement) {
        let mut last = self.said.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if last.as_ref() != Some(&said) {
            *last = Some(said.clone());
            let _ = self.commands.send(Command::List(said));
        }
    }

    //what has happened since this was last asked
    pub(crate) fn poll(&self) -> Vec<DirectoryEvent> {
        self.events.try_iter().collect()
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        //(a game that is closed is taken off the list at once, so no one looks for it, but a slow service is not waited for)
        let _ = self.commands.send(Command::Close);
        let _ = self.finished.recv_timeout(CLOSE_WAIT);
    }
}

impl NetHost {
    //put this game on the list of the lobby server (it is told how things stand from `announce` on). fails if the address is not one
    pub(crate) fn list_on(&mut self, server: &str) -> Result<(), String> {
        self.directory = Some(Directory::connect(server)?);
        self.announce(false);
        Ok(())
    }

    //tell the list who is in the game now, and whether it has started. nothing, if the game is not on a list
    pub(crate) fn announce(&self, started: bool) {
        let Some(directory) = &self.directory else {
            return;
        };
        let members = self.members();
        let Some(host) = members.first() else {
            return;
        };
        directory.list(Announcement { name: host.name.clone(), port: self.port, players: members.len(), max: MAX_PLAYERS, started });
    }

    //what the list has said about this game since it was last asked
    pub(crate) fn listing_news(&self) -> Vec<DirectoryEvent> {
        self.directory.as_ref().map(Directory::poll).unwrap_or_default()
    }
}
