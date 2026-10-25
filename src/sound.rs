//! sounds, made as samples in code: the synthesis, the table of cues and playback

use crate::prelude::*;

pub(crate) const SAMPLE_RATE: u32 = 22050;
pub(crate) const MASTER_VOLUME: f32 = 0.4; //every sound is quiet, a fight can make a lot of them
pub(crate) const SOUND_PEAK: f32 = 0.9; //how loud each sound is made before the volumes of the cues are applied

//the pops a streak of babies growing climbs through, and how many semitones above the first each is: a pentatonic scale
const POPS: [Cue; 6] = [Cue::Pop, Cue::Pop2, Cue::Pop3, Cue::Pop4, Cue::Pop5, Cue::Pop6];
const POP_SEMITONES: [f32; 6] = [0.0, 2.0, 4.0, 7.0, 9.0, 12.0];
//how many pops in a streak before it climbs a step
const POPS_PER_STEP: u32 = 2;

//the shapes of sound a cue is made of
#[derive(Clone, Copy)]
pub(crate) enum Wave {
    Sine,
    Triangle,
    Saw,    //buzzy, brighter than a triangle
    Square, //hollow and harsh
    Noise,  //hiss, `from` and `to` are how high it is, as a pitch
}

//mix one sound into a buffer, starting `start` seconds in: a wave gliding from `from` Hz to `to` Hz over `length` seconds, at
//`level`, dying away by `decay` per second. it fades in and out over a few milliseconds so it doesn't click. the same
//arguments always give the same samples
pub(crate) fn add_tone(buffer: &mut Vec<f32>, start: f32, length: f32, from: f32, to: f32, wave: Wave, level: f32, decay: f32) {
    let rate = SAMPLE_RATE as f32;
    let first = (start * rate) as usize;
    let count = (length * rate) as usize;
    if buffer.len() < first + count {
        buffer.resize(first + count, 0.0);
    }
    let fade = 0.004 * rate;
    let mut phase = 0.0f32;
    let mut noise = 0x2545_F491u32 ^ (first as u32).wrapping_mul(2_654_435_761);
    let mut hiss = 0.0f32;
    for i in 0..count {
        let progress = i as f32 / count as f32;
        let hz = from * (to / from).powf(progress);
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
            Wave::Noise => {
                noise ^= noise << 13;
                noise ^= noise >> 17;
                noise ^= noise << 5;
                let white = noise as f32 / u32::MAX as f32 * 2.0 - 1.0;
                //a low pass filter, which the glide opens or closes
                hiss += (white - hiss) * (hz * std::f32::consts::TAU / rate).min(1.0);
                hiss * 2.0
            }
        };
        let envelope = (i as f32 / fade).min(1.0) * ((count - i) as f32 / fade).min(1.0) * (-decay * i as f32 / rate).exp();
        buffer[first + i] += raw * level * envelope;
    }
}

