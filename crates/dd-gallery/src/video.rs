//! A gallery as a finished video (D33): the photos at their changes over the soundtrack,
//! with the gallery's transition, in 1080p H.264 (an `.mp4` or `.mov`) at its frames a
//! second.
//!
//! Every frame is drawn here, from the same schedule as the exported timeline
//! (`shots`): frame `n` is the picture at `n / fps` seconds, so the changes land on the
//! frames the timeline and markers give, and the video runs exactly as long as the
//! soundtrack (rounded up to a whole frame). ffmpeg, found on the system, reads the
//! photos (scaled to fit the frame, black bars around) and encodes the frames and the
//! soundtrack; it isn't bundled.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use dd_seq::fonts::FontBank;
use dd_seq::render::render_wav;

use crate::doc::Document;
use crate::shots::{Shot, layers, shots};

/// HD: the frame's size.
pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;

/// Where ffmpeg is: `$KIBBLE_FFMPEG`, beside the app, on the `PATH`, or where Homebrew
/// and the usual installers put it.
pub fn ffmpeg() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let mut places: Vec<PathBuf> = Vec::new();
    if let Some(p) = std::env::var_os("KIBBLE_FFMPEG") {
        places.push(p.into());
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        places.push(dir.join(exe));
    }
    if let Some(path) = std::env::var_os("PATH") {
        places.extend(std::env::split_paths(&path).map(|d| d.join(exe)));
    }
    for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "C:\\ffmpeg\\bin"] {
        places.push(Path::new(d).join(exe));
    }
    places.into_iter().find(|p| p.is_file())
}

const NO_FFMPEG: &str = "Video export needs ffmpeg, which isn't bundled: install it (macOS: brew install ffmpeg; Windows: from ffmpeg.org, \
     with ffmpeg.exe beside Kibble-481 or on the PATH), or set KIBBLE_FFMPEG to it";

