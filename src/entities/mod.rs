use std::fmt;
use std::fs::ReadDir;
use std::path::PathBuf;
use std::result;
use std::vec;

#[derive(Debug)]
pub struct Error {
    msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg.as_str())
    }
}

type Result<T> = ::std::result::Result<T, Error>;

pub struct Tree {
    pub root: PathBuf,
}

#[derive(Debug)]
pub struct DirEntry {
    pub path: PathBuf,
    // pub depth: usize,
    // pub parent: Option<PathBuf>,
}

impl IntoIterator for Tree {
    type IntoIter = TreeIterator;
    type Item = Result<DirEntry>;

    fn into_iter(self) -> Self::IntoIter {
        TreeIterator {
            stack_list: vec![],
            root: Some(self.root),
            diagram: String::new(),
        }
    }
}

#[derive(Debug)]
pub enum DirEnt {
    Open(result::Result<ReadDir, Option<Error>>),
    Closed(vec::IntoIter<Result<DirEntry>>),
}

impl Iterator for DirEnt {
    type Item = Result<DirEntry>;

    fn next(&mut self) -> Option<Result<DirEntry>> {
        match *self {
            DirEnt::Open(ref mut it) => match *it {
                Err(ref mut err) => err.take().map(Err),
                Ok(ref mut rd) => rd.next().map(|r| match r {
                    Err(err) => Err(Error {
                        msg: String::from("Shit"),
                    }),
                    Ok(d) => Ok(DirEntry { path: d.path() }),
                }),
            },
            DirEnt::Closed(ref mut dir_list) => dir_list.next(),
        }
    }
}

pub struct TreeIterator {
    pub stack_list: Vec<DirEnt>,
    pub diagram: String,
    root: Option<PathBuf>,
}

impl Iterator for TreeIterator {
    type Item = Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(root) = self.root.take() {
            if let Some(entry) = self.handle_entry(DirEntry { path: root }) {
                return Some(entry);
            }
        }

        while !self.stack_list.is_empty() {
            let next = self
                .stack_list
                .last_mut()
                .expect("BUG: stack list should not be empty")
                .next();

            match next {
                None => {
                    self.stack_list.pop();
                }
                Some(Err(err)) => return Some(Err(err)),
                Some(Ok(dent)) => {
                    if let Some(Ok(result)) = self.handle_entry(dent) {
                        return Some(Ok(result));
                    }
                }
            }
        }

        None
    }
}

impl TreeIterator {
    pub fn handle_entry(
        &mut self,
        entry: DirEntry,
    ) -> Option<Result<DirEntry>> {
        if entry.path.is_dir() {
            let rd =
                std::fs::read_dir(entry.path.to_path_buf()).map_err(|err| {
                    Some(Error {
                        msg: String::from("Error reading entry"),
                    })
                });

            self.stack_list.push(DirEnt::Open(rd));
        }

        self.diagram
            .push_str(entry.path.as_os_str().to_str().unwrap());

        return Some(Ok(entry));
    }
}
