//! Panel widgets: slider rows, accordion sections, segmented buttons, dropdown rows,
//! text and icon buttons, and line icons.
//!
//! Copied from Ambient MIDI Generator, adapted from BW Darkroom's widgets (https://github.com/catskinroyale/bnw-darkroom).
//! The slider, section header and text button there are adapted from LightCraft's
//! `crates/ui-egui/src/widgets.rs` (https://github.com/storytold/lightcraft), Copyright (c)
//! 2026 ArtCraft Team and the LightCraft contributors, licensed under the MIT License (see
//! `assets/LICENSE-LightCraft-MIT`). Modified here: words at the slider's ends saying which
//! way is less and which more, minutes:seconds values, and transport icons.

// A shared widget library: not every widget is used by this app yet.
#![allow(dead_code)]

use eframe::egui::{self, Align2, Color32, Id, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::theme::Tokens;

/// Content inset from the panel's left and right edges, as in LightCraft.
pub const PAD_L: f32 = 24.0;
pub const PAD_R: f32 = 22.0;
const SLIDER_ROW_H: f32 = 45.0;
const THUMB_RING: f32 = 7.0;

/// Run `content` inset to the panel's content column.
pub fn inset<R>(ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::NONE.inner_margin(egui::Margin { left: PAD_L as i8, right: PAD_R as i8, top: 0, bottom: 0 }).show(ui, content).inner
}

/// A slider row: what it controls and how values map onto its track.
pub struct SliderSpec<'a> {
    /// Unique id (also the persistent key for transient state).
    pub id: &'a str,
    pub label: &'a str,
    pub help: &'a str,
    pub min: f64,
    pub max: f64,
    /// What double-clicking resets to.
    pub default: f64,
    pub step: f64,
    pub decimals: usize,
    pub suffix: &'a str,
    /// Show "+" on positive values.
    pub signed: bool,
    /// Logarithmic track (e.g. kelvin, EI).
    pub log: bool,
    /// Words under the ends of the track, e.g. ("sparse", "busy").
    pub ends: Option<(&'a str, &'a str)>,
    /// Show and type the value as minutes:seconds.
    pub time: bool,
}

impl Default for SliderSpec<'_> {
    fn default() -> Self {
        SliderSpec {
            id: "",
            label: "",
            help: "",
            min: 0.0,
            max: 1.0,
            default: 0.5,
            step: 0.01,
            decimals: 2,
            suffix: "",
            signed: false,
            log: false,
            ends: None,
            time: false,
        }
    }
}

impl SliderSpec<'_> {
    fn position(&self, v: f64) -> f64 {
        let v = v.clamp(self.min, self.max);
        if self.log && self.min > 0.0 {
            (v.ln() - self.min.ln()) / (self.max.ln() - self.min.ln())
        } else {
            (v - self.min) / (self.max - self.min)
        }
    }

    fn value_at(&self, p: f64) -> f64 {
        let p = p.clamp(0.0, 1.0);
        if self.log && self.min > 0.0 {
            (self.min.ln() + p * (self.max.ln() - self.min.ln())).exp()
        } else {
            self.min + p * (self.max - self.min)
        }
    }

    fn snap(&self, v: f64) -> f64 {
        let step = self.step.max(1e-9);
        ((v / step).round() * step).clamp(self.min, self.max)
    }

    /// The value as shown next to the label.
    pub fn format(&self, v: f64) -> String {
        if self.time {
            let s = v.max(0.0).round() as u64;
            return format!("{}:{:02}", s / 60, s % 60);
        }
        let text = format!("{v:.*}", self.decimals);
        let zero = text.trim_start_matches('-').chars().all(|c| c == '0' || c == '.');
        if zero {
            format!("0{}", self.suffix)
        } else if self.signed && v > 0.0 {
            format!("+{text}{}", self.suffix)
        } else {
            format!("{text}{}", self.suffix)
        }
    }

    /// A typed value: "0.5", "+12", "-1,5", "5600 K"; on the steps and in range.
    fn parse(&self, text: &str) -> Option<f64> {
        if self.time
            && let Some((m, s)) = text.trim().split_once(':')
        {
            let v = m.trim().parse::<f64>().ok()? * 60.0 + s.trim().parse::<f64>().ok()?;
            return Some(self.snap(v));
        }
        let t = text.trim().replace(',', ".");
        let t = t.trim_end_matches(|c: char| c.is_alphabetic() || c.is_whitespace() || c == '°' || c == '×' || c == '%');
        let t = t.trim_start_matches('×');
        t.parse::<f64>().ok().filter(|v| v.is_finite()).map(|v| self.snap(v))
    }
}

