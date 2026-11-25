//! Reordering the gallery's photos by hand (D34): a pane of thumbnails, dragged into the
//! order the exports use (the Resolve timeline and the HD video). Thumbnails are read on
//! a few background threads, small (`THUMB` pixels at most), so a big folder opens at
//! once and fills in.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, Sense, Stroke, vec2};

use crate::theme::Tokens;

/// The longest side of a thumbnail, in pixels.
const THUMB: u32 = 320;
/// A tile on screen, in points.
const TILE: egui::Vec2 = vec2(168.0, 150.0);
const PICTURE_H: f32 = 116.0;

/// What the pane asks of the app when it closes.
pub enum Outcome {
    /// Keep this order.
    Save(Vec<PathBuf>),
    Cancel,
}

pub struct Reorder {
    /// The photos in the order being set.
    order: Vec<PathBuf>,
    /// The folder's own order, to go back to.
    folder_order: Vec<PathBuf>,
    thumbs: HashMap<PathBuf, Option<egui::TextureHandle>>,
    rx: Receiver<(PathBuf, Option<egui::ColorImage>)>,
    /// The tile last clicked, for the buttons and the arrow keys.
    selected: Option<usize>,
}

impl Reorder {
    /// Open on `order` (the photos as the exports have them now); `folder_order` is the
    /// folder's listing by name or date.
    pub fn new(ctx: &egui::Context, order: Vec<PathBuf>, folder_order: Vec<PathBuf>) -> Reorder {
        let (tx, rx) = channel();
        let queue = Arc::new(Mutex::new(order.iter().cloned().collect::<VecDeque<_>>()));
        let workers = std::thread::available_parallelism().map_or(2, |n| n.get().clamp(1, 4));
        for _ in 0..workers {
            let (queue, tx, ctx) = (queue.clone(), tx.clone(), ctx.clone());
            std::thread::spawn(move || {
                while let Some(path) = queue.lock().ok().and_then(|mut q| q.pop_front()) {
                    let img = thumbnail(&path);
                    if tx.send((path, img)).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            });
        }
        Reorder { order, folder_order, thumbs: HashMap::new(), rx, selected: None }
    }

    /// Draw the pane; `Some` once it's saved or cancelled.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Outcome> {
        while let Ok((path, img)) = self.rx.try_recv() {
            let tex = img.map(|i| ctx.load_texture(format!("thumb {}", path.display()), i, egui::TextureOptions::LINEAR));
            self.thumbs.insert(path, tex);
        }
        let t = Tokens::get(ctx);
        let screen = ctx.content_rect();
        let mut outcome = None;
        egui::Modal::new(egui::Id::unique("reorder-photos")).show(ctx, |ui| {
            let w = (screen.width() - 80.0).clamp(360.0, 1400.0);
            ui.set_width(w);
            ui.horizontal(|ui| {
                ui.heading("Reorder images");
                ui.label(
                    egui::RichText::new(format!(
                        "{} photos · drag a picture to where it should go; the first opens the gallery",
                        self.order.len()
                    ))
                    .color(t.text_dim),
                );
            });
            ui.add_space(6.0);
            let mut moved: Option<(usize, usize)> = None;
            egui::ScrollArea::vertical().max_height((screen.height() - 200.0).max(200.0)).auto_shrink([false, true]).show(ui, |ui| {
                // rows of as many tiles as fit (laid out by hand: each tile is its own
                // drag source, which a wrapping layout doesn't wrap)
                let gap = 10.0;
                let cols = (((ui.available_width() + gap) / (TILE.x + gap)).floor() as usize).max(1);
                ui.spacing_mut().item_spacing = vec2(gap, gap);
                let len = self.order.len();
                for row in 0..len.div_ceil(cols) {
                    ui.horizontal(|ui| {
                        for i in row * cols..((row + 1) * cols).min(len) {
                            let path = self.order[i].clone();
                            let thumb = self.thumbs.get(&path);
                            let selected = self.selected == Some(i);
                            let inner = ui.dnd_drag_source(egui::Id::unique(("reorder-tile", &path)), i, |ui| {
                                tile(ui, &t, i, &path, thumb, selected)
                            });
                            let r = inner.inner;
                            if r.clicked() {
                                self.selected = Some(i);
                            }
                            r.context_menu(|ui| {
                                if ui.button("Move to the front").clicked() {
                                    moved = Some((i, 0));
                                    ui.close();
                                }
                                if ui.button("Move to the end").clicked() {
                                    moved = Some((i, len));
                                    ui.close();
                                }
                            });
                            // where a dragged picture would land: before this one, or after it
                            let after = ui.input(|inp| inp.pointer.interact_pos()).is_some_and(|p| p.x > r.rect.center().x);
                            if let Some(from) = r.dnd_hover_payload::<usize>()
                                && *from != i
                            {
                                let x = if after { r.rect.right() + gap / 2.0 } else { r.rect.left() - gap / 2.0 };
                                ui.painter().line_segment(
                                    [egui::pos2(x, r.rect.top()), egui::pos2(x, r.rect.bottom())],
                                    Stroke::new(3.0, t.accent),
                                );
                            }
                            if let Some(from) = r.dnd_release_payload::<usize>() {
                                moved = Some((*from, if after { i + 1 } else { i }));
                            }
                        }
                    });
                }
            });
            if let Some((from, to)) = moved {
                self.selected = Some(move_photo(&mut self.order, from, to));
            }
            // the arrow keys move the selected picture
            if let Some(i) = self.selected.filter(|i| *i < self.order.len()) {
                let (l, r) = ui.input(|inp| (inp.key_pressed(egui::Key::ArrowLeft), inp.key_pressed(egui::Key::ArrowRight)));
                if l && i > 0 {
                    self.selected = Some(move_photo(&mut self.order, i, i - 1));
                } else if r && i + 1 < self.order.len() {
                    self.selected = Some(move_photo(&mut self.order, i, i + 2));
                }
            }
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                let sel = self.selected.filter(|i| *i < self.order.len());
                if ui
                    .add_enabled(sel.is_some_and(|i| i > 0), egui::Button::new("Move to front"))
                    .on_hover_text("The picture clicked last, first")
                    .clicked()
                    && let Some(i) = sel
                {
                    self.selected = Some(move_photo(&mut self.order, i, 0));
                }
                if ui.add_enabled(sel.is_some(), egui::Button::new("Move to end")).clicked()
                    && let Some(i) = sel
                {
                    let end = self.order.len();
                    self.selected = Some(move_photo(&mut self.order, i, end));
                }
                if ui.button("Folder order").on_hover_text("Back to the folder's order, by name or date").clicked() {
                    self.order = self.folder_order.clone();
                    self.selected = None;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Save and close").strong()).clicked() {
                        outcome = Some(Outcome::Save(self.order.clone()));
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = Some(Outcome::Cancel);
                    }
                    ui.label(egui::RichText::new("← → move the selected picture").color(t.text_dim));
                });
            });
        });
        outcome
    }
}

