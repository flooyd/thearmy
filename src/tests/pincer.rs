//hordes that come from two sides, each led by a captain as big as a boss

use super::*;

const CENTRE: Vec2 = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);
const REACH: f32 = HORDE_DISTANCE_TILES * TILE_SIZE;

fn pincer(name: &str) -> &'static HordeDef {
    let def = horde_named(name).expect("a horde");
    assert!(def.captain.is_some(), "{name} has no captain");
    def
}

//a horde with a captain arriving at the two places straight across from each other, after a second. returns it, and what is there
fn arrive(name: &str, army: usize) -> (Hordes, Vec<Enemy>, usize) {
    let mut hordes = Hordes::new();
    let mut enemies = Vec::new();
    hordes.coming = Some(Coming { def: pincer(name), at: CENTRE + Vec2::new(REACH, 0.0), left: 0.5, other: Some(CENTRE - Vec2::new(REACH, 0.0)) });
    let count = match hordes.update(&mut enemies, CENTRE, army, army, false, 1.0) {
        Some(HordeBeat::Arrived { count, .. }) => count,
        _ => panic!("{name} should have arrived"),
    };
    (hordes, enemies, count)
}

fn captains(enemies: &[Enemy]) -> Vec<&Enemy> {
    enemies.iter().filter(|enemy| enemy.is_captain()).collect()
}

#[test]
fn the_hordes_with_captains_are_real_ones_that_a_big_enough_army_meets_and_the_rest_come_from_one_side() {
    let pincers: Vec<&HordeDef> = HORDES.iter().filter(|def| def.captain.is_some()).collect();
    assert!(pincers.len() >= 2, "a silver one and a gold one");
    assert!(pincers.iter().any(|def| def.prize == 2) && pincers.iter().any(|def| def.prize == 3));
    for def in &pincers {
        let captain = kind_named(def.captain.unwrap());
        assert!(def.from_army >= captain.from_army, "{} comes before its captain can", def.name);
        assert!(captain.size >= 100.0, "a captain is a big one");
        assert_eq!(def.sides(), 2);
        assert!(def.prize >= 2, "two captains are worth more than a bronze chest");
    }
    for def in HORDES.iter().filter(|def| def.captain.is_none()) {
        assert_eq!(def.sides(), 1, "{}", def.name);
    }
}

#[test]
fn a_captain_is_a_boss_sized_titan_that_is_much_tougher_and_worth_more_and_never_slower_than_the_crowd() {
    rand::srand(5);
    for army in [100usize, 800, 20_000] {
        let plain = kind_at("Titan", MIDDLE);
        let mut captain = plain.clone();
        captain.make_captain(army);
        assert_eq!(captain.modifier, Modifier::Captain);
        assert!(captain.is_captain() && !plain.is_captain());
        //as big as a boss is for that army, which is what it was asked to be, whatever size a titan is
        assert!((captain.size - (CAPTAIN_SIZE_BASE + CAPTAIN_SIZE_GROWTH * (army as f32).sqrt())).abs() < 1.0e-3);
        for def in BOSSES.iter() {
            let boss = Enemy::boss(def, army, 0, MIDDLE);
            assert!(captain.size > boss.size * 0.4 && captain.size < boss.size * 2.0, "a captain of {} px against {} at {army}", captain.size, def.name);
        }
        assert!((captain.max_health / plain.max_health - CAPTAIN_HEALTH).abs() < 1.0e-4 && captain.health == captain.max_health);
        assert!((captain.attack / plain.attack - CAPTAIN_ATTACK).abs() < 1.0e-4);
        assert_eq!(captain.babies, (plain.babies as f32 * CAPTAIN_BABIES).round() as usize);
        assert!(captain.armor >= CAPTAIN_ARMOR && captain.speed >= CAPTAIN_MIN_SPEED);
        assert!(captain.size >= 120.0, "as big as the smallest boss: {}", captain.size);
        assert!(kill_points(&captain) > kill_points(&plain) * 1.5);
    }
    let mut captain = kind_at("Titan", MIDDLE);
    captain.make_captain(600);
    assert_eq!(captain.title(), "Captain Titan");
    assert!(traits(&captain).iter().any(|line| line == "leads a horde, rallies those round it"));
    assert_ne!(Modifier::Captain.color(), Modifier::None.color());
    //(a captain is only ever made, it is not one of the elites that turn up now and then)
    assert!(!MODIFIERS.contains(&Modifier::Captain));
    rand::srand(6);
    assert!((0..3000).map(|_| spawn_enemy(MIDDLE.x, MIDDLE.y, 5000)).all(|enemy| !enemy.is_captain()));
}