/// A LightCraft-style slider row: label and value above a thin track with a hollow ring
/// thumb. Drag the track (Shift: a tenth as fast); double-click the label or thumb to reset;
/// click the value to type one (Return applies, Esc cancels). Returns true when `value`
/// changed.
pub fn slider(ui: &mut Ui, spec: &SliderSpec, value: &mut f64, enabled: bool) -> bool {
    let t = Tokens::get(ui.ctx());
    let w = ui.available_width();
    let ends_h = if spec.ends.is_some() { 12.0 } else { 0.0 };
    let (row, _) = ui.allocate_exact_size(vec2(w, SLIDER_ROW_H + ends_h), Sense::hover());
    let label_rect = Rect::from_min_size(pos2(row.left() + PAD_L, row.top() + 4.0), vec2((w - PAD_L - PAD_R).max(40.0), 18.0));
    let track_rect = Rect::from_min_max(pos2(row.left() + PAD_L, row.top() + 22.0), pos2(row.right() - PAD_R, row.top() + 40.0));
    let id = Id::unique(("slider", spec.id));
    let sense = if enabled { Sense::click_and_drag() } else { Sense::hover() };
    let resp = ui.interact(track_rect.expand2(vec2(8.0, 2.0)), id, sense);
    let value_rect = Rect::from_min_max(pos2(label_rect.right() - 72.0, label_rect.top()), label_rect.max);
    let name_rect = Rect::from_min_max(label_rect.min, pos2(value_rect.left(), label_rect.bottom()));
    let label_sense = if enabled { Sense::click() } else { Sense::hover() };
    let help = match spec.ends {
        Some((less, more)) => format!("{}\nLeft: {less}. Right: {more}.", spec.help),
        None => spec.help.to_string(),
    };
    let label_resp = ui.interact(name_rect, id.with("label"), label_sense).on_hover_text(format!("{help}\nDouble-click to reset."));
    let value_resp = ui.interact(value_rect, id.with("value"), label_sense).on_hover_text("Click to type a value");

    let before = *value;
    let typing_id = id.with("typing");
    let mut typing: Option<(String, bool)> = ui.data(|d| d.get_temp(typing_id));
    if enabled && typing.is_none() && value_resp.clicked() {
        typing = Some((spec.format(*value).trim_start_matches('+').to_string(), true));
    }

    let to_x = |v: f64| track_rect.left() + spec.position(v) as f32 * track_rect.width();
    // Where the thumb was this frame, before any change.
    let thumb_x = to_x(*value);
    let on_thumb = |p: Pos2| (p.x - thumb_x).abs() <= THUMB_RING + 1.0;

    if let Some((mut text, first)) = typing.take() {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(value_rect).layout(egui::Layout::right_to_left(egui::Align::Center)));
        let edit = child.add(
            egui::TextEdit::singleline(&mut text)
                .font(t.mono(12.5))
                .align(egui::Align2::RIGHT_CENTER)
                .desired_width(value_rect.width())
                .margin(egui::Margin::ZERO),
        );
        if first {
            edit.request_focus();
        }
        let (enter, escape) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        if escape || !enabled {
            ui.data_mut(|d| d.remove::<(String, bool)>(typing_id));
        } else if enter || (!first && edit.lost_focus()) {
            if let Some(v) = spec.parse(&text) {
                *value = v;
            }
            ui.data_mut(|d| d.remove::<(String, bool)>(typing_id));
        } else {
            typing = Some((text.clone(), false));
            ui.data_mut(|d| d.insert_temp(typing_id, (text, false)));
        }
    } else if enabled && (label_resp.double_clicked() || resp.double_clicked()) {
        *value = spec.default;
    } else if enabled && (resp.dragged() || resp.drag_started()) {
        if let Some(p) = resp.interact_pointer_pos() {
            let fine_id = id.with("fine");
            let pos = if ui.input(|i| i.modifiers.shift) {
                // A tenth of the pointer's travel; the unrounded position is kept so slow
                // drags add up instead of rounding back to the same step.
                let dx = ui.input(|i| i.pointer.delta().x);
                let kept: Option<f64> = ui.data(|d| d.get_temp(fine_id));
                let start = kept.unwrap_or_else(|| spec.position(*value));
                (start + f64::from(dx / track_rect.width()) * 0.1).clamp(0.0, 1.0)
            } else {
                f64::from((p.x - track_rect.left()) / track_rect.width())
            };
            ui.data_mut(|d| d.insert_temp(fine_id, pos));
            *value = spec.snap(spec.value_at(pos));
        }
        if resp.drag_stopped() {
            ui.data_mut(|d| d.remove::<f64>(id.with("fine")));
        }
    } else if enabled
        && resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
        && !on_thumb(p)
    {
        // A click on the thumb leaves it, so a double-click there is a clean reset.
        let pos = f64::from((p.x - track_rect.left()) / track_rect.width());
        *value = spec.snap(spec.value_at(pos));
    }
    if enabled && resp.hovered() && resp.hover_pos().is_some_and(on_thumb) {
        resp.clone().on_hover_text(format!("{help}\nDouble-click to reset."));
    }

    // Paint.
    let hovered = resp.hovered() || resp.dragged();
    let text_c = if enabled { t.text_label } else { t.text_disabled };
    let p = ui.painter();
    p.text(label_rect.left_center(), Align2::LEFT_CENTER, spec.label, t.font(13.0), text_c);
    if typing.is_none() {
        p.text(label_rect.right_center(), Align2::RIGHT_CENTER, spec.format(*value), t.mono(12.5), text_c);
    }
    let y = track_rect.center().y;
    let tx = to_x(*value);
    p.rect_filled(Rect::from_min_max(pos2(track_rect.left(), y - 1.0), pos2(track_rect.right(), y + 1.0)), 1.0, t.track);
    if enabled {
        let from = to_x(spec.min.max(0.0).min(spec.max)).min(tx);
        let to = to_x(spec.min.max(0.0).min(spec.max)).max(tx);
        p.rect_filled(Rect::from_min_max(pos2(from, y - 1.0), pos2(to, y + 1.0)), 1.0, t.accent.gamma_multiply(0.45));
    }
    p.rect_filled(Rect::from_center_size(pos2(tx, y), vec2(THUMB_RING * 2.0 + 4.0, 6.0)), 0.0, t.chrome);
    let ring_c = if !enabled {
        t.text_disabled
    } else if hovered {
        t.thumb_hover
    } else {
        t.thumb
    };
    if let Some((less, more)) = spec.ends {
        // below the thumb ring, so the ring never covers them
        let ey = track_rect.center().y + THUMB_RING + 8.0;
        p.text(pos2(track_rect.left(), ey), Align2::LEFT_CENTER, less, t.font(10.5), t.text_dim);
        p.text(pos2(track_rect.right(), ey), Align2::RIGHT_CENTER, more, t.font(10.5), t.text_dim);
    }
    p.circle_filled(pos2(tx, y), THUMB_RING, ring_c);
    p.circle_stroke(pos2(tx, y), THUMB_RING, Stroke::new(1.5, t.chrome));
    if hovered && enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    (*value - before).abs() > 1e-12
}

/// Persistent open/closed state for a section or expandable row.
fn open_state(ui: &Ui, key: &str, default: bool) -> (Id, bool) {
    let id = Id::unique(("bw-open", key));
    let open = ui.data_mut(|d| *d.get_persisted_mut_or(id, default));
    (id, open)
}

