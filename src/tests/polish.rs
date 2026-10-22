//the finishing touches: kill streaks, the failing army, marks on the ground, debris, the camera punch, positioned and climbing
//sounds, and the music

use super::*;
use super::sound::{seconds_of, zero_crossings};

// ---------- kill streaks ----------

#[test]
fn a_kill_streak_grows_with_each_kill_and_ends_when_the_window_runs_out() {
    let mut combo = Combo::default();
    assert_eq!(combo.add(0), None);
    assert_eq!(combo.count, 0);
    assert_eq!(combo.add(3), None);
    combo.update(COMBO_WINDOW * 0.9);
    assert_eq!(combo.count, 3, "still going");
    combo.add(2);
    combo.update(COMBO_WINDOW * 0.9);
    assert_eq!(combo.count, 5, "the new kills gave it a new window");
    combo.update(COMBO_WINDOW * 0.2);
    assert_eq!(combo.count, 0);
    assert!(combo.name().is_none());
}

#[test]
fn each_step_of_a_streak_is_announced_once_even_when_a_lot_are_destroyed_at_once() {
    let mut combo = Combo::default();
    let mut announced = Vec::new();
    for _ in 0..450 {
        if let Some((size, _)) = combo.add(1) {
            announced.push(size);
        }
    }
    assert_eq!(announced, COMBO_STEPS.iter().map(|&(size, _)| size).collect::<Vec<_>>());
    //a jump over several steps announces the highest
    let mut combo = Combo::default();
    assert_eq!(combo.add(120).map(|(size, _)| size), Some(100));
    assert_eq!(combo.name(), Some("CARNAGE"));
    assert!(COMBO_STEPS.windows(2).all(|pair| pair[0].0 < pair[1].0));
}

// ---------- a failing army ----------

#[test]
fn an_army_is_only_in_peril_once_it_has_lost_most_of_what_it_had() {
    //a run that never got anywhere has nothing to lose
    assert_eq!(peril(0, PERIL_MIN_PEAK - 1), 0.0);
    assert_eq!(peril(2, 10), 0.0);
    let peak = 1000;
    assert_eq!(peril(peak, peak), 0.0);
    assert_eq!(peril((peak as f32 * PERIL_FROM) as usize + 1, peak), 0.0);
    let hurt = peril(100, peak);
    let nearly_gone = peril(10, peak);
    assert!(hurt > 0.0 && nearly_gone > hurt && nearly_gone < 1.0, "{hurt} {nearly_gone}");
    assert_eq!(peril(0, peak), 1.0);
}

// ---------- the camera ----------

#[test]
fn the_camera_punch_stops_at_full_and_settles_back_to_nothing() {
    let mut fx = Effects::default();
    fx.add_punch(0.8);
    fx.add_punch(0.8);
    assert_eq!(fx.punch, 1.0);
    let mut last = fx.punch;
    for _ in 0..600 {
        fx.update(1.0 / 60.0, Vec2::ZERO);
        assert!(fx.punch <= last);
        last = fx.punch;
    }
    assert_eq!(fx.punch, 0.0);
}

// ---------- units appearing ----------

