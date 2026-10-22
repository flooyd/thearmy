//champions: the roster, what each does in a fight and on its own, how they are won and kept, and how they look on screen

use super::*;

fn champion_at(name: &str, at: Vec2) -> Unit {
    champion_unit(champion_named(name).unwrap_or_else(|| panic!("no champion called {name}")), at)
}

//an enemy that nothing can kill in a test, and that does no harm
fn dummy(at: Vec2) -> Enemy {
    let mut enemy = kind_at("Grunt", at);
    enemy.max_health = 1.0e9;
    enemy.health = 1.0e9;
    enemy.attack = 0.0;
    enemy
}

//what has been done to it, as it counts for itself: taking it from its health would lose the small amounts, with numbers this big
fn damage_taken(enemy: &Enemy) -> f32 {
    enemy.hurt
}

//the champions' powers for `seconds`, in frames of a sixtieth
fn act_for(units: &mut Vec<Unit>, enemies: &mut Vec<Enemy>, babies: &mut Vec<Baby>, seconds: f32, fx: &mut Effects) {
    for _ in 0..(seconds * 60.0).round() as usize {
        champion_acts(units, enemies, babies, &Upgrades::default(), MAX_UNITS, 1.0 / 60.0, fx);
    }
}

fn act_once(units: &mut Vec<Unit>, enemies: &mut Vec<Enemy>) -> Effects {
    let mut fx = Effects::default();
    champion_acts(units, enemies, &mut Vec::new(), &Upgrades::default(), MAX_UNITS, 1.0 / 60.0, &mut fx);
    fx
}

// ---------- the roster ----------

#[test]
fn there_are_fourteen_kinds_of_champion_and_no_two_are_alike() {
    assert_eq!(CHAMPIONS.len(), 14);
    for (i, a) in CHAMPIONS.iter().enumerate() {
        assert!(!a.name.is_empty() && !a.blurb.is_empty() && a.tag.chars().count() == 2, "{}", a.name);
        assert!((3..=8).contains(&a.sides), "{} has {} sides", a.name, a.sides);
        assert!(a.might >= 1.0 && a.ward > 0.0 && a.ward <= 1.0, "{}", a.name);
        assert!(a.size >= 1.0 && a.size <= MAX_CHAMPION_SIZE, "{} is {} times a unit", a.name, a.size);
        for b in &CHAMPIONS[i + 1..] {
            assert!(a.name != b.name && a.tag != b.tag, "{} and {}", a.name, b.name);
            let apart = (a.color.r - b.color.r).abs() + (a.color.g - b.color.g).abs() + (a.color.b - b.color.b).abs();
            assert!(apart > 0.15, "{} and {} are the same colour", a.name, b.name);
        }
        assert_eq!(champion_named(a.name), Some(i));
    }
    assert_eq!(champion_named("Nobody"), None);
}

#[test]
fn every_grade_has_champions_and_the_rare_ones_are_the_fewest() {
    let count = |grade: Rarity| CHAMPIONS.iter().filter(|champion| champion.grade == grade).count();
    assert!(count(Rarity::Common) >= 4 && count(Rarity::Uncommon) >= 4 && count(Rarity::Rare) >= 3);
    assert!(count(Rarity::Rare) <= count(Rarity::Common));
}

#[test]
fn every_role_is_covered() {
    //something for each thing an army might want: bulk, auras, shooting from afar, control, more units, and what the run gives
    let has = |wanted: fn(&Power) -> bool| CHAMPIONS.iter().any(|champion| wanted(&champion.power));
    assert!(has(|power| matches!(power, Power::Brute | Power::Rage { .. })));
    assert!(has(|power| matches!(power, Power::Banner { .. })) && has(|power| matches!(power, Power::Bulwark { .. })) && has(|power| matches!(power, Power::Mend { .. })));
    assert!(has(|power| matches!(power, Power::Fireball { .. })) && has(|power| matches!(power, Power::Snipe { .. })) && has(|power| matches!(power, Power::Lightning { .. })));
    assert!(has(|power| matches!(power, Power::Frost { .. })) && has(|power| matches!(power, Power::Quake { .. })));
    assert!(has(|power| matches!(power, Power::Muster { .. })) && has(|power| matches!(power, Power::Rebirth { .. })));
    assert!(has(|power| matches!(power, Power::Scout { .. })) && has(|power| matches!(power, Power::Quartermaster { .. })));
}

#[test]
fn a_champion_is_a_bigger_unit_and_an_ordinary_unit_is_not_one() {
    let ordinary = unit_at(100.0, 100.0);
    assert!(ordinary.champion_kind().is_none() && ordinary.radius() == RADIUS);
    assert_eq!((ordinary.might, ordinary.ward), (1.0, 1.0));
    for (kind, champion) in CHAMPIONS.iter().enumerate() {
        let unit = champion_unit(kind, Vec2::new(500.0, 600.0));
        assert_eq!(unit.champion_kind().map(|found| found.name), Some(champion.name));
        assert_eq!((unit.x, unit.y), (500.0, 600.0));
        assert!(unit.radius() >= RADIUS && (unit.radius() - RADIUS * champion.size).abs() < 1e-3);
        assert_eq!((unit.health, unit.max_health(), unit.level), (CHAMPION_HEALTH, CHAMPION_HEALTH, 1), "a champion is much tougher than a unit");
        assert!(CHAMPION_HEALTH > UNIT_HEALTH * 3.0);
        assert_eq!(unit.speed, UNIT_MAX_SPEED, "a champion keeps up with the player");
        assert_eq!(unit.age, 0.0, "it arrives with a flourish");
    }
    //a unit that has a number no champion has is no champion, rather than a crash
    let mut odd = unit_at(0.0, 0.0);
    odd.champion = 200;
    assert!(odd.champion_kind().is_none() && odd.radius() == RADIUS);
}

#[test]
fn champions_are_counted_by_kind() {
    let mut units: Vec<Unit> = (0..30).map(|i| unit_at(i as f32, 0.0)).collect();
    units.push(champion_at("Medic", Vec2::ZERO));
    units.push(champion_at("Medic", Vec2::ZERO));
    units.push(champion_at("Phoenix", Vec2::ZERO));
    assert_eq!(champions_held(&units), 3);
    let held = champions_by_kind(&units);
    assert_eq!(held[champion_named("Medic").unwrap()], 2);
    assert_eq!(held[champion_named("Phoenix").unwrap()], 1);
    assert_eq!(held.iter().sum::<usize>(), 3);
}

// ---------- in a fight ----------

#[test]
fn a_juggernaut_fights_as_fourteen_units() {
    let damage_by = |mut units: Vec<Unit>| {
        apply_champions(&mut units, 0.0);
        let mut enemies = vec![dummy(MIDDLE)];
        fight(&mut enemies, &mut units, &mut Vec::new(), 1.0, &Upgrades::default(), &mut Effects::default());
        damage_taken(&enemies[0])
    };
    let one = damage_by(vec![unit_at(MIDDLE.x, MIDDLE.y)]);
    assert!((one - UNIT_DAMAGE).abs() < 1.0, "{one}");
    let juggernaut = damage_by(vec![champion_at("Juggernaut", MIDDLE)]);
    assert!(juggernaut / one > 14.0 && juggernaut / one < 14.2, "{}", juggernaut / one);
}

