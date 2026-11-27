//! The `.kibble` document: a project and its gallery settings, as JSON.

use std::path::Path;

use dd_seq::project::Project;
use serde::{Deserialize, Serialize};

use crate::slideshow::{Accents, Fit, Grid, Plan, Slideshow};

pub const EXTENSION: &str = "kibble";
/// Projects saved before the rename; they open as before.
pub const OLD_EXTENSION: &str = "ddream";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Document {
    pub app: String,
    pub project: Project,
    pub gallery: Slideshow,
}

impl Default for Document {
    fn default() -> Self {
        Document { app: format!("kibble-481 {}", env!("CARGO_PKG_VERSION")), project: Project::default(), gallery: Slideshow::default() }
    }
}

impl Document {
    pub fn new(project: Project) -> Self {
        Document { project, ..Document::default() }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn load(path: &Path) -> Result<Document, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut doc: Document = serde_json::from_str(&s).map_err(|e| format!("{}: {e}", path.display()))?;
        // (field sounds saved on City tracks open as beds of ambience, D36)
        dd_seq::ambience::adopt_opened(&mut doc.project);
        Ok(doc)
    }

    /// Where the gallery's images fall in the music as it stands.
    pub fn plan(&self) -> Plan {
        plan(&self.project, &self.gallery, None)
    }

    /// The plan for this many photos: no more images than there are photos.
    pub fn plan_for(&self, photos: usize) -> Plan {
        plan(&self.project, &self.gallery, Some(photos))
    }
}

/// The gallery's plan for a project: the images the score as it stands holds (exactly
/// `hold` seconds each, or on bar lines, moved onto the music's accents if asked), no
/// more than there are photos when that's known.
pub fn plan(p: &Project, g: &Slideshow, photos: Option<usize>) -> Plan {
    let grid = Grid::of(p);
    // (older documents may ask to size the track; the calculator does that now)
    let g = Slideshow { fit: Fit::ImagesToTrack, ..g.clone() };
    let plan = Plan::for_track(&g, grid, p.music_secs());
    let follow = g.follow_music && !g.exact && !g.even_bars();
    match photos {
        // the score holds them all: as many images as photos, the accents only moving
        // where each change falls (else following the music could hold fewer than the
        // calculator promised)
        Some(n) if n > 0 && follow && plan.images >= n as u32 => {
            let g = Slideshow { fit: Fit::TrackToImages, ..g };
            plan.limit(n as u32).follow(&g, &Accents::of(p))
        }
        Some(n) if n > 0 => if follow { plan.follow(&g, &Accents::of(p)) } else { plan }.limit(n as u32),
        _ if follow => plan.follow(&g, &Accents::of(p)),
        _ => plan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slideshow::Calc;
    use dd_seq::compose::{Recipe, compose};

    /// With the chords, on composed pieces in every style and at several tempos, each
    /// image starts on a chord change (or halfway through a chord), never three bars
    /// into four; in square phrases, every 2, 4 or 8 bars.
    #[test]
    fn images_change_with_the_chords() {
        use dd_seq::compose::{Style, chord_bars_at};
        for (k, style) in Style::ALL.into_iter().enumerate() {
            for bpm in [48.0, 66.0, 90.0, 124.0] {
                for square in [false, true] {
                    let r = Recipe { style, square, ..Recipe::random(k as u64) };
                    let p = compose(&r, bpm, style.meter_from((4, 4)), 32);
                    let cb = chord_bars_at(&p, p.bpm);
                    if square {
                        assert!(cb.is_power_of_two(), "{} at {bpm}: {cb} bars a chord", style.label());
                    }
                    let g = Slideshow { hold: 12.0, ..Slideshow::default() };
                    let plan = plan(&p, &g, None);
                    let grid = Grid::of(&p);
                    let n = crate::slideshow::phrase_bars(&g, &grid);
                    for t in plan.starts() {
                        let bar = (t / grid.bar()).round() as u32;
                        assert_eq!(bar % n.min(cb), 0, "{} at {bpm}: image at bar {bar}, {cb} bars a chord", style.label());
                    }
                }
            }
        }
    }

    /// Set to the calculator's length, the score holds every photo, following the music
    /// or not.
    #[test]
    fn the_calculators_length_holds_every_photo() {
        for (seed, photos, hold, follow) in [(1u64, 30u32, 12.0f32, true), (2, 30, 12.0, false), (3, 12, 8.0, true), (4, 50, 6.0, true)] {
            let g = Slideshow { hold, follow_music: follow, ..Slideshow::default() };
            let r = Recipe::random(seed);
            let p = compose(&r, 66.0, (4, 4), 24);
            let grid = Grid::new(p.bpm, p.meter);
            let c = Calc::new(&g, grid, p.music_secs(), photos);
            let p = compose(&r, 66.0, (4, 4), c.needed_bars);
            let plan = plan(&p, &g, Some(photos as usize));
            assert_eq!(plan.images, photos, "seed {seed}, {photos} photos, follow {follow}");
            assert!(plan.starts.windows(2).all(|w| w[1] > w[0]));
            assert!(plan.end <= p.music_secs() + 1e-6);
        }
    }
}
