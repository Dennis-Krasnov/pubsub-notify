use std::thread;
use std::time::Duration;

fn main() {
    let broadcast = pubsub_notify::Broadcast::new();

    // ...
    thread::spawn({
        let broadcast = broadcast.clone();
        move || {
            thread::sleep(Duration::from_secs(5));
            
            broadcast.notify(&1);
            broadcast.notify(&2);
            broadcast.notify(&3);
        }
    });

    // ...
    thread::scope(|s| {
        s.spawn(|| broadcast.wait(1));
        s.spawn(|| broadcast.wait(2));
        s.spawn(|| broadcast.wait(3));
    });
}
