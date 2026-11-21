//! Building the element list for a frame. Backends hand these to a damage
//! tracker (nested) or a DRM compositor (TTY) and only changed regions get redrawn.

use std::sync::Mutex;

use smithay::{
    backend::renderer::{
        element::{
            memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
            utils::RescaleRenderElement,
            render_elements,
            solid::{SolidColorBuffer, SolidColorRenderElement},
            surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
            Kind,
        },
        gles::GlesRenderer,
        Color32F,
    },
    desktop::PopupManager,
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Physical, Point, Rectangle},
    wayland::compositor::{with_states, with_surface_tree_downward, SurfaceAttributes, TraversalAction},
};

use smithay::wayland::shell::wlr_layer::Layer;

use crate::{
    state::{Hover, State},
    ui::{self, StartKey, TaskbarKey, MENU_W, TASKBAR_H},
};

pub const CLEAR: Color32F = Color32F::new(0.0, 0.1, 0.24, 1.0);
pub const BORDER_ACTIVE: Color32F = Color32F::new(0.0, 0.47, 0.84, 1.0);
pub const BORDER_IDLE: Color32F = Color32F::new(0.6, 0.6, 0.6, 1.0);

render_elements! {
    pub Elem<=GlesRenderer>;
    Surface=WaylandSurfaceRenderElement<GlesRenderer>,
    Solid=SolidColorRenderElement,
    /// CPU-painted pictures: cursor, taskbar, start menu, titlebars, wallpaper.
    Image=MemoryRenderBufferRenderElement<GlesRenderer>,
    /// A window drawn shrunk (Alt+Tab thumbnails, and windows zooming open or minimizing).
    Thumb=RescaleRenderElement<WaylandSurfaceRenderElement<GlesRenderer>>,
    /// A titlebar or border zooming along with its window.
    ScaledImage=RescaleRenderElement<MemoryRenderBufferRenderElement<GlesRenderer>>,
    ScaledSolid=RescaleRenderElement<SolidColorRenderElement>,
}

/// How something mid-animation is drawn: faded, scaled about `origin`, and shifted.
#[derive(Clone, Copy)]
struct Fx {
    alpha: f32,
    scale: f64,
    origin: Point<i32, Logical>,
    offset: Point<i32, Logical>,
}

impl Fx {
    fn fade(alpha: f32) -> Fx {
        Fx::slide(alpha, 0.0, 0.0)
    }

    fn slide(alpha: f32, dx: f32, dy: f32) -> Fx {
        Fx { alpha, scale: 1.0, origin: (0, 0).into(), offset: (dx.round() as i32, dy.round() as i32).into() }
    }

    fn scaled(&self) -> bool {
        (self.scale - 1.0).abs() > 1e-3
    }
}

fn surface_tree_fx(renderer: &mut GlesRenderer, v: &View, s: &WlSurface, at: Point<i32, Logical>, fx: &Fx) -> Vec<Elem> {
    let els: Vec<WaylandSurfaceRenderElement<GlesRenderer>> =
        render_elements_from_surface_tree(renderer, s, v.px(at + fx.offset), v.scale, fx.alpha, Kind::Unspecified);
    if fx.scaled() {
        let origin = v.px(fx.origin + fx.offset);
        els.into_iter().map(|e| Elem::Thumb(RescaleRenderElement::from_element(e, origin, fx.scale))).collect()
    } else {
        els.into_iter().map(Elem::Surface).collect()
    }
}

fn image_fx(renderer: &mut GlesRenderer, v: &View, buf: &MemoryRenderBuffer, at: Point<i32, Logical>, fx: &Fx) -> Option<Elem> {
    let loc = (at + fx.offset - v.loc).to_f64().to_physical(v.scale);
    let e = MemoryRenderBufferRenderElement::from_buffer(renderer, loc, buf, Some(fx.alpha), None, None, Kind::Unspecified).ok()?;
    Some(if fx.scaled() { Elem::ScaledImage(RescaleRenderElement::from_element(e, v.px(fx.origin + fx.offset), fx.scale)) } else { Elem::Image(e) })
}

fn solid_fx(v: &View, buf: &SolidColorBuffer, at: Point<i32, Logical>, fx: &Fx) -> Elem {
    let e = SolidColorRenderElement::from_buffer(buf, v.px(at + fx.offset), v.scale, fx.alpha, Kind::Unspecified);
    if fx.scaled() { Elem::ScaledSolid(RescaleRenderElement::from_element(e, v.px(fx.origin + fx.offset), fx.scale)) } else { Elem::Solid(e) }
}

