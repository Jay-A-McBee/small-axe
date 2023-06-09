use std::fs;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(linux)]
use std::os::linux::fs::MetadataExt;

use crate::cli::Cmd;

const PERMISSIONS_READ: &str = "r";
const PERMISSIONS_WRITE: &str = "w";
const PERMISSIONS_EXEC: &str = "x";
const PERMISSIONS_DASH: &str = "-";

const S_IFMT: u32 = 0o170_000;
const S_IFSOCK: u32 = 0o140_000;
const S_IFIFO: u32 = 0o10_000;

const MINUTE: u64 = 60_u64;
const HOUR: u64 = MINUTE * 60_u64;
const DAY: u64 = HOUR * 24_u64;
const NON_LEAP_YEAR: u64 = DAY * 365_u64;
const LEAP_YEAR: u64 = DAY * 366_u64;

const KB: u64 = 1000;
const MB: u64 = KB * 1000;

fn calc_years(time: u64) -> (u64, u64) {
    let mut count = 0;
    let mut next_leap_year = 2;
    let mut current = time;

    while (current >= NON_LEAP_YEAR) || (current >= LEAP_YEAR) {
        if count == next_leap_year {
            current -= LEAP_YEAR;
            next_leap_year += 4;
        } else {
            current -= NON_LEAP_YEAR;
        }

        count += 1;
    }

    (count, current)
}

pub enum ExtData {
    Inode,
    Gid,
    Uid,
    Device,
    Permissions,
}

#[derive(Debug)]
pub struct DirEntry {
    path: PathBuf,
    metadata: std::fs::Metadata,
    pub depth: usize,
    file_type: std::fs::FileType,
    linked_path: Option<PathBuf>,
    pub is_recursive_link: bool,
}

impl DirEntry {
    pub fn from_path(path: PathBuf, depth: usize) -> Self {
        let md = fs::metadata(&path).expect("failed to get metadata");

        let linked_path: Option<PathBuf> = if md.file_type().is_symlink() {
            Some(fs::read_link(&path).expect("failed to get linked path"))
        } else {
            None
        };

        Self {
            depth,
            path,
            linked_path,
            file_type: md.file_type(),
            metadata: md,
            is_recursive_link: false,
        }
    }

    pub fn from_entry(entry: fs::DirEntry, depth: usize) -> Self {
        let path = entry.path();

        let md = entry.metadata().expect("failed to get metadata");

        // if path.is_symlink() {
        //     std::fs::symlink_metadata(&path).expect("failed to get metadata")
        // } else {
        // };

        let linked_path: Option<PathBuf> = if md.file_type().is_symlink() {
            Some(fs::read_link(&path).expect("failed to get linked path"))
        } else {
            None
        };

        Self {
            depth,
            path,
            linked_path,
            file_type: md.file_type(),
            metadata: md,
            is_recursive_link: false,
        }
    }

    pub fn get_clean_name(&self) -> &str {
        self.path().file_name().map_or_else(
            || self.path().to_str().unwrap_or(""),
            |n| match n.to_str().unwrap().strip_prefix('.') {
                Some(name) => name,
                None => n.to_str().unwrap(),
            },
        )
    }

    pub fn is_hidden(&self) -> bool {
        self.path().file_name().map_or(false, |n| {
            n.to_str().map_or(false, |name| name.starts_with('.'))
        })
    }

    pub fn get_name(&self) -> Option<&std::ffi::OsStr> {
        self.path().file_name()
    }

    pub fn get_depth(&self) -> &usize {
        &self.depth
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn full_path(&self) -> PathBuf {
        if self.linked_path.is_some() {
            PathBuf::from(self.get_name().unwrap())
        } else {
            self.path.canonicalize().unwrap()
        }
    }

    pub fn linked_path(&self) -> Option<&PathBuf> {
        self.linked_path.as_ref()
    }

    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }

    pub fn is_symlink(&self) -> bool {
        self.file_type.is_symlink()
    }

    pub fn is_executable(&self) -> bool {
        !self.is_dir() && self.metadata.mode() & 0o111 != 0
    }

