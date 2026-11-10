//! Rendering a project offline to a WAV file: the whole mix, or one track's stem
//! (that track alone, with its reverb and echo), for finishing in an editor such as
//! DaVinci Resolve's Fairlight page.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::engine::{Cmd, Engine, MixSettings, Shared};
use crate::fonts::{FontBank, make_instrument};
use crate::project::Project;
use crate::timeline::Timeline;

/// Resolve's native audio rate.
pub const EXPORT_RATE: u32 = 48_000;

/// An engine set up to play the project from the start, every track or one alone.
pub fn engine_for(p: &Project, fonts: &FontBank, sample_rate: u32, solo: Option<usize>) -> (Engine, f64) {
    let mut p = p.clone();
    if let Some(s) = solo {
        for (i, t) in p.tracks.iter_mut().enumerate() {
            t.strip.solo = i == s;
            t.strip.mute = false;
        }
    }
    let tl = Arc::new(Timeline::from_project(&p));
    let duration = tl.duration;
    let mut e = Engine::new(sample_rate, Shared::new());
    e.handle(Cmd::Mix(Box::new(MixSettings::of(&p))));
    e.handle(Cmd::Instruments(p.tracks.iter().map(|t| make_instrument(&t.source, fonts, sample_rate).0).collect()));
    e.handle(Cmd::Timeline(tl));
    e.handle(Cmd::Play(0.0));
    (e, duration)
}

/// The longest a reverb or echo tail is let ring after the music, in seconds.
pub const MAX_TAIL_SECS: f64 = 60.0;
/// Quieter than this (-84 dBFS) counts as silence when the tail has rung out.
const QUIET: f32 = 6.3e-5;

/// Render to a 24-bit stereo WAV. `progress` counts to 1000; setting `cancel` stops
/// early and removes the file. The file holds the music and then rings on until its
/// reverb, echo and releases have died away (to -84 dBFS, at most a minute), so nothing
/// is cut off. Returns its length in seconds.
pub fn render_wav(
    p: &Project,
    fonts: &FontBank,
    solo: Option<usize>,
    path: &Path,
    progress: &AtomicU32,
    cancel: &AtomicBool,
) -> Result<f64, String> {
    let sr = EXPORT_RATE;
    let (mut e, _) = engine_for(p, fonts, sr, solo);
    let music = (p.music_secs() * sr as f64).round() as usize;
    let spec = hound::WavSpec { channels: 2, sample_rate: sr, bits_per_sample: 24, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    let full = 8_388_607.0;
    let write = |w: &mut hound::WavWriter<_>, l: &[f32], r: &[f32]| -> Result<(), String> {
        for (a, b) in l.iter().zip(r) {
            w.write_sample((a.clamp(-1.0, 1.0) * full) as i32).map_err(|e| e.to_string())?;
            w.write_sample((b.clamp(-1.0, 1.0) * full) as i32).map_err(|e| e.to_string())?;
        }
        Ok(())
    };
    let cancelled = |w: hound::WavWriter<_>| {
        drop(w);
        let _ = std::fs::remove_file(path);
        Err::<f64, String>("Export cancelled".into())
    };
    // the music, streamed to the file (progress to 90%)
    let (mut l, mut r) = (vec![0.0f32; 1024], vec![0.0f32; 1024]);
    let mut done = 0;
    while done < music {
        if cancel.load(Ordering::Relaxed) {
            return cancelled(w);
        }
        let n = (music - done).min(l.len());
        e.render(&mut l[..n], &mut r[..n]);
        write(&mut w, &l[..n], &r[..n])?;
        done += n;
        progress.store((done as f64 / music.max(1) as f64 * 900.0) as u32, Ordering::Relaxed);
    }
    // the tail, held until it has been quiet for half a second
    let (mut tl, mut tr) = (Vec::new(), Vec::new());
    let max = (MAX_TAIL_SECS * sr as f64) as usize;
    let (window, mut quiet_for, mut last_loud) = (sr as usize / 2, 0usize, 0usize);
    while tl.len() < max {
        if cancel.load(Ordering::Relaxed) {
            return cancelled(w);
        }
        e.render(&mut l, &mut r);
        for i in 0..l.len() {
            if l[i].abs() > QUIET || r[i].abs() > QUIET {
                quiet_for = 0;
                last_loud = tl.len() + i + 1;
            } else {
                quiet_for += 1;
            }
        }
        tl.extend_from_slice(&l);
        tr.extend_from_slice(&r);
        progress.store(900 + (tl.len() as f64 / max as f64 * 100.0) as u32, Ordering::Relaxed);
        // at least the old fixed tail, so a quiet moment just after the end isn't the end
        if quiet_for >= window && tl.len() as f64 >= crate::project::TAIL_SECS * sr as f64 * 0.5 {
            break;
        }
    }
    // keep a breath after the last audible sample, then fade the last 50 ms (inaudible
    // by then; it only matters when the minute ran out)
    let keep = (last_loud + sr as usize / 10).min(tl.len());
    tl.truncate(keep);
    tr.truncate(keep);
    let fade = (sr as usize / 20).min(keep);
    for i in 0..fade {
        let g = i as f32 / fade as f32;
        tl[keep - 1 - i] *= g;
        tr[keep - 1 - i] *= g;
    }
    write(&mut w, &tl, &tr)?;
    progress.store(1000, Ordering::Relaxed);
    w.finalize().map_err(|e| e.to_string())?;
    Ok((music + keep) as f64 / sr as f64)
}

/// Render into memory (for tests and previews).
pub fn render_buffers(p: &Project, fonts: &FontBank, sample_rate: u32, secs: Option<f64>) -> (Vec<f32>, Vec<f32>) {
    let (mut e, duration) = engine_for(p, fonts, sample_rate, None);
    let total = (secs.unwrap_or(duration) * sample_rate as f64) as usize;
    let (mut l, mut r) = (vec![0.0f32; total], vec![0.0f32; total]);
    for (a, b) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
        e.render(a, b);
    }
    (l, r)
}