/// An accordion section: a 52 pt header with a chevron and a semibold title, then its
/// content when open. Open state persists between runs.
pub fn section(ui: &mut Ui, key: &str, title: &str, default_open: bool, content: impl FnOnce(&mut Ui)) {
    let t = Tokens::get(ui.ctx());
    let (state, open) = open_state(ui, key, default_open);
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 52.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(r, 0.0, t.chrome.gamma_multiply(1.06));
    }
    let chev = Rect::from_center_size(pos2(r.left() + 30.0, r.center().y), vec2(14.0, 14.0));
    paint_icon(p, chev, if open { Icon::ChevronDown } else { Icon::ChevronRight }, t.accent);
    p.text(pos2(r.left() + 44.0, r.center().y), Align2::LEFT_CENTER, title.to_uppercase(), t.label(10.5), t.text);
    let open = if resp.clicked() { !open } else { open };
    ui.data_mut(|d| d.insert_persisted(state, open));
    if open {
        content(ui);
        ui.add_space(10.0);
    }
    divider(ui);
}

/// Open or close the sections named in `keys`.
pub fn set_open(ctx: &egui::Context, keys: &[&str], open: bool) {
    ctx.data_mut(|d| {
        for key in keys {
            d.insert_persisted(Id::unique(("bw-open", *key)), open);
        }
    });
}

/// A thin divider between sections.
pub fn divider(ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, t.divider);
}

/// A sub-heading inside a section.
pub fn subtitle(ui: &mut Ui, title: &str, help: &str) {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
    ui.painter().text(pos2(r.left() + PAD_L, r.center().y + 2.0), Align2::LEFT_CENTER, title, t.semibold(13.0), t.text_label);
    if !help.is_empty() {
        resp.on_hover_text(help);
    }
}

/// A dim one-line note inside a section.
pub fn note(ui: &mut Ui, text: impl Into<String>, help: &str) {
    let t = Tokens::get(ui.ctx());
    inset(ui, |ui| {
        let r = ui.label(egui::RichText::new(text.into()).font(t.font(12.0)).color(t.text_dim));
        if !help.is_empty() {
            r.on_hover_text(help);
        }
    });
}

/// A small bordered text button (24 pt), as LightCraft's Auto / Reset buttons.
pub fn text_button(ui: &mut Ui, label: &str, active: bool, enabled: bool) -> Response {
    let t = Tokens::get(ui.ctx());
    let galley = ui.painter().layout_no_wrap(label.to_string(), t.semibold(11.5), t.text);
    let w = (galley.size().x + 16.0).max(37.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(vec2(w, 24.0), sense);
    paint_button(ui.painter(), r, &resp, label, active, enabled, &t);
    resp
}

fn paint_button(p: &Painter, r: Rect, resp: &Response, label: &str, active: bool, enabled: bool, t: &Tokens) {
    let fill = if active {
        t.pressed
    } else if enabled && resp.hovered() {
        t.hover
    } else {
        t.button
    };
    p.rect_filled(r, 3.0, fill);
    let border = if active { t.accent } else { t.button_border };
    p.rect_stroke(r, 3.0, Stroke::new(1.0, border), StrokeKind::Inside);
    let c = if !enabled {
        t.text_disabled
    } else if active {
        t.accent
    } else {
        t.text_label
    };
    p.text(r.center(), Align2::CENTER_CENTER, label, t.semibold(11.5), c);
}

/// One option of a segmented control.
pub struct Segment<'a> {
    pub label: &'a str,
    pub selected: bool,
    pub enabled: bool,
    pub help: &'a str,
}

/// Equal-width buttons, `per_row` to a row, inset to the content column. Returns the
/// index of the option clicked.
pub fn segmented(ui: &mut Ui, segments: &[Segment], per_row: usize) -> Option<usize> {
    let t = Tokens::get(ui.ctx());
    let per_row = per_row.max(1);
    let mut clicked = None;
    inset(ui, |ui| {
        let gap = 4.0;
        let full = ui.available_width();
        for (row, chunk) in segments.chunks(per_row).enumerate() {
            let w = (full - gap * (per_row as f32 - 1.0)) / per_row as f32;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (i, s) in chunk.iter().enumerate() {
                    let sense = if s.enabled { Sense::click() } else { Sense::hover() };
                    let (r, resp) = ui.allocate_exact_size(vec2(w, 24.0), sense);
                    paint_button(ui.painter(), r, &resp, s.label, s.selected, s.enabled, &t);
                    if !s.help.is_empty() {
                        resp.clone().on_hover_text(s.help);
                    }
                    if resp.clicked() {
                        clicked = Some(row * per_row + i);
                    }
                }
            });
            ui.add_space(gap);
        }
    });
    clicked
}

/// A "Label  Value ▾" row that opens a menu, as LightCraft's profile picker.
pub fn dropdown(ui: &mut Ui, key: &str, label: &str, value: &str, help: &str, menu: impl FnOnce(&mut Ui)) {
    let t = Tokens::get(ui.ctx());
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    let p = ui.painter();
    let label_end = p.text(pos2(r.left() + PAD_L, r.center().y), Align2::LEFT_CENTER, label, t.font(13.0), t.text_dim).right();
    let x = (label_end + 10.0).max(r.left() + PAD_L + 70.0);
    let value_galley = p.layout_no_wrap(value.to_string(), t.font(13.0), t.text);
    let max_value_w = (r.right() - PAD_R - 20.0 - x).max(20.0);
    let value_w = value_galley.size().x.min(max_value_w);
    let hit = Rect::from_min_max(pos2(x - 4.0, r.top() + 4.0), pos2(x + value_w + 22.0, r.bottom() - 4.0));
    let resp = ui.interact(hit, Id::unique(("dropdown", key)), Sense::click()).on_hover_text(help);
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(hit, 4.0, t.hover.gamma_multiply(0.7));
    }
    let clip = Rect::from_min_max(pos2(x, r.top()), pos2(x + value_w, r.bottom()));
    p.with_clip_rect(clip).galley(pos2(x, r.center().y - value_galley.size().y / 2.0), value_galley, t.text);
    paint_icon(p, Rect::from_center_size(pos2(x + value_w + 11.0, r.center().y), vec2(12.0, 12.0)), Icon::ChevronDown, t.text_label);
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(200.0);
        menu(ui);
    });
}

