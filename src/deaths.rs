//! learning from what goes wrong. what killed the army: each unit that dies is put down to what killed it (`Unit::bane`, a number standing for a kind
//! of enemy, a boss or one of the kinds of a boss's attacks, see `cause_of`), the run keeps a count of each (`Run::killers`), and the defeat screen
//! lists the worst of them. and the bestiary: every kind of enemy, boss, boss's attack and elite's affix the player has met, kept from run to run in
//! bestiary.txt beside the saves, each with what it does and what to do about it, on a page of the menu

use crate::prelude::*;
use std::sync::Mutex;

//every name a cause of death can have, in the order of their numbers from 1 (0 is nothing known): the kinds of enemy, the bosses, the kinds of attack,
//and lava (see biomes.rs)
fn cause_names() -> impl Iterator<Item = &'static str> {
    ENEMY_KINDS.iter().map(|kind| kind.name).chain(BOSSES.iter().map(|boss| boss.name)).chain([FINAL_BOSS.name]).chain(CIRCLES.iter().map(|circle| circle.name())).chain([Biome::Lava.name()])
}

//the number a cause of death has, from its name: 0 for one that is not known
pub(crate) fn cause_of(name: &str) -> u16 {
    cause_names().position(|known| known == name).map_or(0, |at| at as u16 + 1)
}

//a cause of death as it is shown: a kind's or a boss's name, or an attack's in the case of a name ("ECLIPSE SWEEP" is "Eclipse Sweep")
pub(crate) fn cause_label(cause: u16) -> String {
    match cause_names().nth((cause as usize).wrapping_sub(1)) {
        Some(name) if name.chars().any(|letter| letter.is_lowercase()) => name.to_owned(),
        Some(name) => in_title_case(name),
        None => "unknown".to_owned(),
    }
}

fn in_title_case(name: &str) -> String {
    name.split(' ')
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or(String::new(), |first| first.to_string() + &letters.as_str().to_lowercase())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

//the units that died, by what killed them, added to the run's count
pub(crate) fn tally_killers(killers: &mut Vec<(u16, usize)>, banes: &[u16]) {
    for &bane in banes {
        match killers.iter_mut().find(|(cause, _)| *cause == bane) {
            Some(entry) => entry.1 += 1,
            None => killers.push((bane, 1)),
        }
    }
}

//the `most` that killed the most, the worst first, as what to show and how many
pub(crate) fn worst_killers(killers: &[(u16, usize)], most: usize) -> Vec<(String, usize)> {
    let mut sorted: Vec<(u16, usize)> = killers.to_vec();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    sorted.into_iter().take(most).map(|(cause, count)| (cause_label(cause), count)).collect()
}

// ---------- the bestiary ----------

//the names of all that has been met, kept in bestiary.txt in the folder of the saves, read the first time it is needed
static MET: Mutex<Option<HashSet<String>>> = Mutex::new(None);
pub(crate) static BESTIARY_PAGE: AtomicUsize = AtomicUsize::new(0);
pub(crate) const BESTIARY_PER_PAGE: usize = 8;

fn bestiary_path() -> std::path::PathBuf {
    //(the tests keep theirs apart, so they never touch the player's)
    if cfg!(test) {
        return std::env::temp_dir().join("the-army-tests-bestiary.txt");
    }
    Saves::in_home().folder().join("bestiary.txt")
}

fn with_met<T>(work: impl FnOnce(&mut HashSet<String>) -> T) -> T {
    let mut held = MET.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let met = held.get_or_insert_with(|| std::fs::read_to_string(bestiary_path()).map(|text| text.lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_owned).collect()).unwrap_or_default());
    work(met)
}

#[cfg(test)]
pub(crate) fn has_met(name: &str) -> bool {
    with_met(|met| met.contains(name))
}

//what is met now goes in the bestiary, and the file is written when it holds something new
pub(crate) fn meet<'a>(names: impl Iterator<Item = &'a str>) {
    let grew = with_met(|met| {
        let before = met.len();
        for name in names {
            if !met.contains(name) {
                met.insert(name.to_owned());
            }
        }
        (met.len() > before).then(|| {
            let mut list: Vec<&String> = met.iter().collect();
            list.sort();
            list.into_iter().map(|name| format!("{name}\n")).collect::<String>()
        })
    });
    if let Some(text) = grew {
        let path = bestiary_path();
        let written = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&path, text));
        if let Err(error) = written {
            eprintln!("could not keep the bestiary in {}: {error}", path.display());
        }
    }
}

