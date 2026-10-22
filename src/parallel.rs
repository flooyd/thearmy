//! sharing work out over the processor's cores, for the jobs that are the same for every unit and don't depend on each other

use crate::prelude::*;
use rayon::prelude::*;
use std::sync::OnceLock;

//how many threads work on a job together: the processor's cores, up to MAX_WORKERS. the threads are started the first time
//they are needed and kept, starting one for every job would cost more than the job
pub(crate) fn worker_count() -> usize {
    static COUNT: OnceLock<usize> = OnceLock::new();
    *COUNT.get_or_init(|| {
        let count = std::thread::available_parallelism().map_or(1, |cores| cores.get()).min(MAX_WORKERS);
        //this only fails if the threads were already started, with their own count
        let _ = rayon::ThreadPoolBuilder::new().num_threads(count).build_global();
        count
    })
}

//call `work(first, chunk)` on the items cut into one chunk for each worker, `first` being where the chunk starts in the
//whole list, and wait for them all. the chunks don't overlap so none of them can see what another one does, which is what
//makes the result the same as one worker doing the whole list. a short list isn't worth the hand-over, it is done in one go
//here
pub(crate) fn share_out<T: Send>(items: &mut [T], work: impl Fn(usize, &mut [T]) + Sync) {
    if items.len() < PARALLEL_FROM || worker_count() <= 1 {
        work(0, items);
        return;
    }
    let size = items.len().div_ceil(worker_count());
    items.par_chunks_mut(size).enumerate().for_each(|(n, chunk)| work(n * size, chunk));
}
