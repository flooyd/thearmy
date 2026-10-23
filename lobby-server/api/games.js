// GET    /api/games            the games that are listed
// POST   /api/games            list a game, or say again that it is still there (see lib/lobby.js for what to send)
// DELETE /api/games?port=47821 take the game on that port (of the address the request comes from) off the list

import { handle } from '../lib/lobby.js';
import { storeFromEnv } from '../lib/store.js';

let store; // kept from one request to the next while this copy of the function stays warm

// where a request came from. Vercel sets this header itself and throws away whatever the sender wrote in it. anywhere else, name the header
// your proxy sets in LOBBY_IP_HEADER, or leave it empty to use the connection itself
export function addressOf(req, env = process.env) {
  const header = env.LOBBY_IP_HEADER ?? 'x-vercel-forwarded-for';
  const sent = header ? req.headers[header.toLowerCase()] : undefined;
  return (Array.isArray(sent) ? sent[0] : sent) ?? req.socket?.remoteAddress;
}

export default async function handler(req, res, env = process.env) {
  store ??= storeFromEnv(env);
  res.setHeader('Cache-Control', 'no-store');
  res.setHeader('Access-Control-Allow-Origin', '*');
  try {
    const answer = await handle(
      { method: req.method, address: addressOf(req, env), query: req.query ?? {}, body: req.body, now: Date.now() },
      store,
      { allowPrivate: env.LOBBY_ALLOW_PRIVATE === '1' },
    );
    for (const [name, value] of Object.entries(answer.headers)) res.setHeader(name, value);
    res.status(answer.status).json(answer.body);
  } catch (error) {
    console.error(error);
    res.status(500).json({ error: 'the lobby could not answer' });
  }
}
