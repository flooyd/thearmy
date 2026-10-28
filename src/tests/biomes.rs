//the patches of ice, lava and swamp: where they are laid, what they do to what stands on them, and that the shader is given what it needs

use super::*;

fn patch(biome: Biome) -> Patch {
    Patch { biome, position: MIDDLE, radius: 1500.0 }
}

#[test]
fn a_patch_holds_what_is_well_inside_it_and_not_what_is_well_outside_with_a_ragged_edge_between() {
    let ice = patch(Biome::Ice);
    assert!(ice.holds(MIDDLE) && ice.holds(MIDDLE + Vec2::new(1500.0 * (1.0 - PATCH_WOBBLE) - 1.0, 0.0)));
    assert!(!ice.holds(MIDDLE + Vec2::new(0.0, 1500.0 * (1.0 + PATCH_WOBBLE) + 1.0)));
    assert_eq!(biome_at(&[ice], MIDDLE), Some(Biome::Ice));
    assert_eq!(biome_at(&[ice], Vec2::splat(100.0)), None);
    //round the edge some places are on it and some are not, so it is ragged, not a circle
    let round: Vec<bool> = (0..360).map(|k| ice.holds(MIDDLE + Vec2::from_angle((k as f32).to_radians()) * 1500.0)).collect();
    let on = round.iter().filter(|&&on| on).count();
    assert!(on > 30 && on < 330, "{on} of 360 round the edge are on it");
    //and the noise that does it is between 0 and 1, and the same each time
    for k in 0..500 {
        let at = Vec2::new(k as f32 * 137.3, k as f32 * 71.9);
        let n = edge_noise(at);
        assert!((0.0..=1.0).contains(&n) && n == edge_noise(at));
    }
}

#[test]
fn the_patches_are_laid_from_the_sites_clear_of_the_start_the_sites_the_edge_and_each_other() {
    rand::srand(11);
    let sites = generate_sites();
    let patches = lay_patches(&sites);
    assert_eq!(patches, lay_patches(&sites), "the same sites, the same ground");
    let mut moved = sites.clone();
    moved[0].position.x += 64.0;
    assert_ne!(patches, lay_patches(&moved), "and other sites, other ground");
    for biome in BIOMES {
        let count = patches.iter().filter(|patch| patch.biome == biome).count();
        assert!(count >= 2, "{count} of {biome:?}");
    }
    let centre = Vec2::splat(WORLD_SIZE / 2.0);
    for (i, one) in patches.iter().enumerate() {
        assert!(one.radius >= PATCH_RADII.0 && one.radius <= PATCH_RADII.1);
        assert!(one.position.distance(centre) >= one.reach() + SITE_START_CLEAR_TILES * TILE_SIZE, "nothing at the start");
        assert!(one.position.x - one.reach() >= PATCH_GAP && one.position.x + one.reach() <= WORLD_SIZE - PATCH_GAP);
        assert!(one.position.y - one.reach() >= PATCH_GAP && one.position.y + one.reach() <= WORLD_SIZE - PATCH_GAP);
        assert!(sites.iter().all(|site| !one.holds(site.position)), "no site stands on one");
        for other in &patches[i + 1..] {
            assert!(one.position.distance(other.position) >= one.reach() + other.reach(), "they don't touch");
        }
    }
    assert_eq!(biome_at(&patches, centre), None);
}

#[test]
fn the_shader_is_given_the_patches_in_view_nearest_the_middle_first_and_no_more_than_it_takes() {
    let view = Rect::new(MIDDLE.x - 1000.0, MIDDLE.y - 600.0, 2000.0, 1200.0);
    let at = |dx: f32, biome: Biome| Patch { biome, position: MIDDLE + Vec2::new(dx, 0.0), radius: 1000.0 };
    let far = at(4000.0, Biome::Lava);
    assert!(patches_in(&[far], view).is_empty(), "one wholly off the screen is left out");
    let shown = patches_in(&[at(1800.0, Biome::Ice), at(0.0, Biome::Swamp), far], view);
    assert_eq!(shown.iter().map(|patch| patch.biome).collect::<Vec<_>>(), vec![Biome::Swamp, Biome::Ice]);
    let crowd: Vec<Patch> = (0..20).map(|k| at(k as f32 * 10.0, Biome::Ice)).collect();
    assert_eq!(patches_in(&crowd, view).len(), MAX_PATCHES_DRAWN);
    let source = grass_fragment_source();
    for left in ["MAX_PATCHES", "EDGE_SCALE", "WOBBLE", "RIM", "CELL_"] {
        assert!(!source.contains(left), "{left} is not pasted in");
    }
}

