//! joining a game without a socket: what is said, how a password is proved without being sent, and how many tries someone gets

use super::*;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

const SALT: [u8; SALT_BYTES] = [7; SALT_BYTES];
const NONCE: [u8; NONCE_BYTES] = [9; NONCE_BYTES];
const BINDING: [u8; BINDING_BYTES] = [3; BINDING_BYTES];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn address(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(203, 0, 113, last))
}

// ---------- the primitives are the ones they are meant to be ----------

#[test]
fn the_password_hash_is_pbkdf2_with_hmac_sha256() {
    //(the published test vectors, so a library that changed what it does would be caught)
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(b"password", b"salt", 1, &mut key);
    assert_eq!(hex(&key), "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b");
    pbkdf2::pbkdf2_hmac::<Sha256>(b"password", b"salt", 4096, &mut key);
    assert_eq!(hex(&key), "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a");
}

#[test]
fn the_proof_is_an_hmac_sha256() {
    //(RFC 4231, test case 1)
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(&[0x0b; 20]).unwrap();
    mac.update(b"Hi There");
    assert_eq!(hex(mac.finalize().into_bytes().as_slice()), "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");
}

#[test]
fn a_key_comes_from_the_password_the_salt_and_how_hard_it_was_asked_to_work() {
    let key = derive_key("hunter22", &SALT, 1000);
    assert_eq!(key, derive_key("hunter22", &SALT, 1000), "the same every time");
    assert_ne!(key, derive_key("hunter23", &SALT, 1000), "another password");
    assert_ne!(key, derive_key("Hunter22", &SALT, 1000), "capitals count");
    assert_ne!(key, derive_key("hunter22", &[8; SALT_BYTES], 1000), "another salt");
    assert_ne!(key, derive_key("hunter22", &SALT, 1001), "another amount of work");
    //and it is what the library makes of the same, so the host and the player (who both call this) can't disagree about how
    let mut expected = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(b"hunter22", &SALT, 1000, &mut expected);
    assert_eq!(key, expected);
}

#[test]
fn random_bytes_are_not_the_same_twice() {
    let first: [u8; 32] = random_bytes();
    let second: [u8; 32] = random_bytes();
    assert_ne!(first, second);
    assert_ne!(first, [0; 32], "not all zeros");
}

// ---------- proving the password ----------

#[test]
fn the_right_password_is_proved_and_nothing_else_is() {
    let key = derive_key("open sesame", &SALT, 1000);
    let proof = make_proof(&key, &NONCE, &BINDING, "Ann");
    assert!(check_proof(&key, &NONCE, &BINDING, "Ann", &proof));
    //the wrong password
    let wrong = derive_key("open sesamE", &SALT, 1000);
    assert!(!check_proof(&key, &NONCE, &BINDING, "Ann", &make_proof(&wrong, &NONCE, &BINDING, "Ann")));
    //a proof is for the question it answered: not another nonce (so one that was overheard can't be sent again)...
    assert!(!check_proof(&key, &[10; NONCE_BYTES], &BINDING, "Ann", &proof));
    //...nor another connection (so one that is passed on from a connection to the host can't be used on another)...
    assert!(!check_proof(&key, &NONCE, &[4; BINDING_BYTES], "Ann", &proof));
    //...nor another name (so a player can't be let in as someone else with the proof they made as themselves)
    assert!(!check_proof(&key, &NONCE, &BINDING, "Bob", &proof));
    assert!(!check_proof(&key, &NONCE, &BINDING, "ann", &proof));
}

#[test]
fn a_proof_with_any_one_bit_wrong_is_not_a_proof() {
    let key = derive_key("open sesame", &SALT, 1000);
    let proof = make_proof(&key, &NONCE, &BINDING, "Ann");
    for bit in 0..PROOF_BYTES * 8 {
        let mut bad = proof;
        bad[bit / 8] ^= 1 << (bit % 8);
        assert!(!check_proof(&key, &NONCE, &BINDING, "Ann", &bad), "bit {bit}");
    }
}

#[test]
fn a_proof_does_not_contain_the_password_or_the_key() {
    let key = derive_key("open sesame", &SALT, 1000);
    let proof = make_proof(&key, &NONCE, &BINDING, "Ann");
    assert_ne!(proof, key);
    assert!(!proof.windows(4).any(|part| part == b"open"));
}