/// A window's open, restore, minimize or resize animation, if one is running; clears
/// finished ones. `button` is the centre of its taskbar button, where minimizing sends it.
fn window_fx(w: &mut crate::wm::Window, on: bool, button: Option<Point<i32, Logical>>, animating: &mut bool) -> Option<Fx> {
    use crate::anim::*;
    if !on {
        w.appear = None;
        w.vanish = None;
        w.morph = None;
        return None;
    }
    let f = w.frame();
    let centre = f.loc + Point::from((f.size.w / 2, f.size.h / 2));
    // Flying into (or out of) the taskbar: `t` is how far along that trip it is, shrinking to
    // a tenth of its size about its centre while the centre travels to the button. Without a
    // button (taskbar hidden) it drops toward the bottom of the screen.
    let to_taskbar = |t: f32| {
        let target = button.unwrap_or(centre + Point::from((0, f.size.h / 2 + 60)));
        let d = target - centre;
        Fx {
            alpha: (1.0 - t * t).clamp(0.0, 1.0),
            scale: 1.0 - 0.9 * t as f64,
            origin: centre,
            offset: ((d.x as f32 * t).round() as i32, (d.y as f32 * t).round() as i32).into(),
        }
    };
    if let Some(t0) = w.vanish {
        let p = progress(t0, WINDOW_MINIMIZE);
        if p >= 1.0 {
            w.vanish = None;
            return None;
        }
        *animating = true;
        return Some(to_taskbar(ease_in_out(p)));
    }
    if let Some(t0) = w.appear {
        let restored = w.restored;
        let p = progress(t0, if restored { WINDOW_RESTORE } else { WINDOW_OPEN });
        if p >= 1.0 {
            w.appear = None;
            w.restored = false;
        } else {
            *animating = true;
            if restored {
                return Some(to_taskbar(1.0 - ease_out(p)));
            }
            // Zooms up from 82% with a little pop past full size, fading in fast and rising
            // into place.
            return Some(Fx {
                alpha: ease_out_quint(p),
                scale: (0.82 + 0.18 * ease_out_back(p)) as f64,
                origin: centre,
                offset: (0, (28.0 * (1.0 - ease_out(p))).round() as i32).into(),
            });
        }
    }
    let (t0, from) = w.morph?;
    let p = progress(t0, WINDOW_MORPH);
    if p >= 1.0 || f.size.w <= 0 {
        w.morph = None;
        return None;
    }
    *animating = true;
    // One scale for both axes (that's all a rescale element does): match the width and let
    // the height follow; for under 200 ms the difference doesn't show.
    let r = lerp_rect(from, f, ease_out(p));
    Some(Fx { alpha: 1.0, scale: r.size.w as f64 / f.size.w as f64, origin: f.loc, offset: r.loc - f.loc })
}

/// Draw a panel through its open or close animation. While open it shows `now` (and
/// remembers it); once closed it fades out from the last picture it showed.
fn panel_fx(
    renderer: &mut GlesRenderer,
    v: &View,
    out: &mut Vec<Elem>,
    edge: &mut crate::anim::Edge,
    phase: crate::anim::Phase,
    now: Option<(MemoryRenderBuffer, Point<i32, Logical>)>,
    fx: impl Fn(f32, Point<i32, Logical>) -> Fx,
) {
    use crate::anim::Phase;
    let shown = match phase {
        Phase::Open(_) => {
            edge.last = now.clone();
            now
        }
        Phase::Closing(_) => edge.last.clone(),
        Phase::Hidden => None,
    };
    if let Some((buf, at)) = shown {
        out.extend(image_fx(renderer, v, &buf, at, &fx(phase.e(), at)));
    }
}

pub type BorderPiece<'a> = (&'a SolidColorBuffer, Point<i32, Logical>);

/// The part of the global layout a frame shows (one monitor, or all of them nested) and
/// at what scale. Elements are placed relative to its corner.
#[derive(Clone, Copy)]
pub struct View {
    pub loc: Point<i32, Logical>,
    pub scale: f64,
}

impl View {
    fn px(&self, at: Point<i32, Logical>) -> Point<i32, Physical> {
        (at - self.loc).to_physical_precise_round(self.scale)
    }
}

fn image(renderer: &mut GlesRenderer, v: &View, buf: &MemoryRenderBuffer, at: Point<i32, Logical>, kind: Kind) -> Option<Elem> {
    MemoryRenderBufferRenderElement::from_buffer(renderer, (at - v.loc).to_f64().to_physical(v.scale), buf, None, None, None, kind)
        .ok()
        .map(Elem::Image)
}

