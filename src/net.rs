//! the connection between a host and the players who join it: QUIC, which is encrypted and goes over one UDP port, on threads of its own, so the
//! frame loop only ever sends commands and reads events and never waits on the network. the first stream of a connection is how a player joins (see
//! `handshake`) and carries what the host tells everyone about the game

use crate::prelude::*;
use quinn::{ClientConfig, Connection, Endpoint, Incoming, RecvStream, SendStream, ServerConfig, TransportConfig};
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use rustls::crypto::{CryptoProvider, ring::default_provider};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

pub(crate) const DEFAULT_PORT: u16 = 47821;
const ALPN: &[u8] = b"the-army/1";
pub(crate) const SERVER_NAME: &str = "the-army";
pub(crate) const HANDSHAKE_SECONDS: u64 = if cfg!(test) { 3 } else { 8 }; //a connection that has not joined by then is dropped (the tests don't wait as long)
const CONNECT_SECONDS: u64 = if cfg!(test) { 2 } else { 10 };
const IDLE_SECONDS: u64 = 10; //a connection that has been silent this long is gone
const KEEPALIVE_SECONDS: u64 = 4;
pub(crate) const ROSTER_SECONDS: u64 = 2; //how often everyone is told who is in the game and how far away they are
pub(crate) const STALE_SECONDS: u64 = if cfg!(test) { 3 } else { 6 }; //a player who has not been heard from at all (not a keepalive, not an answer to the roster) this long is gone, and
                                                           //their name is for the one who joins again with it, who is them with another connection
const MAX_JOINING: usize = 8; //connections that have not joined yet, at once
const MAX_QUEUED_INPUTS: usize = 512; //inputs that have been heard from the players and not yet taken by the game, all together
const CLOSE_SECONDS: u64 = 1; //a stream that has been finished is given this long to be read before the connection is closed on it

pub(crate) type NetResult<T> = Result<T, String>;

// ---------- addresses ----------

//the address a player typed, as something that can be connected to: `host` and `host:port` and `1.2.3.4` and `[::1]:port` all work, and the port the game uses
//is the one if there is none
pub(crate) fn parse_target(text: &str) -> NetResult<String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("type the address of the game, like 203.0.113.7".to_owned());
    }
    if text.chars().any(|letter| letter.is_whitespace() || letter.is_control()) {
        return Err("an address has no spaces in it".to_owned());
    }
    let with_port = |host: &str, port: &str| -> NetResult<String> {
        let port: u16 = port.parse().ok().filter(|&port| port > 0).ok_or_else(|| format!("{port} is not a port"))?;
        if host.is_empty() {
            return Err("there is no address in front of the port".to_owned());
        }
        Ok(if host.contains(':') { format!("[{host}]:{port}") } else { format!("{host}:{port}") })
    };
    if let Some(rest) = text.strip_prefix('[') {
        //[::1] or [::1]:47821
        let (host, after) = rest.split_once(']').ok_or_else(|| "a [ without a ]".to_owned())?;
        return match after.strip_prefix(':') {
            Some(port) => with_port(host, port),
            None if after.is_empty() => with_port(host, &DEFAULT_PORT.to_string()),
            None => Err("there is something after the ]".to_owned()),
        };
    }
    match text.matches(':').count() {
        0 => with_port(text, &DEFAULT_PORT.to_string()),
        1 => {
            let (host, port) = text.split_once(':').expect("there is one colon");
            with_port(host, port)
        }
        //(an IPv6 address without the brackets has no port)
        _ => with_port(text, &DEFAULT_PORT.to_string()),
    }
}

//the port a host typed, from 1024 up, which are the ones a program may use without being told it can
pub(crate) fn parse_port(text: &str) -> NetResult<u16> {
    let port: u16 = text.trim().parse().map_err(|_| format!("{} is not a port: use a number from 1024 to 65535", text.trim()))?;
    if port < 1024 {
        return Err("ports under 1024 are kept for other programs: use 1024 to 65535".to_owned());
    }
    Ok(port)
}

//the address of this machine on its own network, which is what others on it join (not the one on the internet, which only the router knows). found by
//asking the system which way it would go to reach somewhere, without sending anything
pub(crate) fn local_address() -> Option<IpAddr> {
    let socket = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    let address = socket.local_addr().ok()?.ip();
    (!address.is_unspecified() && !address.is_loopback()).then_some(address)
}

// ---------- encryption ----------

fn provider() -> Arc<CryptoProvider> {
    Arc::new(default_provider())
}

//a certificate that the host made for itself: it is what the connection is encrypted with, but no one is vouching for it, so players don't ask
//who is (and the password, which is tied to the connection, is what shows the host that it is them, and them that it is the host)
pub(crate) fn server_config() -> NetResult<ServerConfig> {
    let certified = rcgen::generate_simple_self_signed(vec![SERVER_NAME.to_owned()]).map_err(|error| format!("could not make a certificate: {error}"))?;
    let chain = vec![CertificateDer::from(certified.cert.der().to_vec())];
    let key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let mut crypto = rustls::ServerConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| error.to_string())?
        .with_no_client_auth()
        .with_single_cert(chain, key.into())
        .map_err(|error| error.to_string())?;
    crypto.alpn_protocols = vec![ALPN.to_vec()];
    let quic = QuicServerConfig::try_from(crypto).map_err(|error| error.to_string())?;
    let mut config = ServerConfig::with_crypto(Arc::new(quic));
    config.transport_config(Arc::new(transport()));
    Ok(config)
}

//what one stream to talk on, one more each way to play over, no sending of datagrams, and a silence that ends a connection mean
fn transport() -> TransportConfig {
    let mut transport = TransportConfig::default();
    transport
        .max_concurrent_bidi_streams(1u32.into())
        .max_concurrent_uni_streams(1u32.into())
        .keep_alive_interval(Some(Duration::from_secs(KEEPALIVE_SECONDS)))
        .max_idle_timeout(Some(Duration::from_secs(IDLE_SECONDS).try_into().expect("a timeout that fits")));
    transport
}

