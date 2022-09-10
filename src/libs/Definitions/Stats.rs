use std::time::Duration;
use std::time::Instant;

#[derive(Debug)]
pub struct Stats {
    pub instruction_count: isize,
    pub cycle_count: usize,
    pub start_time: Instant,
    pub exec_total_time: Duration,
}

pub fn new(pipeline_stages: usize) -> Stats {
    Stats {
        instruction_count: -(pipeline_stages as isize) + 1,
        cycle_count: 0,
        start_time: Instant::now(),
        exec_total_time: Duration::new(0, 0),
    }
}

impl Stats {
    pub fn CPI(&self) -> f32 {
        self.cycle_count as f32 / self.instruction_count as f32
    }

    pub fn cycle_incr(&mut self) {
        self.cycle_count += 1;
    }

    pub fn instr_incr(&mut self) {
        self.instruction_count += 1
    }

    pub fn mark_finished(&mut self) -> Duration {
        self.exec_total_time = self.start_time.elapsed();
        self.exec_total_time
    }

    pub fn exec_total_time(&self) -> Duration {
        self.exec_total_time
    }

    pub fn avg_time_per_instr(&self) -> f32 {
        self.exec_total_time().as_secs_f32() / self.instruction_count as f32
    }
}
