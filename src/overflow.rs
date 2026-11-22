//! The tray's overflow, as on Windoze 10: apps' tray icons (Discord, Steam...) live behind the
//! ^ chevron left of our own, in a little grid that opens above it. Click one to open the app,
//! right-click for its menu; a click anywhere else or Esc puts the grid away.

use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    utils::{Logical, Point, Rectangle},
};

use crate::{canvas::Canvas, state::State, theme::th, ui::TASKBAR_H};

const CELL: i32 = 40;
const PAD: i32 = 6;
const COLS: usize = 5;

#[derive(Default)]
pub struct Overflow {
    /// Open, above the chevron centred on this x.
    pub open: Option<i32>,
    pub hover: Option<usize>,
    picture: Option<(String, MemoryRenderBuffer)>,
}

impl State {
    pub fn overflow_toggle(&mut self, chevron_x: i32) {
        self.overflow.open = if self.overflow.open.is_some() { None } else { Some(chevron_x) };
        self.overflow.hover = None;
        self.needs_redraw = true;
    }

    pub fn overflow_close(&mut self) {
        if self.overflow.open.take().is_some() {
            self.needs_redraw = true;
        }
    }

    fn overflow_grid(&self) -> (usize, usize) {
        let n = self.sni_items.len().max(1);
        let cols = n.min(COLS);
        (cols, n.div_ceil(cols))
    }

    pub fn overflow_rect(&self) -> Option<Rectangle<i32, Logical>> {
        let cx = self.overflow.open?;
        let (cols, rows) = self.overflow_grid();
        let (w, h) = (cols as i32 * CELL + 2 * PAD, rows as i32 * CELL + 2 * PAD);
        let x = (cx - w / 2).clamp(0, (self.output_size.w - w).max(0));
        Some(Rectangle::new((x, self.output_size.h - TASKBAR_H - h).into(), (w, h).into()))
    }

    /// Which icon is under `p`, if the grid is open and `p` is on it.
    pub fn overflow_at(&self, p: Point<f64, Logical>) -> Option<Option<usize>> {
        let r = self.overflow_rect()?;
        if !r.to_f64().contains(p) {
            return None;
        }
        let local: Point<i32, Logical> = (p - r.loc.to_f64()).to_i32_round();
        let (cols, _) = self.overflow_grid();
        let (c, row) = ((local.x - PAD) / CELL, (local.y - PAD) / CELL);
        if local.x < PAD || local.y < PAD || c as usize >= cols {
            return Some(None);
        }
        let i = row as usize * cols + c as usize;
        Some((i < self.sni_items.len()).then_some(i))
    }

    pub fn overflow_picture(&mut self) -> Option<(MemoryRenderBuffer, Point<i32, Logical>)> {
        let r = self.overflow_rect()?;
        let icons = crate::render::tray_icons(self);
        let key = format!(
            "{:?}{:?}{}{:?}",
            self.sni_items.iter().map(crate::render::tray_fingerprint).collect::<Vec<_>>(),
            self.overflow.hover,
            crate::theme::light(),
            r.size
        );
        if self.overflow.picture.as_ref().is_none_or(|(k, _)| *k != key) {
            let (cols, _) = self.overflow_grid();
            let mut c = Canvas::new(r.size.w, r.size.h);
            c.fill(th().panel);
            c.outline(0, 0, r.size.w, r.size.h, th().border);
            for (i, icon) in icons.iter().enumerate() {
                let (x, y) = (PAD + (i % cols) as i32 * CELL, PAD + (i / cols) as i32 * CELL);
                if self.overflow.hover == Some(i) {
                    c.rect(x, y, CELL, CELL, th().hover);
                }
                match icon {
                    Some(icon) => c.icon(icon, x + (CELL - icon.w as i32) / 2, y + (CELL - icon.h as i32) / 2),
                    None => c.round_rect(x + 12, y + 12, 16, 16, 3, crate::theme::ACCENT),
                }
            }
            self.overflow.picture = Some((key, c.into_buffer()));
        }
        Some((self.overflow.picture.as_ref().unwrap().1.clone(), r.loc))
    }
}
