//! what a player asks of the world in a frame, as plain data: where to walk, what to build, who to send squads at, which relic to take. `Controls` reads
//! it from the keyboard and mouse of whoever is at the machine, and the world's step (see `Sim`) takes it from anyone, so it is checked as it comes
//! in: it can be written to bytes and read back, and what is read back is only ever something a player could have asked for

use crate::prelude::*;

//the window a player is taken to have if nothing has been said, and the sizes that are believed
pub(crate) const DEFAULT_SCREEN: Vec2 = Vec2::new(1920.0, 1080.0);
pub(crate) const MIN_SCREEN: f32 = 320.0;
pub(crate) const MAX_SCREEN: f32 = 8192.0;

//the most squares one input can build on or delete from, a drag across the world and no more
pub(crate) const MAX_SQUARES_AN_INPUT: usize = 256;

//what is done about the relics on offer
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RelicAct {
    Pick(u8), //take the one in this place among the three
    Reroll,   //ask for three others
}

//a player choosing how one of their champions grows: which champion (by its id), at which of EVOLUTION_LEVELS, and which of the two ways
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct EvolvePick {
    pub(crate) champion: u32,
    pub(crate) tier: u8,
    pub(crate) choice: u8,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct PlayerInput {
    pub(crate) direction: Vec2,        //the way they walk: no longer than 1, zero to stand still
    pub(crate) screen: Vec2,           //how big their window is, which is how far a big army of theirs reaches for babies
    pub(crate) formation: Formation,   //the shape their army keeps
    pub(crate) select: Option<u8>,     //the kind of wall they chose to build (its place in BUILDABLE)
    pub(crate) place: Vec<(i32, i32)>, //the squares they built on
    pub(crate) delete: Vec<(i32, i32)>, //the squares they deleted from
    pub(crate) tower: Option<(i32, i32)>, //a tower they asked their units to man
    pub(crate) recall: bool,           //they called every squad back
    pub(crate) send: Option<u32>,      //the enemy (by its id) they sent a squad at
    pub(crate) relic: Option<RelicAct>,
    pub(crate) evolve: Option<EvolvePick>, //how they chose for a champion of theirs to grow
}

impl PlayerInput {
    //a player doing nothing: standing still, their window as it is and their army in the shape it was
    pub(crate) fn idle(formation: Formation, screen: Vec2) -> PlayerInput {
        PlayerInput { direction: Vec2::ZERO, screen, formation, select: None, place: Vec::new(), delete: Vec::new(), tower: None, recall: false, send: None, relic: None, evolve: None }
    }

    //the same thing with whatever couldn't be asked for put right: walking no faster than walking, a window of a believable size, no more squares than a drag
    //reaches, and a kind of wall that exists. (numbers that aren't numbers can't get this far, see `Reader::f32`)
    pub(crate) fn tidy(mut self) -> PlayerInput {
        self.direction = self.direction.clamp_length_max(1.0);
        self.screen = self.screen.clamp(Vec2::splat(MIN_SCREEN), Vec2::splat(MAX_SCREEN));
        self.select = self.select.filter(|&kind| (kind as usize) < BUILDABLE.len());
        self.place.truncate(MAX_SQUARES_AN_INPUT);
        self.delete.truncate(MAX_SQUARES_AN_INPUT);
        self.relic = self.relic.filter(|act| !matches!(act, RelicAct::Pick(place) if *place >= 3));
        self.evolve = self.evolve.filter(|pick| (pick.tier as usize) < EVOLUTION_LEVELS.len() && pick.choice < 2);
        self
    }

    //this input and the one after it as one, for a world that takes a step between two of them: what was asked for in either is asked for, and of two things
    //that can't both happen the later is the one that does (but not the relic or an evolution, as the first thing a player does about a choice on offer is what they meant)
    pub(crate) fn then(mut self, later: PlayerInput) -> PlayerInput {
        self.direction = later.direction;
        self.screen = later.screen;
        self.formation = later.formation;
        self.select = later.select.or(self.select);
        self.place.extend(later.place);
        self.delete.extend(later.delete);
        self.place.truncate(MAX_SQUARES_AN_INPUT);
        self.delete.truncate(MAX_SQUARES_AN_INPUT);
        self.tower = later.tower.or(self.tower);
        self.recall |= later.recall;
        self.send = later.send.or(self.send);
        self.relic = self.relic.or(later.relic);
        self.evolve = self.evolve.or(later.evolve);
        self
    }

    //whether a square is one this player could have reached: in the world, and in the window they have (zoomed all the way out, with a square to spare)
    pub(crate) fn reaches(&self, from: Vec2, tile: (i32, i32)) -> bool {
        if !(0..WORLD_TILES).contains(&tile.0) || !(0..WORLD_TILES).contains(&tile.1) {
            return false;
        }
        let apart = (tile_center(tile) - from).abs();
        let half = self.screen / MIN_ZOOM / 2.0 + Vec2::splat(TILE_SIZE);
        apart.x <= half.x && apart.y <= half.y
    }

    pub(crate) fn write(&self, out: &mut Writer) {
        out.vec2(self.direction);
        out.vec2(self.screen);
        out.u8(FORMATIONS.iter().position(|&formation| formation == self.formation).unwrap_or(0) as u8);
        out.flag(self.select.is_some());
        out.u8(self.select.unwrap_or(0));
        write_squares(out, &self.place);
        write_squares(out, &self.delete);
        out.flag(self.tower.is_some());
        let (col, row) = self.tower.unwrap_or((0, 0));
        out.i32(col);
        out.i32(row);
        out.flag(self.recall);
        out.flag(self.send.is_some());
        out.u32(self.send.unwrap_or(0));
        match self.relic {
            None => out.u8(0),
            Some(RelicAct::Pick(place)) => {
                out.u8(1);
                out.u8(place);
            }
            Some(RelicAct::Reroll) => out.u8(2),
        }
        out.flag(self.evolve.is_some());
        let pick = self.evolve.unwrap_or(EvolvePick { champion: 0, tier: 0, choice: 0 });
        out.u32(pick.champion);
        out.u8(pick.tier);
        out.u8(pick.choice);
    }

    pub(crate) fn read(input: &mut Reader) -> Option<PlayerInput> {
        let direction = input.vec2()?;
        let screen = input.vec2()?;
        let formation = *FORMATIONS.get(input.u8()? as usize)?;
        let (has_select, select) = (input.flag()?, input.u8()?);
        let place = read_squares(input)?;
        let delete = read_squares(input)?;
        let (has_tower, col, row) = (input.flag()?, input.i32()?, input.i32()?);
        let recall = input.flag()?;
        let (has_send, send) = (input.flag()?, input.u32()?);
        let relic = match input.u8()? {
            0 => None,
            1 => Some(RelicAct::Pick(input.u8()?)),
            2 => Some(RelicAct::Reroll),
            _ => return None,
        };
        let (has_evolve, champion, tier, choice) = (input.flag()?, input.u32()?, input.u8()?, input.u8()?);
        Some(PlayerInput {
            direction,
            screen,
            formation,
            select: has_select.then_some(select),
            place,
            delete,
            tower: has_tower.then_some((col, row)),
            recall,
            send: has_send.then_some(send),
            relic,
            evolve: has_evolve.then_some(EvolvePick { champion, tier, choice }),
        })
    }

    #[cfg(test)]
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut out = Writer::new();
        self.write(&mut out);
        out.bytes
    }

    //the input in a message, or None if it is cut short, garbled, or has anything left over after it
    #[cfg(test)]
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<PlayerInput> {
        let mut input = Reader::new(bytes);
        let read = PlayerInput::read(&mut input)?;
        input.is_done().then_some(read)
    }
}

