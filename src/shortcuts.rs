//! The user's own keyboard shortcuts: ~/.config/vide-de/shortcuts.conf, one per line,
//!
//! ```text
//! # keys = command
//! super+shift+t = konsole
//! ctrl+alt+f = firefox --new-window
//! ```
//!
//! They win over our built-in binds, so any of those can be repointed. Edits apply within a
//! second or so, no sign-out needed.

use std::{path::PathBuf, time::SystemTime};

use smithay::input::keyboard::{xkb, Keysym, ModifiersState};

pub struct Shortcut {
    ctrl: bool,
    alt: bool,
    shift: bool,
    logo: bool,
    key: Keysym,
    pub command: String,
}

#[derive(Default)]
pub struct Shortcuts {
    pub list: Vec<Shortcut>,
    mtime: Option<SystemTime>,
}

fn path() -> PathBuf {
    vide_config::Config::path().with_file_name("shortcuts.conf")
}

/// One line, or `None` (with the reason logged) if it doesn't make sense.
fn parse_line(line: &str) -> Option<Shortcut> {
    let (keys, command) = line.split_once('=')?;
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    let mut s = Shortcut { ctrl: false, alt: false, shift: false, logo: false, key: Keysym::NoSymbol, command: command.into() };
    for part in keys.split('+').map(|p| p.trim().to_lowercase()) {
        match part.as_str() {
            "ctrl" | "control" => s.ctrl = true,
            "alt" => s.alt = true,
            "shift" => s.shift = true,
            "super" | "win" | "logo" | "meta" => s.logo = true,
            name => {
                let sym = xkb::keysym_from_name(name, xkb::KEYSYM_CASE_INSENSITIVE);
                if sym.raw() == 0 {
                    tracing::warn!("shortcuts.conf: unknown key {name:?}");
                    return None;
                }
                s.key = sym;
            }
        }
    }
    (s.key != Keysym::NoSymbol).then_some(s)
}

impl Shortcuts {
    /// Re-read the file if it changed (called once a second, and at startup).
    pub fn reload_if_changed(&mut self) {
        let mtime = std::fs::metadata(path()).and_then(|m| m.modified()).ok();
        if mtime == self.mtime {
            return;
        }
        self.mtime = mtime;
        let text = std::fs::read_to_string(path()).unwrap_or_default();
        self.list = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(parse_line)
            .collect();
        tracing::info!("loaded {} custom shortcuts", self.list.len());
    }

    /// The command bound to these modifiers and key, if any. `syms` are the key's symbols with
    /// and without modifiers applied, so `shift+1` and `shift+exclam` both work. Names are read
    /// case-insensitively, which gives the lowercase sym a letter key sends.
    pub fn find(&self, mods: &ModifiersState, syms: &[Keysym]) -> Option<&str> {
        self.list
            .iter()
            .find(|s| {
                s.ctrl == mods.ctrl
                    && s.alt == mods.alt
                    && s.shift == mods.shift
                    && s.logo == mods.logo
                    && syms.contains(&s.key)
            })
            .map(|s| s.command.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lines() {
        let s = parse_line("Super+Shift+T = konsole --new-tab").unwrap();
        assert!(s.logo && s.shift && !s.ctrl && !s.alt);
        assert_eq!(s.key, Keysym::t);
        assert_eq!(s.command, "konsole --new-tab");
        assert!(parse_line("ctrl+alt+nosuchkey = x").is_none());
        assert!(parse_line("ctrl+alt = x").is_none());
        assert!(parse_line("ctrl+f =").is_none());
        assert_eq!(parse_line("ctrl+F5 = a=b").unwrap().command, "a=b");
    }
}
