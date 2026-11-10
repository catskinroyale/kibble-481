//! Automated UI checks, from Ambient MIDI Generator (`snapshot.rs`, MIT). With
//! `KIBBLE_SCREENSHOT=window.bmp` the app saves a picture of its window once the first
//! frames have settled, then quits, without saving any settings.
//! `KIBBLE_SCREENSHOT_SHOW` can name things to show first, comma-separated: `mixer`,
//! `effects`, `score`, `gallery`, `about`, `sax` (select the sax track), `bells`
//! (select the bells track), `layer2` (the synth's second layer), `long` (lengthen the
//! piece to 48 bars), `fast` (124 bpm), `edited` (edit a note first),
//! `bass`, `city`, `drone` (select that track), `added` (add a Neon section).
//! `KIBBLE_SCREENSHOT_INPUT` plays input first, `;`-separated, six frames apart:
//! `c:x,y` clicks there, `w:x,y,dy` scrolls there (points).
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;

pub struct Snapshot {
    path: PathBuf,
    pub show: Vec<String>,
    frame: u32,
    done: Arc<AtomicBool>,
    input: Vec<String>,
}

impl Snapshot {
    pub fn from_env() -> Option<Snapshot> {
        let path = PathBuf::from(std::env::var_os("KIBBLE_SCREENSHOT")?);
        let show = std::env::var("KIBBLE_SCREENSHOT_SHOW")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let input = std::env::var("KIBBLE_SCREENSHOT_INPUT")
            .unwrap_or_default()
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Some(Snapshot { path, show, frame: 0, done: Arc::new(AtomicBool::new(false)), input })
    }

    /// Play the scripted input: each step gets six frames, from frame 20.
    pub fn inject(&mut self, raw: &mut egui::RawInput) {
        let f = self.frame as i64 - 20;
        if f < 0 || f % 6 > 2 {
            return;
        }
        let Some(step) = self.input.get((f / 6) as usize) else { return };
        let nums: Vec<f32> = step.get(2..).unwrap_or("").split(',').filter_map(|x| x.trim().parse().ok()).collect();
        if nums.len() < 2 {
            return;
        }
        let pos = egui::pos2(nums[0], nums[1]);
        raw.events.push(egui::Event::PointerMoved(pos));
        match (step.as_bytes()[0], f % 6) {
            (b'c', 1) => raw.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }),
            (b'c', 2) => raw.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }),
            (b'w', 1) => raw.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, nums.get(2).copied().unwrap_or(-300.0)),
                modifiers: Default::default(),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
            }),
            _ => {}
        }
    }

    pub fn wants(&self, what: &str) -> bool {
        self.show.iter().any(|s| s == what)
    }

    /// Call once a frame: takes the picture after a short settling time, and
    /// closes the window once it is saved.
    pub fn tick(&mut self, ctx: &egui::Context) {
        self.frame += 1;
        if self.done.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.frame == 45 + 6 * self.input.len() as u32 {
            let (path, done) = (self.path.clone(), self.done.clone());
            ctx.request_screenshot(move |image| {
                if let Err(e) = write_bmp(&path, &image) {
                    eprintln!("screenshot failed: {e}");
                }
                done.store(true, Ordering::Relaxed);
            });
        }
        ctx.request_repaint();
    }
}

/// A 24-bit uncompressed BMP: simple enough to need no image library.
fn write_bmp(path: &std::path::Path, image: &egui::ColorImage) -> std::io::Result<()> {
    let [w, h] = image.size;
    let row = (w * 3).div_ceil(4) * 4;
    let size = 54 + row * h;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in (0..h).rev() {
        let start = out.len();
        for x in 0..w {
            let c = image.pixels[y * w + x];
            out.extend_from_slice(&[c.b(), c.g(), c.r()]);
        }
        out.resize(start + row, 0);
    }
    std::fs::write(path, out)
}
