//! Remote control, for the RemoteDesktop portal (RustDesk and friends being let in).
//!
//! The user says yes in vide-portal's prompt; the portal then opens one connection to our
//! control socket with `remote-begin <app>` and keeps it open, sending the remote side's mouse
//! and keyboard down it, a line per event. We take that only from the vide-portal we started
//! ourselves (checked by the socket's peer PID), so another program on the socket can't drive
//! the desktop. While it's open a border runs round every monitor and a notification says who
//! is in control, with Stop; closing the connection, from either end, ends it.

use std::{
    io::{ErrorKind, Read, Write},
    os::unix::net::UnixStream,
    time::Instant,
};

use smithay::{
    backend::input::{Axis, AxisSource, ButtonState},
    input::pointer::AxisFrame,
    reexports::calloop::{generic::Generic, Interest, Mode, PostAction, RegistrationToken},
    utils::{Logical, Point},
};

use crate::{
    notifications::{NotifEvent, Notification},
    state::State,
};

/// The "is being controlled" notification's id; ours, never sent over D-Bus.
pub const NOTIF_ID: u32 = u32::MAX - 8;

pub struct Remote {
    pub app: String,
    token: RegistrationToken,
    /// What the remote side holds down, let go of when it leaves so nothing stays stuck.
    keys: Vec<u32>,
    buttons: Vec<u32>,
    partial: String,
    pub since: Instant,
}

impl State {
    pub fn remote_begin(&mut self, stream: UnixStream, app: &str) {
        use smithay::reexports::rustix::net::sockopt::socket_peercred;
        let peer = socket_peercred(&stream).ok().map(|c| c.pid.as_raw_nonzero().get() as u32);
        let portal = self.supervised_pid("vide-portal");
        if peer.is_none() || peer != portal {
            return tracing::warn!("control socket: remote-begin from pid {peer:?}, not vide-portal ({portal:?}); refused");
        }
        if self.remote.is_some() {
            return tracing::warn!("control socket: a remote session is already running; refused");
        }
        let app = if app.trim().is_empty() { "An app".to_string() } else { app.trim().to_string() };
        let _ = (&stream).write_all(b"ok\n");
        let _ = stream.set_nonblocking(true);
        let source = Generic::new(stream, Interest::READ, Mode::Level);
        let token = match self.loop_handle.insert_source(source, |_, stream, state| {
            let mut buf = [0u8; 4096];
            loop {
                // SAFETY: we only read from the stream; calloop keeps it registered meanwhile.
                match unsafe { stream.get_mut() }.read(&mut buf) {
                    Ok(0) => {
                        state.remote_ended();
                        return Ok(PostAction::Remove);
                    }
                    Ok(n) => state.remote_input(&String::from_utf8_lossy(&buf[..n])),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(PostAction::Continue),
                    Err(_) => {
                        state.remote_ended();
                        return Ok(PostAction::Remove);
                    }
                }
            }
        }) {
            Ok(t) => t,
            Err(e) => return tracing::warn!("remote control: {e}"),
        };
        // The portal waits for this before it sends anything, so nothing it sends can be lost in
        // the buffer the command line was read through.
        tracing::info!("remote control started by {app}");
        self.remote = Some(Remote { app: app.clone(), token, keys: Vec::new(), buttons: Vec::new(), partial: String::new(), since: Instant::now() });
        self.handle_notif_event(NotifEvent::Added(Notification {
            id: NOTIF_ID,
            app_name: "VIDE-DE".into(),
            app_icon: "preferences-desktop-remote-desktop".into(),
            summary: format!("{app} is controlling this PC"),
            body: "It can see and use your screen.".into(),
            actions: vec![("stop".into(), "Stop".into())],
            timestamp: crate::state::clock_now(self.config.clock_24h).0,
            urgency: 2,
            timeout_ms: 0,
            silent: false,
            sound_file: None,
            desktop_entry: None,
        }));
        self.needs_redraw = true;
    }

    /// Stop, from the notification: drop the connection, which tells the portal too.
    pub fn remote_stop(&mut self) {
        if let Some(r) = self.remote.as_ref() {
            self.loop_handle.remove(r.token);
        }
        self.remote_ended();
    }

    fn remote_ended(&mut self) {
        let Some(r) = self.remote.take() else { return };
        tracing::info!("remote control by {} ended after {}s", r.app, r.since.elapsed().as_secs());
        let t = self.now_ms();
        for k in r.keys {
            self.key_event(k, false);
        }
        for b in r.buttons {
            self.on_button(b, ButtonState::Released, t);
        }
        self.handle_notif_event(NotifEvent::Closed(NOTIF_ID));
        self.action_center.unseen = self.action_center.unseen.saturating_sub(1);
        self.taskbar = None;
        self.needs_redraw = true;
    }

    fn remote_input(&mut self, data: &str) {
        let Some(r) = self.remote.as_mut() else { return };
        r.partial.push_str(data);
        let Some(end) = r.partial.rfind('\n') else { return };
        let lines: Vec<String> = r.partial[..end].lines().map(String::from).collect();
        r.partial.drain(..=end);
        for line in lines {
            self.remote_event(&line);
        }
        self.needs_redraw = true;
    }

