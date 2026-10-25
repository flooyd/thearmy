//! joining a game: the messages that cross the first stream of a connection, how a password is proved without being sent, and how many tries
//! someone gets. there are no sockets here, so all of it can be tested with nothing but numbers

use crate::prelude::*;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::collections::VecDeque;
use std::net::IpAddr;

//changes whenever what is sent between two copies of the game changes, so a host and a player that are not the same can tell
pub(crate) const PROTOCOL: u16 = 3;
pub(crate) const MAGIC: [u8; 4] = *b"ARMY";
pub(crate) const MAX_FRAME: usize = 8192; //the most bytes in one message on the first stream
pub(crate) const MAX_VERSION_BYTES: usize = 32;
pub(crate) const SALT_BYTES: usize = 16;
pub(crate) const NONCE_BYTES: usize = 32;
pub(crate) const BINDING_BYTES: usize = 32;
pub(crate) const PROOF_BYTES: usize = 32;
pub(crate) const KDF_ITERATIONS: u32 = 60_000; //what a host asks of a password: a few milliseconds each for whoever joins, a lot for whoever guesses
pub(crate) const MAX_KDF_ITERATIONS: u32 = 400_000; //a player that is asked for more than this refuses, so a host can't make them work for ever
pub(crate) const MIN_PASSWORD: usize = 4;
pub(crate) const MAX_PASSWORD: usize = 64;
pub(crate) const BINDING_LABEL: &[u8] = b"EXPORTER the-army join";
const PROOF_CONTEXT: &[u8] = b"the-army join 1";

//someone in the game, as the others are told of them
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Member {
    pub(crate) slot: u8, //their place in the list of players
    pub(crate) name: String,
    pub(crate) ping_ms: u16, //how long a message takes to get to them and back
}

//why someone was not let in, or was put out
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Refusal {
    WrongVersion { host_protocol: u16, host_version: String },
    WrongPassword,
    NameTaken,
    BadName,
    Full,
    TooManyTries { wait_seconds: u16 },
    Busy,
    Malformed,
    Kicked,
    HostClosed,
}

impl Refusal {
    pub(crate) fn text(&self) -> String {
        match self {
            Refusal::WrongVersion { host_version, .. } => format!("the host is playing a different version of the game ({host_version}, you have {})", env!("CARGO_PKG_VERSION")),
            Refusal::WrongPassword => "that is not the password".to_owned(),
            Refusal::NameTaken => "someone in the game has that name already".to_owned(),
            Refusal::BadName => "that name can't be used".to_owned(),
            Refusal::Full => "the game is full".to_owned(),
            Refusal::TooManyTries { wait_seconds } => format!("too many tries: wait {wait_seconds} seconds"),
            Refusal::Busy => "the host is busy, try again in a moment".to_owned(),
            Refusal::Malformed => "the host could not make sense of what was sent".to_owned(),
            Refusal::Kicked => "the host put you out of the game".to_owned(),
            Refusal::HostClosed => "the host has closed the game".to_owned(),
        }
    }

    fn write(&self, out: &mut Writer) {
        match self {
            Refusal::WrongVersion { host_protocol, host_version } => {
                out.u8(1);
                out.u16(*host_protocol);
                out.text(host_version);
            }
            Refusal::WrongPassword => out.u8(2),
            Refusal::NameTaken => out.u8(3),
            Refusal::BadName => out.u8(4),
            Refusal::Full => out.u8(5),
            Refusal::TooManyTries { wait_seconds } => {
                out.u8(6);
                out.u16(*wait_seconds);
            }
            Refusal::Busy => out.u8(7),
            Refusal::Malformed => out.u8(8),
            Refusal::Kicked => out.u8(9),
            Refusal::HostClosed => out.u8(10),
        }
    }

    fn read(input: &mut Reader) -> Option<Refusal> {
        Some(match input.u8()? {
            1 => Refusal::WrongVersion { host_protocol: input.u16()?, host_version: input.text().filter(|version| version.len() <= MAX_VERSION_BYTES)? },
            2 => Refusal::WrongPassword,
            3 => Refusal::NameTaken,
            4 => Refusal::BadName,
            5 => Refusal::Full,
            6 => Refusal::TooManyTries { wait_seconds: input.u16()? },
            7 => Refusal::Busy,
            8 => Refusal::Malformed,
            9 => Refusal::Kicked,
            10 => Refusal::HostClosed,
            _ => return None,
        })
    }
}

fn write_members(out: &mut Writer, members: &[Member]) {
    let members = &members[..members.len().min(MAX_PLAYERS)];
    out.u8(members.len() as u8);
    for member in members {
        out.u8(member.slot);
        out.text(&member.name);
        out.u16(member.ping_ms);
    }
}

fn read_members(input: &mut Reader) -> Option<Vec<Member>> {
    let count = input.u8()? as usize;
    if count > MAX_PLAYERS {
        return None;
    }
    let mut members = Vec::with_capacity(count);
    for _ in 0..count {
        let slot = input.u8().filter(|&slot| (slot as usize) < MAX_PLAYERS)?;
        members.push(Member { slot, name: input.text()?, ping_ms: input.u16()? });
    }
    Some(members)
}