//a verifier that takes whatever certificate the host has, as long as it really holds the key for it (a host has no certificate anyone has signed)
#[derive(Debug)]
struct AnyCertificate(Arc<CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for AnyCertificate {
    fn verify_server_cert(&self, _end_entity: &CertificateDer<'_>, _intermediates: &[CertificateDer<'_>], _server_name: &ServerName<'_>, _ocsp: &[u8], _now: UnixTime) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &rustls::DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &rustls::DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

pub(crate) fn client_config() -> NetResult<ClientConfig> {
    let provider = provider();
    let mut crypto = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| error.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AnyCertificate(provider)))
        .with_no_client_auth();
    crypto.alpn_protocols = vec![ALPN.to_vec()];
    let quic = QuicClientConfig::try_from(crypto).map_err(|error| error.to_string())?;
    let mut config = ClientConfig::new(Arc::new(quic));
    config.transport_config(Arc::new(transport()));
    Ok(config)
}

//what ties a password's proof to this connection and no other: both ends get the same from the encryption, and no one passing messages between two
//connections can have both
pub(crate) fn binding_of(connection: &Connection) -> NetResult<[u8; BINDING_BYTES]> {
    let mut binding = [0; BINDING_BYTES];
    connection.export_keying_material(&mut binding, BINDING_LABEL, b"").map_err(|_| "the connection could not be tied to the password".to_owned())?;
    Ok(binding)
}

// ---------- messages on a stream ----------

//(these and the configs above are not only for this file: the tests talk to a host, and stand in for one, by hand to try what a real player or host would never send)

pub(crate) async fn read_frame(stream: &mut RecvStream) -> NetResult<Vec<u8>> {
    let mut header = [0; 2];
    stream.read_exact(&mut header).await.map_err(|error| error.to_string())?;
    let length = frame_length(header).ok_or_else(|| "a message of a length that can't be".to_owned())?;
    let mut message = vec![0; length];
    stream.read_exact(&mut message).await.map_err(|error| error.to_string())?;
    Ok(message)
}

pub(crate) async fn write_frame(stream: &mut SendStream, message: &[u8]) -> NetResult<()> {
    stream.write_all(&frame(message)).await.map_err(|error| error.to_string())
}

//the last thing on a stream: it is finished and given a moment to be read before the connection goes
async fn say_goodbye(mut send: SendStream, connection: &Connection, why: &[u8]) {
    let _ = send.finish();
    let _ = tokio::time::timeout(Duration::from_secs(CLOSE_SECONDS), send.stopped()).await;
    connection.close(0u32.into(), why);
}

// ---------- the streams the game is played over ----------

//the messages on these are the world going one way and what the player asks for going the other, which are much bigger than a message to the host
//or a roster: how long it is (four bytes) and then what it says. a length past what is allowed is not a message, and is not believed enough to make room for
async fn read_big(stream: &mut RecvStream, most: usize) -> NetResult<Vec<u8>> {
    let mut header = [0; 4];
    stream.read_exact(&mut header).await.map_err(|error| error.to_string())?;
    let length = u32::from_le_bytes(header) as usize;
    if length == 0 || length > most {
        return Err("a message of a length that can't be".to_owned());
    }
    let mut message = Vec::with_capacity(length.min(1 << 16));
    let mut chunk = vec![0; 1 << 16];
    while message.len() < length {
        let want = (length - message.len()).min(chunk.len());
        stream.read_exact(&mut chunk[..want]).await.map_err(|error| error.to_string())?;
        message.extend_from_slice(&chunk[..want]);
    }
    Ok(message)
}

async fn write_big(stream: &mut SendStream, message: &[u8]) -> NetResult<()> {
    stream.write_all(&(message.len() as u32).to_le_bytes()).await.map_err(|error| error.to_string())?;
    stream.write_all(message).await.map_err(|error| error.to_string())
}

//a message on its way to the other end, with when it was handed over
struct Outgoing {
    at: Instant,
    bytes: Vec<u8>,
}

//for trying the game over a connection that is not as good as the one on this machine (THE_ARMY_NET_LAG_MS and THE_ARMY_NET_JITTER_MS, in milliseconds): every
//message the game is played over is held back by the lag and up to the jitter more, and they still arrive in order
#[derive(Clone, Copy)]
struct Shim {
    lag: Duration,
    jitter: Duration,
}

impl Shim {
    fn from_env() -> Option<Shim> {
        let read = |name: &str| std::env::var(name).ok().and_then(|value| value.trim().parse::<u64>().ok()).map(Duration::from_millis);
        let (lag, jitter) = (read("THE_ARMY_NET_LAG_MS").unwrap_or_default(), read("THE_ARMY_NET_JITTER_MS").unwrap_or_default());
        (!lag.is_zero() || !jitter.is_zero()).then_some(Shim { lag, jitter })
    }

    //when a message handed over at a time is let go, which is never before the one before it
    fn due(&self, handed_over: Instant, before: Instant) -> Instant {
        let [byte] = random_bytes::<1>();
        (handed_over + self.lag + self.jitter.mul_f32(byte as f32 / 255.0)).max(before)
    }
}

//the world going to a player: messages are written to the stream in order, held back by the shim if there is one, and counted off as they go
async fn write_world(connection: Connection, mut world: mpsc::UnboundedReceiver<Outgoing>, pending: Arc<AtomicUsize>, shim: Option<Shim>) -> NetResult<()> {
    let mut send = connection.open_uni().await.map_err(|error| error.to_string())?;
    let mut before = Instant::now();
    while let Some(Outgoing { at, bytes }) = world.recv().await {
        if let Some(shim) = shim {
            before = shim.due(at, before);
            tokio::time::sleep_until(before.into()).await;
        }
        let length = bytes.len();
        write_big(&mut send, &bytes).await?;
        pending.fetch_sub(length, Ordering::Relaxed);
    }
    let _ = send.finish();
    Ok(())
}

