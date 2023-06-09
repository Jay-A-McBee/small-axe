use std::ffi::OsStr;

// Box drawing unicode chars
const HORIZONTAL_PIPE: &str = "\u{2500}";
const VERTICAL_PIPE: &str = "\u{2502}";
const L_RIGHT: &str = "\u{2514}";
const T_RIGHT: &str = "\u{251C}";
const _L_LEFT: &str = "\u{2510}";
const _ARROW: &str = "\u{25B8}";

const ANSI_COLOR_RESET: &str = "\x1B[0m";

#[derive(Debug, Default)]
pub struct Ledger(pub &'static str);

impl Ledger {
    pub fn extend_indent_list(
        indent_levels: &[Option<()>],
        remaining: bool,
        level: u32,
    ) -> Vec<Option<()>> {
        let mut list = if !remaining {
            indent_levels
                .iter()
                .enumerate()
                .map(|(idx, curr)| if idx == level as usize { None } else { *curr })
                .collect::<Vec<Option<()>>>()
        } else {
            indent_levels.to_owned()
        };

        list.push(Some(()));
        list
    }

    pub fn add_connectors(
        &self,
        f: &mut std::fmt::Formatter,
        indent_levels: &[Option<()>],
        remaining: bool,
    ) -> std::fmt::Result {
        let final_idx = indent_levels.len() - 1;

        let connectors =
            indent_levels
                .iter()
                .enumerate()
                .fold(String::new(), |mut acc, (idx, &space)| {
                    let pipe = match (idx == final_idx, space) {
                        (true, _) if remaining => T_RIGHT,
                        (true, _) => L_RIGHT,
                        (false, Some(_)) => VERTICAL_PIPE,
                        _ => " ",
                    };

                    let offset = if idx > 0 { self.0 } else { "" };

                    acc.push_str(format!("{offset}{pipe}").as_str());
                    acc
                });

        write!(f, "{}", connectors)
    }

    fn get_ansi_color_esc_seq(colors: &(String, String)) -> (String, &str) {
        let all_parts = [colors.0.as_str(), ";", colors.1.as_str(), "m"];

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

    pub fn add_name_entry(
        &self,
        f: &mut std::fmt::Formatter,
        name: &OsStr,
        additional_info: &str,
        colors: &(String, String),
        lossy: bool,
    ) -> std::fmt::Result {
        let (fg_bg, reset) = Self::get_ansi_color_esc_seq(colors);

        let connectors = if self.0.is_empty() {
            additional_info.to_string()
        } else {
            format!("{HORIZONTAL_PIPE}{HORIZONTAL_PIPE}{HORIZONTAL_PIPE}{additional_info}")
        };

        let line = if lossy {
            let converted = name.to_string_lossy().to_string();
            format!("{connectors} {fg_bg}{converted}{reset}\n")
        } else {
            let converted = name.to_str().map_or_else(
                || name.to_string_lossy().to_string(),
                std::borrow::ToOwned::to_owned,
            );

            format!("{connectors} {fg_bg}{converted}{reset}\n")
        };

        write!(f, "{}", line)
    }
}