#[test]
fn a_swamp_bogs_down_ice_chills_and_lava_sets_alight_the_enemies_on_them() {
    //(and whether it was frozen at any time on the way)
    let stood = |biome: Biome, seconds: f32| {
        let mut enemies = vec![Enemy::new(MIDDLE, kind_named("Brute"), 1.0)];
        let mut froze = false;
        for _ in 0..(seconds * 60.0) as usize {
            terrain_effects(&mut enemies, &Walls::default(), 1.0 / 60.0, &mut Effects::default());
            ground_effects_on_enemies(&mut enemies, &[patch(biome)], 1.0 / 60.0);
            tick_statuses(&mut enemies, 1.0 / 60.0);
            froze |= enemies[0].status.frozen > 0.0;
        }
        (enemies.remove(0), froze)
    };
    let on = |biome: Biome, seconds: f32| stood(biome, seconds).0;
    let swamped = on(Biome::Swamp, 0.1);
    assert!((speed_kept(&swamped) - SWAMP_KEPT).abs() < 1e-4);
    let iced = on(Biome::Ice, 1.0);
    assert!(iced.status.chill > 0.0 && speed_kept(&iced) < 1.0);
    assert!(stood(Biome::Ice, 12.0).1, "left on it long enough, one freezes");
    assert!(!stood(Biome::Ice, 3.0).1, "but not at once");
    let burnt = on(Biome::Lava, 1.0);
    assert!(burnt.status.burning > 0.0 && burnt.health < burnt.max_health);
    //and off them nothing
    let mut enemies = vec![Enemy::new(Vec2::splat(500.0), kind_named("Brute"), 1.0)];
    ground_effects_on_enemies(&mut enemies, &[patch(Biome::Lava)], 0.5);
    assert_eq!((enemies[0].status.burning, enemies[0].status.slow), (0.0, 1.0));
}

#[test]
fn the_army_is_bogged_down_in_a_swamp_slides_on_ice_and_burns_on_lava() {
    let dt = 1.0 / 60.0;
    //a unit that meant to go 10 pixels to the right this frame, having been going up
    let walked = |biome: Biome| {
        let mut units = vec![unit_on(MIDDLE)];
        units[0].velocity = Vec2::new(0.0, -300.0);
        let footing = footing_of(&units, &[patch(biome)], &[MIDDLE]);
        assert_eq!(footing.len(), 1);
        units[0].x += 10.0;
        ground_the_army(&mut units, &footing, &Upgrades::default(), dt);
        units.remove(0)
    };
    let swamp = walked(Biome::Swamp);
    assert!((swamp.x - (MIDDLE.x + 10.0 * SWAMP_KEPT)).abs() < 1e-3 && swamp.y == MIDDLE.y);
    let ice = walked(Biome::Ice);
    assert!(ice.x > MIDDLE.x && ice.x < MIDDLE.x + 10.0 && ice.y < MIDDLE.y, "it carries on up a while: {:?}", (ice.x - MIDDLE.x, ice.y - MIDDLE.y));
    let lava = walked(Biome::Lava);
    assert_eq!(lava.x, MIDDLE.x + 10.0);
    assert!((lava.health - (UNIT_HEALTH - UNIT_HEALTH * LAVA_BURN * dt)).abs() < 1e-3);
    //left on lava, a unit dies of it, and lava is what the defeat screen says killed it
    let mut units = vec![unit_on(MIDDLE)];
    for _ in 0..(2.0 / LAVA_BURN / dt) as usize {
        let footing = footing_of(&units, &[patch(Biome::Lava)], &[MIDDLE]);
        ground_the_army(&mut units, &footing, &Upgrades::default(), dt);
    }
    assert!(units[0].health <= 0.0 && cause_label(units[0].bane) == "Lava");
    //a patch far from every player is not looked at, and a unit in a tower is above it
    assert!(footing_of(&[unit_on(MIDDLE)], &[patch(Biome::Lava)], &[MIDDLE + Vec2::splat(30000.0)]).is_empty());
    let mut towered = unit_on(MIDDLE);
    towered.garrison = Some((0, 0));
    assert!(footing_of(&[towered], &[patch(Biome::Lava)], &[MIDDLE]).is_empty());
}

#[test]
fn a_player_walks_slower_in_a_swamp_and_on_the_host_and_a_guest_alike() {
    assert_eq!(ground_pace(&[patch(Biome::Swamp)], MIDDLE), SWAMP_KEPT);
    assert_eq!(ground_pace(&[patch(Biome::Ice)], MIDDLE), 1.0);
    assert_eq!(ground_pace(&[patch(Biome::Swamp)], Vec2::splat(500.0)), 1.0);
}
