// The lobby list on this machine, for trying the game against it: `npm run dev` (or `node dev.js [port]`).
// Games from this machine are listed too, which the real thing refuses (a home address is no use to anyone else). a request can also say
// which address it comes from in an x-dev-address header, to fill the list with games from elsewhere.

import { createServer } from 'node:http';
import handler from './api/games.js';

const port = Number(process.argv[2] ?? process.env.PORT ?? 8787);
const env = { ...process.env, LOBBY_ALLOW_PRIVATE: '1', LOBBY_IP_HEADER: 'x-dev-address' };

createServer((req, res) => {
  const url = new URL(req.url, 'http://localhost');
  if (url.pathname !== '/api/games') {
    res.writeHead(404).end('not here');
    return;
  }
  req.query = Object.fromEntries(url.searchParams);
  let text = '';
  req.on('data', (chunk) => (text += chunk));
  req.on('end', () => {
    try {
      req.body = text ? JSON.parse(text) : undefined;
    } catch {
      res.writeHead(400, { 'Content-Type': 'application/json' }).end('{"error":"that is not JSON"}');
      return;
    }
    // (the same helpers Vercel gives a function)
    res.status = (code) => {
      res.statusCode = code;
      return res;
    };
    res.json = (body) => res.setHeader('Content-Type', 'application/json').end(JSON.stringify(body));
    handler(req, res, env);
  });
}).listen(port, '127.0.0.1', () => console.log(`the lobby list is at http://127.0.0.1:${port}  (games from this machine are allowed)`));