    pub fn get_file_type(&self) -> &'static str {
        if self.is_symlink() {
            "sym_link"
        } else if self.is_dir() {
            "directory"
        } else if self.is_executable() {
            "executable"
        } else {
            ""
        }
    }

    pub fn get_last_modified(&self) -> Duration {
        self.metadata.modified().map_or_else(
            |_| {
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("error getting last modified")
            },
            |mod_time| {
                mod_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("error getting last modified")
            },
        )
    }

    pub fn get_size(&self) -> u64 {
        self.metadata.len()
    }

    pub fn get_additional_info(&self, cmds: &Cmd) -> String {
        let mut additional_info_list = Vec::new();

        let flags = &cmds.flags;

        if flags.protections {
            additional_info_list.push(self.get_ext_data(ExtData::Permissions));
        }

        if flags.version {
            todo!()
        }

        if flags.size && !flags.human_readable_size {
            let size = format!("{} B", self.get_size());
            additional_info_list.push(size)
        }

        if flags.human_readable_size {
            let bytes = self.get_size();
            // TODO: Something still isn't quite right with this calculation
            let formatted = if bytes > MB {
                format!("{:?}.{} M", bytes / MB, (bytes % MB) / 100)
            } else if bytes < KB {
                format!("{bytes:?} B")
            } else {
                format!("{:?}.{} K", bytes / KB, (bytes % KB) / 100)
            };

            additional_info_list.push(formatted)
        }

        if flags.last_modified {
            let total_sec_since_1970 = self.get_last_modified().as_secs();

            let (years, mut leftover) = calc_years(total_sec_since_1970);
            let _days = leftover / DAY;
            leftover %= DAY;

            let offset = (leftover as i64 / HOUR as i64) - 8;

            let _hours = if offset < 0 { 24 + offset } else { offset };

            leftover %= HOUR;
            let _mins = leftover / MINUTE;

            additional_info_list.push(years.to_string());
        }

        if flags.inode {
            additional_info_list.push(self.get_ext_data(ExtData::Inode));
        }

        if flags.group {
            additional_info_list.push(self.get_ext_data(ExtData::Gid));
        }

        if flags.device {
            additional_info_list.push(self.get_ext_data(ExtData::Device));
        }

        if flags.username {
            additional_info_list.push(self.get_ext_data(ExtData::Uid));
        }

        if !additional_info_list.is_empty() {
            return format!("[{}] ", additional_info_list.join(" "));
        }

        String::from("")
    }

    pub fn get_identity_character(&self) -> &str {
        let mode = self.metadata.mode();

        if self.is_symlink() {
            ""
        } else if self.is_dir() {
            "/"
        } else if mode & S_IFMT == S_IFIFO {
            "|" // FIFO
        } else if mode & S_IFMT == S_IFSOCK {
            "=" // socket
        } else {
            "*" // executable file
        }
    }

    #[cfg(linux)]
    pub fn get_ext_data(&self, ext_data: ExtData) -> String {
        match ext_data {
            ExtData::Inode => self.metadata.st_ino().to_string(),
            ExtData::Gid => self.metadata.st_gid().to_string(),
            ExtData::Uid => self.metadata.st_uid().to_string(),
            ExtData::Device => self.metadata.st_dev().to_string(),
            ExtData::Permissions => {
                let mode = self.metadata.st_mode();
                // first char in permissions string
                let mut permissions = if self.is_dir() {
                    String::from("d")
                } else if self.is_symlink() {
                    String::from("l")
                } else if mode & S_IFMT == S_IFIFO {
                    String::from("p") // FIFO
                } else if mode & S_IFMT == S_IFSOCK {
                    String::from("s") // socket
                } else {
                    String::from(PERMISSIONS_DASH)
                };

                let ugo_perms = [
                    (PERMISSIONS_READ, mode & 0o400, 256), // user
                    (PERMISSIONS_WRITE, mode & 0o200, 128),
                    (PERMISSIONS_EXEC, mode & 0o100, 64),
                    (PERMISSIONS_READ, mode & 0o040, 32), // group
                    (PERMISSIONS_WRITE, mode & 0o020, 16),
                    (PERMISSIONS_EXEC, mode & 0o010, 8),
                    (PERMISSIONS_READ, mode & 0o004, 4), // other
                    (PERMISSIONS_WRITE, mode & 0o002, 2),
                    (PERMISSIONS_EXEC, mode & 0o001, 1),
                ]
                .into_iter()
                .map(|(character, result, expected)| {
                    if result == expected {
                        return character;
                    }

                    PERMISSIONS_DASH
                })
                .collect::<String>();

                permissions.push_str(ugo_perms.as_str());
                permissions
            }
        }
    }

    #[cfg(unix)]
    pub fn get_ext_data(&self, ext_data: ExtData) -> String {
        match ext_data {
            ExtData::Inode => self.metadata.ino().to_string(),
            ExtData::Gid => self.metadata.gid().to_string(),
            ExtData::Uid => self.metadata.uid().to_string(),
            ExtData::Device => self.metadata.dev().to_string(),
            ExtData::Permissions => {
                let mode = self.metadata.mode();
                // first char in permissions string
                let mut permissions = if self.is_dir() {
                    String::from("d")
                } else if self.is_symlink() {
                    String::from("l")
                } else if mode & S_IFMT == S_IFIFO {
                    String::from("p") // FIFO
                } else if mode & S_IFMT == S_IFSOCK {
                    String::from("s") // socket
                } else {
                    String::from(PERMISSIONS_DASH)
                };

                let ugo_perms = [
                    (PERMISSIONS_READ, mode & 0o400, 256), // user
                    (PERMISSIONS_WRITE, mode & 0o200, 128),
                    (PERMISSIONS_EXEC, mode & 0o100, 64),
                    (PERMISSIONS_READ, mode & 0o040, 32), // group
                    (PERMISSIONS_WRITE, mode & 0o020, 16),
                    (PERMISSIONS_EXEC, mode & 0o010, 8),
                    (PERMISSIONS_READ, mode & 0o004, 4), // other
                    (PERMISSIONS_WRITE, mode & 0o002, 2),
                    (PERMISSIONS_EXEC, mode & 0o001, 1),
                ]
                .into_iter()
                .map(|(character, result, expected)| {
                    if result == expected {
                        return character;
                    }

                    PERMISSIONS_DASH
                })
                .collect::<String>();

                permissions.push_str(ugo_perms.as_str());
                permissions
            }
        }
    }
}
