//! The mixer: a channel strip per track and the master. The fader law, meters and
//! knobs are Ambient MIDI Generator's (`mixer_ui.rs`, MIT).

use dd_seq::project::SILENT_DB;
use eframe::egui::{self, Align2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::App;
use crate::panels::toggle;
use crate::theme::Tokens;

const FADER_H: f32 = 110.0;
const TOP_DB: f32 = 6.0;
const STRIP_W: f32 = 92.0;

impl App {
    pub(crate) fn mixer(&mut self, ui: &mut Ui) {
        let t = Tokens::get(ui.ctx());
        let n = self.doc.project.tracks.len();
        let peaks: Vec<f32> = (0..n).map(|i| self.meter(i)).collect();
        let master = (self.meter(n), self.meter(n + 1));
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let sel = self.sel;
                let mut clicked = None;
                for (i, tr) in self.doc.project.tracks.iter_mut().enumerate() {
                    let frame = egui::Frame::new()
                        .fill(if i == sel { t.hover } else { t.inset })
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(5, 4));
                    // the strip's own click sits under its controls, so M, S and the faders get theirs
                    let r = ui.scope_builder(egui::UiBuilder::new().id_salt(("strip", i)).sense(Sense::click()), |ui| {
                        frame.show(ui, |ui| {
                            ui.set_width(STRIP_W);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                let c = crate::theme::track_colour(tr.color);
                                ui.add(egui::Label::new(egui::RichText::new(&tr.name).font(t.semibold(11.5)).color(c)).truncate());
                                let s = &mut tr.strip;
                                knob(
                                    ui,
                                    "Pan",
                                    &mut s.pan,
                                    -1.0..=1.0,
                                    0.0,
                                    |v| {
                                        if v.abs() < 0.01 {
                                            "C".into()
                                        } else if v < 0.0 {
                                            format!("L{:.0}", -v * 100.0)
                                        } else {
                                            format!("R{:.0}", v * 100.0)
                                        }
                                    },
                                    "Left or right",
                                    &t,
                                );
                                knob(ui, "Width", &mut s.width, 0.0..=2.0, 1.0, |v| format!("{:.0}%", v * 100.0), "Mono to extra wide", &t);
                                knob(
                                    ui,
                                    "Reverb",
                                    &mut s.reverb,
                                    0.0..=1.0,
                                    0.35,
                                    |v| format!("{:.0}%", v * 100.0),
                                    "Send to the reverb",
                                    &t,
                                );
                                knob(ui, "Echo", &mut s.echo, 0.0..=1.0, 0.0, |v| format!("{:.0}%", v * 100.0), "Send to the echo", &t);
                                ui.horizontal(|ui| {
                                    fader(ui, &mut s.gain_db, &t);
                                    meter(ui, &[peaks[i]], true, &t);
                                    ui.vertical(|ui| {
                                        toggle(ui, "M", &mut s.mute, t.mute, "Mute");
                                        toggle(ui, "S", &mut s.solo, t.solo, "Solo");
                                        toggle(ui, "E", &mut s.ensemble, t.accent, "Through the ensemble chorus (amount under Effects)");
                                    });
                                });
                                ui.label(egui::RichText::new(db_text(s.gain_db)).font(t.mono(10.5)).color(t.text_dim));
                            });
                        })
                    });
                    if r.response.clicked() {
                        clicked = Some(i);
                    }
                }
                if let Some(i) = clicked {
                    self.sel = i;
                }
                egui::Frame::new().fill(t.inset).corner_radius(4.0).inner_margin(egui::Margin::symmetric(6, 4)).show(ui, |ui| {
                    ui.vertical(|ui| {
                        // a fixed width, or past the scroll area's edge its labels wrap a
                        // letter to a line and make the mixer tall
                        ui.set_width(52.0);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.label(egui::RichText::new("Master").font(t.semibold(11.5)));
                        ui.add_space(4.0 * 18.0 + 6.0);
                        ui.horizontal(|ui| {
                            fader(ui, &mut self.doc.project.master_db, &t);
                            meter(ui, &[master.0, master.1], true, &t);
                        });
                        ui.label(egui::RichText::new(db_text(self.doc.project.master_db)).font(t.mono(10.5)).color(t.text_dim));
                    });
                });
            });
        });
    }
}

fn db_text(db: f32) -> String {
    if db <= SILENT_DB { "−∞ dB".into() } else { format!("{db:+.1} dB") }
}

