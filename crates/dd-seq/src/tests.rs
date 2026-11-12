//! Whole-engine tests with the bundled SoundFonts.

use std::path::Path;

use crate::demo::{SAX_FONT, rain_on_neon};
use crate::engine::{Cmd, Engine, Shared};
use crate::fonts::{FontBank, make_instrument};
use crate::project::Source;
use crate::render::render_buffers;
use crate::timeline::Msg;

fn fonts() -> FontBank {
    let mut f = FontBank::default();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../soundfonts");
    let errors = f.load_dir(&dir);
    assert!(errors.is_empty(), "{errors:?}");
    f
}

fn peak(v: &[f32]) -> f32 {
    v.iter().fold(0.0f32, |m, x| m.max(x.abs()))
}

#[test]
fn bundled_fonts_have_the_presets_the_demo_uses() {
    let f = fonts();
    let p = rain_on_neon();
    for t in &p.tracks {
        if let Source::SoundFont { font, bank, program, name } = &t.source {
            let lf = f.get(font).unwrap_or_else(|| panic!("{font} not loaded"));
            assert!(lf.find(*bank, *program).is_some(), "{name}: {font} has no {bank}:{program}: {:?}", lf.presets);
        }
    }
    assert!(f.get(SAX_FONT).is_some());
}

#[test]
fn demo_renders_at_a_healthy_level() {
    let f = fonts();
    let p = rain_on_neon();
    let (l, r) = render_buffers(&p, &f, 24000, None);
    assert!(l.iter().chain(&r).all(|x| x.is_finite()));
    let pk = peak(&l).max(peak(&r));
    // loud enough, with headroom (only transients such as thunder reach the soft clipper)
    assert!(pk > 0.25 && pk < 0.95, "peak {pk}");
    // every track makes sound in the mix
    for (i, t) in p.tracks.iter().enumerate() {
        let (inst, err) = make_instrument(&t.source, &f, 24000);
        assert!(err.is_none(), "{err:?}");
        drop(inst);
        let mut solo = p.clone();
        solo.tracks.iter_mut().enumerate().for_each(|(j, x)| x.strip.solo = j == i);
        let (l, _) = render_buffers(&solo, &f, 24000, None);
        assert!(peak(&l) > 0.01, "{} is silent", t.name);
    }
    // the end is the reverb dying away
    let tail = &l[l.len() - 2400..];
    assert!(peak(tail) < pk * 0.5);
}

#[test]
fn rendering_is_deterministic() {
    let f = fonts();
    let p = rain_on_neon();
    let a = render_buffers(&p, &f, 16000, Some(12.0));
    let b = render_buffers(&p, &f, 16000, Some(12.0));
    assert!(a == b);
}

#[test]
fn live_notes_sound_while_stopped_and_loop_wraps() {
    let f = fonts();
    let p = rain_on_neon();
    let sr = 24000;
    let shared = Shared::new();
    let mut e = Engine::new(sr, shared.clone());
    e.handle(Cmd::Mix(Box::new(crate::engine::MixSettings::of(&p))));
    e.handle(Cmd::Instruments(p.tracks.iter().map(|t| make_instrument(&t.source, &f, sr).0).collect()));
    e.handle(Cmd::Timeline(std::sync::Arc::new(crate::timeline::Timeline::from_project(&p))));
    e.handle(Cmd::Live(0, Msg::On(60, 110)));
    let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
    e.render(&mut l, &mut r);
    assert!(peak(&l) > 0.01);
    assert!(!e.is_playing());
    let bar = p.bar_secs();
    e.handle(Cmd::Loop(Some((bar, bar * 2.0))));
    e.handle(Cmd::Play(bar * 1.9));
    for _ in 0..10 {
        e.render(&mut l, &mut r);
    }
    let pos = e.position();
    assert!(pos >= bar && pos < bar * 2.0, "{pos}");
    assert!((shared.position() - pos).abs() < 1e-9);
}

