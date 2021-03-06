use uuid::Uuid;
use std::collections::BTreeMap;
use std::collections::Bound::{Included, Unbounded};
use time::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::*;
use std::thread;
use threadpool::ThreadPool;
use std::collections::HashMap;

pub struct CronJob {
    pub id: Uuid,
    pub timestamp: Timespec,
    pub executor: Box<CronJobExecutor>
}

impl CronJob{
    pub fn new(ts: Timespec, executor: Box<CronJobExecutor>) -> CronJob {
        CronJob {
            id: Uuid::new_v4(),
            timestamp: ts,
            executor: executor
        }
    }
}

pub struct CronJobResult {
    pub id: Uuid,
    pub timestamp: Timespec,
    pub execution: Timespec,
}

impl CronJobResult {
    pub fn new(id: Uuid, timestamp: Timespec, execution: Timespec) -> CronJobResult {
        CronJobResult {
            id: id,
            timestamp: timestamp,
            execution: execution,
        }
    }
}

pub trait CronJobExecutor: Send + Sync {
    fn execute(&self, cron_job: &CronJob);
}

pub struct DummyCronJobExecutor;

impl CronJobExecutor for DummyCronJobExecutor {
    fn execute(&self, cron_job: &CronJob) {

    }
}

pub struct EchoCronJobExecutor;

impl CronJobExecutor for EchoCronJobExecutor {
    fn execute(&self, cron_job: &CronJob) {
        println!("[{:?}]Executing job {}", now().to_timespec(), cron_job.id);
    }
}

pub struct CronWrapper {
    pub cron_ref: Arc<Mutex<Cron>>,
    pub tx_input: Sender<CronJob>,
    pub rx_output: Receiver<CronJobResult>,
}

impl CronWrapper {
    pub fn new() -> CronWrapper {
        let (tx_input, rx_input) = channel();
        let (tx_output, rx_output) = channel();

        let c = Cron::new_with_out_channel(tx_output);
        let rc = Mutex::new(c);
        let arc = Arc::new(rc);
        let run_cron = arc.clone();

        thread::spawn(move || {
            loop {
                let mut _cron = run_cron.lock().unwrap();
                _cron.run();
            }
        });

        let recv_cron = arc.clone();

        thread::spawn(move || {
            for job in rx_input.iter() {
                let mut _cron = recv_cron.lock().unwrap();
                _cron.schedule(job);
            }
        });

        let cw = CronWrapper {cron_ref: arc.clone(), tx_input: tx_input, rx_output: rx_output};
        cw
    }

    pub fn schedule(&self, job: CronJob) -> Result<(), &'static str> {
       let result = self.tx_input.send(job); 
       match result {
            Ok(_) => Result::Ok(()),
            Err(_) => Result::Err("Can not schedule the job"),
       }
    }

    pub fn count(&self) -> u32 {
        let r = self.cron_ref.clone();
        let _cron = r.lock().unwrap();
        _cron.count()
    }

    pub fn has(&self, id: Uuid) -> bool {
        let r = self.cron_ref.clone();
        let _cron = r.lock().unwrap();
        _cron.has(id)
    }
}

pub struct Cron {
    pub jobs: BTreeMap<Timespec, Vec<CronJob>>,
    pub jobs_hash: HashMap<Uuid, bool>, // Maybe bitmap?
    num_jobs: u32,
    thread_pool: ThreadPool,
    done_jobs_tx: Option<Sender<CronJobResult>>,
}

impl Cron {
    pub fn new() -> Cron {
        let c = Cron {
            jobs: BTreeMap::new(),
            jobs_hash: HashMap::new(),
            num_jobs: 0,
            thread_pool: ThreadPool::new(1),
            done_jobs_tx: None,
        };
        c
    }

    pub fn new_with_out_channel(tx_channel: Sender<CronJobResult>) -> Cron {
        let mut c = Cron::new();
        c.done_jobs_tx = Some(tx_channel);
        c
    }

    pub fn run(&mut self) {
        let current_time = now().to_timespec();
        self.check(current_time);
    }

