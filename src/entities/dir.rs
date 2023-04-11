use once_cell::sync::Lazy;

use std::cmp::Ordering;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::fs::DirEntry;
use std::fs::Metadata;
use std::io;
use std::path::{self, Path, PathBuf};
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::vec;

use crate::cli::flags::{Cmd, Flags};
use crate::output::colors::ColorParser;
use crate::output::ledger::Ledger;
use crate::output::pattern::PatternParser;

use super::contents::ExtData;
use super::{contents::Contents, file::File};

static mut VISITED: Lazy<HashSet<PathBuf>> = Lazy::new(HashSet::new);

// Box drawing unicode chars
const HORIZONTAL_PIPE: &str = "\u{2500}";
const VERTICAL_PIPE: &str = "\u{2502}";
const L_RIGHT: &str = "\u{2514}";
const T_RIGHT: &str = "\u{251C}";
const _L_LEFT: &str = "\u{2510}";
const _ARROW: &str = "\u{25B8}";

const ANSI_COLOR_RESET: &str = "\x1B[0m";

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

// pub trait Tree {
//     fn tree(
//         &self,
//         ledger: &mut Ledger,
//         config: Vec<Option<()>>,
//         level: usize,
//         with_meta: bool,
//         pattern_parser: &Option<PatternParser>,
//         remaining: bool,
//     ) -> std::io::Result<()>;

//     fn get_additional_info(&self) -> io::Result<String>;
//     fn get_permissions(&self, meta_data: &Metadata) -> io::Result<String>;
//     fn get_clean_name(&self) -> &str;
//     fn get_last_modified(&self) -> u64;
//     fn add_connectors(&self, config: &[Option<()>], remaining: bool);
//     fn extend_indent_list(
//         indent_levels: &[Option<()>],
//         remaining: bool,
//         level: usize,
//     ) -> Vec<Option<()>>;
//     fn add_name_entry(
//         name: &OsStr,
//         additional_info: &str,
//         level: usize,
//         // colors: &(String, String),
//         lossy: bool,
//     );
// }

// impl Tree for PathBuf {
//     fn tree(
//         &self,
//         ledger: &mut Ledger,
//         config: Vec<Option<()>>,
//         level: usize,
//         with_meta: bool,
//         pattern_parser: &Option<PatternParser>,
//         remaining: bool,
//     ) -> std::io::Result<()> {
//         let flags = Cmd::global();

//         let name = self.file_name().and_then(|name| name.to_str());
//         let is_dir = self.is_dir();

//         let remove_hidden = !flags.all && name.is_some() && name.unwrap().starts_with('.');
//         let remove_file = !is_dir && flags.dirs && !flags.prune;

//         if remove_hidden || remove_file {
//             return Ok(());
//         }

//         if pattern_parser.is_some() {
//             let pattern = pattern_parser.as_ref().unwrap();

//             if !pattern.is_match(name.unwrap()) && pattern.inclusive
//                 || pattern.is_match(name.unwrap()) && !pattern.inclusive
//             {
//                 return Ok(());
//             }
//         };

//         let is_symlink = self.is_symlink();

//         if is_symlink {
//             fs::read_link(&self).and_then(|linked_path| {
//                 if linked_path.is_dir() {
//                     if flags.follow_symlinks {
//                         linked_path.tree(
//                             ledger,
//                             <PathBuf as Tree>::extend_indent_list(&config, remaining, level),
//                             level + 1,
//                             with_meta,
//                             pattern_parser,
//                             remaining,
//                         )
//                     } else {
//                         ledger.add_connectors(&config, remaining);

//                         let mut display_name = if flags.full_path {
//                             self.canonicalize().map_or_else(
//                                 |_| std::ffi::OsString::new(),
//                                 |full_path| full_path.as_os_str().to_os_string(),
//                             )
//                         } else {
//                             self.file_name()
//                                 .expect("Error getting file name")
//                                 .to_os_string()
//                         };

//                         display_name.push(format!(" -> {:?}", linked_path.as_os_str()).as_str());

//                         ledger.add_name_entry(
//                             display_name.as_os_str(),
//                             "",
//                             flags.unprintable_question_mark,
//                             level,
//                         );

//                         Ok(())
//                     }
//                 } else {
//                     // file_count += 1;
//                     // print file
//                     todo!()
//                 }
//             })?;
//         } else if is_dir {
//             if config.len() > 1 {
//                 ledger.add_connectors(&config, remaining);
//             }

//             let additional_info = self.get_additional_info()?;

