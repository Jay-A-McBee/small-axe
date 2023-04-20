use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::vec;

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
        let md = std::fs::metadata(&path).expect("failed getting metadata");

        Self {
            depth,
            file_type: md.file_type(),
            path: path.to_path_buf(),
            metadata: md,
        }
    }

    pub fn from_entry(entry: std::fs::DirEntry, depth: usize) -> Self {
        let md = entry.metadata().expect("Error getting metadata");

        Self {
            depth,
            file_type: md.file_type(),
            path: entry.path(),
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
