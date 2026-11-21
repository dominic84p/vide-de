---
title: 'Autostart & Session Restore'
description: 'Login items from ~/.config/autostart, and reopening last session''s apps'
---

# Autostart (`autostart.rs`) and Session Restore (`session.rs`)

## Autostart

At login, VIDE-DE starts the programs in the freedesktop autostart folders, the same ones
KDE and GNOME use: `~/.config/autostart` (or `$XDG_CONFIG_HOME/autostart`), then
`$XDG_CONFIG_DIRS/autostart` and `/etc/xdg/autostart`. This is where Discord, Steam,
Nextcloud and the like put themselves when you tick "start with the computer".

- A file in your own folder hides a system one with the same name, so `Hidden=true` in
  `~/.config/autostart/foo.desktop` turns off `/etc/xdg/autostart/foo.desktop`.
- `Hidden=true`, `X-GNOME-Autostart-enabled=false` and a missing `TryExec` program skip it.
- For `OnlyShowIn`/`NotShowIn` our name is **`VIDE-DE` only**. Apps see `VIDE-DE:KDE` in
  `XDG_CURRENT_DESKTOP` so they look right, but every piece of Plasma (plasmashell, powerdevil,
  xembedsniproxy...) autostarts with `OnlyShowIn=KDE` and must never start inside VIDE-DE.
- Things we already start ourselves (the KWallet unlock, the polkit agent) and `nm-applet`
  (it would add a second network icon next to ours) are skipped.

The compositor log has a line per item: `autostart discord.desktop: discord --start-minimized`.
`VIBE_STARTUP` still works on top of this, for testing.

## Session restore

While you work, VIDE-DE keeps a list of the apps you have open in
`~/.config/vide-de/session.conf`: which app (by desktop entry), which desktop, where, and
whether it was maximized. It's written once the window list has stayed the same for a few
seconds, and again right before Sign out, Restart or Shut down. The wait matters: shutting
down closes the apps a moment before the compositor, and that empty desktop must not replace
the real session.

At the next login, if anything was open, a toast asks **Restore previous session?** (like
Firefox after a restart), listing the apps. It stays until you answer:

- **Restore session** opens each app and puts its windows back on the same desktop, at the
  same place and size, maximized if they were.
- **Not now**, closing it in the Action Center, or **Clear all** says no. The saved session
  is kept until you have worked a few seconds in the new one.

Nothing is saved while the question is waiting, so a session you haven't answered for yet
is never lost.

Apps that restore their own windows (Firefox, Chromium) are started once and bring back
their own tabs and windows; VIDE-DE just puts them where they were. Apps that open one window
per launch (PowerSlop, say) are started again for each window that was open. Windows of apps
with no desktop entry can't be started again, and one-shot tools (Emoji Panic, Snip & Sketchy,
About) aren't saved.

To test it nested: `scripts/nested-test.sh --session scripts/nested-tests/data/session-restore.conf scripts/nested-tests/session-restore.test`.
