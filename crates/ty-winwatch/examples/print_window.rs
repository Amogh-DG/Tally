use std::thread::sleep;
use std::time::Duration;
use ty_winwatch::WindowWatcher;

fn main() {
    println!("Starting title catcher (ty-winwatch example)...");
    let mut watcher = WindowWatcher::new();
    loop {
        if let Some(sample) = watcher.poll() {
            println!("App: {}, Title: {}", sample.app, sample.title);
        } else {
            println!("(No foreground window)");
        }
        sleep(Duration::from_secs(1));
    }
}
