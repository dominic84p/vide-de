//! Emoji Panic: Windoze's emoji panel (Win+.). Search or browse, click one (or Enter for the
//! first match) and it goes into the app you were typing in.
//!
//! We can't type into another app ourselves, so the pick goes to the compositor over
//! `$VIDE_IPC` as `emoji <text>`. It only accepts that once, right after you pressed Win+.,
//! then puts the emoji on the clipboard, gives the app its focus back and pastes.

use std::{io::Write, path::PathBuf};

use eframe::egui::{self, Color32, FontId, Key, RichText, Sense, vec2};

const CELL: f32 = 40.0;
const COLS: usize = 9;
const RECENT_MAX: usize = 36;

struct Theme {
    bg: Color32,
    field: Color32,
    hover: Color32,
    text: Color32,
    dim: Color32,
    accent: Color32,
}

const DARK: Theme = Theme {
    bg: Color32::from_rgb(0x2b, 0x2b, 0x2b),
    field: Color32::from_rgb(0x1f, 0x1f, 0x1f),
    hover: Color32::from_rgb(0x45, 0x45, 0x45),
    text: Color32::from_rgb(0xf2, 0xf2, 0xf2),
    dim: Color32::from_rgb(0xa0, 0xa0, 0xa0),
    accent: Color32::from_rgb(0x00, 0x78, 0xd7),
};
const LIGHT: Theme = Theme {
    bg: Color32::from_rgb(0xf2, 0xf2, 0xf2),
    field: Color32::WHITE,
    hover: Color32::from_rgb(0xdd, 0xdd, 0xdd),
    text: Color32::from_rgb(0x11, 0x11, 0x11),
    dim: Color32::from_rgb(0x60, 0x60, 0x60),
    accent: Color32::from_rgb(0x00, 0x78, 0xd7),
};

/// The tabs along the top: recently used, then emoji's own groups.
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Recent,
    Group(emojis::Group),
}

const TABS: [(Tab, &str, &str); 10] = [
    (Tab::Recent, "🕒", "Most recently used"),
    (Tab::Group(emojis::Group::SmileysAndEmotion), "😀", "Smiley faces"),
    (Tab::Group(emojis::Group::PeopleAndBody), "🧑", "People"),
    (Tab::Group(emojis::Group::AnimalsAndNature), "🐻", "Animals and nature"),
    (Tab::Group(emojis::Group::FoodAndDrink), "🍔", "Food and drink"),
    (Tab::Group(emojis::Group::TravelAndPlaces), "🚗", "Travel and places"),
    (Tab::Group(emojis::Group::Activities), "⚽", "Activities"),
    (Tab::Group(emojis::Group::Objects), "💡", "Objects"),
    (Tab::Group(emojis::Group::Symbols), "❤", "Symbols"),
    (Tab::Group(emojis::Group::Flags), "🏁", "Flags"),
];

struct App {
    query: String,
    tab: Tab,
    recent: Vec<String>,
    theme: &'static Theme,
    hovered: Option<&'static emojis::Emoji>,
    focus_search: bool,
}

fn recent_path() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"));
    base.join("vide-de/emoji-recent")
}

/// Every emoji worth showing: the default skin tone only, like Windoze's grid.
fn all() -> impl Iterator<Item = &'static emojis::Emoji> {
    emojis::iter().filter(|e| e.skin_tone().is_none_or(|t| t == emojis::SkinTone::Default))
}

fn matches(e: &emojis::Emoji, words: &[String]) -> bool {
    let name = e.name().to_lowercase();
    words.iter().all(|w| name.contains(w.as_str()) || e.shortcodes().any(|s| s.contains(w.as_str())))
}

impl App {
    fn new() -> Self {
        let recent: Vec<String> = std::fs::read_to_string(recent_path()).unwrap_or_default().lines().map(str::to_string).filter(|l| !l.is_empty()).collect();
        let tab = if recent.is_empty() { Tab::Group(emojis::Group::SmileysAndEmotion) } else { Tab::Recent };
        let theme = if vide_config::Config::load().light_mode { &LIGHT } else { &DARK };
        Self { query: String::new(), tab, recent, theme, hovered: None, focus_search: true }
    }

