use std::ffi::OsString;
use std::fs::Metadata;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

use super::{dir::DirTree, file::File};
use crate::cli::flags::Flags;

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

const PERMISSIONS_READ: &str = "r";
const PERMISSIONS_WRITE: &str = "w";
const PERMISSIONS_EXEC: &str = "x";
const PERMISSIONS_DASH: &str = "-";

static S_IFMT: u32 = 0o170000;
static S_IFSOCK: u32 = 0o140000;
static S_IFIFO: u32 = 0o10000;

pub enum ExtData {
    Inode,
    Gid,
    Uid,
    Device,
    Permissions,
}

#[derive(Debug, Clone)]
pub enum Contents {
    File(File),
    Dir(DirTree),
}

impl Contents {
    pub fn is_dir(&self) -> bool {
        self.get_path().is_dir()
    }

    pub fn is_symlink(&self) -> bool {
        self.get_path().is_symlink()
    }

    pub fn is_fifo(&self) -> bool {
        self.get_metadata()
            .map_or(false, |meta| meta.mode() & S_IFMT == S_IFIFO)
    }

    pub fn is_socket(&self) -> bool {
        self.get_metadata()
            .map_or(false, |meta| meta.mode() & S_IFMT == S_IFSOCK)
    }

    pub fn get_count(&self) -> (u32, u32) {
        match &self {
            Contents::Dir(dir) => dir.get_total_contents_count(),
            Contents::File(_file) => (0_u32, 1_u32),
        }
    }

    pub fn get_metadata(&self) -> Option<&Metadata> {
        let metadata = match &self {
            Contents::Dir(dir) => &dir.metadata,
            Contents::File(file) => &file.metadata,
        };

        metadata.as_ref()
    }

    pub fn get_path(&self) -> &PathBuf {
        match &self {
            Contents::Dir(dir) => &dir.path,
            Contents::File(file) => &file.path,
        }
    }

    pub fn get_linked_path(&self) -> Option<&PathBuf> {
        let linked_path = match &self {
            Contents::Dir(dir) => &dir.linked_path,
            Contents::File(_) => &None,
        };

        linked_path.as_ref()
    }

    pub fn get_children(&self) -> Option<&Vec<Contents>> {
        match &self {
            Contents::Dir(dir) => Some(&dir.children),
            _ => None,
        }
    }

    pub fn get_raw_name(&self) -> OsString {
        let path = self.get_path();

        path.file_name()
            .map_or_else(|| path.clone().into_os_string(), |name| name.to_os_string())
    }

    pub fn get_clean_name(&self) -> &str {
        let path = self.get_path();

        path.file_name().and_then(|name| name.to_str()).map_or(
            "NAME_UNAVAILABLE",
            |name| match name.strip_prefix('.') {
                Some(n) => n,
                _ => name,
            },
        )
    }

    pub fn get_last_modified(&self) -> Duration {
        self.get_metadata().map_or_else(
            || {
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("error getting last modified")
            },
            |meta| {
                let sys_time = meta.modified().expect("error getting last modified");

                sys_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("error getting last modified")
            },
        )
    }

    pub fn get_size(&self) -> u64 {
        let size = self.get_metadata().map_or(0, |meta| meta.len());

        match &self {
            Contents::Dir(dir) => {
                let child_size: u64 = dir.children.iter().map(|child| child.get_size()).sum();
                size + child_size
            }
            _ => size,
        }
    }

    pub fn get_additional_info(&self, flags: &Flags) -> String {
        let mut additional_info_list = Vec::new();

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
            return format!("[{}]", additional_info_list.join(" "));
        }

        String::from("")
    }

    pub fn get_identity_character(&self) -> &str {
        if self.is_symlink() {
            ""
        } else if self.is_dir() {
            "/"
        } else if self.is_fifo() {
            "|" // FIFO
        } else if self.is_socket() {
            "=" // socket
        } else {
            "*" // executable file
        }
    }

    #[cfg(unix)]
    pub fn get_ext_data(&self, ext_data: ExtData) -> String {
        self.get_metadata()
            .map_or(String::new(), |meta| match ext_data {
                ExtData::Inode => meta.ino().to_string(),
                ExtData::Gid => meta.gid().to_string(),
                ExtData::Uid => meta.uid().to_string(),
                ExtData::Device => meta.dev().to_string(),
                ExtData::Permissions => {
                    let mode = meta.mode();
                    // first char in permissions string
                    let mut permissions = if self.is_dir() {
                        String::from("d")
                    } else if self.is_symlink() {
                        String::from("l")
                    } else if self.is_fifo() {
                        String::from("p") // FIFO
                    } else if self.is_socket() {
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
            })
    }
}
