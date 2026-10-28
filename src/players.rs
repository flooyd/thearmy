//! the players: each has a name, an army, relics, a store of wall points and squads of their own, and walks the one world

use crate::prelude::*;

//the colours of the players, in the order they join: the first is the blue the player has always been
pub(crate) const PLAYER_COLORS: [Color; MAX_PLAYERS] = [BLUE, Color::new(1.0, 0.6, 0.2, 1.0), Color::new(0.92, 0.42, 0.88, 1.0), Color::new(0.95, 0.9, 0.3, 1.0)];

//the longest a name can be, in characters
pub(crate) const MAX_NAME_LENGTH: usize = 16;

pub(crate) struct Player {
    pub(crate) name: String,
    pub(crate) color: Color,
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2, //pixels per second, how they have been moving lately, smoothed
    pub(crate) facing: Vec2,   //the way the army's formation is turned
    pub(crate) formation: Formation,
    pub(crate) upgrades: Upgrades,
    pub(crate) rerolls: u32,
    pub(crate) choice: Option<RelicChoice>, //the relics on offer to them
    pub(crate) owed: Vec<RelicChoice>,        //the choices they have won while choosing, to come up one by one
    pub(crate) mason: Mason,
    pub(crate) squads: Squads,
    pub(crate) route: ArmyRoute, //the way their army follows them
    pub(crate) foes: FoeRoute,   //the way the enemies come at them
    pub(crate) salvo_wait: f32,  //seconds until their opening salvo can go off again
    pub(crate) held: Option<Vec<Unit>>, //their army while they are away: out of the world, where nothing can hurt it, for them to have back (see `Sim::hold`)
}

impl Player {
    //a player as another machine shows them: with nothing of the way their army and the enemies find them
    pub(crate) fn viewing(slot: usize, name: &str) -> Player {
        Player {
            name: clean_name(name),
            color: PLAYER_COLORS[slot.min(MAX_PLAYERS - 1)],
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            facing: Vec2::new(0.0, -1.0),
            formation: Formation::Ring,
            upgrades: Upgrades::default(),
            rerolls: 0,
            choice: None,
            owed: Vec::new(),
            mason: Mason::new(),
            squads: Squads::new(),
            route: ArmyRoute::blank(),
            foes: FoeRoute::blank(),
            salvo_wait: 0.0,
            held: None,
        }
    }

    //a player at a place, with the army they start with still to be made (see `starting_units`)
    pub(crate) fn new(slot: usize, name: &str, position: Vec2, walls: &Walls) -> Player {
        Player {
            name: clean_name(name),
            color: PLAYER_COLORS[slot.min(MAX_PLAYERS - 1)],
            position,
            velocity: Vec2::ZERO,
            facing: Vec2::new(0.0, -1.0),
            formation: Formation::Ring,
            upgrades: Upgrades::default(),
            rerolls: rerolls_earned(0),
            choice: None,
            owed: Vec::new(),
            mason: Mason::new(),
            squads: Squads::new(),
            route: ArmyRoute::new(position, walls),
            foes: FoeRoute::new(position, walls),
            salvo_wait: 0.0,
            held: None,
        }
    }

    //one frame of walking in a direction (a unit vector, or zero to stand still) at `speed` pixels a second, round the walls
    pub(crate) fn walk(&mut self, direction: Vec2, speed: f32, dt: f32, walls: &Walls) {
        self.position = walk_player(self.position, direction, speed, dt, walls);
    }

    //after the frame's moving is all done (a gravity well can move them too): how they have been moving, and so the way the army faces
    pub(crate) fn settle(&mut self, was: Vec2, dt: f32) {
        self.velocity = smoothed_velocity(self.velocity, was, self.position, dt);
        self.facing = turn_to_face(self.facing, self.velocity, dt);
    }

    //a choice of relics for them: they are offered it now, or when they have chosen from the ones on offer. the world does not stand still for
    //it when there are others in it, so a second one can be won before the first is taken
    pub(crate) fn offer(&mut self, choice: RelicChoice) {
        if self.choice.is_none() {
            self.choice = Some(choice);
        } else {
            self.owed.push(choice);
        }
    }

    //they have taken a relic: the next choice they are owed comes up, if there is one
    pub(crate) fn chosen(&mut self) {
        self.choice = if self.owed.is_empty() { None } else { Some(self.owed.remove(0)) };
    }

    //whether they are away: not at a machine, with their army held for them
    pub(crate) fn is_away(&self) -> bool {
        self.held.is_some()
    }

