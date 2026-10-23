import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  handle, normalizeAddress, isPublic, cleanName, gameId, TTL_SECONDS, MAX_GAMES, MAX_PER_ADDRESS, WRITES_PER_MINUTE, READS_PER_MINUTE,
} from '../lib/lobby.js';
import { memoryStore, upstashStore, storeFromEnv } from '../lib/store.js';

const HOST = '93.184.216.34';
const T0 = 1_700_000_000_000;
const game = (extra = {}) => ({ name: 'Ann', port: 47821, players: 1, max: 4, locked: true, started: false, version: '0.1.4', ...extra });
const post = (store, body, address = HOST, now = T0) => handle({ method: 'POST', address, body, now }, store);
const list = async (store, now = T0, address = '198.51.101.9') => (await handle({ method: 'GET', address, now }, store)).body.games;

test('a game that is listed can be read by others', async () => {
  const store = memoryStore();
  const made = await post(store, game());
  assert.equal(made.status, 201);
  assert.equal(made.body.ttl, TTL_SECONDS);
  const games = await list(store);
  assert.equal(games.length, 1);
  assert.deepEqual({ ...games[0], id: undefined }, { id: undefined, name: 'Ann', address: HOST, port: 47821, players: 1, max: 4, locked: true, started: false, version: '0.1.4', age: 0 });
  assert.equal(games[0].id, made.body.id);
});

test('the address is the one the request came from, whatever the host writes', async () => {
  const store = memoryStore();
  await post(store, { ...game(), address: '8.8.8.8', ip: '8.8.4.4', host: '1.1.1.1' });
  const [listed] = await list(store);
  assert.equal(listed.address, HOST);
  assert.equal(Object.keys(listed).includes('ip'), false);
});

test('saying it again refreshes the game instead of listing it twice', async () => {
  const store = memoryStore();
  const first = await post(store, game(), HOST, T0);
  const again = await post(store, game({ players: 3, started: true }), HOST, T0 + 30_000);
  assert.equal(again.status, 200);
  assert.equal(again.body.id, first.body.id);
  const games = await list(store, T0 + 40_000);
  assert.equal(games.length, 1);
  assert.equal(games[0].players, 3);
  assert.equal(games[0].started, true);
  assert.equal(games[0].age, 40); // (it has been listed since the first time)
});

test('a game that stops being said falls off the list', async () => {
  const store = memoryStore();
  await post(store, game(), HOST, T0);
  assert.equal((await list(store, T0 + (TTL_SECONDS - 1) * 1000)).length, 1);
  assert.equal((await list(store, T0 + TTL_SECONDS * 1000)).length, 0);
});

test('a host can take a game off, but only its own', async () => {
  const store = memoryStore();
  await post(store, game());
  const other = await handle({ method: 'DELETE', address: '93.184.216.35', query: { port: '47821' }, now: T0 }, store);
  assert.deepEqual(other.body, { removed: false });
  assert.equal((await list(store)).length, 1);
  const own = await handle({ method: 'DELETE', address: HOST, query: { port: '47821' }, now: T0 }, store);
  assert.deepEqual(own.body, { removed: true });
  assert.equal((await list(store)).length, 0);
  const none = await handle({ method: 'DELETE', address: HOST, query: { port: '47821' }, now: T0 }, store);
  assert.deepEqual(none.body, { removed: false });
  assert.equal((await handle({ method: 'DELETE', address: HOST, query: {}, now: T0 }, store)).status, 400);
});

test('what is wrong with a listing is said', async () => {
  const store = memoryStore();
  const bad = [
    ['name', { name: '' }], ['name', { name: '!!!' }], ['name', { name: 7 }], ['port', { port: 80 }], ['port', { port: 70000 }], ['port', { port: 4782.5 }],
    ['port', { port: '47821' }], ['max', { max: 0 }], ['max', { max: 99 }], ['players', { players: 0 }], ['players', { players: 5 }], ['locked', { locked: 'yes' }],
    ['started', { started: undefined }], ['version', { version: '' }], ['version', { version: 'a b' }], ['version', { version: 'x'.repeat(40) }],
  ];
  let from = 0; // (each from an address of its own, as one can only ask so often)
  const elsewhere = () => `8.8.${from++}.1`;
  for (const [field, change] of bad) {
    const answer = await post(store, { ...game(), ...change }, elsewhere());
    assert.equal(answer.status, 400, JSON.stringify(change));
    assert.match(answer.body.error, new RegExp(`^${field}`), JSON.stringify(change));
  }
  for (const body of [undefined, null, 'text', 7, [], [game()]]) assert.equal((await post(store, body, elsewhere())).status, 400);
  assert.equal((await post(store, { ...game(), padding: 'x'.repeat(2000) }, elsewhere())).status, 400);
  assert.equal((await list(store)).length, 0);
});

