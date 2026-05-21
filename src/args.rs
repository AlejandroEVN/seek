use std::{fs::FileType, os::unix::fs::FileTypeExt, str::FromStr};

use clap::{Parser, ValueEnum};

#[derive(Clone, Debug, ValueEnum)]
pub enum ArgsFileType {
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
pub struct Args {
    #[arg(short, long, default_value = ".")]
    pub path: String,

    #[arg(short = 't', long = "type", value_enum)]
    pub dir_type: Option<ArgsFileType>,

    #[arg(long, num_args=1..)]
    pub prune: Option<Vec<PruneItem>>,
}

#[derive(Clone, Debug)]
pub enum PruneItemType {
    File,
    Dir,
    Symlink,
    Fifo,
    Socket,
    CharDevice,
    BlockDevice,
    All,
}

#[derive(Clone, Debug)]
pub struct PruneItem {
    pub prune_type: PruneItemType,
    pub prune_name: String,
}

impl PartialEq<FileType> for PruneItem {
    fn eq(&self, other: &FileType) -> bool {
        match &self.prune_type {
            PruneItemType::File => other.is_file(),
            PruneItemType::Dir => other.is_dir(),
            PruneItemType::Symlink => other.is_symlink(),
            PruneItemType::Fifo => other.is_fifo(),
            PruneItemType::Socket => other.is_socket(),
            PruneItemType::CharDevice => other.is_char_device(),
            PruneItemType::BlockDevice => other.is_block_device(),
            PruneItemType::All => true,
        }
    }
}

impl PruneItem {
    fn build_from_prefix(
        prune_type: PruneItemType,
        prefix: &str,
        name: &str,
    ) -> Result<Self, String> {
        if name.is_empty() {
            return Err(format!("Prune name cannot be empty after {}", prefix));
        }

        Ok(PruneItem {
            prune_type,
            prune_name: name.to_string(),
        })
    }
}

impl FromStr for PruneItem {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("--prune <PRUNE> cannot be empty".to_string());
        }

        match s.get(..3) {
            Some("@f/") => Self::build_from_prefix(PruneItemType::File, "@f/", &s[3..]),
            Some("@d/") => Self::build_from_prefix(PruneItemType::Dir, "@d/", &s[3..]),
            Some("@l/") => Self::build_from_prefix(PruneItemType::Symlink, "@l/", &s[3..]),
            Some("@p/") => Self::build_from_prefix(PruneItemType::Fifo, "@p/", &s[3..]),
            Some("@s/") => Self::build_from_prefix(PruneItemType::Socket, "@s/", &s[3..]),
            Some("@c/") => Self::build_from_prefix(PruneItemType::CharDevice, "@c/", &s[3..]),
            Some("@b/") => Self::build_from_prefix(PruneItemType::BlockDevice, "@b/", &s[3..]),
            _ => {
                if s.starts_with("@") && s.chars().nth(2) == Some('/') {
                    return Err(format!(
                        "Invalid type flag in {}. Use @f/, @d/, @l/, @p/, @s/, @c/ or @b/",
                        s
                    ));
                }

                Ok(PruneItem {
                    prune_type: PruneItemType::All,
                    prune_name: s.to_string(),
                })
            }
        }
    }
}

impl Args {
    pub fn get() -> Args {
        Args::parse()
    }
}