#[test]
fn a_champions_might_grows_with_the_army_and_an_ordinary_unit_s_does_not() {
    let might_in = |army: usize| {
        let mut units: Vec<Unit> = (0..army).map(|i| unit_at(i as f32 * 100.0, 0.0)).collect();
        units.push(champion_at("Juggernaut", Vec2::new(-500.0, 0.0)));
        apply_champions(&mut units, 0.0);
        assert!(units[..army].iter().all(|unit| unit.might == 1.0));
        units[army].might
    };
    let (small, big) = (might_in(10), might_in(1000));
    assert!(big > small * 3.0, "{small} against {big}");
    //a champion that is only one unit's worth has nothing to grow
    let mut medic = vec![champion_at("Medic", Vec2::ZERO)];
    apply_champions(&mut medic, 0.0);
    assert_eq!(medic[0].might, 1.0);
}

#[test]
fn a_berserker_hits_harder_the_more_it_is_hurt() {
    let might_at = |health: f32| {
        let mut units = vec![champion_at("Berserker", Vec2::ZERO)];
        units[0].health = health;
        apply_champions(&mut units, 0.0);
        units[0].might
    };
    let full = champion_health(1);
    let (fresh, hurt, nearly_dead) = (might_at(full), might_at(full / 2.0), might_at(full * 0.05));
    assert!(fresh > 9.9 && fresh < 10.2, "{fresh}");
    assert!(hurt > fresh * 1.9 && hurt < fresh * 2.1, "{hurt}");
    assert!(nearly_dead > fresh * 2.7 && nearly_dead <= fresh * 3.0, "{nearly_dead}");
}

#[test]
fn a_champion_takes_a_share_of_the_damage_and_an_ordinary_unit_takes_it_all() {
    let lost_by = |unit: Unit| {
        let mut units = vec![unit];
        apply_champions(&mut units, 0.0);
        let mut enemy = kind_at("Grunt", Vec2::new(units[0].x, units[0].y));
        enemy.max_health = 1.0e9;
        enemy.health = 1.0e9;
        enemy.attack = 25.0;
        let mut enemies = vec![enemy];
        let full = units[0].max_health();
        fight(&mut enemies, &mut units, &mut Vec::new(), 1.0, &Upgrades::default(), &mut Effects::default());
        full - units[0].health
    };
    let ordinary = lost_by(unit_at(MIDDLE.x, MIDDLE.y));
    assert!((ordinary - 25.0).abs() < 0.01, "{ordinary}");
    let juggernaut = lost_by(champion_at("Juggernaut", MIDDLE));
    assert!((juggernaut - 25.0 * 0.22).abs() < 0.01, "{juggernaut}");
    //the same goes for a shot or a blast, which hurt the unit directly
    let mut hit = champion_at("Juggernaut", MIDDLE);
    hit.ward = 0.22;
    hit.hurt(50.0);
    assert!((hit.health - (CHAMPION_HEALTH - 11.0)).abs() < 1e-3);
}

#[test]
fn a_champion_soaks_up_more_of_an_enemys_attack_before_it_falls() {
    //a hit that kills an ordinary unit outright is not even half of what a juggernaut can take, so it is the one that is
    //hurt and the rest of the attack goes on to the next
    let mut units = vec![champion_at("Juggernaut", MIDDLE), unit_at(MIDDLE.x + 5.0, MIDDLE.y)];
    apply_champions(&mut units, 0.0);
    let mut enemy = kind_at("Grunt", MIDDLE);
    enemy.max_health = 1.0e9;
    enemy.health = 1.0e9;
    enemy.attack = 250.0;
    fight(&mut vec![enemy], &mut units, &mut Vec::new(), 1.0, &Upgrades::default(), &mut Effects::default());
    //250 of attack: the juggernaut is first in the army and takes all of it, as 55 damage
    assert!((CHAMPION_HEALTH - units[0].health - 55.0).abs() < 0.01, "{}", units[0].health);
    assert_eq!(units[1].health, UNIT_HEALTH);
}

#[test]
fn a_champion_heals_by_itself_faster_than_a_unit() {
    let mut units = vec![champion_at("Juggernaut", Vec2::ZERO), unit_at(5000.0, 0.0)];
    for unit in &mut units {
        unit.health = 50.0;
    }
    apply_champions(&mut units, 1.0);
    //(a champion mends a share of its health a second as well as a fixed amount, so a tougher one is mended faster)
    assert!((units[0].health - (50.0 + CHAMPION_REGEN + CHAMPION_HEALTH * CHAMPION_REGEN_SHARE)).abs() < 1e-3);
    assert_eq!(units[1].health, 50.0, "an ordinary unit is left to heal as it always did");
}

#[test]
fn a_banner_makes_the_units_round_it_hit_harder_and_the_ones_beyond_it_do_not() {
    let mut units = vec![champion_at("Warlord", MIDDLE), unit_at(MIDDLE.x + 100.0, MIDDLE.y), unit_at(MIDDLE.x - 500.0, MIDDLE.y), unit_at(MIDDLE.x, MIDDLE.y + 900.0)];
    apply_champions(&mut units, 0.0);
    assert!((units[1].might - 1.35).abs() < 1e-3 && (units[2].might - 1.35).abs() < 1e-3);
    assert_eq!(units[3].might, 1.0);
    assert!(units[0].might > 6.0 * 1.35 * 0.99, "it is under its own banner");
    //two banners stack
    units.push(champion_at("Warlord", Vec2::new(MIDDLE.x + 50.0, MIDDLE.y)));
    apply_champions(&mut units, 0.0);
    assert!((units[1].might - 1.35 * 1.35).abs() < 1e-3);
    //and the damage really is more: the same two units against an enemy, with and without a banner not far off
    let damage_with = |banner: bool| {
        let at = Vec2::new(MIDDLE.x, MIDDLE.y + 700.0);
        let mut units = vec![unit_at(at.x, at.y), unit_at(at.x + 10.0, at.y)];
        if banner {
            units.push(champion_at("Warlord", at + Vec2::new(0.0, 300.0)));
        }
        apply_champions(&mut units, 0.0);
        let mut enemies = vec![dummy(at)];
        fight(&mut enemies, &mut units, &mut Vec::new(), 1.0, &Upgrades::default(), &mut Effects::default());
        damage_taken(&enemies[0])
    };
    let ratio = damage_with(true) / damage_with(false);
    assert!(ratio > 1.34 && ratio < 1.36, "{ratio}");
}

#[test]
fn a_bulwark_shields_the_units_round_it_and_shields_stack_to_a_limit() {
    let mut units = vec![champion_at("Bulwark", MIDDLE), unit_at(MIDDLE.x + 300.0, MIDDLE.y), unit_at(MIDDLE.x + 900.0, MIDDLE.y)];
    apply_champions(&mut units, 0.0);
    assert!((units[1].ward - 0.75).abs() < 1e-3);
    assert_eq!(units[2].ward, 1.0);
    for _ in 0..12 {
        units.push(champion_at("Bulwark", Vec2::new(MIDDLE.x + 200.0, MIDDLE.y)));
    }
    apply_champions(&mut units, 0.0);
    assert!((units[1].ward - MIN_WARD).abs() < 1e-3, "{}", units[1].ward);
    assert!(units.iter().all(|unit| unit.ward >= MIN_WARD - 1e-6));
    //a shielded unit really does take less from a hit
    let mut shielded = unit_at(0.0, 0.0);
    shielded.ward = 0.5;
    shielded.hurt(40.0);
    assert!((shielded.health - 80.0).abs() < 1e-3);
}

