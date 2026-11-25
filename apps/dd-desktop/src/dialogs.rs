//! File and folder dialogs that don't block the window.
//!
//! A dialog opened from inside a frame (rfd's blocking `FileDialog`) runs its own modal
//! loop while winit is still handling that frame's event. Dragging a file or folder into
//! the dialog makes AppKit deliver winit a drag event re-entrantly, winit panics inside an
//! Objective-C callback and the app aborts (0.7.0, macOS: Choose photo folder… and a
//! folder dragged onto the dialog). So every dialog is rfd's `AsyncFileDialog`: created
//! here on the main thread (on macOS a sheet on our window, on Windows its own thread),
//! waited on from a thread, and its answer picked up by [`App::poll_dialog`] on a later
//! frame.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use crate::app::App;

/// What to do with the path the dialog gives.
pub(crate) enum Want {
    Open,
    MergeMidi,
    Save,
    SaveMidi,
    ExportWav,
    Stems,
    Resolve,
    Video,
    PhotoFolder,
    /// Write this patch (as JSON) to the file chosen.
    SavePatch(String),
    /// Load a patch into this track.
    LoadPatch(usize),
    /// Load a SoundFont onto this track.
    SoundFont(usize),
}

pub(crate) enum Kind {
    File,
    Folder,
    Save,
}

pub(crate) struct Asking {
    want: Want,
    rx: Receiver<Option<PathBuf>>,
}

impl App {
    /// Open `dialog` without blocking; the answer is acted on as `want` says. While one
    /// dialog is open, asking for another does nothing.
    pub(crate) fn ask(&mut self, want: Want, kind: Kind, dialog: rfd::AsyncFileDialog) {
        if self.asking.is_some() {
            return;
        }
        // made now, on the main thread, where the dialog opens; only the waiting is elsewhere
        let pending: Pin<Box<dyn Future<Output = Option<rfd::FileHandle>> + Send>> = match kind {
            Kind::File => Box::pin(dialog.pick_file()),
            Kind::Folder => Box::pin(dialog.pick_folder()),
            Kind::Save => Box::pin(dialog.save_file()),
        };
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(pollster::block_on(pending).map(|h| h.path().to_path_buf()));
        });
        self.asking = Some(Asking { want, rx });
    }

    /// Act on the open dialog's answer once it has one.
    pub(crate) fn poll_dialog(&mut self, ctx: &eframe::egui::Context) {
        let Some(a) = &self.asking else { return };
        let path = match a.rx.try_recv() {
            Ok(p) => p,
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
                return;
            }
            Err(TryRecvError::Disconnected) => None,
        };
        let Some(Asking { want, .. }) = self.asking.take() else { return };
        let Some(path) = path else { return };
        match want {
            Want::Open => self.open_path(path),
            Want::MergeMidi => self.merge_midi_from(path),
            Want::Save => self.save_to(path.with_extension(crate::app::EXTENSION)),
            Want::SaveMidi => self.save_midi_to(path),
            Want::ExportWav => self.export_wav_to(path),
            Want::Stems => self.export_stems_to(path),
            Want::Resolve => self.export_resolve_to(path),
            Want::Video => self.export_video_to(path),
            Want::PhotoFolder => self.doc.gallery.folder = Some(path),
            Want::SavePatch(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    self.status = format!("Couldn't save the patch: {e}");
                }
            }
            Want::LoadPatch(track) => self.load_patch_from(track, path),
            Want::SoundFont(track) => self.load_font_from(track, path),
        }
    }
}