//the names of what is in view to be met: the enemies (and the affixes of the elites) and the kinds of a boss's attacks on the ground
pub(crate) fn meet_what_is_seen(enemies: &[Enemy], hazards: &[Hazard], view: Rect) {
    let mut names: Vec<&'static str> = Vec::new();
    for enemy in enemies.iter().filter(|enemy| !enemy.lurking() && view.contains(enemy.position)) {
        names.push(enemy.name);
        if let Some(first) = Affix::of(enemy.modifier) {
            names.push(first.label());
        }
        names.extend(enemy.affixes.list().map(Affix::label));
    }
    names.extend(hazards.iter().filter(|hazard| hazard.armed && hazard.circle.is_some() && view.overlaps(&Rect::new(hazard.position.x - hazard.radius, hazard.position.y - hazard.radius, hazard.radius * 2.0, hazard.radius * 2.0))).filter_map(|hazard| hazard.circle.map(BossCircle::name)));
    names.sort_unstable();
    names.dedup();
    meet(names.into_iter());
}

//a line of the bestiary: what it is called, how it is shown, its colour, what it does and what to do about it, and what sort of thing it is
pub(crate) struct Lesson {
    pub(crate) name: &'static str,
    pub(crate) shown: String,
    pub(crate) color: Color,
    pub(crate) lesson: String,
    pub(crate) sort: &'static str,
}

//every line of the bestiary, met or not: the enemies, the bosses, the bosses' attacks and the elites' affixes
pub(crate) fn lessons() -> Vec<Lesson> {
    let mut all: Vec<Lesson> = ENEMY_KINDS.iter().map(|kind| Lesson { name: kind.name, shown: kind.name.to_owned(), color: kind.color, lesson: kind_lesson(kind.name).to_owned(), sort: "an enemy" }).collect();
    all.extend(BOSSES.iter().chain([&FINAL_BOSS]).map(|boss| Lesson { name: boss.name, shown: boss.name.to_owned(), color: boss.color, lesson: boss_lesson(boss.name).to_owned(), sort: "a boss" }));
    all.extend(CIRCLES.iter().map(|circle| Lesson { name: circle.name(), shown: in_title_case(circle.name()), color: circle.color(), lesson: format!("{} ({})", circle.advice(), circle.colour_name().to_lowercase()), sort: "a boss's attack" }));
    all.extend(REACTIONS.iter().map(|reaction| Lesson { name: reaction.word(), shown: in_title_case(reaction.word()), color: reaction.color(), lesson: reaction.lesson().to_owned(), sort: "a reaction" }));
    all.extend(AFFIXES.iter().map(|affix| Lesson { name: affix.label(), shown: affix.label().to_owned(), color: affix.color(), lesson: format!("an elite that {}", affix_phrase(*affix)), sort: "an elite's affix" }));
    all
}

fn affix_phrase(affix: Affix) -> &'static str {
    match affix {
        Affix::Swift => "is fast: meet it, don't run",
        Affix::Armored => "shrugs off half the damage: bring more",
        Affix::Volatile => "explodes when destroyed: keep the champions clear",
        Affix::Frenzied => "hits twice as hard: don't stand in it",
        Affix::Vampiric => "heals from the harm it does: kill it quickly",
        Affix::Thorned => "hurts the units that hurt it: use the bolts",
        Affix::Teleporting => "jumps at the army: be ready",
        Affix::Magnetic => "drags the units near it in: keep away",
        Affix::Blazing => "burns the units near it: keep away",
        Affix::Frigid => "slows the units near it: keep away",
        Affix::Conjuring => "calls up imps: kill it first",
        Affix::Shielded => "has a shield that grows back: keep hitting",
        Affix::Regenerating => "mends itself: don't let it rest",
    }
}

