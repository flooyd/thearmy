import { test } from 'node:test';
import assert from 'node:assert/strict';
import handler, { addressOf } from '../api/games.js';

// what Vercel hands a function, near enough
function call(req, env) {
  return new Promise((resolve) => {
    const res = {
      headers: {},
      setHeader(name, value) {
        this.headers[name.toLowerCase()] = value;
      },
      status(code) {
        this.code = code;
        return this;
      },
      json(body) {
        resolve({ status: this.code, body, headers: this.headers });
      },
    };
    handler({ query: {}, socket: {}, ...req }, res, env);
  });
}

const env = { LOBBY_IP_HEADER: 'x-vercel-forwarded-for' };
const listing = { name: 'Ann', port: 47821, players: 1, max: 4, locked: false, started: false, version: '0.1.4' };

test('the address is taken from the header the platform sets, then from the connection', () => {
  assert.equal(addressOf({ headers: { 'x-vercel-forwarded-for': '93.184.216.34' }, socket: { remoteAddress: '10.0.0.1' } }, {}), '93.184.216.34');
  assert.equal(addressOf({ headers: { 'x-vercel-forwarded-for': ['93.184.216.34'] } }, {}), '93.184.216.34');
  assert.equal(addressOf({ headers: {}, socket: { remoteAddress: '93.184.216.34' } }, {}), '93.184.216.34');
  // (what the sender writes in other headers is not believed)
  assert.equal(addressOf({ headers: { 'x-forwarded-for': '1.2.3.4', 'x-real-ip': '1.2.3.4' }, socket: { remoteAddress: '93.184.216.34' } }, {}), '93.184.216.34');
  assert.equal(addressOf({ headers: { 'x-vercel-forwarded-for': '1.2.3.4' }, socket: { remoteAddress: '93.184.216.34' } }, { LOBBY_IP_HEADER: '' }), '93.184.216.34');
  assert.equal(addressOf({ headers: { 'x-edge-ip': '1.2.3.4' } }, { LOBBY_IP_HEADER: 'X-Edge-IP' }), '1.2.3.4');
});

test('the function lists a game and shows it', async () => {
  const headers = { 'x-vercel-forwarded-for': '93.184.216.34' };
  const made = await call({ method: 'POST', headers, body: listing }, env);
  assert.equal(made.status, 201);
  assert.equal(made.headers['cache-control'], 'no-store');
  const read = await call({ method: 'GET', headers: { 'x-vercel-forwarded-for': '8.8.8.8' } }, env);
  assert.equal(read.status, 200);
  assert.equal(read.body.games.length, 1);
  assert.equal(read.body.games[0].address, '93.184.216.34');
  assert.equal(read.body.store, 'memory');
  const gone = await call({ method: 'DELETE', headers, query: { port: '47821' } }, env);
  assert.deepEqual(gone.body, { removed: true });
});

test('a request with no address, or from a home one, is turned away', async () => {
  assert.equal((await call({ method: 'GET', headers: {} }, env)).status, 400);
  assert.equal((await call({ method: 'POST', headers: { 'x-vercel-forwarded-for': '192.168.1.5' }, body: listing }, env)).status, 400);
});