//             let display_name = if flags.full_path {
//                 self.canonicalize().map_or_else(
//                     |_| std::ffi::OsString::new(),
//                     |full_path| full_path.as_os_str().to_os_string(),
//                 )
//             } else {
//                 self.file_name()
//                     .expect("Error getting file name")
//                     .to_os_string()
//             };

//             ledger.add_name_entry(
//                 display_name.as_os_str(),
//                 additional_info.as_str(),
//                 flags.unprintable_question_mark,
//                 level,
//             );

//             fs::read_dir(&self).map(|entries| {
//                 let mut paths: Vec<PathBuf> = entries.fold(vec![], |mut acc, entry| {
//                     if entry.is_ok() {
//                         acc.push(entry.unwrap().path());
//                     }
//                     acc
//                 });

//                 paths.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
//                     (true, false) if flags.dirs_first => Ordering::Less,
//                     (false, true) if flags.dirs_first => Ordering::Greater,
//                     _ if flags.last_modified_sort => {
//                         a.get_last_modified().cmp(&b.get_last_modified())
//                     }
//                     _ => {
//                         let a_name = a.get_clean_name();
//                         let b_name = b.get_clean_name();

//                         if flags.reverse_alpha_sort {
//                             b_name.cmp(a_name)
//                         } else {
//                             a_name.cmp(b_name)
//                         }
//                     }
//                 });

//                 let final_idx = if paths.is_empty() { 0 } else { paths.len() - 1 };

//                 for (idx, path_buf) in paths.iter().enumerate() {
//                     // println!("remaining::idx::{idx}::{final_idx}");
//                     let _ = path_buf.tree(
//                         ledger,
//                         <PathBuf as Tree>::extend_indent_list(&config, remaining, level),
//                         level + 1,
//                         with_meta,
//                         pattern_parser,
//                         idx < final_idx,
//                     );
//                 }
//             })?;
//         } else {
//             ledger.add_connectors(&config, remaining);
//             let additional_info = self.get_additional_info()?;

//             let display_name = if flags.full_path {
//                 self.canonicalize().map_or_else(
//                     |_| std::ffi::OsString::new(),
//                     |full_path| full_path.as_os_str().to_os_string(),
//                 )
//             } else {
//                 self.file_name()
//                     .expect("Error getting file name")
//                     .to_os_string()
//             };

//             ledger.add_name_entry(
//                 display_name.as_os_str(),
//                 additional_info.as_str(),
//                 flags.unprintable_question_mark,
//                 level,
//             );
//             // file_count += 1;
//             // Some(Contents::File(File::new(path.to_owned(), with_meta)))
//             // print file
//             // todo!()
//         }

//         Ok(())
//     }

//     fn get_additional_info(&self) -> io::Result<String> {
//         let flags = Cmd::global();
//         let mut additional_info_list = Vec::new();

//         self.metadata().and_then(|meta_data| {
//             if flags.protections {
//                 let permissions = self.get_permissions(&meta_data)?;
//                 additional_info_list.push(permissions);
//             }

//             if flags.version {
//                 todo!()
//             }

//             if flags.size && !flags.human_readable_size {
//                 let size = format!("{} B", meta_data.len());
//                 additional_info_list.push(size)
//             }

//             if flags.human_readable_size {
//                 let bytes = meta_data.len();
//                 // TODO: Something still isn't quite right with this calculation
//                 let formatted = if bytes > MB {
//                     format!("{:?}.{} M", bytes / MB, (bytes % MB) / 100)
//                 } else if bytes < KB {
//                     format!("{bytes:?} B")
//                 } else {
//                     format!("{:?}.{} K", bytes / KB, (bytes % KB) / 100)
//                 };

//                 additional_info_list.push(formatted)
//             }

//             if flags.last_modified {
//                 let total_sec_since_1970 = self.get_last_modified();

//                 let (years, mut leftover) = calc_years(total_sec_since_1970);
//                 let _days = leftover / DAY;
//                 leftover %= DAY;

//                 let offset = (leftover as i64 / HOUR as i64) - 8;

//                 let _hours = if offset < 0 { 24 + offset } else { offset };

//                 leftover %= HOUR;
//                 let _mins = leftover / MINUTE;

//                 additional_info_list.push(years.to_string());
//             }

//             if flags.inode {
//                 additional_info_list.push(meta_data.ino().to_string());
//             }

//             if flags.group {
//                 additional_info_list.push(meta_data.gid().to_string());
//             }

//             if flags.device {
//                 additional_info_list.push(meta_data.dev().to_string());
//             }

