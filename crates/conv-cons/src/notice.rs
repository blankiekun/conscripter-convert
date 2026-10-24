//! The notice shown once, on the very first launch, before the title: what
//! this port is, where to report problems, and a word for buying the games.

use std::fmt::Write;

/// A global, 1 once the notice has been dismissed: a name ([`DEFINE`]),
/// kept in gloval.sav by name.
pub(crate) const SEEN: &str = "%conv_notice_seen";
/// Its declaration, in the define section after `globalon`.
pub(crate) const DEFINE: &str = "defvar conv_notice_seen,global";
/// A save slot written and deleted at once: a save writes gloval.sav (and
/// syncs it on the web build, where the page may close without a quit).
const FLUSH_SLOT: u32 = 892;

/// The lines, top to bottom (`None` a gap), the user's wording: the long
/// sentences broken at a comma or space to fit a 4:3 screen. The last,
/// dimmed, is the prompt.
const LINES: &[Option<&str>] = &[
    Some("本非公式移植版は、原作を可能な限り忠実に再現できるよう制作されています。"),
    Some("不具合などを発見された場合は、以下のリンクよりご報告いただけますと幸いです。"),
    Some(ISSUES),
    Some("本作を気に入っていただけましたら、"),
    Some("ぜひ原作（製品版）のご購入をお願いいたします。"),
    None,
    Some("This port aims to be as faithful to the original as possible."),
    Some("Found a bug or issue? Please report it at:"),
    Some(ISSUES),
    Some("If you enjoy this game, please consider purchasing the official"),
    Some("release to support the original creators."),
    None,
    Some("クリックで進む / Click to continue"),
];
const ISSUES: &str = "https://github.com/blankiekun/conscripter-convert/issues";

/// The notice for a `width` × `height` screen: white text centred on black,
/// faded in, dismissed by a click or key, never shown again.
/// An `en` build (a font without Japanese glyphs) gets the English lines only.
pub(crate) fn emit(out: &mut String, width: u32, height: u32, language: &str) {
    let lines: Vec<Option<&str>> = if language == "en" {
        let mut v = LINES[6..11].to_vec();
        v.extend([None, Some("Click to continue")]);
        v
    } else {
        LINES.to_vec()
    };
    let lines = &lines[..];
    let size = (height / 30).max(12) as i32;
    let pitch = size * 3 / 2;
    let top = (height as i32 - lines.len() as i32 * pitch) / 2;
    let sprites = crate::MENU_SPRITES;
    writeln!(out, "if {SEEN} == 1 goto *conv_notice_done").unwrap();
    writeln!(out, "bg black,1").unwrap();
    for (i, line) in lines.iter().enumerate() {
        let Some(text) = line else { continue };
        // Half-width characters are half as wide (as `len` counts them).
        let units: i32 = text.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum();
        let x = (width as i32 - units * size / 2) / 2;
        let color = if i + 1 == lines.len() { "#A0A0A0" } else { "#FFFFFF" };
        writeln!(out, "strsp {},\"{text}\",{x},{},64,1,{size},{size},0,0,0,1,\"{color}\"", sprites.start + i as u32, top + i as i32 * pitch).unwrap();
    }
    writeln!(out, "print 10,400").unwrap();
    writeln!(out, "click").unwrap();
    writeln!(out, "mov {SEEN},1:savegame {FLUSH_SLOT}:delsavegame {FLUSH_SLOT}").unwrap();
    let clear: Vec<String> = (0..lines.len() as u32).map(|i| format!("csp {}", sprites.start + i)).collect();
    writeln!(out, "{}", clear.join(":")).unwrap();
    writeln!(out, "print 10,400").unwrap();
    writeln!(out, "*conv_notice_done").unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn notice_fits_and_is_shown_once() {
        let mut out = String::new();
        super::emit(&mut out, 640, 480, "");
        assert!(out.starts_with("if %conv_notice_seen == 1 goto *conv_notice_done\n"), "{out}");
        assert!(out.contains("click\nmov %conv_notice_seen,1:"), "{out}");
        // Every line inside a 640-wide screen.
        for l in out.lines().filter(|l| l.starts_with("strsp")) {
            let x: i32 = l.split("\",").nth(1).unwrap().split(',').next().unwrap().parse().unwrap();
            assert!(x >= 8, "{l}");
        }
        let mut en = String::new();
        super::emit(&mut en, 640, 480, "en");
        assert!(en.contains("Click to continue") && !en.contains("クリック"), "{en}");
        assert!(super::LINES.len() as u32 <= (crate::MENU_SPRITES.end - crate::MENU_SPRITES.start));
    }
}