    fn remote_event(&mut self, line: &str) {
        let mut it = line.split_whitespace();
        let cmd = it.next().unwrap_or("");
        let nums: Vec<f64> = it.filter_map(|v| v.parse().ok()).collect();
        let n = |i: usize| nums.get(i).copied().unwrap_or(0.0);
        let t = self.now_ms();
        match cmd {
            // Relative, in logical pixels.
            "motion" => {
                let to = self.pointer_loc + Point::<f64, Logical>::from((n(0), n(1)));
                let to = self.clamp_to_monitors(self.pointer_loc, to);
                self.pointer_to(to, t);
            }
            // Absolute, in the shared screen's coordinates: the primary monitor at (0, 0).
            "abs" => {
                let p = Point::from((n(0), n(1)));
                let to = self.clamp_to_monitors(self.pointer_loc, p);
                self.pointer_to(to, t);
            }
            "button" => {
                let (code, down) = (n(0) as u32, n(1) != 0.0);
                if !(0x110..=0x117).contains(&code) {
                    return;
                }
                let Some(r) = self.remote.as_mut() else { return };
                r.buttons.retain(|b| *b != code);
                if down {
                    r.buttons.push(code);
                }
                self.on_button(code, if down { ButtonState::Pressed } else { ButtonState::Released }, t);
            }
            "axis" => self.remote_scroll(n(0), n(1), None, t),
            // Wheel clicks: axis 0 is vertical, 1 horizontal (as the portal numbers them).
            "axis-discrete" => {
                let steps = n(1);
                if n(0) == 0.0 {
                    self.remote_scroll(0.0, steps * 15.0, Some((Axis::Vertical, steps as i32)), t)
                } else {
                    self.remote_scroll(steps * 15.0, 0.0, Some((Axis::Horizontal, steps as i32)), t)
                }
            }
            // Evdev key codes.
            "key" => {
                let (code, down) = (n(0) as u32, n(1) != 0.0);
                if code == 0 || code > 767 {
                    return;
                }
                self.remote_key(code, down);
            }
            // An X keysym: typed with the key that makes it on a US layout, shifted if need be.
            "keysym" => {
                let (sym, down) = (n(0) as u32, n(1) != 0.0);
                let Some((code, shift)) = keysym_to_evdev(sym) else { return };
                if shift && down {
                    self.remote_key(42, true);
                }
                self.remote_key(code, down);
                if shift && !down {
                    self.remote_key(42, false);
                }
            }
            _ => {}
        }
    }

    fn remote_key(&mut self, code: u32, down: bool) {
        let Some(r) = self.remote.as_mut() else { return };
        r.keys.retain(|k| *k != code);
        if down {
            r.keys.push(code);
        }
        self.key_event(code, down);
    }

    fn remote_scroll(&mut self, dx: f64, dy: f64, discrete: Option<(Axis, i32)>, t: u32) {
        let mut frame = AxisFrame::new(t).source(if discrete.is_some() { AxisSource::Wheel } else { AxisSource::Finger });
        if dx != 0.0 {
            frame = frame.value(Axis::Horizontal, dx);
        }
        if dy != 0.0 {
            frame = frame.value(Axis::Vertical, dy);
        }
        if let Some((axis, steps)) = discrete {
            frame = frame.v120(axis, steps * 120);
        }
        let ptr = self.pointer.clone();
        ptr.axis(self, frame);
        ptr.frame(self);
    }
}

/// The key (evdev code) and whether Shift is needed for an X keysym, for what a US keyboard
/// can type. RustDesk mostly sends key codes; this covers text it sends as symbols.
fn keysym_to_evdev(sym: u32) -> Option<(u32, bool)> {
    let named = match sym {
        0xff08 => Some(14),  // BackSpace
        0xff09 => Some(15),  // Tab
        0xff0d => Some(28),  // Return
        0xff1b => Some(1),   // Escape
        0xffff => Some(111), // Delete
        0xff50 => Some(102), // Home
        0xff57 => Some(107), // End
        0xff51 => Some(105), // Left
        0xff52 => Some(103), // Up
        0xff53 => Some(106), // Right
        0xff54 => Some(108), // Down
        0xff55 => Some(104), // Page Up
        0xff56 => Some(109), // Page Down
        0xffe1 => Some(42),  // Shift_L
        0xffe3 => Some(29),  // Control_L
        0xffe9 => Some(56),  // Alt_L
        0xffeb => Some(125), // Super_L
        0xffbe..=0xffc9 => Some(match sym - 0xffbe { 10 => 87, 11 => 88, f => 59 + f }), // F1..F12
        _ => None,
    };
    if let Some(code) = named {
        return Some((code, false));
    }
    let ch = char::from_u32(sym).filter(|c| c.is_ascii() && !c.is_ascii_control())?;
    const SHIFTED: &str = "~!@#$%^&*()_+{}|:\"<>?";
    const PLAIN: &str = "`1234567890-=[]\\;',./";
    if ch.is_ascii_uppercase() {
        return crate::ipc::evdev_key(&ch.to_ascii_lowercase().to_string()).map(|c| (c, true));
    }
    if let Some(i) = SHIFTED.find(ch) {
        let base = PLAIN.chars().nth(i)?;
        return crate::ipc::evdev_key(&base.to_string()).map(|c| (c, true));
    }
    let name = if ch == ' ' { "space".to_string() } else { ch.to_string() };
    crate::ipc::evdev_key(&name).map(|c| (c, false))
}

#[cfg(test)]
mod tests {
    use super::keysym_to_evdev;

    #[test]
    fn keysyms() {
        assert_eq!(keysym_to_evdev('a' as u32), Some((30, false)));
        assert_eq!(keysym_to_evdev('A' as u32), Some((30, true)));
        assert_eq!(keysym_to_evdev(0xff0d), Some((28, false)));
        assert_eq!(keysym_to_evdev(0xffbe), Some((59, false)));
        assert_eq!(keysym_to_evdev(0xffc9), Some((88, false)));
        assert_eq!(keysym_to_evdev(0x20ac), None);
    }
}