//             if flags.username {
//                 additional_info_list.push(meta_data.uid().to_string());
//             }

//             if !additional_info_list.is_empty() {
//                 return Ok(format!("[{}]", additional_info_list.join(" ")));
//             }

//             Ok(String::from(""))
//         })
//     }

//     #[cfg(unix)]
//     fn get_permissions(&self, meta: &Metadata) -> io::Result<String> {
//         let mode = meta.mode();
//         // first char in permissions string
//         let mut permissions = if self.is_dir() {
//             String::from("d")
//         } else if self.is_symlink() {
//             String::from("l")
//         } else if mode & 0o10000 == 4096 {
//             String::from("p") // FIFO
//         } else if mode & 0o140000 == 49125 {
//             String::from("s") // socket
//         } else {
//             String::from(PERMISSIONS_DASH)
//         };

//         let ugo_perms = [
//             (PERMISSIONS_READ, mode & 0o400, 256), // user
//             (PERMISSIONS_WRITE, mode & 0o200, 128),
//             (PERMISSIONS_EXEC, mode & 0o100, 64),
//             (PERMISSIONS_READ, mode & 0o040, 32), // group
//             (PERMISSIONS_WRITE, mode & 0o020, 16),
//             (PERMISSIONS_EXEC, mode & 0o010, 8),
//             (PERMISSIONS_READ, mode & 0o004, 4), // other
//             (PERMISSIONS_WRITE, mode & 0o002, 2),
//             (PERMISSIONS_EXEC, mode & 0o001, 1),
//         ]
//         .into_iter()
//         .map(|(character, result, expected)| {
//             if result == expected {
//                 return character;
//             }

//             PERMISSIONS_DASH
//         })
//         .collect::<String>();

//         permissions.push_str(ugo_perms.as_str());
//         Ok(permissions)
//     }

//     fn get_clean_name(&self) -> &str {
//         self.file_name().and_then(|name| name.to_str()).map_or(
//             "NAME_UNAVAILABLE",
//             |name| match name.strip_prefix('.') {
//                 Some(n) => n,
//                 _ => name,
//             },
//         )
//     }

//     fn get_last_modified(&self) -> u64 {
//         self.metadata().map_or_else(
//             |_| {
//                 SystemTime::now()
//                     .duration_since(SystemTime::UNIX_EPOCH)
//                     .expect("error getting last modified")
//                     .as_secs()
//             },
//             |meta| {
//                 let sys_time = meta.modified().expect("error getting last modified");

//                 sys_time
//                     .duration_since(SystemTime::UNIX_EPOCH)
//                     .expect("error getting last modified")
//                     .as_secs()
//             },
//         )
//     }

//     fn add_connectors(&self, indent_levels: &[Option<()>], remaining: bool) {
//         let final_idx = indent_levels.len() - 1;

//         let connectors =
//             indent_levels
//                 .iter()
//                 .enumerate()
//                 .fold(String::new(), |mut acc, (idx, &space)| {
//                     let pipe = match (idx == final_idx, space) {
//                         (true, _) if remaining => T_RIGHT,
//                         (true, _) => L_RIGHT,
//                         (false, Some(_)) => VERTICAL_PIPE,
//                         _ => " ",
//                     };

//                     let offset = if idx > 1 { "    " } else { "" };

//                     acc.push_str(format!("{offset}{pipe}").as_str());
//                     acc
//                 });

//         print!("{connectors}");
//     }

//     fn extend_indent_list(
//         indent_levels: &[Option<()>],
//         remaining: bool,
//         level: usize,
//     ) -> Vec<Option<()>> {
//         let mut list = if !remaining {
//             indent_levels
//                 .iter()
//                 .enumerate()
//                 .map(|(idx, curr)| if idx == level as usize { None } else { *curr })
//                 .collect::<Vec<Option<()>>>()
//         } else {
//             indent_levels.to_owned()
//         };

//         list.push(Some(()));
//         list
//     }

//     // fn get_ansi_color_esc_seq(colors: &(String, String)) -> (String, &str) {
//     //     let all_parts = [colors.0.as_str(), ";", colors.1.as_str(), "m"];

//     //     let fg_bg = all_parts.iter().fold(String::new(), |mut acc, &val| {
//     //         match val {
//     //             "" => (),
//     //             ";" if colors.1.is_empty() => (),
//     //             "m" if acc.is_empty() => (),
//     //             _ => acc.push_str(val),
//     //         };
//     //         acc
//     //     });