//what a player asks of the world, read from the stream they open for it and handed to the host's game. a player who asks faster than the game takes it in
//is not read from until it has (the connection then holds them back, and no more than so many ever wait)
async fn read_inputs(connection: Connection, slot: u8, inputs: std_mpsc::Sender<(u8, Vec<u8>)>, queued: Arc<AtomicUsize>) -> String {
    let mut receive = match connection.accept_uni().await {
        Ok(receive) => receive,
        Err(error) => return format!("the connection ended ({error})"),
    };
    loop {
        match read_big(&mut receive, MAX_UP_BYTES).await {
            Ok(message) => {
                while queued.load(Ordering::Relaxed) >= MAX_QUEUED_INPUTS {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                queued.fetch_add(1, Ordering::Relaxed);
                if inputs.send((slot, message)).is_err() {
                    return "the game was closed".to_owned();
                }
            }
            Err(error) => return format!("the connection ended ({error})"),
        }
    }
}

//what the player asks of the world, written to a stream they open for it (when they first have something to say)
async fn write_inputs(connection: Connection, mut inputs: mpsc::UnboundedReceiver<Outgoing>, shim: Option<Shim>) {
    let Ok(mut send) = connection.open_uni().await else { return };
    let mut before = Instant::now();
    while let Some(Outgoing { at, bytes }) = inputs.recv().await {
        if let Some(shim) = shim {
            before = shim.due(at, before);
            tokio::time::sleep_until(before.into()).await;
        }
        if write_big(&mut send, &bytes).await.is_err() {
            return;
        }
    }
    let _ = send.finish();
}

// ---------- the host ----------

pub(crate) struct HostConfig {
    pub(crate) name: String,     //the host's own, which is the first player
    pub(crate) password: String, //what a player has to know to join
    pub(crate) port: u16,        //0 for any that is free
    pub(crate) address: IpAddr,  //which of this machine's addresses to listen on: all of them, unless a test says otherwise
    pub(crate) version: String,  //the game's, which a player that is turned away for it is told
}

impl HostConfig {
    pub(crate) fn new(name: &str, password: &str, port: u16) -> HostConfig {
        HostConfig { name: clean_name(name), password: password.to_owned(), port, address: Ipv4Addr::UNSPECIFIED.into(), version: env!("CARGO_PKG_VERSION").to_owned() }
    }
}

//what the host's game hears of the network
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum HostEvent {
    Joined { slot: u8, name: String, address: SocketAddr },
    Left { slot: u8, name: String, why: String },
    TurnedAway { address: SocketAddr, why: String }, //someone who was not let in, and why
}

enum HostCommand {
    Start,
    Kick(u8),
    Shutdown,
}

//what the tasks that look after each connection tell the one that looks after the game
enum Hub {
    Join { name: String, address: SocketAddr, outbound: mpsc::UnboundedSender<ServerMsg>, pipe: Pipe, connection: Connection, reply: oneshot::Sender<Result<(u8, Vec<Member>), Refusal>> },
    Left { slot: u8, connection: usize, why: String }, //(which connection it was, as their place can be taken by another before they are heard to have gone)
    WrongPassword { address: SocketAddr },
    RightPassword { address: SocketAddr },
    TurnedAway { address: SocketAddr, refusal: Refusal },
}

//the way the host's game puts the world into a player's connection, and sees how much of it is still on its way
#[derive(Clone)]
struct Pipe {
    world: mpsc::UnboundedSender<Outgoing>,
    pending: Arc<AtomicUsize>, //bytes that have been handed over and not yet written
}

pub(crate) struct NetHost {
    events: std_mpsc::Receiver<HostEvent>,
    commands: mpsc::UnboundedSender<HostCommand>,
    members: Arc<Mutex<Vec<Member>>>,
    pipes: Arc<Mutex<Vec<Option<Pipe>>>>,
    inputs: std_mpsc::Receiver<(u8, Vec<u8>)>,
    queued: Arc<AtomicUsize>, //inputs that have been heard and not yet taken
    pub(crate) port: u16, //the one it is listening on
    pub(crate) directory: Option<Directory>, //the lobby list this game is on, if it is on one (see directory.rs)
    thread: Option<std::thread::JoinHandle<()>>,
}

impl NetHost {
    //start listening. fails, with something to tell the player, if the port is taken or the address can't be listened on
    pub(crate) fn start(config: HostConfig) -> NetResult<NetHost> {
        if let Some(problem) = password_problem(&config.password) {
            return Err(problem);
        }
        if config.name.is_empty() {
            return Err("the host needs a name".to_owned());
        }
        let salt: [u8; SALT_BYTES] = random_bytes();
        let (input_tx, inputs) = std_mpsc::channel();
        let queued = Arc::new(AtomicUsize::new(0));
        let ctx = Arc::new(HostContext { version: config.version.clone(), key: derive_key(&config.password, &salt, KDF_ITERATIONS), salt, iterations: KDF_ITERATIONS, inputs: input_tx, queued: queued.clone(), shim: Shim::from_env() });
        let members = Arc::new(Mutex::new(vec![Member { slot: 0, name: config.name.clone(), ping_ms: 0 }]));
        let pipes = Arc::new(Mutex::new((0..MAX_PLAYERS).map(|_| None).collect::<Vec<Option<Pipe>>>()));
        let (event_tx, events) = std_mpsc::channel();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (started_tx, started_rx) = std_mpsc::sync_channel::<NetResult<u16>>(1);
        let shared = members.clone();
        let shared_pipes = pipes.clone();
        let thread = std::thread::Builder::new()
            .name("net-host".to_owned())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = started_tx.send(Err(format!("could not start the network: {error}")));
                        return;
                    }
                };
                runtime.block_on(async move {
                    let endpoint = match server_config().and_then(|server| Endpoint::server(server, SocketAddr::new(config.address, config.port)).map_err(|error| listen_problem(&error, config.port))) {
                        Ok(endpoint) => endpoint,
                        Err(problem) => {
                            let _ = started_tx.send(Err(problem));
                            return;
                        }
                    };
                    let port = endpoint.local_addr().map(|address| address.port()).unwrap_or(config.port);
                    let _ = started_tx.send(Ok(port));
                    run_host(endpoint, ctx, config.name, shared, shared_pipes, command_rx, event_tx).await;
                });
                runtime.shutdown_timeout(Duration::from_millis(300));
            })
            .map_err(|error| format!("could not start the network: {error}"))?;
        match started_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(port)) => Ok(NetHost { events, commands: command_tx, members, pipes, inputs, queued, port, directory: None, thread: Some(thread) }),
            Ok(Err(problem)) => {
                let _ = thread.join();
                Err(problem)
            }
            Err(_) => Err("the network did not start".to_owned()),
        }
    }

    //what has happened since this was last asked
    pub(crate) fn poll(&self) -> Vec<HostEvent> {
        self.events.try_iter().collect()
    }

    //the oldest of them, leaving the rest to be asked for
    #[cfg(test)]
    pub(crate) fn next(&mut self) -> Option<HostEvent> {
        self.events.try_recv().ok()
    }

    //who is in the game now, the host first
    pub(crate) fn members(&self) -> Vec<Member> {
        self.members.lock().map(|members| members.clone()).unwrap_or_default()
    }

    //hand a player a message to be sent them (a message of the world, see `Viewer`). false if they are not there to send it to
    pub(crate) fn send_world(&self, slot: u8, message: Vec<u8>) -> bool {
        let Ok(pipes) = self.pipes.lock() else { return false };
        let Some(Some(pipe)) = pipes.get(slot as usize) else { return false };
        if message.is_empty() || message.len() > MAX_DOWN_BYTES {
            return false;
        }
        pipe.pending.fetch_add(message.len(), Ordering::Relaxed);
        pipe.world.send(Outgoing { at: Instant::now(), bytes: message }).is_ok()
    }

    //how many bytes of what a player was sent are not yet on their way (the host does not send more to a player who has this much waiting)
    pub(crate) fn waiting(&self, slot: u8) -> usize {
        let Ok(pipes) = self.pipes.lock() else { return 0 };
        pipes.get(slot as usize).and_then(|pipe| pipe.as_ref()).map_or(0, |pipe| pipe.pending.load(Ordering::Relaxed))
    }

    //what the players have sent since this was last asked, in order: who it is from, and the message
    pub(crate) fn take_inputs(&self) -> Vec<(u8, Vec<u8>)> {
        let taken: Vec<(u8, Vec<u8>)> = self.inputs.try_iter().collect();
        self.queued.fetch_sub(taken.len(), Ordering::Relaxed);
        taken
    }

    //tell everyone who has joined that the run has started
    pub(crate) fn start_run(&self) {
        let _ = self.commands.send(HostCommand::Start);
    }

    //put a player out of the game
    pub(crate) fn kick(&self, slot: u8) {
        let _ = self.commands.send(HostCommand::Kick(slot));
    }
}

