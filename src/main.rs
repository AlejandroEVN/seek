use clap::{Parser, ValueEnum};
use std::{
    fs::{self},
    io::{self, Write},
    os::unix::fs::FileTypeExt,
    path::PathBuf,
};

#[derive(Clone, Debug, ValueEnum)]
enum FileType {
    #[value(name = "f", help = "(file)")]
    File,
    #[value(name = "d", help = "(directory)")]
    Dir,
    #[value(name = "l", help = "(symlink)")]
    Symlink,
    #[value(name = "p", help = "(fifo)")]
    Fifo,
    #[value(name = "s", help = "(socket)")]
    Socket,
    #[value(name = "c", help = "(char-device)")]
    CharDevice,
    #[value(name = "b", help = "(block-device)")]
    BlockDevice,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    path: String,

    #[arg(short, long, value_enum)]
    dir_type: Option<FileType>,
}

fn main() {
    let args = Args::parse();

    process(&args);

    std::process::exit(0);
}

fn process(args: &Args) {
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(stdout);
    let mut to_visit = vec![PathBuf::from(&args.path)];

    while let Some(path) = to_visit.pop() {
        let dirs = match fs::read_dir(path) {
            Ok(dirs) => dirs,
            Err(err) => {
                eprintln!("{err}");
                continue;
            }
        };

        for entry in dirs.flatten() {
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                to_visit.push(entry.path());
            }

            let filter_by_type = match &args.dir_type {
                Some(fbt) => fbt,
                None => {
                    writeln!(handle, "{}", entry.path().to_str().unwrap()).unwrap();
                    continue;
                }
            };

            let mut matched = None;

            match filter_by_type {
                FileType::File => {
                    if file_type.is_file() {
                        matched = Some(entry.path());
                    }
                }
                FileType::Dir => {
                    if file_type.is_dir() {
                        matched = Some(entry.path());
                    }
                }
                FileType::Symlink => {
                    if file_type.is_symlink() {
                        matched = Some(entry.path());
                    }
                }
                FileType::Fifo => {
                    if file_type.is_fifo() {
                        matched = Some(entry.path());
                    }
                }
                FileType::Socket => {
                    if file_type.is_socket() {
                        matched = Some(entry.path());
                    }
                }
                FileType::CharDevice => {
                    if file_type.is_char_device() {
                        matched = Some(entry.path());
                    }
                }
                FileType::BlockDevice => {
                    if file_type.is_block_device() {
                        matched = Some(entry.path());
                    }
                }
            }

            if let Some(matched) = matched {
                writeln!(handle, "{:?}", matched).unwrap();
            }
        }
    }
}