#[test]
fn passwords_have_to_be_long_enough_and_not_too_long() {
    assert!(password_problem("").is_some());
    assert!(password_problem("abc").is_some());
    assert!(password_problem("abcd").is_none());
    assert!(password_problem(&"x".repeat(MAX_PASSWORD)).is_none());
    assert!(password_problem(&"x".repeat(MAX_PASSWORD + 1)).is_some());
    //(it is letters that count, not bytes)
    assert!(password_problem("ééé").is_some());
    assert!(password_problem("éééé").is_none());
}

// ---------- the messages ----------

fn members() -> Vec<Member> {
    vec![Member { slot: 0, name: "Hostess".to_owned(), ping_ms: 0 }, Member { slot: 2, name: "Ann Marie".to_owned(), ping_ms: 48 }]
}

fn every_refusal() -> Vec<Refusal> {
    vec![
        Refusal::WrongVersion { host_protocol: 4, host_version: "9.9.9".to_owned() },
        Refusal::WrongPassword,
        Refusal::NameTaken,
        Refusal::BadName,
        Refusal::Full,
        Refusal::TooManyTries { wait_seconds: 40 },
        Refusal::Busy,
        Refusal::Malformed,
        Refusal::Kicked,
        Refusal::HostClosed,
    ]
}

fn every_client_message() -> Vec<ClientMsg> {
    vec![ClientMsg::Hello { protocol: PROTOCOL, version: "0.1.3".to_owned(), name: "Ann".to_owned() }, ClientMsg::Proof([5; PROOF_BYTES]), ClientMsg::Leave]
}

fn every_server_message() -> Vec<ServerMsg> {
    let mut all = vec![ServerMsg::Challenge { salt: SALT, nonce: NONCE, iterations: KDF_ITERATIONS }, ServerMsg::Welcome { slot: 2, members: members() }, ServerMsg::Roster(members()), ServerMsg::Roster(Vec::new()), ServerMsg::Start];
    all.extend(every_refusal().into_iter().map(ServerMsg::Refused));
    all
}

#[test]
fn every_message_comes_back_as_it_was_sent() {
    for message in every_client_message() {
        assert_eq!(ClientMsg::from_bytes(&message.to_bytes()), Some(message.clone()), "{message:?}");
    }
    for message in every_server_message() {
        assert_eq!(ServerMsg::from_bytes(&message.to_bytes()), Some(message.clone()), "{message:?}");
    }
}

#[test]
fn a_message_that_is_cut_short_or_has_more_on_the_end_is_not_a_message() {
    for message in every_client_message() {
        let bytes = message.to_bytes();
        for length in 0..bytes.len() {
            assert_eq!(ClientMsg::from_bytes(&bytes[..length]), None, "{message:?} cut to {length}");
        }
        let mut longer = bytes.clone();
        longer.push(0);
        assert_eq!(ClientMsg::from_bytes(&longer), None, "{message:?} with a byte more");
    }
    for message in every_server_message() {
        let bytes = message.to_bytes();
        for length in 0..bytes.len() {
            assert_eq!(ServerMsg::from_bytes(&bytes[..length]), None, "{message:?} cut to {length}");
        }
        let mut longer = bytes.clone();
        longer.push(0);
        assert_eq!(ServerMsg::from_bytes(&longer), None, "{message:?} with a byte more");
    }
}

#[test]
fn messages_that_say_nonsense_are_not_messages() {
    //(a kind of message there is none of)
    assert_eq!(ClientMsg::from_bytes(&[0]), None);
    assert_eq!(ClientMsg::from_bytes(&[99, 1, 2, 3]), None);
    assert_eq!(ServerMsg::from_bytes(&[0]), None);
    assert_eq!(ServerMsg::from_bytes(&[99]), None);
    assert_eq!(ServerMsg::from_bytes(&[3, 99]), None, "a refusal of no known kind");
    //(a hello from something that is not this game)
    let mut hello = ClientMsg::Hello { protocol: PROTOCOL, version: "1".to_owned(), name: "Ann".to_owned() }.to_bytes();
    hello[1] ^= 0xff;
    assert_eq!(ClientMsg::from_bytes(&hello), None);
    //(a version no version is as long as)
    let long = ClientMsg::Hello { protocol: PROTOCOL, version: "9".repeat(MAX_VERSION_BYTES + 1), name: "Ann".to_owned() };
    assert_eq!(ClientMsg::from_bytes(&long.to_bytes()), None);
    let long = ServerMsg::Refused(Refusal::WrongVersion { host_protocol: 2, host_version: "9".repeat(MAX_VERSION_BYTES + 1) });
    assert_eq!(ServerMsg::from_bytes(&long.to_bytes()), None);
    //(more players than a game has, and a player in a place it hasn't)
    let mut crowd = Writer::new();
    crowd.u8(4);
    crowd.u8(MAX_PLAYERS as u8 + 1);
    for _ in 0..=MAX_PLAYERS {
        crowd.u8(0);
        crowd.text("x");
        crowd.u16(0);
    }
    assert_eq!(ServerMsg::from_bytes(&crowd.bytes), None);
    let misplaced = ServerMsg::Roster(vec![Member { slot: MAX_PLAYERS as u8, name: "x".to_owned(), ping_ms: 0 }]);
    assert_eq!(ServerMsg::from_bytes(&misplaced.to_bytes()), None);
    assert_eq!(ServerMsg::from_bytes(&ServerMsg::Welcome { slot: MAX_PLAYERS as u8, members: Vec::new() }.to_bytes()), None);
}

