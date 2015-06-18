use uuid::Uuid;
use std::collections::BTreeMap;
use std::collections::Bound::{Included, Unbounded};
use time::*;
use std::rc::Rc;

use test::Bencher;

pub trait CronJob {
    fn execute(&mut self);
    fn get_time(&self) -> u32;
    fn get_id(&self) -> Uuid;
}

pub struct TestCronJob {
    pub id: Uuid,
    pub tag: Option<String>,
    pub timestamp: u32,
    pub callback: Box<Fn()->()>,
    counter: u32,
}

impl TestCronJob {
    pub fn new(tag: Option<String>, ts: u32, cb: Box<Fn()->()>) -> TestCronJob {
        TestCronJob {
            id: Uuid::new_v4(),
            tag: tag,
            timestamp: ts,
            callback: cb,
            counter: 0,
        }
    }

    pub fn get_counter(&self) -> u32 {
        self.counter
    }
}

impl CronJob for TestCronJob {
    fn execute(&mut self) {
        println!("Test cron job");
    }

    fn get_time(&self) -> u32 {
        self.timestamp
    }

    fn get_id(&self) -> Uuid {
        self.id
    }
}

pub struct Cron<J> where J: CronJob {
    pub jobs: BTreeMap<u32, Vec<Box<J>>>,
    num_jobs: u32,
}

impl<J> Cron<J> where J: CronJob {
    pub fn new() -> Cron<J> {
        Cron {jobs: BTreeMap::new(), num_jobs: 0}
    }

    pub fn schedule(&mut self, job: Box<J>) {
        let ts = job.get_time();

        if !self.jobs.contains_key(&ts) {
            let v: Vec<Box<J>> = Vec::new();
            self.jobs.insert(ts, v);
        }
        
        let mut v = self.jobs.get_mut(&ts).unwrap();
        v.push(job);
        self.num_jobs = self.num_jobs + 1;
    }

    pub fn check(&mut self, current_time: u32) {
        let mut keys_to_remove: Vec<u32> = Vec::new();
        for (&key, value) in self.jobs.range(Unbounded, Included(&current_time)) {
            keys_to_remove.push(key);
            println!("{}: {}", key, value.len())
        }

        for k in keys_to_remove {
            let mut jobs_to_process = self.jobs.remove(&k).unwrap();
            self.num_jobs = self.num_jobs - jobs_to_process.len() as u32;
            for job in jobs_to_process.iter_mut() {
                job.execute()
            }
        }
    }

    pub fn count(&self) -> u32 {
        self.num_jobs
    }
}

fn dummy_callback() {
    println!("Callback!");
}

#[test]
fn it_can_contain_multiple_cron_jobs() {
    let mut c = Cron::new();

    for i in 0..100 {
        let cj = TestCronJob::new(None, 100 * i, Box::new(dummy_callback));
        c.schedule(Box::new(cj));
    }

    assert_eq!(c.count(), 100)
}

#[test]
fn it_can_check_which_jobs_are_outdated() {
    let mut c = Cron::new();

    for i in 0..100 {
        let cb = Box::new(dummy_callback);
        let cj = TestCronJob::new(None, 100 * i, cb);
        c.schedule(Box::new(cj));
    }

    c.check(400);
    assert_eq!(c.count(), 100 - 5);
}

#[test]
fn it_call_callbalcks_on_outdated_jobs() {
    let mut c = Cron::new();
   
    let cj1 = Box::new(TestCronJob::new(None, 10, Box::new(dummy_callback)));
    let cj2 = Box::new(TestCronJob::new(None, 20, Box::new(dummy_callback)));

    c.schedule(cj1);
    c.schedule(cj2);

    c.check(15);
    assert_eq!(c.count(), 1);
}

#[bench]
fn insert_performance(b: &mut Bencher) {
    let mut c = Cron::new();

    b.iter(|| {
        let timestamp = now().to_timespec().sec as u32;
        let cb = Box::new(dummy_callback);
        let cj = TestCronJob::new(None, timestamp, cb);

        c.schedule(Box::new(cj));  
    });
}
