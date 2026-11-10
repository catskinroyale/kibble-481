//! The interface theme: palettes, fonts and spacing.
//!
//! Adapted (via Ambient MIDI Generator) from BW Darkroom's theme (https://github.com/catskinroyale/bnw-darkroom),
//! itself adapted from LightCraft's `crates/ui-egui/src/theme.rs`
//! (https://github.com/storytold/lightcraft), Copyright (c) 2026 ArtCraft Team and the
//! LightCraft contributors, licensed under the MIT License (see
//! `assets/LICENSE-LightCraft-MIT`). Kibble-481's change: five palettes from its mood
//! board, each one ground, one panel tone, two supporting colours and a lead accent, and
//! its typefaces (all SIL Open Font License 1.1, `assets/fonts/OFL-*.txt`): Michroma for
//! the wordmark and panel labels, Share Tech Mono for readouts and values, VT323 for the
//! patch display, Cutive Mono for typewritten notes, Sorts Mill Goudy for the manual and
//! Jost for everything else. Inter stays as a fallback for symbols.

use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU8, Ordering};

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Shadow, Stroke, TextStyle, Visuals};

/// The app's name, wherever it is shown.
pub const APP_NAME: &str = "Kibble-481";
/// The wordmark: the name in capitals, as the logo shows it.
pub const WORDMARK: &str = "KIBBLE-481";

/// Named font families.
pub const FONT_SEMIBOLD: &str = "semibold";
pub const FONT_MEDIUM: &str = "medium";
pub const FONT_LABEL: &str = "label";
pub const FONT_CRT: &str = "crt";
pub const FONT_TYPEWRITER: &str = "typewriter";
pub const FONT_BOOK: &str = "book";

/// Colours and sizes shared by the custom widgets.
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    /// Side panels, top bar, tool strip.
    pub chrome: Color32,
    /// Behind the piano roll: the palette's ground.
    pub canvas: Color32,
    pub divider: Color32,
    /// Inset boxes (expandable rows, track cards).
    pub inset: Color32,
    pub field: Color32,
    pub button: Color32,
    pub button_border: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub tool_active: Color32,
    pub text: Color32,
    pub text_label: Color32,
    pub text_dim: Color32,
    pub text_disabled: Color32,
    pub icon: Color32,
    pub track: Color32,
    pub thumb: Color32,
    pub thumb_hover: Color32,
    /// The lead accent.
    pub accent: Color32,
    /// The supporting colour (rust, magenta, burnt orange…).
    pub secondary: Color32,
    /// Text on an accent fill.
    pub on_accent: Color32,
    /// Icons of the selected tool.
    pub pick: Color32,
    pub caution: Color32,
    /// Recording, clipping.
    pub alert: Color32,
    pub solo: Color32,
    pub mute: Color32,
    /// Piano keys: the white keys, the black keys and the lines and names on them.
    pub key_white: Color32,
    pub key_black: Color32,
    pub key_line: Color32,
    /// The palette's colours for tracks, in the order of `dd_seq::project::NEON`.
    pub tracks: [Color32; 8],
}

const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// The palettes. Their names describe them rather than borrow from films or books.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Palette {
    /// Warm phosphor on near-black: sodium streetlight and a glowing interior.
    Amber,
    /// Cyan and magenta signage on wet blue-black.
    Neon,
    /// Rust, verdigris, dust and bone: a worn panel.
    Verdigris,
    /// Cream, ink, burnt orange and mustard with a teal accent: a light panel.
    Paperback,
    /// Green monochrome terminal.
    Phosphor,
}

/// One ground, one panel tone, two supporting colours, a lead accent and the text.
struct Swatches {
    ground: u32,
    panel: u32,
    secondary: u32,
    support: u32,
    accent: u32,
    text: u32,
    /// How much of the panel tone goes into the panels (a light dust panel is used
    /// sparingly, so text stays readable).
    panel_strength: f32,
    tracks: [u32; 8],
}

impl Palette {
    pub const ALL: [Palette; 5] = [Palette::Amber, Palette::Neon, Palette::Verdigris, Palette::Paperback, Palette::Phosphor];