#[test]
fn a_medic_mends_the_units_near_it_and_no_others() {
    let mut units = vec![champion_at("Medic", MIDDLE), unit_at(MIDDLE.x + 200.0, MIDDLE.y), unit_at(MIDDLE.x - 300.0, MIDDLE.y), unit_at(MIDDLE.x + 2000.0, MIDDLE.y)];
    for unit in &mut units[1..] {
        unit.health = 50.0;
    }
    apply_champions(&mut units, 1.0);
    assert!((units[1].health - 64.0).abs() < 1e-3 && (units[2].health - 64.0).abs() < 1e-3, "{} {}", units[1].health, units[2].health);
    assert_eq!(units[3].health, 50.0);
    //never past full
    for _ in 0..10 {
        apply_champions(&mut units, 1.0);
    }
    assert_eq!(units[1].health, UNIT_HEALTH);
}

#[test]
fn an_army_with_no_champions_is_not_changed_by_them() {
    let mut units: Vec<Unit> = (0..200).map(|i| unit_at(MIDDLE.x + i as f32 * 7.0, MIDDLE.y)).collect();
    units[10].health = 40.0;
    let before: Vec<(f32, f32, f32)> = units.iter().map(|unit| (unit.x, unit.health, unit.guard)).collect();
    apply_champions(&mut units, 1.0);
    champion_acts(&mut units, &mut vec![dummy(MIDDLE)], &mut Vec::new(), &Upgrades::default(), MAX_UNITS, 1.0, &mut Effects::default());
    assert_eq!(before, units.iter().map(|unit| (unit.x, unit.health, unit.guard)).collect::<Vec<_>>());
    assert!(units.iter().all(|unit| unit.might == 1.0 && unit.ward == 1.0));
}

// ---------- the powers ----------

#[test]
fn a_fireball_hits_the_thickest_crowd_in_range_and_waits_for_its_next_turn() {
    let mut units = vec![champion_at("Pyromancer", MIDDLE)];
    units[0].power = 0.0;
    let cluster = Vec2::new(MIDDLE.x + 400.0, MIDDLE.y);
    let lone = Vec2::new(MIDDLE.x - 700.0, MIDDLE.y);
    let far = Vec2::new(MIDDLE.x, MIDDLE.y + 3000.0);
    let mut enemies = vec![dummy(lone), dummy(cluster), dummy(cluster + Vec2::new(80.0, 40.0)), dummy(cluster + Vec2::new(-90.0, 0.0)), dummy(far)];
    let fx = act_once(&mut units, &mut enemies);
    assert!(damage_taken(&enemies[1]) > 1.0e8 && damage_taken(&enemies[2]) > 1.0e8 && damage_taken(&enemies[3]) > 1.0e8, "the crowd burns");
    assert_eq!(damage_taken(&enemies[0]), 0.0, "the one on its own is left");
    assert_eq!(damage_taken(&enemies[4]), 0.0, "and the one out of range");
    assert!(fx.cues.contains(&Cue::Explosion) && !fx.beams.is_empty());
    //it is the champion's turn again after a few seconds, not before
    let taken = damage_taken(&enemies[1]);
    act_for(&mut units, &mut enemies, &mut Vec::new(), 3.0, &mut Effects::default());
    assert_eq!(damage_taken(&enemies[1]), taken);
    act_for(&mut units, &mut enemies, &mut Vec::new(), 2.0, &mut Effects::default());
    assert!(damage_taken(&enemies[1]) > taken * 1.5);
}

#[test]
fn a_fireball_takes_less_of_a_bosss_health_and_none_goes_off_with_nothing_to_hit() {
    let mut units = vec![champion_at("Pyromancer", MIDDLE)];
    units[0].power = 0.0;
    let at = Vec2::new(MIDDLE.x + 300.0, MIDDLE.y);
    let mut boss = Enemy::boss(&BOSSES[0], 100, 0, at);
    boss.max_health = 1.0e9;
    boss.health = 1.0e9;
    //a fireball each, from the same champion
    let mut by_itself = vec![boss];
    act_once(&mut units, &mut by_itself);
    units[0].power = 0.0;
    let mut normal = vec![dummy(at)];
    act_once(&mut units, &mut normal);
    assert!(damage_taken(&by_itself[0]) > 0.0);
    assert!(damage_taken(&by_itself[0]) < damage_taken(&normal[0]) * 0.3, "{} against {}", damage_taken(&by_itself[0]), damage_taken(&normal[0]));
    //nothing in range: no fireball, no wasted wait
    let mut units = vec![champion_at("Pyromancer", MIDDLE)];
    units[0].power = 0.0;
    let fx = act_once(&mut units, &mut vec![dummy(Vec2::new(MIDDLE.x, MIDDLE.y + 5000.0))]);
    assert!(fx.cues.is_empty() && units[0].power <= 0.25);
}

#[test]
fn a_sharpshooter_picks_off_the_toughest_enemy_in_sight() {
    let mut units = vec![champion_at("Sharpshooter", MIDDLE)];
    units[0].power = 0.0;
    let mut weak = dummy(Vec2::new(MIDDLE.x + 300.0, MIDDLE.y));
    weak.health = 1.0e6;
    let mut strong = dummy(Vec2::new(MIDDLE.x - 600.0, MIDDLE.y + 200.0));
    strong.health = 5.0e8;
    let mut tougher_but_out_of_sight = dummy(Vec2::new(MIDDLE.x, MIDDLE.y - 4000.0));
    tougher_but_out_of_sight.health = 9.0e8;
    let mut enemies = vec![weak, strong, tougher_but_out_of_sight];
    let before: Vec<f32> = enemies.iter().map(|enemy| enemy.health).collect();
    let fx = act_once(&mut units, &mut enemies);
    assert_eq!(enemies[0].health, before[0]);
    assert!(enemies[1].health < before[1]);
    assert_eq!(enemies[2].health, before[2]);
    assert!(fx.cues.contains(&Cue::Arrow));
    //twice a second
    let shot = enemies[1].health;
    act_for(&mut units, &mut enemies, &mut Vec::new(), 2.0, &mut Effects::default());
    let shots = ((shot - enemies[1].health) / (before[1] - shot)).round();
    assert!((3.0..=5.0).contains(&shots), "{shots} more shots in two seconds");
}

#[test]
fn lightning_jumps_from_one_enemy_to_the_next_as_far_as_it_can() {
    let mut units = vec![champion_at("Stormcaller", MIDDLE)];
    units[0].power = 0.0;
    //ten in a line, 150 apart, the first within range, so a bolt that jumps six times hits the first six
    let mut enemies: Vec<Enemy> = (0..10).map(|i| dummy(Vec2::new(MIDDLE.x + 300.0 + 150.0 * i as f32, MIDDLE.y))).collect();
    let apart = dummy(Vec2::new(MIDDLE.x - 600.0, MIDDLE.y + 600.0));
    enemies.push(apart);
    let fx = act_once(&mut units, &mut enemies);
    let hit: Vec<bool> = enemies.iter().map(|enemy| damage_taken(enemy) > 0.0).collect();
    assert_eq!(hit[..10].iter().filter(|&&hit| hit).count(), 6, "{hit:?}");
    assert!(hit[..6].iter().all(|&hit| hit), "it goes down the line");
    assert!(!hit[10], "one that is too far from the others is not reached by a jump");
    assert_eq!(fx.beams.len(), 6);
    assert!(fx.cues.contains(&Cue::Zap));
    //a gap wider than a jump ends it
    let mut units = vec![champion_at("Stormcaller", MIDDLE)];
    units[0].power = 0.0;
    let mut enemies = vec![dummy(Vec2::new(MIDDLE.x + 300.0, MIDDLE.y)), dummy(Vec2::new(MIDDLE.x + 300.0 + CHAMPION_JUMP_RANGE * 2.5, MIDDLE.y))];
    act_once(&mut units, &mut enemies);
    assert!(damage_taken(&enemies[0]) > 0.0 && damage_taken(&enemies[1]) == 0.0);
}

