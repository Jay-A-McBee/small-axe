use std::collections::{HashMap, HashSet};

use once_cell::sync::OnceCell;

enum ColorFormats {
    LsColors(String),
    LsColorsDelimited(String),
    Undefined,
}

const ANSI_COLOR_RESET: &str = "\x1B[0m";
static DEFAULT_COLOR: (String, &'static str) = (String::new(), "");

static COLORS: OnceCell<Option<HashMap<&'static str, (String, &'static str)>>> =
    OnceCell::new();

#[derive(Debug, Default)]
pub struct Colors {}

impl Colors {
    pub fn from_ls_colors(with_colors: bool) {
        if with_colors {
            let color_fmt = Self::get_color_var();
            let colors = Self::create_color_map(color_fmt);

            COLORS
                .set(colors)
                .expect("Failed to set global colors value")
        }
    }

    fn get_color_var() -> ColorFormats {
        match (std::env::var("LSCOLORS"), std::env::var("LS_COLORS")) {
            (Ok(color_fmt), _) => ColorFormats::LsColors(color_fmt),
            (_, Ok(color_fmt)) => ColorFormats::LsColorsDelimited(color_fmt),
            _ => ColorFormats::Undefined,
        }
    }

    // returns (fg, bg) color code tuples - ex. ("31", "103")
    fn map_chars_to_ansi_color_code(
        color_var: &str,
    ) -> Vec<(String, &'static str)> {
        // Map of letter to tuple of ANSI color codes - (foreground, background)
        let mapped_ls_colors = HashMap::from([
            ('a', ("30", "40")),   // black
            ('b', ("31", "41")),   // red
            ('c', ("32", "42")),   // green
            ('d', ("33", "43")),   // yellow
            ('e', ("34", "44")),   // blue
            ('f', ("35", "45")),   // magenta
            ('g', ("36", "46")),   // cyan
            ('h', ("37", "47")),   // white
            ('A', ("90", "100")),  // bright black
            ('B', ("91", "101")),  // bright red
            ('C', ("92", "102)")), // bright green
            ('D', ("93", "103")),  // bright yellow
            ('E', ("94", "104)")), // bright blue
            ('F', ("95", "105")),  // bright magenta
            ('G', ("96", "106")),  // bright cyan
            ('H', ("97", "107")),  // bright white
            ('x', ("", "")),
        ]);

        color_var
            .chars()
            .collect::<Vec<char>>()
            .chunks(2)
            .map(|chunk| {
                let (fg_color, _) = mapped_ls_colors.get(&chunk[0]).unwrap();
                let (_, bg_color) = mapped_ls_colors.get(&chunk[1]).unwrap();

                Colors::map_color_to_esc_seq(fg_color, bg_color)
            })
            .collect()
    }

