//! the random numbers, one stream for each thread. macroquad's are one stream for the whole program, so tests run side by side drew from each other's,
//! and a fight seeded the same way came out differently in a full run than alone. the prelude puts this in place of macroquad's `rand`, so the rest of the
//! game asks for `rand::gen_range` as before. the game draws them on its main thread, which it seeds at the start; a thread that is never seeded gets a
//! stream of its own all the same, so no two are alike

use macroquad::rand::{RandGenerator, RandomRange};
use std::sync::atomic::{AtomicU64, Ordering};

static STREAMS: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static DICE: RandGenerator = {
        let dice = RandGenerator::new();
        dice.srand(STREAMS.fetch_add(1, Ordering::Relaxed).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        dice
    };
}

//the stream of this thread started again from `seed`
pub(crate) fn srand(seed: u64) {
    DICE.with(|dice| dice.srand(seed));
}

//a number from 0 to u32::MAX
pub(crate) fn rand() -> u32 {
    DICE.with(|dice| dice.rand())
}

//a number from `low` up to (not including, for whole numbers) `high`
pub(crate) fn gen_range<T: RandomRange>(low: T, high: T) -> T {
    DICE.with(|dice| dice.gen_range(low, high))
}