fn surface_tree(renderer: &mut GlesRenderer, v: &View, s: &WlSurface, at: Point<i32, Logical>, kind: Kind) -> Vec<Elem> {
    render_elements_from_surface_tree(renderer, s, v.px(at), v.scale, 1.0, kind)
        .into_iter()
        .map(Elem::Surface)
        .collect()
}

/// A window drawn live, shrunk to fit `area` (never enlarged) and centred in it.
fn live_thumbnail(renderer: &mut GlesRenderer, v: &View, state: &State, s: &WlSurface, area: Rectangle<i32, Logical>, alpha: f32) -> Vec<Elem> {
    let Some(w) = state.wm.find(s) else { return Vec::new() };
    let g = w.geometry();
    if g.size.w <= 0 || g.size.h <= 0 {
        return Vec::new();
    }
    let scale = (area.size.w as f64 / g.size.w as f64).min(area.size.h as f64 / g.size.h as f64).min(1.0);
    let (tw, th) = ((g.size.w as f64 * scale) as i32, (g.size.h as f64 * scale) as i32);
    // Shift by the geometry offset so client shadows sit outside the thumbnail.
    let off = w.surface_loc() - w.pos;
    let at = area.loc + Point::from(((area.size.w - tw) / 2, (area.size.h - th) / 2));
    // Draw at full size anchored at `at`, then shrink everything around that point.
    let origin_px = v.px(at);
    render_elements_from_surface_tree(renderer, s, v.px(at + off), v.scale, alpha, Kind::Unspecified)
        .into_iter()
        .map(|e| Elem::Thumb(RescaleRenderElement::from_element(e, origin_px, scale)))
        .collect()
}

/// Everything in `view` (global logical coordinates), front to back.
pub fn elements(renderer: &mut GlesRenderer, state: &mut State, view: Rectangle<i32, Logical>) -> Vec<Elem> {
    elements_with(renderer, state, view, true)
}

