//! Reads the window button colors of the user's GTK theme (Linux), so Dotnote's
//! title bar buttons look like the other GNOME apps.
//!
//! Themes such as adw-colors (and adw-gtk3, which copies them) draw the buttons as
//! colored rings and write them to `~/.config/gtk-4.0/gtk.css`. When that file has
//! a ring color for close, maximize and minimize, Dotnote draws the same rings;
//! otherwise it draws libadwaita's round grey buttons.

use std::fs;
use std::path::Path;

/// The ring color of each window button, as `0xRRGGBB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingColors {
    pub close: u32,
    pub maximize: u32,
    pub minimize: u32,
}

/// The ring colors of the user's GTK 4 theme, if it draws ring buttons.
/// Reads `<config>/gtk-4.0/gtk.css` and the files it `@import`s.
pub fn read_ring_colors() -> Option<RingColors> {
    read_ring_colors_from(&dirs::config_dir()?.join("gtk-4.0"))
}

/// Like [`read_ring_colors`], for the GTK config folder `dir`.
pub fn read_ring_colors_from(dir: &Path) -> Option<RingColors> {
    let css = fs::read_to_string(dir.join("gtk.css")).ok()?;
    // adw-gtk3's sync tool puts the buttons in a file of their own: follow the imports.
    let imported: String = css
        .split("@import url(\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .filter_map(|name| fs::read_to_string(dir.join(name)).ok())
        .collect();
    parse_ring_colors(&(css + &imported))
}

/// Finds each button's ring color in GTK theme CSS: the `box-shadow: inset 0 0 0 Npx #rrggbb`
/// of `windowcontrols > button.close > image` (GTK 4) or `button.titlebutton.close` (GTK 3),
/// not of their `:hover` or `:backdrop` states. `None` unless all three have one.
pub fn parse_ring_colors(css: &str) -> Option<RingColors> {
    // One space between words, so the selectors match however the file is indented.
    let css = css.split_whitespace().collect::<Vec<_>>().join(" ");
    let color = |name: &str| {
        let selectors = [
            format!("button.{name} > image {{"),
            format!("button.titlebutton.{name} {{"),
        ];
        selectors
            .iter()
            .find_map(|selector| ring_in_block(css.split(selector.as_str()).nth(1)?))
    };
    Some(RingColors {
        close: color("close")?,
        maximize: color("maximize")?,
        minimize: color("minimize")?,
    })
}

/// The `#rrggbb` (or `#rgb`) of the first `box-shadow: inset 0 0 0 Npx` in a CSS block.
fn ring_in_block(after_selector: &str) -> Option<u32> {
    let block = after_selector.split('}').next()?;
    let shadow = block.split("box-shadow:").nth(1)?.trim_start();
    let hex = shadow
        .strip_prefix("inset 0 0 0 ")?
        .split_once("px ")?
        .1
        .strip_prefix('#')?;
    let hex: String = hex.chars().take_while(char::is_ascii_hexdigit).collect();
    let hex = match hex.len() {
        3 => hex.chars().flat_map(|c| [c, c]).collect(),
        6 => hex,
        _ => return None,
    };
    u32::from_str_radix(&hex, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_from_adw_colors() {
        let css =
            "windowcontrols > button.close > image:hover { box-shadow: inset 0 0 0 8px #ffffff; }
            windowcontrols > button.close > image {
              box-shadow: inset 0 0 0 6px #e86150;
            }
            windowcontrols > button.maximize > image { box-shadow: inset 0 0 0 6px #75c2b3; }
            button.titlebutton:backdrop.minimize { box-shadow: inset 0 0 0 6px #000; }
            button.titlebutton.minimize { box-shadow: inset 0 0 0 6px #d7c781; }";
        let rings = parse_ring_colors(css).unwrap();
        assert_eq!(rings.close, 0xe86150);
        assert_eq!(rings.maximize, 0x75c2b3);
        assert_eq!(rings.minimize, 0xd7c781);
    }

    #[test]
    fn short_hex_is_expanded() {
        let css = "button.close > image { box-shadow: inset 0 0 0 6px #f00; }
            button.maximize > image { box-shadow: inset 0 0 0 6px #0f0; }
            button.minimize > image { box-shadow: inset 0 0 0 6px #00f; }";
        let rings = parse_ring_colors(css).unwrap();
        assert_eq!(
            (rings.close, rings.maximize, rings.minimize),
            (0xff0000, 0x00ff00, 0x0000ff)
        );
    }

    #[test]
    fn no_rings_in_a_plain_theme() {
        assert_eq!(
            parse_ring_colors("@define-color accent_bg_color #3584e4;"),
            None
        );
        // Only close has a ring: not a ring theme.
        assert_eq!(
            parse_ring_colors(
                "windowcontrols > button.close > image { box-shadow: inset 0 0 0 6px #e86150; }"
            ),
            None
        );
    }
}