#[test]
fn an_earthshaker_slams_everything_round_it_but_only_when_there_is_something_there() {
    let mut units = vec![champion_at("Earthshaker", MIDDLE)];
    units[0].power = 0.0;
    //nothing near: it holds off and looks again soon
    let mut enemies = vec![dummy(Vec2::new(MIDDLE.x + 1500.0, MIDDLE.y))];
    let fx = act_once(&mut units, &mut enemies);
    assert!(fx.cues.is_empty() && damage_taken(&enemies[0]) == 0.0 && units[0].power <= 0.25);
    //then one comes close
    enemies.push(dummy(Vec2::new(MIDDLE.x + 200.0, MIDDLE.y)));
    enemies.push(dummy(Vec2::new(MIDDLE.x, MIDDLE.y - 250.0)));
    act_for(&mut units, &mut enemies, &mut Vec::new(), 0.5, &mut Effects::default());
    assert!(damage_taken(&enemies[1]) > 0.0 && damage_taken(&enemies[2]) > 0.0, "all round it are crushed");
    assert_eq!(damage_taken(&enemies[0]), 0.0);
}

#[test]
fn frost_slows_the_enemies_near_it_and_now_and_then_freezes_them() {
    let mut units = vec![champion_at("Frostweaver", MIDDLE)];
    units[0].power = 5.0;
    let near = dummy(Vec2::new(MIDDLE.x + 300.0, MIDDLE.y));
    let far = dummy(Vec2::new(MIDDLE.x + 1500.0, MIDDLE.y));
    let mut boss = Enemy::boss(&BOSSES[0], 100, 0, Vec2::new(MIDDLE.x, MIDDLE.y + 300.0));
    boss.slow = 1.0;
    let mut enemies = vec![near, far, boss];
    act_once(&mut units, &mut enemies);
    assert!((enemies[0].slow - 0.55).abs() < 1e-3);
    assert_eq!(enemies[1].slow, 1.0);
    //it is the terrain that sets the slow each frame, and the slow only takes from what is there, so it doesn't stack with tar
    enemies[0].slow = TAR_SLOW;
    act_once(&mut units, &mut enemies);
    assert_eq!(enemies[0].slow, TAR_SLOW.min(0.55));
    //the pulse
    units[0].power = 0.0;
    enemies[0].slow = 1.0;
    enemies[2].slow = 1.0;
    let fx = act_once(&mut units, &mut enemies);
    assert!(fx.cues.contains(&Cue::Frost));
    assert_eq!(enemies[0].slow, FROZEN_SLOW);
    assert_eq!(enemies[2].slow, FROZEN_BOSS_SLOW, "a boss is not frozen solid");
    assert_eq!(enemies[1].slow, 1.0);
    //and it thaws
    for _ in 0..(1.6 * 60.0) as usize {
        for enemy in &mut enemies {
            enemy.slow = 1.0;
        }
        act_once(&mut units, &mut enemies);
    }
    assert!((enemies[0].slow - 0.55).abs() < 1e-3, "{}", enemies[0].slow);
}

#[test]
fn a_drummer_calls_recruits_when_there_is_room_and_waits_when_there_is_not() {
    let mut units = vec![champion_at("Drummer", MIDDLE)];
    units[0].power = 0.0;
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    champion_acts(&mut units, &mut Vec::new(), &mut babies, &Upgrades::default(), MAX_UNITS, 1.0 / 60.0, &mut fx);
    assert_eq!(babies.len(), 4);
    assert!(babies.iter().all(|baby| baby.position.distance(MIDDLE) <= RADIUS * 5.0 + 1.0));
    //not again for a while
    act_for(&mut units, &mut Vec::new(), &mut babies, 5.0, &mut fx);
    assert_eq!(babies.len(), 4);
    act_for(&mut units, &mut Vec::new(), &mut babies, 5.0, &mut fx);
    assert_eq!(babies.len(), 8);
    //an army that is as big as it can be is not given more, and the drummer tries again soon
    units[0].power = 0.0;
    let mut none = Vec::new();
    champion_acts(&mut units, &mut Vec::new(), &mut none, &Upgrades::default(), 1, 1.0 / 60.0, &mut fx);
    assert!(none.is_empty() && units[0].power > 0.0 && units[0].power <= 1.0);
}

#[test]
fn a_phoenix_rises_once_and_then_has_to_wait_before_it_can_again() {
    let mut units = vec![champion_at("Phoenix", MIDDLE), unit_at(MIDDLE.x + 40.0, MIDDLE.y)];
    let mut fx = Effects::default();
    units[0].health = 0.0;
    assert_eq!(bury_units(&mut units, &mut fx), 0, "it did not die");
    assert_eq!(units.len(), 2);
    assert_eq!(units[0].health, CHAMPION_HEALTH);
    assert_eq!(units[0].power, 90.0);
    assert_eq!(fx.rebirths.len(), 1);
    assert!(fx.announcements.is_empty());
    //it blazes on the next frame, hurting the enemies near it and none far away
    let mut enemies = vec![dummy(Vec2::new(MIDDLE.x + 200.0, MIDDLE.y)), dummy(Vec2::new(MIDDLE.x + 2000.0, MIDDLE.y))];
    champion_acts(&mut units, &mut enemies, &mut Vec::new(), &Upgrades::default(), MAX_UNITS, 1.0 / 60.0, &mut fx);
    assert!(damage_taken(&enemies[0]) > 1.0e8 && damage_taken(&enemies[1]) == 0.0);
    assert!(fx.rebirths.is_empty() && fx.cues.contains(&Cue::Explosion));
    //it can't again straight away
    units[0].health = -5.0;
    assert_eq!(bury_units(&mut units, &mut fx), 1);
    assert_eq!(units.len(), 1);
    assert!(fx.announcements.iter().any(|(text, _)| text == "PHOENIX HAS FALLEN"), "{:?}", fx.announcements);
    assert!(fx.cues.contains(&Cue::Fall));
    //but once the time is up it can
    let mut units = vec![champion_at("Phoenix", MIDDLE)];
    units[0].health = 0.0;
    bury_units(&mut units, &mut fx);
    act_for(&mut units, &mut Vec::new(), &mut Vec::new(), 91.0, &mut fx);
    assert_eq!(units[0].power, 0.0);
    units[0].health = 0.0;
    assert_eq!(bury_units(&mut units, &mut fx), 0);
}

