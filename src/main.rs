use clap::{Parser, ValueEnum};
use std::{
    fs::{self, FileType},
    io::{self, Write, stdout},
    os::unix::fs::FileTypeExt,
    path::PathBuf,
};

#[derive(Clone, Debug, ValueEnum)]
enum ArgsFileType {
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
    dir_type: Option<ArgsFileType>,
}

fn main() {
    let args = Args::parse();

    process(&args, &mut stdout());

    std::process::exit(0);
}

fn process<W: Write>(args: &Args, writer: &mut W) {
    let mut handle = io::BufWriter::new(writer);
    let root_path = PathBuf::from(&args.path);

    let root_dir = fs::symlink_metadata(&root_path).unwrap();
    let should_print_root = should_print_entry(&root_dir.file_type(), args);

    if should_print_root {
        writeln!(handle, "{}", root_path.to_str().unwrap().to_string()).unwrap();
    }

    let mut to_visit = vec![root_path];

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

            if entry.file_type().unwrap().is_dir() {
                to_visit.push(entry.path());
            }

            let should_print = should_print_entry(&file_type, args);

            if should_print {
                writeln!(handle, "{}", entry.path().to_str().unwrap()).unwrap();
            }
        }
    }

    handle.flush().unwrap();
}

fn should_print_entry(file_type: &FileType, args: &Args) -> bool {
    let should_print = match &args.dir_type {
        Some(fbt) => match fbt {
            ArgsFileType::File => file_type.is_file(),
            ArgsFileType::Dir => file_type.is_dir(),
            ArgsFileType::Symlink => file_type.is_symlink(),
            ArgsFileType::Fifo => file_type.is_fifo(),
            ArgsFileType::Socket => file_type.is_socket(),
            ArgsFileType::CharDevice => file_type.is_char_device(),
            ArgsFileType::BlockDevice => file_type.is_block_device(),
        },
        None => true,
    };

    should_print
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};

    use super::*;
    use std::fs::File;

    const TMP_NESTED_DIR_PATH: &str = "nested_dir";
    const TMP_FILE_NAMES: [&str; 4] = ["text.c", "hello.txt", "nested_hello.txt", "nested_text.c"];

    fn create_tmp_dirs_and_files() -> (TempDir, String) {
        let tmp_dir = tempdir().unwrap();
        let tmp_path = tmp_dir.path();

        let nested_dir_path = tmp_path.join(TMP_NESTED_DIR_PATH);
        fs::create_dir(&nested_dir_path).expect("Test: failed to create nested dir in tmp");

        for fake_file in TMP_FILE_NAMES {
            let parent_dir_path = match fake_file.starts_with("nested_") {
                true => nested_dir_path.as_path(),
                false => tmp_path,
            };

            let fake_file_path = parent_dir_path.join(fake_file);
            File::create_new(&fake_file_path)
                .expect(format!("Test: failed to create fake file {}", fake_file).as_str());
        }

        let path_string = tmp_path.to_str().unwrap().to_string();

        (tmp_dir, path_string)
    }

    #[test]
    fn test_returns_dir() {
        let (_keep_alive, tmp_dir_path) = create_tmp_dirs_and_files();

        let args = Args {
            path: tmp_dir_path,
            dir_type: Some(ArgsFileType::Dir),
        };

        let mut output_buf = Vec::new();

        process(&args, &mut output_buf);

        let output_string = String::from_utf8(output_buf).unwrap();

        let output_as_string_vec: Vec<String> = output_string
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert!(output_as_string_vec.len() == 2, "Expected 2 dirs");
        assert!(
            output_as_string_vec[1].contains(&TMP_NESTED_DIR_PATH.to_string()),
            "Expected nested dir. Found none"
        );
    }

    #[test]
    fn test_returns_files() {
        let (_keep_alive, tmp_dir_path) = create_tmp_dirs_and_files();

        let args = Args {
            path: tmp_dir_path,
            dir_type: Some(ArgsFileType::File),
        };

        let mut output_buf = Vec::new();

        process(&args, &mut output_buf);

        let output_string = String::from_utf8(output_buf).unwrap();

        let output_as_string_vec: Vec<String> = output_string
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert!(output_as_string_vec.len() == 4, "Expected 4 files");

        for (i, fake_file_path) in output_as_string_vec.iter().enumerate() {
            assert!(
                fake_file_path.contains(TMP_FILE_NAMES[i]),
                "Expected nested file. Found none"
            );
        }
    }
}