/// A palette in the palette menu: its name over a strip of its colours.
pub fn palette_row(ui: &mut Ui, palette: crate::theme::Palette, selected: bool) -> Response {
    let t = Tokens::get(ui.ctx());
    let pt = Tokens::of(palette);
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(240.0), 44.0), Sense::click());
    let p = ui.painter();
    if selected || resp.hovered() {
        p.rect_filled(r, 3.0, if selected { t.pressed } else { t.hover });
    }
    if selected {
        p.rect_stroke(r, 3.0, Stroke::new(1.0, t.accent), StrokeKind::Inside);
    }
    p.text(pos2(r.left() + 10.0, r.top() + 13.0), Align2::LEFT_CENTER, palette.label(), t.medium(13.0), t.text);
    let chips = [pt.canvas, pt.button, pt.secondary, pt.accent, pt.text];
    for (i, c) in chips.iter().enumerate() {
        let cr = Rect::from_min_size(pos2(r.left() + 10.0 + i as f32 * 30.0, r.top() + 25.0), vec2(26.0, 12.0));
        p.rect_filled(cr, 2.0, *c);
        p.rect_stroke(cr, 2.0, Stroke::new(1.0, t.button_border), StrokeKind::Inside);
    }
    resp.on_hover_text(palette.help())
}

/// A small icon button for the top bar: a rounded square when active or hovered.
pub fn icon_button(ui: &mut Ui, icon: Icon, active: bool, help: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(28.0, 24.0), Sense::click());
    let p = ui.painter();
    if active {
        p.rect_filled(r, 4.0, t.tool_active);
    } else if resp.hovered() {
        p.rect_filled(r, 4.0, t.hover.gamma_multiply(0.7));
    }
    let c = if active || resp.hovered() { t.pick } else { t.icon };
    paint_icon(p, Rect::from_center_size(r.center(), vec2(18.0, 18.0)), icon, c);
    resp.on_hover_text(help)
}

/// Line icons drawn in code on a 20 × 20 grid with 1.5 px strokes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    ChevronDown,
    ChevronRight,
    Info,
    /// Show or hide the control panel: a window with a sidebar.
    Sidebar,
    /// Light theme.
    Sun,
    /// Dark theme.
    Moon,
    /// Theme follows the system: a half-filled circle.
    Auto,
    /// The colour palette: three swatches.
    Palette,
    Play,
    Pause,
    Stop,
    /// Loop playback: two arrows chasing round.
    Loop,
    /// A speaker, for the volume.
    Speaker,
}

/// Paint `icon` in `rect`.
pub fn paint_icon(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height()) / 20.0;
    let o = rect.center() - vec2(10.0, 10.0) * s;
    let at = |x: f32, y: f32| o + vec2(x, y) * s;
    let stroke = Stroke::new(1.5, color);
    let line = |pts: &[(f32, f32)]| {
        for w in pts.windows(2) {
            p.line_segment([at(w[0].0, w[0].1), at(w[1].0, w[1].1)], stroke);
        }
    };
    let arc = |cx: f32, cy: f32, r: f32, from: f32, to: f32| -> Vec<Pos2> {
        (0..=16)
            .map(|i| {
                let a = from + (to - from) * i as f32 / 16.0;
                at(cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    };
    use std::f32::consts::PI;
    match icon {
        Icon::ChevronDown => line(&[(5.0, 7.5), (10.0, 12.5), (15.0, 7.5)]),
        Icon::ChevronRight => line(&[(7.5, 5.0), (12.5, 10.0), (7.5, 15.0)]),
        Icon::Info => {
            p.circle_stroke(at(10.0, 10.0), 7.5 * s, stroke);
            line(&[(10.0, 9.0), (10.0, 14.0)]);
            p.circle_filled(at(10.0, 6.3), 1.0 * s, color);
        }
        Icon::Sidebar => {
            p.rect_stroke(Rect::from_min_size(at(2.5, 4.0), vec2(15.0, 12.0) * s), 2.0, stroke, StrokeKind::Middle);
            line(&[(7.5, 4.0), (7.5, 16.0)]);
        }
        Icon::Sun => {
            p.circle_stroke(at(10.0, 10.0), 3.2 * s, stroke);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let (c, sn) = (a.cos(), a.sin());
                p.line_segment([at(10.0 + 5.5 * c, 10.0 + 5.5 * sn), at(10.0 + 7.5 * c, 10.0 + 7.5 * sn)], stroke);
            }
        }
        Icon::Moon => {
            // A crescent: an outer arc and an inner arc meeting at the tips.
            let mut pts = arc(10.0, 10.0, 7.0, -0.35 * PI, 1.35 * PI);
            pts.extend(arc(12.5, 7.5, 5.2, 1.08 * PI, -0.08 * PI));
            pts.push(pts[0]);
            p.add(egui::Shape::line(pts, stroke));
        }
        Icon::Palette => {
            for (i, x) in [3.0f32, 8.5, 14.0].iter().enumerate() {
                let r = Rect::from_min_size(at(*x, 6.0), vec2(4.0, 8.0) * s);
                if i == 1 {
                    p.rect_filled(r, 1.0, color);
                } else {
                    p.rect_stroke(r, 1.0, stroke, StrokeKind::Middle);
                }
            }
        }
        Icon::Auto => {
            p.circle_stroke(at(10.0, 10.0), 7.0 * s, stroke);
            p.add(egui::Shape::convex_polygon(arc(10.0, 10.0, 7.0, -0.5 * PI, 0.5 * PI), color, Stroke::NONE));
        }
        Icon::Play => {
            p.add(egui::Shape::convex_polygon(vec![at(6.0, 4.0), at(16.0, 10.0), at(6.0, 16.0)], color, Stroke::NONE));
        }
        Icon::Pause => {
            for x in [5.5, 11.5] {
                p.rect_filled(Rect::from_min_size(at(x, 4.0), vec2(3.0, 12.0) * s), 0.5, color);
            }
        }
        Icon::Stop => {
            p.rect_filled(Rect::from_min_size(at(5.0, 5.0), vec2(10.0, 10.0) * s), 1.0, color);
        }
        Icon::Loop => {
            p.add(egui::Shape::line(arc(10.0, 10.0, 6.0, 0.15 * PI, 0.95 * PI), stroke));
            p.add(egui::Shape::line(arc(10.0, 10.0, 6.0, 1.15 * PI, 1.95 * PI), stroke));
            line(&[(13.0, 2.5), (15.7, 4.6), (13.2, 7.0)]);
            line(&[(7.0, 17.5), (4.3, 15.4), (6.8, 13.0)]);
        }
        Icon::Speaker => {
            line(&[(3.0, 8.0), (6.0, 8.0), (10.0, 4.5), (10.0, 15.5), (6.0, 12.0), (3.0, 12.0), (3.0, 8.0)]);
            p.add(egui::Shape::line(arc(10.0, 10.0, 4.0, -0.3 * PI, 0.3 * PI), stroke));
            p.add(egui::Shape::line(arc(10.0, 10.0, 7.0, -0.3 * PI, 0.3 * PI), stroke));
        }
    }
}

/// How a dial looks: its label, hover help, size, and whether its value is always
/// shown under the label (else only while hovered, in the label's place).
pub struct DialSpec<'a> {
    pub label: &'a str,
    pub help: &'a str,
    /// The value it returns to on a double-click, shown straight up.
    pub neutral: f32,
    /// How far it turns: -1 is fully left (7:30), 0 straight up, 1 fully right (4:30).
    pub turns: std::ops::RangeInclusive<f32>,
    pub radius: f32,
    pub layout: DialLayout,
}

