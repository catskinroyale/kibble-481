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
        plan(&self.project, &self.gallery)
    }
}

/// The gallery's plan for a project: images counted for the track, or the track
/// sized for the images, then moved onto the music's accents if asked.
pub fn plan(p: &Project, g: &Slideshow) -> Plan {
    let grid = Grid::new(p.bpm, p.meter);
    let plan = match g.fit {
        Fit::ImagesToTrack => Plan::for_track(g, grid, p.music_secs()),
        Fit::TrackToImages => Plan::for_images(g, grid),
    };
    if g.follow_music { plan.follow(g, &Accents::of(p)) } else { plan }
}
