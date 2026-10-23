// Where the lobby list keeps what it knows. Two kinds:
//   memory   - in this process only. fine for trying it out (npm run dev) and for tests; on Vercel every request may land on a
//              different copy of the function, so it must not be used there
//   upstash  - a Redis database reached over HTTPS (the Upstash Redis that Vercel's Marketplace sets up). what a real deployment uses
//
// Both give the same four things:
//   all(now)        every game that is still listed (those past their time are dropped on the way)
//   put(game)       list a game, or replace its entry
//   remove(id)      take a game off
//   hit(key, secs)  count one more use of `key` and say how many there have been, the count going away `secs` after it was started

export function memoryStore() {
  const games = new Map();
  const counts = new Map();
  return {
    kind: 'memory',
    async all(now) {
      for (const [id, game] of games) if (game.expires <= now) games.delete(id);
      return [...games.values()];
    },
    async put(game) {
      games.set(game.id, game);
    },
    async remove(id) {
      games.delete(id);
    },
    async hit(key, seconds) {
      const now = Date.now();
      for (const [name, entry] of counts) if (entry.until <= now) counts.delete(name);
      const entry = counts.get(key) ?? { count: 0, until: now + seconds * 1000 };
      entry.count += 1;
      counts.set(key, entry);
      return entry.count;
    },
  };
}

const GAMES_KEY = 'army:games';

// Upstash's REST interface: a list of commands in, a list of { result } or { error } out
export function upstashStore({ url, token, fetch: send = globalThis.fetch }) {
  const base = url.replace(/\/+$/, '');
  async function run(commands) {
    const response = await send(`${base}/pipeline`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify(commands),
    });
    if (!response.ok) throw new Error(`the database said ${response.status}`);
    const results = await response.json();
    const broken = results.find((entry) => entry.error);
    if (broken) throw new Error(`the database said: ${broken.error}`);
    return results.map((entry) => entry.result);
  }
  return {
    kind: 'upstash',
    async all(now) {
      const [flat] = await run([['HGETALL', GAMES_KEY]]);
      // (a hash comes back as a flat list of names and values, or as an object, depending on the protocol in use)
      const pairs = Array.isArray(flat) ? Array.from({ length: flat.length / 2 }, (_, i) => [flat[2 * i], flat[2 * i + 1]]) : Object.entries(flat ?? {});
      const live = [];
      const stale = [];
      for (const [id, text] of pairs) {
        let game = null;
        try {
          game = JSON.parse(text);
        } catch {
          // (a mangled entry is dropped like an old one)
        }
        if (game && game.expires > now) live.push(game);
        else stale.push(id);
      }
      if (stale.length) await run([['HDEL', GAMES_KEY, ...stale]]);
      return live;
    },
    async put(game) {
      await run([['HSET', GAMES_KEY, game.id, JSON.stringify(game)]]);
    },
    async remove(id) {
      await run([['HDEL', GAMES_KEY, id]]);
    },
    async hit(key, seconds) {
      const [count] = await run([['INCR', `army:hits:${key}`], ['EXPIRE', `army:hits:${key}`, String(seconds)]]);
      return Number(count);
    },
  };
}

// the store the environment asks for: Upstash if it has been given one (under either of the names Vercel uses), otherwise memory
export function storeFromEnv(env) {
  const url = env.UPSTASH_REDIS_REST_URL ?? env.KV_REST_API_URL;
  const token = env.UPSTASH_REDIS_REST_TOKEN ?? env.KV_REST_API_TOKEN;
  return url && token ? upstashStore({ url, token }) : memoryStore();
}