    /// What the grid shows right now.
    fn shown(&self) -> Vec<&'static emojis::Emoji> {
        let q = self.query.trim().to_lowercase();
        if !q.is_empty() {
            let words: Vec<String> = q.split_whitespace().map(str::to_string).collect();
            return all().filter(|e| matches(e, &words)).take(400).collect();
        }
        match self.tab {
            Tab::Recent => self.recent.iter().filter_map(|s| emojis::get(s)).collect(),
            Tab::Group(g) => all().filter(|e| e.group() == g).collect(),
        }
    }

    fn pick(&mut self, ctx: &egui::Context, e: &emojis::Emoji) {
        let s = e.as_str().to_string();
        self.recent.retain(|r| *r != s);
        self.recent.insert(0, s.clone());
        self.recent.truncate(RECENT_MAX);
        let path = recent_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, self.recent.join("\n"));
        if !send(&format!("emoji {s}")) {
            // Not under VIDE-DE (or it said no): leave it on our clipboard to paste by hand.
            ctx.copy_text(s);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

fn send(cmd: &str) -> bool {
    let Some(path) = std::env::var_os("VIDE_IPC") else { return false };
    std::os::unix::net::UnixStream::connect(path).and_then(|mut s| s.write_all(format!("{cmd}\n").as_bytes())).is_ok()
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let t = self.theme;
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let shown = self.shown();
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            if let Some(e) = shown.first().copied() {
                self.pick(&ctx, e);
                return;
            }
        }

        egui::CentralPanel::default().frame(egui::Frame::new().fill(t.bg).inner_margin(egui::Margin::same(10))).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 6.0);
            let search = egui::TextEdit::singleline(&mut self.query)
                .hint_text("Search emoji")
                .font(FontId::proportional(15.0))
                .text_color(t.text)
                .background_color(t.field)
                .desired_width(f32::INFINITY);
            let r = ui.add(search);
            if self.focus_search {
                r.request_focus();
                self.focus_search = false;
            }

            if self.query.trim().is_empty() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    for (tab, icon, name) in TABS {
                        let on = self.tab == tab;
                        let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 32.0), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(rect, 4.0, t.hover);
                        }
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::proportional(18.0), t.text);
                        if on {
                            ui.painter().rect_filled(egui::Rect::from_min_size(rect.left_bottom() - vec2(-6.0, 3.0), vec2(22.0, 3.0)), 1.0, t.accent);
                        }
                        if resp.on_hover_text(name).clicked() {
                            self.tab = tab;
                        }
                    }
                });
            }

            let footer = 24.0;
            let h = ui.available_height() - footer;
            self.hovered = None;
            egui::ScrollArea::vertical().max_height(h).auto_shrink([false, false]).show(ui, |ui| {
                if shown.is_empty() {
                    let msg = if self.tab == Tab::Recent && self.query.trim().is_empty() { "Emoji you use show up here." } else { "No emoji found. Panic!" };
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| ui.label(RichText::new(msg).color(t.dim)));
                    return;
                }
                let mut picked = None;
                for row in shown.chunks(COLS) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        for e in row {
                            let (rect, resp) = ui.allocate_exact_size(vec2(CELL, CELL), Sense::click());
                            if resp.hovered() {
                                ui.painter().rect_filled(rect, 4.0, t.hover);
                                self.hovered = Some(e);
                            }
                            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, e.as_str(), FontId::proportional(24.0), t.text);
                            if resp.clicked() {
                                picked = Some(*e);
                            }
                        }
                    });
                }
                if let Some(e) = picked {
                    self.pick(&ctx, e);
                }
            });

            // The name of the one under the pointer, or what Enter would pick.
            let label = self.hovered.or_else(|| (!self.query.trim().is_empty()).then(|| shown.first().copied()).flatten());
            let text = label.map(|e| format!("{}  {}", e.as_str(), e.name())).unwrap_or_default();
            ui.add_sized(vec2(ui.available_width(), footer), egui::Label::new(RichText::new(text).color(t.dim).size(13.0)).truncate());
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Emoji Panic")
            .with_app_id("emoji-panic")
            .with_inner_size([COLS as f32 * CELL + 36.0, 430.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Emoji Panic",
        options,
        Box::new(|cc| {
            vide_fonts::install_egui!(eframe::egui, cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}
