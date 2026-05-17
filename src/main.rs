use clap::Parser;
use std::{
    fs::{self, DirEntry},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Instant,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    path: String,

    #[arg(short, long)]
    threads: Option<usize>,
}

const DEFAULT_THREADS: usize = 1;

fn main() {
    let args = Args::parse();

    measure("naive", || naive_implementation(&args));
    measure("pro", || pro_implementation(&args));

    std::process::exit(0);
}

fn measure<T, F: FnOnce() -> T>(label: &str, f: F) {
    let start = Instant::now();
    f();
    println!("{} took: {:?}", label, start.elapsed());
}

fn naive_implementation(args: &Args) {
    let files = process(args.path.clone());

    println!("{:?}", files.len());
}

fn process<T: AsRef<Path>>(path: T) -> Vec<DirEntry> {
    let mut files = Vec::new();
    let mut to_visit = vec![path.as_ref().to_path_buf()];

    while let Some(path) = to_visit.pop() {
        match fs::read_dir(path) {
            Ok(mut dirs) => {
                while let Some(Ok(dir_entry)) = dirs.next() {
                    if dir_entry.metadata().unwrap().is_dir() {
                        to_visit.push(dir_entry.path());
                    } else {
                        files.push(dir_entry);
                    }
                }
            }
            Err(_) => {
                //eprintln!("error: no such path: {err}");
                continue;
            }
        }
    }

    files
}

fn pro_implementation(args: &Args) {
    let thread_count = args.threads.unwrap_or_else(|| {
        thread::available_parallelism()
            .map(|t| t.get())
            .unwrap_or(DEFAULT_THREADS)
    });

    let (tx, rx) = mpsc::channel::<PathBuf>();
    let (results_tx, results_rx) = mpsc::channel::<Vec<DirEntry>>();
    let shared_rx = Arc::new(Mutex::new(rx));

    let mut threads_handlers = vec![];

    tx.send(PathBuf::from(&args.path)).unwrap();

    for _ in 0..thread_count {
        let rx_clone = Arc::clone(&shared_rx);
        let tx_clone = tx.clone();
        let results_tx_clone = results_tx.clone();

        let handle = thread::spawn(move || {
            loop {
                let lock = rx_clone.lock().unwrap();

                match lock.recv() {
                    Ok(dir_entry) => {
                        drop(lock);

                        let mut files = Vec::new();

                        if dir_entry.metadata().unwrap().is_dir() {
                            match fs::read_dir(dir_entry) {
                                Ok(mut dirs) => {
                                    while let Some(Ok(dir_entry)) = dirs.next() {
                                        if dir_entry.metadata().unwrap().is_dir() {
                                            tx_clone.send(dir_entry.path()).unwrap();
                                            continue;
                                        } else {
                                            files.push(dir_entry);
                                        }
                                    }
                                }
                                Err(_) => {
                                    continue;
                                }
                            }
                        }

                        results_tx_clone.send(files).unwrap();
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });

        threads_handlers.push(handle);
    }

    drop(tx);
    drop(results_tx);

    for handle in threads_handlers {
        handle.join().unwrap();
    }

    let mut total = 0;
    for rx in results_rx {
        total += rx.len();
    }

    println!("{total}");
}
