//the grass the world stands on: there are blades to see at every zoom, and the shader has its numbers pasted in

use super::*;

#[test]
fn there_are_blades_to_see_at_every_zoom_and_the_sizes_fade_from_one_to_the_next() {
    let mut zoom = FURTHEST_ZOOM;
    while zoom <= MAX_ZOOM * 1.01 {
        let fades = grass_fades(zoom);
        assert!(fades.iter().any(|&fade| fade > 0.9), "at zoom {zoom} some size of blade is all there: {fades:?}");
        assert!(fades.iter().all(|fade| (0.0..=1.0).contains(fade)));
        zoom *= 1.05;
    }
    //all the way out it is the biggest blades, close up the smallest, and those never fade away however close
    assert!(grass_fades(MIN_ZOOM)[0] > 0.9 && grass_fades(MIN_ZOOM)[2] == 0.0);
    assert!(grass_fades(MAX_ZOOM)[2] == 1.0 && grass_fades(MAX_ZOOM)[0] == 0.0);
    assert_eq!(grass_fades(50.0)[2], 1.0);
}

#[test]
fn the_grass_shader_has_the_sizes_of_its_blades_pasted_in() {
    let source = grass_fragment_source();
    assert!(!source.contains("CELL_"), "every size is pasted in");
    for cell in GRASS_CELLS {
        assert!(source.contains(&format!("{cell:?}")), "{cell}");
    }
    assert!(source.starts_with("#version 100"));
}
