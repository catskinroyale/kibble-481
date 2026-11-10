//! Everything DaVinci Resolve needs for a gallery, in one folder: the soundtrack, a
//! timeline with each photo at its change and a dissolve between them, the changes as
//! markers, and a cue list.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32};

use dd_seq::fonts::FontBank;
use dd_seq::render::render_wav;

use crate::doc::Document;
use crate::resolve;

#[derive(Debug, Default)]
pub struct Exported {
    pub wav: PathBuf,
    pub timeline: Option<PathBuf>,
    pub markers: PathBuf,
    pub cues: PathBuf,
    pub stems: Vec<PathBuf>,
}

fn safe_name(s: &str) -> String {
    let n: String = s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == ' ' { c } else { '_' }).collect();
    let n = n.trim().to_string();
    if n.is_empty() { "Untitled".into() } else { n }
}

/// Write `<dir>/<title>.wav`, `.fcpxml` (when there are photos), `.edl` and
/// `-cues.csv`, and with `stems`, `<title> - <track>.wav` for every track.
pub fn export_for_resolve(
    doc: &Document,
    fonts: &FontBank,
    dir: &Path,
    stems: bool,
    progress: &AtomicU32,
    cancel: &AtomicBool,
) -> Result<Exported, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let p = &doc.project;
    let g = &doc.gallery;
    let title = safe_name(&p.title);
    let plan = doc.plan();
    let wav = dir.join(format!("{title}.wav"));
    let audio_secs = render_wav(p, fonts, None, &wav, progress, cancel)?;
    let mut out = Exported { wav: wav.clone(), ..Exported::default() };
    if stems {
        for (i, t) in p.tracks.iter().enumerate() {
            if t.notes.is_empty() {
                continue;
            }
            let path = dir.join(format!("{title} - {}.wav", safe_name(&t.name)));
            render_wav(p, fonts, Some(i), &path, progress, cancel)?;
            out.stems.push(path);
        }
    }
    if let Some(folder) = &g.folder {
        let photos = resolve::photos(folder, g.order)?;
        if !photos.is_empty() {
            let xml = resolve::fcpxml(&plan, &photos, &wav, audio_secs, g.fade as f64, g.fps, &p.title);
            let path = dir.join(format!("{title}.fcpxml"));
            std::fs::write(&path, xml).map_err(|e| e.to_string())?;
            out.timeline = Some(path);
        }
    }
    out.markers = dir.join(format!("{title}.edl"));
    std::fs::write(&out.markers, resolve::marker_edl(&plan, g.fps, &p.title)).map_err(|e| e.to_string())?;
    out.cues = dir.join(format!("{title}-cues.csv"));
    std::fs::write(&out.cues, plan.cue_csv(g.fade as f64)).map_err(|e| e.to_string())?;
    Ok(out)
}
