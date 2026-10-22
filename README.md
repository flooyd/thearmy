# The Army

A top-down game about a growing army. You are one dot at the centre of a few thousand of them, and the world is a
1000 x 1000 tile field full of things that want to stop you: nests, raiding hordes, and eight bosses (the last one, the Sovereign,
comes back after you fell it). Pick up the babies that enemies drop, grow the army to the thousands, point it at whatever is
in the way, and wall in whatever you can't fight.

Written in Rust with [macroquad](https://macroquad.rs). Every sound and the music are synthesised in code, so the only assets
are the fonts.

![A ring of 6,000 units in the middle of a horde fight](docs/screenshots/horde-army.jpg)

## What you do

- **Grow.** Enemies drop babies, villages are freed into more army, and barracks you take send recruits and sometimes a champion.
  The army fights on its own around you. Walk where you want it to be.
- **Send squads.** Click an enemy to send part of the army at it, sized to what it takes to win. `R` recalls them. Three
  formations (`F`) change how the rest of the army follows you: a ring with you safe in the middle, a wedge, or a line.
- **Survive bosses.** Seven bosses with several phases each, plus the Sovereign. They raise barriers, call meteors, charge,
  slam and mark the ground in red. When you hear "get out of the red", walk. Slaying one gives you a choice of relics.
- **Break hordes.** Hordes (Swarm Tide, Iron Column, Wildfire, Stampede, Titan March and more) are announced on screen and
  an arrow shows where they come from. Break one and it drops a bronze, silver or gold chest: wall points, recruits, a
  champion and, from gold, a choice of relic.
- **Collect champions.** Special units won as loot: Juggernaut, Medic, Pyromancer, Stormcaller, Phoenix, Frostweaver and
  eight more, each with a power of its own. A second of a kind merges into the first and raises its level.
- **Build.** Earn wall points by destroying enemies, then spend them on walls, gates, towers you can man, spikes and tar.
  Enemies path around what you build, and walls decide where a fight happens.
- **Pick relics.** Whetstone, Iron Resolve, Magnet, Quarry, Muster, Bloodlust, Aegis, Freezing (frozen enemies shatter into shards) and many more stack across the run.
- **Combos.** Chain kills for a MASSACRE, then a RAMPAGE, and watch the numbers climb.

The game is built for a lot of units: 15,000 by default (the settings take it anywhere from 5,000 to 50,000), drawn as instanced circles with the army's
work shared over all your processor cores. It runs at your display's refresh rate (it was developed on a 120 Hz screen).

| | |
|---|---|
| ![Fighting The Eclipse](docs/screenshots/boss-eclipse.jpg) | ![MASSACRE combo in a horde](docs/screenshots/horde-massacre.jpg) |
| The Eclipse, one of seven bosses | A MASSACRE combo with three hordes closing in |

![A gold chest and its relics](docs/screenshots/gold-chest.jpg)

## Controls

| Key | Does |
|---|---|
| `W A S D` | move |
| mouse wheel | zoom |
| click an enemy | send a squad |
| `R` | recall the squads |
| `F` | change the formation |
| click, drag | build walls |
| right click | delete a wall |
| `1` - `5` | what to build |
| `G` | man a tower |
| `Tab` | minimap: near or world |
| `M` / `N` | mute / music on or off |
| `F5` / `F9` | quick save / quick load |
| `F11` | fullscreen |
| `Space` | skip a boss's entrance |
| `1 2 3` | choose a relic |
| `Esc` | menu (settings, controls, save, load) |

The game also saves on its own, and the title screen offers to carry on from where you left off.

## Download

Builds for Windows and Linux are attached to each
[release](https://github.com/flooyd/thearmy/releases). They need nothing installed.

**Windows:** unzip it and run `TheArmy.exe`. Windows may say "Windows protected your PC" because the program isn't
signed. Click "More info", then "Run anyway". (Or right-click the zip, choose Properties, tick Unblock, and extract it after that.)

**Linux:** extract the tarball and run `./TheArmy`. It needs the usual desktop libraries; on Arch and CachyOS:

```sh
sudo pacman -S --needed alsa-lib libx11 libxi mesa
```

Saves and settings live in `%APPDATA%\TheArmy` on Windows and `~/.local/share/TheArmy` on Linux.

## Build from source

You need [Rust](https://rustup.rs) (edition 2024, so a recent stable toolchain).

```sh
cargo run --release
```

On Linux you also need the development libraries:

```sh
# Debian / Ubuntu
sudo apt install libasound2-dev libx11-dev libxi-dev libgl1-mesa-dev pkg-config
```

Use `--release`: a debug build is much slower with thousands of units. Set `THE_ARMY_WINDOWED=1` to start in a window
instead of filling the screen.

```sh
cargo test --release
```

The code is one file for each part of the game; `src/main.rs` has the map of them, and every tuning constant is in
`src/config.rs`. Pushing a tag like `v0.1.0` makes GitHub Actions build the Windows and Linux downloads and publish a release.

## Credits

The fonts are [Audiowide](https://fonts.google.com/specimen/Audiowide) and
[Chakra Petch](https://fonts.google.com/specimen/Chakra+Petch), both under the SIL Open Font License (the licenses are in
`assets/fonts`).