#[test]
fn the_two_places_a_pincer_comes_from_are_across_the_player_from_each_other_and_in_the_world() {
    rand::srand(9);
    for player in [CENTRE, CENTRE + Vec2::new(9000.0, -4000.0), Vec2::new(WORLD_SIZE * 0.2, WORLD_SIZE * 0.5)] {
        for _ in 0..100 {
            let (first, second) = horde_spots(player).expect("room for both");
            for at in [first, second] {
                assert!((at.distance(player) - REACH).abs() < 1.0, "{at:?}");
                let edge = HORDE_EDGE_TILES * TILE_SIZE;
                assert!(at.x > edge && at.y > edge && at.x < WORLD_SIZE - edge && at.y < WORLD_SIZE - edge);
            }
            let turn = (first - player).angle_between(second - player).abs();
            assert!(turn >= std::f32::consts::PI - PINCER_SKEW - 0.001, "{turn} is not across");
            assert_ne!(compass(first - player), compass(second - player));
        }
    }
    //squeezed into a corner there is no room for the two to be across from each other
    assert!(horde_spots(Vec2::new(300.0, 300.0)).is_none());
}

#[test]
fn both_sides_arrive_together_each_behind_a_captain_and_the_crowd_is_shared_between_them() {
    rand::srand(14);
    let army = 600;
    let (hordes, enemies, count) = arrive("Pincer Host", army);
    let def = pincer("Pincer Host");
    let followers = def.count(army);
    assert_eq!(count, followers + 2, "the captains come as well");
    assert_eq!(enemies.len(), count);
    assert!(enemies.iter().all(|enemy| in_a_horde(enemy) && enemy.chasing && !is_wild(enemy)));
    let captains = captains(&enemies);
    assert_eq!(captains.len(), 2);
    let (east, west) = (CENTRE + Vec2::new(REACH, 0.0), CENTRE - Vec2::new(REACH, 0.0));
    assert!(captains.iter().any(|captain| captain.position.distance(east) < 1.0) && captains.iter().any(|captain| captain.position.distance(west) < 1.0), "one at the head of each side");
    //the followers: each is on one side or the other, and the sides are the same size give or take one
    let spread = HORDE_SPREAD + ((followers as f32 / 2.0).ceil()).sqrt() * HORDE_SPREAD_PER_ROOT;
    //(they start round the captain, not inside its body where they could not be seen)
    let clear = captains[0].size / 2.0;
    let (mut on_east, mut on_west) = (0, 0);
    for enemy in enemies.iter().filter(|enemy| !enemy.is_captain()) {
        assert!(def.kinds.iter().any(|(kind, _)| *kind == enemy.name), "{}", enemy.name);
        let (to_east, to_west) = (enemy.position.distance(east), enemy.position.distance(west));
        let nearest = to_east.min(to_west);
        assert!(nearest >= clear - 1.0 && nearest <= clear + spread + 1.0, "a follower {nearest} px from its captain's middle, which is {clear} across");
        if to_east < to_west { on_east += 1 } else { on_west += 1 }
    }
    assert_eq!((on_east + on_west, on_east), (followers, followers.div_ceil(2)));
    //the captains are bigger than anything they lead, and they lead the fight: it is one fight
    let biggest_follower = enemies.iter().filter(|enemy| !enemy.is_captain()).map(|enemy| enemy.max_health).fold(0.0, f32::max);
    assert!(captains.iter().all(|captain| captain.max_health > biggest_follower * 1.5 && captain.size > 150.0));
    assert_eq!(hordes.fight.expect("one fight").total, count);
    assert_eq!(hordes.status(&enemies).map(|(line, _)| line), Some(format!("HORDE: Pincer Host ({count} left, 2 captains)")));
}

