use crate::cli::flags::Cmd;
use crate::core::tree::Tree;
use crate::output::{colors::ColorParser, pattern::PatternParser};

pub mod cli;
pub mod core;
pub mod output;

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
        let tree = Tree::new()
            .with_opts(std::env::args().skip(1).collect::<Vec<_>>());

        let mut ret = String::new();
        let mut left: std::collections::HashSet<usize> =
            std::collections::HashSet::new();

        for (remaining, entry) in tree {
            let depth = entry.get_depth();

            if *depth == 0 {
                ret.push_str(
                    entry.path().file_name().unwrap().to_str().unwrap(),
                )
            } else {
                let mut branch = String::new();

                for level in 1..=*depth {
                    if level != *depth {
                        if left.contains(&level) {
                            branch.push_str(VERTICAL_PIPE);
                            branch.push_str("    ");
                        } else {
                            branch.push_str("     ");
                        }
                    } else {
                        let connector = if remaining > 1 {
                            left.insert(*depth);
                            T_RIGHT
                        } else {
                            left.remove(depth);
                            L_RIGHT
                        };

                        let name = entry
                            .path()
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap();

                        let additional_info = format!(
                            "[{}{}{}] {name}",
                            "some", "other", "stuff"
                        );

                        for val in [
                            connector,
                            NAME_CONNECTOR,
                            " ",
                            additional_info.as_str(),
                        ] {
                            branch.push_str(val)
                        }
                    }
                }
                ret.push_str(branch.as_str());
            }

            ret.push_str("\n");
        }

        println!("{ret}");
        // let with_meta = Cmd::requires_metadata();

        // ColorParser::from_ls_colors(flags.colors);

        // let pattern_parser = match (&flags.pattern_match, &flags.pattern_exclude) {
        //     (Some(match_pattern), None) => {
        //         Some(PatternParser::parse_pattern(match_pattern.as_str(), true))
        //     }

        //     (None, Some(exclude_pattern)) => Some(PatternParser::parse_pattern(
        //         exclude_pattern.as_str(),
        //         false,
        //     )),

        //     _ => None,
        // };

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
