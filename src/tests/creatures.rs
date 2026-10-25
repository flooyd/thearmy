//the enemies, the bosses and their marks as they are drawn: the records the graphics card reads, which drawing each one gets, the shaders that are made for them, and the
//way an enemy turns. there is no graphics card in a test, so these are the parts that decide what is drawn rather than the drawing itself

use super::*;
use crate::creature_art::{ARTS, HELPERS};

const MARKS: [&str; 7] = ["Swift", "Armored", "Volatile", "Frenzied", "Captain", "Frozen", "Shield"];

#[test]
fn a_creature_is_exactly_what_the_shader_reads_for_each_one() {
    //four floats for the place, the radius and the seed, two for the way it faces, four bytes of colour and a float for the look of its last stand
    assert_eq!(std::mem::size_of::<Instance>(), 32);
    assert_eq!(std::mem::align_of::<Instance>(), 4);
    assert_eq!(std::mem::offset_of!(Instance, radius), 8);
    assert_eq!(std::mem::offset_of!(Instance, seed), 12);
    assert_eq!(std::mem::offset_of!(Instance, facing), 16);
    assert_eq!(std::mem::offset_of!(Instance, color), 24);
    assert_eq!(std::mem::offset_of!(Instance, hurt), 28);
}

#[test]
fn every_enemy_and_boss_and_mark_has_a_drawing() {
    for kind in &ENEMY_KINDS {
        assert!(shape_named(kind.name).is_some_and(|shape| !ARTS[shape - 1].mark), "no drawing for {}", kind.name);
    }
    for def in BOSSES.iter().chain(std::iter::once(&FINAL_BOSS)) {
        assert!(shape_named(def.name).is_some_and(|shape| !ARTS[shape - 1].mark), "no drawing for {}", def.name);
    }
    for name in MARKS {
        assert!(shape_named(name).is_some_and(|shape| ARTS[shape - 1].mark), "no drawing for the mark {name}");
    }
    //(and nothing is drawn that nothing in the game is: the rest are the champions' pictures)
    let champions = (0..CHAMPIONS.len()).filter(|&kind| champion_shape(kind).is_some()).count();
    assert_eq!(ARTS.len(), ENEMY_KINDS.len() + BOSSES.len() + 1 + MARKS.len() + champions);
    assert_eq!(shape_named("nothing"), None);
}

#[test]
fn the_marks_the_game_puts_on_an_enemy_all_have_a_drawing() {
    let mut enemy = Enemy::new(Vec2::new(500.0, 500.0), kind_named("Grunt"), 1.0);
    enemy.modifier = Modifier::Captain;
    enemy.armor = 0.5;
    enemy.frozen = 1.0;
    enemy.shield = 10.0;
    enemy.max_shield = 20.0;
    let marks = marks_of(&enemy);
    assert_eq!(marks.iter().map(|(name, _)| *name).collect::<Vec<_>>(), ["Captain", "Armored", "Frozen", "Shield"]);
    for (name, _) in marks {
        assert!(shape_named(name).is_some(), "{name}");
    }
    //an elite that is armored is not marked twice
    enemy.modifier = Modifier::Armored;
    assert_eq!(marks_of(&enemy).iter().filter(|(name, _)| *name == "Armored").count(), 1);
    for modifier in MODIFIERS {
        assert!(shape_named(modifier.label()).is_some());
    }
}

#[test]
fn a_creature_is_drawn_on_a_square_that_reaches_past_its_hitbox() {
    let creature = Creature::new(shape_named("Grunt").unwrap(), Vec2::new(10.0, 20.0), 44.0, Vec2::new(1.0, 0.0), Color::new(1.0, 0.0, 0.0, 1.0));
    let instance = creature.instance;
    assert_eq!((instance.x, instance.y), (10.0, 20.0));
    //the hitbox is the middle of the picture, so the picture is wider by the same share as the drawing is
    assert_eq!(instance.radius, 22.0 * PICTURE);
    assert_eq!(instance.color, [255, 0, 0, 255]);
    assert_eq!(instance.facing, [1.0, 0.0]);
    //the picture is 160 units across in the drawing and the hitbox 100 of them
    assert!((PICTURE - 160.0 / 100.0).abs() < 1e-6 && SPAN == 80.0);
}