#[test]
fn count_in_starts_before_zero() {
    let p = rain_on_neon();
    let f = FontBank::default();
    let mut e = Engine::new(24000, Shared::new());
    e.handle(Cmd::Mix(Box::new(crate::engine::MixSettings::of(&p))));
    e.handle(Cmd::Instruments(p.tracks.iter().map(|t| make_instrument(&t.source, &f, 24000).0).collect()));
    e.handle(Cmd::Timeline(std::sync::Arc::new(crate::timeline::Timeline::from_project(&p))));
    e.handle(Cmd::Play(-p.bar_secs()));
    let (mut l, mut r) = (vec![0.0; 2400], vec![0.0; 2400]);
    e.render(&mut l, &mut r);
    // the first click
    assert!(peak(&l) > 0.2);
    assert!(e.position() < 0.0);
}

/// Changing a track's sound while it plays: the new sound picks up the notes already
/// held, at once, rather than waiting for the next note.
#[test]
fn a_new_sound_takes_over_held_notes() {
    use crate::project::{Note, Project, Track};
    let f = fonts();
    let sr = 24000;
    let mut p = Project { bpm: 60.0, length_bars: 8, ..Project::default() };
    let mut t = Track::new("Pad", Source::Synth(dd_dsp::patch::by_name("Rain Pad")), 0);
    // one chord held for eight bars
    for k in [57, 60, 64] {
        t.notes.push(Note { start: 0, len: crate::project::PPQ * 32, key: k, vel: 100 });
    }
    p.tracks.push(t);
    for to in [
        Source::SoundFont { font: crate::demo::GM_FONT.into(), bank: 0, program: 48, name: "Strings".into() },
        Source::Synth(dd_dsp::patch::by_name("Smoky Lead")), // poly to mono
    ] {
        let mut e = Engine::new(sr, Shared::new());
        e.handle(Cmd::Mix(Box::new(crate::engine::MixSettings::of(&p))));
        e.handle(Cmd::Instruments(vec![make_instrument(&p.tracks[0].source, &f, sr).0]));
        e.handle(Cmd::Timeline(std::sync::Arc::new(crate::timeline::Timeline::from_project(&p))));
        e.handle(Cmd::Play(0.0));
        let (mut l, mut r) = (vec![0.0; sr as usize * 4], vec![0.0; sr as usize * 4]);
        e.render(&mut l, &mut r);
        match &to {
            Source::Synth(patch) => e.handle(Cmd::Patch(0, Box::new(patch.clone()))),
            _ => e.handle(Cmd::Instrument(0, make_instrument(&to, &f, sr).0)),
        }
        // let the reverb of the old sound die, then listen to the dry track
        let (mut l, mut r) = (vec![0.0; sr as usize * 2], vec![0.0; sr as usize * 2]);
        e.render(&mut l, &mut r);
        let peak = e.shared_peak(0);
        assert!(peak > 0.01, "{to:?} stays silent over the held chord ({peak})");
    }
}

/// Clicking a sound while stopped plays a short preview, which then ends.
#[test]
fn an_audition_plays_and_ends() {
    let f = FontBank::default();
    let sr = 24000;
    let mut e = Engine::new(sr, Shared::new());
    e.handle(Cmd::Instruments(vec![make_instrument(&Source::Synth(dd_dsp::patch::by_name("Night Bells")), &f, sr).0]));
    e.handle(Cmd::Audition(0, vec![(0.0, 0.4, 72, 100), (0.5, 0.4, 76, 100)]));
    let (mut l, mut r) = (vec![0.0; sr as usize], vec![0.0; sr as usize]);
    e.render(&mut l, &mut r);
    assert!(peak(&l) > 0.01);
    assert!(!e.is_playing());
    for _ in 0..12 {
        e.render(&mut l, &mut r);
    }
    assert!(peak(&l) < 0.002, "the preview should end: {}", peak(&l));
}