/// Where a dial's name and value go.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DialLayout {
    /// The dial at the left of a row, its name and then its value after it, as a
    /// slider row reads.
    Beside,
    /// The name under the dial; the value shows in its place while hovered.
    Under,
    /// The name under the dial and the value, smaller, under that.
    UnderWithValue,
}

/// A rotary dial that points straight up (12 o'clock) at its neutral setting.
/// `to_turn` and `from_turn` map a value to a turn (-1..1) and back. Drag up or right
/// to turn it clockwise; double-click for neutral. It writes `v` only when it moves,
/// and says whether it did.
pub fn dial(
    ui: &mut Ui,
    spec: &DialSpec,
    v: &mut f32,
    to_turn: impl Fn(f32) -> f32,
    from_turn: impl Fn(f32) -> f32,
    value: impl Fn(f32) -> String,
) -> bool {
    const SWEEP: f32 = 0.75 * std::f32::consts::PI; // either side of straight up
    let t = Tokens::get(ui.ctx());
    let r = spec.radius;
    let h = match spec.layout {
        DialLayout::Beside => 2.0 * r + 10.0,
        DialLayout::Under => 2.0 * r + 4.0 + 15.0,
        DialLayout::UnderWithValue => 2.0 * r + 4.0 + 27.0,
    };
    let (cell, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click_and_drag());
    let id = resp.id.with("turn");
    let mut changed = false;
    if resp.drag_started() {
        // the unrounded turn, held while dragging, so values that round still move smoothly
        ui.data_mut(|d| d.insert_temp(id, to_turn(*v)));
    }
    if resp.dragged() {
        let d = resp.drag_delta();
        let held: f32 = ui.data(|m| m.get_temp(id)).unwrap_or_else(|| to_turn(*v));
        // a full sweep is about 150 points of drag
        let x = (held + (d.x - d.y) / 75.0).clamp(*spec.turns.start(), *spec.turns.end());
        ui.data_mut(|m| m.insert_temp(id, x));
        let nv = from_turn(x);
        changed = nv != *v;
        *v = nv;
    }
    if resp.double_clicked() && *v != spec.neutral {
        *v = spec.neutral;
        changed = true;
    }
    let turn = to_turn(*v).clamp(-1.0, 1.0);
    let c = if spec.layout == DialLayout::Beside {
        pos2(cell.left() + r + 3.0, cell.center().y + 1.0)
    } else {
        pos2(cell.center().x, cell.top() + r + 2.0)
    };
    let at = |a: f32, rr: f32| c + vec2(a.sin(), -a.cos()) * rr;
    let arc = |from: f32, to: f32, rr: f32| -> Vec<Pos2> {
        let n = ((to - from).abs() / 0.12).ceil().max(1.0) as usize;
        (0..=n).map(|i| at(from + (to - from) * i as f32 / n as f32, rr)).collect()
    };
    let painter = ui.painter();
    let active = resp.hovered() || resp.dragged();
    let w = if r >= 13.0 { 2.5 } else { 2.0 };
    // the whole sweep faintly, the travel this dial has, then how far it is from neutral
    let (lo, hi) = (*spec.turns.start() * SWEEP, *spec.turns.end() * SWEEP);
    painter.add(egui::Shape::line(arc(-SWEEP, SWEEP, r), Stroke::new(1.0, t.track.gamma_multiply(0.6))));
    painter.add(egui::Shape::line(arc(lo, hi, r), Stroke::new(w, t.track)));
    if turn.abs() > 1e-3 {
        painter.add(egui::Shape::line(arc(0.0, turn * SWEEP, r), Stroke::new(w, t.accent)));
    }
    painter.line_segment([at(0.0, r + 2.0), at(0.0, r + 4.0)], Stroke::new(1.0, t.text_dim));
    let body = r - 3.0 - (w - 2.0);
    painter.circle(c, body, if active { t.hover } else { t.button }, Stroke::new(1.0, t.button_border));
    let tip = if active { t.thumb_hover } else { t.thumb };
    painter.line_segment([c + (at(turn * SWEEP, body) - c) * 0.25, at(turn * SWEEP, body)], Stroke::new(w, tip));
    let name_c = if active { t.text } else { t.text_label };
    match spec.layout {
        DialLayout::Beside => {
            // as a slider row: the name, and the value at the right
            let x = c.x + r + 8.0;
            painter.text(pos2(x, cell.center().y), Align2::LEFT_CENTER, spec.label, t.font(13.0), name_c);
            painter.text(pos2(cell.right() - 2.0, cell.center().y), Align2::RIGHT_CENTER, value(*v), t.mono(12.5), t.text_dim);
        }
        DialLayout::Under => {
            let text = if active { value(*v) } else { spec.label.to_string() };
            painter.text(pos2(cell.center().x, c.y + r + 4.0), Align2::CENTER_TOP, text, t.font(10.5), name_c);
        }
        DialLayout::UnderWithValue => {
            painter.text(pos2(cell.center().x, c.y + r + 4.0), Align2::CENTER_TOP, spec.label, t.font(11.5), name_c);
            painter.text(pos2(cell.center().x, c.y + r + 18.0), Align2::CENTER_TOP, value(*v), t.mono(10.0), t.text_dim);
        }
    }
    resp.on_hover_text(format!(
        "{}: {} · {}. Drag up or right to turn clockwise; double-click for neutral",
        spec.label,
        value(*v),
        spec.help
    ));
    changed
}

