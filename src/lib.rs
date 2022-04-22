use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

// dyn = dynamic
// job is a type that implements some traits, but don’t specify what type the return value will be
type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        // if false -> assert! call panic!
        assert!(size > 0); // usize include 0, but create 0 thread has no sense
        let (sender, receiver) = mpsc::channel();

        // multiple producer single consumer
        // -> Arc: let multiple workers own the receiver
        // -> Mutex: let only one worker access the receiver at one time
        let receiver = Arc::new(Mutex::new(receiver));

        // with_capacity same as new, but more efficient (preallocates space vs resizes when
        // inserting)
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        // thread for running a request will only execute that request’s closure one time -> FnOnce
        // trait bound Send -> transfer closure between multiple thread
        // lifetime 'static -> don't know lifetime of the thread
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

// external code does not need to know -> private
struct Worker {
    id: usize,
    // The spawn function returns a JoinHandle<T> -> try to use it
    // () because this is the closure does not return anything
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => {
                    println!("Worker {} got a job; executing.", id);
                    job();
                }
                Message::Terminate => {
                    println!("Worker {} was told to terminate.", id);
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
