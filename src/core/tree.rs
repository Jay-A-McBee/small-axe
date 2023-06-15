use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::vec;

use crate::same_file::Handle;

use crate::cli::TreeIteratorFlags;

use super::dirent::DirEntry;
use super::pattern::Pattern;

pub struct Tree {
    pub pattern: Option<Pattern>,
    pub root: Option<PathBuf>,
    pub visit_all: bool,
    pub dirs_only: bool,
    pub max_depth: Option<usize>,
    pub dirs_first: bool,
    pub rev_alpha_sort: bool,
    pub last_mod_sort: bool,
    pub follow_symlinks: bool,
}

impl Tree {
    pub fn new(
        tree_iterator_flags: &mut TreeIteratorFlags,
        pattern: Option<Pattern>,
    ) -> Self {
        Tree {
            pattern,
            root: tree_iterator_flags.root.take(),
            max_depth: tree_iterator_flags.max_depth.take(),
            visit_all: tree_iterator_flags.visit_all,
            dirs_only: tree_iterator_flags.dirs_only,
            dirs_first: tree_iterator_flags.dirs_first,
            rev_alpha_sort: tree_iterator_flags.rev_alpha_sort,
            last_mod_sort: tree_iterator_flags.last_mod_sort,
            follow_symlinks: tree_iterator_flags.follow_symlinks,
        }
    }
}

#[derive(Debug)]
struct Visited {
    pub path: PathBuf,
}

pub struct TreeIterator {
    start: Option<PathBuf>,
    dirent_list: Vec<std::vec::IntoIter<DirEntry>>,
    visited_paths: Vec<DirHandle>,
    visit_all: bool,
    dirs_only: bool,
    dirs_first: bool,
    rev_alpha_sort: bool,
    last_mod_sort: bool,
    follow_symlinks: bool,
    max_depth: Option<usize>,
    depth: usize,
    pattern: Option<Pattern>,
}

impl TreeIterator {
    pub fn handle_entry(
        &mut self,
        mut dirent: DirEntry,
    ) -> std::io::Result<Option<DirEntry>> {
        // Don't descend into symlinked dir if linked path
        // is present in visited paths. (Recursion detected)
        //
        // This mirrors the behavior of linux tree cmd.

        // if dirent.is_symlink() && self.follow_symlinks {
        //     println!(
        //         "{:?} {}",
        //         self.visited_paths,
        //         format!(
        //             "{:?}/{:?}",
        //             self.root.as_ref().unwrap(),
        //             dirent.linked_path().unwrap()
        //         )
        //     );
        // }
        if dirent.is_symlink()
            && self.follow_symlinks
            && self.is_recursive(dirent.linked_path().unwrap().as_path())
        {
            dirent.is_recursive_link = true;
            return Ok(Some(dirent));
        }

        let (is_dir, dir_path) = if dirent.is_dir() {
            (true, Some(dirent.path()))
        } else if dirent.is_symlink()
            && self.follow_symlinks
            && dirent.linked_path().unwrap().is_dir()
        {
            (true, Some(dirent.linked_path().unwrap().as_path()))
        } else {
            (false, None)
        };

        if is_dir {
            let rd = std::fs::read_dir(dir_path.unwrap())
                .expect("Error reading dir");

            let mut entry_list: Vec<DirEntry> = rd
                .filter_map(|entry| {
                    if let Ok(entry) = entry {
                        let dir_entry =
                            DirEntry::from_entry(entry, self.depth + 1);

                        if dir_entry.is_dir() && self.follow_symlinks {
                            self.visited_paths.push(DirHandle {
                                path: dir_entry.path().to_path_buf(),
                            });
                        }

                        let keep = match (
                            self.pattern.as_ref(),
                            dir_entry.get_clean_name(),
                            dir_entry.is_dir(),
                        ) {
                            (Some(matcher), name, false) => {
                                let is_match = matcher.is_match(name);
                                (is_match && matcher.inclusive)
                                    || (!is_match && !matcher.inclusive)
                            }
                            _ => true,
                        };

                        return match (
                            keep,
                            self.visit_all,
                            dir_entry.is_hidden(),
                            self.dirs_only,
                            dir_entry.is_dir(),
                        ) {
                            (false, _, _, _, _) => None,
                            (true, false, true, _, _)
                            | (true, _, _, true, false) => None,
                            _ => Some(dir_entry),
                        };
                    }

                    None
                })
                .collect();

            entry_list.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
                (true, false) if self.dirs_first => Ordering::Less,
                (false, true) if self.dirs_first => Ordering::Greater,
                _ if self.last_mod_sort => {
                    todo!()
                    // a.get_last_modified().cmp(&b.get_last_modified())
                }
                _ => {
                    let a_name = a.get_clean_name();
                    let b_name = b.get_clean_name();

                    if self.rev_alpha_sort {
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

    fn is_recursive(&self, path: &Path) -> bool {
        Handle::from_path(path).map_or(false, |h| {
            self.visited_paths
                .iter()
                .any(|visited| Handle::from_path(&visited.path).unwrap() == h)
        })
    }
}

struct DirHandle {
    path: PathBuf,
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
                .expect("Dirent_list should not be empty");

            let (remaining, _) = iter.size_hint();

            if let Some(dent) = iter.next() {
                if let Ok(Some(dent)) = self.handle_entry(dent) {
                    if dent.is_dir()
                        && dent.depth > self.max_depth.unwrap_or(dent.depth)
                    {
                        // Pop this off the stack so we don't descend into this dir
                        self.dirent_list.pop();
                    } else {
                        self.depth = dent.depth;
                    }

                    return Some((remaining, dent));
                }
            } else {
                self.dirent_list.pop();
                self.depth -= 1;
            }
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
            dirent_list: vec![],
            visited_paths: vec![],
            visit_all: self.visit_all,
            dirs_only: self.dirs_only,
            max_depth: self.max_depth,
            dirs_first: self.dirs_first,
            rev_alpha_sort: self.rev_alpha_sort,
            last_mod_sort: self.last_mod_sort,
            follow_symlinks: self.follow_symlinks,
            depth: 0,
            pattern: self.pattern.take(),
        }
    }
}