//what a player says to the host, on the first stream of a connection
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ClientMsg {
    Hello { protocol: u16, version: String, name: String }, //who they are and which game they have
    Proof([u8; PROOF_BYTES]),                               //that they know the password
    Leave,
}

impl ClientMsg {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut out = Writer::new();
        match self {
            ClientMsg::Hello { protocol, version, name } => {
                out.u8(1);
                out.raw(&MAGIC);
                out.u16(*protocol);
                out.text(version);
                out.text(name);
            }
            ClientMsg::Proof(proof) => {
                out.u8(2);
                out.raw(proof);
            }
            ClientMsg::Leave => out.u8(3),
        }
        out.bytes
    }

    //the message in some bytes, or None if they are cut short, garbled, or have something left over after it
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<ClientMsg> {
        let mut input = Reader::new(bytes);
        let message = match input.u8()? {
            1 => {
                if input.array::<4>()? != MAGIC {
                    return None;
                }
                ClientMsg::Hello { protocol: input.u16()?, version: input.text().filter(|version| version.len() <= MAX_VERSION_BYTES)?, name: input.text()? }
            }
            2 => ClientMsg::Proof(input.array()?),
            3 => ClientMsg::Leave,
            _ => return None,
        };
        input.is_done().then_some(message)
    }
}

//what the host says to a player
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ServerMsg {
    Challenge { salt: [u8; SALT_BYTES], nonce: [u8; NONCE_BYTES], iterations: u32 }, //prove you know the password
    Welcome { slot: u8, members: Vec<Member> },                                     //you are in, as this player, with these
    Refused(Refusal),
    Roster(Vec<Member>), //who is in the game now
    Start,               //the host has started the run
}

impl ServerMsg {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut out = Writer::new();
        match self {
            ServerMsg::Challenge { salt, nonce, iterations } => {
                out.u8(1);
                out.raw(salt);
                out.raw(nonce);
                out.u32(*iterations);
            }
            ServerMsg::Welcome { slot, members } => {
                out.u8(2);
                out.u8(*slot);
                write_members(&mut out, members);
            }
            ServerMsg::Refused(refusal) => {
                out.u8(3);
                refusal.write(&mut out);
            }
            ServerMsg::Roster(members) => {
                out.u8(4);
                write_members(&mut out, members);
            }
            ServerMsg::Start => out.u8(5),
        }
        out.bytes
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<ServerMsg> {
        let mut input = Reader::new(bytes);
        let message = match input.u8()? {
            1 => ServerMsg::Challenge { salt: input.array()?, nonce: input.array()?, iterations: input.u32()? },
            2 => ServerMsg::Welcome { slot: input.u8().filter(|&slot| (slot as usize) < MAX_PLAYERS)?, members: read_members(&mut input)? },
            3 => ServerMsg::Refused(Refusal::read(&mut input)?),
            4 => ServerMsg::Roster(read_members(&mut input)?),
            5 => ServerMsg::Start,
            _ => return None,
        };
        input.is_done().then_some(message)
    }
}

//a message ready to go on a stream: how long it is, then what it says
pub(crate) fn frame(message: &[u8]) -> Vec<u8> {
    assert!(message.len() <= MAX_FRAME, "a message of {} bytes does not fit in a frame", message.len());
    let mut framed = Vec::with_capacity(message.len() + 2);
    framed.extend_from_slice(&(message.len() as u16).to_le_bytes());
    framed.extend_from_slice(message);
    framed
}

//how long the message that follows is, if that is something that can be believed
pub(crate) fn frame_length(header: [u8; 2]) -> Option<usize> {
    let length = u16::from_le_bytes(header) as usize;
    (length > 0 && length <= MAX_FRAME).then_some(length)
}

// ---------- the password ----------

//a password as it is kept: not empty or short, and not long enough to be used to make the host work
pub(crate) fn password_problem(password: &str) -> Option<String> {
    let length = password.chars().count();
    if length < MIN_PASSWORD {
        Some(format!("the password needs at least {MIN_PASSWORD} characters"))
    } else if length > MAX_PASSWORD {
        Some(format!("the password can't have more than {MAX_PASSWORD} characters"))
    } else {
        None
    }
}

pub(crate) fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).expect("the system can give random numbers");
    bytes
}

//what a password is turned into, which is slow on purpose: both ends work it out from the host's salt, so what the host is sent says
//nothing of the password even to someone who has all of it and time to try every guess
pub(crate) fn derive_key(password: &str, salt: &[u8; SALT_BYTES], iterations: u32) -> [u8; 32] {
    let mut key = [0; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);
    key
}

fn proof_of(key: &[u8; 32], nonce: &[u8; NONCE_BYTES], binding: &[u8; BINDING_BYTES], name: &str) -> Hmac<Sha256> {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key).expect("a key of any length will do");
    mac.update(PROOF_CONTEXT);
    mac.update(nonce);
    mac.update(binding);
    mac.update(name.as_bytes());
    mac
}