/// The turn for a 0..1 value whose neutral is `d`: the left half of the dial covers
/// 0..d and the right half d..1 (one side only when `d` is at an end).
pub fn unit_turn(v: f32, d: f32) -> f32 {
    if d <= 0.0 {
        v
    } else if d >= 1.0 {
        v - 1.0
    } else if v < d {
        v / d - 1.0
    } else {
        (v - d) / (1.0 - d)
    }
}

/// The 0..1 value at a turn, the inverse of [`unit_turn`].
pub fn unit_value(x: f32, d: f32) -> f32 {
    if d <= 0.0 {
        x.clamp(0.0, 1.0)
    } else if d >= 1.0 {
        (1.0 + x).clamp(0.0, 1.0)
    } else if x < 0.0 {
        d * (1.0 + x)
    } else {
        d + x * (1.0 - d)
    }
}

/// How far a dial for a 0..1 value with neutral `d` turns.
pub fn unit_turns(d: f32) -> std::ops::RangeInclusive<f32> {
    if d <= 0.0 {
        0.0..=1.0
    } else if d >= 1.0 {
        -1.0..=0.0
    } else {
        -1.0..=1.0
    }
}

/// A 0..1 amount as a dial with its neutral straight up.
#[allow(clippy::too_many_arguments)]
pub fn unit_dial(
    ui: &mut Ui,
    label: &str,
    help: &str,
    v: &mut f32,
    neutral: f32,
    radius: f32,
    layout: DialLayout,
    value: impl Fn(f32) -> String,
) -> bool {
    let spec = DialSpec { label, help, neutral, turns: unit_turns(neutral), radius, layout };
    dial(ui, &spec, v, |v| unit_turn(v, neutral), |x| unit_value(x, neutral), value)
}

// ---- piano-key push buttons, as on a 1950s car radio or radiogram: rectangular
// ivory keys side by side in one chrome bezel. A key punched in sinks and lights from
// within like a neon; it stays down until another is punched.

const KEY_H: f32 = 26.0;
pub const KEY_GAP: f32 = 3.0;
pub const BEZEL_PAD: f32 = 4.0;

/// The bezel the keys sit in: a dark recess with a chrome rim.
fn paint_bezel(p: &Painter, r: Rect, t: &Tokens) {
    p.rect_filled(r, 5.0, t.canvas);
    // the rim, catching the light along its top
    p.rect_stroke(r, 5.0, Stroke::new(1.5, lerp_colour(t.text_dim, t.button_border, 0.4)), StrokeKind::Outside);
    p.hline(r.x_range().shrink(5.0), r.top() - 1.0, Stroke::new(1.0, t.text_label.gamma_multiply(0.5)));
    p.rect_stroke(r.shrink(1.0), 4.0, Stroke::new(1.0, Color32::BLACK.gamma_multiply(0.35)), StrokeKind::Inside);
}

/// One key in `r`, `p` from out (0) to in (1). Its neon flickers as it strikes.
fn paint_key(ui: &Ui, r: Rect, label: &str, p: f32, hover: bool, enabled: bool, t: &Tokens) {
    let painter = ui.painter();
    let neon = if p > 0.0 && p < 1.0 { p * (0.55 + 0.45 * (p * 37.0).sin().abs()) } else { p };
    const DEPTH: f32 = 4.0;
    const ROUND: f32 = 2.5;
    let ivory = lerp_colour(t.key_white, t.button, 0.18);
    let ivory = if hover && enabled { lerp_colour(ivory, Color32::WHITE, 0.08) } else { ivory };
    let ivory = if enabled { ivory } else { lerp_colour(ivory, t.button, 0.65) };
    // the key's side shows below its face while it stands proud; punched in, the face sinks
    let travel = DEPTH * p;
    let face = Rect::from_min_max(pos2(r.left(), r.top() + travel), pos2(r.right(), r.bottom() - DEPTH + travel));
    let side = Rect::from_min_max(face.min, r.max);
    painter.rect_filled(side, ROUND, lerp_colour(ivory, Color32::BLACK, 0.45));
    let lit = if enabled { neon } else { 0.0 };
    if lit > 0.0 {
        painter.rect_filled(face.expand(2.5), ROUND + 2.0, t.accent.gamma_multiply(0.22 * lit));
    }
    let shaded = lerp_colour(ivory, Color32::BLACK, 0.2 * p);
    let fill = lerp_colour(shaded, t.accent, 0.72 * lit);
    painter.rect_filled(face, ROUND, fill);
    if p < 1.0 {
        // light along the top edge of a key that stands proud
        let hl = Rect::from_min_max(face.min + vec2(2.0, 1.0), pos2(face.right() - 2.0, face.top() + 2.5));
        painter.rect_filled(hl, 1.0, Color32::WHITE.gamma_multiply(0.22 * (1.0 - p)));
    }
    painter.rect_stroke(face, ROUND, Stroke::new(1.0, Color32::BLACK.gamma_multiply(0.35)), StrokeKind::Inside);
    let c = if !enabled { t.text_disabled } else { lerp_colour(t.key_black, t.on_accent, lit) };
    painter.text(face.center(), Align2::CENTER_CENTER, label, t.semibold(11.5), c);
}

