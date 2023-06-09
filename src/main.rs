use crate::cli::{Cmd, TreeIteratorFlags};
use crate::core::colors::Colors;
use crate::core::display::Display;
use crate::core::pattern::Pattern;
use crate::core::tree::Tree;

pub mod cli;
pub mod core;

extern crate once_cell;

const HELP: &str = r"
  usage: tree [-adfipshugqrtnoCFPIN] --[help version noreport inodes device dirsfirst prune filelimit] [path]

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

fn main() {
    let mut cmd = Cmd::from(std::env::args());

    if cmd.flags.help {
        println!("{HELP}");
    } else if !cmd.flags.dir_path.as_ref().unwrap().is_dir() {
        println!("Path is not a directory - {:?}", cmd.flags.dir_path);
    } else {
        Colors::from_ls_colors(cmd.flags.colors);

        let pattern =
            match (&cmd.flags.pattern_match, &cmd.flags.pattern_exclude) {
                (Some(match_pattern), None) => {
                    Some(Pattern::parse(match_pattern.as_str(), true))
                }

                (None, Some(exclude_pattern)) => {
                    Some(Pattern::parse(exclude_pattern.as_str(), false))
                }

                _ => None,
            };

        Display::print(
            Tree::new(
                &mut TreeIteratorFlags {
                    root: cmd.flags.dir_path.take(),
                    max_depth: cmd.flags.max_depth.take(),
                    visit_all: cmd.flags.all,
                    dirs_only: cmd.flags.dirs,
                    dirs_first: cmd.flags.dirs_first,
                    last_mod_sort: cmd.flags.last_modified_sort,
                    rev_alpha_sort: cmd.flags.reverse_alpha_sort,
                    follow_symlinks: cmd.flags.follow_symlinks,
                },
                pattern,
            ),
            &cmd,
        );
    }
}