/// Move the photo at `from` to land before the one at `to` (the end at `len`); where it
/// lands.
fn move_photo(order: &mut Vec<PathBuf>, from: usize, to: usize) -> usize {
    if from >= order.len() {
        return from;
    }
    let p = order.remove(from);
    let to = if to > from { to - 1 } else { to }.min(order.len());
    order.insert(to, p);
    to
}

/// One picture: its thumbnail, fitted, its place and its name.
fn tile(
    ui: &mut egui::Ui,
    t: &Tokens,
    i: usize,
    path: &Path,
    thumb: Option<&Option<egui::TextureHandle>>,
    selected: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(TILE, Sense::click());
    let p = ui.painter();
    let hovered = resp.hovered();
    p.rect_filled(
        rect,
        6.0,
        if selected {
            t.pressed
        } else if hovered {
            t.hover
        } else {
            t.inset
        },
    );
    if selected {
        p.rect_stroke(rect, 6.0, Stroke::new(2.0, t.accent), egui::StrokeKind::Inside);
    }
    let pic = egui::Rect::from_min_size(rect.min + vec2(6.0, 6.0), vec2(TILE.x - 12.0, PICTURE_H));
    p.rect_filled(pic, 3.0, Color32::BLACK);
    match thumb {
        Some(Some(tex)) => {
            let size = tex.size_vec2();
            let k = (pic.width() / size.x).min(pic.height() / size.y);
            let r = egui::Rect::from_center_size(pic.center(), size * k);
            p.image(tex.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
        }
        Some(None) => {
            p.text(pic.center(), egui::Align2::CENTER_CENTER, "Can't show this one", egui::FontId::proportional(11.0), t.text_dim);
        }
        None => {
            p.text(pic.center(), egui::Align2::CENTER_CENTER, "…", egui::FontId::proportional(16.0), t.text_dim);
        }
    }
    // its place, over the picture's corner
    let badge = egui::Rect::from_min_size(pic.min + vec2(4.0, 4.0), vec2(if i >= 99 { 34.0 } else { 26.0 }, 18.0));
    p.rect_filled(badge, 4.0, if i == 0 { t.accent } else { Color32::from_black_alpha(170) });
    p.text(
        badge.center(),
        egui::Align2::CENTER_CENTER,
        format!("{}", i + 1),
        egui::FontId::proportional(11.5),
        if i == 0 { t.on_accent } else { Color32::WHITE },
    );
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let name = if name.chars().count() > 24 { format!("{}…", name.chars().take(23).collect::<String>()) } else { name };
    p.text(
        egui::pos2(rect.center().x, pic.bottom() + 14.0),
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(11.0),
        t.text_label,
    );
    resp.on_hover_text(path.display().to_string())
}

/// A photo's thumbnail, turned as its camera says; ffmpeg reads what the picture library
/// can't (HEIC, raw), if it's there.
fn thumbnail(path: &Path) -> Option<egui::ColorImage> {
    let read = || -> Option<image::DynamicImage> {
        use image::ImageDecoder;
        let mut dec = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.into_decoder().ok()?;
        let turn = dec.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
        let mut img = image::DynamicImage::from_decoder(dec).ok()?.thumbnail(THUMB, THUMB);
        img.apply_orientation(turn);
        Some(img)
    };
    let img = read().or_else(|| {
        let png = dd_gallery::video::thumbnail_png(path, THUMB)?;
        image::load_from_memory_with_format(&png, image::ImageFormat::Png).ok()
    })?;
    let rgba = img.to_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photos_move_to_where_they_are_dropped() {
        let v = |s: &str| s.chars().map(|c| PathBuf::from(c.to_string())).collect::<Vec<_>>();
        let mut o = v("abcde");
        // d dropped before b
        assert_eq!(move_photo(&mut o, 3, 1), 1);
        assert_eq!(o, v("adbce"));
        // a dropped after c (before e)
        assert_eq!(move_photo(&mut o, 0, 4), 3);
        assert_eq!(o, v("dbcae"));
        // to the end, and to the front
        move_photo(&mut o, 0, 5);
        assert_eq!(o, v("bcaed"));
        move_photo(&mut o, 4, 0);
        assert_eq!(o, v("dbcae"));
        // dropped on itself: no change
        move_photo(&mut o, 2, 2);
        move_photo(&mut o, 2, 3);
        assert_eq!(o, v("dbcae"));
    }
}
