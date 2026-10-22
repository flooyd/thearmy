//the synthesised sounds

use super::*;


// ---------- the synthesised sounds ----------

pub(super) fn seconds_of(samples: &[f32]) -> f32 {
    samples.len() as f32 / SAMPLE_RATE as f32
}

//how many times the wave crosses zero in a stretch of samples, which is about twice its pitch in Hz times the seconds
pub(super) fn zero_crossings(samples: &[f32]) -> usize {
    samples.windows(2).filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0)).count()
}

#[test]
fn every_cue_is_a_clean_finite_sound_of_a_sensible_length_and_loudness() {
    for cue in ALL_CUES {
        let samples = synthesize(cue);
        assert!(!samples.is_empty(), "{cue:?}");
        assert!(samples.iter().all(|sample| sample.is_finite()), "{cue:?}");
        let peak = samples.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!((peak - SOUND_PEAK).abs() < 0.001, "{cue:?} peaks at {peak}");
        assert!(seconds_of(&samples) > 0.05 && seconds_of(&samples) < 2.0, "{cue:?} is {} s", seconds_of(&samples));
        //it doesn't start or stop with a click
        let edge = 5; //samples, a quarter of a millisecond
        assert!(samples[..edge].iter().all(|sample| sample.abs() < 0.1), "{cue:?} starts loud");
        assert!(samples[samples.len() - edge..].iter().all(|sample| sample.abs() < 0.05), "{cue:?} ends loud");
        assert!(samples[0].abs() < 0.01);
        //it has some body, not one spike
        let rms = (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt();
        assert!(rms > 0.02 && rms < 0.7, "{cue:?} rms {rms}");
    }
}

#[test]
fn the_cues_differ_in_length_the_way_they_should() {
    let length = |cue| seconds_of(&synthesize(cue));
    assert!(length(Cue::Pop) < 0.15);
    assert!(length(Cue::Death) < 0.15);
    assert!(length(Cue::Defeat) > 1.0);
    assert!(length(Cue::Milestone) > length(Cue::Alert));
    assert!(length(Cue::Thud) > length(Cue::Shoot));
}

#[test]
fn no_two_cues_sound_the_same() {
    for (i, a) in ALL_CUES.iter().enumerate() {
        for b in &ALL_CUES[i + 1..] {
            assert_ne!(synthesize(*a), synthesize(*b), "{a:?} and {b:?}");
        }
    }
}

#[test]
fn the_same_cue_always_sounds_exactly_the_same() {
    for cue in ALL_CUES {
        assert_eq!(wav_bytes(&synthesize(cue)), wav_bytes(&synthesize(cue)));
    }
}

#[test]
fn the_pop_rises_in_pitch() {
    let samples = synthesize(Cue::Pop);
    let half = samples.len() / 2;
    assert!(zero_crossings(&samples[half..]) > zero_crossings(&samples[..half]) + 3);
}

#[test]
fn the_defeat_has_a_rumble_that_falls() {
    //after the hiss at the start is over, the pitch only goes down
    let samples = synthesize(Cue::Defeat);
    let from = (0.4 * SAMPLE_RATE as f32) as usize;
    let rest = &samples[from..];
    let third = rest.len() / 3;
    assert!(zero_crossings(&rest[..third]) > zero_crossings(&rest[2 * third..]));
}

#[test]
fn a_wav_file_has_the_header_and_the_samples_the_loader_expects() {
    let samples = [0.0, 0.5, -0.5, 1.0, -1.0, 2.0];
    let wav = wav_bytes(&samples);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[12..16], b"fmt ");
    let word = |at: usize| u32::from_le_bytes(wav[at..at + 4].try_into().unwrap());
    let half = |at: usize| u16::from_le_bytes(wav[at..at + 2].try_into().unwrap());
    assert_eq!(word(4) as usize, wav.len() - 8);
    assert_eq!(word(16), 16);
    assert_eq!(half(20), 1, "pcm");
    assert_eq!(half(22), 1, "mono");
    assert_eq!(word(24), SAMPLE_RATE);
    assert_eq!(word(28), SAMPLE_RATE * 2);
    assert_eq!(half(32), 2);
    assert_eq!(half(34), 16);
    assert_eq!(&wav[36..40], b"data");
    assert_eq!(word(40) as usize, samples.len() * 2);
    assert_eq!(wav.len(), 44 + samples.len() * 2);
    let sample = |i: usize| i16::from_le_bytes(wav[44 + 2 * i..46 + 2 * i].try_into().unwrap());
    assert_eq!(sample(0), 0);
    assert!((sample(1) as i32 - 16383).abs() <= 1);
    assert!((sample(2) as i32 + 16383).abs() <= 1);
    assert_eq!(sample(3), i16::MAX);
    assert_eq!(sample(4), -i16::MAX);
    //past full is held there, not wrapped round
    assert_eq!(sample(5), i16::MAX);
}

#[test]
fn a_hundred_of_the_same_cue_in_a_frame_sound_once_and_again_after_the_cooldown() {
    let mut gate = CueGate::new();
    let admitted = (0..100).filter(|_| gate.admit(Cue::Pop, 10.0)).count();
    assert_eq!(admitted, 1);
    assert!(!gate.admit(Cue::Pop, 10.0 + Cue::Pop.cooldown() as f64 * 0.5));
    assert!(gate.admit(Cue::Pop, 10.0 + Cue::Pop.cooldown() as f64 * 1.01));
}

#[test]
fn different_cues_do_not_hold_each_other_up() {
    let mut gate = CueGate::new();
    for cue in ALL_CUES {
        assert!(gate.admit(cue, 5.0), "{cue:?}");
    }
    for cue in ALL_CUES {
        assert!(!gate.admit(cue, 5.0), "{cue:?}");
    }
}

#[test]
fn every_cue_has_its_own_slot_and_sensible_volume_and_cooldown() {
    for (i, cue) in ALL_CUES.iter().enumerate() {
        assert_eq!(*cue as usize, i, "{cue:?} is not where ALL_CUES has it");
        assert!(cue.volume() > 0.0 && cue.volume() <= 1.0);
        assert!(cue.cooldown() > 0.0 && cue.cooldown() < 10.0);
    }
    //the biggest events aren't drowned out by the busiest ones
    assert!(Cue::Milestone.volume() > Cue::Pop.volume());
    assert!(Cue::Thud.volume() > Cue::Shoot.volume());
}

#[test]
fn the_cues_a_fight_makes_are_the_ones_that_are_synthesised() {
    //the cues the fighting code asks for are all in ALL_CUES, so each has a sound to play
    let at = Vec2::new(16000.0, 16000.0);
    let mut fx = Effects::default();
    let mut enemies = vec![still_enemy(at, 10.0)];
    let mut units = vec![unit_on(at)];
    fight(&mut enemies, &mut units, &mut Vec::new(), 0.2, &Upgrades::default(), &mut fx);
    units[0].health = 0.0;
    bury_units(&mut units, &mut fx);
    for cue in &fx.cues {
        assert!(ALL_CUES.contains(cue));
    }
    assert!(fx.cues.contains(&Cue::Thud) && fx.cues.contains(&Cue::Death));
}
