//! Snap layouts: rest the pointer on a window's maximize button and, as on Windoze 11, a panel
//! of layouts drops below it. Click a zone in one and the window snaps there.

use std::time::{Duration, Instant};

use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    reexports::{
        calloop::timer::{TimeoutAction, Timer},
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER},
};

use crate::{
    canvas::Canvas,
    state::{Hover, State},
    theme::{th, ACCENT},
    ui::{TitleButton, TITLE_BTN_W},
};

/// How long the pointer rests on maximize before the panel shows.
const DELAY: Duration = Duration::from_millis(500);
const THUMB_W: i32 = 96;
const THUMB_H: i32 = 60;
const GAP: i32 = 12;
const PER_ROW: usize = 3;
const W: i32 = PER_ROW as i32 * THUMB_W + (PER_ROW as i32 - 1) * GAP + 2 * GAP;

/// Each layout's zones as fractions of the work area: (x, y, w, h).
const LAYOUTS: &[&[(f32, f32, f32, f32)]] = &[
    &[(0.0, 0.0, 0.5, 1.0), (0.5, 0.0, 0.5, 1.0)],
    &[(0.0, 0.0, 2.0 / 3.0, 1.0), (2.0 / 3.0, 0.0, 1.0 / 3.0, 1.0)],
    &[(0.0, 0.0, 1.0 / 3.0, 1.0), (1.0 / 3.0, 0.0, 1.0 / 3.0, 1.0), (2.0 / 3.0, 0.0, 1.0 / 3.0, 1.0)],
    &[(0.0, 0.0, 0.5, 0.5), (0.5, 0.0, 0.5, 0.5), (0.0, 0.5, 0.5, 0.5), (0.5, 0.5, 0.5, 0.5)],
    &[(0.0, 0.0, 0.5, 1.0), (0.5, 0.0, 0.5, 0.5), (0.5, 0.5, 0.5, 0.5)],
    &[(0.0, 0.0, 0.25, 1.0), (0.25, 0.0, 0.5, 1.0), (0.75, 0.0, 0.25, 1.0)],
];

fn height() -> i32 {
    let rows = LAYOUTS.len().div_ceil(PER_ROW) as i32;
    rows * THUMB_H + (rows - 1) * GAP + 2 * GAP
}

/// Every zone of every layout: its fractions, and where it's drawn in the panel.
fn zones() -> Vec<((f32, f32, f32, f32), Rectangle<i32, Logical>)> {
    let mut out = Vec::new();
    for (i, layout) in LAYOUTS.iter().enumerate() {
        let tx = GAP + (i % PER_ROW) as i32 * (THUMB_W + GAP);
        let ty = GAP + (i / PER_ROW) as i32 * (THUMB_H + GAP);
        for &(x, y, w, h) in layout.iter() {
            // A 2px gutter between zones, so each reads as its own target.
            let r = Rectangle::new(
                (tx + (x * THUMB_W as f32).round() as i32 + 1, ty + (y * THUMB_H as f32).round() as i32 + 1).into(),
                ((w * THUMB_W as f32).round() as i32 - 2, (h * THUMB_H as f32).round() as i32 - 2).into(),
            );
            out.push(((x, y, w, h), r));
        }
    }
    out
}

pub struct SnapLayouts {
    pub surface: WlSurface,
    pub at: Point<i32, Logical>,
    pub hover: Option<usize>,
    picture: Option<(Option<usize>, MemoryRenderBuffer)>,
}

#[derive(Default)]
pub struct SnapState {
    pub shown: Option<SnapLayouts>,
    /// The window whose maximize button the pointer rests on, and since when.
    pending: Option<(WlSurface, Instant)>,
}

impl State {
    pub fn snap_layouts_rect(&self) -> Option<Rectangle<i32, Logical>> {
        let s = self.snap.shown.as_ref()?;
        Some(Rectangle::new(s.at, (W, height()).into()))
    }

    /// Which zone is under `p`, if the panel is there at all.
    pub fn snap_layouts_at(&self, p: Point<f64, Logical>) -> Option<Option<usize>> {
        let r = self.snap_layouts_rect()?;
        if !r.to_f64().contains(p) {
            return None;
        }
        let local = (p - r.loc.to_f64()).to_i32_round();
        Some(zones().iter().position(|(_, z)| z.contains(local)))
    }

