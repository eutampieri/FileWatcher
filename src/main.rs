use log::{debug, error, info, LevelFilter};
use notify::{watcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use syslog::{BasicLogger, Facility, Formatter3164};

fn main() {
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: "cd_watcher".into(),
        pid: 0,
    };
    let logger = syslog::unix(formatter).expect("could not connect to syslog");
    log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .expect("Could not initialize logger");

    let hooks: Vec<(String, String)> = std::env::var("CDWATCHER_CONF")
        .map(|x| std::fs::read_to_string(x).expect("Couldn't open conf file"))
        .unwrap_or("".into())
        .split("\n")
        .map(|x| x.split("\t").collect::<Vec<&str>>())
        .filter(|x| x.len() == 2)
        .map(|x| (x[0].into(), x[1].into()))
        .collect();
    info!("Loading {} watchers", hooks.len());
    for hook in hooks {
        info!("Loading watcher for {}", hook.0);
        std::thread::spawn(move || {
            // Create a channel to receive the events.
            let (tx, rx) = channel();

            // Create a watcher object, delivering debounced events.
            // The notification back-end is selected based on the platform.
            let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

            // Add a path to be watched. All files and directories at that path and
            // below will be monitored for changes.
            watcher.watch(&hook.0, RecursiveMode::NonRecursive).unwrap();

            loop {
                match rx.recv() {
                    Ok(_) => {
                        let mut pieces =
                            hook.1.split(" ").map(|x| x.into()).collect::<Vec<String>>();
                        let cmd_name = pieces.remove(0);
                        std::process::Command::new(cmd_name).args(pieces)
                    }
                    .spawn()
                    .map_or_else(
                        |r| error!("Could not execute {}: {:?}", hook.1, r),
                        |x| debug!("{} result: {:?}", hook.1, x),
                    ),
                    Err(e) => error!("watch error: {:?}", e),
                }
            }
        });
    }
    info!("Waiting for events...");
    loop {
        std::thread::sleep_ms(60000);
    }
}
