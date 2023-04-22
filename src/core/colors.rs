use std::collections::{HashMap, HashSet};

use once_cell::sync::OnceCell;

enum ColorFormats {
    LsColors(String),
    LsColorsDelimited(String),
    Undefined,
}

static DEFAULT: (String, String) = (String::new(), String::new());
static COLORS: OnceCell<Option<HashMap<&'static str, (String, String)>>> =
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
    fn map_chars_to_ansi_color_code(color_var: &str) -> Vec<(String, String)> {
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
                (fg_color.to_string(), bg_color.to_string())
            })
            .collect()
    }

    fn create_color_map(
        color_fmt: ColorFormats,
    ) -> Option<HashMap<&'static str, (String, String)>> {
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
                                            let parsed = cfg_val
                                                .parse::<u8>()
                                                .expect("failed to parse value"); // we only care about ansi color codes
                                            (30..=47).contains(&parsed) // standard fg colors
                                                || (90..=107).contains(&parsed) // standard bg colors
                                        })
                                        .collect::<Vec<&str>>();

                                    if resource_colors.len() == 1 {
                                        Some((
                                            *resource_map
                                                .get(resource)
                                                .expect("Failed to get resource colors"),
                                            (
                                                (*resource_colors.first().unwrap()).to_string(),
                                                String::new(),
                                            ),
                                        ))
                                    } else {
                                        Some((
                                            *resource_map
                                                .get(resource)
                                                .expect("Failed to get resource color"),
                                            (
                                                (*resource_colors
                                                    .first()
                                                    .expect("Failed to get first colors"))
                                                .to_string(),
                                                (*resource_colors
                                                    .get(1)
                                                    .expect("Failed to get last colors"))
                                                .to_string(),
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
                        .collect::<HashMap<&str, (String, String)>>(),
                )
            }
            ColorFormats::Undefined => None,
        };

        colors
    }

    pub fn get_color_tuple(resource_type: &str) -> &(String, String) {
        if let Some(Some(colors)) = COLORS.get() {
            return colors.get(resource_type).unwrap_or(&DEFAULT);
        }

        &DEFAULT
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
                ("directory", (String::from("32"), String::from(""))),
                ("sym_link", (String::from("35"), String::from(""))),
                ("socket", (String::from("32"), String::from(""))),
                ("pipe", (String::from("33"), String::from(""))),
                ("executable", (String::from("31"), String::from(""))),
                ("special_block", (String::from("34"), String::from("46"))),
                ("special_char", (String::from("34"), String::from("43"))),
                ("exe_set_uid", (String::from("30"), String::from("41"))),
                ("exe_set_gid", (String::from("30"), String::from("46"))),
                ("dwo_sticky", (String::from("30"), String::from("42"))),
                ("dwo_non_sticky", (String::from("30"), String::from("43")))
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
                ("directory", (String::from("31"), String::from(""))),
                ("sym_link", (String::from("32"), String::from(""))),
                ("socket", (String::from("32"), String::from(""))),
                ("pipe", (String::from("101"), String::from(""))),
                ("executable", (String::from("35"), String::from(""))),
                ("special_block", (String::from("105"), String::from(""))),
                ("special_char", (String::from("40"), String::from(""))),
                ("exe_set_uid", (String::from("35"), String::from(""))),
                ("exe_set_gid", (String::from("35"), String::from(""))),
                ("dwo_sticky", (String::from("35"), String::from("101"))),
                ("dwo_non_sticky", (String::from("35"), String::from("")))
            ]))
        )
    }

    #[test]
    fn ls_colors_undefined() {
        let result = Colors::create_color_map(ColorFormats::Undefined);

        assert_eq!(result, None)
    }
}
