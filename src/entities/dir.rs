use once_cell::sync::Lazy;

use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::flags::{Cmd, Flags};
use crate::output::colors::ColorParser;
use crate::output::ledger::Ledger;
use crate::output::pattern::PatternParser;

use super::{contents::Contents, file::File};

static mut VISITED: Lazy<HashSet<PathBuf>> = Lazy::new(HashSet::new);

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
        dir_path: &Path,
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

            mapped.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
                (true, false) if flags.dirs_first => Ordering::Less,
                (false, true) if flags.dirs_first => Ordering::Greater,
                _ if flags.last_modified_sort => a.get_last_modified().cmp(&b.get_last_modified()),
                _ => {
                    let a_name = a.get_clean_name();
                    let b_name = b.get_clean_name();

                    if flags.reverse_alpha_sort {
                        b_name.cmp(a_name)
                    } else {
                        a_name.cmp(b_name)
                    }
                }
            });

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
