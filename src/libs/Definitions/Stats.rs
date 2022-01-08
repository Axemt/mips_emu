use std::time::Instant;
use std::time::Duration;

#[derive(Debug)]
pub struct Stats {
    pub instr_count: usize,
    pub cycl_count: usize,
    pub st_time: Instant,
    pub exec_total_time: Duration,

}

pub fn new() -> Stats {
    Stats {instr_count: 0, cycl_count: 0, st_time: Instant::now(), exec_total_time: Duration::new(0,0)}
}

impl Stats {

    pub fn CPI(&self) -> f32 {
        self.cycl_count as f32 / self.instr_count as f32
    }

    pub fn cycle_incr(&mut self) {
        self.cycl_count += 1;
    }

    pub fn instr_incr(&mut self) {
        self.instr_count += 1
    }

    pub fn mark_finished(&mut self) -> Duration {
        self.exec_total_time = self.st_time.elapsed();
        self.exec_total_time

    }

    pub fn exec_total_time(&self) -> Duration {
        self.exec_total_time
    }

    pub fn avg_time_per_instr(&self) -> f32 {
        self.exec_total_time().as_secs_f32() / self.instr_count as f32
    }

}