//what a cue sounds like, as samples between -1 and 1 with the loudest at SOUND_PEAK
pub(crate) fn synthesize(cue: Cue) -> Vec<f32> {
    use Wave::*;
    let mut buffer = Vec::new();
    match cue {
        //a short bright blip that rises
        Cue::Pop => add_tone(&mut buffer, 0.0, 0.09, 520.0, 1040.0, Sine, 1.0, 22.0),
        //a drop in pitch with a crash of hiss on top
        Cue::Thud => {
            add_tone(&mut buffer, 0.0, 0.4, 150.0, 40.0, Sine, 1.0, 7.0);
            add_tone(&mut buffer, 0.0, 0.25, 2500.0, 200.0, Noise, 0.6, 14.0);
        }
        //two quick notes going up, like a command being given
        Cue::Dispatch => {
            add_tone(&mut buffer, 0.0, 0.07, 523.0, 523.0, Triangle, 0.8, 10.0);
            add_tone(&mut buffer, 0.075, 0.1, 784.0, 784.0, Triangle, 0.8, 12.0);
        }
        //two sharp notes falling, a warning
        Cue::Alert => {
            add_tone(&mut buffer, 0.0, 0.1, 880.0, 700.0, Triangle, 0.9, 8.0);
            add_tone(&mut buffer, 0.13, 0.1, 880.0, 700.0, Triangle, 0.9, 8.0);
        }
        //a zap
        Cue::Shoot => {
            add_tone(&mut buffer, 0.0, 0.12, 4000.0, 600.0, Noise, 0.7, 25.0);
            add_tone(&mut buffer, 0.0, 0.12, 900.0, 180.0, Sine, 0.8, 22.0);
        }
        //a small puff
        Cue::Death => {
            add_tone(&mut buffer, 0.0, 0.1, 1800.0, 300.0, Noise, 0.6, 30.0);
            add_tone(&mut buffer, 0.0, 0.1, 330.0, 110.0, Sine, 0.7, 28.0);
        }
        //four notes climbing a chord, each a bell: the note and its octave
        Cue::Milestone => {
            for (i, hz) in [523.25, 659.25, 783.99, 1046.5].into_iter().enumerate() {
                let length = if i == 3 { 0.7 } else { 0.35 };
                add_tone(&mut buffer, 0.1 * i as f32, length, hz, hz, Triangle, 0.7, 5.0);
                add_tone(&mut buffer, 0.1 * i as f32, length, hz * 2.0, hz * 2.0, Sine, 0.25, 7.0);
            }
        }
        //a slow fall and a rumble
        Cue::Defeat => {
            add_tone(&mut buffer, 0.0, 1.2, 392.0, 196.0, Sine, 0.9, 1.2);
            add_tone(&mut buffer, 0.0, 1.4, 98.0, 49.0, Triangle, 0.8, 1.5);
            add_tone(&mut buffer, 0.0, 0.3, 400.0, 100.0, Noise, 0.3, 3.0);
        }
        //a siren: two harsh notes, the second higher
        Cue::Warning => {
            add_tone(&mut buffer, 0.0, 0.18, 440.0, 440.0, Square, 0.55, 4.0);
            add_tone(&mut buffer, 0.2, 0.22, 587.0, 587.0, Square, 0.55, 4.0);
        }
        //a deep boom that shakes: a low note falling away and a crash of hiss
        Cue::Slam => {
            add_tone(&mut buffer, 0.0, 0.8, 95.0, 28.0, Sine, 1.0, 4.5);
            add_tone(&mut buffer, 0.0, 0.5, 1800.0, 90.0, Noise, 0.8, 7.0);
            add_tone(&mut buffer, 0.0, 0.3, 180.0, 60.0, Saw, 0.4, 12.0);
        }
        //a blast: hiss that closes up and a thump under it
        Cue::Explosion => {
            add_tone(&mut buffer, 0.0, 0.55, 3200.0, 140.0, Noise, 1.0, 6.0);
            add_tone(&mut buffer, 0.0, 0.35, 130.0, 42.0, Sine, 0.9, 8.0);
        }
        //a long sweep climbing, with a bell on top
        Cue::PhaseShift => {
            add_tone(&mut buffer, 0.0, 1.0, 70.0, 520.0, Saw, 0.55, 1.5);
            add_tone(&mut buffer, 0.0, 1.0, 140.0, 1400.0, Sine, 0.6, 1.8);
            add_tone(&mut buffer, 0.8, 0.5, 880.0, 880.0, Triangle, 0.6, 5.0);
        }
        //bright notes climbing, then held, like treasure
        Cue::Relic => {
            for (i, hz) in [523.25, 659.25, 783.99, 1046.5, 1318.5].into_iter().enumerate() {
                add_tone(&mut buffer, 0.07 * i as f32, 0.5, hz, hz, Triangle, 0.6, 5.0);
                add_tone(&mut buffer, 0.07 * i as f32, 0.5, hz * 2.0, hz * 2.0, Sine, 0.3, 6.0);
            }
        }
        //a whoosh rising
        Cue::Charge => {
            add_tone(&mut buffer, 0.0, 0.45, 300.0, 3000.0, Noise, 0.7, 2.0);
            add_tone(&mut buffer, 0.0, 0.45, 110.0, 330.0, Saw, 0.5, 2.5);
        }
        //a soft chime rising
        Cue::Heal => {
            add_tone(&mut buffer, 0.0, 0.25, 660.0, 990.0, Sine, 0.8, 6.0);
            add_tone(&mut buffer, 0.1, 0.3, 990.0, 1320.0, Sine, 0.6, 6.0);
        }
        //a low swirl that rises and then drops away
        Cue::Summon => {
            add_tone(&mut buffer, 0.0, 0.3, 150.0, 480.0, Saw, 0.5, 4.0);
            add_tone(&mut buffer, 0.3, 0.3, 480.0, 120.0, Triangle, 0.6, 4.0);
            add_tone(&mut buffer, 0.0, 0.6, 600.0, 200.0, Noise, 0.2, 4.0);
        }
        //stone cracking and falling: a burst of hiss and a dull thump
        Cue::WallBreak => {
            add_tone(&mut buffer, 0.0, 0.3, 2200.0, 250.0, Noise, 0.9, 11.0);
            add_tone(&mut buffer, 0.0, 0.25, 160.0, 55.0, Sine, 0.8, 12.0);
            add_tone(&mut buffer, 0.05, 0.15, 700.0, 300.0, Square, 0.2, 20.0);
        }
        //a bowstring: a short twang that falls
        Cue::Arrow => {
            add_tone(&mut buffer, 0.0, 0.11, 1500.0, 500.0, Triangle, 0.7, 24.0);
            add_tone(&mut buffer, 0.0, 0.08, 5000.0, 1200.0, Noise, 0.3, 40.0);
        }
        //a gravity well: a deep swell that climbs into a whine
        Cue::Pull => {
            add_tone(&mut buffer, 0.0, 1.2, 55.0, 140.0, Sine, 0.9, 0.6);
            add_tone(&mut buffer, 0.0, 1.2, 90.0, 520.0, Saw, 0.25, 1.0);
            add_tone(&mut buffer, 0.2, 1.0, 300.0, 1400.0, Noise, 0.15, 1.5);
        }
        //something tearing in two: a wet crack and a falling tone
        Cue::Split => {
            add_tone(&mut buffer, 0.0, 0.25, 1800.0, 200.0, Noise, 0.8, 9.0);
            add_tone(&mut buffer, 0.0, 0.4, 420.0, 110.0, Saw, 0.6, 6.0);
            add_tone(&mut buffer, 0.12, 0.3, 300.0, 90.0, Square, 0.25, 8.0);
        }
        //a fanfare: three notes climbing and a chord held
        Cue::Victory => {
            add_tone(&mut buffer, 0.0, 0.25, 523.0, 523.0, Triangle, 0.7, 3.0);
            add_tone(&mut buffer, 0.2, 0.25, 659.0, 659.0, Triangle, 0.7, 3.0);
            add_tone(&mut buffer, 0.4, 0.25, 784.0, 784.0, Triangle, 0.7, 3.0);
            add_tone(&mut buffer, 0.6, 1.2, 1047.0, 1047.0, Triangle, 0.7, 1.6);
            add_tone(&mut buffer, 0.6, 1.2, 659.0, 659.0, Sine, 0.5, 1.6);
            add_tone(&mut buffer, 0.6, 1.2, 784.0, 784.0, Sine, 0.5, 1.6);
        }
        //a village bell: two bright notes
        Cue::Liberate => {
            add_tone(&mut buffer, 0.0, 0.5, 880.0, 880.0, Sine, 0.7, 5.0);
            add_tone(&mut buffer, 0.12, 0.6, 1320.0, 1320.0, Sine, 0.6, 5.0);
            add_tone(&mut buffer, 0.12, 0.3, 2640.0, 2640.0, Sine, 0.15, 9.0);
        }
        //two low thumps close together, lub-dub
        Cue::Heartbeat => {
            add_tone(&mut buffer, 0.0, 0.14, 72.0, 44.0, Sine, 1.0, 16.0);
            add_tone(&mut buffer, 0.2, 0.16, 62.0, 38.0, Sine, 0.75, 14.0);
        }
        //a sad little fall, two notes going down and a dull thump
        Cue::SquadLost => {
            add_tone(&mut buffer, 0.0, 0.28, 392.0, 349.0, Triangle, 0.8, 6.0);
            add_tone(&mut buffer, 0.22, 0.5, 294.0, 196.0, Triangle, 0.8, 5.0);
            add_tone(&mut buffer, 0.0, 0.2, 200.0, 70.0, Sine, 0.6, 12.0);
        }
        //a quick bright zing climbing
        Cue::Combo => {
            add_tone(&mut buffer, 0.0, 0.12, 880.0, 1320.0, Triangle, 0.8, 9.0);
            add_tone(&mut buffer, 0.09, 0.28, 1320.0, 1760.0, Triangle, 0.8, 6.0);
            add_tone(&mut buffer, 0.09, 0.28, 2640.0, 3520.0, Sine, 0.2, 8.0);
        }
        //a bright fanfare: a rising chord with a ring on top, for a champion joining
        Cue::Champion => {
            for (i, hz) in [392.0, 523.25, 659.25, 783.99, 1046.5].into_iter().enumerate() {
                add_tone(&mut buffer, 0.06 * i as f32, 0.55, hz, hz, Triangle, 0.6, 4.0);
                add_tone(&mut buffer, 0.06 * i as f32, 0.55, hz * 2.0, hz * 2.0, Sine, 0.25, 6.0);
            }
            add_tone(&mut buffer, 0.0, 0.3, 90.0, 60.0, Sine, 0.6, 9.0);
        }
        //a slow, low fall for a champion lost
        Cue::Fall => {
            add_tone(&mut buffer, 0.0, 0.7, 330.0, 150.0, Triangle, 0.8, 3.0);
            add_tone(&mut buffer, 0.15, 0.8, 247.0, 110.0, Triangle, 0.6, 3.0);
            add_tone(&mut buffer, 0.0, 0.5, 90.0, 40.0, Sine, 0.8, 5.0);
        }
        //a crackle: hiss that dives and a buzzing tone that falls
        Cue::Zap => {
            add_tone(&mut buffer, 0.0, 0.2, 6000.0, 700.0, Noise, 0.8, 14.0);
            add_tone(&mut buffer, 0.0, 0.18, 1800.0, 260.0, Saw, 0.5, 16.0);
            add_tone(&mut buffer, 0.05, 0.12, 2400.0, 500.0, Square, 0.25, 22.0);
        }
        //an icy chime with a breath of hiss
        Cue::Frost => {
            add_tone(&mut buffer, 0.0, 0.5, 1760.0, 1760.0, Sine, 0.6, 6.0);
            add_tone(&mut buffer, 0.05, 0.5, 2349.0, 2349.0, Sine, 0.4, 7.0);
            add_tone(&mut buffer, 0.0, 0.35, 5000.0, 2500.0, Noise, 0.2, 8.0);
        }
        //a small hard tick
        Cue::Hit => {
            add_tone(&mut buffer, 0.0, 0.07, 3200.0, 900.0, Noise, 0.6, 40.0);
            add_tone(&mut buffer, 0.0, 0.04, 700.0, 320.0, Sine, 0.5, 50.0);
        }
        //a bright ping with a crack under it
        Cue::Crit => {
            add_tone(&mut buffer, 0.0, 0.14, 2400.0, 1800.0, Triangle, 0.8, 16.0);
            add_tone(&mut buffer, 0.0, 0.08, 6000.0, 1800.0, Noise, 0.4, 30.0);
            add_tone(&mut buffer, 0.0, 0.1, 300.0, 120.0, Sine, 0.5, 24.0);
        }
        //a soft rattle of fire
        Cue::Volley => {
            add_tone(&mut buffer, 0.0, 0.05, 1900.0, 900.0, Square, 0.25, 42.0);
            add_tone(&mut buffer, 0.0, 0.06, 4500.0, 1500.0, Noise, 0.35, 38.0);
        }
        //glass breaking: a high crash of hiss and two bright, ringing notes falling a little
        Cue::Shatter => {
            add_tone(&mut buffer, 0.0, 0.25, 8000.0, 2500.0, Noise, 0.7, 14.0);
            add_tone(&mut buffer, 0.0, 0.35, 3100.0, 2700.0, Sine, 0.5, 8.0);
            add_tone(&mut buffer, 0.03, 0.3, 4400.0, 3900.0, Sine, 0.35, 9.0);
        }
        //the same blip as a pop, a step higher each time up a scale
        Cue::Pop2 | Cue::Pop3 | Cue::Pop4 | Cue::Pop5 | Cue::Pop6 => {
            let ratio = 2f32.powf(POP_SEMITONES[POPS.iter().position(|&pop| pop == cue).unwrap_or(0)] / 12.0);
            add_tone(&mut buffer, 0.0, 0.09, 520.0 * ratio, 1040.0 * ratio, Sine, 1.0, 22.0);
        }
    }
    let peak = buffer.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    if peak > 0.0 {
        for sample in &mut buffer {
            *sample *= SOUND_PEAK / peak;
        }
    }
    buffer
}

