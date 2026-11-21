//! Login items: the freedesktop autostart folders (~/.config/autostart, then /etc/xdg/autostart),
//! where apps like Discord, Steam and Nextcloud put themselves to start with the desktop.
//!
//! For OnlyShowIn/NotShowIn we are `VIDE-DE` and nothing else. Apps see `VIDE-DE:KDE` in
//! XDG_CURRENT_DESKTOP so they look right, but every piece of Plasma (plasmashell,
//! powerdevil, xembedsniproxy...) autostarts with OnlyShowIn=KDE, and must not start here.

use std::{collections::HashSet, path::PathBuf};

/// Things `State::run_startup` already starts its own way, or that we are: nm-applet would put a
/// second network icon next to our own.
const OURS: &[&str] = &["pam_kwallet_init", "polkit-kde-authentication-agent-1", "polkit-gnome-authentication-agent-1", "nm-applet"];

fn dirs() -> Vec<PathBuf> {
    let config_home = match std::env::var("XDG_CONFIG_HOME") {
        Ok(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"),
    };
    let mut out = vec![config_home.join("autostart")];
    let xdg = std::env::var("XDG_CONFIG_DIRS").unwrap_or_default();
    out.extend(xdg.split(':').filter(|s| !s.is_empty()).map(|d| PathBuf::from(d).join("autostart")));
    out.push(PathBuf::from("/etc/xdg/autostart"));
    let mut seen = HashSet::new();
    out.retain(|d| seen.insert(d.clone()));
    out
}

/// The command to run for one autostart file, or `None` if it shouldn't start here.
fn command(text: &str) -> Option<String> {
    let mut exec = None;
    let mut in_entry = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        let Some((k, v)) = line.split_once('=').filter(|_| in_entry) else { continue };
        let (k, v) = (k.trim(), v.trim());
        let list = || v.split(';').map(str::trim).filter(|s| !s.is_empty());
        match k {
            "Exec" => exec = Some(v.to_string()),
            "Hidden" if v == "true" => return None,
            "X-GNOME-Autostart-enabled" if v == "false" => return None,
            "OnlyShowIn" if !list().any(|d| d == "VIDE-DE") => return None,
            "NotShowIn" if list().any(|d| d == "VIDE-DE") => return None,
            "TryExec" if !on_path(v) => return None,
            _ => {}
        }
    }
    let exec = crate::apps::strip_field_codes(&exec?);
    let program = exec.split_whitespace().next()?;
    let name = program.rsplit('/').next().unwrap_or(program);
    (!OURS.contains(&name)).then_some(exec)
}

fn on_path(program: &str) -> bool {
    if program.contains('/') {
        return std::path::Path::new(program).exists();
    }
    std::env::var("PATH").unwrap_or_default().split(':').any(|d| std::path::Path::new(d).join(program).exists())
}

/// What to start at login. A file in an earlier folder hides one with the same name in a
/// later one, which is how a user turns off a system item (Hidden=true in their own copy).
pub fn commands() -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for dir in dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut files: Vec<_> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "desktop")).collect();
        files.sort();
        for path in files {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if !seen.insert(name.clone()) {
                continue;
            }
            if let Some(cmd) = std::fs::read_to_string(&path).ok().and_then(|t| command(&t)) {
                out.push((name, cmd));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::command;

    #[test]
    fn show_rules() {
        let e = |extra: &str| format!("[Desktop Entry]\nType=Application\nExec=discord --start-minimized %U\n{extra}");
        assert_eq!(command(&e("")).as_deref(), Some("discord --start-minimized"));
        assert_eq!(command(&e("OnlyShowIn=KDE;")), None);
        assert_eq!(command(&e("OnlyShowIn=KDE;VIDE-DE;")).as_deref(), Some("discord --start-minimized"));
        assert_eq!(command(&e("NotShowIn=VIDE-DE;")), None);
        assert_eq!(command(&e("NotShowIn=KDE;GNOME;")).as_deref(), Some("discord --start-minimized"));
        assert_eq!(command(&e("Hidden=true")), None);
        assert_eq!(command(&e("X-GNOME-Autostart-enabled=false")), None);
        assert_eq!(command(&e("TryExec=/no/such/program")), None);
        assert_eq!(command("[Desktop Entry]\nExec=/usr/lib/pam_kwallet_init\n"), None);
        // Keys from other groups (actions) don't count.
        assert_eq!(command(&e("[Desktop Action x]\nOnlyShowIn=KDE;")).as_deref(), Some("discord --start-minimized"));
    }
}
