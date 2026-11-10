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
