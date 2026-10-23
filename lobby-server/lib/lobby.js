// The rules of the lobby list, apart from where it runs (api/games.js) and where it keeps things (store.js).
//
// A host says "I am open" with POST and says it again every so often; a game that stops being said falls off the list. The address a
// game is listed at is the one the request came from, never one the host writes down, so nobody can list somebody else's machine.
// Players read the list with GET and then connect to the host directly: nothing about the game itself passes through here.

import { createHash } from 'node:crypto';
import { isIP } from 'node:net';

export const TTL_SECONDS = 90; // how long a game stays listed after it was last said
export const MAX_GAMES = 200; // the most that are listed at once
export const MAX_PER_ADDRESS = 3; // the most one address can have listed (a few people can share a home)
export const WRITES_PER_MINUTE = 20; // listing, refreshing and withdrawing, from one address
export const READS_PER_MINUTE = 60; // reading the list, from one address
export const MIN_PORT = 1024;
export const MAX_NAME_LENGTH = 16; // the same as the game's own limit
export const MAX_PLAYERS = 16; // the game has four; this is only so a bigger game later does not need a new server
export const MAX_BODY_BYTES = 1024;

const reply = (status, body, headers = {}) => ({ status, body, headers });
const failure = (status, error, headers) => reply(status, { error }, headers);

// ---------- addresses ----------

// an address as a string that is the same however it was written, or null if it is not one
export function normalizeAddress(raw) {
  if (typeof raw !== 'string') return null;
  let text = raw.trim();
  const zone = text.indexOf('%');
  if (zone >= 0) text = text.slice(0, zone);
  const mapped = /^::ffff:(\d+\.\d+\.\d+\.\d+)$/i.exec(text);
  if (mapped) text = mapped[1];
  return isIP(text) ? text.toLowerCase() : null;
}

const V4_PRIVATE = [
  ['0.0.0.0', 8], ['10.0.0.0', 8], ['100.64.0.0', 10], ['127.0.0.0', 8], ['169.254.0.0', 16], ['172.16.0.0', 12],
  ['192.0.0.0', 24], ['192.0.2.0', 24], ['192.168.0.0', 16], ['198.18.0.0', 15], ['198.51.100.0', 24], ['203.0.113.0', 24],
  ['224.0.0.0', 4], ['240.0.0.0', 4],
];

const v4Number = (address) => address.split('.').reduce((total, part) => total * 256 + Number(part), 0);

// whether an address is one the rest of the internet could reach (not a home network's, a loopback, a documentation range...)
export function isPublic(address) {
  if (isIP(address) === 4) {
    const value = v4Number(address);
    return !V4_PRIVATE.some(([base, bits]) => Math.floor(value / 2 ** (32 - bits)) === Math.floor(v4Number(base) / 2 ** (32 - bits)));
  }
  const groups = expandV6(address);
  if (!groups) return false;
  const [first, second] = groups;
  if (groups.every((group) => group === 0)) return false; // ::
  if (groups.slice(0, 7).every((group) => group === 0) && groups[7] === 1) return false; // ::1
  if ((first & 0xfe00) === 0xfc00) return false; // fc00::/7, private
  if ((first & 0xffc0) === 0xfe80) return false; // fe80::/10, link-local
  if ((first & 0xff00) === 0xff00) return false; // ff00::/8, multicast
  if (first === 0x2001 && second === 0x0db8) return false; // documentation
  return true;
}

function expandV6(address) {
  const [head, tail, extra] = address.split('::');
  if (extra !== undefined) return null;
  const parse = (part) => (part ? part.split(':').map((group) => parseInt(group, 16)) : []);
  const front = parse(head);
  const back = parse(tail);
  if (tail === undefined) return front.length === 8 ? front : null;
  const fill = 8 - front.length - back.length;
  return fill < 0 ? null : [...front, ...Array(fill).fill(0), ...back];
}

// ---------- what a host says ----------

const isCount = (value, least, most) => Number.isInteger(value) && value >= least && value <= most;

