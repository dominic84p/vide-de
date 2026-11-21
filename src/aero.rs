//! Two Windoze 7 tricks.
//!
//! Aero Shake: grab a window by its titlebar and shake it, and every other window on this
//! desktop minimizes; shake it again and they come back.
//!
//! Aero Peek: rest the pointer on the taskbar's far corner and the windows turn see-through to
//! show the desktop; rest it on a taskbar preview and only that window shows, in its place.
//! Moving away puts everything back.

use std::time::{Duration, Instant};

use smithay::{
    reexports::{
        calloop::timer::{TimeoutAction, Timer},
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Point},
};

use crate::{preview::PreviewHit, state::State, ui::TaskbarHit};

/// A turn only counts once the window has travelled this far since the last one.
const SHAKE_PX: f64 = 40.0;
/// This many turns, all within `SHAKE_WINDOW`, make a shake.
const SHAKE_TURNS: usize = 3;
const SHAKE_WINDOW: Duration = Duration::from_millis(900);
/// How long the pointer rests before a peek.
const PEEK_DELAY: Duration = Duration::from_millis(500);
/// The other windows fade out over this long.
pub const PEEK_FADE: Duration = Duration::from_millis(160);

#[derive(Default)]
pub struct Shake {
    /// The furthest x reached in the current direction, and which way that is (-1, 0, 1).
    extreme: Option<f64>,
    dir: i8,
    turns: Vec<Instant>,
    /// What the last shake minimized, and which window did it, to bring them back.
    shaken: Option<(WlSurface, Vec<WlSurface>)>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum PeekAt {
    Desktop,
    Window(WlSurface),
}

#[derive(Default)]
pub struct Peek {
    pending: Option<(PeekAt, Instant)>,
    pub shown: Option<(PeekAt, Instant)>,
}

impl State {
    /// The pointer moved while a window is being dragged by its titlebar.
    pub fn shake_track(&mut self, surface: &WlSurface, pos: Point<f64, Logical>) {
        let s = &mut self.shake;
        let Some(extreme) = s.extreme else {
            s.extreme = Some(pos.x);
            return;
        };
        let dx = pos.x - extreme;
        if s.dir == 0 {
            if dx.abs() > 2.0 {
                s.dir = dx.signum() as i8;
                s.extreme = Some(pos.x);
            }
            return;
        }
        // Still going the same way: the turning point moves along with it.
        if dx.signum() as i8 == s.dir {
            s.extreme = Some(pos.x);
            return;
        }
        if dx.abs() < SHAKE_PX {
            return;
        }
        // Turned round, far enough to be deliberate.
        let now = Instant::now();
        s.dir = -s.dir;
        s.extreme = Some(pos.x);
        s.turns.retain(|t| now - *t < SHAKE_WINDOW);
        s.turns.push(now);
        if s.turns.len() >= SHAKE_TURNS {
            s.turns.clear();
            self.shake_others(surface);
        }
    }

    /// The drag ended: the next one starts counting afresh.
    pub fn shake_reset(&mut self) {
        self.shake.dir = 0;
        self.shake.extreme = None;
        self.shake.turns.clear();
    }

    fn shake_others(&mut self, surface: &WlSurface) {
        tracing::info!("aero shake (restoring: {})", self.shake.shaken.is_some());
        let ws = self.wm.current_workspace;
        if let Some((by, hidden)) = self.shake.shaken.take() {
            let back: Vec<_> = hidden.into_iter().filter(|s| self.wm.find(s).is_some_and(|w| w.minimized)).collect();
            if by == *surface && !back.is_empty() {
                for w in self.wm.windows.iter_mut().filter(|w| back.contains(w.surface())) {
                    w.minimized = false;
                    w.vanish = None;
                    w.appear = Some(Instant::now());
                    w.restored = true;
                }
                // The shaken window stays on top of what came back.
                self.wm.raise(surface);
                self.needs_redraw = true;
                return;
            }
        }
        let mut hidden = Vec::new();
        for w in self.wm.windows.iter_mut().filter(|w| w.workspace == ws && w.mapped && !w.minimized && w.surface() != surface && !w.on_top) {
            w.minimized = true;
            w.vanish = Some(Instant::now());
            hidden.push(w.surface().clone());
        }
        if !hidden.is_empty() {
            self.shake.shaken = Some((surface.clone(), hidden));
            self.needs_redraw = true;
        }
    }

    /// Called on pointer motion with what's under the pointer.
    pub fn peek_track(&mut self, taskbar: Option<&TaskbarHit>, preview: Option<PreviewHit>) {
        let want = match (taskbar, preview) {
            (Some(TaskbarHit::ShowDesktop), _) => Some(PeekAt::Desktop),
            (_, Some(PreviewHit::Window(i))) => {
                self.preview.shown.as_ref().and_then(|(_, w, _)| w.get(i).cloned()).map(PeekAt::Window)
            }
            _ => None,
        };
        let Some(want) = want else {
            self.peek.pending = None;
            if self.peek.shown.take().is_some() {
                self.needs_redraw = true;
            }
            return;
        };
        if self.peek.shown.as_ref().is_some_and(|(p, _)| *p == want) {
            return;
        }
        // Already peeking (moving along the previews): switch straight away, like Windoze.
        if self.peek.shown.is_some() {
            self.peek.shown = Some((want, Instant::now()));
            self.needs_redraw = true;
            return;
        }
        if self.peek.pending.as_ref().is_some_and(|(p, _)| *p == want) {
            return;
        }
        self.peek.pending = Some((want.clone(), Instant::now()));
        let _ = self.loop_handle.insert_source(Timer::from_duration(PEEK_DELAY), move |_, _, state| {
            if let Some((p, t0)) = state.peek.pending.clone() {
                if p == want && t0.elapsed() >= PEEK_DELAY - Duration::from_millis(5) {
                    state.peek.pending = None;
                    state.peek.shown = Some((p, Instant::now()));
                    state.needs_redraw = true;
                }
            }
            TimeoutAction::Drop
        });
    }

    pub fn peek_end(&mut self) {
        self.peek.pending = None;
        if self.peek.shown.take().is_some() {
            self.needs_redraw = true;
        }
    }

    /// How visible a window is while peeking (1 = as usual), and whether a peek is still
    /// fading in.
    pub fn peek_alpha(&self, surface: &WlSurface) -> (f32, bool) {
        let Some((at, since)) = &self.peek.shown else { return (1.0, false) };
        if matches!(at, PeekAt::Window(s) if s == surface) {
            return (1.0, false);
        }
        let e = (since.elapsed().as_secs_f32() / PEEK_FADE.as_secs_f32()).min(1.0);
        let e = if self.config.animations { e } else { 1.0 };
        (1.0 - e, e < 1.0)
    }

    /// The window being peeked at, which is shown even if it's minimized.
    pub fn peeked_window(&self) -> Option<&WlSurface> {
        match &self.peek.shown {
            Some((PeekAt::Window(s), _)) => Some(s),
            _ => None,
        }
    }
}
