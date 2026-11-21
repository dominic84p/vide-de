//! Session restore. Which apps were open, on which desktop and where, is written to
//! session.conf (next to settings.conf) as you work. At the next login a toast asks
//! "Restore previous session?", like Firefox after a restart; yes reopens those apps and puts
//! their windows back where they were.

use std::time::{Duration, Instant};

use smithay::{
    reexports::{
        calloop::timer::{TimeoutAction, Timer},
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Rectangle},
};

use crate::{
    notifications::{NotifEvent, Notification},
    state::State,
};

/// The prompt's notification id: far above what the D-Bus server hands out, and never sent to
/// it, since it's ours.
pub const PROMPT_ID: u32 = u32::MAX - 7;
/// Windows of apps we don't bring back: one-shot tools.
const SKIP: &[&str] = &["emoji-panic", "snip-and-sketchy", "vide-about"];
/// A restored app gets this long for its windows to show up and take their old places.
const PLACE_FOR: Duration = Duration::from_secs(90);
/// How long the window list must stay the same before it's saved. Shutting down closes the
/// apps a moment before us; this keeps that empty desktop from replacing the real session.
const SETTLE: u32 = 3;

#[derive(Clone, PartialEq, Debug)]
pub struct Saved {
    /// Desktop-file id.
    app: String,
    workspace: usize,
    rect: Rectangle<i32, Logical>,
    maximized: bool,
}

#[derive(Default)]
pub struct Session {
    /// Last session's windows, while the prompt waits for an answer. Nothing is saved
    /// meanwhile, or the empty new session would replace the one on offer.
    offer: Option<Vec<Saved>>,
    /// Restoring: windows still to be put back, and until when to wait for them.
    placing: Vec<Saved>,
    placing_until: Option<Instant>,
    /// What's on disk, and the window list as seen on the last ticks (to tell when it settled).
    written: Option<Vec<Saved>>,
    last: Vec<Saved>,
    steady: u32,
}

fn path() -> std::path::PathBuf {
    vide_config::Config::path().with_file_name("session.conf")
}

fn parse(text: &str) -> Vec<Saved> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.strip_prefix("window=")?.split('\t').collect();
            let n = |i: usize| f.get(i)?.parse::<i32>().ok();
            Some(Saved {
                app: f.first()?.to_string(),
                workspace: n(1)?.max(0) as usize,
                rect: Rectangle::new((n(2)?, n(3)?).into(), (n(4)?.max(1), n(5)?.max(1)).into()),
                maximized: n(6)? == 1,
            })
        })
        .collect()
}

fn format(list: &[Saved]) -> String {
    list.iter()
        .map(|s| {
            let r = s.rect;
            format!("window={}\t{}\t{}\t{}\t{}\t{}\t{}\n", s.app, s.workspace, r.loc.x, r.loc.y, r.size.w, r.size.h, s.maximized as u8)
        })
        .collect()
}

