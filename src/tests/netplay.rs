//! a host and the players who join it, over real connections on this machine (127.0.0.1): joining, being turned away, leaving, being put out, and what a
//! host does with a connection that says what no player would

use super::*;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use std::net::{Ipv4Addr, SocketAddr};

const PASSWORD: &str = "correct horse";
const PATIENCE: Duration = Duration::from_secs(12);

// ---------- helpers ----------

fn hosting(name: &str, password: &str) -> NetHost {
    let mut config = HostConfig::new(name, password, 0);
    config.address = Ipv4Addr::LOCALHOST.into();
    NetHost::start(config).expect("the host starts")
}

fn join(host: &NetHost, name: &str, password: &str) -> NetClient {
    NetClient::connect(&format!("127.0.0.1:{}", host.port), name, password)
}

//the next event a player hears that `wanted` picks out (and it fails if there is none soon). the ones before it are passed over, the ones after it are kept
fn await_client(client: &mut NetClient, mut wanted: impl FnMut(&ClientEvent) -> bool) -> ClientEvent {
    let until = Instant::now() + PATIENCE;
    loop {
        while let Some(event) = client.next() {
            if wanted(&event) {
                return event;
            }
        }
        assert!(Instant::now() < until, "the player never heard what it was waiting for");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn await_host(host: &mut NetHost, mut wanted: impl FnMut(&HostEvent) -> bool) -> HostEvent {
    let until = Instant::now() + PATIENCE;
    loop {
        while let Some(event) = host.next() {
            if wanted(&event) {
                return event;
            }
        }
        assert!(Instant::now() < until, "the host never heard what it was waiting for");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn names(members: &[Member]) -> Vec<&str> {
    members.iter().map(|member| member.name.as_str()).collect()
}

//a player that is in the game
fn joined(host: &NetHost, name: &str) -> NetClient {
    let mut client = join(host, name, PASSWORD);
    await_client(&mut client, |event| matches!(event, ClientEvent::Joined { .. }));
    client
}

fn refused_with(host: &NetHost, name: &str, password: &str) -> Refusal {
    let mut client = join(host, name, password);
    match await_client(&mut client, |event| matches!(event, ClientEvent::Refused(_) | ClientEvent::Failed(_) | ClientEvent::Joined { .. })) {
        ClientEvent::Refused(refusal) => refusal,
        other => panic!("expected to be refused, got {other:?}"),
    }
}

//a connection made by hand, to say what a real player would not
struct Raw {
    _endpoint: Endpoint,
    connection: Connection,
    send: SendStream,
    receive: RecvStream,
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread().enable_all().build().expect("a runtime")
}

async fn raw_connect(port: u16) -> Raw {
    let mut endpoint = Endpoint::client(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).expect("an endpoint");
    endpoint.set_default_client_config(client_config().expect("a config"));
    let connection = endpoint.connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port)), SERVER_NAME).expect("connecting").await.expect("connected");
    let (send, receive) = connection.open_bi().await.expect("a stream");
    Raw { _endpoint: endpoint, connection, send, receive }
}

impl Raw {
    async fn say(&mut self, message: &ClientMsg) {
        write_frame(&mut self.send, &message.to_bytes()).await.expect("sent");
    }

    async fn hear(&mut self) -> Option<ServerMsg> {
        let bytes = read_frame(&mut self.receive).await.ok()?;
        ServerMsg::from_bytes(&bytes)
    }

    //says hello and answers the challenge the way a player would, with the password and name asked for, and the proof tied to `binding` (this connection's own, if None)
    async fn join(port: u16, name: &str, password: &str, protocol: u16, binding: Option<[u8; BINDING_BYTES]>) -> (Raw, ServerMsg, [u8; PROOF_BYTES]) {
        let mut raw = raw_connect(port).await;
        raw.say(&ClientMsg::Hello { protocol, version: "test".to_owned(), name: name.to_owned() }).await;
        let Some(ServerMsg::Challenge { salt, nonce, iterations }) = raw.hear().await else {
            let answer = raw.hear().await;
            panic!("no challenge: {answer:?}");
        };
        let binding = binding.unwrap_or_else(|| binding_of(&raw.connection).expect("a binding"));
        let proof = make_proof(&derive_key(password, &salt, iterations), &nonce, &binding, name);
        raw.say(&ClientMsg::Proof(proof)).await;
        let answer = raw.hear().await.expect("an answer");
        (raw, answer, proof)
    }

    //whether the host closes the connection, within `seconds`
    async fn is_closed_within(&self, seconds: u64) -> bool {
        tokio::time::timeout(Duration::from_secs(seconds), self.connection.closed()).await.is_ok()
    }
}

// ---------- addresses ----------

#[test]
fn an_address_is_read_the_ways_people_write_it() {
    let default = DEFAULT_PORT;
    assert_eq!(parse_target("203.0.113.7").unwrap(), format!("203.0.113.7:{default}"));
    assert_eq!(parse_target("  203.0.113.7:5000 ").unwrap(), "203.0.113.7:5000");
    assert_eq!(parse_target("my-pc.example.com").unwrap(), format!("my-pc.example.com:{default}"));
    assert_eq!(parse_target("my-pc.example.com:6001").unwrap(), "my-pc.example.com:6001");
    assert_eq!(parse_target("[::1]").unwrap(), format!("[::1]:{default}"));
    assert_eq!(parse_target("[::1]:6001").unwrap(), "[::1]:6001");
    assert_eq!(parse_target("::1").unwrap(), format!("[::1]:{default}"), "an address with no brackets has no port");
}

#[test]
fn an_address_that_cannot_be_one_says_what_is_wrong() {
    for bad in ["", "   ", "a b", "host:", "host:0", "host:70000", "host:port", ":5000", "[::1", "[::1]x", "[::1]:", "host\t:1"] {
        assert!(parse_target(bad).is_err(), "{bad:?}");
    }
    assert!(parse_target("").unwrap_err().contains("address"));
}

#[test]
fn a_port_to_host_on_is_one_a_program_may_use() {
    assert_eq!(parse_port("47821").unwrap(), 47821);
    assert_eq!(parse_port(" 1024 ").unwrap(), 1024);
    assert_eq!(parse_port("65535").unwrap(), 65535);
    for bad in ["", "1023", "80", "0", "65536", "-1", "abc", "47 821"] {
        assert!(parse_port(bad).is_err(), "{bad:?}");
    }
}

// ---------- hosting ----------

#[test]
fn a_host_needs_a_name_and_a_password_that_will_do() {
    let mut config = HostConfig::new("", PASSWORD, 0);
    config.address = Ipv4Addr::LOCALHOST.into();
    assert!(NetHost::start(config).is_err(), "no name");
    let mut config = HostConfig::new("Hostess", "abc", 0);
    config.address = Ipv4Addr::LOCALHOST.into();
    assert!(NetHost::start(config).err().expect("a password that is too short is refused").contains("password"));
}

#[test]
fn a_host_is_listening_and_alone_to_begin_with() {
    let host = hosting("Hostess", PASSWORD);
    assert!(host.port > 0);
    assert_eq!(names(&host.members()), ["Hostess"]);
    assert_eq!(host.members()[0].slot, 0);
    assert!(host.poll().is_empty());
}

#[test]
fn a_port_in_use_is_said_so_and_is_free_again_when_the_host_is_gone() {
    let first = hosting("Hostess", PASSWORD);
    let port = first.port;
    let mut config = HostConfig::new("Other", PASSWORD, port);
    config.address = Ipv4Addr::LOCALHOST.into();
    let problem = NetHost::start(config).err().expect("the port is taken");
    assert!(problem.contains(&port.to_string()), "{problem}");
    drop(first);
    let mut config = HostConfig::new("Other", PASSWORD, port);
    config.address = Ipv4Addr::LOCALHOST.into();
    assert!(NetHost::start(config).is_ok(), "the port is free again once the first host is gone");
}

// ---------- joining ----------

#[test]
fn a_player_with_the_password_joins_and_both_sides_know() {
    let mut host = hosting("Hostess", PASSWORD);
    let mut ann = join(&host, "Ann", PASSWORD);
    let ClientEvent::Joined { slot, members } = await_client(&mut ann, |event| matches!(event, ClientEvent::Joined { .. })) else { unreachable!() };
    assert_eq!(slot, 1);
    assert_eq!(names(&members), ["Hostess", "Ann"]);
    let HostEvent::Joined { slot, name, address } = await_host(&mut host, |event| matches!(event, HostEvent::Joined { .. })) else { unreachable!() };
    assert_eq!((slot, name.as_str()), (1, "Ann"));
    assert!(address.ip().is_loopback());
    assert_eq!(names(&host.members()), ["Hostess", "Ann"]);
}

#[test]
fn the_name_a_player_gives_is_cleaned_the_way_the_game_cleans_it() {
    let host = hosting("Hostess", PASSWORD);
    let mut client = join(&host, "  Ann   ", PASSWORD);
    await_client(&mut client, |event| matches!(event, ClientEvent::Joined { .. }));
    assert_eq!(names(&host.members()), ["Hostess", "Ann"]);
    let _ = host.poll();
}

#[test]
fn the_others_are_told_when_someone_joins() {
    let host = hosting("Hostess", PASSWORD);
    let mut ann = joined(&host, "Ann");
    let _bob = joined(&host, "Bob");
    let ClientEvent::Roster(members) = await_client(&mut ann, |event| matches!(event, ClientEvent::Roster(members) if members.len() == 3)) else { unreachable!() };
    assert_eq!(names(&members), ["Hostess", "Ann", "Bob"]);
    assert_eq!(members.iter().map(|member| member.slot).collect::<Vec<_>>(), [0, 1, 2]);
}

#[test]
fn a_wrong_password_is_refused_and_told_to_the_host() {
    let mut host = hosting("Hostess", PASSWORD);
    assert_eq!(refused_with(&host, "Ann", "correct horsE"), Refusal::WrongPassword);
    let HostEvent::TurnedAway { why, .. } = await_host(&mut host, |event| matches!(event, HostEvent::TurnedAway { .. })) else { unreachable!() };
    assert!(why.contains("password"), "{why}");
    assert_eq!(names(&host.members()), ["Hostess"]);
}

#[test]
fn a_name_in_use_is_refused_whatever_its_capitals() {
    let host = hosting("Hostess", PASSWORD);
    assert_eq!(refused_with(&host, "hostess", PASSWORD), Refusal::NameTaken, "the host's own name");
    let _ann = joined(&host, "Ann");
    assert_eq!(refused_with(&host, "ANN", PASSWORD), Refusal::NameTaken, "a player's");
}

#[test]
fn a_name_with_nothing_usable_in_it_is_refused() {
    let host = hosting("Hostess", PASSWORD);
    assert_eq!(refused_with(&host, "!!!", PASSWORD), Refusal::BadName);
    assert_eq!(refused_with(&host, "   ", PASSWORD), Refusal::BadName);
}

#[test]
fn a_game_has_room_for_four_and_the_fifth_is_told_it_is_full() {
    let host = hosting("Hostess", PASSWORD);
    let _one = joined(&host, "One");
    let _two = joined(&host, "Two");
    let _three = joined(&host, "Three");
    assert_eq!(host.members().len(), MAX_PLAYERS);
    assert_eq!(refused_with(&host, "Late", PASSWORD), Refusal::Full);
}

#[test]
fn a_place_that_is_left_is_given_to_the_next_player() {
    let mut host = hosting("Hostess", PASSWORD);
    let mut ann = joined(&host, "Ann");
    let _bob = joined(&host, "Bob");
    ann.leave();
    await_host(&mut host, |event| matches!(event, HostEvent::Left { name, .. } if name == "Ann"));
    let mut cy = join(&host, "Cy", PASSWORD);
    let ClientEvent::Joined { slot, .. } = await_client(&mut cy, |event| matches!(event, ClientEvent::Joined { .. })) else { unreachable!() };
    assert_eq!(slot, 1, "the place Ann left");
    assert_eq!(names(&host.members()), ["Hostess", "Cy", "Bob"]);
}

// ---------- leaving and being put out ----------

#[test]
fn a_player_who_leaves_is_missed_by_the_host_and_the_others() {
    let mut host = hosting("Hostess", PASSWORD);
    let mut ann = joined(&host, "Ann");
    let mut bob = joined(&host, "Bob");
    ann.leave();
    let HostEvent::Left { slot, name, why } = await_host(&mut host, |event| matches!(event, HostEvent::Left { .. })) else { unreachable!() };
    assert_eq!((slot, name.as_str()), (1, "Ann"));
    assert!(why.contains("left"), "{why}");
    await_client(&mut bob, |event| matches!(event, ClientEvent::Roster(members) if members.len() == 2));
    assert_eq!(names(&host.members()), ["Hostess", "Bob"]);
}

#[test]
fn dropping_a_player_says_goodbye_too() {
    let mut host = hosting("Hostess", PASSWORD);
    let ann = joined(&host, "Ann");
    drop(ann);
    await_host(&mut host, |event| matches!(event, HostEvent::Left { name, .. } if name == "Ann"));
    assert_eq!(names(&host.members()), ["Hostess"]);
}

#[test]
fn a_connection_that_goes_without_a_word_is_noticed() {
    let mut host = hosting("Hostess", PASSWORD);
    let port = host.port;
    runtime().block_on(async {
        let (raw, answer, _) = Raw::join(port, "Ann", PASSWORD, PROTOCOL, None).await;
        assert!(matches!(answer, ServerMsg::Welcome { slot: 1, .. }), "{answer:?}");
        raw.connection.close(5u32.into(), b"gone");
        raw._endpoint.wait_idle().await;
    });
    let HostEvent::Left { why, .. } = await_host(&mut host, |event| matches!(event, HostEvent::Left { .. })) else { unreachable!() };
    assert!(why.contains("ended"), "{why}");
    assert_eq!(names(&host.members()), ["Hostess"]);
}

#[test]
fn a_player_who_sends_what_nobody_would_is_put_out() {
    let mut host = hosting("Hostess", PASSWORD);
    let port = host.port;
    runtime().block_on(async {
        let (mut raw, _, _) = Raw::join(port, "Ann", PASSWORD, PROTOCOL, None).await;
        write_frame(&mut raw.send, &[200, 1, 2, 3]).await.unwrap();
        assert!(raw.is_closed_within(5).await, "the host closed the connection");
    });
    let HostEvent::Left { why, .. } = await_host(&mut host, |event| matches!(event, HostEvent::Left { .. })) else { unreachable!() };
    assert!(why.contains("understood"), "{why}");
}

#[test]
fn the_host_can_put_a_player_out() {
    let mut host = hosting("Hostess", PASSWORD);
    let mut ann = joined(&host, "Ann");
    host.kick(1);
    assert_eq!(await_client(&mut ann, |event| matches!(event, ClientEvent::Refused(_))), ClientEvent::Refused(Refusal::Kicked));
    let HostEvent::Left { name, why, .. } = await_host(&mut host, |event| matches!(event, HostEvent::Left { .. })) else { unreachable!() };
    assert_eq!(name, "Ann");
    assert!(why.contains("put out"), "{why}");
    assert_eq!(names(&host.members()), ["Hostess"]);
    //(and the host can't put out itself, or a place that is empty)
    host.kick(0);
    host.kick(3);
    assert_eq!(names(&host.members()), ["Hostess"]);
}

#[test]
fn starting_the_run_tells_every_player() {
    let host = hosting("Hostess", PASSWORD);
    let mut ann = joined(&host, "Ann");
    let mut bob = joined(&host, "Bob");
    host.start_run();
    assert_eq!(await_client(&mut ann, |event| matches!(event, ClientEvent::Started)), ClientEvent::Started);
    assert_eq!(await_client(&mut bob, |event| matches!(event, ClientEvent::Started)), ClientEvent::Started);
}

#[test]
fn closing_the_game_tells_the_players_it_is_closed() {
    let host = hosting("Hostess", PASSWORD);
    let mut ann = joined(&host, "Ann");
    drop(host);
    assert_eq!(await_client(&mut ann, |event| matches!(event, ClientEvent::Refused(_) | ClientEvent::Closed(_))), ClientEvent::Refused(Refusal::HostClosed));
}

#[test]
fn the_players_are_told_how_far_away_everyone_is() {
    let host = hosting("Hostess", PASSWORD);
    let mut ann = joined(&host, "Ann");
    //(the host tells everyone who is in the game every couple of seconds, and a connection on this machine is a few milliseconds at most)
    let ClientEvent::Roster(members) = await_client(&mut ann, |event| matches!(event, ClientEvent::Roster(_))) else { unreachable!() };
    assert_eq!(members[0].ping_ms, 0, "the host is not far from itself");
    assert!(members.iter().all(|member| member.ping_ms < 1000), "{members:?}");
}

// ---------- what a player or a host that is not honest does ----------

#[test]
fn a_player_with_another_version_of_the_game_is_told_so() {
    let host = hosting("Hostess", PASSWORD);
    let port = host.port;
    runtime().block_on(async {
        let mut raw = raw_connect(port).await;
        raw.say(&ClientMsg::Hello { protocol: PROTOCOL + 1, version: "9.9.9".to_owned(), name: "Ann".to_owned() }).await;
        let Some(ServerMsg::Refused(Refusal::WrongVersion { host_protocol, .. })) = raw.hear().await else { panic!("not turned away for the version") };
        assert_eq!(host_protocol, PROTOCOL);
    });
    assert_eq!(names(&host.members()), ["Hostess"]);
}

#[test]
fn a_proof_made_for_another_connection_is_no_proof() {
    //the password is right but the proof is tied to a connection that is not this one, as it would be if it were passed on from another
    let host = hosting("Hostess", PASSWORD);
    let port = host.port;
    runtime().block_on(async {
        let (_, answer, _) = Raw::join(port, "Ann", PASSWORD, PROTOCOL, Some([1; BINDING_BYTES])).await;
        assert_eq!(answer, ServerMsg::Refused(Refusal::WrongPassword));
    });
}

#[test]
fn a_proof_that_was_overheard_cannot_be_used_again() {
    let host = hosting("Hostess", PASSWORD);
    let port = host.port;
    runtime().block_on(async {
        //(a player gets in, and what they sent is kept)
        let (first, answer, proof) = Raw::join(port, "Ann", PASSWORD, PROTOCOL, None).await;
        assert!(matches!(answer, ServerMsg::Welcome { .. }));
        first.connection.close(0u32.into(), b"done");
        //(and sent again on another connection, which has a challenge of its own)
        let mut raw = raw_connect(port).await;
        raw.say(&ClientMsg::Hello { protocol: PROTOCOL, version: "test".to_owned(), name: "Ann".to_owned() }).await;
        assert!(matches!(raw.hear().await, Some(ServerMsg::Challenge { .. })));
        raw.say(&ClientMsg::Proof(proof)).await;
        assert_eq!(raw.hear().await, Some(ServerMsg::Refused(Refusal::WrongPassword)));
    });
}

#[test]
fn a_proof_cannot_be_used_for_another_name() {
    let host = hosting("Hostess", PASSWORD);
    let port = host.port;
    runtime().block_on(async {
        let mut raw = raw_connect(port).await;
        raw.say(&ClientMsg::Hello { protocol: PROTOCOL, version: "test".to_owned(), name: "Bob".to_owned() }).await;
        let Some(ServerMsg::Challenge { salt, nonce, iterations }) = raw.hear().await else { panic!("no challenge") };
        //(made for Ann, but sent as Bob)
        let proof = make_proof(&derive_key(PASSWORD, &salt, iterations), &nonce, &binding_of(&raw.connection).unwrap(), "Ann");
        raw.say(&ClientMsg::Proof(proof)).await;
        assert_eq!(raw.hear().await, Some(ServerMsg::Refused(Refusal::WrongPassword)));
    });
}

#[test]
fn what_a_host_asks_a_player_is_new_each_time() {
    let host = hosting("Hostess", PASSWORD);
    let port = host.port;
    runtime().block_on(async {
        let mut challenges = Vec::new();
        for _ in 0..2 {
            let mut raw = raw_connect(port).await;
            raw.say(&ClientMsg::Hello { protocol: PROTOCOL, version: "test".to_owned(), name: "Ann".to_owned() }).await;
            let Some(ServerMsg::Challenge { salt, nonce, iterations }) = raw.hear().await else { panic!("no challenge") };
            challenges.push((salt, nonce, iterations));
        }
        assert_eq!(challenges[0].0, challenges[1].0, "the salt belongs to the host's password");
        assert_ne!(challenges[0].1, challenges[1].1, "the nonce is new each time");
        assert_eq!(challenges[0].2, KDF_ITERATIONS);
    });
}

#[test]
fn a_connection_that_does_not_say_hello_properly_is_dropped() {
    let host = hosting("Hostess", PASSWORD);
    let port = host.port;
    runtime().block_on(async {
        //noise
        let mut raw = raw_connect(port).await;
        write_frame(&mut raw.send, &[1, 2, 3, 4, 5, 6, 7, 8]).await.unwrap();
        assert!(raw.is_closed_within(5).await, "noise");
        //a message that is a different one (a proof before a hello)
        let mut raw = raw_connect(port).await;
        raw.say(&ClientMsg::Proof([0; PROOF_BYTES])).await;
        assert!(raw.is_closed_within(5).await, "a proof before a hello");
        //a message that says it is longer than any is allowed to be
        let mut raw = raw_connect(port).await;
        raw.send.write_all(&(MAX_FRAME as u16 + 1).to_le_bytes()).await.unwrap();
        raw.send.write_all(&[0; 64]).await.unwrap();
        assert!(raw.is_closed_within(5).await, "a message too long");
        //a message of no length
        let mut raw = raw_connect(port).await;
        raw.send.write_all(&[0, 0]).await.unwrap();
        assert!(raw.is_closed_within(5).await, "a message of nothing");
    });
    assert_eq!(names(&host.members()), ["Hostess"]);
}

#[test]
fn a_connection_that_stops_halfway_through_joining_is_dropped() {
    let host = hosting("Hostess", PASSWORD);
    let port = host.port;
    runtime().block_on(async {
        let mut raw = raw_connect(port).await;
        raw.say(&ClientMsg::Hello { protocol: PROTOCOL, version: "test".to_owned(), name: "Ann".to_owned() }).await;
        assert!(matches!(raw.hear().await, Some(ServerMsg::Challenge { .. })));
        //(and then says nothing at all)
        assert!(raw.is_closed_within(HANDSHAKE_SECONDS + 4).await, "the host gave up on it");
    });
    assert_eq!(names(&host.members()), ["Hostess"]);
}

#[test]
fn too_many_wrong_passwords_and_the_address_has_to_wait() {
    let mut host = hosting("Hostess", PASSWORD);
    for _ in 0..=FREE_FAILURES {
        assert_eq!(refused_with(&host, "Ann", "not the password"), Refusal::WrongPassword);
    }
    //now even the right password is not tried
    let Refusal::TooManyTries { wait_seconds } = refused_with(&host, "Ann", PASSWORD) else { panic!("not told to wait") };
    assert!((1..=LOCKOUT_SECONDS as u16).contains(&wait_seconds), "{wait_seconds}");
    assert_eq!(names(&host.members()), ["Hostess"]);
    //(the host hears of each)
    let mut turned_away = 0;
    while turned_away < FREE_FAILURES as usize + 2 {
        await_host(&mut host, |event| matches!(event, HostEvent::TurnedAway { .. }));
        turned_away += 1;
    }
}

#[test]
fn a_host_that_asks_for_a_great_deal_of_work_is_not_obeyed() {
    //a host by hand that wants the password worked on for a million rounds
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().expect("a runtime");
    let (port, listener) = runtime.block_on(async {
        let endpoint = Endpoint::server(server_config().unwrap(), SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).unwrap();
        let port = endpoint.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            let connection = endpoint.accept().await.unwrap().await.unwrap();
            let (mut send, mut receive) = connection.accept_bi().await.unwrap();
            let _hello = read_frame(&mut receive).await.unwrap();
            let ask = ServerMsg::Challenge { salt: [1; SALT_BYTES], nonce: [2; NONCE_BYTES], iterations: MAX_KDF_ITERATIONS + 1 };
            write_frame(&mut send, &ask.to_bytes()).await.unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;
            drop(endpoint);
        });
        (port, task)
    });
    let mut client = NetClient::connect(&format!("127.0.0.1:{port}"), "Ann", PASSWORD);
    let ClientEvent::Failed(problem) = await_client(&mut client, |event| matches!(event, ClientEvent::Failed(_) | ClientEvent::Joined { .. })) else { panic!("it went along with it") };
    assert!(problem.contains("work"), "{problem}");
    runtime.block_on(async { listener.abort() });
}

