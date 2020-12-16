use std::{
    sync::{
        mpsc::{self, Receiver, SendError, Sender},
        Arc, Mutex,
    },
    thread,
};

type JobQueue = Arc<Mutex<Receiver<Box<dyn FnOnce() + Send>>>>;

struct Worker {
    _handle: thread::JoinHandle<()>,
}

impl Worker {
    fn new(job_queue: JobQueue) -> Self {
        let handle = thread::spawn(move || loop {
            let job = {
                let rx = job_queue.lock().unwrap();
                match rx.recv() {
                    Ok(j) => j,
                    Err(_) => break,
                }
            };
            job()
        });
        Self { _handle: handle }
    }
}

pub struct ThreadPool {
    _workers: Vec<Worker>,
    job_queue: Sender<Box<dyn FnOnce() + Send>>,
}

impl ThreadPool {
    pub fn new(num_threads: u16) -> Self {
        assert_ne!(num_threads, 0, "Expected non zero `num_threads`");
        let mut workers = Vec::with_capacity(num_threads as usize);

        let (tx, rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));
        for _ in 0..num_threads {
            let rx = Arc::clone(&rx);
            workers.push(Worker::new(rx));
        }

        Self {
            _workers: workers,
            job_queue: tx,
        }
    }

    pub fn execute<F: 'static + FnOnce() + Send>(
        &self,
        job: F,
    ) -> Result<(), SendError<Box<dyn FnOnce() + Send>>> {
        self.job_queue.send(Box::new(job))
    }
}

#[cfg(test)]
mod tests {
    use super::ThreadPool;
    use std::sync::mpsc;

    const TEST_THREADS: u16 = 4;

    #[test]
    fn it_works() {
        let pool = ThreadPool::new(TEST_THREADS);

        let (tx, rx) = mpsc::channel();
        for _ in 0..TEST_THREADS {
            let tx = tx.clone();
            pool.execute(move || {
                tx.send(1).unwrap();
            }).ok();
        }

        assert_eq!(
            rx.iter().take(TEST_THREADS as usize).fold(0, |a, b| a + b),
            TEST_THREADS
        );
    }

    #[test]
    #[should_panic]
    fn zero_threads_panic() {
        ThreadPool::new(0);
    }
}