#[test]
fn a_unit_that_has_just_grown_swells_from_a_green_speck_and_a_full_grown_one_is_plain() {
    let view = Rect::new(0.0, 0.0, 1000.0, 1000.0);
    let mut baby_grown = new_unit(Vec2::new(100.0, 100.0));
    let old = Unit::new(200.0, 200.0, 300.0);
    assert!(baby_grown.age < UNIT_GROW_SECONDS && old.age >= UNIT_GROW_SECONDS);
    let discs = unit_discs(&[baby_grown.clone(), old.clone()], view);
    assert!(discs[0].radius < RADIUS * 0.5, "{}", discs[0].radius);
    assert!(discs[0].color[1] > discs[0].color[0], "it starts green");
    assert_eq!(discs[1].radius, RADIUS);
    assert_eq!(discs[1].color, [255, 255, 255, 255]);
    //it ends up the same size and colour
    baby_grown.age = UNIT_GROW_SECONDS;
    let grown = unit_discs(&[baby_grown], view);
    assert_eq!(grown[0].radius, RADIUS);
    //in between it is bigger than at the start and it never goes wild
    let mut seen = Vec::new();
    for step in 0..=10 {
        let mut unit = new_unit(Vec2::new(100.0, 100.0));
        unit.age = UNIT_GROW_SECONDS * step as f32 / 10.0;
        seen.push(unit_discs(&[unit], view)[0].radius);
    }
    assert!(seen[1] > seen[0] && seen.iter().all(|&radius| radius > 0.0 && radius < RADIUS * 1.3), "{seen:?}");
    //units age as the frames go by
    let mut units = vec![new_unit(Vec2::ZERO)];
    heal_units(&mut units, 0.1, &Upgrades::default());
    assert!((units[0].age - 0.1).abs() < 1e-6);
}

// ---------- marks on the ground ----------

fn mark_at(x: f32, kind: MarkKind) -> Mark {
    Mark { position: Vec2::new(x, 500.0), radius: 80.0, color: RED, kind, age: 0.0 }
}

#[test]
fn marks_are_kept_for_a_while_then_fade_and_the_oldest_go_first_when_there_are_too_many() {
    let mut ground = Vec::new();
    let mut fresh: Vec<Mark> = (0..MAX_MARKS + 100).map(|i| mark_at(i as f32, MarkKind::Splat)).collect();
    update_marks(&mut ground, &mut fresh, 0.0);
    assert!(fresh.is_empty());
    assert_eq!(ground.len(), MAX_MARKS);
    assert_eq!(ground[0].position.x, 100.0, "the first hundred were dropped");
    update_marks(&mut ground, &mut Vec::new(), MARK_LIFE + 1.0);
    assert!(ground.is_empty());
    //one request in a frame is limited too
    let mut fx = Effects::default();
    for _ in 0..MAX_MARKS_A_FRAME * 3 {
        fx.mark(Vec2::ZERO, 10.0, RED, MarkKind::Scorch);
    }
    assert_eq!(fx.marks.len(), MAX_MARKS_A_FRAME);
}

#[test]
fn marks_are_drawn_as_a_few_discs_only_when_they_are_in_view_and_fade_out_at_the_end_of_their_life() {
    let view = Rect::new(0.0, 0.0, 1000.0, 1000.0);
    let marks = vec![mark_at(500.0, MarkKind::Scorch), mark_at(5000.0, MarkKind::Splat), mark_at(300.0, MarkKind::Splat)];
    let discs = mark_discs(&marks, view);
    assert!(!discs.is_empty());
    assert!(discs.iter().all(|disc| disc.x > -200.0 && disc.x < 1200.0), "the one far away isn't drawn");
    //the same marks always make the same discs, a mark does not shimmer
    assert_eq!(discs, mark_discs(&marks, view));
    assert!(discs.iter().all(|disc| disc.radius > 0.0 && disc.radius <= 80.0 * 1.3 && disc.color[3] > 0));
    //a scorch glows when it is new and not when it is old
    let alpha_total = |marks: &[Mark]| mark_discs(marks, view).iter().map(|disc| disc.color[3] as u32).sum::<u32>();
    let mut scorch = mark_at(500.0, MarkKind::Scorch);
    let new = alpha_total(&[scorch.clone()]);
    scorch.age = MARK_GLOW + 1.0;
    let settled = alpha_total(&[scorch.clone()]);
    assert!(new > settled);
    //fading: the last of its life it gets fainter and fainter
    scorch.age = MARK_LIFE - MARK_FADE / 2.0;
    let fading = alpha_total(&[scorch.clone()]);
    scorch.age = MARK_LIFE - 0.5;
    let almost_gone = alpha_total(&[scorch]);
    assert!(settled > fading && fading > almost_gone, "{settled} {fading} {almost_gone}");
}

