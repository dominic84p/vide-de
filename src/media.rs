//! What's playing, for the volume flyout: Windoze 10 puts the media controls of whatever plays
//! sound under the volume slider. We ask MPRIS (the D-Bus interface every Linux player
//! speaks) on a thread, only while the flyout is open, and send the answer back to the loop.

use std::{collections::HashMap, time::Duration};

use smithay::reexports::calloop::{
    channel::{self, Sender},
    timer::{TimeoutAction, Timer},
};
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::OwnedValue,
};

use crate::{state::State, tray::Flyout};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct NowPlaying {
    /// The player's bus name, to send it commands.
    pub player: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
}

#[derive(Default)]
pub struct Media {
    pub now: Option<NowPlaying>,
    tx: Option<Sender<Option<NowPlaying>>>,
}

/// The first player that's playing, else the first one there is.
fn query() -> Option<NowPlaying> {
    let conn = Connection::session().ok()?;
    let bus = Proxy::new(&conn, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus").ok()?;
    let names: Vec<String> = bus.call("ListNames", &()).ok()?;
    let mut found = Vec::new();
    for name in names.into_iter().filter(|n| n.starts_with("org.mpris.MediaPlayer2.")) {
        let Ok(p) = Proxy::new(&conn, name.clone(), "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player") else { continue };
        let playing = p.get_property::<String>("PlaybackStatus").is_ok_and(|s| s == "Playing");
        let meta: HashMap<String, OwnedValue> = p.get_property("Metadata").unwrap_or_default();
        let text = |k: &str| meta.get(k).and_then(|v| String::try_from(v.clone()).ok()).unwrap_or_default();
        let artist = meta.get("xesam:artist").and_then(|v| Vec::<String>::try_from(v.clone()).ok()).map(|a| a.join(", ")).unwrap_or_default();
        let title = text("xesam:title");
        if title.is_empty() && artist.is_empty() {
            continue;
        }
        found.push(NowPlaying { player: name, title, artist, playing });
    }
    let i = found.iter().position(|n| n.playing).unwrap_or(0);
    (!found.is_empty()).then(|| found.swap_remove(i))
}

impl State {
    fn media_sender(&mut self) -> Sender<Option<NowPlaying>> {
        if let Some(tx) = &self.media.tx {
            return tx.clone();
        }
        let (tx, rx) = channel::channel();
        let _ = self.loop_handle.insert_source(rx, |ev, _, state| {
            if let channel::Event::Msg(now) = ev {
                if state.media.now != now {
                    state.media.now = now;
                    state.needs_redraw = true;
                }
            }
        });
        self.media.tx = Some(tx.clone());
        tx
    }

    /// Look up what's playing, now and every two seconds while the volume flyout is open.
    pub fn media_watch(&mut self) {
        let tx = self.media_sender();
        std::thread::spawn(move || {
            let _ = tx.send(query());
        });
        let _ = self.loop_handle.insert_source(Timer::from_duration(Duration::from_secs(2)), |_, _, state| {
            if state.tray.flyout == Some(Flyout::Volume) {
                let tx = state.media_sender();
                std::thread::spawn(move || {
                    let _ = tx.send(query());
                });
                return TimeoutAction::ToDuration(Duration::from_secs(2));
            }
            TimeoutAction::Drop
        });
    }

    /// Previous, PlayPause or Next on the player the flyout shows, then look again so the
    /// card follows.
    pub fn media_command(&mut self, method: &str) {
        if self.media.now.is_none() {
            return;
        }
        if method == "PlayPause" {
            if let Some(n) = self.media.now.as_mut() {
                n.playing = !n.playing;
            }
        }
        let player = self.media.now.as_ref().map(|n| n.player.clone()).unwrap_or_default();
        let tx = self.media_sender();
        let method = method.to_string();
        std::thread::spawn(move || {
            if let Ok(conn) = Connection::session() {
                if let Ok(p) = Proxy::new(&conn, player.as_str(), "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player") {
                    let _: zbus::Result<()> = p.call(method.as_str(), &());
                }
            }
            // Players take a moment to switch tracks.
            std::thread::sleep(Duration::from_millis(350));
            let _ = tx.send(query());
        });
        self.needs_redraw = true;
    }
}

#[cfg(test)]
mod tests {
    /// Talks to the real session bus: `cargo test -- --ignored media_query`.
    #[test]
    #[ignore]
    fn media_query() {
        let now = super::query();
        println!(
            "player found: {}, title len {}, artist len {}, playing {:?}",
            now.is_some(),
            now.as_ref().map_or(0, |n| n.title.len()),
            now.as_ref().map_or(0, |n| n.artist.len()),
            now.as_ref().map(|n| n.playing)
        );
    }
}