/// "Firefox, Discord and PowerSlop".
fn names(list: &[String]) -> String {
    match list {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

impl State {
    /// The windows worth bringing back: apps with a desktop entry, not dialogs.
    fn session_now(&self) -> Vec<Saved> {
        self.wm
            .windows
            .iter()
            .filter(|w| w.mapped && self.wm.parent_surface(w).is_none())
            .filter_map(|w| {
                let app = crate::apps::match_app(&self.apps, &w.app_id())?;
                if SKIP.contains(&app.id.as_str()) {
                    return None;
                }
                Some(Saved { app: app.id.clone(), workspace: w.workspace, rect: w.maximized.unwrap_or(w.geometry()), maximized: w.maximized.is_some() })
            })
            .collect()
    }

    /// Called once a second: write the window list once it has settled and changed.
    pub fn session_tick(&mut self) {
        if self.session.offer.is_some() {
            return;
        }
        let now = self.session_now();
        if now == self.session.last {
            self.session.steady += 1;
        } else {
            self.session.last = now;
            self.session.steady = 0;
        }
        if self.session.steady >= SETTLE && self.session.written.as_ref() != Some(&self.session.last) {
            self.session_write();
        }
    }

    /// Write it now (signing out, restarting): the apps are all still open at this point.
    pub fn session_save_now(&mut self) {
        if self.session.offer.is_none() {
            self.session.last = self.session_now();
            self.session_write();
        }
    }

    fn session_write(&mut self) {
        if let Err(e) = std::fs::write(path(), format(&self.session.last)) {
            tracing::warn!("couldn't save the session: {e}");
        }
        self.session.written = Some(self.session.last.clone());
    }

    /// At login: if the last session had apps open, ask whether to bring them back.
    pub fn session_offer(&mut self) {
        let saved: Vec<Saved> = std::fs::read_to_string(path()).map(|t| parse(&t)).unwrap_or_default();
        let saved: Vec<Saved> = saved.into_iter().filter(|s| self.apps.iter().any(|a| a.id == s.app)).collect();
        if saved.is_empty() {
            return;
        }
        let mut apps: Vec<String> = Vec::new();
        for s in &saved {
            let name = self.apps.iter().find(|a| a.id == s.app).map(|a| a.name.clone()).unwrap_or_default();
            if !apps.contains(&name) {
                apps.push(name);
            }
        }
        // Just the names: a toast has one line for the body.
        let body = names(&apps);
        self.session.offer = Some(saved);
        // A moment after login, so it doesn't land while the desktop is still coming up.
        let notif = Notification {
            id: PROMPT_ID,
            app_name: "VIDE-DE".into(),
            app_icon: "document-revert".into(),
            summary: "Restore previous session?".into(),
            body,
            actions: vec![("restore".into(), "Restore session".into()), ("no".into(), "Not now".into())],
            timestamp: crate::state::clock_now(self.config.clock_24h).0,
            urgency: 1,
            // Stays until answered.
            timeout_ms: 0,
            silent: true,
            sound_file: None,
            desktop_entry: None,
        };
        let _ = self.loop_handle.insert_source(Timer::from_duration(Duration::from_secs(2)), move |_, _, state| {
            if state.session.offer.is_some() {
                state.handle_notif_event(NotifEvent::Added(notif.clone()));
            }
            TimeoutAction::Drop
        });
    }

    /// The prompt was answered (`restore`), turned down (`no`) or dismissed.
    pub fn session_answer(&mut self, action: &str) {
        let Some(saved) = self.session.offer.take() else { return };
        self.handle_notif_event(NotifEvent::Closed(PROMPT_ID));
        // Answered, so it's not news waiting in the Action Center either.
        self.action_center.unseen = self.action_center.unseen.saturating_sub(1);
        self.taskbar = None;
        if action != "restore" {
            return;
        }
        tracing::info!("restoring {} windows", saved.len());
        let mut launched: Vec<String> = Vec::new();
        for s in &saved {
            if launched.contains(&s.app) {
                continue;
            }
            launched.push(s.app.clone());
            if let Some(app) = self.apps.iter().find(|a| a.id == s.app).cloned() {
                self.launch(&app);
            }
        }
        let mut wanted: Vec<(String, usize)> = Vec::new();
        for s in &saved {
            match wanted.iter_mut().find(|(a, _)| *a == s.app) {
                Some((_, n)) => *n += 1,
                None => wanted.push((s.app.clone(), 1)),
            }
        }
        self.session.placing = saved;
        self.session.placing_until = Some(Instant::now() + PLACE_FOR);
        // Apps that bring back their own windows (Firefox, Chromium) have several by now, and
        // ones still starting have none; leave both be. An app that made exactly one of several
        // is a one-window-per-launch app (PowerSlop): start it again for each one missing.
        let _ = self.loop_handle.insert_source(Timer::from_duration(Duration::from_secs(6)), move |_, _, state| {
            for (id, n) in &wanted {
                let missing = state.session.placing.iter().filter(|s| s.app == *id).count();
                if *n > 1 && n - missing == 1 {
                    if let Some(app) = state.apps.iter().find(|a| a.id == *id).cloned() {
                        for _ in 0..missing {
                            state.launch(&app);
                        }
                    }
                }
            }
            TimeoutAction::Drop
        });
    }

    /// A window just appeared: if it's one we're restoring, put it back where it was.
    pub fn session_place(&mut self, surface: &WlSurface) {
        if self.session.placing_until.is_none_or(|t| Instant::now() > t) {
            self.session.placing.clear();
            return;
        }
        let Some(w) = self.wm.find(surface) else { return };
        if self.wm.parent_surface(w).is_some() {
            return;
        }
        let Some(id) = crate::apps::match_app(&self.apps, &w.app_id()).map(|a| a.id.clone()) else { return };
        let Some(i) = self.session.placing.iter().position(|s| s.app == id) else { return };
        let s = self.session.placing.remove(i);
        if let Some(w) = self.wm.find_mut(surface) {
            w.workspace = s.workspace;
            w.pos = s.rect.loc;
            w.configure(Some(s.rect.size), &[]);
        }
        if s.maximized {
            let area = self.work_area_of(surface);
            self.wm.maximize(surface, area);
        }
        self.needs_redraw = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let list = vec![
            Saved { app: "firefox".into(), workspace: 0, rect: Rectangle::new((10, 40).into(), (800, 600).into()), maximized: false },
            Saved { app: "org.kde.dolphin".into(), workspace: 2, rect: Rectangle::new((0, 0).into(), (1920, 1040).into()), maximized: true },
        ];
        assert_eq!(parse(&format(&list)), list);
        assert!(parse("window=broken\n").is_empty());
    }

    #[test]
    fn names_read_naturally() {
        let s = |v: &[&str]| names(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(s(&["Firefox"]), "Firefox");
        assert_eq!(s(&["Firefox", "Discord"]), "Firefox and Discord");
        assert_eq!(s(&["Firefox", "Discord", "PowerSlop"]), "Firefox, Discord and PowerSlop");
    }
}
