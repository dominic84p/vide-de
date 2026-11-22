//! The other monitors' taskbars. Each has Start, search, Task View, the clock and a button for
//! every app with a window on that monitor; the tray and the Action Center stay on the main
//! one, and the main one still shows every window, as on Windoze 10 ("main taskbar and taskbar
//! where window is open").

use smithay::utils::{Logical, Point, Rectangle};

use crate::{
    state::State,
    ui::{self, TaskItem, TaskbarHit, TaskbarKey, TASKBAR_H},
};

impl State {
    /// Each secondary monitor's taskbar strip, by monitor index (1..).
    pub fn side_bar_rects(&self) -> Vec<(usize, Rectangle<i32, Logical>)> {
        self.monitors
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, m)| !self.side_bar_hidden(m.geo))
            .map(|(i, m)| (i, Rectangle::new((m.geo.loc.x, m.geo.loc.y + m.geo.size.h - TASKBAR_H).into(), (m.geo.size.w, TASKBAR_H).into())))
            .collect()
    }

    /// A fullscreen window on top of that monitor hides its taskbar, unless the shell is open.
    fn side_bar_hidden(&self, geo: Rectangle<i32, Logical>) -> bool {
        if self.start.open || self.desktops.open {
            return false;
        }
        let ws = self.wm.current_workspace;
        let top = self.wm.windows.iter().rev().find(|w| w.mapped && !w.minimized && w.workspace == ws && self.screen_of(w.surface()) == geo);
        top.is_some_and(|w| w.fullscreen.is_some())
    }

    /// The main taskbar's buttons, cut down to the windows on this monitor.
    fn side_items(&self, geo: Rectangle<i32, Logical>) -> Vec<TaskItem> {
        let focused = self.focused();
        self.taskbar_items(focused.as_ref())
            .into_iter()
            .filter_map(|mut it| {
                it.windows.retain(|s| self.screen_of(s) == geo);
                if it.windows.is_empty() {
                    return None;
                }
                it.focused = it.windows.iter().any(|s| Some(s) == focused.as_ref() && self.wm.find(s).is_some_and(|w| !w.minimized));
                Some(it)
            })
            .collect()
    }

    /// Paint (or reuse) monitor `i`'s taskbar.
    pub fn side_bar(&mut self, i: usize) -> Option<&ui::Taskbar> {
        let geo = self.monitors.get(i)?.geo;
        let hover = self.side_hover.as_ref().filter(|(m, _)| *m == i).map(|(_, h)| h.clone());
        let key = TaskbarKey {
            width: geo.size.w,
            items: self.side_items(geo),
            time: self.clock.0.clone(),
            date: self.clock.1.clone(),
            hover,
            start_open: self.start.open,
            volume: (0, false),
            net: self.tray.icon(),
            flyout: None,
            current_workspace: self.wm.current_workspace,
            taskview_open: self.desktops.open,
            tray_apps: Vec::new(),
            search_text: None,
            notifications: (false, 0),
            action_center_open: false,
            secondary: true,
            theme_icons: None,
            overflow_open: false,
        };
        while self.side_bars.len() <= i {
            self.side_bars.push(None);
        }
        if self.side_bars[i].as_ref().is_none_or(|t| t.key != key) {
            let bar = ui::draw_taskbar(self.font.as_ref(), &mut self.icons, self.logo.as_ref(), key);
            self.side_bars[i] = Some(bar);
        }
        self.side_bars[i].as_ref()
    }

    /// Which monitor's taskbar is under `p`, and what on it.
    pub fn side_bar_at(&self, p: Point<f64, Logical>) -> Option<(usize, TaskbarHit)> {
        let (i, r) = self.side_bar_rects().into_iter().find(|(_, r)| r.to_f64().contains(p))?;
        let local = (p - r.loc.to_f64()).to_i32_round();
        let hit = self
            .side_bars
            .get(i)
            .and_then(|b| b.as_ref())
            .and_then(|b| b.hits.iter().find(|(r, _)| r.contains(local)).map(|(_, h)| h.clone()))
            .unwrap_or(TaskbarHit::Nothing);
        Some((i, hit))
    }

    pub fn side_bar_click(&mut self, i: usize, hit: TaskbarHit, left: bool, right: bool, middle: bool) {
        let item = |s: &Self, n: usize| s.side_bars.get(i).and_then(|b| b.as_ref()).and_then(|b| b.key.items.get(n)).cloned();
        match hit {
            TaskbarHit::Start | TaskbarHit::Search if left => self.toggle_start(),
            TaskbarHit::Start if right => self.open_quick_link_menu(),
            TaskbarHit::TaskView if left => self.toggle_desktops(),
            TaskbarHit::ShowDesktop if left => self.show_desktop(),
            TaskbarHit::Clock if left => self.toggle_flyout(crate::tray::Flyout::Calendar),
            TaskbarHit::Button(n) if left => {
                if let Some(it) = item(self, n) {
                    self.click_task_item(it);
                }
            }
            TaskbarHit::Button(n) if right => {
                if let Some(it) = item(self, n) {
                    let x = self.pointer_loc.x as i32;
                    self.open_taskbar_menu(&it.windows, x);
                }
            }
            TaskbarHit::Button(n) if middle => {
                let app = item(self, n)
                    .and_then(|it| it.windows.first().cloned())
                    .and_then(|s| self.wm.find(&s).and_then(|w| crate::apps::match_app(&self.apps, &w.app_id()).cloned()));
                if let Some(app) = app {
                    self.launch(&app);
                }
            }
            _ => {}
        }
        self.needs_redraw = true;
    }
}