impl Drop for NetHost {
    //closing the game closes the port, which is free again by the time this returns
    fn drop(&mut self) {
        let _ = self.commands.send(HostCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn listen_problem(error: &std::io::Error, port: u16) -> String {
    match error.kind() {
        std::io::ErrorKind::AddrInUse => format!("port {port} is already in use: close the other game, or use another port"),
        std::io::ErrorKind::PermissionDenied => format!("not allowed to use port {port}"),
        _ => format!("could not listen on port {port}: {error}"),
    }
}

//what every connection to the host needs to know, which doesn't change
struct HostContext {
    version: String,
    key: [u8; 32], //the password as it is made ready to be asked for (see `derive_key`)
    salt: [u8; SALT_BYTES],
    iterations: u32,
    inputs: std_mpsc::Sender<(u8, Vec<u8>)>, //where what the players send goes
    queued: Arc<AtomicUsize>,
    shim: Option<Shim>,
}

//a connection that has not joined yet is counted while it is being looked after
struct Joining(Arc<AtomicUsize>);

impl Drop for Joining {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

struct Joined {
    name: String,
    outbound: mpsc::UnboundedSender<ServerMsg>,
    connection: Connection,
    ping_ms: u16,
    received: u64,        //how many datagrams have come from them, when last looked
    heard_at: Instant,    //when that was last more than it had been
}

impl Joined {
    fn new(name: String, outbound: mpsc::UnboundedSender<ServerMsg>, connection: Connection) -> Joined {
        Joined { name, outbound, connection, ping_ms: 0, received: 0, heard_at: Instant::now() }
    }

    //how long it is since anything came from them
    fn look(&mut self) {
        let received = self.connection.stats().udp_rx.datagrams;
        if received != self.received {
            self.received = received;
            self.heard_at = Instant::now();
        }
    }

    fn silent(&self) -> bool {
        self.heard_at.elapsed() >= Duration::from_secs(STALE_SECONDS)
    }
}

//the one task that knows who is in the game: it hears of everyone who comes and goes, and tells everyone else
async fn run_host(endpoint: Endpoint, ctx: Arc<HostContext>, host_name: String, shared: Arc<Mutex<Vec<Member>>>, pipes: Arc<Mutex<Vec<Option<Pipe>>>>, mut commands: mpsc::UnboundedReceiver<HostCommand>, events: std_mpsc::Sender<HostEvent>) {
    let started = Instant::now();
    let mut guard = Guard::new();
    let mut players: Vec<Option<Joined>> = (0..MAX_PLAYERS).map(|_| None).collect(); //(the first place is the host, who is not a connection)
    let (hub_tx, mut hub_rx) = mpsc::unbounded_channel::<Hub>();
    let joining = Arc::new(AtomicUsize::new(0));
    let mut running = false; //whether the run has started, so that whoever joins after that is told
    let mut tick = tokio::time::interval(Duration::from_secs(ROSTER_SECONDS));
    let roster = |players: &[Option<Joined>]| -> Vec<Member> {
        let mut members = vec![Member { slot: 0, name: host_name.clone(), ping_ms: 0 }];
        members.extend(players.iter().enumerate().filter_map(|(slot, joined)| joined.as_ref().map(|joined| Member { slot: slot as u8, name: joined.name.clone(), ping_ms: joined.ping_ms })));
        members
    };
    let publish = |players: &[Option<Joined>]| {
        let members = roster(players);
        if let Ok(mut shared) = shared.lock() {
            *shared = members.clone();
        }
        for joined in players.iter().flatten() {
            let _ = joined.outbound.send(ServerMsg::Roster(members.clone()));
        }
    };
    loop {
        tokio::select! {
            incoming = endpoint.accept() => {
                let Some(incoming) = incoming else { break };
                //(an address that has not been shown to be the sender's is asked to prove it first, so one that is made up can't use up the tries of someone's real one)
                if !incoming.remote_address_validated() {
                    let _ = incoming.retry();
                    continue;
                }
                let address = incoming.remote_address();
                if joining.load(Ordering::Relaxed) >= MAX_JOINING {
                    incoming.refuse();
                    continue;
                }
                match guard.admit(address.ip(), started.elapsed().as_secs_f64()) {
                    Admission::Flood => incoming.ignore(),
                    Admission::Locked(wait) => {
                        let _ = events.send(HostEvent::TurnedAway { address, why: Refusal::TooManyTries { wait_seconds: wait }.text() });
                        joining.fetch_add(1, Ordering::Relaxed);
                        tokio::spawn(turn_away(incoming, Refusal::TooManyTries { wait_seconds: wait }, Joining(joining.clone())));
                    }
                    Admission::Go => {
                        joining.fetch_add(1, Ordering::Relaxed);
                        tokio::spawn(host_handshake(incoming, ctx.clone(), hub_tx.clone(), Joining(joining.clone())));
                    }
                }
            }
            command = commands.recv() => match command {
                Some(HostCommand::Start) => {
                    running = true;
                    for joined in players.iter().flatten() {
                        let _ = joined.outbound.send(ServerMsg::Start);
                    }
                }
                Some(HostCommand::Kick(slot)) => {
                    if let Some(joined) = players.get_mut(slot as usize).and_then(|joined| joined.as_ref()) {
                        let _ = joined.outbound.send(ServerMsg::Refused(Refusal::Kicked));
                    }
                }
                Some(HostCommand::Shutdown) | None => break,
            },
            Some(event) = hub_rx.recv() => match event {
                Hub::Join { name, address, outbound, pipe, connection, reply } => {
                    //(a player who is on a connection that has gone quiet, and joins again, takes their place back: it was theirs, and the host is the last to know it is gone)
                    if let Some(old) = (1..MAX_PLAYERS).find(|&slot| players[slot].as_ref().is_some_and(|joined| joined.name.eq_ignore_ascii_case(&name) && joined.silent())) {
                        if let Some(gone) = players[old].take() {
                            if let Ok(mut pipes) = pipes.lock() {
                                pipes[old] = None;
                            }
                            gone.connection.close(2u32.into(), b"joined again from another connection");
                            let _ = events.send(HostEvent::Left { slot: old as u8, name: gone.name, why: "joined again from another connection".to_owned() });
                        }
                    }
                    let taken = std::iter::once(host_name.as_str()).chain(players.iter().flatten().map(|joined| joined.name.as_str())).any(|other| other.eq_ignore_ascii_case(&name));
                    let free = (1..MAX_PLAYERS).find(|&slot| players[slot].is_none());
                    let answer = match (taken, free) {
                        (true, _) => Err(Refusal::NameTaken),
                        (false, None) => Err(Refusal::Full),
                        (false, Some(slot)) => {
                            if running {
                                let _ = outbound.send(ServerMsg::Start);
                            }
                            players[slot] = Some(Joined::new(name.clone(), outbound, connection));
                            if let Ok(mut pipes) = pipes.lock() {
                                pipes[slot] = Some(pipe);
                            }
                            Ok((slot as u8, roster(&players)))
                        }
                    };
                    if let Ok((slot, _)) = &answer {
                        let _ = events.send(HostEvent::Joined { slot: *slot, name, address });
                    }
                    let joined_now = answer.is_ok();
                    let _ = reply.send(answer);
                    if joined_now {
                        publish(&players);
                    }
                }
                Hub::Left { slot, connection, why } => {
                    let theirs = players.get(slot as usize).and_then(|joined| joined.as_ref()).is_some_and(|joined| joined.connection.stable_id() == connection);
                    if let Some(joined) = players.get_mut(slot as usize).filter(|_| theirs).and_then(|joined| joined.take()) {
                        if let Ok(mut pipes) = pipes.lock() {
                            pipes[slot as usize] = None;
                        }
                        let _ = events.send(HostEvent::Left { slot, name: joined.name, why });
                        publish(&players);
                    }
                }
                Hub::WrongPassword { address } => guard.failed(address.ip(), started.elapsed().as_secs_f64()),
                Hub::RightPassword { address } => guard.passed(address.ip()),
                Hub::TurnedAway { address, refusal } => {
                    let _ = events.send(HostEvent::TurnedAway { address, why: refusal.text() });
                }
            },
            _ = tick.tick() => {
                for joined in players.iter_mut().flatten() {
                    joined.ping_ms = joined.connection.rtt().as_millis().min(u16::MAX as u128) as u16;
                    joined.look();
                }
                publish(&players);
            }
        }
    }
    //(everyone is told, and given a moment to hear it before the port closes)
    for joined in players.iter().flatten() {
        let _ = joined.outbound.send(ServerMsg::Refused(Refusal::HostClosed));
    }
    tokio::time::sleep(Duration::from_millis(150)).await;
    endpoint.close(0u32.into(), b"the host has closed the game");
    let _ = tokio::time::timeout(Duration::from_millis(300), endpoint.wait_idle()).await;
}

//someone who is not let try: they are told why, and that is all
async fn turn_away(incoming: Incoming, refusal: Refusal, _joining: Joining) {
    let Ok(connecting) = incoming.accept() else { return };
    let Ok(Ok(connection)) = tokio::time::timeout(Duration::from_secs(HANDSHAKE_SECONDS), connecting).await else { return };
    let Ok(Ok((mut send, _receive))) = tokio::time::timeout(Duration::from_secs(HANDSHAKE_SECONDS), connection.accept_bi()).await else { return };
    let _ = write_frame(&mut send, &ServerMsg::Refused(refusal).to_bytes()).await;
    say_goodbye(send, &connection, b"turned away").await;
}

//what ends a try to join: the connection is dropped without a word (it was not a player, or it went away), or the player is told why they are not let in
enum Denied {
    Dropped,
    Told(SendStream, Refusal),
}

//a connection that has joined, with what is needed to look after it
struct Admitted {
    slot: u8,
    send: SendStream,
    receive: RecvStream,
    outbound: mpsc::UnboundedReceiver<ServerMsg>,
    world: mpsc::UnboundedReceiver<Outgoing>,
    pending: Arc<AtomicUsize>,
    inputs: std_mpsc::Sender<(u8, Vec<u8>)>,
    queued: Arc<AtomicUsize>,
    shim: Option<Shim>,
}

//what a connection has to get through before it is a player: who they are, which game they have, and the password
async fn host_handshake(incoming: Incoming, ctx: Arc<HostContext>, hub: mpsc::UnboundedSender<Hub>, joining: Joining) {
    let address = incoming.remote_address();
    let Ok(connecting) = incoming.accept() else { return };
    let Ok(Ok(connection)) = tokio::time::timeout(Duration::from_secs(HANDSHAKE_SECONDS), connecting).await else { return };
    let result = tokio::time::timeout(Duration::from_secs(HANDSHAKE_SECONDS), host_admit(&connection, address, &ctx, &hub)).await;
    drop(joining);
    match result {
        Ok(Ok(admitted)) => admitted.run(connection, hub).await,
        Ok(Err(Denied::Told(mut send, refusal))) => {
            let _ = hub.send(Hub::TurnedAway { address, refusal: refusal.clone() });
            let _ = write_frame(&mut send, &ServerMsg::Refused(refusal).to_bytes()).await;
            say_goodbye(send, &connection, b"refused").await;
        }
        Ok(Err(Denied::Dropped)) | Err(_) => connection.close(1u32.into(), b"too slow or not understood"),
    }
}

//the handshake itself
async fn host_admit(connection: &Connection, address: SocketAddr, ctx: &HostContext, hub: &mpsc::UnboundedSender<Hub>) -> Result<Admitted, Denied> {
    let (mut send, mut receive) = connection.accept_bi().await.map_err(|_| Denied::Dropped)?;
    let hello = read_frame(&mut receive).await.map_err(|_| Denied::Dropped)?;
    let name = match ClientMsg::from_bytes(&hello) {
        Some(ClientMsg::Hello { protocol, version: _, name }) => {
            if protocol != PROTOCOL {
                return Err(Denied::Told(send, Refusal::WrongVersion { host_protocol: PROTOCOL, host_version: ctx.version.clone() }));
            }
            if name.is_empty() || clean_name(&name) != name {
                return Err(Denied::Told(send, Refusal::BadName));
            }
            name
        }
        _ => return Err(Denied::Dropped),
    };
    let nonce: [u8; NONCE_BYTES] = random_bytes();
    let challenge = ServerMsg::Challenge { salt: ctx.salt, nonce, iterations: ctx.iterations };
    write_frame(&mut send, &challenge.to_bytes()).await.map_err(|_| Denied::Dropped)?;
    let answer = read_frame(&mut receive).await.map_err(|_| Denied::Dropped)?;
    let Some(ClientMsg::Proof(proof)) = ClientMsg::from_bytes(&answer) else {
        return Err(Denied::Dropped);
    };
    let Ok(binding) = binding_of(connection) else {
        return Err(Denied::Dropped);
    };
    if !check_proof(&ctx.key, &nonce, &binding, &name, &proof) {
        let _ = hub.send(Hub::WrongPassword { address });
        return Err(Denied::Told(send, Refusal::WrongPassword));
    }
    let _ = hub.send(Hub::RightPassword { address });
    let (outbound_tx, outbound) = mpsc::unbounded_channel();
    let (world_tx, world) = mpsc::unbounded_channel();
    let pending = Arc::new(AtomicUsize::new(0));
    let pipe = Pipe { world: world_tx, pending: pending.clone() };
    let (reply_tx, reply) = oneshot::channel();
    if hub.send(Hub::Join { name, address, outbound: outbound_tx, pipe, connection: connection.clone(), reply: reply_tx }).is_err() {
        return Err(Denied::Told(send, Refusal::HostClosed));
    }
    match reply.await {
        Ok(Ok((slot, members))) => {
            if write_frame(&mut send, &ServerMsg::Welcome { slot, members }.to_bytes()).await.is_err() {
                let _ = hub.send(Hub::Left { slot, connection: connection.stable_id(), why: "the connection was lost".to_owned() });
                return Err(Denied::Dropped);
            }
            Ok(Admitted { slot, send, receive, outbound, world, pending, inputs: ctx.inputs.clone(), queued: ctx.queued.clone(), shim: ctx.shim })
        }
        Ok(Err(refusal)) => Err(Denied::Told(send, refusal)),
        Err(_) => Err(Denied::Told(send, Refusal::HostClosed)),
    }
}

impl Admitted {
    //a player that is in: what they send is read (that they leave on the first stream, what they ask of the world on the one they open), the world is
    //written to them on a stream of its own, and what the host has to tell them is sent, until one of them ends. the host is told once that they have
    //gone, because their place can be given to someone else straight after
    async fn run(self, connection: Connection, hub: mpsc::UnboundedSender<Hub>) {
        let Admitted { slot, mut send, mut receive, mut outbound, world, pending, inputs, queued, shim } = self;
        let mut reader = tokio::spawn(async move {
            match read_frame(&mut receive).await {
                Ok(message) => match ClientMsg::from_bytes(&message) {
                    Some(ClientMsg::Leave) => "left the game".to_owned(),
                    _ => "sent what could not be understood".to_owned(),
                },
                Err(error) => format!("the connection ended ({error})"),
            }
        });
        let mut writer = tokio::spawn(write_world(connection.clone(), world, pending, shim));
        let mut listener = tokio::spawn(read_inputs(connection.clone(), slot, inputs, queued));
        let why = loop {
            tokio::select! {
                heard = &mut reader => break heard.unwrap_or_else(|_| "the connection ended".to_owned()),
                done = &mut writer => break match done {
                    Ok(Err(error)) => format!("the connection ended ({error})"),
                    _ => "the connection ended".to_owned(),
                },
                heard = &mut listener => break heard.unwrap_or_else(|_| "the connection ended".to_owned()),
                message = outbound.recv() => match message {
                    Some(message) => {
                        let over = matches!(message, ServerMsg::Refused(_));
                        if write_frame(&mut send, &message.to_bytes()).await.is_err() {
                            break "the connection ended".to_owned();
                        }
                        if over {
                            break "was put out".to_owned();
                        }
                    }
                    None => break "the game was closed".to_owned(),
                },
            }
        };
        reader.abort();
        writer.abort();
        listener.abort();
        let _ = hub.send(Hub::Left { slot, connection: connection.stable_id(), why });
        say_goodbye(send, &connection, b"goodbye").await;
    }
}

// ---------- a player ----------

//what a player's game hears of the host it joined
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ClientEvent {
    Joined { slot: u8, members: Vec<Member> },
    Roster(Vec<Member>),
    Started,
    World(Vec<u8>), //a message of the world (see `Replica`)
    Refused(Refusal),
    Failed(String), //the host could not be reached, or something went wrong on the way
    Closed(String), //the connection ended, with what is known of why
}

pub(crate) struct NetClient {
    events: std_mpsc::Receiver<ClientEvent>,
    inputs: mpsc::UnboundedSender<Outgoing>,
    leave: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    reconnect: Reconnect, //(kept for as long as the game is, in this machine's memory and nowhere else)
}

impl NetClient {
    //start connecting to a host (`target` is what was typed, see `parse_target`), without waiting: how it goes comes as events
    pub(crate) fn connect(target: &str, name: &str, password: &str) -> NetClient {
        let (event_tx, events) = std_mpsc::channel();
        let (leave_tx, leave_rx) = oneshot::channel();
        let (input_tx, input_rx) = mpsc::unbounded_channel();
        let (target, name, password) = (target.to_owned(), clean_name(name), password.to_owned());
        let reconnect = Reconnect { target: target.clone(), name: name.clone(), password: password.clone() };
        let failed = event_tx.clone();
        let spawned = std::thread::Builder::new().name("net-client".to_owned()).spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = event_tx.send(ClientEvent::Failed(format!("could not start the network: {error}")));
                    return;
                }
            };
            runtime.block_on(run_client(target, name, password, event_tx, input_rx, leave_rx));
            runtime.shutdown_timeout(Duration::from_millis(300));
        });
        let thread = match spawned {
            Ok(thread) => Some(thread),
            Err(error) => {
                let _ = failed.send(ClientEvent::Failed(format!("could not start the network: {error}")));
                None
            }
        };
        NetClient { events, inputs: input_tx, leave: Some(leave_tx), thread, reconnect }
    }

