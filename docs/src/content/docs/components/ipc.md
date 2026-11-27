---
title: 'IPC Socket (ipc.rs)'
description: 'Control socket for things only the compositor can do'
---

# IPC Socket (`ipc.rs`)

A Unix domain socket for our own apps, for things only the compositor can do (capture the
screen, put a PNG on the clipboard, list and focus windows). The full command list is in the
[IPC Commands reference](/vide-de/reference/ipc-commands/).

## Where it is

`ipc::start` binds `$XDG_RUNTIME_DIR/vide-de-<socket name>.sock` (e.g. `vide-de-wayland-0.sock`)
and exports the path to clients as `$VIDE_IPC`. If binding fails the socket is just disabled
with a warning; nothing else depends on it.

## Protocol

Plain text, **one newline-terminated command per connection**: `command [argument]`. The
compositor reads one line (200 ms timeout, so a stuck client can't block the main loop),
acts on it, writes a reply for the commands that have one (`windows`, `screen-info`,
`pointer-pos`, ...) and hangs up. Unknown commands are logged and ignored.

The exception is `remote-begin`: after replying `ok` the connection stays open and carries a
RemoteDesktop portal session's input, and only our own `vide-portal` may open one.

## Client usage

```bash
echo snip | socat - UNIX-CONNECT:$VIDE_IPC
echo screenshot | socat - UNIX-CONNECT:$VIDE_IPC
echo "copy-png /tmp/shot.png" | socat - UNIX-CONNECT:$VIDE_IPC
echo windows | socat - UNIX-CONNECT:$VIDE_IPC
```

## Security

- The socket lives in `$XDG_RUNTIME_DIR`, which only the user can access.
- Input injection (`pointer-*`, `key`, `type`), `screenshot <path>`, `popups` and `stall` are
  refused unless the compositor was started with `VIBE_TEST_INPUT=1` (as `scripts/nested-test.sh`
  does). A normal session never exposes them.
- `emoji` is accepted once per `Super+.` press, so it can't make apps paste at will.
- `remote-begin` checks the peer's PID against the supervised `vide-portal`.

## Related Files

- `screenshot.rs`: snip and screenshot capture
- `snip/`: Snip & Sketchy, which sends `snip` and `copy-png`
- `mangler/`: Task Mangler, which uses `windows`, `focus-window`, `close-window`
- `emoji/`: Emoji Panic, which sends `emoji`
- `remote.rs`: the remote-control session behind `remote-begin`