/// The exported WAV rings on until the reverb has died away, rather than stopping a
/// fixed time after the music.
#[test]
fn export_lets_the_tail_ring_out() {
    use std::sync::atomic::{AtomicBool, AtomicU32};
    let f = fonts();
    let mut p = rain_on_neon();
    p.fx.space = dd_dsp::fx::Space::BlackHole;
    let dir = std::env::temp_dir().join(format!("kibble-tail-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tail.wav");
    let secs = crate::render::render_wav(&p, &f, None, &path, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
    let mut rd = hound::WavReader::open(&path).unwrap();
    let samples: Vec<f32> = rd.samples::<i32>().map(|s| s.unwrap() as f32 / 8_388_607.0).collect();
    let _ = std::fs::remove_dir_all(&dir);
    let sr = crate::render::EXPORT_RATE as f64;
    assert!((samples.len() as f64 / 2.0 / sr - secs).abs() < 1e-6);
    assert!(secs > p.music_secs() + crate::project::TAIL_SECS, "a long hall rings past the old 4 s tail: {secs:.1} s");
    // what was cut at the old length is still audible there
    let at = |t: f64| ((t * sr) as usize * 2).min(samples.len());
    let old_end = &samples[at(p.music_secs() + crate::project::TAIL_SECS - 0.25)..at(p.music_secs() + crate::project::TAIL_SECS)];
    assert!(old_end.iter().any(|s| s.abs() > 1e-3), "the tail was still sounding at the old cut");
    // and the file ends in silence
    let last = &samples[samples.len() - (0.2 * sr) as usize * 2..];
    assert!(last.iter().all(|s| s.abs() < 1e-4));
}

/// A synth track holding one chord for eight bars, for the bar-level tests.
fn held_chord() -> crate::project::Project {
    use crate::project::{Note, PPQ, Project, Track};
    let mut p = Project { bpm: 120.0, length_bars: 8, ..Project::default() };
    let mut t = Track::new("Pad", Source::Synth(dd_dsp::patch::by_name("Rain Pad")), 0);
    t.strip.reverb = 0.0;
    for k in [57, 60, 64] {
        t.notes.push(Note { start: 0, len: 8 * 4 * PPQ, key: k, vel: 100 });
    }
    p.tracks.push(t);
    p.fx.duck = 0.0;
    p
}

/// RMS of the left channel over bar `b` (from 0), skipping its first quarter.
fn bar_rms(l: &[f32], p: &crate::project::Project, sr: u32, b: u32) -> f32 {
    let n = (p.bar_secs() * sr as f64) as usize;
    let s = &l[b as usize * n + n / 4..(b as usize + 1) * n];
    (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
}

/// Bar levels: silent where set to 0, ramping between the bars set, holding after.
#[test]
fn bar_levels_shape_the_volume() {
    use crate::project::{Level, set_level};
    let f = fonts();
    let sr = 24000;
    let mut p = held_chord();
    let (flat, _) = render_buffers(&p, &f, sr, None);
    // silent in bar 1, ramping back up to the fader's level at bar 5, then held
    p.tracks[0].levels = vec![Level { bar: 1, value: 0 }];
    set_level(&mut p.tracks[0].levels, 5, 100);
    let (shaped, _) = render_buffers(&p, &f, sr, None);
    let (a, b) = (bar_rms(&flat, &p, sr, 6), bar_rms(&shaped, &p, sr, 6));
    assert!((a - b).abs() < a * 0.05, "held after the last: {a} vs {b}");
    // before the first point it is the first point's level (silent)
    assert!(bar_rms(&shaped, &p, sr, 0) < bar_rms(&flat, &p, sr, 0) * 0.02);
    let ramp: Vec<f32> = (1..5).map(|b| bar_rms(&shaped, &p, sr, b)).collect();
    assert!(ramp.windows(2).all(|w| w[1] > w[0]), "ramps up: {ramp:?}");
    let ratios: Vec<f32> = (1..5).map(|b| bar_rms(&shaped, &p, sr, b) / bar_rms(&flat, &p, sr, b)).collect();
    assert!(ratios.windows(2).all(|w| w[1] > w[0]) && ratios[3] < 0.95, "below the fader's level until bar 5: {ratios:?}");
}

/// Moving a section takes its bar levels with it; a new section has none, so it follows.
#[test]
fn bar_levels_move_with_their_sections() {
    use crate::project::{Block, Level, Part, Section};
    let mut p = held_chord();
    p.sections = vec![Section::new(0, Part::Neon), Section::new(4, Part::Swell)];
    p.tracks[0].levels = vec![Level { bar: 1, value: 40 }, Level { bar: 5, value: 120 }];
    // swap them and add a Fade of 3 bars at the start
    let a = p.arrangement();
    let fade = Block { name: "Fade".into(), part: Some(Part::Fade), bars: 3 };
    p.rearrange(&[fade, a[1].clone(), a[0].clone()], &[None, Some(1), Some(0)]);
    assert_eq!(p.length_bars, 11);
    assert_eq!(p.tracks[0].levels, vec![Level { bar: 4, value: 120 }, Level { bar: 8, value: 40 }]);
    // shortening a section keeps its last level at its new end
    let mut a = p.arrangement();
    a[1].bars = 1;
    let origin: Vec<Option<usize>> = (0..a.len()).map(Some).collect();
    p.rearrange(&a, &origin);
    assert_eq!(p.tracks[0].levels, vec![Level { bar: 3, value: 120 }, Level { bar: 5, value: 40 }]);
}

/// The MIDI file's volume follows the bar levels a beat at a time.
#[test]
fn bar_levels_reach_the_midi_file() {
    use crate::project::Level;
    use midly::{MidiMessage, Smf, TrackEventKind};
    let mut p = held_chord();
    p.tracks[0].levels = vec![Level { bar: 0, value: 100 }, Level { bar: 2, value: 50 }];
    let bytes = crate::midi_file::to_bytes(&p).unwrap();
    let smf = Smf::parse(&bytes).unwrap();
    let vols: Vec<u8> = smf.tracks[1]
        .iter()
        .filter_map(|e| match e.kind {
            TrackEventKind::Midi { message: MidiMessage::Controller { controller, value }, .. } if controller == 7 => Some(value.as_int()),
            _ => None,
        })
        .collect();
    assert_eq!(vols.first(), Some(&100));
    assert_eq!(vols.last(), Some(&50));
    assert!(vols.len() >= 8 && vols.windows(2).all(|w| w[1] < w[0]), "{vols:?}");
    // and the project comes back with them
    assert_eq!(crate::midi_file::from_bytes(&bytes, &FontBank::default()).unwrap().tracks[0].levels, p.tracks[0].levels);
}

/// Every style renders at a healthy level, loud enough and short of the clipper.
#[test]
fn every_style_renders_at_a_healthy_level() {
    use crate::compose::{Recipe, Style, compose};
    let f = fonts();
    for style in Style::ALL {
        for seed in [3, 77] {
            let p = compose(&Recipe { seed, style, ..Recipe::default() }, style.tempo(), (4, 4), 16);
            let (l, r) = render_buffers(&p, &f, 24000, None);
            let pk = peak(&l).max(peak(&r));
            assert!(pk > 0.2 && pk < 0.98, "{style:?} {seed}: peak {pk}");
        }
    }
}

/// A track's low and high cuts work on its sound whatever plays it, a SoundFont
/// included: the high cut takes the top off, the low cut the bottom.
#[test]
fn cuts_shape_soundfont_tracks() {
    let f = fonts();
    let mut p = rain_on_neon();
    let i = p.tracks.iter().position(|t| matches!(t.source, Source::SoundFont { .. }) && !t.notes.is_empty()).expect("a SoundFont track");
    p.tracks[i].strip.solo = true;
    p.tracks[i].strip.reverb = 0.0;
    p.tracks[i].strip.echo = 0.0;
    // brightness: the energy of the sample-to-sample change against the energy itself
    let bright = |p: &crate::project::Project| {
        let (l, _) = render_buffers(p, &f, 24000, None);
        let e: f64 = l.iter().map(|x| (*x as f64).powi(2)).sum();
        let d: f64 = l.windows(2).map(|w| ((w[1] - w[0]) as f64).powi(2)).sum();
        (e, d / e.max(1e-12))
    };
    let (e0, b0) = bright(&p);
    p.tracks[i].strip.high_cut = 400.0;
    let (_, b1) = bright(&p);
    assert!(e0 > 0.0 && b1 < b0 * 0.3, "high cut: brightness {b0} -> {b1}");
    p.tracks[i].strip.high_cut = crate::project::HIGH_CUT_OFF;
    p.tracks[i].strip.low_cut = 2000.0;
    let (e2, b2) = bright(&p);
    assert!(b2 > b0 * 1.5 && e2 < e0, "low cut: brightness {b0} -> {b2}, energy {e0} -> {e2}");
}