/// A one-line labelled slider: a thin track filled from its resting value,
/// with the value shown while hovered. Double-click resets it.
#[allow(clippy::too_many_arguments)]
fn knob(
    ui: &mut Ui,
    label: &str,
    v: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    default: f32,
    value: impl Fn(f32) -> String,
    help: &str,
    t: &Tokens,
) {
    let (lo, hi) = (*range.start(), *range.end());
    let w = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
    let label_w = 44.0;
    let track = Rect::from_min_max(pos2(row.left() + label_w, row.center().y - 7.0), pos2(row.right() - 4.0, row.center().y + 7.0));
    let resp = ui.interact(
        track.expand2(vec2(4.0, 2.0)),
        ui.make_persistent_id(("knob", label, row.min.x as i32, row.min.y as i32)),
        Sense::click_and_drag(),
    );
    if (resp.dragged() || resp.clicked())
        && let Some(p) = resp.interact_pointer_pos()
    {
        let x = ((p.x - track.left()) / track.width()).clamp(0.0, 1.0);
        *v = lo + x * (hi - lo);
    }
    if resp.double_clicked() {
        *v = default;
    }
    let painter = ui.painter();
    let active = resp.hovered() || resp.dragged();
    let text = if active { value(*v) } else { label.to_string() };
    painter.text(pos2(row.left(), row.center().y), Align2::LEFT_CENTER, text, t.font(10.5), if active { t.text } else { t.text_label });
    let x_of = |x: f32| track.left() + track.width() * ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    let y = track.center().y;
    painter.hline(track.x_range(), y, Stroke::new(2.0, t.track));
    let (a, b) = (x_of(default), x_of(*v));
    painter.hline(a.min(b)..=a.max(b), y, Stroke::new(2.0, t.accent));
    painter.vline(a, (y - 3.0)..=(y + 3.0), Stroke::new(1.0, t.text_dim.gamma_multiply(0.6)));
    let col = if active { t.thumb_hover } else { t.thumb };
    painter.circle_filled(pos2(b, y), 4.0, col);
    resp.on_hover_text(format!("{label}: {} · {help}", value(*v)));
}

/// A vertical fader in dB. Double-click resets it to 0 dB.
fn fader(ui: &mut Ui, db: &mut f32, t: &Tokens) {
    let (r, resp) = ui.allocate_exact_size(vec2(18.0, FADER_H), Sense::click_and_drag());
    let y_of = |d: f32| r.bottom() - 6.0 - (r.height() - 12.0) * db_pos(d);
    if (resp.dragged() || resp.clicked())
        && let Some(p) = resp.interact_pointer_pos()
    {
        let x = ((r.bottom() - 6.0 - p.y) / (r.height() - 12.0)).clamp(0.0, 1.0);
        *db = pos_db(x);
    }
    if resp.double_clicked() {
        *db = 0.0;
    }
    let painter = ui.painter();
    let track = Rect::from_center_size(pos2(r.center().x, r.center().y), vec2(3.0, r.height() - 12.0));
    painter.rect_filled(track, 1.5, t.track);
    let zero = y_of(0.0);
    painter.hline(r.x_range(), zero, Stroke::new(1.0, t.text_dim.gamma_multiply(0.6)));
    let y = y_of(*db);
    let cap = Rect::from_center_size(pos2(r.center().x, y), vec2(16.0, 9.0));
    let fill = if resp.hovered() || resp.dragged() { t.thumb_hover } else { t.thumb };
    painter.rect_filled(cap, 2.5, fill);
    painter.hline(cap.x_range().shrink(3.0), y, Stroke::new(1.0, t.inset));
    resp.on_hover_text(format!("{} · drag to set, double-click for 0 dB", db_text(*db)));
}

/// Fader travel: the top quarter covers -6 to +6 dB, the rest down to silence.
pub(crate) fn db_pos(db: f32) -> f32 {
    if db <= SILENT_DB {
        0.0
    } else if db >= -6.0 {
        0.75 + 0.25 * (db + 6.0) / (TOP_DB + 6.0)
    } else {
        0.75 * ((db - SILENT_DB) / (-6.0 - SILENT_DB)).powf(1.6)
    }
}

fn pos_db(x: f32) -> f32 {
    if x <= 0.0 {
        SILENT_DB
    } else if x >= 0.75 {
        -6.0 + (x - 0.75) / 0.25 * (TOP_DB + 6.0)
    } else {
        SILENT_DB + (x / 0.75).powf(1.0 / 1.6) * (-6.0 - SILENT_DB)
    }
}

/// Peak meters, one bar per level, on the fader's scale.
fn meter(ui: &mut Ui, levels: &[f32], active: bool, t: &Tokens) {
    let w = 5.0 * levels.len() as f32 + 2.0 * (levels.len() as f32 - 1.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, FADER_H), Sense::hover());
    let painter = ui.painter();
    let span = r.height() - 12.0;
    for (i, l) in levels.iter().enumerate() {
        let bar = Rect::from_min_size(pos2(r.left() + i as f32 * 7.0, r.top() + 6.0), vec2(5.0, span));
        painter.rect_filled(bar, 1.0, t.canvas);
        let db = if *l > 1e-6 { 20.0 * l.log10() } else { SILENT_DB };
        let h = span * db_pos(db);
        if h > 0.5 {
            let col = if db > -0.5 {
                t.alert
            } else if db > -6.0 {
                t.caution
            } else {
                t.accent
            };
            let col = if active { col } else { col.gamma_multiply(0.35) };
            painter.rect_filled(Rect::from_min_max(pos2(bar.left(), bar.bottom() - h), bar.max), 1.0, col);
        }
        painter.rect_stroke(bar, 1.0, Stroke::new(1.0, t.button_border.gamma_multiply(0.5)), StrokeKind::Outside);
    }
}