#[test]
fn what_blows_up_leaves_a_scorch_and_what_is_destroyed_leaves_a_splat_and_debris() {
    let mut fx = Effects::default();
    let mut hazards = vec![hazard_at(MIDDLE, 120.0, 0.0, 10.0, 3)];
    update_hazards(&mut hazards, &mut [], &Upgrades::default(), 0.1, &mut fx);
    assert!(fx.marks.iter().any(|mark| mark.kind == MarkKind::Scorch && mark.position == MIDDLE && mark.radius > 60.0));
    let mut fx = Effects::default();
    let mut enemies = vec![still_enemy(MIDDLE, 1.0)];
    enemies[0].health = 0.0;
    let destroyed = resolve_deaths(&mut enemies, &mut Vec::new(), 10, &Upgrades::default(), &mut fx);
    assert_eq!(destroyed, 1);
    assert!(fx.marks.iter().any(|mark| mark.kind == MarkKind::Splat));
    assert_eq!(fx.shards.len(), 1);
}

// ---------- debris ----------

#[test]
fn a_destroyed_enemy_throws_chips_that_spin_slow_down_and_are_gone_in_a_second() {
    let mut shards = Vec::new();
    let mut small = vec![ShardBurst { position: MIDDLE, color: RED, size: 30.0 }];
    throw_shards(&mut shards, &mut small);
    let few = shards.len();
    assert!(small.is_empty() && few >= 3);
    let mut big = vec![ShardBurst { position: MIDDLE, color: RED, size: 400.0 }];
    throw_shards(&mut shards, &mut big);
    assert!(shards.len() - few > few, "a big one breaks into more pieces");
    assert!(shards.iter().all(|shard| shard.size >= 4.0 && shard.size <= 46.0 && shard.life <= 1.1));
    let fastest = |shards: &[Shard]| shards.iter().map(|shard| shard.velocity.length()).fold(0.0, f32::max);
    let before = fastest(&shards);
    update_shards(&mut shards, 0.3);
    assert!(fastest(&shards) < before);
    update_shards(&mut shards, 1.5);
    assert!(shards.is_empty());
    //there is a limit to how many are in the air
    let mut crowd = Vec::new();
    for _ in 0..200 {
        let mut burst = vec![ShardBurst { position: MIDDLE, color: RED, size: 600.0 }];
        throw_shards(&mut crowd, &mut burst);
    }
    assert_eq!(crowd.len(), MAX_SHARDS);
}

// ---------- sounds ----------

#[test]
fn a_sound_is_quieter_the_further_away_it_was_and_never_silent() {
    assert_eq!(distance_share(0.0), 1.0);
    assert!((distance_share(SOUND_RANGE) - SOUND_FAR_SHARE).abs() < 1e-6);
    assert_eq!(distance_share(SOUND_RANGE * 10.0), SOUND_FAR_SHARE);
    let mut last = 1.0;
    for step in 1..=20 {
        let share = distance_share(SOUND_RANGE * step as f32 / 20.0);
        assert!(share < last + 1e-6 && share > 0.0);
        last = share;
    }
}

#[test]
fn the_cues_come_out_with_where_they_happened_and_none_are_left_behind() {
    let mut fx = Effects::default();
    fx.cue(Cue::Pop);
    fx.cue_at(Cue::Thud, Vec2::new(5.0, 6.0));
    fx.cue(Cue::Death);
    assert_eq!(fx.cues, vec![Cue::Pop, Cue::Thud, Cue::Death], "the old way of looking at them still works");
    let taken = fx.take_cues();
    assert_eq!(taken, vec![(Cue::Pop, None), (Cue::Thud, Some(Vec2::new(5.0, 6.0))), (Cue::Death, None)]);
    assert!(fx.cues.is_empty() && fx.take_cues().is_empty());
    //cues cleared by hand don't leave the places out of step with the next ones
    fx.cue_at(Cue::Arrow, Vec2::ONE);
    fx.cues.clear();
    fx.cue(Cue::Alert);
    let taken = fx.take_cues();
    assert_eq!(taken.len(), 1);
}