    //what it takes to join the same game again, as the same player
    pub(crate) fn reconnection(&self) -> Reconnect {
        self.reconnect.clone()
    }

    //what has happened since this was last asked
    pub(crate) fn poll(&mut self) -> Vec<ClientEvent> {
        self.events.try_iter().collect()
    }

    #[cfg(test)]
    pub(crate) fn next(&mut self) -> Option<ClientEvent> {
        self.events.try_recv().ok()
    }

    //send the host what the player asked for in a frame (see `ClientFrame`)
    pub(crate) fn send_input(&self, message: Vec<u8>) {
        if !message.is_empty() && message.len() <= MAX_UP_BYTES {
            let _ = self.inputs.send(Outgoing { at: Instant::now(), bytes: message });
        }
    }

    //say goodbye to the host (this is also done when the client is dropped)
    pub(crate) fn leave(&mut self) {
        if let Some(leave) = self.leave.take() {
            let _ = leave.send(());
        }
    }
}

impl Drop for NetClient {
    fn drop(&mut self) {
        self.leave();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

//how a try to join, or a stay in a game, ends other than by the player leaving
enum Ended {
    Refused(Refusal),
    Failed(String),
}

impl From<String> for Ended {
    fn from(problem: String) -> Ended {
        Ended::Failed(problem)
    }
}

async fn run_client(target: String, name: String, password: String, events: std_mpsc::Sender<ClientEvent>, inputs: mpsc::UnboundedReceiver<Outgoing>, leave: oneshot::Receiver<()>) {
    let event = match client_session(&target, &name, &password, &events, inputs, leave).await {
        Ok(why) => ClientEvent::Closed(why),
        Err(Ended::Refused(refusal)) => ClientEvent::Refused(refusal),
        Err(Ended::Failed(problem)) => ClientEvent::Failed(problem),
    };
    let _ = events.send(event);
}

//a connection to the host that has joined: the endpoint has to be kept for as long as the connection is
struct Seat {
    _endpoint: Endpoint,
    connection: Connection,
    send: SendStream,
    receive: RecvStream,
}

async fn client_session(target: &str, name: &str, password: &str, events: &std_mpsc::Sender<ClientEvent>, inputs: mpsc::UnboundedReceiver<Outgoing>, mut leave: oneshot::Receiver<()>) -> Result<String, Ended> {
    //(leaving while it is still trying to join just stops trying)
    let seat = tokio::select! {
        seat = join(target, name, password, events) => seat?,
        _ = &mut leave => return Ok("you left the game".to_owned()),
    };
    let Seat { _endpoint, connection, mut send, mut receive } = seat;
    //(the world comes on a stream the host opens when it has something to say, and what the player asks for goes on one of their own)
    let world_events = events.clone();
    let world_connection = connection.clone();
    tokio::spawn(async move {
        let Ok(mut receive) = world_connection.accept_uni().await else { return };
        while let Ok(message) = read_big(&mut receive, MAX_DOWN_BYTES).await {
            if world_events.send(ClientEvent::World(message)).is_err() {
                break;
            }
        }
    });
    tokio::spawn(write_inputs(connection.clone(), inputs, Shim::from_env()));
    //what the host says is read in a task of its own, as reading half a message can't be cancelled
    let (heard_tx, mut heard) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            let message = read_frame(&mut receive).await;
            let ended = message.is_err();
            if heard_tx.send(message).is_err() || ended {
                break;
            }
        }
    });
    loop {
        tokio::select! {
            message = heard.recv() => match message {
                Some(Ok(bytes)) => match ServerMsg::from_bytes(&bytes) {
                    Some(ServerMsg::Roster(members)) => {
                        if events.send(ClientEvent::Roster(members)).is_err() {
                            return Ok("the game was closed".to_owned());
                        }
                    }
                    Some(ServerMsg::Start) => {
                        let _ = events.send(ClientEvent::Started);
                    }
                    Some(ServerMsg::Refused(refusal)) => return Err(Ended::Refused(refusal)),
                    _ => return Err(Ended::Failed("the host sent what could not be understood".to_owned())),
                },
                Some(Err(error)) => return Ok(format!("the connection ended ({error})")),
                None => return Ok("the connection ended".to_owned()),
            },
            _ = &mut leave => {
                let goodbye = ClientMsg::Leave.to_bytes();
                let _ = tokio::time::timeout(Duration::from_secs(CLOSE_SECONDS), write_frame(&mut send, &goodbye)).await;
                say_goodbye(send, &connection, b"left").await;
                return Ok("you left the game".to_owned());
            }
        }
    }
}

