//! Icon themes: which are installed, and finding an icon in the one chosen in Settings
//! (`Config::icon_theme`). Every VIDE-DE program asks here first and falls back to its own
//! lookup (Breeze and friends) for anything the theme doesn't have.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

/// Where icon themes live, most personal first.
fn bases() -> Vec<PathBuf> {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let mut out = vec![crate::data_home().join("icons"), home.join(".icons")];
    let dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
    out.extend(dirs.split(':').filter(|d| !d.is_empty()).map(|d| PathBuf::from(d).join("icons")));
    out.push(PathBuf::from("/usr/share/icons"));
    let mut seen = Vec::new();
    out.retain(|p| {
        let new = !seen.contains(p);
        seen.push(p.clone());
        new
    });
    out
}

/// What a theme's index.theme says about it.
#[derive(Clone, Default)]
struct Theme {
    name: String,
    hidden: bool,
    inherits: Vec<String>,
    /// Each directory with its nominal size and whether it's scalable.
    dirs: Vec<(String, u32, bool)>,
}

fn parse(text: &str) -> Theme {
    let mut t = Theme::default();
    let mut section = String::new();
    let mut sizes: HashMap<String, (u32, bool)> = HashMap::new();
    let mut order = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line.trim_matches(['[', ']']).to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        if section == "Icon Theme" {
            match k {
                "Name" => t.name = v.to_string(),
                "Hidden" => t.hidden = v == "true",
                "Inherits" => t.inherits = v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                "Directories" => order = v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                _ => {}
            }
        } else {
            let e = sizes.entry(section.clone()).or_insert((0, false));
            match k {
                "Size" => e.0 = v.parse().unwrap_or(0),
                "Type" => e.1 = v == "Scalable",
                _ => {}
            }
        }
    }
    t.dirs = order.into_iter().map(|d| {
        let (size, scalable) = sizes.get(&d).copied().unwrap_or((0, false));
        (d, size, scalable)
    }).collect();
    t
}

fn theme(id: &str) -> Option<Theme> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Theme>>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    cache
        .entry(id.to_string())
        .or_insert_with(|| bases().iter().find_map(|b| std::fs::read_to_string(b.join(id).join("index.theme")).ok()).map(|t| parse(&t)))
        .clone()
}

/// Installed icon themes, as (folder name, display name), sorted by name. Cursor themes and
/// hidden ones are left out.
pub fn installed() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for base in bases() {
        for e in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            let id = e.file_name().to_string_lossy().into_owned();
            if id == "hicolor" || id == "default" || out.iter().any(|(i, _)| *i == id) {
                continue;
            }
            let Some(t) = theme(&id) else { continue };
            if t.hidden || t.dirs.is_empty() {
                continue;
            }
            out.push((id.clone(), if t.name.is_empty() { id } else { t.name }));
        }
    }
    out.sort_by_key(|(_, n)| n.to_lowercase());
    out
}

/// `name` in theme `id` (or what it inherits, hicolor aside), closest to `size` pixels.
pub fn find_in(id: &str, name: &str, size: u32) -> Option<PathBuf> {
    lookup(id, name, size, true, None)
}

/// `name` in theme `id` itself, not what it inherits: for icons that should be the theme's
/// own look or ours, never a third theme's.
pub fn find_own(id: &str, name: &str, size: u32) -> Option<PathBuf> {
    lookup(id, name, size, false, None)
}

/// Like `find_own`, preferring folders whose path has `context` in it ("status": panel and
/// tray icons, which are drawn for a panel's background, over toolbar ones of the same name).
pub fn find_own_in(id: &str, name: &str, size: u32, context: &str) -> Option<PathBuf> {
    lookup(id, name, size, false, Some(context))
}

fn lookup(id: &str, name: &str, size: u32, inherit: bool, context: Option<&str>) -> Option<PathBuf> {
    fn walk(id: &str, name: &str, size: u32, inherit: bool, context: Option<&str>, seen: &mut Vec<String>) -> Option<PathBuf> {
        if seen.iter().any(|s| s == id) || seen.len() > 4 || id == "hicolor" {
            return None;
        }
        seen.push(id.to_string());
        let t = theme(id)?;
        let mut best: Option<(u32, PathBuf)> = None;
        for base in bases() {
            let root = base.join(id);
            if !root.is_dir() {
                continue;
            }
            for (dir, dsize, scalable) in &t.dirs {
                for ext in ["svg", "png"] {
                    let p = root.join(dir).join(format!("{name}.{ext}"));
                    if !p.exists() {
                        continue;
                    }
                    // A scalable folder fits any size; otherwise the nearest size wins, SVG or
                    // not (Breeze's 16px icons are SVGs too, but monochrome).
                    let mut cost = if *scalable { 0 } else { dsize.abs_diff(size) + 1 };
                    if context.is_some_and(|c| !dir.contains(c)) {
                        cost += 1000;
                    }
                    if best.as_ref().is_none_or(|(c, _)| cost < *c) {
                        best = Some((cost, p));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        if !inherit {
            return best.map(|(_, p)| p);
        }
        best.map(|(_, p)| p).or_else(|| t.inherits.iter().find_map(|i| walk(i, name, size, inherit, context, seen)))
    }
    walk(id, name, size, inherit, context, &mut Vec::new())
}

static CHOSEN: Mutex<Option<String>> = Mutex::new(None);

/// Switch theme while running (a program that outlives a change in Settings, like
/// vide-desktop). Others read it from settings.conf when they start.
pub fn set_theme(id: &str) {
    *CHOSEN.lock().unwrap() = Some(id.to_string());
}

/// The desktop's own icon theme: KDE's choice (kdeglobals), else Breeze.
pub fn system_theme() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let rc = std::fs::read_to_string(format!("{home}/.config/kdeglobals")).unwrap_or_default();
    let mut in_icons = false;
    for line in rc.lines() {
        if line.starts_with('[') {
            in_icons = line == "[Icons]";
        } else if let (true, Some(t)) = (in_icons, line.strip_prefix("Theme=")) {
            if !t.trim().is_empty() {
                return t.trim().to_string();
            }
        }
    }
    "breeze".into()
}

/// `name` in the icon theme chosen in Settings, if one is chosen. Whatever that theme (and what
/// it inherits) lacks is filled in from the system's theme, then from our own `vide` pack, so
/// a theme with gaps never leaves a blank. Looked up once per theme, name and size.
pub fn themed_icon(name: &str, size: u32) -> Option<PathBuf> {
    static CACHE: OnceLock<Mutex<HashMap<(String, String, u32), Option<PathBuf>>>> = OnceLock::new();
    let chosen = CHOSEN.lock().unwrap().get_or_insert_with(|| crate::Config::load().icon_theme).clone();
    if chosen.is_empty() {
        return None;
    }
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    cache
        .entry((chosen.clone(), name.to_string(), size))
        .or_insert_with(|| {
            let system = system_theme();
            [chosen.as_str(), system.as_str(), "breeze", "vide"].iter().find_map(|t| find_in(t, name, size))
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn index_theme() {
        let t = parse("[Icon Theme]\nName=Win11\nInherits=hicolor,breeze\nDirectories=places/16,places/scalable\n\n[places/16]\nSize=16\nType=Fixed\n\n[places/scalable]\nSize=48\nType=Scalable\n");
        assert_eq!(t.name, "Win11");
        assert_eq!(t.inherits, vec!["hicolor", "breeze"]);
        assert_eq!(t.dirs, vec![("places/16".to_string(), 16, false), ("places/scalable".to_string(), 48, true)]);
    }
}
