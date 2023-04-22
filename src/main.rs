use crate::cli::flags::Cmd;
use crate::core::{colors::Colors, pattern::Pattern, tree::Tree};
use std::borrow::Cow;

pub mod cli;
pub mod core;

extern crate once_cell;

const HELP: &str = r"
  --help                    -- list all flags
  --version                 -- prints version of tree
  --noreport                -- silence total directory and file count
  --inodes                  -- include inode of resource
  --device                  -- include device id of resource
  --dirsfirst               -- print directories before files
  --prune                   -- remove empty directories from output
  --filelimit [#]           -- skips directories with a file count over this limit
  -D                        -- print last modified
  -a                        -- include hidden files
  -d                        -- include directories only
  -f                        -- print full path of resource
  -F                        -- print '/' to identify directories
  -i                        -- no indentation
  -l                        -- follow symlinks
  -P [wildcard pattern]     -- include files and directories that match pattern
  -I [wildcard pattern]     -- exclude files and directories that match pattern
  -p                        -- print protections on resource
  -s                        -- print resource size
  -h                        -- print human readable resource size
  -u                        -- print user name
  -g                        -- print group
  -q                        -- replace unprintable characters with '?'
  -N                        -- print unprintable characters as is
  -r                        -- reverse alphabetic sort
  -t                        -- last modified sort
  -n                        -- no colors
  -C                        -- use ls colors
  -L                        -- sets max-depth of tree traversal
  -o                        -- output file path
";

const ANSI_COLOR_RESET: &str = "\x1B[0m";

fn get_ansi_color_esc_seq(colors: &(String, String)) -> (String, &'static str) {
    let all_parts = [&*colors.0, ";", &*colors.1, "m"];

    let fg_bg = all_parts.iter().fold(String::new(), |mut acc, &val| {
        match val {
            "" => (),
            ";" if colors.1.is_empty() => (),
            "m" if acc.is_empty() => (),
            _ => acc.push_str(val),
        };
        acc
    });

    if fg_bg.is_empty() {
        (fg_bg, "")
    } else {
        (format!("\x1B[{fg_bg}"), ANSI_COLOR_RESET)
    }
}

const VERTICAL_PIPE: &str = "\u{2502}";
const L_RIGHT: &str = "\u{2514}";
const T_RIGHT: &str = "\u{251C}";
const _L_LEFT: &str = "\u{2510}";
const _ARROW: &str = "\u{25B8}";

const NAME_CONNECTOR: &str = "\u{2500}\u{2500}\u{2500}";

fn main() -> std::io::Result<()> {
    Cmd::from_cli(std::env::args().skip(1).collect::<Vec<_>>());

    let flags = Cmd::global();

    if flags.help {
        println!("{HELP}");
    } else if !flags.dir_path.as_ref().unwrap().is_dir() {
        println!("Path is not a directory - {:?}", flags.dir_path);
    } else {
        Colors::from_ls_colors(flags.colors);

        // let pattern = match (&flags.pattern_match, &flags.pattern_exclude) {
        //     (Some(match_pattern), None) => {
        //         Some(Pattern::parse(match_pattern.as_str(), true))
        //     }

        //     (None, Some(exclude_pattern)) => {
        //         Some(Pattern::parse(exclude_pattern.as_str(), false))
        //     }

        //     _ => None,
        // };

        let tree =
            Tree::new().with_opts(std::env::args().skip(1).collect::<Vec<_>>());

        let mut ret = String::new();
        let mut left: std::collections::HashSet<usize> =
            std::collections::HashSet::new();

        let mut file_count = 0;
        let mut dir_count = 0;

        for (remaining, entry) in tree {
            let depth = entry.get_depth();

            if *depth == 0 {
                let name = match entry.get_name() {
                    Some(n) => n.to_str().unwrap_or("Failed to get name"),
                    _ => "Failed to get name",
                };
                ret.push_str(name)
            } else {
                for level in 1..=*depth {
                    if level != *depth {
                        if left.contains(&level) {
                            ret.push_str(VERTICAL_PIPE);
                            ret.push_str("    ");
                        } else {
                            ret.push_str("     ");
                        }
                    } else {
                        let connector = if remaining > 1 {
                            left.insert(*depth);
                            T_RIGHT
                        } else {
                            left.remove(depth);
                            L_RIGHT
                        };

                        if !entry.is_dir() {
                            file_count += 1;
                        } else if entry.is_dir() {
                            dir_count += 1;
                        }

                        let name = match entry.get_name() {
                            Some(n) => {
                                n.to_str().unwrap_or("Failed to get name")
                            }
                            _ => "Failed to get name",
                        };

                        let color_tup =
                            match (entry.is_dir(), entry.is_symlink()) {
                                (true, false) => {
                                    Colors::get_color_tuple("directory")
                                }
                                (false, true) => {
                                    Colors::get_color_tuple("sym_link")
                                }
                                _ => Colors::get_color_tuple(""),
                            };

                        let (fg_bg, reset) = get_ansi_color_esc_seq(color_tup);

                        let additional_info = format!(
                            "{}{fg_bg}{}{reset}",
                            entry.get_additional_info(),
                            name
                        );

                        let (arrow_chars, linked_path) = if entry.is_symlink() {
                            (
                                " -> ",
                                std::fs::read_link(entry.path()).map_or(
                                    "failed to get linked path",
                                    |linked| {
                                        linked.as_os_str().to_str().unwrap()
                                    },
                                ),
                            )
                        } else {
                            ("", "")
                        };

                        for val in [
                            connector,
                            NAME_CONNECTOR,
                            " ",
                            additional_info.as_str(),
                            arrow_chars,
                            linked_path,
                        ] {
                            ret.push_str(val)
                        }
                    }
                }
            }

            ret.push_str("\n");
        }

        println!("{ret}");
        println!("Total directories: {dir_count} Total files: {file_count}");
        // let with_meta = Cmd::requires_metadata();

        // if let Some(tree) = DirTree::new(
        //     flags.dir_path.as_ref().unwrap(),
        //     0,
        //     with_meta,
        //     &pattern_parser,
        // ) {
        //     println!("{}", tree);

        //     if !flags.no_report {
        //         let (total_dir_count, total_file_count) = tree.get_total_contents_count();
        //         let report = format!(
        //             "\nTotal directories: {total_dir_count} Total files: {total_file_count}\n"
        //         );

        //         println!("{}", report)
        //     }
        // } else {
        //     println!("Uh-oh something went wrong creating the directory structure")
        // }
    }

    Ok(())
}