#[test]
fn bytes_of_any_kind_are_never_a_panic() {
    //(noise of every length that is cheap to try, and every first byte)
    let mut seed = 0x1234_5678_u32;
    for length in 0..300 {
        let noise: Vec<u8> = (0..length)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed >> 24) as u8
            })
            .collect();
        let _ = ClientMsg::from_bytes(&noise);
        let _ = ServerMsg::from_bytes(&noise);
    }
    for first in 0..=255u8 {
        let _ = ClientMsg::from_bytes(&[first; 40]);
        let _ = ServerMsg::from_bytes(&[first; 40]);
    }
}

#[test]
fn a_frame_says_how_long_it_is() {
    let message = ServerMsg::Start.to_bytes();
    let framed = frame(&message);
    assert_eq!(framed.len(), message.len() + 2);
    assert_eq!(frame_length([framed[0], framed[1]]), Some(message.len()));
    assert_eq!(&framed[2..], message.as_slice());
    //nothing is said of a length of none, or one that is more than a message is allowed to be
    assert_eq!(frame_length([0, 0]), None);
    assert_eq!(frame_length((MAX_FRAME as u16).to_le_bytes()), Some(MAX_FRAME));
    assert_eq!(frame_length((MAX_FRAME as u16 + 1).to_le_bytes()), None);
    assert_eq!(frame_length([0xff, 0xff]), None);
}

#[test]
#[should_panic(expected = "does not fit in a frame")]
fn a_message_too_big_for_a_frame_is_a_mistake_in_the_game_not_something_to_send() {
    frame(&vec![0; MAX_FRAME + 1]);
}

#[test]
fn a_refusal_says_why_in_words() {
    for refusal in every_refusal() {
        assert!(refusal.text().len() > 8, "{refusal:?}");
    }
    assert!(Refusal::TooManyTries { wait_seconds: 40 }.text().contains("40"));
    assert!(Refusal::WrongVersion { host_protocol: 4, host_version: "9.9.9".to_owned() }.text().contains("9.9.9"));
}

// ---------- how many tries ----------

#[test]
fn someone_can_try_a_dozen_times_a_minute_and_no_more() {
    let mut guard = Guard::new();
    for i in 0..ATTEMPTS_A_MINUTE {
        assert_eq!(guard.admit(address(1), i as f64), Admission::Go, "try {i}");
    }
    assert_eq!(guard.admit(address(1), 12.0), Admission::Flood);
    //(other people are not held up by it)
    assert_eq!(guard.admit(address(2), 12.0), Admission::Go);
    //and a minute after the first, they can try again
    assert_eq!(guard.admit(address(1), 60.5), Admission::Go);
}

#[test]
fn a_wrong_password_a_few_times_is_forgiven_and_then_the_wait_doubles() {
    let mut guard = Guard::new();
    let mut now = 0.0;
    for _ in 0..FREE_FAILURES {
        assert_eq!(guard.admit(address(1), now), Admission::Go);
        guard.failed(address(1), now);
        now += 1.0;
    }
    assert_eq!(guard.admit(address(1), now), Admission::Go, "still free after {FREE_FAILURES}");
    guard.failed(address(1), now);
    //the next one waits
    assert_eq!(guard.admit(address(1), now + 1.0), Admission::Locked(LOCKOUT_SECONDS as u16 - 1));
    assert_eq!(guard.admit(address(1), now + LOCKOUT_SECONDS - 0.5), Admission::Locked(1));
    //then is let try again, and if they are wrong again the wait is twice as long
    now += LOCKOUT_SECONDS + 1.0;
    assert_eq!(guard.admit(address(1), now), Admission::Go);
    guard.failed(address(1), now);
    assert_eq!(guard.admit(address(1), now + 1.0), Admission::Locked(LOCKOUT_SECONDS as u16 * 2 - 1));
    //and it doesn't grow without end
    for _ in 0..30 {
        guard.failed(address(1), now);
    }
    let Admission::Locked(wait) = guard.admit(address(1), now + 1.0) else { panic!("locked out") };
    assert!(wait <= MAX_LOCKOUT_SECONDS as u16);
    assert!(wait as f64 >= MAX_LOCKOUT_SECONDS - 2.0, "it reached the longest: {wait}");
}