#[test]
fn the_pop_climbs_a_scale_while_babies_keep_growing_and_starts_again_after_a_pause() {
    let mut streak = PopStreak::default();
    let mut heard = Vec::new();
    for k in 0..16 {
        heard.push(streak.next(10.0 + k as f64 * 0.05));
    }
    assert_eq!(heard[0], Cue::Pop);
    assert_eq!(*heard.last().unwrap(), Cue::Pop6, "it tops out");
    //a pause, and it begins again at the bottom
    assert_eq!(streak.next(20.0), Cue::Pop);
    //each pop is higher than the last
    let pitch = |cue| zero_crossings(&synthesize(cue)) as f32 / seconds_of(&synthesize(cue));
    let pops = [Cue::Pop, Cue::Pop2, Cue::Pop3, Cue::Pop4, Cue::Pop5, Cue::Pop6];
    assert!(pops.windows(2).all(|pair| pitch(pair[1]) > pitch(pair[0])), "{:?}", pops.map(pitch));
}

#[test]
fn the_heartbeat_is_two_low_thumps_and_the_squad_sting_falls() {
    let beat = synthesize(Cue::Heartbeat);
    assert!(seconds_of(&beat) < 0.6);
    let pitch = zero_crossings(&beat) as f32 / seconds_of(&beat) / 2.0;
    assert!(pitch < 150.0, "{pitch} Hz is too high for a heart");
    //a gap of quiet between the two beats
    let at = |seconds: f32| (seconds * SAMPLE_RATE as f32) as usize;
    let peak = |from: f32, to: f32| beat[at(from)..at(to)].iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!(peak(0.0, 0.05) > 0.5 && peak(0.2, 0.25) > 0.3 && peak(0.15, 0.19) < peak(0.0, 0.05) * 0.5);
    let sting = synthesize(Cue::SquadLost);
    let half = sting.len() / 2;
    assert!(zero_crossings(&sting[..half]) + 2 > zero_crossings(&sting[half..]));
}

// ---------- the music ----------