//samples as the bytes of a mono 16 bit wav file, which is what the sound loader reads
pub(crate) fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_length = samples.len() as u32 * 2;
    let mut wav = Vec::with_capacity(44 + samples.len() * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_length).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); //the size of what follows in this part
    wav.extend_from_slice(&1u16.to_le_bytes()); //pcm
    wav.extend_from_slice(&1u16.to_le_bytes()); //channels
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); //bytes a second
    wav.extend_from_slice(&2u16.to_le_bytes()); //bytes in a sample
    wav.extend_from_slice(&16u16.to_le_bytes()); //bits in a sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_length.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes());
    }
    wav
}

pub(crate) const ALL_CUES: [Cue; 38] = [
    Cue::Pop,
    Cue::Thud,
    Cue::Dispatch,
    Cue::Alert,
    Cue::Shoot,
    Cue::Death,
    Cue::Milestone,
    Cue::Defeat,
    Cue::Warning,
    Cue::Slam,
    Cue::Explosion,
    Cue::PhaseShift,
    Cue::Relic,
    Cue::Charge,
    Cue::Heal,
    Cue::Summon,
    Cue::WallBreak,
    Cue::Arrow,
    Cue::Pull,
    Cue::Split,
    Cue::Victory,
    Cue::Liberate,
    Cue::Heartbeat,
    Cue::SquadLost,
    Cue::Combo,
    Cue::Pop2,
    Cue::Pop3,
    Cue::Pop4,
    Cue::Pop5,
    Cue::Pop6,
    Cue::Champion,
    Cue::Fall,
    Cue::Zap,
    Cue::Frost,
    Cue::Hit,
    Cue::Crit,
    Cue::Volley,
    Cue::Shatter,
];

