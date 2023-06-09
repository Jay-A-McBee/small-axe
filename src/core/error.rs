use std::fmt;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub struct Error<'path> {
    depth: usize,
    inner: ErrorInner<'path>,
}

#[derive(Debug)]
enum ErrorInner<'path> {
    Io { path: &'path Path, related: Related },
}

#[derive(Debug)]
pub enum Related {
    Metadata,
    Read,
}

impl<'path> Error<'path> {
    pub fn from_path(
        path: &'path Path,
        depth: usize,
        related: Related,
    ) -> Self {
        Error {
            depth,
            inner: ErrorInner::Io { path, related },
        }
    }
}

impl<'path> fmt::Display for Error<'path> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            ErrorInner::Io { path, related } => {
                write!(
                    f,
                    "io error encountered at the following path: {path:?}"
                )?;

                match related {
                    Related::Metadata => {
                        write!(f, "related to metadata access")?
                    }
                    Related::Read => write!(f, "related to file access")?,
                }
            }
        }

        Ok(())
    }
}

impl From<Error<'_>> for std::io::Error {
    fn from(err: Error) -> Self {
        match err.inner {
            ErrorInner::Io { path, .. } => io::Error::new(
                io::ErrorKind::Other,
                path.to_str().unwrap_or("path error"),
            ),
        }
    }
}
