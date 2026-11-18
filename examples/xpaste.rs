//! A tiny X11 paste client for nested tests: maps a window (click it to give it focus), then
//! after a delay reads the CLIPBOARD as a mime type, the way Chromium does (INCR included),
//! and writes the bytes to a file. Pastes from X11 apps (Claude Desktop runs on Xwayland)
//! are otherwise hard to test without xclip.
//!
//!   DISPLAY=:N xpaste image/png OUT [DELAY_SECS]

use std::time::{Duration, Instant};

use x11rb::{
    connection::Connection,
    protocol::{xproto::*, Event},
    wrapper::ConnectionExt as _,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (mime, out) = (args.get(1).ok_or("usage: xpaste MIME OUT [DELAY]")?, args.get(2).ok_or("usage: xpaste MIME OUT [DELAY]")?);
    let delay: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);

    let (conn, screen) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen].root;
    let win = conn.generate_id()?;
    conn.create_window(
        x11rb::COPY_DEPTH_FROM_PARENT,
        win,
        root,
        0,
        0,
        300,
        200,
        0,
        WindowClass::INPUT_OUTPUT,
        x11rb::COPY_FROM_PARENT,
        &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE).background_pixel(0x336699),
    )?;
    conn.change_property8(PropMode::REPLACE, win, AtomEnum::WM_NAME, AtomEnum::STRING, b"xpaste")?;
    conn.map_window(win)?;
    conn.flush()?;
    std::thread::sleep(Duration::from_secs(delay));

    let atom = |name: &str| -> Result<Atom, Box<dyn std::error::Error>> { Ok(conn.intern_atom(false, name.as_bytes())?.reply()?.atom) };
    let (clipboard, target, prop, incr) = (atom("CLIPBOARD")?, atom(mime)?, atom("XPASTE")?, atom("INCR")?);
    conn.convert_selection(win, clipboard, target, prop, x11rb::CURRENT_TIME)?;
    conn.flush()?;

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut data = Vec::new();
    let mut incremental = false;
    loop {
        if Instant::now() > deadline {
            return Err("timed out".into());
        }
        let Some(ev) = conn.poll_for_event()? else {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        };
        match ev {
            Event::SelectionNotify(n) => {
                if n.property == x11rb::NONE {
                    return Err("selection refused (no access, or mime not offered)".into());
                }
                let r = conn.get_property(true, win, prop, AtomEnum::ANY, 0, u32::MAX / 4)?.reply()?;
                if r.type_ == incr {
                    incremental = true;
                } else {
                    data = r.value;
                    break;
                }
                conn.flush()?;
            }
            Event::PropertyNotify(n) if incremental && n.atom == prop && n.state == Property::NEW_VALUE => {
                let r = conn.get_property(true, win, prop, AtomEnum::ANY, 0, u32::MAX / 4)?.reply()?;
                conn.flush()?;
                if r.value.is_empty() {
                    break;
                }
                data.extend_from_slice(&r.value);
            }
            _ => {}
        }
    }
    std::fs::write(out, &data)?;
    println!("xpaste: {} bytes of {mime}{}", data.len(), if incremental { " (INCR)" } else { "" });
    Ok(())
}
