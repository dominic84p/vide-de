//! Taskbar thumbnails: rest the pointer on a running app's taskbar button and, like Windoze,
//! a strip of live previews of its windows rises above it. Click one to switch to it; its ✕
//! closes it. Moving onto another button swaps the strip at once; moving away hides it.

use std::time::{Duration, Instant};

use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    reexports::{
        calloop::timer::{TimeoutAction, Timer},
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Point, Rectangle},
};

use crate::{
    canvas::{ellipsize, Canvas},
    state::State,
    theme::th,
    ui::{TaskbarHit, TASKBAR_H},
};

/// How long the pointer rests on a button before the strip shows.
const DELAY: Duration = Duration::from_millis(450);
pub const TILE_W: i32 = 212;
const TILE_H: i32 = 156;
const TITLE_H: i32 = 30;
const PAD: i32 = 6;
pub const THUMB: (i32, i32) = (TILE_W - 2 * 12, TILE_H - TITLE_H - 12);
const CLOSE: i32 = 30;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PreviewHit {
    Window(usize),
    Close(usize),
}

#[derive(Default)]
pub struct Preview {
    /// The taskbar button (by index) the pointer rests on, its windows, and since when.
    pending: Option<(usize, Vec<WlSurface>, Instant)>,
    /// Showing: which button, and the windows it shows.
    pub shown: Option<(usize, Vec<WlSurface>, Point<i32, Logical>)>,
    pub hover: Option<PreviewHit>,
    picture: Option<(String, MemoryRenderBuffer)>,
}

impl State {
    /// Called on pointer motion with what's under the pointer.
    pub fn preview_track(&mut self, button: Option<(usize, Point<f64, Logical>)>, over_strip: bool) {
        if over_strip {
            self.preview.pending = None;
            return;
        }
        let busy = self.start.open || self.menu.is_some() || self.tray.flyout.is_some() || self.action_center.open || self.desktops.open;
        let Some((i, pos)) = button.filter(|_| !busy) else {
            self.preview_hide();
            return;
        };
        let windows = self.taskbar.as_ref().and_then(|t| t.key.items.get(i)).map(|it| it.windows.clone()).unwrap_or_default();
        if windows.is_empty() {
            self.preview_hide();
            return;
        }
        let centre_x = self.taskbar_button_centre(i).unwrap_or(pos.x as i32);
        match &self.preview.shown {
            // Already showing for another button: Windoze swaps straight away.
            Some((shown, ..)) if *shown != i => self.preview_show(i, windows, centre_x),
            Some(_) => {}
            None => {
                if self.preview.pending.as_ref().is_none_or(|(p, ..)| *p != i) {
                    self.preview.pending = Some((i, windows, Instant::now()));
                    let _ = self.loop_handle.insert_source(Timer::from_duration(DELAY), move |_, _, state| {
                        if let Some((p, w, t0)) = state.preview.pending.clone() {
                            if p == i && t0.elapsed() >= DELAY - Duration::from_millis(5) {
                                state.preview.pending = None;
                                state.preview_show(i, w, centre_x);
                            }
                        }
                        TimeoutAction::Drop
                    });
                }
            }
        }
    }

    fn taskbar_button_centre(&self, i: usize) -> Option<i32> {
        let bar = self.taskbar.as_ref()?;
        bar.hits.iter().find(|(_, h)| *h == TaskbarHit::Button(i)).map(|(r, _)| r.loc.x + r.size.w / 2)
    }

    fn preview_show(&mut self, i: usize, windows: Vec<WlSurface>, centre_x: i32) {
        let w = strip_width(windows.len());
        let x = (centre_x - w / 2).clamp(0, (self.output_size.w - w).max(0));
        let y = self.output_size.h - TASKBAR_H - TILE_H - 2 * PAD;
        self.preview.shown = Some((i, windows, Point::from((x, y))));
        self.preview.hover = None;
        self.needs_redraw = true;
    }

    pub fn preview_hide(&mut self) {
        self.preview.pending = None;
        if self.preview.shown.take().is_some() {
            self.preview.hover = None;
            self.needs_redraw = true;
        }
    }

    pub fn preview_rect(&self) -> Option<Rectangle<i32, Logical>> {
        let (_, windows, at) = self.preview.shown.as_ref()?;
        Some(Rectangle::new(*at, (strip_width(windows.len()), TILE_H + 2 * PAD).into()))
    }