impl Cue {
    //how loud, 0 to 1, next to the others
    pub(crate) fn volume(self) -> f32 {
        match self {
            Cue::Pop => 0.5,
            Cue::Thud => 0.9,
            Cue::Dispatch => 0.5,
            Cue::Alert => 0.6,
            Cue::Shoot => 0.4,
            Cue::Death => 0.45,
            Cue::Milestone => 0.8,
            Cue::Defeat => 0.9,
            Cue::Warning => 0.55,
            Cue::Slam => 1.0,
            Cue::Explosion => 0.85,
            Cue::PhaseShift => 0.9,
            Cue::Relic => 0.8,
            Cue::Charge => 0.6,
            Cue::Heal => 0.35,
            Cue::Summon => 0.5,
            Cue::WallBreak => 0.7,
            Cue::Arrow => 0.3,
            Cue::Pull => 0.8,
            Cue::Split => 0.8,
            Cue::Victory => 0.9,
            Cue::Liberate => 0.8,
            Cue::Heartbeat => 0.75,
            Cue::SquadLost => 0.7,
            Cue::Combo => 0.6,
            Cue::Pop2 | Cue::Pop3 | Cue::Pop4 | Cue::Pop5 | Cue::Pop6 => 0.5,
            Cue::Champion => 0.8,
            Cue::Fall => 0.7,
            Cue::Zap => 0.45,
            Cue::Frost => 0.4,
            Cue::Hit => 0.22,
            Cue::Crit => 0.5,
            Cue::Volley => 0.16,
            Cue::Shatter => 0.6,
        }
    }

