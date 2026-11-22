---
title: 'System Tray (tray.rs)'
description: 'Volume, brightness, network and calendar flyouts, and the volume/brightness overlay'
---

# System Tray (`tray.rs`)

Our own tray buttons on the right of the taskbar and the flyouts they open. App tray icons
(Discord, Steam, ...) are a separate thing, see [`sni.rs`](./sni).

## Flyouts

One flyout at a time (`Tray::flyout`), painted on the CPU and cached by a key that holds
everything it shows. Any click outside closes it.

| Button | Flyout | What's in it |
|--------|--------|--------------|
| Speaker | Volume | A slider (drag it, or scroll over the speaker); click the speaker to mute. Talks to PipeWire with `wpctl`. When something plays, a card under it (`media.rs`) shows the title and artist with previous, play/pause and next. It asks MPRIS over D-Bus every 2 s while the flyout is open, preferring a player that is playing. |
| Sun | Brightness | A slider. Desktop monitors have no backlight, so brightness is a black veil the compositor draws over everything below the cursor (`State::dim`), never darker than 20%. |
| Network | Network | Windoze 10's network list from NetworkManager (`nmcli`): connect, disconnect, passwords, Wi-Fi / airplane / hotspot toggles. It rescans when opened. |
| Clock | Calendar (`calendar.rs`) | The time, large, and the full date, over a month calendar with today filled in. The arrows or the mouse wheel page through months; **Today** comes back. |

## Volume and brightness overlay (`osd.rs`)

The volume, mute and brightness keys change the level and show Windoze 10's overlay in the
top-left corner: the speaker or sun, a vertical bar and the percentage. It stays 1.6 s after the
last press and fades out; timers wake the loop for that, so nothing redraws while it's idle. It
hides while a tray flyout is open, since the flyout already shows the level.

## Keys

| Key | Action |
|-----|--------|
| `XF86AudioRaiseVolume` / `XF86AudioLowerVolume` | Volume ±5%, with the overlay |
| `XF86AudioMute` | Toggle mute, with the overlay |
| `XF86MonBrightnessUp` / `XF86MonBrightnessDown` | Brightness ±10%, with the overlay |

## Related files

- `calendar.rs`: the calendar flyout's drawing and month maths
- `osd.rs`: the key overlay
- `media.rs`: what's playing (MPRIS), for the volume flyout
- `sni.rs`: app tray icons
- `input.rs`: media keys, flyout clicks and scrolling

## Overflow

Apps' own tray icons (Discord, Steam, nm-applet...) sit behind a ^ chevron left of ours, as on
Windoze 10, instead of on the taskbar. Click it for a grid of them: click an icon to open its
app, right-click for its menu; a click elsewhere or Esc closes the grid. Our own icons (network,
brightness, volume) stay on the taskbar on a 24px grid. See `overflow.rs`.

**Remove tray overflow** (Settings → Personalization → Taskbar) puts apps' tray icons back on
the taskbar itself, as before the chevron, and drops the chevron. It's `inline_tray` in
settings.conf.