    pub(crate) fn drill(&self) -> Drill {
        Drill::new(self.formation, self.facing)
    }
}

//a name as it is kept: without the spaces at the ends or anything that isn't a letter, a number, a space or a dash, an apostrophe or a
//full stop, and no longer than MAX_NAME_LENGTH
pub(crate) fn clean_name(name: &str) -> String {
    let kept: String = name.chars().filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '\'' | '.')).collect();
    kept.trim().chars().take(MAX_NAME_LENGTH).collect::<String>().trim_end().to_owned()
}

//whether a name can be used: it has something in it, and it isn't what one of the others is called (capitals don't make a difference)
pub(crate) fn name_is_free(name: &str, taken: &[Player]) -> bool {
    let name = clean_name(name);
    !name.is_empty() && !taken.iter().any(|player| player.name.eq_ignore_ascii_case(&name))
}

//everyone in the world. a player's place in the list is the number their units are marked with (`Unit::owner`)
pub(crate) struct Players {
    pub(crate) list: Vec<Player>,
}

impl Players {
    //a world with one player in it
    pub(crate) fn solo(player: Player) -> Players {
        Players { list: vec![player] }
    }

    pub(crate) fn len(&self) -> usize {
        self.list.len()
    }

    //the player at a place in the list, or the last one if there is nobody there (a unit marked for someone who has gone)
    pub(crate) fn get(&self, owner: usize) -> &Player {
        &self.list[owner.min(self.list.len() - 1)]
    }

    //a new player joins, if there is room and the name is free. returns their place in the list, which is who their units belong to
    pub(crate) fn join(&mut self, name: &str, position: Vec2, walls: &Walls) -> Option<usize> {
        if self.list.len() >= MAX_PLAYERS || !name_is_free(name, &self.list) {
            return None;
        }
        self.list.push(Player::new(self.list.len(), name, position, walls));
        Some(self.list.len() - 1)
    }

    //a boss has fallen, which is a win for all of them: each has a choice of relics and a chance of a champion (`bosses` is how many have been
    //slain, this one with them), and another reroll for every third boss
    pub(crate) fn share_boss(&mut self, bosses: usize, tier: u32, fx: &mut Effects) {
        let third = bosses % 3 == 0;
        for (i, player) in self.list.iter_mut().enumerate() {
            if third {
                player.rerolls += 1;
            }
            player.offer(RelicChoice::new());
            if boss_drops_champion(bosses, rand::gen_range(0.0, 1.0)) {
                fx.recruits.push((player.position, Odds::Boss(tier), i));
            }
        }
    }

    //the citadel has fallen, a win for all of them: each is sent a champion, and with others in the world, where there is no victory screen to
    //wait for, a rare relic as well
    pub(crate) fn share_citadel(&mut self, fx: &mut Effects) {
        let several = self.list.len() > 1;
        for (i, player) in self.list.iter_mut().enumerate() {
            fx.recruits.push((player.position, Odds::Citadel, i));
            if several {
                player.offer(RelicChoice::rare());
            }
        }
    }

    //whose relics the towers fire with, which no one player built: the best archer's, so that every player is better for the others being there
    pub(crate) fn archer(&self) -> &Upgrades {
        let strength = |player: &&Player| player.upgrades.hit();
        &self.list.iter().max_by(|a, b| strength(a).total_cmp(&strength(b))).unwrap_or(&self.list[0]).upgrades
    }

    //the names of the players who are away, whose armies are held for them
    pub(crate) fn away_names(&self) -> Vec<&str> {
        self.list.iter().skip(1).filter(|player| player.is_away()).map(|player| player.name.as_str()).collect()
    }

    //where the players are
    pub(crate) fn positions(&self) -> Vec<Vec2> {
        self.list.iter().map(|player| player.position).collect()
    }

    //the numbers the game sets for everyone every frame, from the settings: how big an army can be (a world with several players holds
    //them all to the same, whatever the setting says), and how much of the damage the relics add the enemies make up for
    pub(crate) fn apply_settings(&mut self, army_limit: usize, foe_answer: f32) {
        let limit = if self.list.len() > 1 { MULTIPLAYER_ARMY_LIMIT } else { army_limit };
        for player in &mut self.list {
            player.upgrades.army_limit = limit;
            player.upgrades.foe_answer = foe_answer;
        }
    }
}

//each player's relics, for what hurts or helps their units
impl Relics for Players {
    fn of(&self, owner: u8) -> &Upgrades {
        &self.get(owner as usize).upgrades
    }
}