#[test]
fn a_pincer_with_no_second_place_known_comes_from_straight_across_the_player() {
    rand::srand(15);
    let mut hordes = Hordes::new();
    let mut enemies = Vec::new();
    let player = CENTRE + Vec2::new(500.0, 300.0);
    hordes.coming = Some(Coming { def: pincer("Flanking Raid"), at: player + Vec2::new(0.0, -REACH), left: 0.2, other: None });
    assert!(matches!(hordes.update(&mut enemies, player, 400, 400, false, 1.0), Some(HordeBeat::Arrived { .. })));
    let across = player + Vec2::new(0.0, REACH);
    assert!(captains(&enemies).iter().any(|captain| captain.position.distance(across) < 1.0), "the other captain stands across from the first");
}

#[test]
fn a_pincer_is_announced_with_both_directions_and_both_places_are_pointed_at() {
    rand::srand(77);
    let found = (0..400).find_map(|_| {
        let mut hordes = Hordes::new();
        hordes.next_in = 0.0;
        match hordes.update(&mut Vec::new(), CENTRE, 600, 600, false, 0.1) {
            Some(HordeBeat::Warned { name, from }) if horde_named(name).is_some_and(|def| def.captain.is_some()) => Some((hordes, name, from)),
            _ => None,
        }
    });
    let (mut hordes, name, from) = found.expect("one of the thirteen in a few hundred is a pincer");
    let coming = hordes.coming.expect("it is on its way");
    let other = coming.other.expect("with a second place");
    assert_eq!(coming.def.name, name);
    assert_eq!(from, format!("{} AND THE {}", compass(coming.at - CENTRE), compass(other - CENTRE)));
    assert!(from.contains(" AND THE "), "{from}");
    assert_eq!(hordes.headings(&[], CENTRE), vec![coming.at, other], "an arrow for each");
    //an ordinary one has the one
    let mut single = Hordes::new();
    single.coming = Some(Coming { def: &HORDES[0], at: CENTRE, left: 3.0, other: None });
    assert_eq!(single.headings(&[], CENTRE), vec![CENTRE]);
    assert!(Hordes::new().headings(&[], CENTRE).is_empty());
    //and the warning runs out, and then there are two captains
    let mut enemies = Vec::new();
    assert!(matches!(hordes.update(&mut enemies, CENTRE, 600, 600, false, HORDE_WARNING_SECONDS + 0.1), Some(HordeBeat::Arrived { .. })));
    assert_eq!(captains(&enemies).len(), 2);
}

#[test]
fn a_pincer_is_broken_only_when_both_captains_and_all_their_crowds_are_gone_and_leaves_a_chest_that_has_to_be_held() {
    rand::srand(16);
    let (mut hordes, mut enemies, _) = arrive("Pincer Host", 600);
    let prize = pincer("Pincer Host").prize;
    //only the captains are left: the fight goes on
    enemies.retain(|enemy| enemy.is_captain());
    assert!(hordes.update(&mut enemies, CENTRE, 600, 600, false, 0.1).is_none() && hordes.chests.is_empty());
    let (line, _) = hordes.status(&enemies).unwrap();
    assert!(line.contains("2 captains"), "{line}");
    //one of them falls, where it stood
    let fallen = enemies.pop().unwrap();
    assert!(hordes.update(&mut enemies, CENTRE, 600, 600, false, 0.1).is_none());
    assert!(hordes.status(&enemies).unwrap().0.contains("1 captain)"), "{}", hordes.status(&enemies).unwrap().0);
    assert_ne!(hordes.fight.unwrap().last_at, fallen.position, "(it is the last that is remembered)");
    //and then the last
    let last_at = enemies[0].position;
    enemies.clear();
    match hordes.update(&mut enemies, CENTRE, 600, 600, false, 0.1) {
        Some(HordeBeat::Broken { name, tier }) => assert_eq!((name, tier), ("Pincer Host", prize)),
        _ => panic!("both sides are gone"),
    }
    let chest = hordes.chests[0];
    assert_eq!((chest.at, chest.tier), (last_at, prize));
    assert!(chest.hold() > 0.0, "a chest of that tier has to be held");
}