    pub fn label(self) -> &'static str {
        match self {
            Palette::Amber => "Sodium Amber",
            Palette::Neon => "Rain on Neon",
            Palette::Verdigris => "Rust and Verdigris",
            Palette::Paperback => "Paperback",
            Palette::Phosphor => "Phosphor",
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            Palette::Amber => "Warm phosphor on near-black: sodium streetlight and a glowing interior",
            Palette::Neon => "Cyan and magenta signage on wet blue-black",
            Palette::Verdigris => "Rust, verdigris, dust and bone: a worn panel",
            Palette::Paperback => "Cream, ink, burnt orange and mustard with a teal accent: a light panel",
            Palette::Phosphor => "Green monochrome terminal",
        }
    }

    /// The key it's saved under.
    pub fn key(self) -> &'static str {
        match self {
            Palette::Amber => "amber",
            Palette::Neon => "neon",
            Palette::Verdigris => "verdigris",
            Palette::Paperback => "paperback",
            Palette::Phosphor => "phosphor",
        }
    }

    pub fn from_key(k: &str) -> Option<Palette> {
        Palette::ALL.into_iter().find(|p| p.key() == k)
    }

    pub fn light(self) -> bool {
        self == Palette::Paperback
    }

    fn swatches(self) -> Swatches {
        match self {
            Palette::Amber => Swatches {
                ground: 0x0b0a0c,
                panel: 0x231c18,
                secondary: 0xb4532a,
                support: 0xc8a878,
                accent: 0xe8a33d,
                text: 0xf1e4c8,
                panel_strength: 1.0,
                tracks: [0xf08a3c, 0x5fa8a0, 0xe8a33d, 0xb4532a, 0xf1e4c8, 0xe0603a, 0x6f9fb0, 0xb8a86a],
            },
            Palette::Neon => Swatches {
                ground: 0x070b14,
                panel: 0x1b1f3b,
                secondary: 0xe0307a,
                support: 0x8a97a8,
                accent: 0x3fd0e0,
                text: 0xd6dbe4,
                panel_strength: 1.0,
                tracks: [0xe0307a, 0x3fd0e0, 0xf0a040, 0x9a70f0, 0x50e0a8, 0xff5a5a, 0x6a8cff, 0xd8dce6],
            },
            Palette::Verdigris => Swatches {
                ground: 0x1a1815,
                panel: 0x6b6558,
                secondary: 0x8c4a2f,
                support: 0x6b6558,
                accent: 0x5e9a8a,
                text: 0xd9cdb4,
                panel_strength: 0.3,
                tracks: [0xc97b4a, 0x5e9a8a, 0xd9cdb4, 0xa85a38, 0xa8b07a, 0xb86a4a, 0x7a9aa0, 0xa89a80],
            },
            Palette::Paperback => Swatches {
                ground: 0xf2e8d0,
                panel: 0x1c1b1f,
                secondary: 0xd2601a,
                support: 0xe0a526,
                accent: 0x1f6f78,
                text: 0x1c1b1f,
                panel_strength: 1.0,
                tracks: [0xd2601a, 0x1f6f78, 0xb88410, 0x8a3a20, 0x3c3a44, 0xb04030, 0x3a7a90, 0x6a6a30],
            },
            Palette::Phosphor => Swatches {
                ground: 0x050805,
                panel: 0x101a13,
                secondary: 0x2e7d45,
                support: 0x2e7d45,
                accent: 0x6bff8f,
                text: 0xe6f2e8,
                panel_strength: 1.0,
                tracks: [0x6bff8f, 0x3fcf6a, 0xa8ffb8, 0x2e9d55, 0xe6f2e8, 0x8fe0a0, 0x4fbf80, 0xc8f0c0],
            },
        }
    }

    fn tokens(self) -> Tokens {
        let s = self.swatches();
        let (g, x, a, sec) = (rgb(s.ground), rgb(s.text), rgb(s.accent), rgb(s.secondary));
        let tracks = s.tracks.map(rgb);
        if self.light() {
            let white = Color32::WHITE;
            return Tokens {
                chrome: mix(g, x, 0.04),
                canvas: mix(g, white, 0.4),
                divider: mix(g, x, 0.14),
                inset: mix(g, x, 0.07),
                field: mix(g, white, 0.6),
                button: mix(g, white, 0.45),
                button_border: mix(g, x, 0.28),
                hover: mix(g, x, 0.1),
                pressed: mix(g, x, 0.15),
                tool_active: mix(g, x, 0.15),
                text: x,
                text_label: mix(x, g, 0.18),
                text_dim: mix(x, g, 0.42),
                text_disabled: mix(x, g, 0.65),
                icon: mix(x, g, 0.3),
                track: mix(g, x, 0.3),
                thumb: a,
                thumb_hover: sec,
                accent: a,
                secondary: sec,
                on_accent: g,
                pick: x,
                caution: sec,
                alert: rgb(0xc0392b),
                solo: rgb(s.support),
                mute: mix(a, g, 0.25),
                key_white: mix(g, white, 0.75),
                key_black: x,
                key_line: mix(g, x, 0.45),
                tracks,
            };
        }
        let p = mix(g, rgb(s.panel), s.panel_strength);
        Tokens {
            chrome: mix(g, p, 0.45),
            canvas: g,
            divider: g,
            inset: mix(g, p, 0.75),
            field: mix(g, p, 0.3),
            button: p,
            button_border: mix(p, x, 0.16),
            hover: mix(p, x, 0.09),
            pressed: mix(p, x, 0.15),
            tool_active: mix(p, x, 0.15),
            text: x,
            text_label: mix(x, p, 0.2),
            text_dim: mix(x, p, 0.45),
            text_disabled: mix(x, p, 0.7),
            icon: mix(x, p, 0.35),
            track: mix(p, x, 0.28),
            thumb: a,
            thumb_hover: mix(a, x, 0.5),
            accent: a,
            secondary: sec,
            on_accent: g,
            pick: x,
            caution: mix(sec, x, 0.3),
            alert: if self == Palette::Neon { sec } else { mix(rgb(0xe04030), sec, 0.3) },
            solo: a,
            mute: mix(sec, x, 0.15),
            key_white: mix(x, p, 0.12),
            key_black: g,
            key_line: mix(x, g, 0.5),
            tracks,
        }
    }
}