// ---------- when there is no host ----------

#[test]
fn trying_to_join_a_game_that_is_not_there_says_so() {
    //(a port nothing is listening on; the answer comes when the try is given up)
    let port = {
        let socket = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        socket.local_addr().unwrap().port()
    };
    let mut client = NetClient::connect(&format!("127.0.0.1:{port}"), "Ann", PASSWORD);
    let ClientEvent::Failed(problem) = await_client(&mut client, |event| matches!(event, ClientEvent::Failed(_) | ClientEvent::Joined { .. })) else { panic!("somehow joined") };
    assert!(problem.contains("no answer") || problem.contains("could not connect"), "{problem}");
}

#[test]
fn an_address_that_is_not_one_fails_at_once() {
    let mut client = NetClient::connect("not an address", "Ann", PASSWORD);
    let ClientEvent::Failed(problem) = await_client(&mut client, |_| true) else { panic!("not a failure") };
    assert!(problem.contains("spaces"), "{problem}");
    let mut client = NetClient::connect("no-such-host.invalid", "Ann", PASSWORD);
    let ClientEvent::Failed(problem) = await_client(&mut client, |_| true) else { panic!("not a failure") };
    assert!(problem.contains("could not find"), "{problem}");
}

#[test]
fn a_player_who_gives_up_while_connecting_stops_trying() {
    let port = {
        let socket = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        socket.local_addr().unwrap().port()
    };
    let started = Instant::now();
    let mut client = NetClient::connect(&format!("127.0.0.1:{port}"), "Ann", PASSWORD);
    std::thread::sleep(Duration::from_millis(100));
    client.leave();
    drop(client);
    assert!(started.elapsed() < Duration::from_secs(2), "leaving did not wait for the try to end: {:?}", started.elapsed());
}

