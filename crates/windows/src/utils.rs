use std::{
    io,
    sync::{atomic::AtomicBool, Arc},
    thread,
    time::{Duration, Instant},
};

pub fn run_timeout<T: Send + 'static>(
    func: impl FnOnce() -> T + Send + 'static,
    timeout: Duration,
) -> io::Result<T> {
    let done = Arc::new(AtomicBool::new(false));
    let done_inner = done.clone();

    let start_at = Instant::now();
    let thread_handler = thread::Builder::new().spawn(move || {
        let result = func();

        done_inner.swap(true, std::sync::atomic::Ordering::Relaxed);
        result
    })?;

    // wait for done or timeout
    loop {
        if done.load(std::sync::atomic::Ordering::Relaxed) {
            break match thread_handler.join() {
                Ok(result) => Ok(result),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Thread panic")),
            };
        } else if start_at.elapsed() > timeout {
            break Err(io::Error::new(io::ErrorKind::TimedOut, "Timeout"));
        } else {
            thread::sleep(Duration::from_millis(20));
            continue;
        }
    }
}