// a name as the game keeps them: letters, numbers, spaces and a few marks, no more than MAX_NAME_LENGTH of them
export function cleanName(name) {
  if (typeof name !== 'string') return '';
  const kept = [...name].filter((letter) => /[\p{L}\p{N} _'.-]/u.test(letter)).join('');
  return [...kept.replace(/\s+/g, ' ').trim()].slice(0, MAX_NAME_LENGTH).join('').trimEnd();
}

// what is wrong with what a host said (a string for them), or the listing made from it
export function listingFrom(body) {
  if (typeof body !== 'object' || body === null || Array.isArray(body)) return { problem: 'the body must be a JSON object' };
  if (JSON.stringify(body).length > MAX_BODY_BYTES) return { problem: 'that is too much to say' };
  const name = cleanName(body.name);
  if (!name) return { problem: 'name: a game needs the name of its host' };
  if (!isCount(body.port, MIN_PORT, 65535)) return { problem: `port: a number from ${MIN_PORT} to 65535` };
  if (!isCount(body.max, 1, MAX_PLAYERS)) return { problem: `max: a number from 1 to ${MAX_PLAYERS}` };
  if (!isCount(body.players, 1, body.max)) return { problem: 'players: a number from 1 to max' };
  if (typeof body.locked !== 'boolean') return { problem: 'locked: true or false' };
  if (typeof body.started !== 'boolean') return { problem: 'started: true or false' };
  if (typeof body.version !== 'string' || !/^[0-9A-Za-z.+-]{1,24}$/.test(body.version)) return { problem: 'version: the game version, like 0.1.4' };
  const { port, max, players, locked, started, version } = body;
  return { value: { name, port, max, players, locked, started, version } };
}

export const gameId = (address, port) => createHash('sha256').update(`${address}|${port}`).digest('hex').slice(0, 16);

// the order a player sees them in: the ones with room first, the fullest of those first, then by name
function sorted(games) {
  return [...games].sort((a, b) => {
    const room = Number(b.players < b.max) - Number(a.players < a.max);
    return room || b.players - a.players || a.name.localeCompare(b.name) || a.port - b.port;
  });
}

const shown = (game, now) => ({
  id: game.id, name: game.name, address: game.address, port: game.port, players: game.players, max: game.max,
  locked: game.locked, started: game.started, version: game.version, age: Math.max(0, Math.round((now - game.since) / 1000)),
});

// ---------- the requests ----------

// request: { method, address, query, body, now }   store: see store.js   options: { allowPrivate }
// the answer is { status, body, headers }
export async function handle(request, store, options = {}) {
  const { method, query = {}, body, now = Date.now() } = request;
  if (method !== 'GET' && method !== 'POST' && method !== 'DELETE') return failure(405, 'use GET, POST or DELETE', { Allow: 'GET, POST, DELETE' });
  const address = normalizeAddress(request.address);
  if (!address) return failure(400, 'the address this came from could not be told');
  if (!options.allowPrivate && !isPublic(address)) return failure(400, 'only games on public addresses can be listed');

  const writing = method !== 'GET';
  const tries = await store.hit(`${writing ? 'write' : 'read'}:${address}:${Math.floor(now / 60000)}`, 120);
  if (tries > (writing ? WRITES_PER_MINUTE : READS_PER_MINUTE)) {
    const wait = 60 - Math.floor(now / 1000) % 60;
    return failure(429, 'too many requests, try again in a moment', { 'Retry-After': String(wait) });
  }

  const live = await store.all(now);
  if (method === 'GET') {
    return reply(200, { games: sorted(live).map((game) => shown(game, now)), now, ttl: TTL_SECONDS, store: store.kind });
  }

  if (method === 'DELETE') {
    const port = Number(query.port);
    if (!isCount(port, MIN_PORT, 65535)) return failure(400, `port: a number from ${MIN_PORT} to 65535`);
    const id = gameId(address, port);
    const listed = live.some((game) => game.id === id);
    if (listed) await store.remove(id);
    return reply(200, { removed: listed });
  }

  const { problem, value } = listingFrom(body);
  if (problem) return failure(400, problem);
  const id = gameId(address, value.port);
  const before = live.find((game) => game.id === id);
  if (!before) {
    if (live.length >= MAX_GAMES) return failure(503, 'the list is full, try again later');
    if (live.filter((game) => game.address === address).length >= MAX_PER_ADDRESS) return failure(429, `no more than ${MAX_PER_ADDRESS} games from one address`);
  }
  await store.put({ ...value, id, address, since: before ? before.since : now, expires: now + TTL_SECONDS * 1000 });
  return reply(before ? 200 : 201, { id, ttl: TTL_SECONDS, address });
}
