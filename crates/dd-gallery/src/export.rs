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

/// A name safe for a file on any system: letters, digits, spaces, `-`, `.` and `_`.
pub fn safe_name(s: &str) -> String {
    let n: String = s.chars().map(|c| if c.is_alphanumeric() || matches!(c, '-' | ' ' | '.') { c } else { '_' }).collect();
    let n = n.trim().to_string();
    if n.is_empty() { "Untitled".into() } else { n }
}

/// The name to save or export a version of a piece under: its title, its seed (when
/// composed) and when, in UTC, so versions of one piece sort and stay apart, e.g.
/// `Last Monorail - seed 123456 - 2026-11-11T142233Z`.
pub fn version_name(p: &dd_seq::project::Project, when: std::time::SystemTime) -> String {
    let secs = when.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let seed = p.recipe.map_or(String::new(), |r| format!(" - seed {}", r.seed));
    safe_name(&format!("{}{seed} - {}", p.title, utc_stamp(secs)))
}

/// `YYYY-MM-DDTHHMMSSZ` for seconds since 1970 (days to a civil date after Howard
/// Hinnant's algorithm).
fn utc_stamp(secs: u64) -> String {
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}{:02}{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Write `<dir>/<name>.wav`, `.fcpxml` (when there are photos), `.edl` and
/// `-cues.csv`, and with `stems`, `<name> - <track>.wav` for every track. The name is
/// usually `version_name`.
pub fn export_for_resolve(
    doc: &Document,
    fonts: &FontBank,
    dir: &Path,
    name: &str,
    stems: bool,
    progress: &AtomicU32,
    cancel: &AtomicBool,
) -> Result<Exported, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let p = &doc.project;
    let g = &doc.gallery;
    let title = safe_name(name);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn versions_are_named_by_title_seed_and_utc_time() {
        assert_eq!(utc_stamp(0), "1970-01-01T000000Z");
        assert_eq!(utc_stamp(951_782_400), "2000-02-29T000000Z");
        assert_eq!(utc_stamp(1_794_407_553), "2026-11-11T143233Z");
        let mut p = dd_seq::compose::compose(&dd_seq::compose::Recipe { seed: 4242, ..Default::default() }, 66.0, (4, 4), 8);
        p.title = "Last: Monorail".into();
        let when = UNIX_EPOCH + Duration::from_secs(1_794_407_553);
        assert_eq!(version_name(&p, when), "Last_ Monorail - seed 4242 - 2026-11-11T143233Z");
        p.recipe = None;
        assert_eq!(version_name(&p, when), "Last_ Monorail - 2026-11-11T143233Z");
    }
}
