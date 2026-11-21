---
title: 'How-To Guides'
description: 'Step-by-step guides for common tasks'
---

# How-To Guides

Most settings live in the Settings app (`vide-settings`, or Start → Settings), which writes
`~/.config/vide-de/settings.conf`. You can edit that file by hand too: the compositor reads it
again within a second. Every key is in the [Settings Reference](/vide-de/reference/settings-reference/).

## Light or Dark Mode

Settings → Personalization → Colors, or in `settings.conf`:

```ini
light_mode=true
```

## Add a Keybinding

Put a line like `super+shift+t = konsole` in `~/.config/vide-de/shortcuts.conf`; it works within
a second. See [Keybindings](/vide-de/reference/keybindings/) for the format.

## Pin an App to the Taskbar

Right-click it in Start → **Pin to taskbar**, or drag it from Start onto the taskbar. Drag
taskbar buttons to reorder them.

## Change the Wallpaper

Settings → Personalization → Background. The bundled wallpapers are in
`~/.local/share/vide-de/wallpapers/`; to use your own, set `wallpaper=/path/to/image.png` in
`settings.conf`.

## Take a Screenshot

| Shortcut | Result |
|----------|--------|
| `PrtSc` | The whole screen (every monitor), saved to `~/Pictures/Screenshots/` and copied |
| `Shift+PrtSc` or `Win+Shift+S` | Pick a region, then mark it up in Snip & Sketchy |

Copied screenshots paste into any app, X11 ones (Discord, Claude Desktop) included.

## Paste Something You Copied Earlier

`Win+V` opens clipboard history: the last 25 things copied, newest first. Pick one and it's
pasted into the app you're in.

## Type an Emoji

`Win+.` opens Emoji Panic. Search or browse, click one, and it's typed into the app you were in.

## Record the Screen

Not built in yet. `wf-recorder` and OBS work, through our screencast portal (`vide-portal`).

## Let Someone Control This PC (RustDesk)

RustDesk (and other remote desktop apps) ask through the RemoteDesktop portal, which
`vide-portal` provides. When someone connects, VIDE-DE asks **Allow remote control**: the app
wants to see your screen and use your mouse and keyboard. It asks every time; there's no
"remember", since this hands over the whole desktop.

While someone is in control an orange border runs round every monitor and a notification says
which app is controlling the PC. **Stop** in it ends the session at once. Only our own
`vide-portal` can turn remote control on, so another program on the control socket can't.

This needs the portal files from `scripts/install.sh` (`vide.portal` lists RemoteDesktop, and
`vide-portals.conf` points it at us).

## Start an App with the Session

Apps in `~/.config/autostart` (and `/etc/xdg/autostart`) start at login, as on other desktops;
see [Autostart & Session Restore](/vide-de/components/session/). For testing, `VIBE_STARTUP`
runs a command once the compositor (and Xwayland) is up:

```bash
VIBE_STARTUP=firefox vibe-comp --tty
```

## Workspaces, Task View and Alt+Tab

- `Super+1` … `Super+0`: go to workspace 1-10; add `Shift` to move the focused window there.
- `Super+Ctrl+Left` / `Right`: previous / next workspace.
- `Super+Tab`: Task View, every window of this desktop and the desktop strip.
- Hold `Alt` and press `Tab` to pick a window; let go of `Alt` to switch.

## Turn Animations Off

Settings → Ease of Access → Display → **Show animations**, or in `settings.conf`:

```ini
animations=false
```

## Reset All Settings

Delete the file; the defaults come back at once:

```bash
rm ~/.config/vide-de/settings.conf
```

## Run Nested for Testing

```bash
# Private D-Bus + private runtime dir (recommended)
XDG_RUNTIME_DIR=/tmp/vt WAYLAND_DISPLAY=wayland-1 \
dbus-run-session -- \
env VIBE_STARTUP=konsole VIBE_DEBUG_SHOT=1 \
timeout 15 ./target/release/vibe-comp --nested
```

## Debug Start Menu

```bash
VIBE_DEBUG_START=1 ./target/release/vibe-comp --nested
# Opens Start menu at launch

VIBE_DEBUG_START=text ./target/release/vibe-comp --nested
# Opens Start menu + types "text" in search
```

## Capture Compositor Screenshot

```bash
VIBE_DEBUG_SHOT=1 timeout 10 ./target/release/vibe-comp --nested
# Screenshots itself after 6 s
```

## Test TTY Mode Safely

1. Open new terminal
2. `sudo systemctl enable --now seatd`
3. `sudo usermod -aG seat,video $USER`
4. Log out and back in
5. `Ctrl+Alt+F4` (switch to VT)
6. Log in
7. `dbus-run-session -- ./target/release/vibe-comp --tty`
8. `Ctrl+Alt+Backspace` to exit

## Fix Flatpak After Testing

```bash
systemctl --user restart xdg-document-portal
```

## Related

- [FAQ](/vide-de/wiki/faq/)
- [Troubleshooting](/vide-de/wiki/troubleshooting/)
- [Keybindings](/vide-de/reference/keybindings/)
- [Settings Reference](/vide-de/reference/settings-reference/)