#[test]
fn a_creature_that_does_not_turn_is_drawn_upright() {
    let turning = ARTS.iter().position(|art| art.turns && !art.mark).unwrap() + 1;
    let still = ARTS.iter().position(|art| !art.turns && !art.mark).unwrap() + 1;
    let (aside, up) = (Vec2::new(0.0, 1.0), [0.0, -1.0]);
    assert_eq!(Creature::new(turning, Vec2::ZERO, 50.0, aside, RED).instance.facing, aside.to_array());
    assert_eq!(Creature::new(still, Vec2::ZERO, 50.0, aside, RED).instance.facing, up);
    //nowhere to face is facing up
    assert_eq!(Creature::new(turning, Vec2::ZERO, 50.0, Vec2::ZERO, RED).instance.facing, up);
    //what is not the length of one is made one
    let long = Creature::new(turning, Vec2::ZERO, 50.0, Vec2::new(30.0, 40.0), RED).instance.facing;
    assert!((long[0] - 0.6).abs() < 1e-6 && (long[1] - 0.8).abs() < 1e-6, "{long:?}");
}

#[test]
fn the_seed_and_the_last_stand_change_nothing_else() {
    let plain = Creature::new(shape_named("The Eclipse").unwrap(), Vec2::new(1.0, 2.0), 300.0, Vec2::X, BLUE);
    let seeded = plain.seeded(12345);
    assert_ne!(seeded.instance.seed, 0.0);
    assert_eq!(seeded.instance.seed, plain.seeded(12345).instance.seed);
    assert_ne!(seeded.instance.seed, plain.seeded(12346).instance.seed);
    let hurt = plain.hurt(true);
    assert_eq!(hurt.instance.hurt, 1.0);
    assert_eq!(Creature { instance: Instance { hurt: 0.0, ..hurt.instance }, ..hurt }, plain);
    assert_eq!(hurt.hurt(false), plain);
}

#[test]
fn creatures_are_sorted_into_a_list_for_each_drawing_in_the_order_they_came() {
    let (grunt, scout) = (shape_named("Grunt").unwrap(), shape_named("Scout").unwrap());
    let make = |shape: usize, x: f32| Creature::new(shape, Vec2::new(x, 0.0), 40.0, Vec2::X, RED);
    let mut lists = vec![Vec::new(); ARTS.len()];
    sort_into(&[make(grunt, 1.0), make(scout, 2.0), make(grunt, 3.0)], &mut lists);
    assert_eq!(lists[grunt - 1].iter().map(|instance| instance.x).collect::<Vec<_>>(), [1.0, 3.0]);
    assert_eq!(lists[scout - 1].iter().map(|instance| instance.x).collect::<Vec<_>>(), [2.0]);
    assert_eq!(lists.iter().map(Vec::len).sum::<usize>(), 3);
}

