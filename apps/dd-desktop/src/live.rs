//! Playing live: MIDI keyboards (through midir), the computer keyboard and the
//! on-screen keys, ribbon, wheel and touch strip. Every message goes straight to the
//! audio thread for the record-armed track, and into the take while recording,
//! stamped with the moment it was heard.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use dd_seq::engine::{Cmd, Shared};
use dd_seq::record::Take;
use dd_seq::timeline::Msg;
use eframe::egui;

pub struct Live {
    tx: Option<Sender<Cmd>>,
    shared: Option<Arc<Shared>>,
    /// The track played live.
    pub track: AtomicUsize,
    pub take: Mutex<Option<Take>>,
    /// Velocity of each key held now, 0 when up; for the on-screen keyboard.
    pub held: Mutex<[u8; 128]>,
    ctx: egui::Context,
}

impl Live {
    pub fn new(tx: Option<Sender<Cmd>>, shared: Option<Arc<Shared>>, ctx: egui::Context) -> Arc<Live> {
        Arc::new(Live { tx, shared, track: AtomicUsize::new(0), take: Mutex::new(None), held: Mutex::new([0; 128]), ctx })
    }

    pub fn play(&self, msg: Msg) {
        let track = self.track.load(Ordering::Relaxed);
        if let Some(tx) = &self.tx {
            let _ = tx.send(Cmd::Live(track, msg));
        }
        if let Some(take) = self.take.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            let t = self.shared.as_ref().map_or(0.0, |s| s.heard_now());
            take.push(t, msg);
        }
        let mut held = self.held.lock().unwrap_or_else(|e| e.into_inner());
        match msg {
            Msg::On(k, v) => held[k as usize & 127] = v,
            Msg::Off(k) => held[k as usize & 127] = 0,
            _ => {}
        }
        drop(held);
        self.ctx.request_repaint();
    }

    /// Let go of every key held (when the target track changes, or on panic).
    pub fn release_all(&self) {
        let keys: Vec<u8> = {
            let held = self.held.lock().unwrap_or_else(|e| e.into_inner());
            (0..128u8).filter(|k| held[*k as usize] > 0).collect()
        };
        for k in keys {
            self.play(Msg::Off(k));
        }
    }

    pub fn held(&self) -> [u8; 128] {
        *self.held.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn recording(&self) -> bool {
        self.take.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }
}

/// Parse a MIDI channel message (any channel).
pub fn parse(bytes: &[u8]) -> Option<Msg> {
    let (&status, rest) = bytes.split_first()?;
    let d1 = rest.first().copied().unwrap_or(0) & 127;
    let d2 = rest.get(1).copied().unwrap_or(0) & 127;
    match status & 0xF0 {
        0x90 if d2 > 0 => Some(Msg::On(d1, d2)),
        0x80 | 0x90 => Some(Msg::Off(d1)),
        0xB0 => Some(Msg::Cc(d1, d2)),
        0xE0 => Some(Msg::Bend(((d2 as i16) << 7 | d1 as i16) - 8192)),
        0xD0 => Some(Msg::Pressure(d1)),
        // polyphonic aftertouch: treated as the channel's touch
        0xA0 => Some(Msg::Pressure(d2)),
        _ => None,
    }
}

/// MIDI input ports, and the connection to the one chosen.
pub struct MidiIn {
    conn: Option<midir::MidiInputConnection<()>>,
    pub port: Option<String>,
    pub error: Option<String>,
}

impl MidiIn {
    pub fn new() -> Self {
        MidiIn { conn: None, port: None, error: None }
    }

    pub fn ports() -> Vec<String> {
        let Ok(input) = midir::MidiInput::new(crate::theme::APP_NAME) else { return Vec::new() };
        input.ports().iter().filter_map(|p| input.port_name(p).ok()).collect()
    }

    pub fn connect(&mut self, name: Option<&str>, live: Arc<Live>) {
        self.conn = None;
        self.port = None;
        self.error = None;
        let Some(name) = name else { return };
        let result = (|| {
            let mut input = midir::MidiInput::new(crate::theme::APP_NAME).map_err(|e| e.to_string())?;
            input.ignore(midir::Ignore::All);
            let port =
                input.ports().into_iter().find(|p| input.port_name(p).ok().as_deref() == Some(name)).ok_or("That MIDI input has gone")?;
            input
                .connect(
                    &port,
                    "kibble-481-in",
                    move |_, bytes, _| {
                        if let Some(m) = parse(bytes) {
                            live.play(m);
                        }
                    },
                    (),
                )
                .map_err(|e| e.to_string())
        })();
        match result {
            Ok(c) => {
                self.conn = Some(c);
                self.port = Some(name.to_string());
            }
            Err(e) => self.error = Some(e),
        }
    }
}

/// The computer keyboard as a piano, two rows like most DAWs: A W S E D F T G Y H U
/// J K O L P ; play C up to E an octave above.
pub fn qwerty_key(key: egui::Key) -> Option<i32> {
    use egui::Key::*;
    Some(match key {
        A => 0,
        W => 1,
        S => 2,
        E => 3,
        D => 4,
        F => 5,
        T => 6,
        G => 7,
        Y => 8,
        H => 9,
        U => 10,
        J => 11,
        K => 12,
        O => 13,
        L => 14,
        P => 15,
        Semicolon => 16,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_channel_messages() {
        assert_eq!(parse(&[0x93, 60, 100]), Some(Msg::On(60, 100)));
        assert_eq!(parse(&[0x90, 60, 0]), Some(Msg::Off(60)));
        assert_eq!(parse(&[0x80, 61, 40]), Some(Msg::Off(61)));
        assert_eq!(parse(&[0xB0, 1, 64]), Some(Msg::Cc(1, 64)));
        assert_eq!(parse(&[0xE0, 0, 64]), Some(Msg::Bend(0)));
        assert_eq!(parse(&[0xE0, 127, 127]), Some(Msg::Bend(8191)));
        assert_eq!(parse(&[0xE0, 0, 0]), Some(Msg::Bend(-8192)));
        assert_eq!(parse(&[0xD0, 90]), Some(Msg::Pressure(90)));
        assert_eq!(parse(&[0xF8]), None);
    }
}