//     //     if fg_bg.is_empty() {
//     //         (fg_bg, "")
//     //     } else {
//     //         (format!("\x1B[{fg_bg}"), ANSI_COLOR_RESET)
//     //     }
//     // }

//     fn add_name_entry(
//         name: &OsStr,
//         additional_info: &str,
//         level: usize,
//         // colors: &(String, String),
//         lossy: bool,
//     ) {
//         let (fg_bg, reset) = ("", "");
//         // Self::get_ansi_color_esc_seq(colors);

//         let connectors = if level == 0 {
//             format!("{additional_info}")
//         } else {
//             format!("{HORIZONTAL_PIPE}{HORIZONTAL_PIPE}{HORIZONTAL_PIPE}{additional_info}")
//         };

//         let line = if lossy {
//             let converted = name.to_string_lossy().to_string();
//             format!("{connectors} {fg_bg}{converted}{reset}\n")
//         } else {
//             let converted = name
//                 .to_str()
//                 .map_or_else(|| name.to_string_lossy().to_string(), |n| n.to_owned());

//             format!("{connectors} {fg_bg}{converted}{reset}\n")
//         };

//         print!("{line}")
//     }
// }

#[derive(Debug, Clone)]
pub struct DirTree {
    pub children: Vec<Contents>,
    pub metadata: Option<fs::Metadata>,
    pub path: PathBuf,
    pub linked_path: Option<PathBuf>,
    dir_count: u32,
    _file_count: u32,
}