#[test]
fn a_champion_that_dies_is_mourned_and_an_ordinary_unit_is_not() {
    let mut units = vec![champion_at("Juggernaut", MIDDLE), unit_at(10.0, 10.0)];
    units[0].health = 0.0;
    units[1].health = 0.0;
    let mut fx = Effects::default();
    assert_eq!(bury_units(&mut units, &mut fx), 2);
    assert_eq!(fx.announcements.iter().filter(|(text, _)| text.contains("HAS FALLEN")).count(), 1);
    assert!(fx.announcements[0].0.contains("JUGGERNAUT"));
}

#[test]
fn nothing_goes_off_while_the_world_is_standing_still() {
    let mut units: Vec<Unit> = CHAMPIONS.iter().enumerate().map(|(kind, _)| champion_unit(kind, MIDDLE)).collect();
    for unit in &mut units {
        unit.power = 0.0;
    }
    let mut enemies = vec![dummy(Vec2::new(MIDDLE.x + 200.0, MIDDLE.y)), dummy(Vec2::new(MIDDLE.x + 300.0, MIDDLE.y))];
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    champion_acts(&mut units, &mut enemies, &mut babies, &Upgrades::default(), MAX_UNITS, 0.0, &mut fx);
    assert!(fx.cues.is_empty() && babies.is_empty() && enemies.iter().all(|enemy| damage_taken(enemy) == 0.0));
}

#[test]
fn every_kind_of_champion_can_act_for_a_minute_among_enemies_without_trouble() {
    let mut units: Vec<Unit> = (0..CHAMPIONS.len()).map(|kind| champion_unit(kind, MIDDLE + Vec2::new(kind as f32 * 20.0, 0.0))).collect();
    units.extend((0..100).map(|i| unit_at(MIDDLE.x + i as f32 * 12.0, MIDDLE.y + 60.0)));
    let mut enemies: Vec<Enemy> = (0..60).map(|i| dummy(Vec2::new(MIDDLE.x - 600.0 + (i % 12) as f32 * 120.0, MIDDLE.y - 400.0 + (i / 12) as f32 * 120.0))).collect();
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    for _ in 0..60 * 60 {
        apply_champions(&mut units, 1.0 / 60.0);
        for enemy in &mut enemies {
            enemy.slow = 1.0;
        }
        champion_acts(&mut units, &mut enemies, &mut babies, &Upgrades::default(), MAX_UNITS, 1.0 / 60.0, &mut fx);
        fx.cues.clear();
        fx.cue_spots.clear();
        fx.beams.clear();
        fx.bursts.clear();
        fx.rings.clear();
        fx.marks.clear();
        fx.popups.clear();
    }
    assert!(enemies.iter().all(|enemy| enemy.health.is_finite() && enemy.slow.is_finite()));
    assert!(enemies.iter().filter(|enemy| damage_taken(enemy) > 0.0).count() >= 30, "the powers should have reached most of them");
    assert!(units.iter().all(|unit| unit.power.is_finite() && unit.health.is_finite()));
    assert!(!babies.is_empty());
}

// ---------- what they give the whole army ----------

#[test]
fn an_army_with_no_perks_has_none() {
    let units: Vec<Unit> = (0..50).map(|i| unit_at(i as f32, 0.0)).collect();
    let perks = Perks::of(&units);
    assert_eq!(perks, Perks { speed: 1.0, pickup: 1.0, bounty: 1.0, points: 0.0 });
    let mut upgrades = Upgrades::default();
    upgrades.take(Relic::FleetFeet);
    let applied = perks.applied(&upgrades);
    assert_eq!(format!("{applied:?}"), format!("{upgrades:?}"));
}

#[test]
fn a_pathfinder_speeds_the_army_up_and_widens_its_reach_and_the_first_few_stack() {
    let mut units = vec![champion_at("Pathfinder", Vec2::ZERO)];
    let one = Perks::of(&units);
    assert!((one.speed - 1.12).abs() < 1e-4 && (one.pickup - 1.35).abs() < 1e-4);
    assert_eq!((one.bounty, one.points), (1.0, 0.0));
    for _ in 0..5 {
        units.push(champion_at("Pathfinder", Vec2::ZERO));
    }
    let many = Perks::of(&units);
    assert!((many.speed - (1.0 + 0.12 * CHAMPION_STACK as f32)).abs() < 1e-4, "only {CHAMPION_STACK} count: {}", many.speed);
    //on top of the relics, and nothing else changes
    let mut upgrades = Upgrades::default();
    upgrades.take(Relic::FleetFeet);
    let applied = one.applied(&upgrades);
    assert!((applied.speed - 1.12 * 1.12).abs() < 1e-4);
    assert!((applied.pickup - 1.35).abs() < 1e-4);
    assert_eq!((applied.damage, applied.toughness, applied.bounty), (1.0, 1.0, 1.0));
    assert!(applied.army_boost(1000) > upgrades.army_boost(1000) * 1.1);
}

#[test]
fn a_quartermaster_earns_wall_points_and_enemies_drop_more_babies() {
    let units = vec![champion_at("Quartermaster", Vec2::ZERO)];
    let perks = Perks::of(&units);
    assert!((perks.points - 1.5).abs() < 1e-4 && (perks.bounty - 1.25).abs() < 1e-4);
    //the babies are dropped by what is destroyed, which sees the bounty through the upgrades it is given
    let drops = |bounty: f32| {
        let mut enemy = kind_at("Titan", MIDDLE);
        enemy.babies = 40;
        let mut babies = Vec::new();
        drop_babies(&enemy, &mut babies, 10, bounty, MAX_UNITS);
        babies.len()
    };
    assert_eq!(drops(perks.applied(&Upgrades::default()).bounty), 50);
    assert_eq!(drops(1.0), 40);
}

// ---------- winning them ----------

//rolls from 0 to 1 in even steps, so the shares that come out are exact
fn shares(units: &[Unit], weights: [f32; 3]) -> Vec<f32> {
    let steps = 4000;
    let mut count = vec![0usize; CHAMPIONS.len()];
    for i in 0..steps {
        count[pick_champion(units, weights, (i as f32 + 0.5) / steps as f32)] += 1;
    }
    count.into_iter().map(|count| count as f32 / steps as f32).collect()
}

fn grade_share(shares: &[f32], grade: Rarity) -> f32 {
    CHAMPIONS.iter().zip(shares).filter(|(champion, _)| champion.grade == grade).map(|(_, share)| share).sum()
}

#[test]
fn the_grade_of_a_champion_follows_where_it_came_from() {
    let none: Vec<Unit> = Vec::new();
    let barracks = shares(&none, Odds::Barracks.weights());
    assert!((grade_share(&barracks, Rarity::Common) - 0.6).abs() < 0.01);
    assert!((grade_share(&barracks, Rarity::Uncommon) - 0.3).abs() < 0.01);
    assert!((grade_share(&barracks, Rarity::Rare) - 0.1).abs() < 0.01);
    //the citadel's lord only ever gives the best
    let citadel = shares(&none, Odds::Citadel.weights());
    assert!((grade_share(&citadel, Rarity::Rare) - 1.0).abs() < 1e-3);
    //a boss is a better bet than a barracks, and each time round the bosses have been better again
    let first = grade_share(&shares(&none, Odds::Boss(0).weights()), Rarity::Rare);
    let later = grade_share(&shares(&none, Odds::Boss(4).weights()), Rarity::Rare);
    assert!(first > 0.15 && later > first + 0.15, "{first} then {later}");
    //every kind can be won from somewhere, even the rare ones
    let all = shares(&none, [1.0, 1.0, 1.0]);
    assert!(all.iter().all(|&share| share > 0.0));
}