    /// What in the strip is under `p`, if the strip is there at all.
    pub fn preview_at(&self, p: Point<f64, Logical>) -> Option<Option<PreviewHit>> {
        let r = self.preview_rect()?;
        if !r.to_f64().contains(p) {
            return None;
        }
        let local: Point<i32, Logical> = (p - r.loc.to_f64()).to_i32_round();
        let n: usize = self.preview.shown.as_ref().map_or(0, |(_, w, _)| w.len());
        let i = ((local.x - PAD) / TILE_W).clamp(0, n as i32 - 1) as usize;
        let tile_x = PAD + i as i32 * TILE_W;
        if local.y < PAD || local.y >= PAD + TILE_H {
            return Some(None);
        }
        let close = Rectangle::new((tile_x + TILE_W - CLOSE, PAD).into(), (CLOSE, TITLE_H).into());
        Some(Some(if close.contains(local) { PreviewHit::Close(i) } else { PreviewHit::Window(i) }))
    }

    pub fn preview_click(&mut self, hit: PreviewHit) {
        let Some((_, windows, _)) = self.preview.shown.clone() else { return };
        match hit {
            PreviewHit::Window(i) => {
                if let Some(s) = windows.get(i) {
                    self.preview_hide();
                    self.focus(Some(s.clone()), smithay::utils::SERIAL_COUNTER.next_serial());
                }
            }
            PreviewHit::Close(i) => {
                if let Some(w) = windows.get(i).and_then(|s| self.wm.find(s)) {
                    w.close();
                }
                self.preview_hide();
            }
        }
    }

    /// The strip's background and titles, and where each window's live thumbnail goes.
    pub fn preview_picture(&mut self) -> Option<(MemoryRenderBuffer, Point<i32, Logical>, Vec<(WlSurface, Rectangle<i32, Logical>)>)> {
        let (_, windows, at) = self.preview.shown.clone()?;
        // Windows that closed meanwhile drop out; none left, nothing to show.
        let windows: Vec<WlSurface> = windows.into_iter().filter(|s| self.wm.find(s).is_some()).collect();
        if windows.is_empty() {
            self.preview_hide();
            return None;
        }
        let titles: Vec<String> = windows.iter().map(|s| self.wm.find(s).map(|w| w.title()).unwrap_or_default()).collect();
        let focused = self.focused();
        let active: Vec<bool> = windows.iter().map(|s| Some(s) == focused.as_ref()).collect();
        let key = format!("{titles:?}{active:?}{:?}{}", self.preview.hover, crate::theme::light());
        if self.preview.picture.as_ref().is_none_or(|(k, _)| *k != key) {
            self.preview.picture = Some((key, draw(self.font.as_ref(), &titles, &active, self.preview.hover)));
        }
        let buf = self.preview.picture.as_ref().unwrap().1.clone();
        let thumbs = windows
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                let loc = at + Point::from((PAD + i as i32 * TILE_W + 12, PAD + TITLE_H));
                (s, Rectangle::new(loc, THUMB.into()))
            })
            .collect();
        Some((buf, at, thumbs))
    }
}

fn strip_width(n: usize) -> i32 {
    n as i32 * TILE_W + 2 * PAD
}

fn draw(font: Option<&fontdue::Font>, titles: &[String], active: &[bool], hover: Option<PreviewHit>) -> MemoryRenderBuffer {
    let mut c = Canvas::new(strip_width(titles.len()), TILE_H + 2 * PAD);
    c.fill(th().taskbar);
    for (i, title) in titles.iter().enumerate() {
        let x = PAD + i as i32 * TILE_W;
        let hovered = matches!(hover, Some(PreviewHit::Window(j) | PreviewHit::Close(j)) if j == i);
        if hovered || active[i] {
            c.rect(x, PAD, TILE_W, TILE_H, if hovered { th().hover } else { th().active });
        }
        if let Some(f) = font {
            let t = ellipsize(f, 12.0, title, TILE_W - 24 - CLOSE);
            c.text(f, 12.0, x + 12, PAD, TITLE_H, th().text, &t);
        }
        if hovered {
            let cx = x + TILE_W - CLOSE;
            if hover == Some(PreviewHit::Close(i)) {
                c.rect(cx, PAD, CLOSE, TITLE_H, crate::canvas::hex(0xe81123));
            }
            let (mx, my) = (cx as f32 + CLOSE as f32 / 2.0, PAD as f32 + TITLE_H as f32 / 2.0);
            c.line(mx - 5.0, my - 5.0, mx + 5.0, my + 5.0, 1.2, th().text);
            c.line(mx + 5.0, my - 5.0, mx - 5.0, my + 5.0, 1.2, th().text);
        }
    }
    c.into_opaque_buffer()
}