impl DirTree {
    /// Recursively creates a directory tree structure.
    ///
    /// Flags consumed: -a, -d, -l, --dirsfirst, --prune
    pub fn new(
        dir_path: &path::Path,
        level: usize,
        with_meta: bool,
        pattern_parser: &Option<PatternParser>,
    ) -> Option<Self> {
        let flags = Cmd::global();

        if let Some(max_depth) = flags.max_depth {
            if level > max_depth {
                return None;
            }
        }

        if let Ok((children, file_count, dir_count)) = fs::read_dir(dir_path).map(|contents_iter| {
            let (mut file_count, mut dir_count) = (0, 0);

            let mut mapped = contents_iter
                .filter_map(|e| {
                    if let Ok(entry) = e {
                        let path = entry.path();

                        let name = path.file_name().and_then(|name| name.to_str());
                        let is_dir = path.is_dir();

                        let remove_hidden =
                            !flags.all && name.is_some() && name.unwrap().starts_with('.');

                        let remove_file = !is_dir && flags.dirs && !flags.prune;

                        if pattern_parser.is_some() {
                            let pattern = pattern_parser.as_ref().unwrap();

                            if !pattern.is_match(name.unwrap()) && pattern.inclusive
                                || pattern.is_match(name.unwrap()) && !pattern.inclusive
                            {
                                return None;
                            }
                        };

                        if remove_hidden || remove_file {
                            None
                        } else if path.is_symlink() {
                            fs::read_link(&path)
                                .map(|linked_path| {
                                    if linked_path.is_dir() {
                                        if flags.follow_symlinks {
                                            Self::new(
                                                &linked_path.canonicalize().unwrap(),
                                                level + 1,
                                                with_meta,
                                                pattern_parser,
                                            )
                                            .map(
                                                |mut dir_tree| {
                                                    // Bump dir count only if new returned Some(tree).
                                                    // If None, the dir might have been pruned, an unfollowed symlink, etc.
                                                    dir_count += 1;
                                                    dir_tree.set_linked_path(path);
                                                    Contents::Dir(dir_tree)
                                                },
                                            )
                                        } else {
                                            Some(Contents::Dir(DirTree {
                                                children: vec![],
                                                metadata: None,
                                                path,
                                                linked_path: Some(linked_path),
                                                dir_count: 0,
                                                _file_count: 0,
                                            }))
                                        }
                                    } else {
                                        file_count += 1;
                                        Some(Contents::File(File::new(path.to_owned(), with_meta)))
                                    }
                                })
                                .unwrap_or(None)
                        } else if is_dir {
                            Self::new(&path, level + 1, with_meta, pattern_parser).map(|dir_tree| {
                                // Bump dir count only if new returned Some(tree).
                                // If None, the dir might have been pruned.
                                dir_count += 1;
                                Contents::Dir(dir_tree)
                            })
                        } else {
                            file_count += 1;
                            Some(Contents::File(File::new(path.to_owned(), with_meta)))
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<Contents>>();

            mapped.sort_by(|a, b| {
                if flags.reverse_alpha_sort {
                    b.get_clean_name().cmp(a.get_clean_name())
                } else if flags.last_modified_sort {
                    a.get_last_modified().cmp(&b.get_last_modified())
                } else {
                    a.get_clean_name().cmp(b.get_clean_name())
                }
            });

            if flags.dirs_first {
                let (mut dirs, files): (Vec<Contents>, Vec<Contents>) =
                    mapped.into_iter().partition(|contents| contents.is_dir());

                dirs.extend_from_slice(files.as_slice());
                mapped = dirs
            }

            (mapped, file_count, dir_count)
        }) {
            if flags.prune && children.is_empty() {
                return None;
            }

            Some(Self {
                path: dir_path.canonicalize().expect("Error getting full path"),
                linked_path: None,
                _file_count: file_count,
                dir_count,
                children,
                metadata: if with_meta {
                    dir_path.metadata().ok()
                } else {
                    None
                },
            })
        } else {
            None
        }
    }

    pub fn get_total_contents_count(&self) -> (u32, u32) {
        self.children
            .iter()
            .fold((self.dir_count, 0), |mut acc, child| {
                let (dirs, files) = child.get_count();
                acc.0 += dirs;
                acc.1 += files;
                acc
            })
    }

    fn set_linked_path(&mut self, link_path: PathBuf) {
        let temp = self.path.to_path_buf();

        self.path = link_path;
        self.linked_path = Some(temp);
    }

    pub fn display_tree(
        children: &Vec<Contents>,
        f: &mut fmt::Formatter,
        indent_level_list: Vec<Option<()>>,
        level: u32,
        flags: &Flags,
        ledger: &Ledger,
    ) -> fmt::Result {
        let final_idx = children.len() - 1;

        for (idx, child) in children.iter().enumerate() {
            let has_remaining_children = idx < final_idx;

            let additional_info = child.get_additional_info(flags);

            let entity_type = (child.is_symlink(), child.is_dir());

            let (mut name, entity) = match entity_type {
                (true, _) => {
                    let linked_path = child.get_linked_path().map_or(String::new(), |p| unsafe {
                        if VISITED.contains(p) {
                            format!(" [Recursion detected] -> {:?}", p)
                        } else {
                            format!(" -> {:?}", p)
                        }
                    });

                    let mut raw_name = child.get_raw_name();

                    raw_name.push(linked_path);

                    (raw_name, "sym_link")
                }
                (_, true) => {
                    unsafe {
                        VISITED.insert(child.get_path().clone());
                    }
                    (
                        if flags.full_path {
                            child.get_path().as_os_str().to_owned()
                        } else {
                            child.get_raw_name()
                        },
                        "directory",
                    )
                }
                _ => (
                    if flags.full_path {
                        child.get_path().as_os_str().to_owned()
                    } else {
                        child.get_raw_name()
                    },
                    "file",
                ),
            };

            if flags.identify {
                name.push(child.get_identity_character());
            }

            let fg_bg = ColorParser::get_color_tuple(entity);

            ledger.add_connectors(f, &indent_level_list, has_remaining_children)?;
            ledger.add_name_entry(
                f,
                name.as_os_str(),
                additional_info.as_str(),
                fg_bg,
                flags.unprintable_question_mark,
            )?;

            match (child.get_children(), child.get_linked_path()) {
                (Some(children), None) if !children.is_empty() => {
                    let updated_indent_list = Ledger::extend_indent_list(
                        &indent_level_list,
                        has_remaining_children,
                        level,
                    );

                    Self::display_tree(children, f, updated_indent_list, level + 1, flags, ledger)?;
                }
                (Some(children), Some(path)) if !children.is_empty() => unsafe {
                    if !VISITED.contains(path) {
                        let updated_indent_list = Ledger::extend_indent_list(
                            &indent_level_list,
                            has_remaining_children,
                            level,
                        );

                        Self::display_tree(
                            children,
                            f,
                            updated_indent_list,
                            level + 1,
                            flags,
                            ledger,
                        )?;
                    }
                },
                _ => (),
            }
        }

        Ok(())
    }
}

impl fmt::Display for DirTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = self.path.file_name().map_or_else(
            || self.path.clone().into_os_string(),
            |name| name.to_os_string(),
        );

        unsafe {
            VISITED.insert(self.path.clone());
        }

        writeln!(f, "{:?}", name)?;

        let flags = Cmd::global();

        let indent = if flags.no_indent { "" } else { "    " };

        Self::display_tree(&self.children, f, vec![Some(())], 0, flags, &Ledger(indent))?;

        Ok(())
    }
}