fn kind_lesson(name: &str) -> &'static str {
    match name {
        "Grunt" => "the ordinary one: runs at the army when it sees it",
        "Scout" => "quick and weak: comes at the army from far off",
        "Brute" => "big and tough: send a squad, or a tank champion",
        "Titan" => "huge, and worth many babies: keep the army on it",
        "Colossus" => "enormous: wear it down with the whole army",
        "Spitter" => "spits from afar: close in on it",
        "Swarmer" => "comes in packs round the sides: hold the ring",
        "Bulwark" => "a shield that grows back: keep hitting it",
        "Splitter" => "splits when destroyed: finish the pieces",
        "Charger" => "stops, then dashes: step out of its line",
        "Bomber" => "runs at the army and goes off: catch it early",
        "Healer" => "mends the others from afar: hunt it first",
        "Summoner" => "calls up imps: kill it, not the imps",
        "Mortar" => "lobs shells that land later: keep moving",
        "Lancer" => "a bolt that goes through: keep out of its lane",
        "Grenadier" => "grenades hurt all round: spread out",
        "Seeker" => "orbs that home in: kill it, don't run",
        "Ambusher" => "lies hidden, and springs with its pack",
        "Imp" => "called up by others: they die with their maker",
        "Splitling" => "what a splitter breaks into",
        "Drone" => "what a nest sends out",
        "Shard" => "a piece of something frozen",
        "Nest" => "sends out more of its own: destroy it",
        "Executioner" => "winds up a cleave: step out of the cone",
        "Sniper" => "shows a long line, then fires: step out of it",
        "Hexer" => "curses the ground: get out of the purple",
        "Blinker" => "flickers and is behind the army: turn and fight",
        "Sapper" => "lays faint mines: walk round them",
        "Overseer" => "marks a champion for a blow: move the champion",
        _ => "",
    }
}

fn boss_lesson(name: &str) -> &'static str {
    match name {
        "The Hive Mother" => "broods, fire and acid rain: keep moving",
        "The Iron Warden" => "an armoured front, pistons and a shield rush: flank it",
        "Warlord Grakk" => "charges, cleaves, leaps and whirls: read where it goes",
        "The Eclipse" => "novas, beams, a sweep and a black hole: find the gaps",
        "The Swarm Queen" => "tears in two, swarms and twin slams: split up",
        "The Singularity" => "wells and gravity rings: stand close, then run",
        "The Bastion" => "walls itself in and fires lines of shells: break in",
        "The Sovereign" => "the last of them, and its Judgement: learn the order",
        _ => "",
    }
}

//how many pages the bestiary has, and the one it is on
pub(crate) fn bestiary_pages() -> usize {
    lessons().len().div_ceil(BESTIARY_PER_PAGE)
}

pub(crate) fn bestiary_page() -> usize {
    BESTIARY_PAGE.load(Ordering::Relaxed) % bestiary_pages().max(1)
}

pub(crate) fn turn_bestiary(direction: i32) {
    let pages = bestiary_pages().max(1) as i32;
    BESTIARY_PAGE.store((bestiary_page() as i32 + direction).rem_euclid(pages) as usize, Ordering::Relaxed);
}

//the page of the bestiary the menu is on, in the menu's panel from `top` down: a line for each, in its colour if it has been met and as ??? if not
pub(crate) fn draw_bestiary(panel: Rect, top: f32) {
    let all = lessons();
    let met = with_met(|met| met.clone());
    let known = all.iter().filter(|lesson| met.contains(lesson.name)).count();
    let note = format!("{known} of {} met", all.len());
    draw_label_right(&note, panel.x + panel.w - 20.0, panel.y + top - 6.0, 13.0, MUTED, Face::Body);
    for (i, lesson) in all.iter().skip(bestiary_page() * BESTIARY_PER_PAGE).take(BESTIARY_PER_PAGE).enumerate() {
        let y = panel.y + top + 30.0 * i as f32;
        let seen = met.contains(lesson.name);
        draw_rectangle(panel.x + 20.0, y + 8.0, 12.0, 12.0, if seen { lesson.color } else { Color::new(0.3, 0.32, 0.36, 1.0) });
        let name = if seen { lesson.shown.as_str() } else { "???" };
        draw_label(name, panel.x + 40.0, y + 19.0, fit_size(name, 15.0, 150.0, Face::Bold), if seen { INK } else { MUTED }, Face::Bold);
        let text = if seen { lesson.lesson.as_str() } else { lesson.sort };
        let room = panel.w - 220.0;
        draw_label(text, panel.x + 196.0, y + 19.0, fit_size(text, 13.0, room, Face::Body), MUTED, Face::Body);
    }
}
