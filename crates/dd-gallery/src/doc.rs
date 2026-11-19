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
        serde_json::from_str(&s).map_err(|e| format!("{}: {e}", path.display()))
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
    let grid = Grid::new(p.bpm, p.meter);
    // (older documents may ask to size the track; the calculator does that now)
    let g = Slideshow { fit: Fit::ImagesToTrack, ..g.clone() };
    let plan = Plan::for_track(&g, grid, p.music_secs());
    let follow = g.follow_music && !g.exact;
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
