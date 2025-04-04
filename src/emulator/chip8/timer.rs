use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time,
};

const TIMER_RATE: u64 = 60;

pub struct Timer(Arc<Mutex<u8>>);

impl Timer {
    pub fn set(&self, value: u8) {
        let mut timer = self.0.lock().unwrap();
        *timer = value;
        drop(timer);

        let mutex = Arc::clone(&self.0);
        thread::spawn(move || loop {
            let mut timer = mutex.lock().unwrap();
            if *timer == 0 {
                break;
            }
            *timer -= 1;
            drop(timer);
            sleep(time::Duration::from_nanos(1_000_000_000 / TIMER_RATE));
        });
    }

    pub fn get(&self) -> u8 {
        let timer = self.0.lock().unwrap();
        *timer
    }

    pub fn init() -> Self {
        Timer(Arc::new(Mutex::new(0)))
    }
}
