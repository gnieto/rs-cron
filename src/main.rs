#![feature(collections, test)]
extern crate uuid;
extern crate time;
extern crate test;

mod cron;

use cron::cron::{CronJob, Cron};
use uuid::Uuid;

pub struct EchoJob {
    pub ts: u32,
    pub id: Uuid,
}

impl EchoJob {
    pub fn new(ts: u32) -> EchoJob {
        EchoJob {
            ts: ts,
            id: Uuid::new_v4(),
        }
    }
}

impl CronJob for EchoJob {
    fn execute(&mut self) {
        println!("Echo")
    }

    fn get_id(&self) -> Uuid {
        self.id
    }

    fn get_time(&self) -> u32 {
        self.ts
    }
}


fn main() {
    let mut c = Cron::new();

    for i in 0..100 {
        c.schedule(Box::new(EchoJob::new(i as u32)));
    }

    c.check(500);
}