fn lerp_colour(a: Color32, b: Color32, x: f32) -> Color32 {
    let x = x.clamp(0.0, 1.0);
    let m = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * x).round() as u8;
    Color32::from_rgba_premultiplied(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()), m(a.a(), b.a()))
}

/// A row (or rows, `per_row` across) of keys in one bezel, filling the width it's
/// given; `down` says which are in. Returns each key's response.
pub fn key_bank(
    ui: &mut Ui,
    labels: &[&str],
    down: &dyn Fn(usize, &Response) -> bool,
    per_row: usize,
    width: f32,
    enabled: bool,
) -> Vec<Response> {
    let t = Tokens::get(ui.ctx());
    let per_row = per_row.max(1);
    let rows = labels.len().div_ceil(per_row);
    let h = rows as f32 * KEY_H + (rows as f32 - 1.0) * KEY_GAP + 2.0 * BEZEL_PAD;
    let (r, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    paint_bezel(ui.painter(), r, &t);
    let inner = r.shrink(BEZEL_PAD);
    let w = (inner.width() - KEY_GAP * (per_row as f32 - 1.0)) / per_row as f32;
    let base = ui.scope_id().with(("keys", r.min.x as i32, r.min.y as i32));
    let mut out = Vec::new();
    for (k, label) in labels.iter().enumerate() {
        let (row, col) = (k / per_row, k % per_row);
        let kr = Rect::from_min_size(
            pos2(inner.left() + col as f32 * (w + KEY_GAP), inner.top() + row as f32 * (KEY_H + KEY_GAP)),
            vec2(w, KEY_H),
        );
        let resp = ui.interact(kr, base.with(k), if enabled { Sense::click() } else { Sense::hover() });
        let p = ui.ctx().animate_bool_with_time(resp.id.with("in"), down(k, &resp), 0.06);
        paint_key(ui, kr, label, p, resp.hovered(), enabled, &t);
        out.push(resp);
    }
    out
}

/// A bank of keys, one in at a time, `per_row` across the content column: punching
/// one in lets the last one out. Returns the one punched in, when it changes.
pub fn punch_bank(ui: &mut Ui, choices: &[(&str, &str)], selected: usize, per_row: usize, enabled: bool) -> Option<usize> {
    inset(ui, |ui| {
        let picked = punch_bank_here(ui, choices, selected, per_row, enabled);
        ui.add_space(6.0);
        picked
    })
}

/// [`punch_bank`] across the width it's given rather than the content column.
pub fn punch_bank_here(ui: &mut Ui, choices: &[(&str, &str)], selected: usize, per_row: usize, enabled: bool) -> Option<usize> {
    let mut picked = None;
    let labels: Vec<&str> = choices.iter().map(|c| c.0).collect();
    let w = ui.available_width();
    for (k, resp) in key_bank(ui, &labels, &|k, _| k == selected, per_row, w, enabled).into_iter().enumerate() {
        let resp = if choices[k].1.is_empty() { resp } else { resp.on_hover_text(choices[k].1) };
        if resp.clicked() && k != selected {
            picked = Some(k);
        }
    }
    picked
}

/// A key that latches: punch it in, punch it again to let it out.
pub fn punch_latch(ui: &mut Ui, width: f32, label: &str, on: &mut bool, help: &str) -> bool {
    let state = *on;
    let clicked = key_bank(ui, &[label], &|_, _| state, 1, width, true).remove(0).on_hover_text(help).clicked();
    if clicked {
        *on = !*on;
    }
    clicked
}

/// A key that springs back: in while held, out when let go.
pub fn punch_momentary(ui: &mut Ui, width: f32, label: &str, help: &str) -> bool {
    key_bank(ui, &[label], &|_, r| r.is_pointer_button_down_on(), 1, width, true).remove(0).on_hover_text(help).clicked()
}

/// A setting shown as a small dial: any range, linear or by ratio (log), straight up
/// at its usual setting.
pub struct Knob<'a> {
    pub label: &'a str,
    pub v: &'a mut f32,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub log: bool,
    pub decimals: usize,
    pub suffix: &'a str,
    /// Show 0..1 as a percentage.
    pub percent: bool,
    pub help: String,
    pub enabled: bool,
}

impl<'a> Knob<'a> {
    /// A value from `range.0` to `range.1`, shown with `decimals` and `suffix`.
    pub fn new(label: &'a str, v: &'a mut f32, range: (f32, f32), default: f32, decimals: usize, suffix: &'a str, help: &str) -> Self {
        Knob {
            label,
            v,
            min: range.0,
            max: range.1,
            default,
            log: false,
            decimals,
            suffix,
            percent: false,
            help: help.to_string(),
            enabled: true,
        }
    }

    /// A 0..1 amount, shown as a percentage, with what each end means.
    pub fn amount(label: &'a str, v: &'a mut f32, default: f32, ends: (&str, &str), help: &str) -> Self {
        let help = if default <= 0.0 {
            format!("{help}. Straight up is {}; turn right for {}", ends.0, ends.1)
        } else {
            format!("{help}. Turn left for {}, right for {}", ends.0, ends.1)
        };
        Knob { percent: true, ..Knob::new(label, v, (0.0, 1.0), default, 0, "", &help) }
    }

    /// Spread evenly by ratio (octaves, decades) rather than by step.
    pub fn log(self) -> Self {
        Knob { log: true, ..self }
    }

    pub fn enabled(self, enabled: bool) -> Self {
        Knob { enabled, ..self }
    }

    fn unit(&self, v: f32) -> f32 {
        let x = if self.log && self.min > 0.0 {
            (v.max(self.min) / self.min).ln() / (self.max / self.min).ln()
        } else {
            (v - self.min) / (self.max - self.min)
        };
        x.clamp(0.0, 1.0)
    }