test('names are cleaned like the game cleans them', () => {
  assert.equal(cleanName('  Ann   Lee '), 'Ann Lee');
  assert.equal(cleanName('<b>Ann</b>\u0000‮'), 'bAnnb');
  assert.equal(cleanName('x'.repeat(40)), 'x'.repeat(16));
  assert.equal(cleanName("O'Neil-Smith_2.0"), "O'Neil-Smith_2.0");
  assert.equal(cleanName('Zoë Ünal'), 'Zoë Ünal');
  assert.equal(cleanName('abcdefghijklmno pq'), 'abcdefghijklmno');
  assert.equal(cleanName(undefined), '');
});

test('only public addresses can be listed', async () => {
  const store = memoryStore();
  for (const address of ['10.1.2.3', '192.168.0.9', '127.0.0.1', '172.20.0.1', '100.64.0.1', '169.254.1.1', '::1', 'fe80::1', 'fd12::3', '2001:db8::5', null, 'not an address']) {
    assert.equal((await post(store, game(), address)).status, 400, String(address));
  }
  assert.equal((await post(store, game(), '2606:4700:4700::1111')).status, 201);
  const dev = await handle({ method: 'POST', address: '127.0.0.1', body: game(), now: T0 }, memoryStore(), { allowPrivate: true });
  assert.equal(dev.status, 201);
});

test('addresses are told apart properly', () => {
  assert.equal(normalizeAddress('::ffff:93.184.216.34'), '93.184.216.34');
  assert.equal(normalizeAddress(' 2606:4700:4700::1111 '), '2606:4700:4700::1111');
  assert.equal(normalizeAddress('fe80::1%eth0'), 'fe80::1');
  assert.equal(normalizeAddress('999.1.1.1'), null);
  assert.equal(normalizeAddress(5), null);
  assert.equal(isPublic('8.8.8.8'), true);
  assert.equal(isPublic('172.32.0.1'), true);
  assert.equal(isPublic('172.31.255.255'), false);
  assert.equal(isPublic('203.0.113.7'), false); // (documentation)
  assert.equal(isPublic('::'), false);
  assert.equal(isPublic('2a00:1450:4001::1'), true);
});

test('there is a limit to how often one address can ask', async () => {
  const store = memoryStore();
  for (let i = 0; i < WRITES_PER_MINUTE; i++) assert.equal((await post(store, game())).status, i === 0 ? 201 : 200);
  const refused = await post(store, game());
  assert.equal(refused.status, 429);
  assert.ok(Number(refused.headers['Retry-After']) >= 1);
  // (reading is counted apart, and another address has its own)
  assert.equal((await handle({ method: 'GET', address: HOST, now: T0 }, store)).status, 200);
  assert.equal((await post(store, game({ port: 50000 }), '93.184.216.99')).status, 201);
  for (let i = 0; i < READS_PER_MINUTE; i++) assert.equal((await handle({ method: 'GET', address: '8.8.8.8', now: T0 }, store)).status, 200);
  assert.equal((await handle({ method: 'GET', address: '8.8.8.8', now: T0 }, store)).status, 429);
});

test('one address cannot fill the list, and the list can only hold so much', async () => {
  const store = memoryStore();
  for (let i = 0; i < MAX_PER_ADDRESS; i++) assert.equal((await post(store, game({ port: 47821 + i }))).status, 201);
  assert.equal((await post(store, game({ port: 50000 }))).status, 429);
  assert.equal((await post(store, game({ port: 47821 }))).status, 200); // (one it has already is only refreshed)
  const crowded = memoryStore();
  for (let i = 0; i < MAX_GAMES; i++) {
    await crowded.put({ ...game(), id: `g${i}`, address: `8.8.${Math.floor(i / 250)}.${i % 250 + 1}`, since: T0, expires: T0 + 60_000 });
  }
  assert.equal((await post(crowded, game())).status, 503);
});

