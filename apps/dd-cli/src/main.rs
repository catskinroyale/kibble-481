//! kibble-481-cli: Kibble-481 without the window.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32};

use dd_gallery::doc::Document;
use dd_gallery::export::export_for_resolve;
use dd_gallery::resolve::PhotoOrder;
use dd_gallery::slideshow::Fit;
use dd_seq::fonts::{FontBank, soundfont_dir};

const USAGE: &str = "kibble-481-cli: Kibble-481 from the command line

  kibble-481-cli demo <out.kibble>                 write the hand-written scene, Rain on Neon
  kibble-481-cli compose <out.kibble> [--style nocturne|pursuit|blackout|momento|noir] [--seed <n>] [--bpm <n>] [--bars <n>] [--key <0-11>]
                                               compose a new piece (a random seed if none given)
  kibble-481-cli render <in.kibble|.mid> <out.wav> [--stem <track>]
                                               render the mix (24-bit, 48 kHz), or one track
  kibble-481-cli midi <in.kibble> <out.mid>        export a standard MIDI file
  kibble-481-cli import <in.mid> <out.kibble>      turn any MIDI file into a project
  kibble-481-cli resolve <in.kibble> <out-dir> [--photos <dir>] [--by-date] [--fps <n>]
                     [--hold <secs>] [--fade <secs>] [--fit-track] [--stems]
                                               soundtrack, FCPXML timeline, EDL markers and cues
  kibble-481-cli patches                           list the synth's factory patches
  kibble-481-cli fonts                             list the loaded SoundFonts and their presets

SoundFonts are read from $KIBBLE_SOUNDFONTS or the bundled soundfonts folder.";

fn fonts() -> FontBank {
    let mut f = FontBank::default();
    match soundfont_dir() {
        Some(d) => {
            for e in f.load_dir(&d) {
                eprintln!("warning: {e}");
            }
        }
        None => eprintln!("warning: no SoundFont folder found; SoundFont tracks will be silent"),
    }
    f
}

fn open(path: &Path, fonts: &FontBank) -> Result<Document, String> {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()) {
        Some(e) if e == "mid" || e == "midi" => Ok(Document::new(dd_seq::midi_file::load(path, fonts)?)),
        _ => Document::load(path),
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn run(args: Vec<String>) -> Result<(), String> {
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let arg = |i: usize| args.get(i).map(PathBuf::from).ok_or_else(|| USAGE.to_string());
    let (progress, cancel) = (AtomicU32::new(0), AtomicBool::new(false));
    match cmd {
        "demo" => {
            Document::new(dd_seq::demo::rain_on_neon()).save(&arg(1)?)?;
        }
        "compose" => {
            let entropy = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
            let mut r = dd_seq::compose::Recipe::random(entropy);
            if let Some(n) = flag(&args, "--seed") {
                r.seed = n.parse().map_err(|_| "--seed takes a number")?;
            }
            if let Some(n) = flag(&args, "--style") {
                r.style = *dd_seq::compose::Style::ALL
                    .iter()
                    .find(|s| s.label().eq_ignore_ascii_case(&n))
                    .ok_or("--style takes nocturne, pursuit, blackout, momento or noir")?;
            }
            if let Some(n) = flag(&args, "--key") {
                r.root = n.parse::<u8>().map_err(|_| "--key takes 0 (C) to 11 (B)")? % 12;
            }
            let bpm = flag(&args, "--bpm").map_or(Ok(r.style.tempo()), |n| n.parse::<f64>()).map_err(|_| "--bpm takes a number")?;
            let bars = flag(&args, "--bars").map_or(Ok(24), |n| n.parse::<u32>()).map_err(|_| "--bars takes a number")?;
            let p = dd_seq::compose::compose(&r, bpm, (4, 4), bars);
            println!(
                "\"{}\": {}, seed {}, {}, {} bars at {} bpm, {} tracks",
                p.title,
                r.style.label(),
                r.seed,
                r.key_label(),
                p.bars(),
                p.bpm,
                p.tracks.len()
            );
            Document::new(p).save(&arg(1)?)?;
        }
        "render" => {
            let f = fonts();
            let doc = open(&arg(1)?, &f)?;
            let stem = match flag(&args, "--stem") {
                Some(s) => Some(
                    doc.project
                        .tracks
                        .iter()
                        .position(|t| t.name.eq_ignore_ascii_case(&s))
                        .or_else(|| s.parse::<usize>().ok().map(|n| n.saturating_sub(1)))
                        .ok_or(format!("No track {s}"))?,
                ),
                None => None,
            };
            let out = arg(2)?;
            dd_seq::render::render_wav(&doc.project, &f, stem, &out, &progress, &cancel)?;
            println!("{}", out.display());
        }
        "midi" => {
            let f = fonts();
            let doc = open(&arg(1)?, &f)?;
            dd_seq::midi_file::save(&doc.project, &arg(2)?).map_err(|e| e.to_string())?;
        }
        "import" => {
            let f = fonts();
            let p = dd_seq::midi_file::load(&arg(1)?, &f)?;
            println!("{} tracks, {} bars at {} bpm", p.tracks.len(), p.bars(), p.bpm);
            Document::new(p).save(&arg(2)?)?;
        }
        "resolve" => {
            let f = fonts();
            let mut doc = open(&arg(1)?, &f)?;
            let g = &mut doc.gallery;
            if let Some(d) = flag(&args, "--photos") {
                g.folder = Some(d.into());
            }
            if args.iter().any(|a| a == "--by-date") {
                g.order = PhotoOrder::Date;
            }
            if let Some(n) = flag(&args, "--fps") {
                g.fps = n.parse().map_err(|_| "--fps takes a number")?;
            }
            if let Some(n) = flag(&args, "--hold") {
                g.hold = n.parse().map_err(|_| "--hold takes seconds")?;
            }
            if let Some(n) = flag(&args, "--fade") {
                g.fade = n.parse().map_err(|_| "--fade takes seconds")?;
            }
            if args.iter().any(|a| a == "--fit-track") {
                // size the track to the photos
                let photos = g.folder.as_ref().map(|d| dd_gallery::resolve::photos(d, g.order)).transpose()?.unwrap_or_default();
                g.fit = Fit::TrackToImages;
                g.images = photos.len().max(1) as u32;
                let bars = doc.plan().bars();
                doc.project.length_bars = bars;
            }
            let plan = doc.plan();
            println!(
                "{} images, {:.1}–{:.1} s each, music {:.1} s ({} bars)",
                plan.images,
                plan.holds().0,
                plan.holds().1,
                plan.total,
                plan.bars()
            );
            let name = dd_gallery::export::version_name(&doc.project, std::time::SystemTime::now());
            let out = export_for_resolve(&doc, &f, &arg(2)?, &name, args.iter().any(|a| a == "--stems"), &progress, &cancel)?;
            println!("{}", out.wav.display());
            if let Some(t) = &out.timeline {
                println!("{}", t.display());
            }
            println!("{}\n{}", out.markers.display(), out.cues.display());
            for s in &out.stems {
                println!("{}", s.display());
            }
        }
        "patches" => {
            for p in dd_dsp::patch::factory() {
                println!("{:<12} {}", p.category.label(), p.name);
            }
        }
        "fonts" => {
            for font in fonts().fonts {
                println!("{}", font.name);
                for p in &font.presets {
                    println!("  {:>3}:{:<3} {}", p.bank, p.program, p.name);
                }
            }
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn main() {
    if let Err(e) = run(std::env::args().skip(1).collect()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