    fn value_at(&self, x: f32) -> f32 {
        if self.log && self.min > 0.0 { self.min * (self.max / self.min).powf(x) } else { self.min + x * (self.max - self.min) }
    }

    fn text(&self, v: f32) -> String {
        if self.percent {
            format!("{:.0}%", v * 100.0)
        } else if self.min < 0.0 {
            format!("{:+.*}{}", self.decimals, v, self.suffix)
        } else {
            format!("{:.*}{}", self.decimals, v, self.suffix)
        }
    }

    /// The dial, name under it and value on hover, as on a mixer strip.
    pub fn show(&mut self, ui: &mut Ui) {
        let d = self.unit(self.default);
        let mut u = self.unit(*self.v);
        let spec =
            DialSpec { label: self.label, help: &self.help, neutral: d, turns: unit_turns(d), radius: 10.0, layout: DialLayout::Under };
        let me = &*self;
        let changed = ui
            .add_enabled_ui(self.enabled, |ui| {
                dial(ui, &spec, &mut u, |u| unit_turn(u, d), |x| unit_value(x, d), |u| me.text(me.value_at(u)))
            })
            .inner;
        if changed {
            *self.v = if (u - d).abs() < 1e-6 { self.default } else { self.value_at(u) };
        }
    }
}

/// Knobs two to a row, as the dials on a mixer strip.
pub fn knob_rows(ui: &mut Ui, knobs: &mut [Knob]) {
    for pair in knobs.chunks_mut(2) {
        ui.columns(2, |c| {
            for (k, kn) in pair.iter_mut().enumerate() {
                kn.show(&mut c[k]);
            }
        });
        ui.add_space(3.0);
    }
}

/// A small dim heading for a group within a strip.
pub fn strip_heading(ui: &mut Ui, text: &str, enabled: bool) {
    let t = Tokens::get(ui.ctx());
    ui.label(egui::RichText::new(text).font(t.font(10.0)).color(if enabled { t.text_dim } else { t.text_disabled }));
}

/// A choice as a row: a small neon lamp, lit when `lit`, and its name beside it, as a
/// track's lamp and name on the mixer. The whole row is the button.
pub fn lamp_row(ui: &mut Ui, label: &str, lit: bool, enabled: bool) -> Response {
    let t = Tokens::get(ui.ctx());
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 17.0), sense);
    if enabled && resp.hovered() {
        ui.painter().rect_filled(r, 3.0, t.hover);
    }
    let colour = if enabled { t.accent } else { t.text_disabled };
    let lamp_r = Rect::from_min_size(pos2(r.left() + 1.0, r.center().y - 6.0), vec2(12.0, 12.0));
    let mut lui = ui.new_child(egui::UiBuilder::new().max_rect(lamp_r));
    lamp(&mut lui, lit && enabled, colour, 12.0);
    let c = if !enabled {
        t.text_disabled
    } else if lit || resp.hovered() {
        t.text
    } else {
        t.text_label
    };
    let font = if lit && enabled { t.semibold(10.5) } else { t.font(10.5) };
    ui.painter().with_clip_rect(r).text(pos2(r.left() + 17.0, r.center().y), Align2::LEFT_CENTER, label, font, c);
    resp
}

/// A neon indicator lamp in `colour`: glowing when `lit`, dark glass when not.
pub fn lamp(ui: &mut Ui, lit: bool, colour: Color32, size: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let c = r.center();
    let rad = size * 0.32;
    let p = ui.painter();
    if lit {
        // the glow: wide faint halos, then a bright core whitening at its heart
        for (k, a) in [(2.4, 0.07), (1.8, 0.12), (1.35, 0.22)] {
            p.circle_filled(c, rad * k, colour.gamma_multiply(a));
        }
        p.circle_filled(c, rad, colour);
        p.circle_filled(c - vec2(rad * 0.2, rad * 0.2), rad * 0.45, Color32::WHITE.gamma_multiply(0.75));
    } else {
        p.circle_filled(c, rad, t.canvas);
        p.circle_stroke(c, rad, Stroke::new(1.0, colour.gamma_multiply(0.35)));
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(log: bool) -> SliderSpec<'static> {
        SliderSpec {
            id: "t",
            label: "T",
            help: "",
            min: if log { 2000.0 } else { -4.0 },
            max: if log { 12000.0 } else { 4.0 },
            default: 0.0,
            step: if log { 10.0 } else { 0.05 },
            decimals: if log { 0 } else { 2 },
            suffix: if log { " K" } else { "" },
            signed: !log,
            log,
            ends: None,
            time: false,
        }
    }

    #[test]
    fn positions_round_trip() {
        for log in [false, true] {
            let s = spec(log);
            for p in [0.0, 0.25, 0.5, 1.0] {
                assert!((s.position(s.value_at(p)) - p).abs() < 1e-9);
            }
        }
        // Logarithmic: the middle of the track is the geometric mean.
        let s = spec(true);
        assert!((s.value_at(0.5) - (2000.0f64 * 12000.0).sqrt()).abs() < 1e-6);
    }

    #[test]
    fn formats_and_parses_values() {
        let s = spec(false);
        assert_eq!(s.format(0.5), "+0.50");
        assert_eq!(s.format(-0.0), "0");
        assert_eq!(s.format(-1.25), "-1.25");
        assert_eq!(s.parse("+1,5"), Some(1.5));
        assert_eq!(s.parse("9"), Some(4.0));
        assert_eq!(s.parse("abc"), None);
        assert_eq!(spec(true).parse("5600 K"), Some(5600.0));
    }

    #[test]
    fn minutes_and_seconds() {
        let s = SliderSpec { min: 20.0, max: 1800.0, step: 5.0, time: true, ..SliderSpec::default() };
        assert_eq!(s.format(185.0), "3:05");
        assert_eq!(s.parse("3:05"), Some(185.0));
        assert_eq!(s.parse("90"), Some(90.0));
    }
}
