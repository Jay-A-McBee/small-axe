use std::ffi::OsStr;

use super::colors::Colors;
use super::tree::Tree;

use crate::cli::Cmd;

const VERTICAL_PIPE: &str = "\u{2502}";
const L_RIGHT: &str = "\u{2514}";
const T_RIGHT: &str = "\u{251C}";
const NAME_CONNECTOR: &str = "\u{2500}\u{2500}\u{2500}";
const DEFAULT_INDENT: &str = "    ";
pub struct Display {}

impl Display {
    pub fn print(tree: Tree, cmds: &Cmd) {
        let mut ret = String::new();

        let mut has_remaining: std::collections::HashSet<usize> =
            std::collections::HashSet::new();

        let mut file_count = 0;
        let mut dir_count = 0;

        for (remaining, entry) in tree {
            let name = if cmds.flags.full_path {
                String::from(entry.full_path().as_os_str().to_str().unwrap())
            } else {
                entry.get_name().map_or_else(
                    || String::from("Failed to get name"),
                    |name| {
                        String::from(
                            name.to_str().unwrap_or("Failed to get name"),
                        )
                    },
                )
            };

            let depth = entry.get_depth();

            let (fg_bg, reset) =
                Colors::get_color_esc_seq(entry.get_file_type());

            let (recursion_detected, arrow_chars, linked_path) =
                match entry.linked_path() {
                    Some(path) if entry.is_recursive_link => (
                        "[Recursion detected]",
                        " -> ",
                        path.as_os_str()
                            .to_str()
                            .unwrap_or("failed to get linked path"),
                    ),
                    Some(path) => (
                        "",
                        " -> ",
                        path.as_os_str()
                            .to_str()
                            .unwrap_or("failed to get linked path"),
                    ),
                    None => ("", "", ""),
                };

            if *depth != 0 {
                if !entry.is_dir() && !entry.is_symlink() {
                    file_count += 1;
                } else if entry.is_dir() {
                    dir_count += 1;
                }
            }

            if *depth == 0 {
                for val in [fg_bg, name.as_str(), reset, "\n"] {
                    ret.push_str(val);
                }
            } else if cmds.flags.no_indent {
                for val in [
                    fg_bg,
                    name.as_str(),
                    reset,
                    recursion_detected,
                    arrow_chars,
                    linked_path,
                    "\n",
                ] {
                    ret.push_str(val);
                }
            } else {
                for level in 1..*depth {
                    let outer_connector = if has_remaining.contains(&level) {
                        VERTICAL_PIPE
                    } else {
                        " "
                    };

                    ret.push_str(outer_connector);
                    ret.push_str(DEFAULT_INDENT);
                }

                let connector = if remaining > 1 {
                    has_remaining.insert(*depth);
                    T_RIGHT
                } else {
                    has_remaining.remove(depth);
                    L_RIGHT
                };

                for val in [
                    connector,
                    NAME_CONNECTOR,
                    " ",
                    entry.get_additional_info(&cmds).as_str(),
                    fg_bg,
                    name.as_str(),
                    reset,
                    recursion_detected,
                    arrow_chars,
                    linked_path,
                    "\n",
                ] {
                    ret.push_str(val);
                }
            }
        }

        println!("{ret}");

        if !cmds.flags.no_report {
            println!(
                "Total directories: {dir_count} Total files: {file_count}"
            );
        }
    }
}