/// Like `elements`, optionally without the cursor (screenshots leave it out).
pub fn elements_with(renderer: &mut GlesRenderer, state: &mut State, view: Rectangle<i32, Logical>, cursor: bool) -> Vec<Elem> {
    let mut out = Vec::new();
    // The shell lives on the primary monitor, which sits at (0, 0).
    let (ow, oh) = (state.output_size.w, state.output_size.h);
    let v = View { loc: view.loc, scale: state.output_scale };
    let scale = v.scale;
    // Panels animate in when they open; while anything is moving, ask for the next frame.
    let anim_on = state.config.animations;
    let mut animating = false;
    use crate::anim::{self as anim, Phase};
    let mut phase = |edge: &mut anim::Edge, open: bool, ms: u64, ease: fn(f32) -> f32| {
        if !anim_on {
            edge.reset(open);
            return if open { Phase::Open(1.0) } else { Phase::Hidden };
        }
        let p = edge.update(open, ms, ease);
        animating |= p.moving();
        p
    };
    let start_p = phase(&mut state.anims.start, state.start.open, anim::START, anim::ease_out);
    let ac_p = phase(&mut state.anims.action_center, state.action_center.open, anim::ACTION_CENTER, anim::ease_out);
    let flyout_p = phase(&mut state.anims.flyout, state.tray.flyout.is_some(), anim::FLYOUT, anim::ease_out);
    let tv_e = phase(&mut state.anims.task_view, state.desktops.open, anim::TASK_VIEW, anim::ease_out).e();
    let menu_p = phase(&mut state.anims.menu, state.menu.is_some(), anim::MENU, anim::ease_out_quint);
    let switcher_e = phase(&mut state.anims.switcher, state.switcher.is_some(), anim::SWITCHER, anim::ease_out_quint).e();
    let toasts_open = state.toasts_visible();
    let toasts_p = phase(&mut state.anims.toasts, toasts_open, anim::TOASTS, anim::ease_out);
    let preview_open = state.preview.shown.is_some();
    let preview_e = phase(&mut state.anims.preview, preview_open, anim::FLYOUT, anim::ease_out).e();
    // Switching desktops: the new one's windows slide in from the side it lies on.
    let ws = state.wm.current_workspace;
    if state.anims.last_workspace.is_some_and(|last| last != ws) && anim_on {
        let dir = if state.anims.last_workspace.is_some_and(|last| last < ws) { 1 } else { -1 };
        state.anims.desktop_switch = Some((std::time::Instant::now(), dir));
    }
    state.anims.last_workspace = Some(ws);
    let desk_slide = state.anims.desktop_switch.and_then(|(t0, dir)| {
        let p = anim::progress(t0, anim::DESKTOP_SWITCH);
        if p >= 1.0 || !anim_on {
            state.anims.desktop_switch = None;
            return None;
        }
        animating = true;
        let e = anim::ease_out(p);
        Some(((dir as f32 * ow as f32 * 0.18 * (1.0 - e)).round() as i32, anim::ease_out_quint(p)))
    });
    let focused = state.focused();

    // An app being dragged out of Start, just under the cursor.
    let ghost = state.start_drag_ghost().cloned().map(|g| (g, state.pointer_loc.to_i32_round() + Point::from((-16, -42))));

    // Cursor on top of everything.
    let ptr = state.pointer_loc.to_i32_round();
    let status = if cursor { state.cursor_status.clone() } else { CursorImageStatus::Hidden };
    match &status {
        CursorImageStatus::Hidden => {}
        CursorImageStatus::Surface(surface) => {
            let hotspot = with_states(surface, |s| {
                s.data_map.get::<Mutex<CursorImageAttributes>>().map(|a| a.lock().unwrap().hotspot).unwrap_or_default()
            });
            out.extend(surface_tree(renderer, &v, surface, ptr - hotspot, Kind::Cursor));
        }
        CursorImageStatus::Named(icon) => {
            let (buf, hot) = state.cursors.get(*icon);
            out.extend(image(renderer, &v, buf, ptr - *hot, Kind::Cursor));
        }
    }
    if let Some((g, at)) = ghost {
        out.extend(image(renderer, &v, &g, at, Kind::Unspecified));
    }

    // Alt+Tab: live window thumbnails on top of the panel picture.
    crate::supervisor::guard("alt-tab switcher", || {
        if let (Some(origin), Some(panel)) = (state.switcher_origin(), state.switcher_picture()) {
            let sw = state.switcher.as_ref().unwrap();
            for (i, s) in sw.order.iter().enumerate() {
                let tile = sw.tile(i);
                let area = Rectangle::new(
                    origin + tile.loc + Point::from(((tile.size.w - (crate::switcher::TILE_W - 20)) / 2, 30)),
                    (crate::switcher::TILE_W - 20, crate::switcher::THUMB_H).into(),
                );
                out.extend(live_thumbnail(renderer, &v, state, s, area, switcher_e));
            }
            // Pops in: fades up from 94% about its centre.
            let (w, h) = state.switcher.as_ref().unwrap().size();
            let centre = origin + Point::from((w / 2, h / 2));
            let fx = Fx { alpha: switcher_e, scale: 0.94 + 0.06 * switcher_e as f64, origin: centre, offset: (0, 0).into() };
            out.extend(image_fx(renderer, &v, &panel, origin, &fx));
        }
    });

    // Overlay-layer clients (lock screens, notifications) above all our chrome.
    for (s, at) in state.layer_surfaces(&[Layer::Overlay]) {
        out.extend(surface_tree(renderer, &v, &s, at, Kind::Unspecified));
    }

    // Software brightness: everything below the cursor seen through a black veil.
    if state.tray.brightness < 0.999 {
        let alpha = (1.0 - state.tray.brightness) * 0.85;
        state.dim.update(view.size, Color32F::new(0.0, 0.0, 0.0, alpha));
        out.push(Elem::Solid(SolidColorRenderElement::from_buffer(&state.dim, v.px(view.loc), scale, 1.0, Kind::Unspecified)));
    }

    // The volume/brightness overlay from the media keys.
    crate::supervisor::guard("osd", || {
        if let Some((buf, at, alpha)) = state.osd_picture() {
            out.extend(image_fx(renderer, &v, &buf, at, &Fx::fade(alpha)));
        }
    });

    // Notification toasts, above the Action Center and flyouts.
    crate::supervisor::guard("toasts", || {
        // Slide in from the right edge, and back out.
        let now = state.toasts_picture();
        panel_fx(renderer, &v, &mut out, &mut state.anims.toasts, toasts_p, now, |e, _| Fx::slide(e, 380.0 * (1.0 - e), 0.0));
    });

    // Task View: live window thumbnails over its picture.
    crate::supervisor::guard("task view", || {
        let desktops_at = state.desktops_rect().loc;
        if let Some(buf) = state.desktops_picture() {
            // Windows fly from where they are into their place in the grid.
            for (s, area) in state.task_view_thumbs() {
                let from = state.wm.find(&s).map(|w| w.geometry()).filter(|_| tv_e < 1.0).unwrap_or(area);
                out.extend(live_thumbnail(renderer, &v, state, &s, crate::anim::lerp_rect(from, area, tv_e), 1.0));
            }
            out.extend(image_fx(renderer, &v, &buf, desktops_at, &Fx::fade(tv_e)));
        }
    });

    // Action Center (notifications).
    crate::supervisor::guard("action center", || {
        // Slides in from the right edge, and back out.
        let r = state.action_center_rect();
        let now = state.action_center_picture().map(|b| (b, r.loc));
        let w = r.size.w as f32;
        panel_fx(renderer, &v, &mut out, &mut state.anims.action_center, ac_p, now, |e, _| Fx::slide(e.sqrt(), w * (1.0 - e), 0.0));
    });

    // Snipping overlay: everything dimmed except the selection.
    let layout = state.layout_rect();
    if let Some(snip) = state.snip.as_mut() {
        let sel = snip.selection(state.pointer_loc);
        for (buf, at) in snip.overlay(layout, sel) {
            out.push(Elem::Solid(SolidColorRenderElement::from_buffer(buf, v.px(at), scale, 1.0, Kind::Unspecified)));
        }
    }

    // Our right-click menu.
    crate::supervisor::guard("context menu", || {
        // Grows out of the point it was opened at.
        let now = state.menu.as_mut().map(|m| (m.picture(state.font.as_ref()).clone(), m.pos));
        panel_fx(renderer, &v, &mut out, &mut state.anims.menu, menu_p, now, |e, at| Fx {
            alpha: e,
            scale: 0.9 + 0.1 * e as f64,
            origin: at,
            offset: (0, 0).into(),
        });
    });

    // Snap assist: live thumbnails over its picture, on the empty side of the screen.
    crate::supervisor::guard("snap assist", || {
        if let Some((buf, at, thumbs)) = state.snap_assist_picture() {
            for (s, area) in thumbs {
                out.extend(live_thumbnail(renderer, &v, state, &s, area, 1.0));
            }
            out.extend(image(renderer, &v, &buf, at, Kind::Unspecified));
        }
    });

    // Snap layouts, dropped from a maximize button.
    crate::supervisor::guard("snap layouts", || {
        if let Some((buf, at)) = state.snap_layouts_picture() {
            out.extend(image(renderer, &v, &buf, at, Kind::Unspecified));
        }
    });

    // X11 menus and tooltips, above everything but the cursor.
    for x in state.x11_overrides.iter().rev() {
        if let Some(s) = x.wl_surface() {
            out.extend(surface_tree(renderer, &v, &s, x.geometry().loc, Kind::Unspecified));
        }
    }

    // Drag-and-drop icon following the cursor.
    if let Some(icon) = &state.dnd_icon {
        let loc = state.pointer_loc.to_i32_round() + Point::from((12, 12));
        out.extend(surface_tree(renderer, &v, icon, loc, Kind::Unspecified));
    }

    // Taskbar, unless a fullscreen window is in front.
    crate::supervisor::guard("taskbar", || {
        if state.taskbar_shown() {
            let key = TaskbarKey {
                width: ow,
                items: state.taskbar_items(focused.as_ref()),
                time: state.clock.0.clone(),
                date: state.clock.1.clone(),
                hover: match &state.hover {
                    Hover::Taskbar(h) => Some(h.clone()),
                    _ => None,
                },
                start_open: state.start.open,
                volume: ((state.tray.volume * 100.0).round() as u32, state.tray.muted),
                net: state.tray.icon(),
                flyout: state.tray.flyout,
                current_workspace: state.wm.current_workspace,
                taskview_open: state.desktops.open,
                tray_apps: state.sni_items.iter().map(|i| (i.id.clone(), tray_fingerprint(i))).collect(),
                search_text: (state.start.open && state.start.searching()).then(|| state.start.query.clone()),
                notifications: (!state.action_center.notifications.is_empty(), state.action_center.unseen),
                action_center_open: state.action_center.open,
            };
            if state.taskbar.as_ref().is_none_or(|t| t.key != key) {
                let tray_icons = tray_icons(state);
                state.taskbar = Some(ui::draw_taskbar(state.font.as_ref(), &mut state.icons, state.logo.as_ref(), &tray_icons, key));
            }
            let bar = state.taskbar.as_ref().unwrap();
            out.extend(image(renderer, &v, &bar.buffer, (0, oh - TASKBAR_H).into(), Kind::Unspecified));
        }
    });

    // Start, flyouts and previews go behind the taskbar, so they slide out from under its top
    // edge rather than up from the bottom of the screen across it.
    // Start menu.
    crate::supervisor::guard("start menu", || {
        if state.start.open {
            let height = (oh - TASKBAR_H).min(620);
            let _ = &start_p;
            let items = state.start_rows();
            let hover = match &state.hover {
                Hover::Start(h) => Some(h.clone()),
                _ => None,
            };
            let key = StartKey {
                height,
                query: state.start.query.clone(),
                items,
                selected: state.start.selected,
                scroll: state.start.scroll,
                hover,
                power_open: state.start.power_open,
                searching: state.start.searching(),
                filter: state.start.filter,
                kinds: state.start.results.iter().map(|e| state.result_kind(e)).collect(),
                tiles: state.start_tiles().into_iter().map(|(i, g)| (state.apps[i].name.clone(), state.apps[i].icon.clone(), g)).collect(),
                user: std::env::var("USER").unwrap_or_else(|_| "User".into()),
                rail_open: state.start.rail_open,
                preview: if state.start.searching() && !state.start.query.trim().is_empty() { state.start_preview() } else { None },
            };
            if state.start.menu.as_ref().is_none_or(|m| m.key != key) {
                state.start.menu = Some(ui::draw_start_menu(state.font.as_ref(), &mut state.icons, key));
            }
            let menu = state.start.menu.as_ref().unwrap();
            let now = Some((menu.buffer.clone(), Point::from((0, oh - TASKBAR_H - height))));
            // Rises out of the taskbar, and sinks back into it.
            panel_fx(renderer, &v, &mut out, &mut state.anims.start, start_p, now, |e, _| Fx::slide(e.sqrt(), 0.0, 110.0 * (1.0 - e)));
            let _ = MENU_W;
        } else {
            panel_fx(renderer, &v, &mut out, &mut state.anims.start, start_p, None, |e, _| Fx::slide(e, 0.0, 60.0 * (1.0 - e)));
        }
    });

    // Taskbar previews: live thumbnails over the strip's picture, fading in.
    crate::supervisor::guard("taskbar previews", || {
        if let Some((buf, at, thumbs)) = state.preview_picture() {
            let lift = Point::from((0, (14.0 * (1.0 - preview_e)).round() as i32));
            for (s, area) in thumbs {
                out.extend(live_thumbnail(renderer, &v, state, &s, Rectangle::new(area.loc + lift, area.size), preview_e));
            }
            out.extend(image_fx(renderer, &v, &buf, at, &Fx::slide(preview_e, 0.0, lift.y as f32)));
        }
    });

    // Tray flyout (volume/brightness slider), above the taskbar.
    crate::supervisor::guard("tray flyout", || {
        let flyout_at = state.flyout_rect().loc;
        let now = state.flyout_picture().cloned().map(|b| (b, flyout_at));
        panel_fx(renderer, &v, &mut out, &mut state.anims.flyout, flyout_p, now, |e, _| Fx::slide(e, 0.0, 36.0 * (1.0 - e)));
    });

    // Top-layer clients (panels, docks) above windows.
    for (s, at) in state.layer_surfaces(&[Layer::Top]) {
        out.extend(surface_tree(renderer, &v, &s, at, Kind::Unspecified));
    }

    // Where a dragged window will snap.
    if let Some(z) = state.wm.snap_preview {
        state.snap_buf.update(z.size, Color32F::new(0.46, 0.73, 0.93, 0.28));
        out.push(Elem::Solid(SolidColorRenderElement::from_buffer(&state.snap_buf, v.px(z.loc), scale, 1.0, Kind::Unspecified)));
    }

    let taskbar_shown = state.taskbar_shown();
    // Where each window's taskbar button is, for minimizing into it and restoring out of it.
    let buttons: Vec<(WlSurface, Point<i32, Logical>)> = state
        .taskbar
        .as_ref()
        .filter(|_| taskbar_shown)
        .map(|bar| {
            let mut out = Vec::new();
            for (r, hit) in &bar.hits {
                if let ui::TaskbarHit::Button(i) = hit {
                    let centre = Point::from((r.loc.x + r.size.w / 2, oh - TASKBAR_H + r.loc.y + r.size.h / 2));
                    for s in bar.key.items.get(*i).map(|it| it.windows.as_slice()).unwrap_or_default() {
                        out.push((s.clone(), centre));
                    }
                }
            }
            out
        })
        .unwrap_or_default();
    // Aero Peek: the window looked at shows even if minimized; the rest fade away.
    let peeked = state.peeked_window().cloned();
    let peek_alpha: Vec<(WlSurface, f32)> = state
        .wm
        .windows
        .iter()
        .map(|w| {
            let (a, fading) = state.peek_alpha(w.surface());
            animating |= fading;
            (w.surface().clone(), a)
        })
        .collect();
    // Windoze, top to bottom (only current workspace).
    let font = state.font.as_ref();
    let icons = &mut state.icons;
    let apps = &state.apps;
    // A window being minimized is still drawn (shrinking away) until its animation ends.
    for w in state.wm.windows.iter_mut().rev().filter(|w| {
        (!w.minimized || w.vanish.is_some() || peeked.as_ref() == Some(w.surface())) && w.mapped && w.workspace == state.wm.current_workspace
    }) {
        let button = buttons.iter().find(|(s, _)| s == w.surface()).map(|(_, p)| *p);
        let mut fx = window_fx(w, anim_on, button, &mut animating).unwrap_or(Fx::fade(1.0));
        if let Some((dx, alpha)) = desk_slide {
            fx.offset.x += dx;
            fx.alpha *= alpha;
        }
        if w.minimized && w.vanish.is_none() && peeked.as_ref() != Some(w.surface()) {
            continue;
        }
        if let Some((_, a)) = peek_alpha.iter().find(|(s, _)| s == w.surface()) {
            fx.alpha *= a;
            if fx.alpha <= 0.001 {
                continue;
            }
        }
        let active = Some(w.surface()) == focused.as_ref();
        let sloc = w.surface_loc();
        for (popup, offset) in PopupManager::popups_for_surface(w.surface()) {
            // Relative to the parent's window geometry (`pos`), not its surface (`sloc`).
            let ploc = w.pos + offset - popup.geometry().loc;
            out.extend(surface_tree_fx(renderer, &v, popup.wl_surface(), ploc, &fx));
        }
        out.extend(surface_tree_fx(renderer, &v, w.surface(), sloc, &fx));

        let hover = match &state.hover {
            Hover::Title(s, b) if s == w.surface() => Some(*b),
            _ => None,
        };
        if let Some((buf, at)) = w.titlebar(font, icons, apps, active, hover) {
            out.extend(image_fx(renderer, &v, buf, at, &fx));
        }
        // Pinned on top looks like any other window (no outline); its menus say it's pinned.
        let border = if w.decorated() && w.maximized.is_none() {
            Some(if active { BORDER_ACTIVE } else { BORDER_IDLE })
        } else {
            None
        };
        if let Some(color) = border {
            for (buf, at) in w.border(color) {
                out.push(solid_fx(&v, buf, at, &fx));
            }
        }
    }

    // Bottom and background layers: vide-desktop's icons and wallpaper live here.
    state.arrange_layers();
    for (s, at) in state.layer_surfaces(&[Layer::Bottom, Layer::Background]) {
        out.extend(surface_tree(renderer, &v, &s, at, Kind::Unspecified));
    }

    // Our own wallpaper at the very back, on monitors no desktop program covers.
    let bare: Vec<_> = (0..state.monitors.len()).filter(|&i| !state.has_background_layer(i)).map(|i| state.monitors[i].geo).filter(|g| g.overlaps(view)).collect();
    for g in bare {
        if !state.wallpapers.iter().any(|(s, _)| *s == g.size) {
            state.wallpapers.push((g.size, ui::draw_wallpaper(g.size.w, g.size.h)));
        }
        let (_, wp) = state.wallpapers.iter().find(|(s, _)| *s == g.size).unwrap();
        out.extend(image(renderer, &v, wp, g.loc, Kind::Unspecified));
    }

    // Night light: a warm veil over the whole picture, cursor included, like the
    // monitor's own colour temperature (which desktop panels usually don't have).
    if state.config.night_light && state.config.night_light_strength > 0.0 {
        let alpha = (state.config.night_light_strength as f32 * 0.32).clamp(0.0, 0.4);
        state.night.update(view.size, Color32F::new(1.0, 0.72, 0.42, alpha));
        out.push(Elem::Solid(SolidColorRenderElement::from_buffer(&state.night, v.px(view.loc), scale, 1.0, Kind::Unspecified)));
    }

    if animating {
        state.needs_redraw = true;
    }
    out
}

