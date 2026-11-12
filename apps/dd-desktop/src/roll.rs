//! The centre of the window: the ruler (bars, sections, loop, playhead), an overview of
//! every track, the piano roll of the selected track, and a lane below it for
//! velocities, a controller or the track's volume bar by bar.
//!
//! In the roll: click to add a note (drag to set its length), drag a note to move it
//! (hold ⌥ to copy it), drag its right end to resize it, double-click or right-click
//! to delete it, shift-drag to select several. Delete removes the selection, ↑ ↓
//! transpose it (with ⇧ by octaves), ← → nudge it, Q quantises it, ⌘D duplicates it,
//! ⌘C ⌘V copy and paste at the playhead. Scroll to move, ⌘-scroll or pinch to zoom,
//! ⌥-scroll to zoom the keys.

use std::collections::BTreeSet;

use dd_seq::project::{Block, Ctrl, CtrlKind, LEVEL_UNITY, Note, PPQ, Project, level_at, level_gain, set_level};
use dd_seq::record::quantize_notes;
use dd_seq::timeline::Msg;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::app::App;
use crate::theme::Tokens;

const GUTTER: f32 = 58.0;
const RULER_H: f32 = 40.0;
const LANE_H: f32 = 86.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    Velocity,
    /// The track's volume, bar by bar.
    Volume,
    Ctrl(CtrlKind),
}

impl Lane {
    pub const ALL: [Lane; 7] = [
        Lane::Velocity,
        Lane::Volume,
        Lane::Ctrl(CtrlKind::Cc(1)),
        Lane::Ctrl(CtrlKind::Pressure),
        Lane::Ctrl(CtrlKind::Bend),
        Lane::Ctrl(CtrlKind::Cc(74)),
        Lane::Ctrl(CtrlKind::Cc(11)),
    ];

    pub fn label(self) -> String {
        match self {
            Lane::Velocity => "Velocity".into(),
            Lane::Volume => "Volume (bar by bar)".into(),
            Lane::Ctrl(k) => k.label(),
        }
    }
}

enum Drag {
    Move {
        orig: Vec<(usize, Note)>,
        anchor: Note,
    },
    Resize {
        orig: Vec<(usize, Note)>,
        anchor: Note,
    },
    Band {
        from: Pos2,
    },
    Scrub,
    LoopSet {
        from_bar: u32,
    },
    /// The end of section `index` being dragged; the arrangement and the tracks' bar
    /// levels as they were.
    SectionEnd {
        index: usize,
        orig: Vec<Block>,
        levels: Vec<Vec<dd_seq::project::Level>>,
    },
    Lane,
}

pub struct RollState {
    /// The tick at the roll's left edge, and pixels per tick.
    pub x0: f64,
    pub ppt: f32,
    /// The key at the roll's top edge, and the height of a key's row.
    pub key_top: f32,
    pub key_h: f32,
    pub selected: BTreeSet<usize>,
    drag: Option<Drag>,
    pub lane: Lane,
    last_len: u32,
    last_vel: u8,
    audition: Option<u8>,
    seen_track: usize,
    pending_fit: bool,
    /// Bar under the pointer when the ruler's menu opened.
    menu_bar: u32,
    /// The bar and value last set while drawing volumes, to fill the bars between.
    lane_last: Option<(u32, f32)>,
}

impl Default for RollState {
    fn default() -> Self {
        RollState {
            x0: 0.0,
            ppt: 0.05,
            key_top: 84.0,
            key_h: 12.0,
            selected: BTreeSet::new(),
            drag: None,
            lane: Lane::Velocity,
            last_len: PPQ,
            last_vel: 96,
            audition: None,
            seen_track: usize::MAX,
            pending_fit: true,
            menu_bar: 0,
            lane_last: None,
        }
    }
}

impl RollState {
    /// Show the whole piece, and the keys it uses, on the next frame.
    pub fn fit_to(&mut self, _p: &Project) {
        self.pending_fit = true;
    }
}

fn is_black(key: u8) -> bool {
    matches!(key % 12, 1 | 3 | 6 | 8 | 10)
}