#[test]
fn a_kind_the_army_has_already_is_less_likely_to_be_won_again() {
    let none: Vec<Unit> = Vec::new();
    let commons = [1.0, 0.0, 0.0];
    let fresh = shares(&none, commons);
    let juggernaut = champion_named("Juggernaut").unwrap();
    assert!((fresh[juggernaut] - 0.2).abs() < 0.01);
    let held = vec![champion_at("Juggernaut", Vec2::ZERO), champion_at("Juggernaut", Vec2::ZERO)];
    let after = shares(&held, commons);
    let expected = CHAMPION_REPEAT_WEIGHT.powi(2) / (4.0 + CHAMPION_REPEAT_WEIGHT.powi(2));
    assert!((after[juggernaut] - expected).abs() < 0.01, "{} against {expected}", after[juggernaut]);
    assert!(after.iter().sum::<f32>() > 0.999, "the shares still add up");
    //every roll gives a kind that is there, whatever the roll
    for roll in [-1.0, 0.0, 0.5, 0.999_999, 1.0, 7.0] {
        assert!(pick_champion(&held, Odds::Barracks.weights(), roll) < CHAMPIONS.len());
    }
}

#[test]
fn a_champion_that_is_won_joins_the_army_with_a_fanfare() {
    let mut units: Vec<Unit> = (0..20).map(|i| unit_at(MIDDLE.x + i as f32 * 5.0, MIDDLE.y)).collect();
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    let kind = award_champion(&mut units, &mut babies, MIDDLE, Odds::Barracks, MAX_UNITS, &mut fx).expect("there is room");
    assert_eq!(units.len(), 21);
    assert_eq!(champions_held(&units), 1);
    let joined = units.last().unwrap();
    assert_eq!(joined.champion as usize, kind + 1);
    assert!(Vec2::new(joined.x, joined.y).distance(MIDDLE) <= RADIUS * 2.0 + 1.0);
    assert!(fx.announcements.iter().any(|(text, color)| text.contains("JOINS YOUR ARMY") && text.contains(&CHAMPIONS[kind].name.to_uppercase()) && *color == CHAMPIONS[kind].color));
    assert!(fx.cues.contains(&Cue::Champion));
    assert!(fx.punch > 0.0 && !fx.popups.is_empty());
    assert!(babies.is_empty());
    //it counts against the army like any unit, even one that is as big as it can be, which the roster does not have to wait for
    let mut full: Vec<Unit> = (0..10).map(|i| unit_at(i as f32, 0.0)).collect();
    assert!(award_champion(&mut full, &mut babies, MIDDLE, Odds::Boss(0), 10, &mut fx).is_some());
    assert_eq!(full.len(), 11);
}

#[test]
fn the_roster_has_room_for_a_dozen_and_a_champion_past_that_gives_recruits() {
    let mut units: Vec<Unit> = (0..100).map(|i| unit_at(i as f32, 0.0)).collect();
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    //(a dozen won all find a place, either of their own or by joining one of the kind they already have)
    for _ in 0..MAX_CHAMPIONS {
        assert!(award_champion(&mut units, &mut babies, MIDDLE, Odds::Boss(2), MAX_UNITS, &mut fx).is_some());
    }
    assert!(champions_held(&units) <= MAX_CHAMPIONS && babies.is_empty());
    //a dozen that have risen as far as they can leave nowhere for another, of any kind
    let mut units: Vec<Unit> = (0..100).map(|i| unit_at(i as f32, 0.0)).collect();
    for kind in 0..MAX_CHAMPIONS {
        let mut champion = champion_unit(kind, MIDDLE);
        champion.level = CHAMPION_MAX_LEVEL;
        units.push(champion);
    }
    assert_eq!(champions_held(&units), MAX_CHAMPIONS);
    let army = units.len();
    let mut fx = Effects::default();
    assert!(award_champion(&mut units, &mut babies, MIDDLE, Odds::Boss(2), MAX_UNITS, &mut fx).is_none());
    assert_eq!(units.len(), army);
    assert_eq!(babies.len(), CHAMPION_RECRUITS);
    assert!(fx.announcements.iter().any(|(text, _)| text.contains("ROSTER FULL")));
}

#[test]
fn a_second_champion_of_a_kind_merges_into_the_first_and_raises_it_a_level() {
    let kind = champion_named("Juggernaut").unwrap();
    let mut units: Vec<Unit> = (0..20).map(|i| unit_at(MIDDLE.x + i as f32 * 5.0, MIDDLE.y)).collect();
    units.push(champion_unit(kind, MIDDLE));
    units[20].health = 50.0;
    let mut babies = Vec::new();
    let mut fx = Effects::default();
    //(champions are won by chance, so go on until it is a juggernaut that is won, putting aside any other that joins)
    let mut won = None;
    for _ in 0..500 {
        won = award_champion(&mut units, &mut babies, MIDDLE, Odds::Boss(2), MAX_UNITS, &mut fx);
        if won == Some(kind) {
            break;
        }
        units.retain(|unit| unit.champion == 0 || unit.champion as usize == kind + 1);
    }
    assert_eq!(won, Some(kind), "it turns up in the end");
    let juggernauts: Vec<&Unit> = units.iter().filter(|unit| unit.champion as usize == kind + 1).collect();
    assert_eq!(juggernauts.len(), 1, "it takes up no room of its own");
    assert_eq!(juggernauts[0].level, 2);
    assert_eq!(juggernauts[0].health, champion_health(2), "and is mended in full");
    assert!(fx.announcements.iter().any(|(text, _)| text.contains("MERGES") && text.contains("LEVEL 2")), "{:?}", fx.announcements);
}

#[test]
fn a_champion_that_has_risen_is_tougher_and_stronger_in_everything_it_does() {
    let stats = |level: u8| {
        let mut unit = champion_at("Juggernaut", Vec2::ZERO);
        unit.level = level;
        let mut units = vec![unit];
        apply_champions(&mut units, 0.0);
        (champion_health(level), units[0].might, units[0].ward)
    };
    let mut last = stats(1);
    assert_eq!(last.0, CHAMPION_HEALTH);
    for level in 2..=CHAMPION_MAX_LEVEL {
        let now = stats(level);
        assert!(now.0 > last.0 && now.1 > last.1 && now.2 < last.2, "level {level}: {now:?} after {last:?}");
        last = now;
    }
    assert!(last.2 >= MIN_WARD, "there is a limit to how little damage it takes");
    //its power grows too: more damage, from further off, and quicker
    for kind in CHAMPIONS.iter() {
        let (base, top) = (kind.power.at_level(1), kind.power.at_level(CHAMPION_MAX_LEVEL));
        match (base, top) {
            (Power::Fireball { every: a, flat: b, radius: c, .. }, Power::Fireball { every: x, flat: y, radius: z, .. }) => assert!(x < a && y > b && z > c),
            (Power::Mend { per_second: a, .. }, Power::Mend { per_second: x, .. }) => assert!(x > a),
            (Power::Bulwark { ward: a, .. }, Power::Bulwark { ward: x, .. }) => assert!(x < a, "a bulwark's ward lets less through"),
            (Power::Muster { count: a, .. }, Power::Muster { count: x, .. }) => assert!(x > a),
            _ => {}
        }
    }
}