//what a player at another machine sends the host every frame: what they asked for in the frame, numbered so the host can say which it has acted on, and
//which of the host's messages they have taken in, so the host can see how far behind they are
pub(crate) const UP_INPUT: u8 = 0x49;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct ClientFrame {
    pub(crate) seq: u32,      //1 for the first, one more each time
    pub(crate) tick_ack: u32, //how many of the host's messages they have taken in
    pub(crate) zoom: f32,     //how far in their camera is, which with their window is how much of the world they are sent
    pub(crate) input: PlayerInput,
}

impl ClientFrame {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut out = Writer::new();
        out.u8(UP_INPUT);
        out.u32(self.seq);
        out.u32(self.tick_ack);
        out.f32(self.zoom);
        self.input.write(&mut out);
        out.bytes
    }

    //the frame in a message, or None if it is cut short, garbled, or has anything left over after it. what is asked for is put right as far as it can be
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<ClientFrame> {
        let mut input = Reader::new(bytes);
        if input.u8()? != UP_INPUT {
            return None;
        }
        let (seq, tick_ack, zoom) = (input.u32()?, input.u32()?, input.f32()?);
        let frame = ClientFrame { seq, tick_ack, zoom: zoom.clamp(MIN_ZOOM, MAX_ZOOM), input: PlayerInput::read(&mut input)?.tidy() };
        input.is_done().then_some(frame)
    }
}