const TRAY_ICON: u32 = 20;

fn tray_fingerprint(i: &crate::sni::TrayItem) -> String {
    match &i.pixmap {
        // Cheap content hash: size plus a sparse sample of the pixels.
        Some((w, h, px)) => {
            let sample: u64 = px.iter().step_by(37).map(|&b| b as u64).sum();
            format!("px:{w}x{h}:{sample}")
        }
        None => format!("{}|{}", i.theme_path, i.icon_name),
    }
}

/// Load each app tray icon: from the app's own icon folder, the theme, or its raw pixels.
fn tray_icons(state: &mut State) -> Vec<Option<std::rc::Rc<crate::icons::Icon>>> {
    let mut out = Vec::new();
    for item in &state.sni_items {
        let fp = tray_fingerprint(item);
        if !state.tray_icon_cache.contains_key(&fp) {
            let raw_icon = match &item.pixmap {
                Some((w, h, px)) => Some(crate::icons::from_rgba(*w, *h, px.clone(), 48)),
                None => {
                    let in_app_dir = (!item.theme_path.is_empty())
                        .then(|| {
                            ["png", "svg"].iter().find_map(|ext| {
                                state.tray_theme.get(&format!("{}/{}.{ext}", item.theme_path, item.icon_name), 48)
                            })
                        })
                        .flatten();
                    in_app_dir
                        .or_else(|| state.tray_theme.get(&item.icon_name, 48))
                        .map(|rc| crate::icons::from_rgba(rc.w, rc.h, rc.rgba.clone(), 48))
                }
            };
            // Share the tray icon with the window/taskbar icon cache as a custom fallback,
            // so apps launched from a terminal without a .desktop file still show an icon.
            if let Some(ref ic) = raw_icon {
                let copy = |i: &crate::icons::Icon| crate::icons::Icon { w: i.w, h: i.h, rgba: i.rgba.clone() };
                let sni_id = item.id.split('/').next().unwrap_or(&item.id);
                if !sni_id.is_empty() && !sni_id.starts_with(':') {
                    state.icons.set_custom(sni_id, copy(ic));
                }
                if !item.title.is_empty() {
                    state.icons.set_custom(&item.title, copy(ic));
                }
                if !item.icon_name.is_empty() {
                    state.icons.set_custom(&item.icon_name, copy(ic));
                }
                if !item.proc_name.is_empty() {
                    state.icons.set_custom(&item.proc_name, copy(ic));
                }
            }
            let icon = raw_icon.map(|ic| {
                let scaled = crate::icons::from_rgba(ic.w, ic.h, ic.rgba, TRAY_ICON);
                std::rc::Rc::new(lighten_dark_glyph(&scaled).unwrap_or(scaled))
            });
            state.tray_icon_cache.insert(fp.clone(), icon);
        }
        out.push(state.tray_icon_cache[&fp].clone());
    }
    out
}