//the proof that a player knows the password: what the host asked with (`nonce`), tied to the encrypted connection it is sent over (`binding`, which
//no one who is only passing messages along from another connection can have) and to the name asked for
pub(crate) fn make_proof(key: &[u8; 32], nonce: &[u8; NONCE_BYTES], binding: &[u8; BINDING_BYTES], name: &str) -> [u8; PROOF_BYTES] {
    proof_of(key, nonce, binding, name).finalize().into_bytes().into()
}

//whether a proof is the right one, which takes as long if it is nearly right as if it is not
pub(crate) fn check_proof(key: &[u8; 32], nonce: &[u8; NONCE_BYTES], binding: &[u8; BINDING_BYTES], name: &str, proof: &[u8; PROOF_BYTES]) -> bool {
    proof_of(key, nonce, binding, name).verify_slice(proof).is_ok()
}

// ---------- how many tries ----------

pub(crate) const ATTEMPTS_A_MINUTE: usize = 12; //connections from one place in a minute
pub(crate) const ATTEMPTS_ALL_TOGETHER: usize = 60; //connections from everywhere in ten seconds
pub(crate) const FREE_FAILURES: u32 = 4; //wrong passwords from one place before it has to wait
pub(crate) const LOCKOUT_SECONDS: f64 = 20.0; //how long it waits after the next, doubling for each one after that
pub(crate) const MAX_LOCKOUT_SECONDS: f64 = 900.0;
pub(crate) const GUARD_MEMORY: usize = 2048; //places remembered at once

//what is done with someone who is trying to get in
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Admission {
    Go,           //let them try
    Locked(u16),  //told to wait this many seconds, and not let try
    Flood,        //too many: not answered at all
}

#[derive(Default)]
struct Record {
    attempts: VecDeque<f64>, //when they connected, in the last minute
    failures: u32,           //wrong passwords since they last got in
    locked_until: f64,
    last: f64,
}

//who has tried to get in, and how often, so that guessing the password takes more patience than anyone has. times are seconds from any start
#[derive(Default)]
pub(crate) struct Guard {
    places: HashMap<IpAddr, Record>,
    together: VecDeque<f64>,
}

//people on the same network address count as one (the 64 bits at the front of an IPv6 address are what one connection is given)
fn place_of(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V4(_) => address,
        IpAddr::V6(v6) => {
            let mut octets = v6.octets();
            octets[8..].fill(0);
            IpAddr::V6(octets.into())
        }
    }
}

impl Guard {
    pub(crate) fn new() -> Guard {
        Guard::default()
    }

    //someone is connecting: whether they are let try
    pub(crate) fn admit(&mut self, address: IpAddr, now: f64) -> Admission {
        self.together.retain(|&at| at > now - 10.0);
        if self.together.len() >= ATTEMPTS_ALL_TOGETHER {
            return Admission::Flood;
        }
        self.together.push_back(now);
        if self.places.len() >= GUARD_MEMORY && !self.places.contains_key(&place_of(address)) {
            self.forget_old(now);
        }
        let record = self.places.entry(place_of(address)).or_default();
        record.attempts.retain(|&at| at > now - 60.0);
        record.last = now;
        if record.attempts.len() >= ATTEMPTS_A_MINUTE {
            return Admission::Flood;
        }
        record.attempts.push_back(now);
        if record.locked_until > now {
            return Admission::Locked((record.locked_until - now).ceil().min(u16::MAX as f64) as u16);
        }
        Admission::Go
    }

    //they gave the wrong password. after a few of those they wait, longer each time
    pub(crate) fn failed(&mut self, address: IpAddr, now: f64) {
        let record = self.places.entry(place_of(address)).or_default();
        record.failures += 1;
        record.last = now;
        if record.failures > FREE_FAILURES {
            let doublings = (record.failures - FREE_FAILURES - 1).min(10);
            record.locked_until = now + (LOCKOUT_SECONDS * 2f64.powi(doublings as i32)).min(MAX_LOCKOUT_SECONDS);
        }
    }

    //they got in: what they got wrong before is forgotten
    pub(crate) fn passed(&mut self, address: IpAddr) {
        if let Some(record) = self.places.get_mut(&place_of(address)) {
            record.failures = 0;
            record.locked_until = 0.0;
        }
    }

    //places not heard of for ten minutes (and not waiting) are forgotten, and if that is not enough all of them are, so the list can't grow without end
    fn forget_old(&mut self, now: f64) {
        self.places.retain(|_, record| record.last > now - 600.0 || record.locked_until > now);
        //(too many to tell the old from the new: only the ones that are waiting are kept, and if there are too many of those, none)
        if self.places.len() >= GUARD_MEMORY {
            self.places.retain(|_, record| record.locked_until > now);
        }
        if self.places.len() >= GUARD_MEMORY {
            self.places.clear();
        }
    }

    #[cfg(test)]
    pub(crate) fn remembered(&self) -> usize {
        self.places.len()
    }
}