    pub fn schedule(&mut self, job: CronJob) {
        let ts = job.timestamp;
        let id = job.id;

        if !self.jobs.contains_key(&ts) {
            let v: Vec<CronJob> = Vec::new();
            self.jobs.insert(ts, v);
        }
        
        let mut v = self.jobs.get_mut(&ts).unwrap();
        v.push(job);
        self.jobs_hash.insert(id, true);
        self.num_jobs = self.num_jobs + 1;
    }

    pub fn check(&mut self, current_time: Timespec) {
        let mut keys_to_remove: Vec<Timespec> = Vec::new();
        for (&key, value) in self.jobs.range(Unbounded, Included(&current_time)) {
            keys_to_remove.push(key);
        }

        for k in keys_to_remove {
            let mut jobs_to_process = self.jobs.remove(&k).unwrap();
            self.num_jobs = self.num_jobs - jobs_to_process.len() as u32;
            
            loop {
                let element = jobs_to_process.pop();
                match element {
                    None => break,
                    Some(job) => {
                        self.jobs_hash.remove(&job.id);

                        if self.done_jobs_tx.is_some() {
                            let cj_result = CronJobResult::new(job.id, job.timestamp, current_time);
                            self.done_jobs_tx.as_mut().unwrap().send(cj_result);
                        }

                        self.thread_pool.execute(move || {
                            job.executor.execute(&job);
                        });
                    },
                }
            }
        }
    }

    pub fn has(&self, id: Uuid) -> bool {
        return self.jobs_hash.get(&id).is_some();
    }

    pub fn count(&self) -> u32 {
        self.num_jobs
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use test::Bencher;
    use time::*;
    use uuid::Uuid;
    use std::sync::mpsc::*;
    use std::thread;

    #[test]
    fn it_can_contain_multiple_cron_jobs() {
        let mut c = Cron::new();

        for i in 0..100 {
            let t = Timespec::new(100 * i, 0);
            let cj = CronJob::new(t, Box::new(DummyCronJobExecutor));
            c.schedule(cj);
        }

        assert_eq!(c.count(), 100)
    }
 
    #[test]
    fn it_can_check_which_jobs_are_outdated() {
        let mut c = Cron::new();

        for i in 0..100 {
            let t = Timespec::new(100 * i, 0);
            let cj = CronJob::new(t, Box::new(DummyCronJobExecutor));
            c.schedule(cj);
        }

        c.check(Timespec::new(400, 0));
        assert_eq!(c.count(), 100 - 5);
    }

    #[test]
    fn it_call_callbalcks_on_outdated_jobs() {
        // Pending to implement
    }

    #[test]
    fn it_can_check_if_a_job_is_pending_to_be_processed() {
        let mut c = Cron::new();

        let t = Timespec::new(100, 0);
        let cj = CronJob::new(t, Box::new(DummyCronJobExecutor));
        let uuid = cj.id;
        c.schedule(cj);
        assert_eq!(true, c.has(uuid));
        assert_eq!(false, c.has(Uuid::new_v4()));
        c.check(Timespec::new(100, 0));
        assert_eq!(false, c.has(uuid));
    }

    #[test]
    fn it_can_create_a_cron_with_output_channel() {
        let (tx, rx) = channel();
        let mut c = Cron::new_with_out_channel(tx);
    }

    #[test]
    fn cron_wrapper_can_hook_to_done_jobs_channel() {
        let mut cw = CronWrapper::new();
        let job = CronJob::new(Timespec::new(400, 0), Box::new(DummyCronJobExecutor));
        cw.schedule(job);

        let join_handler = thread::spawn(move || {
            for done_job in cw.rx_output.iter() {
                break;
            }
        });
        
        // TODO: Set some timeout to make fail the test
        join_handler.join();
    }

    #[bench]
    fn insert_performance(b: &mut Bencher) {
        let mut c = Cron::new();

        b.iter(|| {
            let timestamp = now().to_timespec();
            let cj = CronJob::new(timestamp, Box::new(DummyCronJobExecutor));
            c.schedule(cj);
        });
    }
}