    //seconds before it can sound again, so a hundred babies growing in a frame is one pop and not a roar
    pub(crate) fn cooldown(self) -> f32 {
        match self {
            Cue::Pop => 0.04,
            Cue::Thud => 0.07,
            Cue::Dispatch => 0.08,
            Cue::Alert => 0.35,
            Cue::Shoot => 0.07,
            Cue::Death => 0.1,
            Cue::Milestone => 0.5,
            Cue::Defeat => 3.0,
            Cue::Warning => 0.4,
            Cue::Slam => 0.15,
            Cue::Explosion => 0.08,
            Cue::PhaseShift => 1.0,
            Cue::Relic => 0.5,
            Cue::Charge => 0.3,
            Cue::Heal => 0.4,
            Cue::Summon => 0.3,
            Cue::WallBreak => 0.12,
            Cue::Arrow => 0.06,
            Cue::Pull => 1.5,
            Cue::Split => 0.4,
            Cue::Victory => 3.0,
            Cue::Liberate => 1.0,
            Cue::Heartbeat => 0.3,
            Cue::SquadLost => 1.0,
            Cue::Combo => 0.25,
            Cue::Pop2 | Cue::Pop3 | Cue::Pop4 | Cue::Pop5 | Cue::Pop6 => 0.04,
            Cue::Champion => 0.4,
            Cue::Fall => 0.6,
            Cue::Zap => 0.12,
            Cue::Frost => 0.5,
            Cue::Hit => 0.05,
            Cue::Crit => 0.12,
            Cue::Volley => 0.07,
            Cue::Shatter => 0.06,
        }
    }
}

