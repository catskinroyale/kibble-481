# Notes for Claude Code

Kibble-481 (formerly DDream; the crates keep the `dd-` names, `.ddream` files still open, D14): a neon-noir synthesiser + sequencer (Rust, egui). Read with `README.md` and the
decision log `docs/decisions.md` (record product and scope decisions there).

## Layout

| Path | What |
|---|---|
| `crates/dd-dsp` | `poly.rs` (16-voice two-layer synth: polyBLEP, ZDF SVF, envelopes at a 16-sample control rate), `patch.rs` (Patch + factory), `sampler.rs` (city kit player), `kit.rs`/`reverb.rs` (copied from midigen), `effects.rs` (echo, ensemble, soft clip), `fx.rs` (effect types copied from midigen). Trait `Instrument`. |
| `crates/dd-seq` | `project.rs` (Project/Track/Note/Ctrl/Source/Strip, PPQ 960), `timeline.rs` (events + chase), `engine.rs` (transport, mixer, metronome; `Cmd`; `Shared` atomics), `fonts.rs` (rustysynth instruments, `soundfont_dir`), `record.rs` (Take, quantise), `midi_file.rs`, `render.rs` (48 kHz/24-bit WAV), `demo.rs` (Rain on Neon), `compose.rs` (the composer; styles Nocturne/Pursuit/Blackout/Momento, D17). Tracks' bar volumes are `Track::levels` (D16) |
| `crates/dd-gallery` | `slideshow.rs`/`resolve.rs` (from midigen), `doc.rs` (`.kibble` = project + gallery), `export.rs` (Resolve package) |
| `apps/dd-desktop` | `app.rs` (state, engine sync, transport, files, undo), `roll.rs`, `panels.rs`, `sound.rs`, `mixer_ui.rs`, `live.rs` (MIDI in via midir, QWERTY), `audio.rs` (cpal) |
| `apps/dd-cli` | `kibble-481-cli` |

## Commands

```sh
source ~/.cargo/env
cargo fmt && cargo clippy --release --all-targets -- -D warnings
cargo test --release --workspace
KIBBLE_SCREENSHOT=/path/w.bmp KIBBLE_SCREENSHOT_SHOW=mixer,effects cargo run --release -p dd-desktop
./target/release/kibble-481-cli resolve x.kibble out/ --photos dir   # end-to-end export check
ffmpeg -i out.wav -af ebur128=peak=true -f null -                 # loudness (demo ≈ -15 LUFS)
```

## Rules

- Licences: only bundle SoundFonts whose licence allows redistribution and commercial music
  (D4). Note provenance at the top of any file copied from another repo (D2).
- No film, character, composer or trademark names in patches, presets or branding (D3).
- The engine renders both live and offline: keep `render` deterministic and allocation-free
  in steady state; never block the audio thread (commands only via `Cmd`).
- Keep the demo mix between about -16 and -13 LUFS with peaks under 0.95 (a test checks peaks).
- egui 0.37: `ui.scope_id()`, `Id::unique`, `egui::Panel::{top,left,right,bottom}`.