/// Tray icons drawn for light menu bars (black "template" glyphs, like Claude's) vanish
/// on our dark taskbar. If an icon is nearly all dark and colourless, redraw it in white,
/// keeping its shape (alpha). Colourful icons are left alone.
fn lighten_dark_glyph(icon: &crate::icons::Icon) -> Option<crate::icons::Icon> {
    let (mut n, mut lum, mut sat) = (0u32, 0.0f32, 0.0f32);
    for px in icon.rgba.chunks_exact(4) {
        let a = px[3] as f32 / 255.0;
        if a < 0.25 {
            continue;
        }
        // Un-premultiply to judge the real colour.
        let (r, g, b) = (px[0] as f32 / 255.0 / a, px[1] as f32 / 255.0 / a, px[2] as f32 / 255.0 / a);
        lum += 0.2126 * r + 0.7152 * g + 0.0722 * b;
        sat += r.max(g).max(b) - r.min(g).min(b);
        n += 1;
    }
    if n == 0 || lum / n as f32 > 0.3 || sat / n as f32 > 0.15 {
        return None;
    }
    let rgba = icon.rgba.chunks_exact(4).flat_map(|px| [px[3], px[3], px[3], px[3]]).collect();
    Some(crate::icons::Icon { w: icon.w, h: icon.h, rgba })
}

/// Tell every visible surface it was just presented so it can draw its next frame.
pub fn send_frames(state: &State, time: u32) {
    let send = |surface: &WlSurface| {
        with_surface_tree_downward(
            surface,
            (),
            |_, _, &()| TraversalAction::DoChildren(()),
            |_, states, &()| {
                for cb in states.cached_state.get::<SurfaceAttributes>().current().frame_callbacks.drain(..) {
                    cb.done(time);
                }
            },
            |_, _, &()| true,
        );
    };
    for w in &state.wm.windows {
        send(w.surface());
        for (popup, _) in PopupManager::popups_for_surface(w.surface()) {
            send(popup.wl_surface());
        }
    }
    for x in &state.x11_overrides {
        if let Some(s) = x.wl_surface() {
            send(&s);
        }
    }
    for (s, _) in state.layer_surfaces(&[Layer::Overlay, Layer::Top, Layer::Bottom, Layer::Background]) {
        send(&s);
    }
    if let CursorImageStatus::Surface(s) = &state.cursor_status {
        send(s);
    }
}