fn write_squares(out: &mut Writer, squares: &[(i32, i32)]) {
    let squares = &squares[..squares.len().min(MAX_SQUARES_AN_INPUT)];
    out.u16(squares.len() as u16);
    for &(col, row) in squares {
        out.i32(col);
        out.i32(row);
    }
}

fn read_squares(input: &mut Reader) -> Option<Vec<(i32, i32)>> {
    let count = input.u16()? as usize;
    if count > MAX_SQUARES_AN_INPUT {
        return None;
    }
    (0..count).map(|_| Some((input.i32()?, input.i32()?))).collect()
}

//what the player at this machine can see of the world, to point at things with the mouse: where the camera is, how far in it is, and the enemies
//(and the map they can be clicked on)
pub(crate) struct ControlView<'a> {
    pub(crate) position: Vec2,
    pub(crate) zoom: f32,
    pub(crate) screen: Vec2,
    pub(crate) formation: Formation,
    pub(crate) map: &'a Minimap,
    pub(crate) enemies: &'a [Enemy],
    pub(crate) over_roster: bool, //the mouse is over the roster of champions, whose clicks are its own
}

//the keyboard and mouse of the player at this machine: what they do becomes input. holds the two drags (building and deleting) that go on
//over many frames
#[derive(Default)]
pub(crate) struct Controls {
    place_drag: Option<Drag>,
    delete_drag: Option<Drag>,
}

impl Controls {
    pub(crate) fn new() -> Controls {
        Controls::default()
    }

    //what they ask for in a frame in which they can act: walking (WASD), recalling the squads (R), a click on an enemy (in the world or on the map)
    //to send a squad at it, G over a tower to man it, the number keys to choose a kind of wall, and dragging the left button to build and the right
    //to delete. a left click that is on an enemy, or on the map, builds nothing, nor does the drag it would have started, and nothing in the world under
    //the roster of champions is clicked, built on or deleted
    pub(crate) fn read(&mut self, view: &ControlView) -> PlayerInput {
        let mut input = PlayerInput::idle(view.formation, view.screen);
        input.direction = keyboard_direction();
        input.recall = is_key_pressed(KeyCode::R);
        let mouse = Vec2::from(mouse_position());
        let over_map = view.map.contains(mouse);
        let mut clicked_enemy = false;
        if is_mouse_button_pressed(MouseButton::Left) && !view.over_roster {
            if over_map {
                //(the map covers the world under it, so a click that misses every dot does nothing)
                input.send = minimap_enemy_at(view.map, mouse, view.enemies).map(|clicked| view.enemies[clicked].id);
                clicked_enemy = true;
            } else if let Some(clicked) = enemy_at(mouse_world(view.position.x, view.position.y, view.zoom), view.enemies, ENEMY_CLICK_MARGIN / view.zoom) {
                input.send = Some(view.enemies[clicked].id);
                clicked_enemy = true;
            }
        }
        let hovered = if over_map || view.over_roster { None } else { hovered_tile(view.position.x, view.position.y, view.zoom) };
        if is_key_pressed(KeyCode::G) {
            input.tower = hovered;
        }
        for (place, key) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5].into_iter().enumerate() {
            if is_key_pressed(key) {
                input.select = Some(place as u8);
            }
        }
        input.place = dragged_squares(MouseButton::Left, &mut self.place_drag, if clicked_enemy { None } else { hovered });
        input.delete = dragged_squares(MouseButton::Right, &mut self.delete_drag, hovered);
        input
    }
}

//what they do about the relics on offer: the number keys or a click on a card take one, R asks for others
pub(crate) fn read_relic_act(screen: Vec2) -> Option<RelicAct> {
    let key = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3].iter().position(|&key| is_key_pressed(key));
    let click = if is_mouse_button_pressed(MouseButton::Left) { relic_card_at(screen, Vec2::from(mouse_position())) } else { None };
    match key.or(click) {
        Some(place) => Some(RelicAct::Pick(place as u8)),
        None if is_key_pressed(KeyCode::R) => Some(RelicAct::Reroll),
        None => None,
    }
}

//which of the two ways a champion can grow they take: 1 or 2, or a click on a card
pub(crate) fn read_evolution_pick(screen: Vec2) -> Option<u8> {
    let key = [KeyCode::Key1, KeyCode::Key2].iter().position(|&key| is_key_pressed(key));
    let mouse = Vec2::from(mouse_position());
    let click = if is_mouse_button_pressed(MouseButton::Left) { choice_cards(screen, 2).iter().position(|card| card.contains(mouse)) } else { None };
    key.or(click).map(|place| place as u8)
}