#[test]
fn getting_in_clears_what_was_wrong_before() {
    let mut guard = Guard::new();
    for i in 0..FREE_FAILURES + 1 {
        guard.admit(address(1), i as f64 * 5.0);
        guard.failed(address(1), i as f64 * 5.0);
    }
    assert!(matches!(guard.admit(address(1), 30.0), Admission::Locked(_)));
    guard.passed(address(1));
    assert_eq!(guard.admit(address(1), 31.0), Admission::Go);
    //(and they have their free tries again)
    for _ in 0..FREE_FAILURES {
        guard.failed(address(1), 32.0);
    }
    assert_eq!(guard.admit(address(1), 33.0), Admission::Go);
}

#[test]
fn what_one_place_does_wrong_is_not_held_against_another() {
    let mut guard = Guard::new();
    for i in 0..FREE_FAILURES + 3 {
        guard.admit(address(1), i as f64);
        guard.failed(address(1), i as f64);
    }
    assert!(matches!(guard.admit(address(1), 10.0), Admission::Locked(_)));
    assert_eq!(guard.admit(address(2), 10.0), Admission::Go);
}

#[test]
fn everyone_together_can_only_try_so_often() {
    let mut guard = Guard::new();
    for i in 0..ATTEMPTS_ALL_TOGETHER {
        assert_eq!(guard.admit(IpAddr::V4(Ipv4Addr::new(10, 0, (i / 250) as u8, (i % 250) as u8)), i as f64 * 0.1), Admission::Go, "try {i}");
    }
    assert_eq!(guard.admit(address(200), 6.5), Admission::Flood, "a flood from all over");
    assert_eq!(guard.admit(address(200), 17.0), Admission::Go, "and it is over ten seconds after");
}

#[test]
fn one_connection_is_one_place_however_many_addresses_it_has() {
    let in_one_home = |last: u16| IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 1, 2, 0, 0, 0, last));
    let next_door = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 1, 3, 0, 0, 0, 1));
    let mut guard = Guard::new();
    for i in 0..ATTEMPTS_A_MINUTE {
        assert_eq!(guard.admit(in_one_home(i as u16 + 1), i as f64), Admission::Go);
    }
    assert_eq!(guard.admit(in_one_home(999), 13.0), Admission::Flood, "a new address in the same home is the same place");
    assert_eq!(guard.admit(next_door, 13.0), Admission::Go);
}

#[test]
fn what_is_remembered_of_places_never_grows_past_a_limit() {
    let mut guard = Guard::new();
    let mut now = 0.0;
    for i in 0..(GUARD_MEMORY * 3) as u32 {
        now += 0.2; //(slowly enough to never be a flood)
        let place = IpAddr::V4(Ipv4Addr::from(0x0a00_0000 + i));
        assert_eq!(guard.admit(place, now), Admission::Go);
        guard.failed(place, now);
        assert!(guard.remembered() <= GUARD_MEMORY, "after {i}: {}", guard.remembered());
    }
}

#[test]
fn a_place_that_is_waiting_is_not_forgotten_to_make_room() {
    let mut guard = Guard::new();
    guard.admit(address(1), 0.0);
    for _ in 0..FREE_FAILURES + 12 {
        guard.failed(address(1), 0.0);
    }
    //many others come and go, enough to fill the memory, over the time it takes the wait to be half over
    for i in 0..(GUARD_MEMORY + 50) as u32 {
        guard.admit(IpAddr::V4(Ipv4Addr::from(0x0b00_0000 + i)), 1.0 + i as f64 * 0.2);
    }
    assert!(matches!(guard.admit(address(1), 430.0), Admission::Locked(_)), "still waiting");
}