pub fn key_name(key: u8) -> String {
    const N: [&str; 12] = ["C", "C♯", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", "B"];
    format!("{}{}", N[key as usize % 12], key as i32 / 12 - 1)
}

fn rgb(c: [u8; 3]) -> Color32 {
    crate::theme::track_colour(c)
}

/// Geometry of the roll for one frame.
#[derive(Clone, Copy)]
struct View {
    x0: f64,
    ppt: f32,
    left: f32,
    key_top: f32,
    key_h: f32,
    top: f32,
}

impl View {
    fn x(&self, tick: f64) -> f32 {
        self.left + ((tick - self.x0) as f32) * self.ppt
    }
    fn tick(&self, x: f32) -> f64 {
        self.x0 + ((x - self.left) / self.ppt) as f64
    }
    fn y(&self, key: u8) -> f32 {
        self.top + (self.key_top - key as f32) * self.key_h
    }
    fn key(&self, y: f32) -> i32 {
        (self.key_top - (y - self.top) / self.key_h).ceil() as i32
    }
    fn note_rect(&self, n: &Note) -> Rect {
        let y = self.y(n.key);
        Rect::from_min_max(
            pos2(self.x(n.start as f64), y + 1.0),
            pos2(self.x(n.end() as f64).max(self.x(n.start as f64) + 3.0), y + self.key_h - 1.0),
        )
    }
}

fn snap(tick: f64, grid: u32) -> u32 {
    let g = grid.max(1) as f64;
    ((tick / g).round() * g).max(0.0) as u32
}

fn snap_down(tick: f64, grid: u32) -> u32 {
    let g = grid.max(1) as f64;
    ((tick / g).floor() * g).max(0.0) as u32
}

impl App {
    pub(crate) fn roll_ui(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        let full = ui.available_rect_before_wrap();
        ui.allocate_rect(full, Sense::hover());
        if self.doc.project.tracks.is_empty() {
            ui.painter().text(full.center(), Align2::CENTER_CENTER, "No tracks: start a new piece from File", t.font(15.0), t.text_dim);
            return;
        }
        let switched = self.roll.seen_track != self.sel;
        if switched {
            self.roll.selected.clear();
            self.roll.seen_track = self.sel;
        }
        let n_tracks = self.doc.project.tracks.len();
        let row_h = (150.0 / n_tracks as f32).clamp(5.0, 16.0);
        let arr_h = row_h * n_tracks as f32 + 4.0;
        let ruler = Rect::from_min_max(full.min, pos2(full.right(), full.top() + RULER_H));
        let arr = Rect::from_min_max(pos2(full.left(), ruler.bottom()), pos2(full.right(), ruler.bottom() + arr_h));
        let lane = Rect::from_min_max(pos2(full.left(), full.bottom() - LANE_H), full.max);
        let roll = Rect::from_min_max(pos2(full.left(), arr.bottom() + 1.0), pos2(full.right(), lane.top() - 1.0));
        let grid_left = full.left() + GUTTER;

        let p = &self.doc.project;
        let end = p.end_tick() as f64;
        // show the newly selected track's notes
        if switched
            && !self.roll.pending_fit
            && let Some(tr) = p.tracks.get(self.sel)
            && !tr.notes.is_empty()
        {
            let (lo, hi) = tr.notes.iter().fold((127u8, 0u8), |(a, b), n| (a.min(n.key), b.max(n.key)));
            let rows = roll.height() / self.roll.key_h;
            let mid = (lo as f32 + hi as f32) / 2.0;
            self.roll.key_top = (mid + rows / 2.0).max(hi as f32 + 2.0).min(lo as f32 + rows - 1.0).clamp(12.0, 127.0);
        }
        if self.roll.pending_fit && roll.width() > 100.0 {
            self.roll.pending_fit = false;
            self.roll.x0 = 0.0;
            self.roll.ppt = ((roll.width() - GUTTER - 20.0) / end as f32).max(0.002);
            let keys: Vec<u8> = p.tracks.get(self.sel).map(|t| t.notes.iter().map(|n| n.key).collect()).unwrap_or_default();
            let (lo, hi) = (keys.iter().min().copied().unwrap_or(48), keys.iter().max().copied().unwrap_or(72));
            let span = (hi - lo) as f32 + 8.0;
            self.roll.key_h = (roll.height() / span).clamp(6.0, 18.0);
            self.roll.key_top = hi as f32 + 4.0 + ((roll.height() / self.roll.key_h - span) / 2.0).max(0.0);
        }

        // scroll and zoom
        let hover = ui.input(|i| i.pointer.hover_pos());
        if let Some(hp) = hover
            && full.contains(hp)
            && hp.x > grid_left
        {
            let (scroll, zoom, mods) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers));
            let pivot = self.roll.x0 + ((hp.x - grid_left) / self.roll.ppt) as f64;
            let mut z = zoom;
            if mods.command && scroll.y != 0.0 {
                z *= (scroll.y * 0.006).exp();
            }
            if (z - 1.0).abs() > 1e-4 {
                self.roll.ppt = (self.roll.ppt * z).clamp(0.002, 2.0);
                self.roll.x0 = pivot - ((hp.x - grid_left) / self.roll.ppt) as f64;
            } else if mods.alt && roll.contains(hp) {
                self.roll.key_h = (self.roll.key_h * (scroll.y * 0.006).exp()).clamp(5.0, 32.0);
            } else {
                self.roll.x0 -= (scroll.x / self.roll.ppt) as f64;
                if roll.contains(hp) {
                    self.roll.key_top = (self.roll.key_top + scroll.y / self.roll.key_h).clamp(12.0, 127.0);
                } else if !mods.shift {
                    self.roll.x0 -= (scroll.y / self.roll.ppt) as f64;
                }
            }
        }
        let max_x0 = (end - ((roll.width() - GUTTER) / self.roll.ppt) as f64 * 0.5).max(0.0);
        self.roll.x0 = self.roll.x0.clamp(-(PPQ as f64), max_x0 + PPQ as f64 * 4.0);
        // follow the playhead
        let pos = self.position();
        let playing = self.playing();
        let play_tick = pos.max(0.0) / p.secs_per_tick();
        let width_ticks = ((roll.width() - GUTTER) / self.roll.ppt) as f64;
        if playing && self.roll.drag.is_none() && (play_tick > self.roll.x0 + width_ticks * 0.95 || play_tick < self.roll.x0) {
            self.roll.x0 = play_tick - width_ticks * 0.05;
        }

        let v = View {
            x0: self.roll.x0,
            ppt: self.roll.ppt,
            left: grid_left,
            key_top: self.roll.key_top,
            key_h: self.roll.key_h,
            top: roll.top(),
        };
        self.ruler_ui(ui, ruler, v, &t);
        self.arrangement_ui(ui, arr, row_h, v, &t);
        self.notes_ui(ui, roll, v, &t);
        self.lane_ui(ui, lane, v, &t);

        // the playhead across everything
        let x = v.x(play_tick);
        if x >= grid_left && x <= full.right() {
            let col = t.secondary;
            ui.painter().vline(x, full.top()..=full.bottom(), Stroke::new(1.5, col));
            ui.painter().add(egui::Shape::convex_polygon(
                vec![pos2(x - 5.0, full.top()), pos2(x + 5.0, full.top()), pos2(x, full.top() + 7.0)],
                col,
                Stroke::NONE,
            ));
        }
        if playing {
            ui.ctx().request_repaint();
        }
    }

    fn grid_lines(&self, painter: &egui::Painter, rect: Rect, v: View, t: &Tokens, fine: bool) {
        let p = &self.doc.project;
        let (bar, beat) = (p.bar_ticks() as f64, p.beat_ticks() as f64);
        let snap_t = self.settings.snap.ticks(p) as f64;
        let first = v.tick(rect.left()).max(0.0);
        let last = v.tick(rect.right());
        let step = if fine && snap_t * v.ppt as f64 > 7.0 {
            snap_t
        } else if beat * v.ppt as f64 > 6.0 {
            beat
        } else {
            bar
        };
        let mut tick = (first / step).floor() * step;
        while tick <= last {
            let x = v.x(tick);
            let on_bar = (tick / bar).fract().abs() < 1e-9;
            let on_beat = (tick / beat).fract().abs() < 1e-9;
            let (w, a) = if on_bar {
                (1.0, 0.55)
            } else if on_beat {
                (1.0, 0.25)
            } else {
                (1.0, 0.1)
            };
            if !on_bar && bar * (v.ppt as f64) < 14.0 {
                tick += step;
                continue;
            }
            painter.vline(x, rect.y_range(), Stroke::new(w, t.track.gamma_multiply(a)));
            tick += step;
        }
        // the end of the piece
        let ex = v.x(p.end_tick() as f64);
        if ex > rect.left() && ex < rect.right() {
            painter.rect_filled(Rect::from_min_max(pos2(ex, rect.top()), rect.max), 0.0, Color32::from_black_alpha(70));
        }
    }

    // ------------------------------------------------------------ ruler

    fn ruler_ui(&mut self, ui: &mut egui::Ui, rect: Rect, v: View, t: &Tokens) {
        let resp =
            ui.interact(Rect::from_min_max(pos2(v.left, rect.top()), rect.max), ui.scope_id().with("ruler"), Sense::click_and_drag());
        let p = &self.doc.project;
        let bar_t = p.bar_ticks();
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, t.chrome);
        let grid = Rect::from_min_max(pos2(v.left, rect.top()), rect.max);
        // loop
        if let Some((a, b)) = p.loop_bars {
            let r = Rect::from_min_max(pos2(v.x((a * bar_t) as f64), rect.top() + 22.0), pos2(v.x((b * bar_t) as f64), rect.bottom()));
            painter.rect_filled(r, 2.0, t.accent.gamma_multiply(0.35));
        }
        // bars
        let every = ((60.0 / (bar_t as f32 * v.ppt)).ceil() as u32).max(1);
        let first = (v.tick(grid.left()).max(0.0) / bar_t as f64) as u32;
        let last = (v.tick(grid.right()) / bar_t as f64) as u32 + 1;
        for b in first..=last {
            let x = v.x((b * bar_t) as f64);
            let major = b % every == 0;
            painter.vline(
                x,
                (rect.bottom() - if major { 12.0 } else { 6.0 })..=rect.bottom(),
                Stroke::new(1.0, t.text_dim.gamma_multiply(0.6)),
            );
            if major {
                painter.text(pos2(x + 3.0, rect.bottom() - 11.0), Align2::LEFT_CENTER, format!("{}", b + 1), t.font(10.5), t.text_label);
            }
        }
        // sections: a block each, from its start to the next
        let blocks = p.arrangement();
        let dragging = match &self.roll.drag {
            Some(Drag::SectionEnd { index, .. }) => Some(*index),
            _ => None,
        };
        let mut ends = Vec::new();
        let mut at = 0u32;
        for (i, b) in blocks.iter().enumerate() {
            let (x0, x1) = (v.x((at * bar_t) as f64), v.x(((at + b.bars) * bar_t) as f64));
            let r = Rect::from_min_max(pos2(x0, rect.top() + 2.0), pos2(x1, rect.top() + 19.0));
            if b.part.is_some() || !b.name.is_empty() {
                let a = if dragging == Some(i) {
                    0.45
                } else if i % 2 == 0 {
                    0.22
                } else {
                    0.14
                };
                painter.rect_filled(r, 3.0, t.accent.gamma_multiply(a));
                painter.vline(x0, rect.top()..=rect.top() + 20.0, Stroke::new(2.0, t.accent));
                let name = b.part.map_or(b.name.as_str(), |p| p.label());
                let label = format!("{name} · {}", b.bars);
                painter.with_clip_rect(r.intersect(rect)).text(
                    pos2(x0 + 5.0, rect.top() + 10.5),
                    Align2::LEFT_CENTER,
                    label,
                    t.semibold(11.0),
                    t.text,
                );
            }
            at += b.bars;
            ends.push((i, x1));
        }
        painter.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, t.divider));
        painter.text(pos2(rect.left() + 8.0, rect.top() + 10.0), Align2::LEFT_CENTER, "Sections", t.font(10.0), t.text_dim);
        painter.text(pos2(rect.left() + 8.0, rect.bottom() - 11.0), Align2::LEFT_CENTER, "Bars", t.font(10.0), t.text_dim);

        let bar_at = |x: f32| (v.tick(x).max(0.0) / bar_t as f64) as u32;
        // each section's end can be dragged a whole bar at a time
        for (i, x) in ends {
            if x < grid.left() - 4.0 || x > grid.right() + 4.0 {
                continue;
            }
            let h = ui
                .interact(
                    Rect::from_min_max(pos2(x - 5.0, rect.top()), pos2(x + 5.0, rect.top() + 21.0)),
                    ui.scope_id().with(("section-end", i)),
                    Sense::drag(),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
                .on_hover_text("Drag to lengthen or shorten this section, a bar at a time");
            if h.hovered() || dragging == Some(i) {
                painter.vline(x, rect.top()..=rect.top() + 20.0, Stroke::new(3.0, t.accent));
            }
            if h.drag_started() {
                let levels = self.doc.project.tracks.iter().map(|t| t.levels.clone()).collect();
                self.roll.drag = Some(Drag::SectionEnd { index: i, orig: blocks.clone(), levels });
            }
            if let (Some(Drag::SectionEnd { index, orig, levels }), Some(pp)) = (&self.roll.drag, h.interact_pointer_pos())
                && *index == i
            {
                let start: u32 = orig[..i].iter().map(|b| b.bars).sum();
                let end_bar = (v.tick(pp.x).max(0.0) / bar_t as f64).round() as u32;
                let mut fresh = orig.clone();
                fresh[i].bars = end_bar.saturating_sub(start).clamp(1, 999);
                if fresh != self.doc.project.arrangement() {
                    // from the arrangement as it was, so the levels move with their sections
                    self.doc.project.set_arrangement(orig);
                    for (t, l) in self.doc.project.tracks.iter_mut().zip(levels) {
                        t.levels = l.clone();
                    }
                    let origin: Vec<Option<usize>> = (0..fresh.len()).map(Some).collect();
                    self.doc.project.rearrange(&fresh, &origin);
                    self.status = format!("{}: {} bars", fresh[i].part.map_or(fresh[i].name.as_str(), |p| p.label()), fresh[i].bars);
                }
            }
            if h.drag_stopped() {
                self.roll.drag = None;
            }
        }
        let shift = ui.input(|i| i.modifiers.shift);
        if resp.drag_started()
            && let Some(o) = ui.input(|i| i.pointer.press_origin())
        {
            self.roll.drag = Some(if shift { Drag::LoopSet { from_bar: bar_at(o.x) } } else { Drag::Scrub });
        }
        if let Some(pp) = resp.interact_pointer_pos() {
            match self.roll.drag {
                Some(Drag::Scrub) => {
                    let secs = self.doc.project.tick_secs(v.tick(pp.x).max(0.0) as u32);
                    self.seek(secs);
                }
                Some(Drag::LoopSet { from_bar }) => {
                    let b = bar_at(pp.x);
                    let (a, z) = (from_bar.min(b), from_bar.max(b) + 1);
                    self.doc.project.loop_bars = Some((a, z));
                }
                _ => {}
            }
        }
        if resp.clicked()
            && let Some(pp) = resp.interact_pointer_pos()
        {
            let secs = self.doc.project.tick_secs(snap(v.tick(pp.x), self.doc.project.beat_ticks()));
            self.seek(secs);
        }
        if resp.double_clicked() {
            self.doc.project.loop_bars = None;
        }
        if resp.drag_stopped() {
            self.roll.drag = None;
        }
        if resp.secondary_clicked()
            && let Some(pp) = resp.interact_pointer_pos()
        {
            self.roll.menu_bar = bar_at(pp.x);
        }
        let bar = self.roll.menu_bar;
        resp.on_hover_text(
            "Click to move the playhead, drag to scrub, ⇧-drag to set the loop, double-click to clear it. Drag a section's end to change its length.",
        )
        .context_menu(|ui| {
            ui.label(format!("Bar {}", bar + 1));
            if ui.button("Loop the next 4 bars").clicked() {
                self.doc.project.loop_bars = Some((bar, bar + 4));
                ui.close();
            }
            if self.doc.project.loop_bars.is_some() && ui.button("Clear the loop").clicked() {
                self.doc.project.loop_bars = None;
                ui.close();
            }
        });
    }

    // ------------------------------------------------------------ arrangement

    fn arrangement_ui(&mut self, ui: &mut egui::Ui, rect: Rect, row_h: f32, v: View, t: &Tokens) {
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, t.inset);
        let grid = Rect::from_min_max(pos2(v.left, rect.top()), rect.max);
        self.grid_lines(&painter, grid, v, t, false);
        let p = &self.doc.project;
        let take = self.live.take.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|tk| (tk.track, tk.preview(p, self.heard())));
        for (i, tr) in p.tracks.iter().enumerate() {
            let top = rect.top() + 2.0 + i as f32 * row_h;
            let row = Rect::from_min_max(pos2(rect.left(), top), pos2(rect.right(), top + row_h));
            if i == self.sel {
                painter.rect_filled(row, 0.0, t.hover.gamma_multiply(0.6));
            }
            let col = rgb(tr.color);
            let audible = p.audible(i);
            let col = if audible { col } else { col.gamma_multiply(0.3) };
            if row_h >= 9.0 {
                painter.text(pos2(rect.left() + 6.0, row.center().y), Align2::LEFT_CENTER, &tr.name, t.font((row_h - 3.0).min(11.0)), col);
            } else {
                painter.rect_filled(
                    Rect::from_min_size(pos2(rect.left() + 6.0, row.top() + 1.0), vec2(GUTTER - 14.0, row_h - 2.0)),
                    1.0,
                    col.gamma_multiply(0.6),
                );
            }
            let (lo, hi) = tr.notes.iter().fold((127u8, 0u8), |(a, b), n| (a.min(n.key), b.max(n.key)));
            let span = (hi.saturating_sub(lo)).max(12) as f32;
            for n in &tr.notes {
                let (x0, x1) = (v.x(n.start as f64), v.x(n.end() as f64));
                if x1 < grid.left() || x0 > grid.right() {
                    continue;
                }
                let y = row.bottom() - 1.5 - (n.key.saturating_sub(lo)) as f32 / span * (row_h - 3.0);
                painter.hline(x0.max(grid.left())..=x1.max(x0 + 1.5), y, Stroke::new(1.5, col));
            }
            if let Some((ti, notes)) = &take
                && *ti == i
            {
                for n in notes {
                    painter.hline(v.x(n.start as f64)..=v.x(n.end() as f64), row.center().y, Stroke::new(2.0, t.alert));
                }
            }
        }
        let resp = ui.interact(rect, ui.scope_id().with("arrangement"), Sense::click());
        if resp.clicked()
            && let Some(pp) = resp.interact_pointer_pos()
        {
            let i = ((pp.y - rect.top() - 2.0) / row_h) as usize;
            if i < self.doc.project.tracks.len() {
                self.sel = i;
            }
        }
        resp.on_hover_text("Every track at a glance: click one to edit it");
        painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, t.divider));
    }

    // ------------------------------------------------------------ notes

    fn hit(&self, v: View, pos: Pos2) -> Option<(usize, bool)> {
        let tr = self.doc.project.tracks.get(self.sel)?;
        tr.notes.iter().enumerate().rev().find_map(|(i, n)| {
            let r = v.note_rect(n);
            r.expand2(vec2(0.0, 0.5)).contains(pos).then(|| {
                let edge = (r.width() * 0.25).clamp(3.0, 8.0);
                (i, pos.x > r.right() - edge)
            })
        })
    }

    fn notes_ui(&mut self, ui: &mut egui::Ui, rect: Rect, v: View, t: &Tokens) {
        let keys_rect = Rect::from_min_max(rect.min, pos2(v.left, rect.bottom()));
        let grid = Rect::from_min_max(pos2(v.left, rect.top()), rect.max);
        let painter = ui.painter().with_clip_rect(rect);
        let held = self.live.held();
        // rows
        let k_lo = v.key(rect.bottom()).clamp(0, 127) as u8;
        let k_hi = (v.key(rect.top()) + 1).clamp(0, 127) as u8;
        for k in k_lo..=k_hi {
            let y = v.y(k);
            let row = Rect::from_min_max(pos2(grid.left(), y), pos2(grid.right(), y + v.key_h));
            if is_black(k) {
                painter.rect_filled(row, 0.0, Color32::from_black_alpha(40));
            }
            if k % 12 == 0 {
                painter.hline(grid.x_range(), y + v.key_h, Stroke::new(1.0, t.track.gamma_multiply(0.45)));
            }
            // the keyboard
            let kr = Rect::from_min_max(pos2(keys_rect.left(), y), pos2(keys_rect.right() - 1.0, y + v.key_h));
            let down = held[k as usize] > 0 || self.roll.audition == Some(k);
            // white keys run the full width behind; black keys are shorter, on top
            painter.rect_filled(kr, 0.0, t.key_white);
            if !is_black(k) && matches!(k % 12, 4 | 11) {
                painter.hline(kr.x_range(), kr.top(), Stroke::new(1.0, t.key_line));
            }
            if is_black(k) {
                painter.hline(kr.x_range(), kr.center().y, Stroke::new(1.0, t.key_line));
                let br = Rect::from_min_max(kr.min, pos2(kr.left() + kr.width() * 0.62, kr.bottom()));
                painter.rect_filled(br, 1.5, if down { t.secondary } else { t.key_black });
            } else if down {
                painter.rect_filled(kr.shrink2(vec2(0.0, 0.5)), 1.5, t.accent);
            }
            if k % 12 == 0 && v.key_h >= 8.0 {
                painter.text(
                    pos2(kr.right() - 3.0, kr.center().y),
                    Align2::RIGHT_CENTER,
                    key_name(k),
                    t.mono((v.key_h - 2.0).min(10.0)),
                    t.key_line,
                );
            }
        }
        self.grid_lines(&painter, grid, v, t, true);
        let p = &self.doc.project;
        if let Some((a, b)) = p.loop_bars {
            let bt = p.bar_ticks() as f64;
            let r = Rect::from_min_max(pos2(v.x(a as f64 * bt), grid.top()), pos2(v.x(b as f64 * bt), grid.bottom()));
            painter.rect_filled(r, 0.0, t.accent.gamma_multiply(0.05));
        }
        let gp = painter.with_clip_rect(grid);
        // other tracks, faintly
        if self.settings.ghosts {
            for (i, tr) in p.tracks.iter().enumerate() {
                if i == self.sel || tr.source.is_drums() {
                    continue;
                }
                let c = rgb(tr.color).gamma_multiply(0.16);
                for n in &tr.notes {
                    let r = v.note_rect(n);
                    if r.right() >= grid.left() && r.left() <= grid.right() && r.bottom() >= grid.top() && r.top() <= grid.bottom() {
                        gp.rect_filled(r, 1.5, c);
                    }
                }
            }
        }
        // this track
        if let Some(tr) = p.tracks.get(self.sel) {
            let base = rgb(tr.color);
            for (i, n) in tr.notes.iter().enumerate() {
                let r = v.note_rect(n);
                if r.right() < grid.left() || r.left() > grid.right() || r.bottom() < grid.top() || r.top() > grid.bottom() {
                    continue;
                }
                let shade = 0.45 + 0.55 * n.vel as f32 / 127.0;
                gp.rect_filled(r, 2.0, base.gamma_multiply(shade));
                let sel = self.roll.selected.contains(&i);
                gp.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(if sel { 1.5 } else { 1.0 }, if sel { Color32::WHITE } else { base.gamma_multiply(0.5) }),
                    StrokeKind::Inside,
                );
                if r.width() > 30.0 && v.key_h >= 10.0 {
                    gp.text(
                        pos2(r.left() + 3.0, r.center().y),
                        Align2::LEFT_CENTER,
                        key_name(n.key),
                        t.font((v.key_h - 3.0).min(10.0)),
                        Color32::from_black_alpha(180),
                    );
                }
            }
        }
        // the take being recorded
        if let Some(take) = self.live.take.lock().unwrap_or_else(|e| e.into_inner()).as_ref()
            && take.track == self.sel
        {
            for n in take.preview(p, self.heard()) {
                gp.rect_stroke(v.note_rect(&n), 2.0, Stroke::new(1.5, t.alert), StrokeKind::Inside);
            }
        }
        if let Some(Drag::Band { from }) = &self.roll.drag
            && let Some(now) = ui.input(|i| i.pointer.hover_pos())
        {
            gp.rect_stroke(Rect::from_two_pos(*from, now), 0.0, Stroke::new(1.0, t.accent), StrokeKind::Inside);
            gp.rect_filled(Rect::from_two_pos(*from, now), 0.0, t.accent.gamma_multiply(0.08));
        }

        // the keyboard plays
        let kresp = ui.interact(keys_rect, ui.scope_id().with("roll-keys"), Sense::click_and_drag());
        let pressed_key =
            if kresp.is_pointer_button_down_on() { kresp.interact_pointer_pos().map(|pp| v.key(pp.y).clamp(0, 127) as u8) } else { None };
        if pressed_key != self.roll.audition {
            if let Some(k) = self.roll.audition {
                self.play_live(Msg::Off(k));
            }
            if let Some(k) = pressed_key {
                self.play_live(Msg::On(k, self.roll.last_vel));
            }
            self.roll.audition = pressed_key;
        }

        let resp = ui.interact(grid, ui.scope_id().with("roll"), Sense::click_and_drag());
        self.roll_mouse(ui, &resp, v);
        if resp.hovered() || resp.dragged() {
            self.roll_keys(ui);
        }
        if let Some(hp) = resp.hover_pos() {
            let k = v.key(hp.y).clamp(0, 127) as u8;
            let tick = snap_down(v.tick(hp.x), self.settings.snap.ticks(&self.doc.project));
            resp.on_hover_text_at_pointer(format!("{}  ·  {}", key_name(k), self.doc.project.position(tick)));
        }
    }

    fn roll_mouse(&mut self, ui: &egui::Ui, resp: &egui::Response, v: View) {
        let Some(track) = self.doc.project.tracks.get(self.sel) else { return };
        let grid_t = self.settings.snap.ticks(&self.doc.project);
        let (mods, origin) = ui.input(|i| (i.modifiers, i.pointer.press_origin()));
        let n_notes = track.notes.len();
        // delete
        if (resp.double_clicked() || resp.secondary_clicked())
            && let Some(pp) = resp.interact_pointer_pos()
            && let Some((i, _)) = self.hit(v, pp)
            && i < n_notes
        {
            self.doc.project.tracks[self.sel].notes.remove(i);
            self.roll.selected.clear();
            self.roll.drag = None;
            return;
        }
        if resp.drag_started()
            && let Some(o) = origin
        {
            match self.hit(v, o) {
                Some((i, edge)) => {
                    if !self.roll.selected.contains(&i) {
                        if !mods.shift {
                            self.roll.selected.clear();
                        }
                        self.roll.selected.insert(i);
                    }
                    let notes = &mut self.doc.project.tracks[self.sel].notes;
                    let anchor = notes[i];
                    if edge {
                        let orig = self.roll.selected.iter().map(|&j| (j, notes[j])).collect();
                        self.roll.drag = Some(Drag::Resize { orig, anchor });
                    } else {
                        if mods.alt {
                            // copy: drag the copies, leave the originals
                            let copies: Vec<Note> = self.roll.selected.iter().map(|&j| notes[j]).collect();
                            let first = notes.len();
                            notes.extend(copies);
                            self.roll.selected = (first..notes.len()).collect();
                        }
                        let orig = self.roll.selected.iter().map(|&j| (j, notes[j])).collect();
                        self.roll.drag = Some(Drag::Move { orig, anchor });
                        self.audition_note(anchor.key);
                    }
                }
                None if mods.shift || mods.command => {
                    if !mods.shift {
                        self.roll.selected.clear();
                    }
                    self.roll.drag = Some(Drag::Band { from: o });
                }
                None => {
                    // draw a note and drag out its length
                    let k = v.key(o.y).clamp(0, 127) as u8;
                    let start = snap_down(v.tick(o.x), grid_t);
                    let n = Note { start, len: grid_t, key: k, vel: self.roll.last_vel };
                    let notes = &mut self.doc.project.tracks[self.sel].notes;
                    notes.push(n);
                    let i = notes.len() - 1;
                    self.roll.selected = [i].into();
                    self.roll.drag = Some(Drag::Resize { orig: vec![(i, n)], anchor: n });
                    self.audition_note(k);
                }
            }
        }
        if resp.dragged()
            && let (Some(pp), Some(o)) = (resp.interact_pointer_pos(), origin)
        {
            let dt = v.tick(pp.x) - v.tick(o.x);
            let notes = &mut self.doc.project.tracks[self.sel].notes;
            match &self.roll.drag {
                Some(Drag::Move { orig, anchor }) => {
                    let new_start =
                        if mods.shift { (anchor.start as f64 + dt).max(0.0) as u32 } else { snap(anchor.start as f64 + dt, grid_t) };
                    let d = new_start as i64 - anchor.start as i64;
                    let dk = v.key(pp.y) - v.key(o.y);
                    let min_start = orig.iter().map(|(_, n)| n.start as i64).min().unwrap_or(0);
                    let d = d.max(-min_start);
                    let lo = orig.iter().map(|(_, n)| n.key as i32).min().unwrap_or(0);
                    let hi = orig.iter().map(|(_, n)| n.key as i32).max().unwrap_or(127);
                    let dk = dk.clamp(-lo, 127 - hi);
                    let before = notes.get(orig[0].0).map(|n| n.key);
                    for (j, n) in orig {
                        if let Some(x) = notes.get_mut(*j) {
                            x.start = (n.start as i64 + d) as u32;
                            x.key = (n.key as i32 + dk) as u8;
                        }
                    }
                    let after = notes.get(orig[0].0).map(|n| n.key);
                    if before != after
                        && let Some(k) = after
                    {
                        self.audition_note(k);
                    }
                }
                Some(Drag::Resize { orig, anchor }) => {
                    let end = if mods.shift { (anchor.end() as f64 + dt) as u32 } else { snap(anchor.end() as f64 + dt, grid_t) };
                    let d = end as i64 - anchor.end() as i64;
                    for (j, n) in orig {
                        if let Some(x) = notes.get_mut(*j) {
                            x.len = (n.len as i64 + d).max(grid_t.min(PPQ / 8) as i64) as u32;
                        }
                    }
                    self.roll.last_len = notes.get(orig[0].0).map_or(self.roll.last_len, |n| n.len);
                }
                Some(Drag::Band { from }) => {
                    let r = Rect::from_two_pos(*from, pp);
                    let base: BTreeSet<usize> = if mods.shift { self.roll.selected.clone() } else { BTreeSet::new() };
                    let inside: BTreeSet<usize> =
                        notes.iter().enumerate().filter(|(_, n)| r.intersects(v.note_rect(n))).map(|(i, _)| i).collect();
                    self.roll.selected = base.union(&inside).copied().collect();
                }
                _ => {}
            }
        }
        if resp.drag_stopped() {
            self.end_audition();
            self.roll.drag = None;
        }
        if resp.clicked()
            && let Some(pp) = resp.interact_pointer_pos()
        {
            match self.hit(v, pp) {
                Some((i, _)) => {
                    if mods.shift {
                        if !self.roll.selected.remove(&i) {
                            self.roll.selected.insert(i);
                        }
                    } else {
                        self.roll.selected = [i].into();
                    }
                    if let Some(n) = self.doc.project.tracks[self.sel].notes.get(i) {
                        self.roll.last_vel = n.vel;
                        self.roll.last_len = n.len;
                        self.audition_note(n.key);
                    }
                }
                None if !self.roll.selected.is_empty() && !mods.shift => self.roll.selected.clear(),
                None => {
                    let k = v.key(pp.y).clamp(0, 127) as u8;
                    let start = snap_down(v.tick(pp.x), grid_t);
                    let notes = &mut self.doc.project.tracks[self.sel].notes;
                    notes.push(Note { start, len: self.roll.last_len, key: k, vel: self.roll.last_vel });
                    self.roll.selected = [notes.len() - 1].into();
                    self.audition_note(k);
                }
            }
        }
        if !resp.is_pointer_button_down_on() && self.roll.drag.is_none() {
            self.end_audition();
        }
    }

    fn audition_note(&mut self, key: u8) {
        if self.playing() {
            return;
        }
        if let Some(k) = self.roll.audition.take() {
            self.play_live(Msg::Off(k));
        }
        self.play_live(Msg::On(key, self.roll.last_vel));
        self.roll.audition = Some(key);
    }

    fn end_audition(&mut self) {
        if let Some(k) = self.roll.audition.take() {
            self.play_live(Msg::Off(k));
        }
    }

    /// Keys that edit the selection while the pointer is over the roll.
    fn roll_keys(&mut self, ui: &egui::Ui) {
        if ui.ctx().memory(|m| m.focused().is_some()) {
            return;
        }
        let grid_t = self.settings.snap.ticks(&self.doc.project);
        let play_tick = self.doc.project.secs_tick(self.position().max(0.0));
        let events = ui.input(|i| i.events.clone());
        let Some(track) = self.doc.project.tracks.get_mut(self.sel) else { return };
        let sel: Vec<usize> = self.roll.selected.iter().copied().filter(|i| *i < track.notes.len()).collect();
        for e in events {
            match e {
                egui::Event::Copy | egui::Event::Cut => {
                    self.clipboard = sel.iter().map(|&i| track.notes[i]).collect();
                    if matches!(e, egui::Event::Cut) {
                        let s: BTreeSet<usize> = sel.iter().copied().collect();
                        let mut i = 0;
                        track.notes.retain(|_| {
                            i += 1;
                            !s.contains(&(i - 1))
                        });
                        self.roll.selected.clear();
                    }
                }
                egui::Event::Paste(_) if !self.clipboard.is_empty() => {
                    let first = self.clipboard.iter().map(|n| n.start).min().unwrap_or(0);
                    let at = snap(play_tick as f64, grid_t);
                    let base = track.notes.len();
                    for n in &self.clipboard {
                        track.notes.push(Note { start: n.start - first + at, ..*n });
                    }
                    self.roll.selected = (base..track.notes.len()).collect();
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    let all_or_sel = || if sel.is_empty() { (0..track.notes.len()).collect::<Vec<_>>() } else { sel.clone() };
                    match key {
                        egui::Key::Delete | egui::Key::Backspace if !sel.is_empty() => {
                            let s: BTreeSet<usize> = sel.iter().copied().collect();
                            let mut i = 0;
                            track.notes.retain(|_| {
                                i += 1;
                                !s.contains(&(i - 1))
                            });
                            self.roll.selected.clear();
                        }
                        egui::Key::A if modifiers.command => self.roll.selected = (0..track.notes.len()).collect(),
                        egui::Key::D if modifiers.command && !sel.is_empty() => {
                            let lo = sel.iter().map(|&i| track.notes[i].start).min().unwrap_or(0);
                            let hi = sel.iter().map(|&i| track.notes[i].end()).max().unwrap_or(0);
                            let shift = (hi - lo).div_ceil(grid_t.max(1)) * grid_t.max(1);
                            let base = track.notes.len();
                            for &i in &sel {
                                let n = track.notes[i];
                                track.notes.push(Note { start: n.start + shift, ..n });
                            }
                            self.roll.selected = (base..track.notes.len()).collect();
                        }
                        egui::Key::ArrowUp | egui::Key::ArrowDown if !sel.is_empty() && !modifiers.command => {
                            let step = if modifiers.shift { 12 } else { 1 };
                            let d: i32 = if key == egui::Key::ArrowUp { step } else { -step };
                            for &i in &sel {
                                let n = &mut track.notes[i];
                                n.key = (n.key as i32 + d).clamp(0, 127) as u8;
                            }
                        }
                        egui::Key::ArrowLeft | egui::Key::ArrowRight if !sel.is_empty() && !modifiers.command => {
                            let d = grid_t as i64 * if key == egui::Key::ArrowRight { 1 } else { -1 };
                            if sel.iter().all(|&i| track.notes[i].start as i64 + d >= 0) {
                                for &i in &sel {
                                    track.notes[i].start = (track.notes[i].start as i64 + d) as u32;
                                }
                            }
                        }
                        egui::Key::Q if !modifiers.command => {
                            let idx = all_or_sel();
                            let mut ns: Vec<Note> = idx.iter().map(|&i| track.notes[i]).collect();
                            quantize_notes(&mut ns, grid_t, 1.0, modifiers.shift);
                            for (&i, n) in idx.iter().zip(ns) {
                                track.notes[i] = n;
                            }
                        }
                        egui::Key::Escape => self.roll.selected.clear(),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    // ------------------------------------------------------------ lane

    fn lane_ui(&mut self, ui: &mut egui::Ui, rect: Rect, v: View, t: &Tokens) {
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, t.inset);
        painter.hline(rect.x_range(), rect.top(), Stroke::new(1.0, t.divider));
        let grid = Rect::from_min_max(pos2(v.left, rect.top() + 4.0), pos2(rect.right(), rect.bottom() - 4.0));
        self.grid_lines(&painter, Rect::from_min_max(pos2(v.left, rect.top()), rect.max), v, t, false);
        // the lane picker
        let picker = Rect::from_min_size(pos2(rect.left() + 3.0, rect.top() + 6.0), vec2(GUTTER - 6.0, 20.0));
        let lane = self.roll.lane;
        ui.scope_builder(egui::UiBuilder::new().max_rect(picker), |ui| {
            egui::ComboBox::from_id_salt("lane")
                .width(GUTTER - 8.0)
                .selected_text(egui::RichText::new(short(lane)).font(t.font(10.0)))
                .show_ui(ui, |ui| {
                    for l in Lane::ALL {
                        ui.selectable_value(&mut self.roll.lane, l, l.label());
                    }
                });
        });
        if matches!(lane, Lane::Ctrl(_) | Lane::Volume)
            && ui
                .put(
                    Rect::from_min_size(pos2(rect.left() + 3.0, rect.top() + 30.0), vec2(GUTTER - 6.0, 18.0)),
                    egui::Button::new(egui::RichText::new("Clear").font(t.font(10.0))),
                )
                .on_hover_text(if lane == Lane::Volume {
                    "Set every bar back to the fader's level"
                } else {
                    "Remove every point of this controller from the track"
                })
                .clicked()
            && let Some(tr) = self.doc.project.tracks.get_mut(self.sel)
        {
            match lane {
                Lane::Ctrl(k) => tr.ctrls.retain(|c| c.kind != k),
                _ => tr.levels.clear(),
            }
        }
        let bar_t = self.doc.project.bar_ticks();
        let bars = self.doc.project.bars();
        let Some(tr) = self.doc.project.tracks.get(self.sel) else { return };
        let col = rgb(tr.color);
        let (lo, hi) = match lane {
            Lane::Ctrl(CtrlKind::Bend) => (-8192.0f32, 8191.0f32),
            _ => (0.0, 127.0),
        };
        let y_of = |val: f32| grid.bottom() - (val - lo) / (hi - lo) * grid.height();
        let val_of = |y: f32| (lo + (grid.bottom() - y) / grid.height() * (hi - lo)).clamp(lo, hi);
        match lane {
            Lane::Velocity => {
                for (i, n) in tr.notes.iter().enumerate() {
                    let x = v.x(n.start as f64);
                    if x < grid.left() - 2.0 || x > grid.right() {
                        continue;
                    }
                    let sel = self.roll.selected.contains(&i);
                    let c = if sel { Color32::WHITE } else { col };
                    painter.vline(x, y_of(n.vel as f32)..=grid.bottom(), Stroke::new(2.0, c.gamma_multiply(0.8)));
                    painter.circle_filled(pos2(x, y_of(n.vel as f32)), 2.5, c);
                }
            }
            Lane::Ctrl(kind) => {
                if kind == CtrlKind::Bend {
                    painter.hline(grid.x_range(), y_of(0.0), Stroke::new(1.0, t.track));
                }
                let mut pts: Vec<&Ctrl> = tr.ctrls.iter().filter(|c| c.kind == kind).collect();
                pts.sort_by_key(|c| c.tick);
                let mut last = (grid.left(), y_of(kind.rest() as f32));
                for c in &pts {
                    let x = v.x(c.tick as f64);
                    let y = y_of(c.value as f32);
                    painter.hline(last.0..=x, last.1, Stroke::new(1.5, col));
                    painter.vline(x, last.1.min(y)..=last.1.max(y), Stroke::new(1.5, col));
                    painter.circle_filled(pos2(x, y), 2.0, col);
                    last = (x, y);
                }
                painter.hline(last.0..=grid.right(), last.1, Stroke::new(1.5, col));
            }
            Lane::Volume => {
                // the fader's level, then the volume ramping from bar to bar, filled below
                painter.hline(grid.x_range(), y_of(LEVEL_UNITY as f32), Stroke::new(1.0, t.track));
                let first = (v.tick(grid.left()).max(0.0) / bar_t as f64).floor() as u32;
                let last = ((v.tick(grid.right()) / bar_t as f64).ceil() as u32).min(bars);
                for b in first..last {
                    let (x0, x1) = (v.x((b * bar_t) as f64), v.x(((b + 1) * bar_t) as f64));
                    let (y0, y1) = (y_of(level_at(&tr.levels, b as f64)), y_of(level_at(&tr.levels, (b + 1) as f64)));
                    let fill = col.gamma_multiply(0.18);
                    painter.add(egui::Shape::convex_polygon(
                        vec![pos2(x0, y0), pos2(x1, y1), pos2(x1, grid.bottom()), pos2(x0, grid.bottom())],
                        fill,
                        Stroke::NONE,
                    ));
                    painter.line_segment([pos2(x0, y0), pos2(x1, y1)], Stroke::new(1.5, col));
                }
                for l in tr.levels.iter().filter(|l| l.bar >= first && l.bar < last) {
                    painter.circle_filled(pos2(v.x((l.bar * bar_t) as f64), y_of(l.value as f32)), 3.0, col);
                }
            }
        }
        let resp = ui.interact(grid, ui.scope_id().with("lane"), Sense::click_and_drag());
        if lane == Lane::Volume {
            self.volume_lane(&resp, grid, v, val_of, bar_t, bars);
            return;
        }
        if (resp.dragged() || resp.clicked())
            && let Some(pp) = resp.interact_pointer_pos()
        {
            self.roll.drag = Some(Drag::Lane);
            let value = val_of(pp.y);
            let tick = v.tick(pp.x).max(0.0);
            let reach = (6.0 / v.ppt) as f64;
            let sel = self.roll.selected.clone();
            let p = &mut self.doc.project;
            let step = (p.beat_ticks() / 8).max(1);
            let tr = &mut p.tracks[self.sel];
            match lane {
                Lane::Velocity => {
                    for (i, n) in tr.notes.iter_mut().enumerate() {
                        if (n.start as f64 - tick).abs() <= reach && (sel.is_empty() || sel.contains(&i)) {
                            n.vel = (value.round() as u8).clamp(1, 127);
                            self.roll.last_vel = n.vel;
                        }
                    }
                }
                Lane::Ctrl(kind) => {
                    let at = snap(tick, step);
                    tr.ctrls.retain(|c| !(c.kind == kind && c.tick == at));
                    tr.ctrls.push(Ctrl { tick: at, kind, value: value.round() as i16 });
                    tr.ctrls.sort_by_key(|c| c.tick);
                }
                // drawn by volume_lane
                Lane::Volume => {}
            }
        }
        if resp.drag_stopped() {
            self.roll.drag = None;
        }
        resp.on_hover_text(match lane {
            Lane::Velocity => "Drag over notes to set how hard they're played (only the selected ones, if any)",
            _ => "Drag to draw the controller; it's played back and saved in the MIDI file",
        });
    }

    /// Drawing the selected track's volume: click or drag to set bars (dragging across
    /// bars sets each one it passes), right-click a bar to let it follow its neighbours.
    fn volume_lane(&mut self, resp: &egui::Response, grid: Rect, v: View, val_of: impl Fn(f32) -> f32, bar_t: u32, bars: u32) {
        let bar_at = |x: f32| ((v.tick(x).max(0.0) / bar_t as f64).floor() as u32).min(bars.saturating_sub(1));
        let Some(tr) = self.doc.project.tracks.get_mut(self.sel) else { return };
        if (resp.dragged() || resp.clicked())
            && let Some(pp) = resp.interact_pointer_pos()
        {
            self.roll.drag = Some(Drag::Lane);
            let (bar, value) = (bar_at(pp.x), val_of(pp.y).round());
            // fill the bars between this one and the last, so a quick sweep leaves no gaps
            let from = if resp.drag_started() || resp.clicked() { None } else { self.roll.lane_last };
            match from {
                Some((b0, v0)) if b0 != bar => {
                    let (lo, hi) = (b0.min(bar), b0.max(bar));
                    for b in lo..=hi {
                        let k = (b as f32 - b0 as f32) / (bar as f32 - b0 as f32);
                        set_level(&mut tr.levels, b, (v0 + (value - v0) * k).round().clamp(0.0, 127.0) as u8);
                    }
                }
                _ => set_level(&mut tr.levels, bar, value.clamp(0.0, 127.0) as u8),
            }
            self.roll.lane_last = Some((bar, value));
        }
        if resp.secondary_clicked()
            && let Some(pp) = resp.interact_pointer_pos()
        {
            let bar = bar_at(pp.x);
            tr.levels.retain(|l| l.bar != bar);
        }
        if resp.drag_stopped() {
            self.roll.drag = None;
            self.roll.lane_last = None;
        }
        let hint = "Drag to set the track's volume bar by bar (100 is the fader's level); it ramps between the bars you set and holds after the last. Right-click a bar to let it follow its neighbours.";
        match resp.hover_pos().filter(|p| grid.contains(*p)) {
            Some(p) => {
                let bar = bar_at(p.x);
                let val = level_at(&tr.levels, bar as f64);
                let db = 20.0 * level_gain(val).max(1e-6).log10();
                let set = if tr.levels.iter().any(|l| l.bar == bar) { "set" } else { "following its neighbours" };
                resp.clone().on_hover_text(format!("Bar {}: {:.0} ({db:+.1} dB), {set}\n\n{hint}", bar + 1, val));
            }
            None => {
                resp.clone().on_hover_text(hint);
            }
        }
    }
}

fn short(l: Lane) -> String {
    match l {
        Lane::Velocity => "Vel".into(),
        Lane::Volume => "Vol".into(),
        Lane::Ctrl(CtrlKind::Cc(1)) => "Mod".into(),
        Lane::Ctrl(CtrlKind::Pressure) => "Touch".into(),
        Lane::Ctrl(CtrlKind::Bend) => "Bend".into(),
        Lane::Ctrl(CtrlKind::Cc(74)) => "Bright".into(),
        Lane::Ctrl(CtrlKind::Cc(11)) => "Expr".into(),
        Lane::Ctrl(k) => k.label(),
    }
}