static TOKENS: LazyLock<[Tokens; 5]> = LazyLock::new(|| Palette::ALL.map(Palette::tokens));
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// The colour a track shows in: its own colour from `NEON`, in this palette's tones.
pub fn track_colour(c: [u8; 3]) -> Color32 {
    match dd_seq::project::NEON.iter().position(|n| *n == c) {
        Some(i) => TOKENS[CURRENT.load(Ordering::Relaxed) as usize % 5].tracks[i],
        None => Color32::from_rgb(c[0], c[1], c[2]),
    }
}

impl Tokens {
    /// The tokens of the palette in use.
    pub fn get(_ctx: &egui::Context) -> Tokens {
        TOKENS[CURRENT.load(Ordering::Relaxed) as usize % 5]
    }

    pub fn of(p: Palette) -> Tokens {
        TOKENS[p as usize]
    }

    /// Jost.
    pub fn font(&self, size: f32) -> FontId {
        FontId::proportional(size)
    }

    /// Jost SemiBold.
    pub fn semibold(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(FONT_SEMIBOLD.into()))
    }

    /// Jost Medium.
    pub fn medium(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(FONT_MEDIUM.into()))
    }

    /// Michroma: the wordmark and panel labels.
    pub fn label(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(FONT_LABEL.into()))
    }

    /// Share Tech Mono: readouts and values.
    pub fn mono(&self, size: f32) -> FontId {
        FontId::monospace(size)
    }

    /// VT323: the patch display.
    pub fn crt(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(FONT_CRT.into()))
    }

    /// Cutive Mono: typewritten notes.
    pub fn typewriter(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(FONT_TYPEWRITER.into()))
    }

    /// Sorts Mill Goudy: the manual and the About page.
    pub fn book(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(FONT_BOOK.into()))
    }
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let mut add = |name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    };
    add("Jost", include_bytes!("../../../assets/fonts/Jost-Regular.ttf"));
    add("Jost-Medium", include_bytes!("../../../assets/fonts/Jost-Medium.ttf"));
    add("Jost-SemiBold", include_bytes!("../../../assets/fonts/Jost-SemiBold.ttf"));
    add("Michroma", include_bytes!("../../../assets/fonts/Michroma-Regular.ttf"));
    add("ShareTechMono", include_bytes!("../../../assets/fonts/ShareTechMono-Regular.ttf"));
    add("VT323", include_bytes!("../../../assets/fonts/VT323-Regular.ttf"));
    add("CutiveMono", include_bytes!("../../../assets/fonts/CutiveMono-Regular.ttf"));
    add("SortsMillGoudy", include_bytes!("../../../assets/fonts/SortsMillGoudy-Regular.ttf"));
    add("Inter", include_bytes!("../../../assets/fonts/Inter-Regular.ttf"));
    // Inter and egui's own faces stay as fallbacks for symbols the others lack.
    let defaults = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mono_defaults = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    let chain = |first: &str| {
        let mut v = vec![first.to_string(), "Inter".to_string()];
        v.extend(defaults.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, chain("Jost"));
    fonts.families.insert(FontFamily::Name(FONT_SEMIBOLD.into()), chain("Jost-SemiBold"));
    fonts.families.insert(FontFamily::Name(FONT_MEDIUM.into()), chain("Jost-Medium"));
    fonts.families.insert(FontFamily::Name(FONT_LABEL.into()), chain("Michroma"));
    fonts.families.insert(FontFamily::Name(FONT_CRT.into()), chain("VT323"));
    fonts.families.insert(FontFamily::Name(FONT_TYPEWRITER.into()), chain("CutiveMono"));
    fonts.families.insert(FontFamily::Name(FONT_BOOK.into()), chain("SortsMillGoudy"));
    let mut mono = vec!["ShareTechMono".to_string()];
    mono.extend(mono_defaults);
    mono.push("Inter".into());
    fonts.families.insert(FontFamily::Monospace, mono);
    fonts
}