//from finding the host to being let in
async fn join(target: &str, name: &str, password: &str, events: &std_mpsc::Sender<ClientEvent>) -> Result<Seat, Ended> {
    let target = parse_target(target)?;
    let address = tokio::net::lookup_host(target.as_str()).await.ok().and_then(|mut found| found.next()).ok_or_else(|| format!("could not find {target}"))?;
    let local = if address.is_ipv4() { SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)) } else { SocketAddr::from(([0u16; 8], 0)) };
    let mut endpoint = Endpoint::client(local).map_err(|error| format!("could not use the network: {error}"))?;
    endpoint.set_default_client_config(client_config()?);
    let connecting = endpoint.connect(address, SERVER_NAME).map_err(|error| format!("could not connect: {error}"))?;
    let connection = match tokio::time::timeout(Duration::from_secs(CONNECT_SECONDS), connecting).await {
        Ok(Ok(connection)) => connection,
        Ok(Err(error)) => return Err(Ended::Failed(connect_problem(&error, &target))),
        Err(_) => return Err(Ended::Failed(format!("no answer from {target}: is the address right, and the port open on the host's router?"))),
    };
    let (mut send, mut receive) = connection.open_bi().await.map_err(|error| error.to_string())?;
    let hello = ClientMsg::Hello { protocol: PROTOCOL, version: env!("CARGO_PKG_VERSION").to_owned(), name: name.to_owned() };
    write_frame(&mut send, &hello.to_bytes()).await?;
    let wait = Duration::from_secs(HANDSHAKE_SECONDS);
    let challenge = tokio::time::timeout(wait, read_frame(&mut receive)).await.map_err(|_| "the host did not answer".to_owned())??;
    let (salt, nonce, iterations) = match ServerMsg::from_bytes(&challenge) {
        Some(ServerMsg::Challenge { salt, nonce, iterations }) => (salt, nonce, iterations),
        Some(ServerMsg::Refused(refusal)) => return Err(Ended::Refused(refusal)),
        _ => return Err(Ended::Failed("the host sent what could not be understood".to_owned())),
    };
    if iterations == 0 || iterations > MAX_KDF_ITERATIONS {
        return Err(Ended::Failed("the host asked for more work than a password takes".to_owned()));
    }
    let binding = binding_of(&connection)?;
    let password = password.to_owned();
    let key = tokio::task::spawn_blocking(move || derive_key(&password, &salt, iterations)).await.map_err(|error| error.to_string())?;
    write_frame(&mut send, &ClientMsg::Proof(make_proof(&key, &nonce, &binding, name)).to_bytes()).await?;
    let welcome = tokio::time::timeout(wait, read_frame(&mut receive)).await.map_err(|_| "the host did not answer".to_owned())??;
    match ServerMsg::from_bytes(&welcome) {
        Some(ServerMsg::Welcome { slot, members }) => {
            let _ = events.send(ClientEvent::Joined { slot, members });
        }
        Some(ServerMsg::Refused(refusal)) => return Err(Ended::Refused(refusal)),
        _ => return Err(Ended::Failed("the host sent what could not be understood".to_owned())),
    }
    Ok(Seat { _endpoint: endpoint, connection, send, receive })
}

fn connect_problem(error: &quinn::ConnectionError, target: &str) -> String {
    match error {
        quinn::ConnectionError::TimedOut => format!("no answer from {target}: is the address right, and the port open on the host's router?"),
        quinn::ConnectionError::ConnectionClosed(_) | quinn::ConnectionError::ApplicationClosed(_) => format!("{target} refused the connection"),
        other => format!("could not connect to {target}: {other}"),
    }
}
