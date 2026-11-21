---
title: 'Keybindings Reference'
description: 'Complete keyboard shortcut list'
---

# Keybindings Reference

On a real TTY the modifier is **Super** (the Windoze key). Nested inside another desktop the
host keeps Super, so it's **Alt** there: read `Super+X` as `Alt+X`.

## Window Management

| Shortcut | Action |
|----------|--------|
| `Super+Q` / `Alt+F4` | Close the focused window |
| `Super+Up` | Maximize |
| `Super+Down` | Minimize |
| `Super+Left` / `Super+Right` | Snap to the left / right half |
| `Super+F` | Toggle fullscreen |
| `Super+D` | Show desktop: minimize every window here; press again to bring them back |
| Hover the maximize button | Snap layouts: a panel of layouts (halves, thirds, quarters...) drops below it; click a zone to snap the window there |
| `Super+Shift+P` | Pin a window on top: opens a picker of this desktop's windows; click one (or arrows + `Enter`) to keep it above everything, fullscreen apps included. Pick a pinned one to unpin. `Esc` closes |
| `Alt+Space` | The focused window's menu (TTY only; nested, Alt is the modifier) |

## Workspace (Virtual Desktops)

| Shortcut | Action |
|----------|--------|
| `Super+1` .. `Super+0` | Switch to desktop 1-10 |
| `Super+Shift+1` .. `Super+Shift+0` | Move the focused window to desktop 1-10 |
| `Super+Ctrl+Left` / `Super+Ctrl+Right` | Previous / next desktop |
| `Super+Ctrl+F4` | Close this desktop: its windows move to the one before it |
| `Super+Tab` | Task View (TTY; nested, `Alt+Tab` is the switcher) |

## Start Menu & Search

| Shortcut | Action |
|----------|--------|
| `Super` (tap) / `Super+Space` | Open or close Start. Typing searches straight away |
| `Up` / `Down` | Move through the results |
| `Enter` | Open the highlighted result |
| `Esc` | Close Start |

## System

| Shortcut | Action |
|----------|--------|
| `Super+X` | Quick link menu over the Start button (also a right click on Start): Apps and Features, Power Options, System, Network Connections, PowerSlop, Task Mangler, Settings, File Exploder, sign out / restart / shut down, Desktop |
| `Super+Enter` | Terminal (PowerSlop, else Konsole) |
| `Super+E` | File Exploder |
| `Super+A` / `Super+N` | Action Center |
| `Super+V` | Clipboard history: the last 25 things you copied (text and images, screenshots included), newest first. Pick one with the mouse or arrows + `Enter` and it's pasted into the focused app. **Clear all** empties it. Password-manager copies are never kept |
| `Super+.` / `Super+;` | Emoji Panic, the emoji panel: search or browse by category, click one (or `Enter` for the first match) and it's typed into the app you were in, through the clipboard. `Esc` closes |
| `Ctrl+Shift+Esc` | Task Mangler |
| `PrtSc` | Screenshot of the whole screen |
| `Shift+PrtSc` / `Super+Shift+S` | Snip a region |
| `Super+Shift+E` / `Ctrl+Alt+Backspace` | Sign out (quit the compositor) |
| `Ctrl+Alt+F1` .. `F12` | Switch to another virtual terminal (TTY) |

## Focus & Navigation

| Shortcut | Action |
|----------|--------|
| `Alt+Tab` / `Alt+Shift+Tab` | Window switcher, forwards / backwards; let go of Alt to pick, `Esc` backs out |

## Media Keys

| Key | Action |
|-----|--------|
| `XF86AudioRaiseVolume` | Volume +5% |
| `XF86AudioLowerVolume` | Volume -5% |
| `XF86AudioMute` | Toggle mute |
| `XF86AudioPlay` / `XF86AudioPause` | Play/pause whatever is playing (first MPRIS player), even unfocused |
| `XF86AudioNext` / `XF86AudioPrev` | Next / previous track |
| `XF86AudioStop` | Stop playback |
| `XF86MonBrightnessUp` | Brightness +10% |
| `XF86MonBrightnessDown` | Brightness -10% |

The volume and brightness keys show Windoze 10's overlay in the top-left corner: the new level on
a vertical bar, fading out about 1.6 s after the last press.

## Mouse Bindings

| Action | Binding |
|--------|---------|
| Move window | Drag the titlebar, or `Super+Left Drag` anywhere on the window |
| Resize window | Drag an edge or corner |
| Maximize / restore | Double-click the titlebar |
| Switch desktop | `Scroll` over the Task View button |
| Show window menu | `Right Click` (titlebar or taskbar button) |
| New window of that app | `Middle Click` (taskbar button) |
| Close a window | `Middle Click` (its taskbar preview) |
| Volume / brightness | `Scroll` over the tray icon or its flyout |
| Keep a flyout open | Its pin button (top right); clicks elsewhere leave it up until you unpin or press `Esc` |

## Customization

Add your own shortcuts in `~/.config/vide-de/shortcuts.conf`, one per line:

```text
# keys = command
super+shift+t = konsole
ctrl+alt+f = firefox --new-window
```

Modifiers are `super` (or `win`), `ctrl`, `alt` and `shift`; the key is any XKB key name
(`t`, `F5`, `Return`, `Print`...). Yours win over the built-in binds, so you can repoint those
too. Edits apply within a second, no sign-out needed. Nested, the host keeps Super, so bind
with `alt` to try them there.

## Debug Shortcuts (Dev Only)

| Shortcut | Action |
|----------|--------|
| `VIBE_DEBUG_START=1` | Open Start menu at launch |
| `VIBE_DEBUG_START=text` | Open Start menu + type "text" |
| `VIBE_DEBUG_DESKTOPS=1` | Open Task View at launch |
| `VIBE_DEBUG_SWITCHER=1` | Step Alt+Tab after 5s |
| `VIBE_DEBUG_SHOT=1` | Screenshot after 6s |

Set via environment variables when launching.

## Related Files

- [Input Handling](/vide-de/components/input-handling/) — Implementation details
- [Configuration](/vide-de/configuration/settings) — Other settings
- [Installation](/vide-de/getting-started/installation/) — Running with debug flags

## Menus

Every menu (right-click menus, the window menu, taskbar jump lists, clipboard history) also
works from the keyboard: `Up`/`Down` move the highlight, `Enter` picks, `Esc` closes.