#[test]
fn the_shaders_of_every_drawing_are_whole() {
    let vertex = creature_vertex();
    assert!(vertex.starts_with("#version 100\n"));
    for art in ARTS.iter() {
        let fragment = creature_fragment(art);
        assert!(fragment.starts_with("#version 100\nprecision highp float;\n"), "{}", art.name);
        //the shapes a drawing uses are defined before the drawing, and the drawing before what calls it
        let (helpers, drawing, main) = (fragment.find(HELPERS).unwrap(), fragment.find("vec4 art(").unwrap(), fragment.find("void main()").unwrap());
        assert!(helpers < drawing && drawing < main, "{}", art.name);
        assert_eq!(fragment.matches("vec4 art(").count(), 1, "{}", art.name);
        assert!(fragment.contains(&format!("const float PLAIN = {:?};", if art.mark { 0.0 } else { 1.0 })), "{}", art.name);
        assert!(!fragment.contains('{') || fragment.matches('{').count() == fragment.matches('}').count(), "{} has braces that do not match", art.name);
    }
    //what the vertex shader sends is what the fragment shader reads
    let fragment = creature_fragment(&ARTS[0]);
    for varying in ["color", "spot", "aa", "look", "seed"] {
        assert!(vertex.contains(&format!(" {varying};")) && fragment.contains(&format!(" {varying};")), "{varying}");
    }
    //and the attributes are the fields of an instance, in order
    let attributes: Vec<&str> = vertex.lines().filter_map(|line| line.strip_prefix("attribute ")).collect();
    assert_eq!(attributes, ["vec2 corner;", "vec4 place;", "vec2 facing;", "vec4 tint;", "float hurt;"]);
}

#[test]
fn the_names_and_the_drawings_are_one_each() {
    let mut names: Vec<&str> = ARTS.iter().map(|art| art.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), ARTS.len());
    //the picture is the size of the hitbox it is drawn for, so a drawing is made for a size the game also uses
    for art in ARTS.iter().filter(|art| !art.mark) {
        assert!(art.size >= 20.0 && art.size <= 400.0, "{} {}", art.name, art.size);
    }
}

#[test]
fn what_is_big_is_drawn_over_what_is_small_and_marks_are_drawn_over_the_lot() {
    let order = draw_order();
    assert_eq!(order.len(), ARTS.len());
    let first_mark = order.iter().position(|&which| ARTS[which].mark).unwrap();
    assert!(order[first_mark..].iter().all(|&which| ARTS[which].mark) && order[..first_mark].iter().all(|&which| !ARTS[which].mark));
    assert!(order[..first_mark].windows(2).all(|pair| ARTS[pair[0]].size <= ARTS[pair[1]].size));
    let mut sorted = order.to_vec();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..ARTS.len()).collect::<Vec<_>>(), "every drawing once");
}

#[test]
fn a_camera_shows_the_part_of_the_world_it_looks_at() {
    let camera = Camera2D { target: Vec2::new(100.0, 200.0), zoom: Vec2::new(2.0 * 0.5 / 800.0, 2.0 * 0.5 / 600.0), ..Default::default() };
    let view = camera_view(&camera);
    //(at a zoom of a half, a window of 800 by 600 shows 1600 by 1200)
    assert!((view.w - 1600.0).abs() < 0.01 && (view.h - 1200.0).abs() < 0.01, "{view:?}");
    assert!((view.x + view.w / 2.0 - 100.0).abs() < 0.01 && (view.y + view.h / 2.0 - 200.0).abs() < 0.01);
    //one drawn upside down shows the same
    let flipped = Camera2D { zoom: Vec2::new(camera.zoom.x, -camera.zoom.y), ..camera };
    assert_eq!(camera_view(&flipped), view);
}

// ---------- which way they face ----------

fn turn_for(heading: &mut Heading, at: Vec2, want: Option<Vec2>, seconds: f32) -> Vec2 {
    let steps = (seconds * 60.0).round() as usize;
    let mut facing = heading.facing;
    for _ in 0..steps {
        facing = heading.turn(at, want, 6.0, 1.0 / 60.0);
    }
    facing
}

