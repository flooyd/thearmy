# The Army lobby list

A small service that holds a list of the games people are hosting, so players can pick one instead of typing an address.

It is only a phone book. A host tells it "I'm open", and a player asks it "who is open?". The game itself never goes through it: once a player picks a game they connect **straight to the host**, as before. The host still needs the game's UDP port (47821 unless changed) forwarded on their router.

- Written for Vercel (functions in `api/`), keeping its list in an Upstash Redis database.
- No dependencies to install. Node 20 or newer.
- A game stays listed for 90 seconds after the host last said it was there (the game says so every 30 seconds), so a game that crashes drops off by itself.

## Deploying it on Vercel

You need a Vercel account (the free Hobby plan is plenty).

1. **Make a project from this folder.** In the Vercel dashboard choose *Add New → Project*, import the GitHub repository, and set **Root Directory** to `lobby-server`. Leave the framework as *Other* and the build settings empty. Deploy.
   (Or from a terminal: `cd lobby-server`, `npx vercel`, and answer the questions.)
2. **Give it a database.** In the project, open *Storage* (or *Marketplace*) and add **Upstash Redis** (the free plan is plenty). Connect it to the project for all environments. This adds the environment variables `KV_REST_API_URL` and `KV_REST_API_TOKEN` (or `UPSTASH_REDIS_REST_URL` and `UPSTASH_REDIS_REST_TOKEN`), which the service picks up by itself.
3. **Deploy again** (*Deployments → … → Redeploy*) so the new variables are in use.
4. **Check it.** Open `https://<your-project>.vercel.app/api/games`. You should see `{"games":[],…,"store":"upstash"}`. If it says `"store":"memory"`, the database is not connected yet: with `memory` every copy of the function has its own list, and the list will look random.
5. **Tell the game.** The game has its address built in: `LOBBY_SERVER` in `src/directory.rs` (it is `https://lobby-server-virid.vercel.app`). To run a list of your own, change it there and build the game. To try one without building, start the game with the environment variable `THE_ARMY_LOBBY` set to its address.

Everyone who should see each other's games has to use the same lobby server, which is why it is not something a player types.

### Notes

- **Where a game is listed.** The service lists a game at the address the request came *from*, never at one the host writes down, so nobody can list somebody else's machine. On Vercel that comes from the `x-vercel-forwarded-for` header, which Vercel sets itself. If you run this anywhere else, set `LOBBY_IP_HEADER` to the header your proxy sets (and make sure a player cannot write it themselves), or set it empty to use the connection's own address.
- **Home addresses are refused** (10.x, 192.168.x, 127.x and the like), because nobody else could reach them. To try it on one machine, use `npm run dev` (below) or set `LOBBY_ALLOW_PRIVATE=1`.
- **Limits** (in `lib/lobby.js`): 200 games at once, 3 per address, 20 changes a minute and 60 readings a minute per address.
- **Cost.** A game being hosted costs about 4 database commands every 30 seconds. Upstash's free plan allows hundreds of thousands a month.
- **Names** shown in the list are cut down to letters, numbers, spaces and `- _ ' .`, up to 16 of them, the same as in the game.
- It tells nothing but what the host chose to say: their name, the port, how many are in, whether the run has started, and the game's version. The list does show a host's public address to anyone who asks, which is the point of it; that is why putting a game on the list is a switch in the game that the host turns on.

## The interface

`GET /api/games` gives

```json
{ "games": [ { "id": "83f9eb132fe4221a", "name": "Ann", "address": "93.184.216.34", "port": 47821,
               "players": 1, "max": 4, "locked": true, "started": false, "version": "0.2.4", "age": 12 } ],
  "now": 1700000000000, "ttl": 90, "store": "upstash" }
```

`POST /api/games` with a JSON body `{ "name", "port", "players", "max", "locked", "started", "version" }` lists a game, or says again that it is still there. It answers `201` (new) or `200` with `{ "id", "ttl", "address" }`.

`DELETE /api/games?port=47821` takes the game on that port, at the address the request comes from, off the list.

Errors are `{ "error": "…" }` with a status of `400` (what was sent is wrong), `429` (too many requests, or too many games from one address; see `Retry-After`), or `503` (the list is full).

## Trying it on this machine

```
cd lobby-server
npm test        # the tests
npm run dev     # the service on http://127.0.0.1:8787, listing games from this machine too
```

Then start the game with `THE_ARMY_LOBBY=http://127.0.0.1:8787` set, so that it uses that one (host in one window, and join from the list in another). To run the game's own end-to-end check against it:

```
THE_ARMY_TEST_LOBBY=http://127.0.0.1:8787 cargo test -- --ignored the_real_lobby_server
```