#[test]
fn every_layer_of_the_music_is_a_clean_loop_of_the_same_length() {
    let mut lengths = HashSet::new();
    for layer in 0..MUSIC_LAYERS {
        let samples = synthesize_layer(layer);
        lengths.insert(samples.len());
        assert!((seconds_of(&samples) - LOOP_SECONDS).abs() < 0.01, "layer {layer} is {} s", seconds_of(&samples));
        assert!(samples.iter().all(|sample| sample.is_finite()));
        let peak = samples.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!((peak - 0.8).abs() < 0.001, "layer {layer} peaks at {peak}");
        //it comes round to where it started without a click: the loop's ends are quiet
        assert!(samples[0].abs() < 0.05 && samples[samples.len() - 1].abs() < 0.05, "layer {layer} clicks at the join");
        let rms = (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt();
        assert!(rms > 0.03 && rms < 0.6, "layer {layer} rms {rms}");
    }
    assert_eq!(lengths.len(), 1, "the layers drift apart if their lengths differ");
}

#[test]
fn the_music_layers_are_different_and_the_same_every_time() {
    let layers: Vec<Vec<f32>> = (0..MUSIC_LAYERS).map(synthesize_layer).collect();
    for (i, a) in layers.iter().enumerate() {
        for b in &layers[i + 1..] {
            assert_ne!(a, b);
        }
    }
    assert_eq!(layers[3], synthesize_layer(3));
    //the drums hit on the beat: loud right at the start of a beat, quieter just before it
    let drums = &layers[3];
    let at = |seconds: f32| (seconds * SAMPLE_RATE as f32) as usize;
    let peak = |from: f32, to: f32| drums[at(from)..at(to)].iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!(peak(BEAT * 4.0, BEAT * 4.0 + 0.05) > 0.3);
    assert!(peak(BEAT * 4.0 - 0.05, BEAT * 4.0 - 0.01) < peak(BEAT * 4.0, BEAT * 4.0 + 0.05));
}

#[test]
fn the_mix_follows_the_size_of_the_army_the_danger_a_boss_and_a_failing_army() {
    let calm = layer_targets(&Mood { army: 1, ..Mood::default() });
    assert!(calm[0] > 0.3, "the pad is always there");
    assert!(calm[1..].iter().all(|&volume| volume == 0.0), "{calm:?}");
    let grown = layer_targets(&Mood { army: 600, ..Mood::default() });
    assert!(grown[1] > 0.0 && grown[3] == 0.0);
    let big = layer_targets(&Mood { army: 4000, ..Mood::default() });
    assert!(big[1] >= grown[1] && big[2] > grown[2] && big[2] > 0.3, "the lead comes in for a big army: {big:?}");
    let fighting = layer_targets(&Mood { army: 600, danger: 1.0, ..Mood::default() });
    assert!(fighting[3] > 0.5 && fighting[0] < grown[0]);
    let boss = layer_targets(&Mood { army: 600, boss: true, ..Mood::default() });
    assert!(boss[3] > 0.5 && boss[4] > 0.5, "a boss brings the drums and the drone: {boss:?}");
    let failing = layer_targets(&Mood { army: 300, peril: 1.0, ..Mood::default() });
    assert!(failing[4] > 0.5 && failing[2] == 0.0);
    for mood in [Mood::default(), Mood { army: 100_000, danger: 5.0, boss: true, peril: 0.0 }] {
        assert!(layer_targets(&mood).iter().all(|&volume| (0.0..=1.0).contains(&volume)));
    }
}

#[test]
fn a_volume_fades_at_a_steady_rate_and_stops_at_its_target() {
    assert_eq!(fade_towards(0.0, 1.0, 0.25), 0.25);
    assert_eq!(fade_towards(0.9, 1.0, 0.25), 1.0);
    assert_eq!(fade_towards(1.0, 0.2, 0.25), 0.75);
    assert_eq!(fade_towards(0.3, 0.3, 0.25), 0.3);
}

// ---------- squads ----------

#[test]
fn a_squad_that_has_all_died_is_reported_where_its_enemy_is_and_one_that_was_recalled_is_not() {
    let enemy = still_enemy(MIDDLE, 1.0e6);
    let id = enemy.id;
    let enemies = vec![enemy];
    let mut squads = Squads::new();
    squads.insert(id, Squad { position: MIDDLE, field: FlowField::new(), next_field: FlowField::new() });
    let mut sent = unit_on(MIDDLE + Vec2::new(100.0, 0.0));
    sent.target = Some(id);
    let mut units = vec![sent];
    assert!(update_squads(&mut squads, &enemies, &mut units, &Walls::default(), false).is_empty());
    assert!(squads.contains_key(&id));
    //every unit of it dies
    units.clear();
    let lost = update_squads(&mut squads, &enemies, &mut units, &Walls::default(), false);
    assert_eq!(lost, vec![MIDDLE]);
    assert!(squads.is_empty());
    //a recall removes the squad first, so there is nothing to mourn
    let mut sent = unit_on(MIDDLE);
    sent.target = Some(id);
    let mut units = vec![sent];
    squads.insert(id, Squad { position: MIDDLE, field: FlowField::new(), next_field: FlowField::new() });
    recall_squads(&mut units, &mut squads, MIDDLE, &mut Effects::default());
    assert!(update_squads(&mut squads, &enemies, &mut units, &Walls::default(), false).is_empty());
    //an enemy that was destroyed is no loss either
    squads.insert(id, Squad { position: MIDDLE, field: FlowField::new(), next_field: FlowField::new() });
    assert!(update_squads(&mut squads, &[], &mut units, &Walls::default(), false).is_empty());
}
