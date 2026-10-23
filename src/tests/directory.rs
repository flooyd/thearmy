//the lobby list: what is asked of the service and what it says back. tried against a web service made for the purpose that answers over a real
//connection on this machine (lobby-server/ has its own tests for the real one)

use super::*;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener};
use std::sync::{Arc, Mutex};

const PATIENCE: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
struct Seen {
    method: String,
    path: String,
    body: String,
    agent: String,
}

struct Fake {
    url: String,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Fake {
    fn requests(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }

    fn count(&self, method: &str) -> usize {
        self.requests().iter().filter(|seen| seen.method == method).count()
    }
}

//a web service on this machine that answers each request with the status and body that `answer` gives for it
fn serve(answer: impl Fn(&Seen) -> (u16, String) + Send + 'static) -> Fake {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a port to listen on");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 2048];
            let head_end = loop {
                if let Some(at) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    break at + 4;
                }
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break bytes.len(),
                    Ok(n) => bytes.extend_from_slice(&chunk[..n]),
                }
            };
            let head = String::from_utf8_lossy(&bytes[..head_end.min(bytes.len())]).to_string();
            let mut lines = head.lines();
            let mut first = lines.next().unwrap_or("").split(' ');
            let (method, path) = (first.next().unwrap_or("").to_owned(), first.next().unwrap_or("").to_owned());
            let header = |name: &str| lines.clone().find_map(|line| line.to_ascii_lowercase().strip_prefix(&format!("{name}:")).map(|value| value.trim().to_owned()));
            let length: usize = header("content-length").and_then(|value| value.parse().ok()).unwrap_or(0);
            while bytes.len() < head_end + length {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => bytes.extend_from_slice(&chunk[..n]),
                }
            }
            let body = String::from_utf8_lossy(&bytes[head_end.min(bytes.len())..]).to_string();
            let request = Seen { method, path, body, agent: header("user-agent").unwrap_or_default() };
            let (status, text) = answer(&request);
            log.lock().unwrap().push(request);
            let reply = format!("HTTP/1.1 {status} Whatever\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
            let _ = stream.write_all(reply.as_bytes());
        }
    });
    Fake { url, seen }
}

//what the service says to a game that is listed
fn listed_as(address: &str) -> (u16, String) {
    (201, format!("{{\"id\":\"abc\",\"ttl\":1,\"address\":\"{address}\"}}"))
}