test('the ones with room come first, then the fullest', async () => {
  const store = memoryStore();
  await post(store, game({ name: 'Zed', players: 4 }), '8.8.8.1');
  await post(store, game({ name: 'Bea', players: 1 }), '8.8.8.2');
  await post(store, game({ name: 'Cal', players: 3 }), '8.8.8.3');
  await post(store, game({ name: 'Abe', players: 3 }), '8.8.8.4');
  assert.deepEqual((await list(store)).map((listed) => listed.name), ['Abe', 'Cal', 'Bea', 'Zed']);
});

test('only GET, POST and DELETE are answered', async () => {
  const answer = await handle({ method: 'PUT', address: HOST, body: game(), now: T0 }, memoryStore());
  assert.equal(answer.status, 405);
  assert.equal(answer.headers.Allow, 'GET, POST, DELETE');
});

test('a game is known by where it is', () => {
  assert.equal(gameId(HOST, 47821), gameId(HOST, 47821));
  assert.notEqual(gameId(HOST, 47821), gameId(HOST, 47822));
  assert.notEqual(gameId(HOST, 47821), gameId('93.184.216.35', 47821));
  assert.match(gameId(HOST, 47821), /^[0-9a-f]{16}$/);
});

// ---------- the database ----------

// enough of Upstash's REST interface for the store to be tried against: the commands it uses, in a pipeline
function fakeUpstash() {
  const hashes = new Map();
  const counters = new Map();
  const calls = [];
  const fetch = async (url, init) => {
    calls.push({ url, init });
    if (init.headers.Authorization !== 'Bearer secret') return { ok: false, status: 401, json: async () => ({ error: 'Unauthorized' }) };
    const commands = JSON.parse(init.body);
    const results = commands.map(([name, key, ...rest]) => {
      switch (name) {
        case 'HGETALL': return { result: [...(hashes.get(key) ?? new Map())].flat() };
        case 'HSET': (hashes.get(key) ?? hashes.set(key, new Map()).get(key)).set(rest[0], rest[1]); return { result: 1 };
        case 'HDEL': return { result: rest.filter((field) => hashes.get(key)?.delete(field)).length };
        case 'INCR': counters.set(key, (counters.get(key) ?? 0) + 1); return { result: counters.get(key) };
        case 'EXPIRE': return { result: 1 };
        default: return { error: `ERR unknown command '${name}'` };
      }
    });
    return { ok: true, status: 200, json: async () => results };
  };
  return { fetch, hashes, calls };
}

test('the Upstash store keeps games and counts in the database', async () => {
  const fake = fakeUpstash();
  const store = upstashStore({ url: 'https://example.upstash.io/', token: 'secret', fetch: fake.fetch });
  assert.equal(store.kind, 'upstash');
  assert.equal((await post(store, game())).status, 201);
  assert.equal((await post(store, game({ players: 2 }), HOST, T0 + 1000)).status, 200);
  const games = await list(store, T0 + 2000);
  assert.equal(games.length, 1);
  assert.equal(games[0].players, 2);
  assert.ok(fake.calls.every((call) => call.url === 'https://example.upstash.io/pipeline'));
  // (an old entry, and one that is not even JSON, are dropped when the list is read)
  fake.hashes.get('army:games').set('old', JSON.stringify({ ...games[0], id: 'old', expires: T0 }));
  fake.hashes.get('army:games').set('mangled', '{not json');
  assert.equal((await list(store, T0 + 3000)).length, 1);
  assert.deepEqual([...fake.hashes.get('army:games').keys()], [games[0].id]);
  await store.remove(games[0].id);
  assert.equal((await list(store, T0 + 4000)).length, 0);
  assert.equal(await store.hit('k', 60), 1);
  assert.equal(await store.hit('k', 60), 2);
});

test('a database that refuses is an error, not an empty list', async () => {
  const fake = fakeUpstash();
  const store = upstashStore({ url: 'https://example.upstash.io', token: 'wrong', fetch: fake.fetch });
  await assert.rejects(() => store.all(T0), /401/);
});

test('the store comes from the environment', () => {
  assert.equal(storeFromEnv({}).kind, 'memory');
  assert.equal(storeFromEnv({ UPSTASH_REDIS_REST_URL: 'https://a.upstash.io', UPSTASH_REDIS_REST_TOKEN: 't' }).kind, 'upstash');
  assert.equal(storeFromEnv({ KV_REST_API_URL: 'https://a.upstash.io', KV_REST_API_TOKEN: 't' }).kind, 'upstash');
  assert.equal(storeFromEnv({ KV_REST_API_URL: 'https://a.upstash.io' }).kind, 'memory');
});
