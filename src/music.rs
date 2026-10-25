//! the music, made in code like the sounds: five loops of the same length that play together, each fading in and out to suit
//! what is going on. a pad is always there, a bass comes in as the army grows, an arpeggio over it when the army is big, drums
//! when there is danger, and a low drone when a boss is about or the army is failing

use crate::prelude::*;

pub(crate) const BEATS_PER_MINUTE: f32 = 96.0;
pub(crate) const BEAT: f32 = 60.0 / BEATS_PER_MINUTE;
pub(crate) const BARS: usize = 8;
pub(crate) const LOOP_SECONDS: f32 = BEAT * 4.0 * BARS as f32; //20 seconds, then it starts again
pub(crate) const MUSIC_LAYERS: usize = 5;
pub(crate) const MUSIC_GAIN: f32 = 0.7; //how loud the music is next to the sounds, which it must not drown (0.3 was lost under a fight)
pub(crate) const MUSIC_FADE: f32 = 0.45; //how much of a layer's volume it gains or loses a second

//what the music follows: how big the army is, how much danger there is, whether a boss is about, and how close the army is to
//being gone
#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct Mood {
    pub(crate) army: usize,
    pub(crate) danger: f32, // 0 to 1
    pub(crate) boss: bool,
    pub(crate) peril: f32, // 0 to 1
}

//the music of the title screen: calm, with the bass and a little of the arpeggio, and no drums
pub(crate) const TITLE_MOOD: Mood = Mood { army: 1200, danger: 0.0, boss: false, peril: 0.0 };
//and of the defeat screen: the pad and the low drone
pub(crate) const DEFEAT_MOOD: Mood = Mood { army: 0, danger: 0.0, boss: false, peril: 1.0 };

//how loud each layer should be, 0 to 1, for a mood
pub(crate) fn layer_targets(mood: &Mood) -> [f32; MUSIC_LAYERS] {
    //the army counts for less and less as it grows, so a hundred units is already something and six thousand is not much more
    let size = (mood.army as f32 / 1500.0).sqrt().min(1.0);
    let ramp = |value: f32, from: f32, to: f32| ((value - from) / (to - from)).clamp(0.0, 1.0);
    let danger = (if mood.boss { mood.danger.max(0.8) } else { mood.danger }).clamp(0.0, 1.0);
    let peril = mood.peril.clamp(0.0, 1.0);
    let failing = peril > 0.0;
    [
        //the pad is the floor, a little lower when something is about to happen
        0.6 - 0.2 * danger,
        ramp(size, 0.12, 0.45) * if failing { 0.5 } else { 0.85 },
        ramp(size, 0.5, 0.9) * 0.7 * (1.0 - peril),
        danger.powf(0.8) * 0.9,
        if mood.boss { 0.8 } else { peril * 0.7 },
    ]
}

// ---------- making the loops ----------

//a note with a soft start and end, mixed into a buffer
fn note(buffer: &mut [f32], start: f32, length: f32, hz: f32, wave: Wave, level: f32, attack: f32, release: f32) {
    let rate = SAMPLE_RATE as f32;
    //a note that would run past the end of the loop is cut short and fades out there, so the loop joins up cleanly
    let length = length.min(buffer.len() as f32 / rate - start);
    if length <= 0.0 {
        return;
    }
    let first = (start * rate) as usize;
    let count = ((length * rate) as usize).min(buffer.len().saturating_sub(first));
    let mut phase = 0.0f32;
    for i in 0..count {
        let t = i as f32 / rate;
        phase = (phase + hz / rate).fract();
        let raw = match wave {
            Wave::Sine => (phase * std::f32::consts::TAU).sin(),
            Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
            Wave::Saw => phase * 2.0 - 1.0,
            Wave::Square => {
                if phase < 0.5 {
                    0.7
                } else {
                    -0.7
                }
            }
            Wave::Noise => 0.0,
        };
        let envelope = (t / attack).min(1.0) * ((length - t) / release).clamp(0.0, 1.0);
        buffer[first + i] += raw * level * envelope;
    }
}

//the chords, two bars each: A minor, F, C, G. as the notes of the chord, and the root a couple of octaves down
const CHORDS: [([f32; 3], f32); 4] = [
    ([220.0, 261.63, 329.63], 55.0),
    ([174.61, 220.0, 261.63], 43.65),
    ([261.63, 329.63, 392.0], 65.41),
    ([196.0, 246.94, 293.66], 49.0),
];

fn chord_seconds() -> f32 {
    LOOP_SECONDS / CHORDS.len() as f32
}