/// The H.264 encoder to use and its settings: x264 at a high quality for stills, else
/// the system's.
fn encoder(ff: &Path) -> Vec<String> {
    let list = Command::new(ff)
        .args(["-hide_banner", "-encoders"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let has = |name: &str| list.lines().any(|l| l.split_whitespace().nth(1) == Some(name));
    let v: &[&str] = if has("libx264") {
        &["-c:v", "libx264", "-preset", "medium", "-crf", "17", "-tune", "stillimage", "-profile:v", "high"]
    } else if has("h264_videotoolbox") {
        &["-c:v", "h264_videotoolbox", "-b:v", "16M", "-profile:v", "high"]
    } else if has("h264_mf") {
        &["-c:v", "h264_mf", "-b:v", "16M"]
    } else {
        &["-c:v", "mpeg4", "-q:v", "2"]
    };
    v.iter().map(|s| s.to_string()).collect()
}

/// A photo as the frame shows it: scaled to fit (turned as its camera says), centred
/// on black, as RGB bytes.
fn load(ff: &Path, photo: &Path) -> Result<Vec<u8>, String> {
    let fit = format!(
        "scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=decrease:flags=lanczos,pad={WIDTH}:{HEIGHT}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,format=rgb24"
    );
    let out = Command::new(ff)
        .args(["-v", "error", "-i"])
        .arg(photo)
        .args(["-frames:v", "1", "-vf", &fit, "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    let size = (WIDTH * HEIGHT * 3) as usize;
    if out.stdout.len() != size {
        let why = String::from_utf8_lossy(&out.stderr);
        return Err(why.lines().last().unwrap_or("not a picture ffmpeg reads").to_string());
    }
    Ok(out.stdout)
}

/// A small picture of a photo, as PNG bytes, read by ffmpeg (for the kinds a picture
/// library may not read, such as HEIC): no larger than `max` pixels either way.
pub fn thumbnail_png(photo: &Path, max: u32) -> Option<Vec<u8>> {
    let ff = ffmpeg()?;
    let fit = format!("scale={max}:{max}:force_original_aspect_ratio=decrease");
    let out = Command::new(ff)
        .args(["-v", "error", "-i"])
        .arg(photo)
        .args(["-frames:v", "1", "-vf", &fit, "-f", "image2pipe", "-vcodec", "png", "-"])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    (out.status.success() && !out.stdout.is_empty()).then_some(out.stdout)
}

/// What a video export did.
#[derive(Debug, Default)]
pub struct Made {
    pub frames: u64,
    pub fps: u32,
    pub secs: f64,
    /// The soundtrack's length (the music and its tail), in seconds.
    pub audio: f64,
    /// Photos that couldn't be read, shown as black.
    pub unread: Vec<String>,
}

/// Render the gallery as a video at `out` (`.mp4`, or `.mov`): the soundtrack first
/// (to a WAV beside it, removed after), then every frame. `progress` counts to 1000.
pub fn export_video(doc: &Document, fonts: &FontBank, out: &Path, progress: &AtomicU32, cancel: &AtomicBool) -> Result<Made, String> {
    let ff = ffmpeg().ok_or(NO_FFMPEG)?;
    let g = &doc.gallery;
    let folder = g.folder.as_ref().ok_or("Choose a photo folder first: the video shows its photos")?;
    let photos = g.photos()?;
    if photos.is_empty() {
        return Err(format!("No photos in {}", folder.display()));
    }
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let plan = doc.plan_for(photos.len());
    let fps = g.fps.max(1);
    // the soundtrack, to 15% of the way
    let wav = out.with_extension("soundtrack.wav");
    let part = AtomicU32::new(0);
    let audio = std::thread::scope(|s| {
        let job = s.spawn(|| render_wav(&doc.project, fonts, None, &wav, &part, cancel));
        while !job.is_finished() {
            progress.store(part.load(Ordering::Relaxed) * 150 / 1000, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        job.join().map_err(|_| "The soundtrack's render failed".to_string())?
    });
    let made = audio.and_then(|secs| {
        let shots = shots(&plan, photos.len(), g, fps, frames_for(secs, fps));
        encode(&ff, &photos, &shots, &wav, secs, fps, out, progress, cancel)
    });
    let _ = std::fs::remove_file(&wav);
    if made.is_err() {
        let _ = std::fs::remove_file(out);
    }
    made
}

/// The video's frames: as long as the soundtrack, to the next whole frame.
fn frames_for(audio_secs: f64, fps: u32) -> u64 {
    (audio_secs * fps as f64 - 1e-9).ceil().max(1.0) as u64
}

/// Every frame of `shots` over the soundtrack `wav` (`audio_secs` long) into `out`.
#[allow(clippy::too_many_arguments)]
fn encode(
    ff: &Path,
    photos: &[PathBuf],
    shots: &[Shot],
    wav: &Path,
    audio_secs: f64,
    fps: u32,
    out: &Path,
    progress: &AtomicU32,
    cancel: &AtomicBool,
) -> Result<Made, String> {
    let frames = frames_for(audio_secs, fps);
    let rate = fps.to_string();
    let mut cmd = Command::new(ff);
    cmd.args([
        "-y",
        "-v",
        "error",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgb24",
        "-s",
        &format!("{WIDTH}x{HEIGHT}"),
        "-framerate",
        &rate,
        "-i",
        "-",
        "-i",
    ])
    .arg(wav)
    .args(["-map", "0:v", "-map", "1:a"])
    .args(encoder(ff))
    // Rec. 709, as HD is
    .args(["-vf", "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p", "-colorspace", "bt709", "-color_primaries", "bt709"])
    .args(["-color_trc", "bt709", "-r", &rate, "-fps_mode", "cfr", "-g", &(fps * 2).to_string()])
    .args(["-c:a", "aac", "-b:a", "320k", "-ar", "48000", "-movflags", "+faststart"])
    .arg(out)
    .stdin(Stdio::piped())
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("Couldn't start ffmpeg: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("ffmpeg has no input")?;
    let mut stderr = child.stderr.take().ok_or("ffmpeg has no output")?;
    let errors = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let size = (WIDTH * HEIGHT * 3) as usize;
    let mut frame = vec![0u8; size];
    let mut cache: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut unread = Vec::new();
    let mut last: Option<Vec<(usize, u16)>> = None;
    let mut failed = None;
    for f in 0..frames {
        if cancel.load(Ordering::Relaxed) {
            failed = Some("Cancelled".to_string());
            break;
        }
        // to 1/256 of the way, which is all a byte shows
        let seen: Vec<(usize, u16)> =
            layers(shots, f).into_iter().map(|(j, a)| (j, (a * 256.0).round() as u16)).filter(|l| l.1 > 0).collect();
        if last.as_ref() != Some(&seen) {
            cache.retain(|j, _| seen.iter().any(|l| l.0 == *j) || shots[*j].from > f);
            frame.fill(0);
            for &(j, a) in &seen {
                let photo = &photos[shots[j].photo];
                let img = cache.entry(j).or_insert_with(|| {
                    load(ff, photo).unwrap_or_else(|e| {
                        unread.push(format!("{}: {e}", photo.display()));
                        vec![0; size]
                    })
                });
                if a >= 256 {
                    frame.copy_from_slice(img);
                } else {
                    let (a, b) = (a as u32, 256 - a as u32);
                    for (o, i) in frame.iter_mut().zip(img.iter()) {
                        *o = ((*o as u32 * b + *i as u32 * a + 128) >> 8) as u8;
                    }
                }
            }
            last = Some(seen);
        }
        if let Err(e) = stdin.write_all(&frame) {
            failed = Some(format!("ffmpeg stopped: {e}"));
            break;
        }
        progress.store(150 + (f * 850 / frames) as u32, Ordering::Relaxed);
    }
    drop(stdin);
    if failed.is_some() {
        let _ = child.kill();
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let errors = errors.join().unwrap_or_default();
    if let Some(e) = failed {
        let detail = errors.lines().last().map(|l| format!(" ({l})")).unwrap_or_default();
        return Err(if e == "Cancelled" { e } else { format!("{e}{detail}") });
    }
    if !status.success() {
        return Err(format!("ffmpeg couldn't make the video: {}", errors.lines().last().unwrap_or("no reason given")));
    }
    progress.store(1000, Ordering::Relaxed);
    Ok(Made { frames, fps, secs: frames as f64 / fps as f64, audio: audio_secs, unread })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slideshow::{Slideshow, Transition};

    /// A plain colour as a 24-bit BMP.
    fn bmp(path: &Path, w: u32, h: u32, rgb: [u8; 3]) {
        let row = (w * 3).div_ceil(4) * 4;
        let mut b = b"BM".to_vec();
        let le = |v: u32| v.to_le_bytes();
        b.extend(le(54 + row * h));
        b.extend([0; 4]);
        b.extend(le(54));
        b.extend(le(40));
        b.extend(le(w));
        b.extend(le(h));
        b.extend([1, 0, 24, 0]);
        b.extend([0; 24]);
        for _ in 0..h {
            for x in 0..row {
                b.push(if x < w * 3 { rgb[2 - (x % 3) as usize] } else { 0 });
            }
        }
        std::fs::write(path, b).unwrap();
    }

    fn probe(path: &Path, entries: &str) -> String {
        let ffprobe = ffmpeg().unwrap().with_file_name(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
        let o = Command::new(ffprobe)
            .args(["-v", "error", "-select_streams", "v:0", "-count_frames", "-show_entries", entries, "-of", "default=nw=1"])
            .arg(path)
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout).into_owned()
    }

    /// The frame `n` of a video, its middle pixel.
    fn pixel(path: &Path, n: u64) -> [u8; 3] {
        let o = Command::new(ffmpeg().unwrap())
            .args(["-v", "error", "-i"])
            .arg(path)
            .args(["-vf", &format!("select=eq(n\\,{n}),crop=2:2:959:539"), "-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
            .output()
            .unwrap();
        [o.stdout[0], o.stdout[1], o.stdout[2]]
    }

    /// 1080p, at the frame rate asked, exactly as long as the soundtrack, the photos'
    /// changes and fades on the frames the plan gives.
    #[test]
    fn hd_video_honours_the_frame_rate_and_timings() {
        if ffmpeg().is_none() {
            eprintln!("no ffmpeg: skipped");
            return;
        }
        let dir = std::env::temp_dir().join(format!("kibble-video-{}", std::process::id()));
        let photos = dir.join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        bmp(&photos.join("1.bmp"), 64, 48, [250, 20, 20]);
        bmp(&photos.join("2.bmp"), 48, 64, [20, 250, 20]);
        // 120 bpm in 4/4: 2 s bars; a one-bar intro, 4 s images, the outro and tail after
        let r = dd_seq::compose::Recipe { seed: 7, ..Default::default() };
        let p = dd_seq::compose::compose(&r, 120.0, (4, 4), 8);
        let gallery = Slideshow {
            folder: Some(photos.clone()),
            hold: 4.0,
            exact: true,
            follow_music: false,
            intro_bars: 1,
            outro_bars: 1,
            transition: Transition::FadeIn,
            fade_in: 1.0,
            fps: 30,
            ..Slideshow::default()
        };
        let doc = Document { project: p, gallery, ..Document::default() };
        let out = dir.join("gallery.mp4");
        let made = export_video(&doc, &FontBank::default(), &out, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
        assert!(made.unread.is_empty(), "{:?}", made.unread);
        let info = probe(&out, "stream=width,height,r_frame_rate,nb_read_frames");
        assert!(info.contains("width=1920") && info.contains("height=1080"), "{info}");
        assert!(info.contains("r_frame_rate=30/1"), "{info}");
        assert!(info.contains(&format!("nb_read_frames={}", made.frames)), "{info}");
        // the soundtrack rings on after the music: the video to its last whole frame
        assert_eq!(made.frames, (made.audio * 30.0).ceil() as u64);
        assert!(made.audio >= doc.project.music_secs());
        assert!(!dir.join("gallery.soundtrack.wav").exists());
        // the intro black; the first photo rising from black from 2 s (frame 60) over 1 s
        let lit = |c: [u8; 3]| c.iter().map(|v| *v as u32).sum::<u32>();
        assert!(lit(pixel(&out, 59)) < 40);
        let half = pixel(&out, 75);
        assert!((100..160).contains(&half[0]) && half[1] < 40, "{half:?}");
        let red = pixel(&out, 100);
        assert!(red[0] > 230 && red[1] < 40, "{red:?}");
        // cut to black at 6 s, the second rising
        assert!(lit(pixel(&out, 180)) < 40);
        let green = pixel(&out, 215);
        assert!(green[1] > 230 && green[0] < 40, "{green:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