#[test]
fn a_creature_turns_towards_where_it_has_to_face_at_the_rate_it_turns() {
    let mut heading = Heading::new(Vec2::ZERO, 7, Some(Vec2::new(0.0, -1.0)));
    assert_eq!(heading.facing, Vec2::new(0.0, -1.0));
    //a frame turns it no more than the rate allows
    let after = heading.turn(Vec2::ZERO, Some(Vec2::new(1.0, 0.0)), 6.0, 0.1);
    let turned = Vec2::new(0.0, -1.0).angle_between(after).abs();
    assert!((turned - 0.6).abs() < 1e-4, "{turned}");
    //and a second is enough to get all the way round to the other side (a half turn is 3.14 radians)
    let mut heading = Heading::new(Vec2::ZERO, 7, Some(Vec2::new(0.0, -1.0)));
    let facing = turn_for(&mut heading, Vec2::ZERO, Some(Vec2::new(0.0, 1.0)), 0.4);
    assert!(facing.dot(Vec2::new(0.0, 1.0)) < 0.99, "not there yet after 2.4 radians: {facing}");
    let facing = turn_for(&mut heading, Vec2::ZERO, Some(Vec2::new(0.0, 1.0)), 0.3);
    assert!(facing.dot(Vec2::new(0.0, 1.0)) > 0.99, "{facing}");
    assert!((facing.length() - 1.0).abs() < 1e-4);
}

#[test]
fn a_creature_with_nowhere_to_face_faces_the_way_it_goes() {
    let mut heading = Heading::new(Vec2::ZERO, 3, Some(Vec2::X));
    //standing still, or being jostled a pixel, leaves it as it was
    assert_eq!(turn_for(&mut heading, Vec2::new(0.5, 0.5), None, 1.0), Vec2::X);
    //going down the screen it comes to face down
    let mut at = Vec2::ZERO;
    let mut facing = heading.facing;
    for _ in 0..90 {
        at.y += 3.0;
        facing = heading.turn(at, None, 6.0, 1.0 / 60.0);
    }
    assert!(facing.dot(Vec2::new(0.0, 1.0)) > 0.99, "{facing}");
    //and if it is put somewhere else it does not turn to face the way it was moved
    let before = heading.facing;
    let after = heading.turn(at + Vec2::new(5000.0, 0.0), None, 6.0, 1.0 / 60.0);
    assert!(before.dot(after) > 0.99, "{before} {after}");
}

#[test]
fn a_creature_that_has_not_been_seen_before_faces_somewhere_even_if_it_has_nowhere_to_face() {
    let one = Heading::new(Vec2::ZERO, 1, None);
    let other = Heading::new(Vec2::ZERO, 2, None);
    assert!((one.facing.length() - 1.0).abs() < 1e-5);
    assert_ne!(one.facing, other.facing, "they do not all start the same way");
    //a way that is nothing at all is not a way
    assert!((Heading::new(Vec2::ZERO, 1, Some(Vec2::ZERO)).facing.length() - 1.0).abs() < 1e-5);
}

fn grunt_at(x: f32, y: f32) -> Enemy {
    Enemy::new(Vec2::new(x, y), kind_named("Grunt"), 1.0)
}

#[test]
fn an_enemy_faces_what_it_is_going_for() {
    let players = [Vec2::new(1000.0, 500.0), Vec2::new(500.0, 3000.0)];
    let mut enemy = grunt_at(500.0, 500.0);
    //walking its route: it faces the way it goes
    assert_eq!(wanted_direction(&enemy, &players), None);
    //running at the army: it faces the nearest of the players
    enemy.chasing = true;
    assert_eq!(wanted_direction(&enemy, &players), Some(Vec2::new(500.0, 0.0)));
    assert_eq!(wanted_direction(&enemy, &[]), None);
    //running away or frozen solid: it faces the way it is going
    enemy.rout = 5.0;
    assert_eq!(wanted_direction(&enemy, &players), None);
    enemy.rout = 0.0;
    enemy.frozen = 2.0;
    assert_eq!(wanted_direction(&enemy, &players), None);
    //a charger faces the way it will dash, and then the way it dashes, whoever is near
    enemy.frozen = 0.0;
    enemy.chasing = false;
    enemy.mode = Mode::Windup { dir: Vec2::new(0.0, 1.0), left: 0.5 };
    assert_eq!(wanted_direction(&enemy, &players), Some(Vec2::new(0.0, 1.0)));
    enemy.mode = Mode::Dash { dir: Vec2::new(-1.0, 0.0), left: 0.5 };
    assert_eq!(wanted_direction(&enemy, &[]), Some(Vec2::new(-1.0, 0.0)));
}

