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
    let plan = if g.follow_music && !g.exact { plan.follow(&g, &Accents::of(p)) } else { plan };
    match photos {
        Some(n) if n > 0 => plan.limit(n as u32),
        _ => plan,
    }
}