    fn create_color_map(
        color_fmt: ColorFormats,
    ) -> Option<HashMap<&'static str, (String, &'static str)>> {
        let ls_colors_indexed_values = [
            "directory",
            "sym_link",
            "socket",
            "pipe",
            "executable",
            "special_block",
            "special_char",
            "exe_set_uid",
            "exe_set_gid",
            "dwo_sticky",
            "dwo_non_sticky",
        ];

        let ls_color_values = [
            "di", "ln", "so", "pi", "ex", "bd", "cd", "su", "sg", "tw", "ow",
        ];

        let colors = match color_fmt {
            ColorFormats::LsColors(color_var) => {
                let mapped_color_tuples =
                    Self::map_chars_to_ansi_color_code(&color_var);
                Some(
                    ls_colors_indexed_values
                        .into_iter()
                        .zip(mapped_color_tuples.into_iter())
                        .collect(),
                )
            }
            ColorFormats::LsColorsDelimited(color_var) => {
                let resource_set = HashSet::from(ls_color_values);

                let resource_map = ls_color_values
                    .into_iter()
                    .zip(ls_colors_indexed_values.into_iter())
                    .collect::<HashMap<&str, &str>>();

                Some(
                    color_var
                        .split(':')
                        .filter_map(|color| {
                            if let [resource, color_config] =
                                color.split('=').collect::<Vec<_>>()[0..=1]
                            {
                                if resource_set.contains(resource) {
                                    let resource_colors = color_config
                                        .split(';')
                                        .filter(|cfg_val| {
                                            let parsed =
                                                cfg_val.parse::<u8>().expect(
                                                    "failed to parse value",
                                                ); // we only care about ansi color codes
                                            (30..=47).contains(&parsed) // standard fg colors
                                                || (90..=107).contains(&parsed) // standard bg colors
                                        })
                                        .collect::<Vec<&str>>();

                                    if resource_colors.len() == 1 {
                                        let fg =
                                            *resource_colors.first().unwrap();
                                        Some((
                                            *resource_map.get(resource).expect(
                                                "Failed to get resource colors",
                                            ),
                                            Colors::map_color_to_esc_seq(
                                                fg, "",
                                            ),
                                        ))
                                    } else {
                                        let fg =
                                            *resource_colors.first().expect(
                                                "Failed to get first colors",
                                            );

                                        let bg =
                                            *resource_colors.get(1).expect(
                                                "Failed to get last colors",
                                            );

                                        Some((
                                            *resource_map.get(resource).expect(
                                                "Failed to get resource color",
                                            ),
                                            Colors::map_color_to_esc_seq(
                                                fg, bg,
                                            ),
                                        ))
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect::<HashMap<&str, (String, &'static str)>>(),
                )
            }
            ColorFormats::Undefined => None,
        };

        colors
    }

    pub fn map_color_to_esc_seq(fg: &str, bg: &str) -> (String, &'static str) {
        let all_parts = [&*fg, ";", &*bg, "m"];

        let fg_bg = all_parts.iter().fold(String::new(), |mut acc, &val| {
            match val {
                "" => (),
                ";" if fg.is_empty() => (),
                "m" if acc.is_empty() => (),
                _ => acc.push_str(val),
            };
            acc
        });

        if fg_bg.is_empty() {
            return (fg_bg, "");
        } else {
            return (format!("\x1B[{fg_bg}"), ANSI_COLOR_RESET);
        }
    }

    pub fn get_color_esc_seq(entity: &str) -> &(String, &'static str) {
        if let Some(Some(color_map)) = COLORS.get() {
            color_map.get(entity).unwrap_or(&DEFAULT_COLOR)
        } else {
            &DEFAULT_COLOR
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ls_colors() {
        let mock_lscolors =
            ColorFormats::LsColors(String::from("cxfxcxdxbxegedabagacad"));

        let result = Colors::create_color_map(mock_lscolors);

        assert_eq!(
            result,
            Some(HashMap::from([
                ("directory", (String::from("\x1B[32;m"), ANSI_COLOR_RESET)),
                ("sym_link", (String::from("\x1B[35;m"), ANSI_COLOR_RESET)),
                ("socket", (String::from("\x1B[32;m"), ANSI_COLOR_RESET)),
                ("pipe", (String::from("\x1B[33;m"), ANSI_COLOR_RESET)),
                ("executable", (String::from("\x1B[31;m"), ANSI_COLOR_RESET)),
                (
                    "special_block",
                    (String::from("\x1B[34;46m"), ANSI_COLOR_RESET)
                ),
                (
                    "special_char",
                    (String::from("\x1B[34;43m"), ANSI_COLOR_RESET)
                ),
                (
                    "exe_set_uid",
                    (String::from("\x1B[30;41m"), ANSI_COLOR_RESET)
                ),
                (
                    "exe_set_gid",
                    (String::from("\x1B[30;46m"), ANSI_COLOR_RESET)
                ),
                (
                    "dwo_sticky",
                    (String::from("\x1B[30;42m"), ANSI_COLOR_RESET)
                ),
                (
                    "dwo_non_sticky",
                    (String::from("\x1B[30;43m"), ANSI_COLOR_RESET)
                )
            ]))
        )
    }

    #[test]
    fn ls_colors_delimited() {
        let mock_ls_colors = ColorFormats::LsColorsDelimited(String::from("di=01;31:ln=01;32:so=01;32:pi=01;101:ex=01;35:bd=01;105:cd=40:su=01;35:sg=01;35:ow=01;35:tw=01;35;101"));

        let result = Colors::create_color_map(mock_ls_colors);

        assert_eq!(
            result,
            Some(HashMap::from([
                ("directory", (String::from("\x1B[31;m"), ANSI_COLOR_RESET)),
                ("sym_link", (String::from("\x1B[32;m"), ANSI_COLOR_RESET)),
                ("socket", (String::from("\x1B[32;m"), ANSI_COLOR_RESET)),
                ("pipe", (String::from("\x1B[101;m"), ANSI_COLOR_RESET)),
                ("executable", (String::from("\x1B[35;m"), ANSI_COLOR_RESET)),
                (
                    "special_block",
                    (String::from("\x1B[105;m"), ANSI_COLOR_RESET)
                ),
                (
                    "special_char",
                    (String::from("\x1B[40;m"), ANSI_COLOR_RESET)
                ),
                ("exe_set_uid", (String::from("\x1B[35;m"), ANSI_COLOR_RESET)),
                ("exe_set_gid", (String::from("\x1B[35;m"), ANSI_COLOR_RESET)),
                (
                    "dwo_sticky",
                    (String::from("\x1B[35;101m"), ANSI_COLOR_RESET)
                ),
                (
                    "dwo_non_sticky",
                    (String::from("\x1B[35;m"), ANSI_COLOR_RESET)
                )
            ]))
        )
    }

    #[test]
    fn ls_colors_undefined() {
        let result = Colors::create_color_map(ColorFormats::Undefined);

        assert_eq!(result, None)
    }
}
