#![feature(collections, test)]
extern crate uuid;
extern crate time;
extern crate test;

mod cron;

use cron::cron::*;
use std::thread;
use time::*;

fn main() {
    let c = CronWrapper::new();

    for i in 0..5 {
        let mut current_time = now();
        current_time = current_time + Duration::seconds(i + 1);
        let t = current_time.to_timespec().sec as u32;

        let job = CronJob::new(t, Box::new(EchoCronJobExecutor));
        let id = job.id;
        c.schedule(job).unwrap();
        println!("Scheduled: {}", id);
    }

    println!("Num jobs: {}", c.count());
    thread::sleep_ms(6000);
    println!("Num jobs: {}", c.count());
}
