---
title: 'Keybindings Reference'
description: 'Complete keyboard shortcut list'
---

# Keybindings Reference

## Window Management

| Shortcut | Action |
|----------|--------|
| `Super+Q` | Close focused window |
| `Super+M` | Toggle maximize |
| `Super+F` | Toggle fullscreen |
| `Super+H` | Minimize window |
| `Super+Shift+Left` | Snap left |
| `Super+Shift+Right` | Snap right |
| `Super+Shift+Up` | Snap up |
| `Super+Shift+Down` | Snap down |
| `Super+Shift+M` | Toggle maximize (alternate) |
| `Super+D` | Show desktop: minimize every window here; press again to bring them back |
| Hover the maximize button | Snap layouts: a panel of layouts (halves, thirds, quarters...) drops below it; click a zone to snap the window there |
| `Super+Shift+P` | Pin a window on top: opens a picker of this desktop's windows; click one (or arrows + `Enter`) to keep it above everything, fullscreen apps included. Pick a pinned one to unpin. `Esc` closes. Nested: `Alt+Shift+P` |

## Workspace (Virtual Desktops)

| Shortcut | Action |
|----------|--------|
| `Super+1` .. `Super+0` | Switch to workspace 1-10 |
| `Super+Shift+1` .. `Super+Shift+0` | Move window to workspace 1-10 |
| `Super+Ctrl+Left` | Previous workspace |
| `Super+Ctrl+Right` | Next workspace |
| `Super+Ctrl+F4` | Close this desktop: its windows move to the one before it |
| `Super+Tab` | Open Task View |

## Start Menu & Search

| Shortcut | Action |
|----------|--------|
| `Super` | Toggle Start menu |
| `Super+R` | Open Run command |
| `Super+S` | Open search (in Start menu) |
| `Escape` | Close Start menu / cancel |
| `Enter` | Launch selected app |
| `Arrow Up/Down` | Navigate results |
| `Tab` | Switch between Pinned/All Apps |

## System

| Shortcut | Action |
|----------|--------|
| `Super+X` | Quick link menu over the Start button (also a right click on Start): Apps and Features, Power Options, System, Network Connections, PowerSlop, Task Mangler, Settings, File Exploder, sign out / restart / shut down, Desktop |
| `Super+Enter` | Launch terminal (konsole) |
| `Super+E` | Launch File Exploder |
| `Super+V` | Clipboard history: the last 25 things you copied (text and images, screenshots included), newest first. Pick one with the mouse or arrows + `Enter` and it's pasted into the focused app. **Clear all** empties it. Password-manager copies are never kept. Nested: `Alt+V` |
| `Super+.` / `Super+;` | Emoji Panic, the emoji panel: search or browse by category, click one (or `Enter` for the first match) and it's typed into the app you were in, through the clipboard. `Esc` closes. Nested: `Alt+.` |
| `Super+Shift+Esc` | Launch Task Mangler |
| `Super+Shift+S` | Region snip (screenshot) |
| `PrtSc` | Fullscreen screenshot |
| `Win+PrtSc` | Fullscreen screenshot (save) |
| `Alt+PrtSc` | Active window screenshot |
| `Super+L` | Lock screen (if configured) |

## Focus & Navigation

| Shortcut | Action |
|----------|--------|
| `Alt+Tab` | Next window (switcher) |
| `Alt+Shift+Tab` | Previous window |
| `Super+Tab` | Task View |
| `Super+Shift+Tab` | Task View (reverse) |

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
| Move window | `Super+Left Drag` (titlebar or anywhere on SSD) |
| Resize window | `Super+Right Drag` (edges) |
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
