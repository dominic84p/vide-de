//! Windoze 10's volume and brightness overlay: press a volume or brightness key and a
//! narrow panel in the top-left corner shows the new level for a moment, then fades.
//! Only keys show it; dragging the tray slider already shows the level in the flyout.

use std::time::{Duration, Instant};

use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    reexports::calloop::timer::{TimeoutAction, Timer},
    utils::{Logical, Point},
};

use crate::{canvas::Canvas, state::State, theme::th};

/// How long it stays fully shown after the last key press, then how long it fades.
const SHOW: Duration = Duration::from_millis(1600);
const FADE: Duration = Duration::from_millis(220);
pub const W: i32 = 48;
pub const H: i32 = 150;
const MARGIN: i32 = 56;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OsdKind {
    Volume,
    Brightness,
}

#[derive(Default)]
pub struct Osd {
    shown: Option<(OsdKind, Instant)>,
    /// The picture and what it shows: (kind, level in percent, muted, light theme).
    picture: Option<((OsdKind, u32, bool, bool), MemoryRenderBuffer)>,
}

impl State {
    /// A volume or brightness key was pressed: show (or keep showing) the overlay.
    pub fn show_osd(&mut self, kind: OsdKind) {
        let now = Instant::now();
        self.osd.shown = Some((kind, now));
        // Wake up to fade it, and again once it's gone; a newer press makes these no-ops.
        for after in [SHOW, SHOW + FADE / 2, SHOW + FADE] {
            let _ = self.loop_handle.insert_source(Timer::from_duration(after), |_, _, state| {
                state.needs_redraw = true;
                TimeoutAction::Drop
            });
        }
        self.needs_redraw = true;
    }

    /// The overlay's picture, where it goes, and how opaque it is; `None` when hidden.
    pub fn osd_picture(&mut self) -> Option<(MemoryRenderBuffer, Point<i32, Logical>, f32)> {
        let (kind, since) = self.osd.shown?;
        let age = since.elapsed();
        if age >= SHOW + FADE || self.tray.flyout.is_some() {
            self.osd.shown = None;
            return None;
        }
        let alpha = if age <= SHOW { 1.0 } else { 1.0 - (age - SHOW).as_secs_f32() / FADE.as_secs_f32() };
        if age > SHOW {
            self.needs_redraw = true;
        }
        let (level, muted) = match kind {
            OsdKind::Volume => (self.tray.volume, self.tray.muted),
            OsdKind::Brightness => (self.tray.brightness, false),
        };
        let key = (kind, (level * 100.0).round() as u32, muted, crate::theme::light());
        if self.osd.picture.as_ref().is_none_or(|(k, _)| *k != key) {
            self.osd.picture = Some((key, draw(self.font.as_ref(), key.0, level, muted)));
        }
        let buf = self.osd.picture.as_ref().unwrap().1.clone();
        Some((buf, Point::from((MARGIN, MARGIN)), alpha))
    }
}

fn draw(font: Option<&fontdue::Font>, kind: OsdKind, level: f32, muted: bool) -> MemoryRenderBuffer {
    let mut c = Canvas::new(W, H);
    c.fill(th().panel);
    c.outline(0, 0, W, H, th().border);
    let cx = W as f32 / 2.0;
    match kind {
        OsdKind::Volume => crate::tray::speaker(&mut c, cx - 5.0, 22.0, muted, level, th().text),
        OsdKind::Brightness => crate::tray::sun(&mut c, cx, 22.0, th().text),
    }
    // A vertical slider: track, filled part, and a thumb at the level.
    let (top, bottom) = (44, H - 34);
    let track_x = W / 2 - 2;
    c.rect(track_x, top, 4, bottom - top, th().divider);
    let shown = if muted { 0.0 } else { level.clamp(0.0, 1.0) };
    let fill = ((bottom - top) as f32 * shown).round() as i32;
    c.rect(track_x, bottom - fill, 4, fill, th().accent_light);
    c.round_rect(W / 2 - 4, bottom - fill - 8, 8, 16, 4, th().text);
    if let Some(f) = font {
        let s = ((level * 100.0).round() as u32).to_string();
        let tw = crate::canvas::measure(f, 15.0, &s);
        c.text(f, 15.0, (W - tw) / 2, H - 28, 22, th().text, &s);
    }
    c.into_opaque_buffer()
}