#[test]
fn a_player_who_has_gone_quiet_is_replaced_by_one_who_joins_again_with_their_name() {
    let mut host = hosting("Hostess", PASSWORD);
    let port = host.port;
    //(a connection made on a runtime that is gone by the time the block ends says nothing more, which is what a cable pulled out does)
    let silent = runtime().block_on(async {
        let (raw, answer, _) = Raw::join(port, "Ann", PASSWORD, PROTOCOL, None).await;
        assert!(matches!(answer, ServerMsg::Welcome { slot: 1, .. }), "{answer:?}");
        raw
    });
    await_host(&mut host, |event| matches!(event, HostEvent::Joined { .. }));
    //while it is as good as any other the name is theirs
    assert_eq!(refused_with(&host, "Ann", PASSWORD), Refusal::NameTaken);
    //and once it has been quiet for a while it is theirs to take back
    std::thread::sleep(Duration::from_secs(STALE_SECONDS + ROSTER_SECONDS + 1));
    let mut again = join(&host, "ann", PASSWORD);
    assert!(matches!(await_client(&mut again, |event| matches!(event, ClientEvent::Joined { .. } | ClientEvent::Refused(_))), ClientEvent::Joined { .. }), "the same player, joining again");
    let HostEvent::Left { name, why, .. } = await_host(&mut host, |event| matches!(event, HostEvent::Left { .. })) else { unreachable!() };
    assert_eq!(name, "Ann");
    assert!(why.contains("again"), "{why}");
    let HostEvent::Joined { slot, .. } = await_host(&mut host, |event| matches!(event, HostEvent::Joined { .. })) else { unreachable!() };
    assert_eq!(slot, 1);
    assert_eq!(names(&host.members()), ["Hostess", "ann"]);
    //(the old connection being found to have gone, afterwards, takes no one's place from the new one)
    drop(silent);
    std::thread::sleep(Duration::from_millis(300));
    assert!(host.poll().iter().all(|event| !matches!(event, HostEvent::Left { .. })), "the new connection is still the player's");
    assert_eq!(names(&host.members()), ["Hostess", "ann"]);
}

#[test]
fn a_player_who_is_in_the_game_and_heard_from_keeps_their_name_from_anyone_else() {
    let mut host = hosting("Hostess", PASSWORD);
    let _ann = joined(&host, "Ann");
    await_host(&mut host, |event| matches!(event, HostEvent::Joined { .. }));
    std::thread::sleep(Duration::from_secs(STALE_SECONDS + ROSTER_SECONDS + 1));
    assert_eq!(refused_with(&host, "Ann", PASSWORD), Refusal::NameTaken, "a connection that answers is not a quiet one, however long it has been in");
    assert_eq!(names(&host.members()), ["Hostess", "Ann"]);
}