#[test]
fn kills_teach_the_champions_and_each_level_takes_more_than_the_last() {
    assert!(champion_xp_needed(2) > champion_xp_needed(1) && champion_xp_needed(5) > champion_xp_needed(4) * 1.5);
    let grunt = kind_at("Grunt", Vec2::ZERO);
    let mut elite = kind_at("Grunt", Vec2::ZERO);
    elite.make_elite(Modifier::Armored);
    let boss = Enemy::boss(&BOSSES[0], 100, 0, Vec2::ZERO);
    assert!(kill_xp(&grunt) < kill_xp(&elite) && kill_xp(&elite) < kill_xp(&boss));
    let mut units = vec![champion_at("Medic", Vec2::ZERO), champion_at("Warlord", Vec2::ZERO), unit_at(0.0, 0.0)];
    let mut fx = Effects::default();
    //not enough for a level: it is kept
    train_champions(&mut units, champion_xp_needed(1) - 1.0, &mut fx);
    assert_eq!((units[0].level, units[1].level), (1, 1));
    assert!((units[0].xp - (champion_xp_needed(1) - 1.0)).abs() < 1e-3 && units[2].xp == 0.0, "an ordinary unit learns nothing");
    assert!(fx.announcements.is_empty());
    //enough: every champion rises, and what was over is kept towards the next
    train_champions(&mut units, 3.0, &mut fx);
    assert_eq!((units[0].level, units[1].level), (2, 2));
    assert!((units[0].xp - 2.0).abs() < 1e-3, "{}", units[0].xp);
    assert_eq!(fx.announcements.len(), 2);
    assert_eq!(units[0].health, champion_health(2));
    //a lot at once can be enough for several levels, but never past the top, where nothing more is kept
    train_champions(&mut units, 1.0e9, &mut fx);
    assert_eq!((units[0].level, units[1].level, units[2].level), (CHAMPION_MAX_LEVEL, CHAMPION_MAX_LEVEL, 0));
    assert_eq!(units[0].xp, 0.0);
}

#[test]
fn the_champions_a_frame_won_are_all_handed_out_once() {
    let mut units: Vec<Unit> = (0..10).map(|i| unit_at(i as f32, 0.0)).collect();
    let mut fx = Effects::default();
    fx.recruits.push((MIDDLE, Odds::Elite));
    fx.recruits.push((MIDDLE + Vec2::new(500.0, 0.0), Odds::Barracks));
    //each is a new champion or, if the army has one of that kind by chance, a level for it: either way two are won, once
    let won = |units: &[Unit]| units.iter().filter(|unit| unit.champion != 0).map(|unit| unit.level as usize).sum::<usize>();
    award_recruits(&mut units, &mut Vec::new(), MAX_UNITS, &mut fx);
    assert_eq!(won(&units), 2);
    assert!(fx.recruits.is_empty());
    award_recruits(&mut units, &mut Vec::new(), MAX_UNITS, &mut fx);
    assert_eq!(won(&units), 2);
    //a new one joins where it was won, which is where the enemy fell or the barracks stood
    if champions_held(&units) == 2 {
        let spots: Vec<f32> = units.iter().filter(|unit| unit.champion != 0).map(|unit| unit.x).collect();
        assert!(spots.iter().any(|&x| x < MIDDLE.x + 100.0) && spots.iter().any(|&x| x > MIDDLE.x + 400.0));
    }
}

#[test]
fn the_first_boss_always_drops_a_champion_and_the_rest_usually_do() {
    assert!(boss_drops_champion(1, 0.999));
    assert!(boss_drops_champion(0, 0.999));
    assert!(boss_drops_champion(2, 0.0));
    assert!(boss_drops_champion(9, CHAMPION_BOSS_CHANCE - 0.001));
    assert!(!boss_drops_champion(2, CHAMPION_BOSS_CHANCE));
    assert!(!boss_drops_champion(9, 0.999));
}

#[test]
fn a_barracks_gives_a_relic_or_a_champion_or_neither_never_both() {
    assert!(barracks_relic_dropped(0.0) && !barracks_champion_dropped(0.0));
    assert!(!barracks_relic_dropped(BARRACKS_RELIC_CHANCE) && barracks_champion_dropped(BARRACKS_RELIC_CHANCE));
    let edge = BARRACKS_RELIC_CHANCE + BARRACKS_CHAMPION_CHANCE;
    assert!(barracks_champion_dropped(edge - 0.001) && !barracks_champion_dropped(edge));
    assert!(!barracks_relic_dropped(0.999) && !barracks_champion_dropped(0.999));
    for i in 0..1000 {
        let roll = i as f32 / 1000.0;
        assert!(!(barracks_relic_dropped(roll) && barracks_champion_dropped(roll)), "{roll}");
    }
    assert!(edge < 1.0, "some of the time it is neither");
}

#[test]
fn an_elite_enemy_now_and_then_leaves_a_champion_and_an_ordinary_one_never_does() {
    let kills = |elite: bool, count: usize| {
        let mut enemies: Vec<Enemy> = (0..count)
            .map(|i| {
                let mut enemy = kind_at("Grunt", Vec2::new(MIDDLE.x + i as f32, MIDDLE.y));
                if elite {
                    enemy.make_elite(Modifier::Swift);
                }
                enemy.health = 0.0;
                enemy
            })
            .collect();
        let mut fx = Effects::default();
        resolve_deaths(&mut enemies, &mut Vec::new(), 100, &Upgrades::default(), &mut fx);
        assert!(fx.recruits.iter().all(|(_, odds)| *odds == Odds::Elite));
        fx.recruits.len()
    };
    assert_eq!(kills(false, 1000), 0);
    let share = (0..4).map(|_| kills(true, 3000) as f32 / 3000.0).find(|share| (CHAMPION_ELITE_CHANCE * 0.5..CHAMPION_ELITE_CHANCE * 1.6).contains(share));
    assert!(share.is_some(), "about {CHAMPION_ELITE_CHANCE} of elites should leave one");
}

// ---------- keeping them with the army ----------

#[test]
fn champions_stay_with_the_player_when_a_squad_is_sent_and_when_a_tower_is_manned() {
    let enemy = kind_at("Grunt", MIDDLE + Vec2::new(500.0, 0.0));
    //the champion is the closest of all to the enemy, and would be the first sent
    let mut units = vec![champion_at("Juggernaut", MIDDLE + Vec2::new(480.0, 0.0))];
    units.extend((0..10).map(|i| unit_at(MIDDLE.x + i as f32 * 20.0, MIDDLE.y)));
    let mut squads = Squads::new();
    let sent = send_squad(&mut units, &mut squads, &enemy, &mut Effects::default());
    assert!(sent >= 1);
    assert!(units[0].target.is_none(), "the champion stays");
    assert!(units[1..].iter().any(|unit| unit.target.is_some()));
    //a tower: the champion, right beside it, is not one of the units that go in
    let mut walls = Walls::default();
    let tile = (600, 600);
    let tower = Wall::made(tile.0, tile.1, WallKind::Tower, Side::Friendly, 400.0);
    let at = tower.center();
    walls.insert(tile, tower);
    let mut units = vec![champion_at("Juggernaut", at)];
    units.extend((0..5).map(|i| unit_at(at.x + 50.0 + i as f32 * 10.0, at.y)));
    let inside = man_tower(&mut walls, &mut units, tile, &mut Effects::default());
    assert_eq!(inside, 5);
    assert!(units[0].garrison.is_none() && units[1..].iter().all(|unit| unit.garrison == Some(tile)));
}

