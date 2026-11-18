---
title: 'Config Library (vide-config)'
description: 'Shared settings crate, polling, type-safe access'
---

# Config Library (`vide-config`)

`config/` is a small library crate shared by the compositor and the Settings app. There is no
command-line tool: edit settings in the Settings app or in the file itself.

## The file

`~/.config/vide-de/settings.conf` (under `$XDG_CONFIG_HOME` if that's set): one `key=value` per
line, `#` lines ignored. Unknown keys are skipped and bad values fall back to the default, so a
hand edit can't break anything. Every key is listed in the
[Settings Reference](/vide-de/reference/settings-reference/).

## API

```rust
pub struct Config {
    pub light_mode: bool,
    pub animations: bool,
    pub wallpaper: PathBuf,
    // ... one public field per key
}

impl Config {
    pub fn path() -> PathBuf;                       // where the file lives
    pub fn load() -> Self;                          // defaults for anything missing
    pub fn save(&self) -> std::io::Result<()>;     // write a temp file, then rename over
    pub fn mtime() -> Option<SystemTime>;           // for cheap change polling
}
```

`save` writes the whole file at once and renames it into place, so a reader never sees half of
it.

## Live changes

The compositor checks `Config::mtime()` once a second and reloads when it changes. Settings
saves on every change, so toggles take effect within a second without restarting anything.
Things that need work to apply (the keyboard layout, display mode, mouse settings) are
re-applied by the compositor when their values differ from the last load.

## Adding a setting

1. Add a field to `Config`, its default in `Default`, a `match` arm in `load` and a line in `save`.
2. Read `state.config.<field>` in the compositor; it's current after each reload.
3. Add a control for it in `settings/`.
4. Add it to the [Settings Reference](/vide-de/reference/settings-reference/).

## Other helpers

The crate also knows where shared data lives: `wallpapers_dir()`, notification sounds
(`SOUNDS`, `sounds_dir()`, `custom_sounds()`, `sound_file()`), and the per-app notification
sound choice (`app_sound`, `set_app_sound`, `notifying_apps()`).