#[test]
fn a_shooter_faces_the_player_only_when_it_can_reach_them_and_a_boss_always_does() {
    let mut spitter = Enemy::new(Vec2::new(1000.0, 1000.0), kind_named("Spitter"), 1.0);
    assert!(spitter.range > 0.0);
    let near = [Vec2::new(1000.0 + spitter.range * 0.5, 1000.0)];
    let far = [Vec2::new(1000.0 + spitter.range * 3.0, 1000.0)];
    assert!(wanted_direction(&spitter, &near).is_some());
    assert_eq!(wanted_direction(&spitter, &far), None);
    spitter.chasing = false;
    let boss = Enemy::boss(&BOSSES[0], 10, 0, Vec2::new(1000.0, 1000.0));
    assert!(wanted_direction(&boss, &far).is_some());
}

#[test]
fn a_big_one_turns_slowly_and_a_boss_slower_and_a_charger_showing_where_it_goes_fast() {
    let small = Enemy::new(Vec2::new(500.0, 500.0), kind_named("Scout"), 1.0);
    let mut big = Enemy::new(Vec2::new(500.0, 500.0), kind_named("Colossus"), 1.0);
    big.size = 220.0;
    assert!(turn_rate(&small) > turn_rate(&big));
    let boss = Enemy::boss(&BOSSES[0], 10, 0, Vec2::new(500.0, 500.0));
    assert!(turn_rate(&boss) < turn_rate(&small));
    let mut charger = Enemy::new(Vec2::new(500.0, 500.0), kind_named("Charger"), 1.0);
    charger.mode = Mode::Windup { dir: Vec2::X, left: 1.0 };
    assert!(turn_rate(&charger) > turn_rate(&small));
}

#[test]
fn an_enemy_first_seen_starts_out_facing_where_it_is_walking() {
    let mut enemy = grunt_at(500.0, 500.0);
    enemy.route = vec![Vec2::new(500.0, 900.0), Vec2::new(900.0, 900.0)];
    enemy.heading_for = 0;
    assert_eq!(first_look(&enemy, &[]), Some(Vec2::new(0.0, 400.0)));
    enemy.heading_for = 1;
    assert_eq!(first_look(&enemy, &[]), Some(Vec2::new(400.0, 400.0)));
    enemy.route.clear();
    assert_eq!(first_look(&enemy, &[]), None);
}

// ---------- the setting that puts the squares back ----------

#[test]
fn the_pictures_are_what_enemies_are_drawn_with_unless_the_setting_says_otherwise() {
    assert!(!Settings::default().simple_shapes);
    assert!(!Settings::from_text("master 0.5\n").simple_shapes, "a file from before there were pictures");
    assert!(Settings::from_text("simple_shapes yes\n").simple_shapes);
    assert!(!Settings::from_text("simple_shapes no\n").simple_shapes);
    assert!(!Settings::from_text("simple_shapes sometimes\n").simple_shapes, "what is not yes or no is left as it was");
    let mut settings = Settings::default();
    settings.simple_shapes = true;
    assert!(Settings::from_text(&settings.to_text()).simple_shapes);
}

#[test]
fn simple_shapes_is_a_row_in_the_settings_that_flips() {
    assert!(page_rows(Page::Settings).contains(&Act::SimpleShapes));
    let mut settings = Settings::default();
    assert_eq!(describe(Act::SimpleShapes, &settings), ("Simple shapes", Look::Toggle(false)));
    assert!(nudge(&mut settings, Act::SimpleShapes, 1, false));
    assert!(settings.simple_shapes);
    assert_eq!(describe(Act::SimpleShapes, &settings), ("Simple shapes", Look::Toggle(true)));
    assert!(nudge(&mut settings, Act::SimpleShapes, -1, false), "either way");
    assert!(!settings.simple_shapes);
    assert!(!explain_in(Act::SimpleShapes, &settings, MenuMode::Solo).is_empty());
}