#[test]
fn captains_run_at_the_army_as_fast_as_the_crowd_they_lead() {
    rand::srand(17);
    let (_, mut enemies, _) = arrive("Flanking Raid", 400);
    let before: Vec<f32> = enemies.iter().map(|enemy| enemy.position.distance(CENTRE)).collect();
    let mut fx = Effects::default();
    for _ in 0..30 {
        patrol_free(&mut enemies, CENTRE, 0.1, 1.0, &mut fx);
    }
    for (enemy, was) in enemies.iter().zip(&before) {
        if enemy.is_captain() {
            assert!(was - enemy.position.distance(CENTRE) >= CAPTAIN_MIN_SPEED * 3.0 * 0.9, "a captain covered {} px in 3 s", was - enemy.position.distance(CENTRE));
        }
    }
}

#[test]
fn a_captain_and_a_horde_with_two_sides_are_kept_in_a_save() {
    rand::srand(18);
    let mut captain = kind_at("Titan", MIDDLE + Vec2::new(400.0, 0.0));
    captain.make_captain(600);
    join_horde(&mut captain);
    let mut writer = SaveWriter::new();
    writer.put(&captain);
    let text = writer.finish();
    assert!(text.contains("Captain"));
    let back: Enemy = SaveReader::new(&text).get().unwrap();
    assert!(back.is_captain() && in_a_horde(&back));
    assert_eq!((back.size, back.max_health, back.armor, back.attack), (captain.size, captain.max_health, captain.armor, captain.attack));
    //the second place of the horde that is coming is on the end, with everything else of the hordes before it
    let mut world = new_run();
    let other = Vec2::new(21000.5, 9000.25);
    world.hordes.coming = Some(Coming { def: pincer("Flanking Raid"), at: Vec2::new(7000.0, 8000.0), left: 4.5, other: Some(other) });
    world.hordes.chests = vec![Chest::new(Vec2::new(15000.0, 15000.0), 3, 20.0)];
    let loaded = Snapshot::from_text(&world.to_text()).unwrap();
    let coming = loaded.hordes.coming.expect("the horde is kept");
    assert_eq!((coming.def.name, coming.at, coming.left, coming.other), ("Flanking Raid", Vec2::new(7000.0, 8000.0), 4.5, Some(other)));
    assert_eq!(loaded.hordes.chests, world.hordes.chests);
    world.hordes.coming = Some(Coming { other: None, ..coming });
    assert_eq!(Snapshot::from_text(&world.to_text()).unwrap().hordes.coming.unwrap().other, None);
    world.hordes.coming = None;
    assert!(Snapshot::from_text(&world.to_text()).unwrap().hordes.coming.is_none());
}

#[test]
fn a_save_from_before_hordes_had_two_sides_still_loads_and_a_damaged_end_is_refused() {
    let hordes = Hordes { next_in: 12.5, coming: Some(Coming { def: &HORDES[1], at: Vec2::new(5000.0, 6000.0), left: 3.0, other: None }), fight: None, chests: vec![Chest::new(Vec2::new(9000.0, 9000.0), 2, 10.0)] };
    //(the old way of writing them: the same, and nothing after the chests)
    let mut writer = SaveWriter::new();
    writer.put(&hordes.next_in);
    writer.put(&hordes.coming);
    writer.put(&hordes.fight);
    writer.put(&hordes.chests);
    let old = writer.finish();
    let loaded: Hordes = SaveReader::new(&old).get().expect("an old one reads");
    assert_eq!((loaded.next_in, loaded.coming.map(|coming| (coming.def.name, coming.other)), loaded.chests.len()), (12.5, Some(("Raiding Party", None)), 1));
    let mut writer = SaveWriter::new();
    writer.put(&hordes);
    let text = writer.finish();
    let again: Hordes = SaveReader::new(&text).get().unwrap();
    assert_eq!(again.coming.unwrap().other, None);
    assert!(SaveReader::new(&format!("{old} nonsense")).get::<Hordes>().is_err());
}
