mod args;

use crate::args::{Args, ArgsFileType};

use std::{
    fs::{self},
    io::{self, Write, stdout},
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
};

fn main() {
    let args = Args::get();

    process(&args, &mut stdout());

    std::process::exit(0);
}

fn process<W: Write>(args: &Args, writer: &mut W) {
    let mut handle = io::BufWriter::new(writer);
    let root_path = PathBuf::from(&args.path);

    if root_path.exists() {
        if should_print_entry_path(&root_path, &args) {
            writeln!(handle, "{}", &root_path.to_str().unwrap().to_string()).unwrap();
        }
    } else {
        writeln!(
            handle,
            "'{}': No such directory",
            &root_path.to_str().unwrap().to_string()
        )
        .unwrap();
        return;
    }

    let mut to_visit = vec![root_path];

    while let Some(path) = to_visit.pop() {
        let dirs = match fs::read_dir(&path) {
            Ok(dirs) => dirs,
            Err(_) => {
                writeln!(
                    handle,
                    "'{}': No such directory",
                    &path.to_str().unwrap().to_string()
                )
                .unwrap();
                continue;
            }
        };

        for entry in dirs.flatten() {
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => {
                    writeln!(
                        handle,
                        "'{}': No such file or directory",
                        entry.path().to_str().unwrap()
                    )
                    .unwrap();
                    continue;
                }
            };

            if let Some(prune_vec) = &args.prune {
                if let Some(name) = entry.file_name().to_str() {
                    if prune_vec
                        .iter()
                        .any(|p| p.prune_name.contains(name) && *p == file_type)
                    {
                        continue;
                    }
                }
            }

            if file_type.is_dir() {
                to_visit.push(entry.path());
            }

            let should_print = should_print_entry_path(&entry.path(), &args);

            if !should_print {
                continue;
            }

            writeln!(handle, "{}", entry.path().to_str().unwrap()).unwrap();
        }
    }

    handle.flush().unwrap();
}

fn should_print_entry_path(path: &Path, args: &Args) -> bool {
    let Ok(metadata) = path.symlink_metadata() else {
        return false;
    };

    let file_type = metadata.file_type();

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

    use crate::args::{PruneItem, PruneItemType};

    use super::*;
    use std::fs::File;

    const TMP_NESTED_DIR_NAME: &str = "nested_dir";
    const TMP_FILE_NAMES: [&str; 4] = ["text.c", "hello.txt", "nested_hello.txt", "nested_text.c"];

    fn create_tmp_dirs_and_files() -> (TempDir, String) {
        let tmp_dir = tempdir().unwrap();
        let tmp_path = tmp_dir.path();

        let nested_dir_path = tmp_path.join(TMP_NESTED_DIR_NAME);
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
            prune: None,
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
            output_as_string_vec[1].contains(&TMP_NESTED_DIR_NAME.to_string()),
            "Expected nested dir. Found none"
        );
    }

    #[test]
    fn test_returns_files() {
        let (_keep_alive, tmp_dir_path) = create_tmp_dirs_and_files();

        let args = Args {
            path: tmp_dir_path,
            dir_type: Some(ArgsFileType::File),
            prune: None,
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

    #[test]
    fn test_excludes_if_prune_arg_matches() {
        let (tmp_root_path, tmp_dir_path) = create_tmp_dirs_and_files();

        let args = Args {
            path: tmp_dir_path,
            dir_type: None,
            prune: Some(vec![PruneItem {
                prune_type: PruneItemType::All,
                prune_name: TMP_NESTED_DIR_NAME.to_string(),
            }]),
        };

        let mut output_buf = Vec::new();

        process(&args, &mut output_buf);

        let output_string = String::from_utf8(output_buf).unwrap();

        let output_as_string_vec: Vec<String> = output_string
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert!(output_as_string_vec.len() == 3, "Expected 3 results");

        assert!(
            output_as_string_vec.first().unwrap() == tmp_root_path.path(),
            "Expected root dir"
        );

        for (i, fake_file_path) in output_as_string_vec.iter().skip(1).enumerate() {
            assert!(
                fake_file_path.contains(TMP_FILE_NAMES[i]),
                "Expected nested file. Found none"
            );
        }
    }
}
