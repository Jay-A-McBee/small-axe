use once_cell::sync::OnceCell;
use std::path::PathBuf;

#[derive(Debug, Default)]
#[cfg(unix)]
pub struct Flags {
    pub dir_path: Option<PathBuf>, // done
    pub help: bool,                // done
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
    pub max_depth: Option<usize>,        // done
}

impl Flags {
    #[cfg(unix)]
    pub fn get_metatdata_flags(&self) -> [bool; 10] {
        [
            self.protections,
            self.size,
            self.human_readable_size,
            self.last_modified,
            self.last_modified_sort,
            self.inode,
            self.group,
            self.device,
            self.username,
            self.colors,
        ]
    }
}

#[cfg(windows)]
pub struct Flags {
    pub alt_line_chars: bool,
    pub file_names: bool,
}

static USER_FLAGS: OnceCell<Flags> = OnceCell::new();

#[derive(Debug)]
pub struct Cmd {}

impl Cmd {
    pub fn process_args(args: Vec<String>) -> Vec<String> {
        let (additional_processing, ready): (Vec<_>, Vec<_>) = args
            .into_iter()
            .partition(|flag| !flag.starts_with("--") && flag.starts_with('-') && flag.len() > 2);

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

    pub fn global() -> &'static Flags {
        USER_FLAGS.get().unwrap()
    }

    pub fn from_cli(args: Vec<String>) {
        let mut flags = Flags {
            dir_path: None,
            ..Flags::default()
        };

        let commands = Cmd::process_args(args);

        let mut cmd_flags = commands.iter();

        while let Some(flag) = cmd_flags.next() {
            match flag.as_str() {
                "--help" => {
                    flags.help = true;
                }
                "--version" => {
                    flags.version = true;
                }
                "--noreport" => {
                    flags.no_report = true;
                }
                "--inodes" => {
                    flags.inode = true;
                }
                "--device" => {
                    flags.device = true;
                }
                "--dirsfirst" => {
                    flags.dirs_first = true;
                }
                "--prune" => {
                    flags.prune = true;
                }
                "--filelimit" => {
                    flags.limit = cmd_flags.next().map(|d| {
                        d.trim()
                            .parse::<usize>()
                            .expect("error parsing file limit value")
                    })
                }
                "-D" => flags.last_modified = true,
                "-a" => flags.all = true,
                "-d" => flags.dirs = true,
                "-f" => flags.full_path = true,
                "-F" => flags.identify = true,
                "-i" => flags.no_indent = true,
                "-l" => flags.follow_symlinks = true,
                "-x" => todo!(),
                "-P" => flags.pattern_match = cmd_flags.next().map(|f| f.trim().to_owned()),
                "-I" => flags.pattern_exclude = cmd_flags.next().map(|f| f.trim().to_owned()),
                "-p" => flags.protections = true,
                "-s" => flags.size = true,
                "-h" => flags.human_readable_size = true,
                "-u" => flags.username = true,
                "-g" => flags.group = true,
                "-q" => flags.unprintable_question_mark = true,
                "-N" => flags.unprintable_as_is = true,
                "-r" => flags.reverse_alpha_sort = true,
                "-t" => flags.last_modified_sort = true,
                "-n" => flags.no_colors = true,
                "-C" => flags.colors = true,
                "-A" => todo!(),
                "-S" => todo!(),
                "-L" => {
                    flags.max_depth = cmd_flags.next().map(|d| {
                        d.trim()
                            .parse::<usize>()
                            .expect("error parsing max depth value")
                    })
                }
                "-o" => {
                    flags.output_file = cmd_flags.next().map(|f| PathBuf::from(f.trim().to_owned()))
                }
                _ => {
                    if flag.starts_with('-') {
                        println!("\n{flag} is not a valid flag.\n");
                    } else {
                        flags.dir_path = Some(PathBuf::from(flag));
                    }
                }
            }
        }

        if flags.dir_path.is_none() {
            flags.dir_path = Some(std::env::current_dir().expect("Error getting cwd"));
        }

        USER_FLAGS
            .set(flags)
            .expect("Failed to set global user flags")
    }

    pub fn requires_metadata() -> bool {
        Self::global()
            .get_metatdata_flags()
            .iter()
            .any(|&flag| flag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stringify(str_refs: Vec<&str>) -> Vec<String> {
        str_refs
            .iter()
            .map(|&a| String::from(a))
            .collect::<Vec<String>>()
    }

    #[test]
    fn parses_hyphen_delimited_args() {
        let args = stringify(vec![
            "-a",
            "-h",
            "-s",
            "-d",
            "--dirsfirst",
            "--prune",
            "src",
        ]);

        let result = Cmd::process_args(args.clone());

        assert_eq!(result, args)
    }

    #[test]
    fn parses_non_hyphen_delimited_args() {
        let args = stringify(vec!["-ahsd", "--dirsfirst", "--prune", "src"]);

        let result = Cmd::process_args(args);

        assert_eq!(
            result,
            stringify(vec![
                "-a",
                "-h",
                "-s",
                "-d",
                "--dirsfirst",
                "--prune",
                "src",
            ])
        )
    }
}
