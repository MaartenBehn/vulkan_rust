use log::info;
use std::time::{Duration, Instant};
pub const ENABLE_SHIP_PROFILING: bool = true;

pub struct TickProfile {
    last_block_placement: Instant,
    time_spent_computing: Duration,
    tick_counter: usize,

    start_ship_computing: Instant,
}

impl TickProfile {
    pub fn new() -> Self {
        TickProfile {
            last_block_placement: Instant::now(),
            time_spent_computing: Duration::ZERO,
            tick_counter: 0,
            start_ship_computing: Instant::now(),
        }
    }

    pub fn reset(&mut self) {
        self.last_block_placement = Instant::now();
        self.time_spent_computing = Duration::ZERO;
        self.tick_counter = 0;
    }

    pub fn ship_computing_start(&mut self, ticks: usize) {
        self.tick_counter += ticks;
        self.start_ship_computing = Instant::now();
    }

    pub fn ship_computing_done(&mut self) {
        self.time_spent_computing += self.start_ship_computing.elapsed();
    }

    pub fn print_state(&self) {
        info!(
            "Builder computet {:.2} sec. It Took {:.2} sec with {} ticks",
            self.time_spent_computing.as_secs_f32(),
            self.last_block_placement.elapsed().as_secs_f32(),
            self.tick_counter
        );
    }
}