    /// Called on pointer motion once `self.hover` is up to date.
    pub fn snap_layouts_track(&mut self, pos: Point<f64, Logical>) {
        let on_max = match &self.hover {
            Hover::Title(s, TitleButton::Maximize) => Some(s.clone()),
            _ => None,
        };
        if let Some(over) = self.snap_layouts_at(pos) {
            let shown = self.snap.shown.as_mut().unwrap();
            if shown.hover != over {
                shown.hover = over;
                self.needs_redraw = true;
            }
            return;
        }
        if let Some(shown) = &self.snap.shown {
            if on_max.as_ref() != Some(&shown.surface) {
                self.snap_layouts_close();
            }
            return;
        }
        let Some(s) = on_max else {
            self.snap.pending = None;
            return;
        };
        if self.snap.pending.as_ref().is_some_and(|(p, _)| *p == s) {
            return;
        }
        self.snap.pending = Some((s.clone(), Instant::now()));
        let _ = self.loop_handle.insert_source(Timer::from_duration(DELAY), move |_, _, state| {
            if let Some((p, t0)) = state.snap.pending.clone() {
                if p == s && t0.elapsed() >= DELAY - Duration::from_millis(5) {
                    state.snap.pending = None;
                    state.snap_layouts_open(&p);
                }
            }
            TimeoutAction::Drop
        });
    }

    fn snap_layouts_open(&mut self, s: &WlSurface) {
        let busy = self.start.open || self.menu.is_some() || self.wm.grab.is_some();
        let Some(bar) = self.wm.find(s).and_then(|w| w.titlebar_rect()) else { return };
        if busy {
            return;
        }
        // Centred under the maximize button, kept on the window's screen.
        let button_x = bar.loc.x + bar.size.w - TITLE_BTN_W * 3 / 2;
        let screen = self.work_area_of(s);
        let x = (button_x - W / 2).clamp(screen.loc.x, (screen.loc.x + screen.size.w - W).max(screen.loc.x));
        let y = (bar.loc.y + bar.size.h).clamp(screen.loc.y, (screen.loc.y + screen.size.h - height()).max(screen.loc.y));
        self.snap.shown = Some(SnapLayouts { surface: s.clone(), at: (x, y).into(), hover: None, picture: None });
        self.needs_redraw = true;
    }

    pub fn snap_layouts_close(&mut self) {
        self.snap.pending = None;
        if self.snap.shown.take().is_some() {
            self.needs_redraw = true;
        }
    }

    pub fn snap_layouts_click(&mut self, zone: usize) {
        let Some(shown) = self.snap.shown.take() else { return };
        self.needs_redraw = true;
        let Some(&((x, y, w, h), _)) = zones().get(zone) else { return };
        let s = shown.surface;
        let area = self.work_area_of(&s);
        let (aw, ah) = (area.size.w as f32, area.size.h as f32);
        let rect = Rectangle::new(
            (area.loc.x + (x * aw).round() as i32, area.loc.y + (y * ah).round() as i32).into(),
            ((w * aw).round() as i32, (h * ah).round() as i32).into(),
        );
        self.wm.unmaximize(&s);
        self.wm.snap(&s, rect, area);
        self.focus(Some(s), SERIAL_COUNTER.next_serial());
    }

    pub fn snap_layouts_picture(&mut self) -> Option<(MemoryRenderBuffer, Point<i32, Logical>)> {
        let shown = self.snap.shown.as_mut()?;
        if shown.picture.as_ref().is_none_or(|(h, _)| *h != shown.hover) {
            shown.picture = Some((shown.hover, draw(shown.hover)));
        }
        Some((shown.picture.as_ref().unwrap().1.clone(), shown.at))
    }
}

fn draw(hover: Option<usize>) -> MemoryRenderBuffer {
    let mut c = Canvas::new(W, height());
    c.fill(th().panel);
    c.outline(0, 0, W, height(), th().border);
    for (i, (_, r)) in zones().iter().enumerate() {
        let fill = if hover == Some(i) { ACCENT } else { th().button };
        c.round_rect(r.loc.x, r.loc.y, r.size.w, r.size.h, 3, fill);
    }
    c.into_buffer()
}
