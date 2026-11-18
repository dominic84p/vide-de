//! Windoze 10's animations, a little livelier: windows zoom up when they open, fly into their
//! taskbar button when minimized and back out of it when restored, and glide between sizes
//! when maximized or snapped. Start, the Action Center, flyouts and menus slide or grow in and
//! fade back out; Task View gathers the windows into its grid; switching desktops slides.
//! Settings → Ease of Access → Display → Show animations (`animations=false`) turns them all off.
//!
//! Nothing here keeps its own timer. `render` works out how far along each animation is from
//! when it began, and while any is still running it asks for another frame
//! (`State::needs_redraw`), so the backends draw every vblank until it settles. Input never
//! waits for an animation: a closing panel is already closed as far as clicks go.

use std::time::Instant;

use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use smithay::utils::{Logical, Point, Rectangle};

/// Durations, in milliseconds. Long enough to see where things go, short enough that nothing
/// ever feels like a wait.
pub const WINDOW_OPEN: u64 = 220;
pub const WINDOW_MINIMIZE: u64 = 240;
pub const WINDOW_RESTORE: u64 = 240;
/// Maximize, restore and snap: the window glides from its old rectangle to its new one.
pub const WINDOW_MORPH: u64 = 190;
pub const START: u64 = 230;
pub const ACTION_CENTER: u64 = 240;
pub const FLYOUT: u64 = 180;
pub const TASK_VIEW: u64 = 260;
pub const MENU: u64 = 130;
pub const SWITCHER: u64 = 130;
pub const TOASTS: u64 = 230;
pub const DESKTOP_SWITCH: u64 = 230;
/// Panels fade out faster than they come in, so a closed one never seems to linger.
pub const CLOSE: u64 = 140;

/// Fast start, gentle landing.
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Even faster start, for things that should feel snappy (menus, fades).
pub fn ease_out_quint(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(5)
}

/// Overshoots its target slightly and settles back: the little pop of a window opening.
pub fn ease_out_back(t: f32) -> f32 {
    const C1: f32 = 1.3;
    const C3: f32 = C1 + 1.0;
    let t = t.clamp(0.0, 1.0) - 1.0;
    1.0 + C3 * t.powi(3) + C1 * t.powi(2)
}

/// Slow start and end, for things travelling a long way (minimize into the taskbar).
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

/// Linear progress (0..=1) of an animation that began at `since` and lasts `ms`.
pub fn progress(since: Instant, ms: u64) -> f32 {
    (since.elapsed().as_secs_f32() * 1000.0 / ms as f32).min(1.0)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn lerp_rect(a: Rectangle<i32, Logical>, b: Rectangle<i32, Logical>, t: f32) -> Rectangle<i32, Logical> {
    let l = |x: i32, y: i32| lerp(x as f32, y as f32, t).round() as i32;
    Rectangle::new((l(a.loc.x, b.loc.x), l(a.loc.y, b.loc.y)).into(), (l(a.size.w, b.size.w), l(a.size.h, b.size.h)).into())
}

/// Where a panel is in its life.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    /// Open, `e` (eased, 0..=1) of the way in; 1.0 once settled.
    Open(f32),
    /// Closed but still fading away from the picture it last showed; `e` goes 1.0 → 0.0.
    Closing(f32),
    Hidden,
}

impl Phase {
    /// How far "in" it is: 1.0 fully shown, 0.0 gone.
    pub fn e(self) -> f32 {
        match self {
            Phase::Open(e) | Phase::Closing(e) => e,
            Phase::Hidden => 0.0,
        }
    }

    pub fn moving(self) -> bool {
        matches!(self, Phase::Open(e) if e < 1.0) || matches!(self, Phase::Closing(_))
    }
}

/// Notices something opening or closing, to animate it. It also keeps the last picture the
/// panel drew, so a closed panel can fade out from it after its own state is gone.
#[derive(Default)]
pub struct Edge {
    open: bool,
    since: Option<Instant>,
    closed_at: Option<Instant>,
    /// What it last looked like and where, for the closing fade.
    pub last: Option<(MemoryRenderBuffer, Point<i32, Logical>)>,
}

impl Edge {
    /// Where something that is now `open` is in its life. `ease` shapes the way in.
    pub fn update(&mut self, open: bool, ms: u64, ease: fn(f32) -> f32) -> Phase {
        if open && !self.open {
            self.since = Some(Instant::now());
            self.closed_at = None;
        }
        if !open && self.open {
            // Only fade out what had finished appearing; a panel closed mid-opening just goes.
            self.closed_at = self.last.is_some().then(Instant::now);
            self.since = None;
        }
        self.open = open;
        if !open {
            let Some(t0) = self.closed_at else {
                self.last = None;
                return Phase::Hidden;
            };
            let p = progress(t0, CLOSE);
            if p >= 1.0 {
                self.closed_at = None;
                self.last = None;
                return Phase::Hidden;
            }
            return Phase::Closing(1.0 - ease_out(p));
        }
        let Some(since) = self.since else { return Phase::Open(1.0) };
        let p = progress(since, ms);
        if p >= 1.0 {
            self.since = None;
        }
        Phase::Open(ease(p))
    }

    /// Forget everything: animations were just turned off.
    pub fn reset(&mut self, open: bool) {
        *self = Edge { open, ..Default::default() };
    }
}

#[derive(Default)]
pub struct Anims {
    pub start: Edge,
    pub action_center: Edge,
    pub flyout: Edge,
    pub task_view: Edge,
    pub menu: Edge,
    pub switcher: Edge,
    pub toasts: Edge,
    /// Switching desktops: when, and which way the new one slides in from (-1 left, 1 right).
    pub desktop_switch: Option<(Instant, i32)>,
    pub last_workspace: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easings_start_at_zero_and_land_on_one() {
        for f in [ease_out, ease_out_quint, ease_out_back, ease_in_out] {
            assert!(f(0.0).abs() < 1e-5);
            assert!((f(1.0) - 1.0).abs() < 1e-5);
        }
        // The pop overshoots, but only a little.
        let peak = (0..=100).map(|i| ease_out_back(i as f32 / 100.0)).fold(0.0, f32::max);
        assert!(peak > 1.0 && peak < 1.12, "{peak}");
    }

    #[test]
    fn a_panel_closed_before_it_drew_just_vanishes() {
        let mut e = Edge::default();
        assert!(matches!(e.update(true, 100, ease_out), Phase::Open(x) if x < 0.5));
        assert_eq!(e.update(false, 100, ease_out), Phase::Hidden);
    }
}
