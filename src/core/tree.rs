use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::vec;

use crate::cli::flags::Cmd;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[derive(Default)]
pub struct UserFlags {
    pub help: bool, // done
    pub version: bool,
    pub all: bool,                     // done
    pub dirs: bool,                    // done
    pub full_path: bool,               // done
    pub no_indent: bool,               // done
    pub follow_symlinks: bool,         // done
    pub pattern_match: Option<String>, // done
    pub pattern_exclude: Option<String>,
    pub prune: bool, // done
    pub limit: Option<usize>,
    pub time_fmt: Option<String>,
    pub no_report: bool,           // done
    pub protections: bool,         // done
    pub size: bool,                //done
    pub human_readable_size: bool, //done
    pub username: bool,            // done unix only
    pub group: bool,               // done unix only
    pub last_modified: bool,
    pub inode: bool,                     // done unix only
    pub device: bool,                    // done unix only
    pub identify: bool,                  // done
    pub unprintable_question_mark: bool, // done
    pub unprintable_as_is: bool,         // done
    pub reverse_alpha_sort: bool,        // done
    pub last_modified_sort: bool,        // done
    pub dirs_first: bool,                // done
    pub output_file: Option<PathBuf>,    // done
    pub no_colors: bool,                 // done
    pub colors: bool,                    // done
    pub max_depth: Option<usize>,
}

const PERMISSIONS_READ: &str = "r";
const PERMISSIONS_WRITE: &str = "w";
const PERMISSIONS_EXEC: &str = "x";
const PERMISSIONS_DASH: &str = "-";

static S_IFMT: u32 = 0o170_000;
static S_IFSOCK: u32 = 0o140_000;
static S_IFIFO: u32 = 0o10_000;

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

pub struct Tree {
    pub root: Option<PathBuf>,
    pub opts: UserFlags,
}

impl Tree {
    pub fn new() -> Self {
        Tree {
            root: None,
            opts: UserFlags::default(),
        }
    }

    pub fn with_opts(mut self, env_args: Vec<String>) -> Self {
        let mut args_iter = Tree::process_opts(env_args).into_iter();

        while let Some(opt) = args_iter.next() {
            match opt.as_str() {
                "--help" => {
                    self.opts.help = true;
                }
                "--version" => {
                    self.opts.version = true;
                }
                "--noreport" => {
                    self.opts.no_report = true;
                }
                "--inodes" => {
                    self.opts.inode = true;
                }
                "--device" => {
                    self.opts.device = true;
                }
                "--dirsfirst" => {
                    self.opts.dirs_first = true;
                }
                "--prune" => {
                    self.opts.prune = true;
                }
                "--filelimit" => {
                    self.opts.limit = args_iter.next().map(|d| {
                        d.trim()
                            .parse::<usize>()
                            .expect("error parsing file limit value")
                    });
                }
                "-D" => self.opts.last_modified = true,
                "-a" => self.opts.all = true,
                "-d" => self.opts.dirs = true,
                "-f" => self.opts.full_path = true,
                "-F" => self.opts.identify = true,
                "-i" => self.opts.no_indent = true,
                "-l" => self.opts.follow_symlinks = true,
                "-x" => todo!(),
                "-P" => {
                    self.opts.pattern_match =
                        args_iter.next().map(|f| f.trim().to_owned())
                }
                "-I" => {
                    self.opts.pattern_exclude =
                        args_iter.next().map(|f| f.trim().to_owned())
                }
                "-p" => self.opts.protections = true,
                "-s" => self.opts.size = true,
                "-h" => self.opts.human_readable_size = true,
                "-u" => self.opts.username = true,
                "-g" => self.opts.group = true,
                "-q" => self.opts.unprintable_question_mark = true,
                "-N" => self.opts.unprintable_as_is = true,
                "-r" => self.opts.reverse_alpha_sort = true,
                "-t" => self.opts.last_modified_sort = true,
                "-n" => self.opts.no_colors = true,
                "-C" => self.opts.colors = true,
                "-A" => todo!(),
                "-S" => todo!(),
                "-L" => {
                    self.opts.max_depth = args_iter.next().map(|d| {
                        d.trim()
                            .parse::<usize>()
                            .expect("error parsing max depth value")
                    });
                }
                "-o" => {
                    self.opts.output_file = args_iter
                        .next()
                        .map(|f| PathBuf::from(f.trim().to_owned()));
                }
                _ => {
                    if opt.starts_with('-') {
                        println!("\nunrecognized flag - {opt}.\n");
                    } else {
                        self.root = Some(PathBuf::from(opt));
                    }
                }
            }
        }

        if self.root.is_none() {
            self.root = std::env::current_dir()
                .map_or_else(|_| Some(PathBuf::from(".")), |cwd| Some(cwd))
        }

        self
    }

