//! The clock's flyout: Windoze 10's big time and date over a month calendar. The arrows (or
//! the mouse wheel) page through months; today is the filled square.

use chrono::{Datelike, Local, NaiveDate};
use smithay::utils::{Logical, Rectangle};

use crate::{
    canvas::{measure, Canvas},
    theme::th,
    tray::FlyoutAction,
};

pub const HEIGHT: i32 = 452;
const CELL_W: i32 = 46;
const CELL_H: i32 = 40;
const GRID_X: i32 = (crate::tray::FLYOUT_W - CELL_W * 7) / 2;

/// The first day of the month `offset` months from this one.
pub fn month(offset: i32) -> NaiveDate {
    let today = Local::now().date_naive();
    let m0 = today.year() * 12 + today.month0() as i32 + offset;
    NaiveDate::from_ymd_opt(m0.div_euclid(12), m0.rem_euclid(12) as u32 + 1, 1).unwrap_or(today)
}

/// The 42 days a month page shows, Sunday first (Windoze's US layout), starting in the week
/// the month begins.
pub fn page(first: NaiveDate) -> Vec<NaiveDate> {
    let start = first - chrono::Duration::days(first.weekday().num_days_from_sunday() as i64);
    (0..42).map(|i| start + chrono::Duration::days(i)).collect()
}

/// Paint the flyout; returns its click targets.
pub fn draw(c: &mut Canvas, font: &fontdue::Font, offset: i32, h24: bool, hover: Option<&FlyoutAction>) -> Vec<(Rectangle<i32, Logical>, FlyoutAction)> {
    let mut hits = Vec::new();
    let now = Local::now();
    let today = now.date_naive();

    // The time, large, and the full date under it.
    let time = if h24 { now.format("%H:%M").to_string() } else { now.format("%-I:%M %p").to_string() };
    c.text(font, 38.0, 24, 14, 52, th().text, &time);
    c.text(font, 14.0, 24, 66, 22, th().link, &now.format("%A, %B %-d, %Y").to_string());
    c.rect(0, 100, c.w, 1, th().divider);

    // Month title and its arrows.
    let first = month(offset);
    c.text(font, 15.0, 24, 112, 32, th().text, &first.format("%B %Y").to_string());
    for (i, (a, up)) in [(FlyoutAction::CalMonth(-1), true), (FlyoutAction::CalMonth(1), false)].into_iter().enumerate() {
        let r = Rectangle::new((c.w - 88 + i as i32 * 40, 112).into(), (32, 32).into());
        if hover == Some(&a) {
            c.rect(r.loc.x, r.loc.y, r.size.w, r.size.h, th().button_hover);
        }
        let (cx, cy) = (r.loc.x as f32 + 16.0, r.loc.y as f32 + 16.0);
        let dy = if up { -3.0 } else { 3.0 };
        c.line(cx - 6.0, cy - dy, cx, cy + dy, 1.5, th().text);
        c.line(cx, cy + dy, cx + 6.0, cy - dy, 1.5, th().text);
        hits.push((r, a));
    }

    // Weekday initials, then six weeks of days.
    let top = 154;
    for (i, d) in ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"].iter().enumerate() {
        let tw = measure(font, 12.0, d);
        c.text(font, 12.0, GRID_X + i as i32 * CELL_W + (CELL_W - tw) / 2, top, 24, th().text, d);
    }
    for (i, day) in page(first).into_iter().enumerate() {
        let (x, y) = (GRID_X + (i % 7) as i32 * CELL_W, top + 28 + (i / 7) as i32 * CELL_H);
        let label = day.day().to_string();
        let tw = measure(font, 14.0, &label);
        let in_month = day.month() == first.month();
        if day == today {
            c.rect(x + 2, y + 2, CELL_W - 4, CELL_H - 4, th().accent_light);
            c.outline(x + 4, y + 4, CELL_W - 8, CELL_H - 8, th().panel);
        }
        let color = if day == today { crate::theme::ON_ACCENT } else if in_month { th().text } else { th().text_dim };
        c.text(font, 14.0, x + (CELL_W - tw) / 2, y, CELL_H, color, &label);
    }
    if offset != 0 {
        // Off on another month: a way back, like Windoze's click on the date.
        let label = "Today";
        let tw = measure(font, 13.0, label);
        let r = Rectangle::new((c.w - 24 - tw - 16, HEIGHT - 36).into(), (tw + 16, 28).into());
        if hover == Some(&FlyoutAction::CalToday) {
            c.rect(r.loc.x, r.loc.y, r.size.w, r.size.h, th().button_hover);
        }
        c.text(font, 13.0, r.loc.x + 8, r.loc.y, 28, th().link, label);
        hits.push((r, FlyoutAction::CalToday));
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_start_on_sunday_and_hold_the_month() {
        let first = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let days = page(first);
        assert_eq!(days.len(), 42);
        assert_eq!(days[0].weekday(), chrono::Weekday::Sun);
        assert!(days.contains(&NaiveDate::from_ymd_opt(2026, 11, 30).unwrap()));
    }
}