#[test]
fn a_champion_is_drawn_by_itself_and_not_among_the_discs_of_the_army() {
    let mut units: Vec<Unit> = (0..10).map(|i| unit_at(MIDDLE.x + i as f32 * 40.0, MIDDLE.y)).collect();
    units.push(champion_at("Warlord", MIDDLE));
    let view = Rect::new(MIDDLE.x - 1000.0, MIDDLE.y - 1000.0, 2000.0, 2000.0);
    assert_eq!(unit_discs(&units, view).len(), 10);
    //its colour is its own, going red as it is hurt, and a squad's colour is only for ordinary units
    let warlord = &units[10];
    assert_eq!(unit_base_color(warlord), CHAMPIONS[champion_named("Warlord").unwrap()].color);
    let mut hurt = warlord.clone();
    hurt.health = 10.0;
    assert!(unit_color(&hurt).r > unit_color(warlord).r || unit_color(&hurt).g < unit_color(warlord).g);
}

#[test]
fn the_roster_fits_on_the_screen_clear_of_the_map_and_the_other_panels() {
    for screen in [Vec2::new(1280.0, 720.0), Vec2::new(1920.0, 1080.0), Vec2::new(1366.0, 768.0), Vec2::new(1024.0, 768.0)] {
        let rows = roster_rows(MAX_CHAMPIONS, screen);
        assert_eq!(rows.len(), MAX_CHAMPIONS);
        let map = Minimap::new(screen, MapView::Near, MIDDLE).rect;
        for (i, row) in rows.iter().enumerate() {
            assert!(row.x >= 0.0 && row.right() <= screen.x && row.y >= 0.0 && row.bottom() <= screen.y, "row {i} is off a {screen} screen");
            assert!(!row.overlaps(&map), "row {i} is over the map on a {screen} screen");
            for other in &rows[i + 1..] {
                assert!(!row.overlaps(other));
            }
        }
        //the status panel is in the top left corner and wide enough only for itself
        assert!(rows[0].x > 12.0 + 410.0 + 20.0, "the roster is clear of the status panel at {screen}");
    }
}

#[test]
fn the_roster_lists_the_champions_by_kind() {
    let mut units: Vec<Unit> = (0..5).map(|i| unit_at(i as f32, 0.0)).collect();
    units.push(champion_at("Phoenix", Vec2::ZERO));
    units.push(champion_at("Juggernaut", Vec2::ZERO));
    units.push(champion_at("Medic", Vec2::ZERO));
    units.push(champion_at("Juggernaut", Vec2::ZERO));
    let names: Vec<&str> = roster_of(&units).iter().filter_map(|unit| unit.champion_kind()).map(|kind| kind.name).collect();
    assert_eq!(names, vec!["Juggernaut", "Juggernaut", "Medic", "Phoenix"]);
}

// ---------- in a save ----------

fn world_with_champions() -> Snapshot {
    let mut world = new_run();
    world.units = (0..60).map(|i| unit_at(MIDDLE.x + i as f32 * 9.0, MIDDLE.y)).collect();
    for (at, name, power) in [(3, "Juggernaut", 0.0), (8, "Phoenix", 37.5), (21, "Frostweaver", 6.25), (22, "Pathfinder", 0.0), (40, "Stormcaller", 1.125)] {
        world.units[at] = champion_at(name, Vec2::new(MIDDLE.x + at as f32, MIDDLE.y));
        world.units[at].power = power;
        world.units[at].health = 63.5;
    }
    world
}

#[test]
fn the_champions_in_the_army_come_back_from_a_save_as_they_were() {
    let world = world_with_champions();
    let loaded = Snapshot::from_text(&world.to_text()).expect("it reads back");
    assert_eq!(loaded.units.len(), world.units.len());
    for (before, after) in world.units.iter().zip(&loaded.units) {
        assert_eq!((before.champion, before.power, before.health), (after.champion, after.power, after.health));
    }
    assert_eq!(champions_held(&loaded.units), 5);
    assert_eq!(loaded.units[8].champion_kind().map(|kind| kind.name), Some("Phoenix"));
    assert_eq!(loaded.units[8].power, 37.5);
    assert_eq!(loaded.body(), world.body());
    //a second trip changes nothing more
    assert_eq!(Snapshot::from_text(&loaded.to_text()).unwrap().body(), loaded.body());
}

#[test]
fn a_save_from_before_there_were_champions_still_loads() {
    let world = world_with_champions();
    let text = world.to_text();
    let cut = text.find("champions ").expect("the save has the section");
    let old = &text[..cut];
    let loaded = Snapshot::from_text(old).expect("an old save loads");
    assert_eq!(loaded.units.len(), world.units.len());
    assert_eq!(champions_held(&loaded.units), 0);
    //an army with none of them writes the section with nothing in it, and that reads back too
    let mut plain = new_run();
    plain.units = (0..30).map(|i| unit_at(i as f32 * 10.0, 5000.0)).collect();
    let text = plain.to_text();
    assert!(text.contains("champions 0"));
    assert_eq!(champions_held(&Snapshot::from_text(&text).unwrap().units), 0);
}

#[test]
fn a_damaged_champions_section_is_refused_rather_than_loaded() {
    let text = world_with_champions().to_text();
    let refused = |from: &str, to: &str| {
        assert!(text.contains(from), "{from:?} is not in the save");
        Snapshot::from_text(&text.replacen(from, to, 1)).is_err()
    };
    assert!(refused(" Phoenix ", " Griffin "), "a champion the game doesn't have");
    assert!(refused("champions 5", "champions 500"), "more of them than there are units");
    assert!(refused("\n40 Stormcaller", "\n4000 Stormcaller"), "a unit that isn't there");
    assert!(refused("Stormcaller 1.125", "Stormcaller soon"), "a number that isn't");
    assert!(refused("champions 5", "champions 6"), "one more than the section has");
}

// ---------- the whole thing, running ----------

#[test]
fn a_big_army_with_champions_in_it_is_fast_to_work_out() {
    let mut units: Vec<Unit> = (0..MAX_UNITS)
        .map(|i| unit_at(MIDDLE.x + (i % 100) as f32 * 30.0, MIDDLE.y + (i / 100) as f32 * 30.0))
        .collect();
    for (i, name) in ["Warlord", "Bulwark", "Medic", "Frostweaver", "Pyromancer", "Phoenix", "Juggernaut", "Berserker"].into_iter().enumerate() {
        units[i * 700 + 350] = champion_at(name, Vec2::new(MIDDLE.x + 1500.0, MIDDLE.y + 900.0));
    }
    let mut enemies: Vec<Enemy> = (0..200).map(|i| dummy(Vec2::new(MIDDLE.x + (i % 40) as f32 * 80.0, MIDDLE.y + (i / 40) as f32 * 300.0))).collect();
    let started = Instant::now();
    let mut fx = Effects::default();
    for _ in 0..20 {
        apply_champions(&mut units, 1.0 / 60.0);
        champion_acts(&mut units, &mut enemies, &mut Vec::new(), &Upgrades::default(), MAX_UNITS, 1.0 / 60.0, &mut fx);
    }
    let each = started.elapsed().as_secs_f32() * 1000.0 / 20.0;
    println!("champions work out in {each:.2} ms a frame with {MAX_UNITS} units");
    assert!(each < 12.0, "{each} ms a frame");
}