    fn process_opts(env_args: Vec<String>) -> Vec<String> {
        let (additional_processing, ready): (Vec<_>, Vec<_>) =
            env_args.into_iter().partition(|flag| {
                !flag.starts_with("--")
                    && flag.starts_with('-')
                    && flag.len() > 2
            });

        if !additional_processing.is_empty() {
            return additional_processing
                .iter()
                .flat_map(|flag| {
                    flag.chars()
                        .skip(1)
                        .map(|ch| format!("-{ch}"))
                        .collect::<Vec<String>>()
                })
                .chain(ready.into_iter())
                .collect::<Vec<String>>();
        }

        ready
    }
}

#[derive(Debug)]
pub struct DirEntry {
    path: PathBuf,
    metadata: std::fs::Metadata,
    pub depth: usize,
    file_type: std::fs::FileType,
}

impl DirEntry {
    pub fn from_path(path: PathBuf, depth: usize) -> Self {
        let md = std::fs::metadata(&path).expect("failed to get metadata");
        // if path.is_symlink() {
        //     std::fs::symlink_metadata(&path).expect("failed to get metadata")
        // } else {
        // };

        Self {
            depth,
            file_type: md.file_type(),
            path: path.to_path_buf(),
            metadata: md,
        }
    }

    pub fn from_entry(entry: std::fs::DirEntry, depth: usize) -> Self {
        let path = entry.path();

        let md = entry.metadata().expect("failed to get metadata");

        // if path.is_symlink() {
        //     std::fs::symlink_metadata(&path).expect("failed to get metadata")
        // } else {
        // };

        Self {
            depth,
            path,
            file_type: md.file_type(),
            metadata: md,
        }
    }

    pub fn get_clean_name(&self) -> &str {
        self.path().file_name().map_or_else(
            || self.path().to_str().unwrap_or(""),
            |n| match n.to_str().unwrap().strip_prefix(".") {
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

    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }

    pub fn is_symlink(&self) -> bool {
        self.file_type.is_symlink()
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

    pub fn get_additional_info(&self) -> String {
        let mut additional_info_list = Vec::new();

        let flags = Cmd::global();

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

pub struct TreeIterator {
    start: Option<PathBuf>,
    opts: UserFlags,
    dirent_list: Vec<std::vec::IntoIter<DirEntry>>,
    depth: usize,
}

impl TreeIterator {
    pub fn handle_entry(
        &mut self,
        dirent: DirEntry,
    ) -> std::io::Result<Option<DirEntry>> {
        if dirent.is_dir() {
            let rd =
                std::fs::read_dir(dirent.path()).expect("Error reading dir");

            let mut entry_list: Vec<DirEntry> = rd
                .filter_map(|entry| {
                    if entry.is_ok() {
                        let dir_entry = DirEntry::from_entry(
                            entry.unwrap(),
                            self.depth + 1,
                        );

                        if (!self.opts.all && dir_entry.is_hidden())
                            || (!dir_entry.is_dir() && self.opts.dirs)
                        {
                            return None;
                        }

                        return Some(dir_entry);
                    }

                    None
                })
                .collect();

            entry_list.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
                (true, false) if self.opts.dirs_first => Ordering::Less,
                (false, true) if self.opts.dirs_first => Ordering::Greater,
                _ if self.opts.last_modified_sort => {
                    todo!()
                    // a.get_last_modified().cmp(&b.get_last_modified())
                }
                _ => {
                    let a_name = a.get_clean_name();
                    let b_name = b.get_clean_name();

                    if self.opts.reverse_alpha_sort {
                        b_name.cmp(a_name)
                    } else {
                        a_name.cmp(b_name)
                    }
                }
            });

            self.dirent_list.push(entry_list.into_iter());
        }

        Ok(Some(dirent))
    }
}

impl Iterator for TreeIterator {
    type Item = (usize, DirEntry);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(root) = self.start.take() {
            if let Ok(Some(dent)) =
                self.handle_entry(DirEntry::from_path(root, self.depth))
            {
                return Some((1, dent));
            }
        }

        while !self.dirent_list.is_empty() {
            self.depth = self.dirent_list.len();

            let iter = self
                .dirent_list
                .last_mut()
                .expect("BUG: dirent_list should not be empty");

            let (remaining, _) = iter.size_hint();

            match iter.next() {
                Some(dent) => {
                    if let Ok(Some(dent)) = self.handle_entry(dent) {
                        if dent.depth
                            > self.opts.max_depth.unwrap_or(dent.depth)
                        {
                            self.dirent_list.pop();
                            return None;
                        }

                        self.depth = dent.depth;
                        return Some((remaining, dent));
                    }
                }
                None => {
                    self.dirent_list.pop();
                    self.depth = self.depth - 1;
                }
            };
        }

        None
    }
}

impl IntoIterator for Tree {
    type IntoIter = TreeIterator;
    type Item = (usize, DirEntry);

    fn into_iter(mut self) -> Self::IntoIter {
        TreeIterator {
            start: self.root.take(),
            opts: self.opts,
            dirent_list: vec![],
            depth: 0,
        }
    }
}