fn visuals(t: &Tokens, dark: bool) -> Visuals {
    let mut v = if dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.chrome;
    v.window_fill = t.chrome;
    v.extreme_bg_color = t.field;
    v.faint_bg_color = t.inset;
    v.window_stroke = Stroke::new(1.0, t.button_border);
    v.window_corner_radius = CornerRadius::same(4);
    v.menu_corner_radius = CornerRadius::same(4);
    // selected buttons and menu items: filled with the accent, in the ground's colour
    v.selection.bg_fill = t.accent;
    v.selection.stroke = Stroke::new(1.0, t.on_accent);
    v.text_cursor.stroke = Stroke::new(2.0, t.accent);
    v.hyperlink_color = t.accent;
    v.warn_fg_color = t.caution;
    v.error_fg_color = t.alert;
    let (popup, window) = if dark { (160, 180) } else { (50, 60) };
    v.popup_shadow = Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(popup) };
    v.window_shadow = Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(window) };
    let w = &mut v.widgets;
    for (state, fill) in [
        (&mut w.noninteractive, t.chrome),
        (&mut w.inactive, t.button),
        (&mut w.hovered, t.hover),
        (&mut w.active, t.pressed),
        (&mut w.open, t.hover),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.corner_radius = CornerRadius::same(3);
        state.fg_stroke = Stroke::new(1.0, t.text_label);
    }
    w.noninteractive.bg_stroke = Stroke::new(1.0, t.divider);
    w.inactive.bg_stroke = Stroke::new(1.0, t.button_border);
    w.hovered.bg_stroke = Stroke::new(1.0, t.accent.gamma_multiply(0.7));
    w.hovered.fg_stroke = Stroke::new(1.0, t.text);
    w.active.fg_stroke = Stroke::new(1.0, t.text);
    v
}

/// Use a palette: its visuals for whichever theme egui shows, light or dark to suit it.
pub fn set_palette(ctx: &egui::Context, p: Palette) {
    CURRENT.store(p as u8, Ordering::Relaxed);
    let t = Tokens::of(p);
    ctx.set_visuals_of(egui::Theme::Dark, visuals(&t, !p.light()));
    ctx.set_visuals_of(egui::Theme::Light, visuals(&t, !p.light()));
    ctx.set_theme(if p.light() { egui::ThemePreference::Light } else { egui::ThemePreference::Dark });
}

/// Install the fonts, the palette and the spacing.
pub fn install(ctx: &egui::Context, p: Palette) {
    ctx.set_fonts(fonts());
    set_palette(ctx, p);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(8.0, 3.0);
        s.spacing.interact_size.y = 22.0;
        let semibold = FontFamily::Name(FONT_SEMIBOLD.into());
        s.text_styles.insert(TextStyle::Body, FontId::proportional(13.5));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(13.5));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(11.5));
        s.text_styles.insert(TextStyle::Monospace, FontId::monospace(12.5));
        s.text_styles.insert(TextStyle::Heading, FontId::new(16.0, semibold));
        s.animation_time = 0.08;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Body text, labels and text on the accent stay readable in every palette (WCAG
    /// contrast: 4.5 for text, 3 for dim text and large labels).
    #[test]
    fn palettes_are_readable() {
        fn lum(c: Color32) -> f64 {
            let f = |v: u8| {
                let v = v as f64 / 255.0;
                if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
            };
            0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
        }
        fn contrast(a: Color32, b: Color32) -> f64 {
            let (x, y) = (lum(a), lum(b));
            (x.max(y) + 0.05) / (x.min(y) + 0.05)
        }
        for p in Palette::ALL {
            let t = Tokens::of(p);
            for (what, fg, bg, min) in [
                ("text on chrome", t.text, t.chrome, 7.0),
                ("labels on chrome", t.text_label, t.chrome, 4.5),
                ("labels on buttons", t.text_label, t.button, 4.5),
                ("dim text on chrome", t.text_dim, t.chrome, 3.0),
                ("accent on chrome", t.accent, t.chrome, 3.0),
                ("text on the accent", t.on_accent, t.accent, 4.5),
            ] {
                let c = contrast(fg, bg);
                assert!(c >= min, "{}: {what} is {c:.2}:1", p.label());
            }
        }
    }
}
