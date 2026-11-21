//! Snap assist, from Windoze 10: snap a window to one side and the rest of the screen offers
//! your other windows as thumbnails. Click one and it fills that side; Esc or a click anywhere
//! else leaves it empty.

use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER},
};

use crate::{
    canvas::{ellipsize, Canvas},
    state::State,
    theme::th,
};

const TILE_W: i32 = 240;
const TILE_H: i32 = 170;
const TITLE_H: i32 = 30;
const GAP: i32 = 16;

pub struct Assist {
    /// The empty part of the screen, and what fills it.
    pub zone: Rectangle<i32, Logical>,
    area: Rectangle<i32, Logical>,
    pub windows: Vec<WlSurface>,
    pub hover: Option<usize>,
    picture: Option<(String, MemoryRenderBuffer)>,
}

impl Assist {
    fn cols(&self) -> usize {
        (((self.zone.size.w - GAP) / (TILE_W + GAP)).max(1) as usize).min(self.windows.len().max(1))
    }

    /// Tile `i`, relative to the zone, centred in it.
    fn tile(&self, i: usize) -> Rectangle<i32, Logical> {
        let cols = self.cols();
        let rows = self.windows.len().div_ceil(cols);
        let w = cols as i32 * (TILE_W + GAP) - GAP;
        let h = rows as i32 * (TILE_H + GAP) - GAP;
        let x0 = ((self.zone.size.w - w) / 2).max(GAP);
        let y0 = ((self.zone.size.h - h) / 2).max(GAP);
        let (c, r) = ((i % cols) as i32, (i / cols) as i32);
        Rectangle::new((x0 + c * (TILE_W + GAP), y0 + r * (TILE_H + GAP)).into(), (TILE_W, TILE_H).into())
    }
}

impl State {
    /// A window was just snapped into `zone` of `area`: offer the rest of the screen, if it's
    /// one clean side (halves and columns; quarters are left alone, as on Windoze 10).
    pub fn snap_assist_offer(&mut self, snapped: &WlSurface, zone: Rectangle<i32, Logical>, area: Rectangle<i32, Logical>) {
        if zone.size.h != area.size.h || zone.size.w >= area.size.w {
            return;
        }
        let rest = if zone.loc.x == area.loc.x {
            Rectangle::new((zone.loc.x + zone.size.w, area.loc.y).into(), (area.size.w - zone.size.w, area.size.h).into())
        } else if zone.loc.x + zone.size.w == area.loc.x + area.size.w {
            Rectangle::new(area.loc, (area.size.w - zone.size.w, area.size.h).into())
        } else {
            return;
        };
        let ws = self.wm.current_workspace;
        let windows: Vec<WlSurface> = self
            .wm
            .windows
            .iter()
            .rev()
            .filter(|w| w.mapped && w.workspace == ws && w.surface() != snapped && w.fullscreen.is_none() && self.wm.parent_surface(w).is_none())
            .map(|w| w.surface().clone())
            .collect();
        if windows.is_empty() {
            return;
        }
        self.snap_assist = Some(Assist { zone: rest, area, windows, hover: None, picture: None });
        self.needs_redraw = true;
    }

    pub fn snap_assist_close(&mut self) {
        if self.snap_assist.take().is_some() {
            self.needs_redraw = true;
        }
    }

    /// Which thumbnail is under `p`, if the assist is up and `p` is in its zone.
    pub fn snap_assist_at(&self, p: Point<f64, Logical>) -> Option<Option<usize>> {
        let a = self.snap_assist.as_ref()?;
        if !a.zone.to_f64().contains(p) {
            return None;
        }
        let local = (p - a.zone.loc.to_f64()).to_i32_round();
        Some((0..a.windows.len()).find(|&i| a.tile(i).contains(local)))
    }

    pub fn snap_assist_pick(&mut self, i: usize) {
        let Some(a) = self.snap_assist.take() else { return };
        self.needs_redraw = true;
        let Some(s) = a.windows.get(i).cloned() else { return };
        if self.wm.find(&s).is_some_and(|w| w.minimized) {
            self.focus(Some(s.clone()), SERIAL_COUNTER.next_serial());
        }
        self.wm.unmaximize(&s);
        self.wm.snap(&s, a.zone, a.area);
        self.focus(Some(s), SERIAL_COUNTER.next_serial());
    }

    /// The backdrop and titles, its place, and where each live thumbnail goes.
    pub fn snap_assist_picture(&mut self) -> Option<(MemoryRenderBuffer, Point<i32, Logical>, Vec<(WlSurface, Rectangle<i32, Logical>)>)> {
        let a = self.snap_assist.as_ref()?;
        // Windows that closed meanwhile drop out.
        let alive: Vec<WlSurface> = a.windows.iter().filter(|s| self.wm.find(s).is_some()).cloned().collect();
        if alive.is_empty() {
            self.snap_assist_close();
            return None;
        }
        let titles: Vec<String> = alive.iter().map(|s| self.wm.find(s).map(|w| w.title()).unwrap_or_default()).collect();
        let font = self.font.clone();
        let a = self.snap_assist.as_mut().unwrap();
        a.windows = alive;
        let key = format!("{:?}{titles:?}{:?}{}", a.zone, a.hover, crate::theme::light());
        if a.picture.as_ref().is_none_or(|(k, _)| *k != key) {
            let mut c = Canvas::new(a.zone.size.w, a.zone.size.h);
            c.fill(crate::canvas::Rgba(0, 0, 0, 110));
            for (i, title) in titles.iter().enumerate() {
                let t = a.tile(i);
                let bg = if a.hover == Some(i) { th().card_hover } else { th().card };
                c.round_rect(t.loc.x, t.loc.y, t.size.w, t.size.h, 4, bg);
                if a.hover == Some(i) {
                    c.outline(t.loc.x, t.loc.y, t.size.w, t.size.h, crate::theme::ACCENT);
                }
                if let Some(f) = font.as_ref() {
                    c.text(f, 12.0, t.loc.x + 10, t.loc.y, TITLE_H, th().text, &ellipsize(f, 12.0, title, TILE_W - 20));
                }
            }
            a.picture = Some((key, c.into_buffer()));
        }
        let thumbs = (0..a.windows.len())
            .map(|i| {
                let t = a.tile(i);
                let r = Rectangle::new(a.zone.loc + t.loc + Point::from((8, TITLE_H)), (TILE_W - 16, TILE_H - TITLE_H - 8).into());
                (a.windows[i].clone(), r)
            })
            .collect();
        Some((a.picture.as_ref().unwrap().1.clone(), a.zone.loc, thumbs))
    }
}