//lets a cue through only if it hasn't sounded too recently
pub(crate) struct CueGate {
    pub(crate) ready_at: [f64; ALL_CUES.len()], //the time, in seconds, each can sound from
}

impl CueGate {
    pub(crate) fn new() -> CueGate {
        CueGate { ready_at: [0.0; ALL_CUES.len()] }
    }

    pub(crate) fn admit(&mut self, cue: Cue, now: f64) -> bool {
        let ready_at = &mut self.ready_at[cue as usize];
        if now < *ready_at {
            return false;
        }
        *ready_at = now + cue.cooldown() as f64;
        true
    }
}

//how loud a sound that happened this far from the player is, as a share of how loud it is beside them: it falls away to
//SOUND_FAR_SHARE at SOUND_RANGE and stays there. there is no panning, so only the volume says how near it was
pub(crate) fn distance_share(distance: f32) -> f32 {
    let far = (distance / SOUND_RANGE).clamp(0.0, 1.0);
    1.0 - (1.0 - SOUND_FAR_SHARE) * far
}

//the game's sounds, made from scratch when it starts
pub(crate) struct Sounds {
    pub(crate) sounds: Vec<Option<audio::Sound>>, //by cue, None for one that couldn't be loaded
    pub(crate) gate: CueGate,
    pub(crate) muted: bool,
    pub(crate) volume: f32, //of everything, 0 to 1
    pub(crate) effects: f32, //of the sounds next to the music, 0 to 1
    pub(crate) music_volume: f32, //of the music, 0 to 1
    pub(crate) music: Music,
    pops: PopStreak,
}

impl Sounds {
    pub(crate) async fn load(volume: f32) -> Sounds {
        let mut sounds = Vec::new();
        for cue in ALL_CUES {
            sounds.push(audio::load_sound_from_bytes(&wav_bytes(&synthesize(cue))).await.ok());
        }
        Sounds { sounds, gate: CueGate::new(), muted: false, volume, effects: 1.0, music_volume: 1.0, music: Music::load().await, pops: PopStreak::default() }
    }

    //play the cues a frame asked for, `now` being the time in seconds and `listener` where the player is. a sound from somewhere
    //is quieter the further away it was, and the nearest of the same kind are the ones that get through the gate
    pub(crate) fn play(&mut self, cues: &[(Cue, Option<Vec2>)], listener: Vec2, now: f64) {
        if self.muted {
            return;
        }
        let mut heard: Vec<(Cue, f32)> = cues
            .iter()
            .map(|&(cue, at)| (cue, at.map_or(1.0, |at| distance_share(at.distance(listener)))))
            .filter(|&(_, share)| share > 0.02)
            .collect();
        heard.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (mut cue, share) in heard {
            if !self.gate.admit(cue, now) {
                continue;
            }
            if cue == Cue::Pop {
                cue = self.pops.next(now);
            }
            if let Some(sound) = &self.sounds[cue as usize] {
                audio::play_sound(sound, audio::PlaySoundParams { looped: false, volume: cue.volume() * share * MASTER_VOLUME * self.volume * self.effects });
            }
        }
    }

    //keep the music going to suit the moment
    pub(crate) fn tune_music(&mut self, mood: &Mood, dt: f32) {
        self.music.update(mood, dt, if self.muted { 0.0 } else { self.volume * self.music_volume });
    }
}

//the babies that have been growing one after another, each close behind the one before, which makes the pop climb a scale
pub(crate) struct PopStreak {
    streak: u32,
    last: f64, //when the last one grew
}

impl Default for PopStreak {
    fn default() -> PopStreak {
        PopStreak { streak: 0, last: f64::MIN }
    }
}

impl PopStreak {
    //which of the pops to play now: the first, and then higher and higher while the babies keep on growing
    pub(crate) fn next(&mut self, now: f64) -> Cue {
        self.streak = if now - self.last <= STREAK_WINDOW as f64 { self.streak + 1 } else { 0 };
        self.last = now;
        POPS[((self.streak / POPS_PER_STEP) as usize).min(POPS.len() - 1)]
    }
}
