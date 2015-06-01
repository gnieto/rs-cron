use uuid::Uuid;

pub struct CronJob {
    pub id: Uuid,
    pub tag: Option<String>,
    pub timestamp: u32,
}

impl CronJob {
    pub fn new(tag: Option<String>, ts: u32) -> CronJob {
        CronJob {
            id: Uuid::new_v4(),
            tag: tag,
            timestamp: ts,
        }
    }
}

pub struct Cron {
    pub jobs: Vec<CronJob>
}

impl Cron {
    pub fn new() -> Cron {
        Cron {jobs: vec!()}
    }

    pub fn schedule(&mut self, job: CronJob) {
        self.jobs.push(job)
    }

    pub fn check(current_time: u32) {
        // for 
    }
}


#[test]
fn test_can_be_created() {
    let c = Cron::new();
}

#[test]
fn it_can_contain_cron_jobs() {
    let cj = CronJob::new(None, 1000);
    let mut c = Cron::new();

    c.schedule(cj);
}

#[test]
fn it_can_contain_multiple_cron_jobs() {
    let mut c = Cron::new();

    for i in 0..100 {
        let cj = CronJob::new(None, 100 * i);
        c.schedule(cj);
    }
}