fn event_where(directory: &Directory, mut wanted: impl FnMut(&DirectoryEvent) -> bool) -> DirectoryEvent {
    let until = Instant::now() + PATIENCE;
    while Instant::now() < until {
        for event in directory.poll() {
            if wanted(&event) {
                return event;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("nothing like that was heard from the lobby list");
}

fn wait_until(mut done: impl FnMut() -> bool) {
    let until = Instant::now() + PATIENCE;
    while !done() {
        assert!(Instant::now() < until, "it did not happen in time");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn announcement(players: usize) -> Announcement {
    Announcement { name: "Ann".to_owned(), port: 47821, players, max: 4, started: false }
}

const TWO_GAMES: &str = r#"{"games":[
    {"id":"a","name":"Ann","address":"93.184.216.34","port":47821,"players":2,"max":4,"locked":true,"started":false,"version":"0.0.1","age":12},
    {"id":"b","name":"Bea","address":"2606:4700:4700::1111","port":50000,"players":4,"max":4,"locked":true,"started":true,"version":"9.9.9","age":300}
],"now":1,"ttl":90,"store":"upstash"}"#;

// ---------- addresses ----------

#[test]
fn the_address_of_a_lobby_server_is_taken_in_the_ways_it_is_written() {
    for typed in ["army.example.com", "https://army.example.com", "https://army.example.com/", "  https://army.example.com/api/games  ", "HTTPS://army.example.com//", "army.example.com/api/games/"] {
        assert_eq!(games_url(typed).as_deref(), Ok("https://army.example.com/api/games"), "{typed}");
    }
    assert_eq!(games_url("http://127.0.0.1:8787").as_deref(), Ok("http://127.0.0.1:8787/api/games"));
    assert_eq!(games_url("army.example.com:8443/lobby").as_deref(), Ok("https://army.example.com:8443/lobby/api/games"));
    for bad in ["", "   ", "ftp://army.example.com", "https://", "https:///api/games", "a b", "https://user@army.example.com", "army.example.com?x=1", "army.example.com#top", "://x"] {
        assert!(games_url(bad).is_err(), "{bad:?} is no address");
    }
}

#[test]
fn the_games_in_an_answer_are_read_and_the_ones_that_make_no_sense_are_left_out() {
    let answer: serde_json::Value = serde_json::from_str(TWO_GAMES).unwrap();
    let games = parse_games(&answer);
    assert_eq!(games.len(), 2);
    assert_eq!((games[0].name.as_str(), games[0].players, games[0].max, games[0].started), ("Ann", 2, 4, false));
    assert_eq!(games[0].target(), "93.184.216.34:47821");
    assert_eq!(games[1].target(), "[2606:4700:4700::1111]:50000", "an address of the newer kind is put in brackets to connect to");
    assert!(games[1].is_full());
    assert!(!games[0].same_version() && !games[1].same_version(), "(these are not this version)");
    assert!(ListedGame { version: env!("CARGO_PKG_VERSION").to_owned(), ..games[0].clone() }.same_version());
    //bad ones, each wrong in one way
    let one = |change: &str, to: &str| {
        let mut game: serde_json::Value = serde_json::from_str(r#"{"name":"Ann","address":"93.184.216.34","port":47821,"players":2,"max":4,"locked":true,"started":false,"version":"0.1.4"}"#).unwrap();
        game[change] = serde_json::from_str(to).unwrap();
        parse_games(&serde_json::json!({ "games": [game] }))
    };
    assert_eq!(one("players", "2").len(), 1);
    for (key, value) in [("name", "\"!!!\""), ("name", "7"), ("address", "\"not an address\""), ("address", "3"), ("port", "0"), ("port", "70000"), ("port", "\"47821\""), ("max", "0"), ("max", "99"), ("players", "5"), ("players", "-1"), ("started", "\"no\""), ("version", "\"a\\u0000b\""), ("version", "null"), ("version", &format!("\"{}\"", "x".repeat(40)))] {
        assert!(one(key, value).is_empty(), "{key}: {value}");
    }
    //and an answer that is not a list gives none
    for odd in ["{}", "{\"games\":7}", "[]", "null", "{\"games\":{}}"] {
        assert!(parse_games(&serde_json::from_str(odd).unwrap()).is_empty(), "{odd}");
    }
    //a name is cleaned as the game cleans them, so nothing odd is ever drawn
    let cleaned = one("name", "\"  <Ann\\u202e> \"");
    assert_eq!(cleaned[0].name, "Ann");
}

#[test]
fn only_so_many_games_are_looked_at() {
    let games: Vec<serde_json::Value> = (0..300).map(|i| serde_json::json!({"name":"Ann","address":"93.184.216.34","port":2000 + i,"players":1,"max":4,"started":false,"version":"0.1.4"})).collect();
    assert_eq!(parse_games(&serde_json::json!({ "games": games })).len(), 100);
}

// ---------- reading the list ----------

#[test]
fn the_list_is_read_from_the_service() {
    let fake = serve(|_| (200, TWO_GAMES.to_owned()));
    let directory = Directory::connect(&fake.url).unwrap();
    directory.refresh();
    let DirectoryEvent::Games(games) = event_where(&directory, |event| matches!(event, DirectoryEvent::Games(_))) else { unreachable!() };
    assert_eq!(games.iter().map(|game| game.name.as_str()).collect::<Vec<_>>(), ["Ann", "Bea"]);
    let seen = fake.requests();
    assert_eq!((seen[0].method.as_str(), seen[0].path.as_str()), ("GET", "/api/games"));
    assert_eq!(seen[0].agent, format!("the-army/{}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn a_service_that_is_not_there_is_said_to_be_out_of_reach() {
    let gone = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let url = format!("http://{}", gone.local_addr().unwrap());
    drop(gone);
    let directory = Directory::connect(&url).unwrap();
    directory.refresh();
    let DirectoryEvent::Unreachable(problem) = event_where(&directory, |event| matches!(event, DirectoryEvent::Unreachable(_))) else { unreachable!() };
    assert!(problem.starts_with("could not reach the lobby server"), "{problem}");
}

#[test]
fn what_the_service_says_is_wrong_is_passed_on() {
    let fake = serve(|_| (429, r#"{"error":"too many requests, try again in a moment"}"#.to_owned()));
    let directory = Directory::connect(&fake.url).unwrap();
    directory.refresh();
    let DirectoryEvent::Unreachable(problem) = event_where(&directory, |event| matches!(event, DirectoryEvent::Unreachable(_))) else { unreachable!() };
    assert_eq!(problem, "the lobby server said: too many requests, try again in a moment");
}

#[test]
fn an_answer_from_something_else_is_noticed() {
    for (status, body, expected) in [(200, "<html>hello</html>", "makes sense"), (404, "<html>not found</html>", "answered 404"), (500, "", "answered 500"), (200, "", "makes sense")] {
        let fake = serve(move |_| (status, body.to_owned()));
        let directory = Directory::connect(&fake.url).unwrap();
        directory.refresh();
        let DirectoryEvent::Unreachable(problem) = event_where(&directory, |event| matches!(event, DirectoryEvent::Unreachable(_))) else { unreachable!() };
        assert!(problem.contains(expected), "{status} {body}: {problem}");
    }
}

#[test]
fn a_lobby_server_that_is_not_an_address_is_refused_at_once() {
    assert!(Directory::connect("ftp://nope").is_err());
    assert!(Directory::connect("").is_err());
}

// ---------- listing a game ----------

#[test]
fn a_game_is_listed_with_what_the_service_needs_and_kept_listed() {
    let fake = serve(|_| listed_as("93.184.216.34"));
    let directory = Directory::connect(&fake.url).unwrap();
    directory.list(announcement(1));
    let DirectoryEvent::Listed(address) = event_where(&directory, |event| matches!(event, DirectoryEvent::Listed(_))) else { unreachable!() };
    assert_eq!(address, "93.184.216.34", "the host is told where it is listed");
    let seen = fake.requests();
    assert_eq!((seen[0].method.as_str(), seen[0].path.as_str()), ("POST", "/api/games"));
    let body: serde_json::Value = serde_json::from_str(&seen[0].body).unwrap();
    assert_eq!(body, serde_json::json!({ "name": "Ann", "port": 47821, "players": 1, "max": 4, "locked": true, "started": false, "version": env!("CARGO_PKG_VERSION") }));
    //(it is said again by itself, as the service forgets a game it hears nothing of)
    wait_until(|| fake.count("POST") >= 3);
    //and the host is told it only once
    assert!(directory.poll().iter().all(|event| !matches!(event, DirectoryEvent::Listed(_))));
}

#[test]
fn a_change_is_passed_on_and_the_same_thing_said_again_is_not() {
    let fake = serve(|_| listed_as("93.184.216.34"));
    let directory = Directory::connect(&fake.url).unwrap();
    directory.list(announcement(1));
    wait_until(|| fake.count("POST") >= 1);
    for _ in 0..50 {
        directory.list(announcement(1));
    }
    directory.list(announcement(2));
    wait_until(|| fake.requests().iter().any(|seen| seen.body.contains("\"players\":2")));
    let started = Announcement { started: true, ..announcement(2) };
    directory.list(started);
    wait_until(|| fake.requests().iter().any(|seen| seen.body.contains("\"started\":true")));
}

#[test]
fn a_game_is_taken_off_the_list_when_it_is_let_go() {
    let fake = serve(|seen| if seen.method == "DELETE" { (200, "{\"removed\":true}".to_owned()) } else { listed_as("93.184.216.34") });
    let directory = Directory::connect(&fake.url).unwrap();
    directory.list(announcement(1));
    wait_until(|| fake.count("POST") >= 1);
    drop(directory);
    let seen = fake.requests();
    let delete = seen.iter().find(|seen| seen.method == "DELETE").expect("it was taken off");
    assert_eq!(delete.path, "/api/games?port=47821");
}

#[test]
fn nothing_is_taken_off_that_was_never_put_on() {
    let fake = serve(|_| (200, TWO_GAMES.to_owned()));
    let directory = Directory::connect(&fake.url).unwrap();
    directory.refresh();
    event_where(&directory, |event| matches!(event, DirectoryEvent::Games(_)));
    drop(directory);
    assert_eq!(fake.count("DELETE"), 0);
}

#[test]
fn trouble_listing_is_said_once_and_so_is_the_end_of_it() {
    let tries = Arc::new(Mutex::new(0));
    let counted = tries.clone();
    let fake = serve(move |_| {
        let mut tries = counted.lock().unwrap();
        *tries += 1;
        if *tries <= 3 { (503, r#"{"error":"the list is full, try again later"}"#.to_owned()) } else { listed_as("93.184.216.34") }
    });
    let directory = Directory::connect(&fake.url).unwrap();
    directory.list(announcement(1));
    let mut heard = Vec::new();
    let until = Instant::now() + PATIENCE;
    while !heard.iter().any(|event| matches!(event, DirectoryEvent::Listed(_))) {
        assert!(Instant::now() < until, "never listed: {heard:?}");
        heard.extend(directory.poll());
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(heard.len(), 2, "one complaint, then the good news: {heard:?}");
    assert_eq!(heard[0], DirectoryEvent::Unlisted("the lobby server said: the list is full, try again later".to_owned()));
    assert!(fake.count("POST") >= 4, "it kept trying");
}

// ---------- a game that is hosted ----------

fn hosting_for_the_list(name: &str) -> NetHost {
    let mut config = HostConfig::new(name, "open sesame", 0);
    config.address = Ipv4Addr::LOCALHOST.into();
    NetHost::start(config).expect("the host starts")
}

#[test]
fn a_hosted_game_tells_the_list_who_is_in_it_and_whether_it_has_started() {
    let fake = serve(|_| listed_as("93.184.216.34"));
    let mut host = hosting_for_the_list("Hostess");
    host.announce(false);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(fake.count("POST"), 0, "a game that is not on a list says nothing");
    host.list_on(&fake.url).unwrap();
    wait_until(|| fake.count("POST") >= 1);
    let body: serde_json::Value = serde_json::from_str(&fake.requests()[0].body).unwrap();
    assert_eq!((body["name"].as_str(), body["port"].as_u64(), body["players"].as_u64(), body["started"].as_bool()), (Some("Hostess"), Some(host.port as u64), Some(1), Some(false)));
    let mut news = Vec::new();
    wait_until(|| {
        news.extend(host.listing_news());
        news.iter().any(|news| matches!(news, DirectoryEvent::Listed(_)))
    });
    host.announce(true);
    wait_until(|| fake.requests().iter().any(|seen| seen.body.contains("\"started\":true")));
    drop(host);
    assert_eq!(fake.count("DELETE"), 1, "ending the game takes it off the list");
}

#[test]
fn a_hosted_game_will_not_be_put_on_a_list_that_is_not_an_address() {
    let mut host = hosting_for_the_list("Hostess");
    assert!(host.list_on("ftp://nope").is_err());
    assert!(host.directory.is_none());
    assert!(host.listing_news().is_empty());
}

// ---------- the real service ----------

//needs a lobby server running (`npm run dev` in lobby-server/), and its address in THE_ARMY_TEST_LOBBY: cargo test -- --ignored the_real_lobby_server
#[test]
#[ignore]
fn the_real_lobby_server_and_the_game_agree_about_what_is_said() {
    let Ok(server) = std::env::var("THE_ARMY_TEST_LOBBY") else { panic!("THE_ARMY_TEST_LOBBY is not set") };
    let mut host = hosting_for_the_list("Hostess");
    host.list_on(&server).unwrap();
    let mut news = Vec::new();
    wait_until(|| {
        news.extend(host.listing_news());
        news.iter().any(|news| matches!(news, DirectoryEvent::Listed(_) | DirectoryEvent::Unlisted(_)))
    });
    assert!(matches!(&news[0], DirectoryEvent::Listed(address) if address == "127.0.0.1"), "{news:?}");
    //a player looking for a game sees it
    let looker = Directory::connect(&server).unwrap();
    looker.refresh();
    let DirectoryEvent::Games(games) = event_where(&looker, |event| matches!(event, DirectoryEvent::Games(_))) else { unreachable!() };
    let game = games.iter().find(|game| game.port == host.port).expect("the game is on the list");
    assert_eq!((game.name.as_str(), game.players, game.max, game.started), ("Hostess", 1, 4 as u8, false));
    assert_eq!(game.target(), format!("127.0.0.1:{}", host.port));
    assert!(game.same_version());
    //and can join it by what the list says
    let mut client = NetClient::connect(&game.target(), "Ann", "open sesame");
    let until = Instant::now() + PATIENCE;
    loop {
        assert!(Instant::now() < until, "never joined");
        if client.poll().iter().any(|event| matches!(event, ClientEvent::Joined { .. })) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    //the list is told when the second player is in
    host.announce(false);
    let until = Instant::now() + PATIENCE;
    loop {
        assert!(Instant::now() < until, "the list never heard of the second player");
        looker.refresh();
        std::thread::sleep(Duration::from_millis(150));
        let seen = looker.poll().into_iter().find_map(|event| if let DirectoryEvent::Games(games) = event { games.into_iter().find(|game| game.port == host.port) } else { None });
        if seen.is_some_and(|game| game.players == 2) {
            break;
        }
    }
    drop(client);
    drop(host);
    looker.refresh();
    let DirectoryEvent::Games(games) = event_where(&looker, |event| matches!(event, DirectoryEvent::Games(_))) else { unreachable!() };
    assert!(games.iter().all(|game| game.name != "Hostess"), "the game came off the list when the host ended it");
}

#[test]
fn the_game_knows_where_the_lobby_list_is_without_being_told() {
    assert_eq!(LOBBY_SERVER, "https://lobby-server-virid.vercel.app");
    assert_eq!(games_url(LOBBY_SERVER).as_deref(), Ok("https://lobby-server-virid.vercel.app/api/games"));
    //(a form starts out with it, and a settings file from when it was a box on the form does not change that)
    let form = Form::new(&Settings::from_text("lobby_server https://old.example.com\n"));
    assert_eq!(form.server, lobby_server());
    assert!(!Settings::default().to_text().contains("lobby_server"));
}