//the loop for one layer, as samples scaled to a peak of 0.8
pub(crate) fn synthesize_layer(layer: usize) -> Vec<f32> {
    use Wave::*;
    let mut buffer = vec![0.0f32; (LOOP_SECONDS * SAMPLE_RATE as f32) as usize];
    let bar = BEAT * 4.0;
    match layer {
        //slow chords that swell and fade into the next
        0 => {
            for (c, (notes, _)) in CHORDS.iter().enumerate() {
                let start = c as f32 * chord_seconds();
                for &hz in notes {
                    note(&mut buffer, start, chord_seconds(), hz, Triangle, 0.5, 1.4, 1.6);
                    note(&mut buffer, start, chord_seconds(), hz * 2.003, Sine, 0.22, 2.0, 1.8);
                    note(&mut buffer, start, chord_seconds(), hz * 0.4995, Sine, 0.3, 1.0, 1.5);
                }
            }
        }
        //the root of the chord, bouncing: on the one, and the and of two, the three and the and of four
        1 => {
            for (c, (_, root)) in CHORDS.iter().enumerate() {
                for b in 0..2 {
                    let bar_start = (c * 2 + b) as f32 * bar;
                    for beat in [0.0, 1.5, 2.0, 3.5] {
                        let octave = if beat == 1.5 { 2.0 } else { 1.0 };
                        note(&mut buffer, bar_start + beat * BEAT, BEAT * 0.8, root * octave, Triangle, 0.9, 0.012, 0.2);
                    }
                }
            }
        }
        //sixteenth notes running up and down the chord, high up
        2 => {
            for (c, (notes, _)) in CHORDS.iter().enumerate() {
                let shape = [notes[0], notes[1], notes[2], notes[0] * 2.0, notes[2], notes[1]];
                let sixteenth = BEAT / 4.0;
                for k in 0..(chord_seconds() / sixteenth) as usize {
                    let hz = shape[k % shape.len()] * 2.0;
                    let accent = if k % 4 == 0 { 1.0 } else { 0.7 };
                    note(&mut buffer, c as f32 * chord_seconds() + k as f32 * sixteenth, sixteenth * 1.4, hz, Triangle, 0.5 * accent, 0.005, 0.1);
                }
            }
        }
        //kick on every beat, snare on two and four, hats on the eighths
        3 => {
            let mut tone = |start: f32, length: f32, from: f32, to: f32, wave: Wave, level: f32, decay: f32| add_tone(&mut buffer, start, length, from, to, wave, level, decay);
            for beat in 0..(BARS * 4) {
                let t = beat as f32 * BEAT;
                tone(t, 0.22, 150.0, 42.0, Sine, 1.0, 13.0);
                if beat % 2 == 1 {
                    tone(t, 0.18, 7000.0, 1800.0, Noise, 0.55, 17.0);
                    tone(t, 0.12, 200.0, 140.0, Triangle, 0.45, 25.0);
                }
                tone(t, 0.05, 11000.0, 9000.0, Noise, 0.16, 70.0);
                tone(t + BEAT / 2.0, 0.05, 11000.0, 9000.0, Noise, 0.22, 70.0);
            }
        }
        //two saws a semitone apart beating against each other, a swell every two bars, and a high shimmer
        _ => {
            for swell in 0..(BARS / 2) {
                let start = swell as f32 * bar * 2.0;
                note(&mut buffer, start, bar * 2.0, 55.0, Saw, 0.35, bar * 0.8, bar * 0.8);
                note(&mut buffer, start, bar * 2.0, 58.27, Saw, 0.3, bar * 0.8, bar * 0.8);
                note(&mut buffer, start, bar * 2.0, 622.25, Sine, 0.12, bar, bar * 0.8);
            }
        }
    }
    let peak = buffer.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    if peak > 0.0 {
        for sample in &mut buffer {
            *sample *= 0.8 / peak;
        }
    }
    buffer
}

// ---------- playing them ----------

pub(crate) struct Music {
    layers: Vec<Option<audio::Sound>>,
    pub(crate) volumes: [f32; MUSIC_LAYERS], //what each is playing at now, 0 to 1
    pub(crate) on: bool,                     //the player can turn the music off and leave the sounds
    started: bool,
}

impl Music {
    pub(crate) async fn load() -> Music {
        let mut layers = Vec::new();
        for layer in 0..MUSIC_LAYERS {
            layers.push(audio::load_sound_from_bytes(&wav_bytes(&synthesize_layer(layer))).await.ok());
        }
        Music { layers, volumes: [0.0; MUSIC_LAYERS], on: true, started: false }
    }

    //move each layer's volume towards what the mood asks for, no faster than MUSIC_FADE a second. `master` is 0 when everything is
    //muted
    pub(crate) fn update(&mut self, mood: &Mood, dt: f32, master: f32) {
        let targets = if self.on { layer_targets(mood) } else { [0.0; MUSIC_LAYERS] };
        for (volume, target) in self.volumes.iter_mut().zip(targets) {
            *volume = fade_towards(*volume, target, MUSIC_FADE * dt);
        }
        for (layer, sound) in self.layers.iter().enumerate() {
            let Some(sound) = sound else {
                continue;
            };
            let volume = self.volumes[layer] * MUSIC_GAIN * MASTER_VOLUME * master;
            //all five start at the same moment and run silently from then on, so they stay in step
            if !self.started {
                audio::play_sound(sound, audio::PlaySoundParams { looped: true, volume });
            } else {
                audio::set_sound_volume(sound, volume);
            }
        }
        self.started = true;
    }
}

//a value moved towards a target by at most `step`
pub(crate) fn fade_towards(value: f32, target: f32, step: f32) -> f32 {
    if (target - value).abs() <= step {
        target
    } else {
        value + step * (target - value).signum()
    }
}
