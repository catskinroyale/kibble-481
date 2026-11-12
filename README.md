# Kibble-481

(Formerly DDream. Projects saved as `.ddream` still open, and settings carry over.)

A synthesiser and sequencer in one window, for writing neon-noir film scores in MIDI:
rain on dark streets, sagging polysynth brass, bells falling like water, a lone sax at
3 a.m. You play it, record it, edit it on a piano roll, mix it, and hand the soundtrack
and a gallery of your photos to DaVinci Resolve as a ready-made timeline.

```sh
cargo run --release -p dd-desktop        # the app
cargo run --release -p dd-cli -- --help  # the same engine from the command line
```

It opens on a newly composed piece every time: press Space to hear it, **🎲 New piece** for
another (new notes and a new arrangement of sections), or **New notes** for new notes to
the sections you have. The hand-written **Rain on Neon** is under File.

## Install

Download `Kibble-481-<version>-macos-arm64.zip` from
[Releases](https://github.com/catskinroyale/kibble-481/releases), unzip it and move
`Kibble-481.app` to Applications. The app is not signed, so the first time, right-click it
and choose **Open** (or run `xattr -dr com.apple.quarantine /Applications/Kibble-481.app`).
The bundle carries both binaries (`Contents/MacOS/kibble-481-cli` is the command line) and
every SoundFont, including the optional extras.

On **Windows** (64-bit), download `Kibble-481-<version>-windows-x64.zip`, unzip it anywhere
and run `kibble-481.exe`; keep the `soundfonts` folder beside it. It is not signed either:
if SmartScreen stops it, choose **More info › Run anyway**. `kibble-481-cli.exe` is the
command line.

## Building

You need [Rust](https://rustup.rs); `rust-toolchain.toml` pins the version (1.99.1) and
rustup fetches it. The seven bundled SoundFonts are in the repo.

```sh
./tools/fetch_soundfonts.sh            # optional: five more CC0 banks into soundfonts/extra
cargo build --release                  # target/release/kibble-481 and kibble-481-cli
./tools/bundle_macos.sh                # dist/Kibble-481.app with the icon and SoundFonts
./tools/bundle_windows.sh              # dist/Kibble-481-<version>-windows-x64.zip (cross-built)
```

The Windows build cross-compiles from macOS or Linux with MinGW-w64 (`brew install
mingw-w64`); on Windows itself, `cargo build --release` works as anywhere else.

## The composer

A seed writes a whole cue: progression, voicings that lead smoothly from chord to chord, a
lead melody in phrases (A A′ B A), bells, a pentatonic pluck, bass, a drone on the tonic
and fifth, drums and the city's rain (one of the eight recordings, sometimes joined by a
second as the piece recedes), in
sections with their sounds and hall. There are five kinds of section: **Intro** (an
opening: the drone, a thin pad and the theme's first notes heard alone), **Neon** (rain, the
pad, the drone and bells), **Swell** (the brass and bass come in, with a heartbeat),
**Night shift** (everything, with the lead melody, plucks and drums) and **Fade** (the
answer as it recedes). A new piece runs Neon, Swell, Night shift, Fade (in Noir, after an
Intro); an Intro can open a piece in any style. In the Score tab's
section list, drag a section by its ≡ grip to move it, drag one of the **+** buttons into
place to add a section there (or click it to add one at the end), and × to remove one; drag
the end of a section on the Sections strip above the roll to make it a bar or more longer
or shorter. A composed piece is written again to follow the arrangement. Set the key and
mode (minor, Dorian, Phrygian, harmonic minor), the seed (◀ ▶ to step through) and the
density in the Score tab; the same settings always write the same piece.

There are five **styles**, each playing the same kinds of section its own way, all
with the built-in synth, kits and bundled SoundFonts (nothing extra to download):

- **Nocturne** (66 bpm): the slow, rain-soaked ballad described above.
- **Pursuit** (104 bpm): driven. A sequenced bass runs in quavers or semiquavers with octave
  and fifth jumps, an arpeggio climbs over it, the brass stabs on pushed beats in the night
  shift, and the drums play four to the bar with a gated backbeat, quaver hats, a taiko fill
  every four bars and a riser into each night shift. Chords move every four seconds or so.
- **Blackout** (56 bpm): dark and low. The kit's analogue and sub drones under everything,
  low root-and-fifth brass swells, a sub bass, a horn far off with long silences between
  its phrases, sub booms on each new section, distant metal and grains, a slow deep pulse
  with taiko. Chords last about ten seconds; no plucks.
- **Momento** (72 bpm): romantic and swaying. Lush seventh and ninth chords that open
  towards the relative major, an electric piano rolling each chord (sometimes anticipating
  it), a soft swung groove (muted kick, half-time snare, shaker, tabla) with a walking
  bass, and in the night shift a sax-like theme of long notes and wide leaps that slides
  up into the top of each leap, the pad blooming an octave wider over a bed of horns and a
  kalimba three against four. Chimes as it fades.
- **Noir** (64 bpm): the quiet melancholy of walking alone at night, told in music alone:
  no rain, city sounds or other effects. Bittersweet minor ninths and sevenths, the sixth
  chord a major seventh with a sharp eleventh, and an ending that doesn't resolve; an
  electric piano laying each chord down; an upright bass walking slowly, stepping
  chromatically into each chord; brushes and a soft ride; a far-off flickering arpeggio;
  a faint tritone under the night shift for unease; and a lead (often a sax) that rests
  every other phrase. It opens on an Intro, and is mixed quieter (about -20 LUFS) with the
  lead back, to sit under a voice-over.

Choosing a style sets the tempo it suits (change it after if you like) and, unless **New
seeds keep my sounds and mix** is on, the sounds it is written for. 🎲 New piece and New
notes keep the style.

**🎲 New piece** draws a new arrangement too, at the piece's length: three to seven
sections in whole chords, sometimes after an Intro (always in Noir), opening with a Neon
(now and then a Swell), reaching at least
one Night shift, never the same kind twice running, and ending on a Fade. **New notes**
writes a new seed, key and mode to the sections as they stand.

It follows the tempo, metre and length. Change any of them and a composed piece is written
again to fit: the sections share a change of length in proportion, chords last about six seconds whatever the
tempo (every bar when slow, every few bars when fast), the bells fall in eighths at slow
tempos and in beats at fast ones, and the echo is timed to about 0.6 s. Once you edit the
notes it keeps them: tempo changes carry your notes with them, a new length cuts or extends
the piece with notes held to the end (rain, the last chord) held to the new end, and
**Write again** brings the composed version back. Fitting the track to a gallery of photos
sets the length too. New seeds choose their own instruments, unless **New seeds keep my
sounds and mix** is ticked. Every step can be undone.

## The instrument

**The synth** is a two-layer polysynth modelled on how the Yamaha CS-80 is built (the
instrument behind most early-80s neon-noir scores), not on any particular recording. Every
note plays two full voices together, each with:

- band-limited saw and pulse oscillators (pulse width swept by the LFO), a sine that can be
  frequency-modulated (bells, tine pianos), noise, and drive
- a resonant high-pass and low-pass, 12 dB/octave like the CS-80's, with its own envelope,
  key tracking and velocity ("initial touch")
- its own amplifier envelope, level, octave (32' to 2'), detune and pan

Around them: glide (poly or mono/legato), **sag** (the pitch droops as a released chord dies
away, the signature of that brass), a **ribbon** bend range up to two octaves, analogue
**drift**, ring modulation, stereo spread, and **touch** (aftertouch) that brightens, adds
vibrato or bends, as the CS-80's second touch did. Sixteen voices.

47 factory patches, all original: seven brass, six pads, three strings, six leads, six
basses, six bells, four keys, two plucks, five drones and two atmospheres. Among them are
the sounds the CS-80's design is known for: brass that blooms and sags, strings with a
delayed vibrato, resonant high- and low-pass "voices", a solo horn played with touch,
ring-modulated bells, a clavinet and an organ. The bells keep to whole-number FM and ring
ratios with no detune or drift, so they ring exactly in tune (a test measures it). Patches save and load as `.ddpatch` files.

**Alternatives for every part.** Each track has a role (brass, pad, strings, lead,
answer, bass, bells, pluck, keys, drone, city, drums), and the Sound tab offers that
role's alternatives from a dropdown under the track in the track list, synth patches and
SoundFont presets together (stopped, choosing one plays a short preview): brass
from Neon Brass to the FreePats synth brass and a GM brass section, bass from Sub Pulse
to a fretless, and so on, 75 in all. Each
alternative is level-matched by measurement (within 1.5 dB of its role's usual sound), and
swapping keeps the track's mix. The composer chooses from the same lists.

**Rain from real recordings.** Nine CC0 field recordings. Most are pitter-patter you can
count drops in (taps on a window-sill, drips, drops on an umbrella, soft rain on a roof,
drops in a back street), chosen by measuring how far the drops stand above the quiet
between them; light rain, and three denser washes (rain on a window, a wet street with
traffic, heavy city rain) are there when wanted. Each plays as recorded and keeps going for
as long as the note is held, moving between random stretches with slow crossfades so it
never audibly loops. The rain, thunder, machinery hum and neon buzz are chosen from the
dropdown under the City track in the track list. Credits in
[`assets/samples/README.md`](assets/samples/README.md).

**The city kit** (from Ambient MIDI Generator, synthesised): machinery
hum and a buzzing neon sign that loop for as long as you hold the key, thunder, distant
metal, sub booms, braams, risers, reversed cymbals, glass grains, drones and timpani; and a
drum kit of dull and heartbeat kicks, a gated snare far off, taiko, tabla and a ticking
clock.

**SoundFonts** for everything sampled. Seven are bundled, all licensed for commercial music
and redistribution (see [`soundfonts/README.md`](soundfonts/README.md)): GeneralUser GS (piano,
koto, shakuhachi, shamisen, choir, taiko…), Ambient Electronic, and five CC0 FreePats banks
(analogue and FM synth brass, a sweep pad, a bell pad, a sampled tenor saxophone).
`tools/fetch_soundfonts.sh` adds five more CC0 banks (FM piano, tubular bells, timpani,
synth choir, upright piano), pinned by checksum; `--musescore` adds MuseScore_General (MIT).
Load any other `.sf2` from the Sound tab.

## Playing and recording

- A **MIDI keyboard** plays the selected track as soon as it is plugged in: notes, velocity,
  pitch bend, modulation wheel, sustain pedal and aftertouch.
- The **computer keyboard** plays too: A W S E D F T G Y H U J K O L P ; (Z X change
  octave, C V how hard).
- The **performance strip** under the roll has a clickable keyboard (lower on a key plays
  harder), a **ribbon** that bends and springs back, a **mod wheel**, a **touch** strip
  (aftertouch) and a **sustain pedal**.
- **Record** (⇧R) onto the selected track: a bar of count-in, an optional click, and it
  keeps going past the end. Takes add to the track or replace what they play over, and can
  be quantised. Controller moves are recorded with the notes. Notes are stamped with the
  moment you heard them, allowing for the audio buffer.
- **Loop** a range of bars (⇧-drag on the ruler) to work on a phrase.

## Editing

- **Piano roll**: click to add a note and drag out its length; drag to move (⌥ copies);
  drag the end to resize; double-click or right-click to delete; ⇧-drag to select. ↑ ↓
  transpose (⇧ by octaves), ← → nudge, Q quantise (⇧Q ends too), ⌘D duplicate, ⌘C ⌘V
  copy and paste at the playhead. The other tracks show faintly behind.
- **Volume** (in the lane's menu): the selected track's volume bar by bar, on top of its
  fader (100 is the fader's level). Drag to set bars, sweeping across several to set each;
  the volume ramps between the bars you set and holds after the last, so bars you haven't
  set, new sections included, follow their neighbours. Right-click a bar to let it follow
  again; **Clear** sets them all back. The levels move with their sections when you
  rearrange them, outlast a new seed or style, and reach the MIDI file as its volume
  controller (CC7, a step a beat through ramps). In the mixer, a track with bar volumes
  shows where they have it as the music plays: an orange marker on its fader and a ▸
  readout, the fader itself staying where you set it.
- **Lane** under the roll: velocities, or draw the mod wheel, touch, pitch bend, brightness
  (CC74, the synth's filter) or expression.
- **Overview** of every track above the roll; **sections** on the strip above the ruler
  (drag a section's end to resize it, a bar at a time), ordered in the Score tab.
- **Tracks**: click anywhere on a track to select it; double-click its name (or Rename in
  its right-click menu) to rename it.
- Undo and redo (⌘Z, ⇧⌘Z) for everything.

## Mixing

Each track has a strip: fader, pan, width, reverb and echo sends, a **low cut** and a
**high cut**, the ensemble chorus, mute and solo. The cuts (12 dB an octave, 20 Hz–20 kHz,
double-click for off) filter the track's sound after its instrument, so they work the same
on the synth, the kits and SoundFonts: a SoundFont has no high-pass of its own and only
some presets answer the brightness controller, so the filter isn't left to it. They are
saved with the project but, having no standard MIDI message, aren't in an exported MIDI
file for other programs. Shared effects (from Ambient MIDI Generator): eight reverb spaces including
**Noir** (a vast, dark 1980s digital hall), Shimmer and Black Hole; a tempo-synced tape
echo; a string-machine ensemble; A = 440/432/415 Hz.

## Getting it out

Saves and exports suggest a name made of the piece's title, its seed and the time in UTC,
such as `Last Monorail - seed 475994 - 2026-11-11T143233Z.wav`, so versions of one piece
sort together and never overwrite each other (stems and the Resolve package's files too).

- **MIDI file**: type 1, a track per track, with tempo, metre, section markers, programs,
  controllers and the mix. Synth tracks carry the nearest General MIDI program; the kit and
  city FX land on the Ambient Electronic SoundFont's Trailer Kit (48) and Trailer FX (119),
  which share their keys, so a DAW with that font plays them right. The project is embedded
  too, so Kibble-481 reopens its own MIDI files exactly.
- **Soundtrack (WAV)**: 24-bit, 48 kHz (Resolve's native rate), the mix exactly as heard,
  with its reverb and echo let ring out (up to a minute).
- **Stems**: a WAV per track, each with its reverb and echo, for Resolve's Fairlight page.
- **Gallery for DaVinci Resolve**: choose a photo folder in the Gallery tab and export. You
  get the soundtrack, an `.fcpxml` timeline with every photo at its change and a cross
  dissolve between them, the changes as `.edl` markers, and a cue list. In Resolve: **File ›
  Import › Timeline**, pick the `.fcpxml`, let it import the media. Images change on bar
  lines and move onto the music's accents (sections, phrases, parts coming in), 4 to 16 s
  each; or size the track to the number of photos.
- **Import** any MIDI file as a project, or add its tracks to this one. Files from Ambient
  MIDI Generator (Neon Noir, Megacity Rain, Neon Glide) get their score kit and city effects
  back.

## Command line

```sh
kibble-481-cli compose piece.kibble [--style momento] [--seed 42] [--bpm 72] [--bars 32] [--key 2]
kibble-481-cli demo rain.kibble
kibble-481-cli render rain.kibble rain.wav [--stem Brass]
kibble-481-cli midi rain.kibble rain.mid
kibble-481-cli import megacity.mid megacity.kibble
kibble-481-cli resolve rain.kibble out/ --photos ~/Pictures/tokyo --fps 25 --hold 8 [--fit-track] [--stems]
kibble-481-cli patches | fonts
```

## Layout

| Path | What |
|---|---|
| `crates/dd-dsp` | the polysynth (`poly.rs`, `patch.rs`), the city kit and its player, reverb, echo, ensemble. No UI. |
| `crates/dd-seq` | project model, timeline, the engine and mixer (live and offline), SoundFont instruments, recording and quantise, MIDI and WAV |
| `crates/dd-gallery` | slideshow timing, Resolve FCPXML/EDL, the `.kibble` document, the Resolve package export |
| `apps/dd-desktop` | the egui app (`kibble-481`); `about.rs` (version, details, licences), `update.rs` (the update check) |
| `apps/dd-cli` | `kibble-481-cli` |

`./tools/bundle_macos.sh` builds `dist/Kibble-481.app` with the icon, both binaries and the
SoundFonts. After changing dependencies, `./tools/gen_licenses.py` rewrites the About
window's list of third-party crates and their licences (a test checks it matches
`Cargo.lock`).

## About

The ⓘ button opens About: the version and the git commit it was built from, what the app
does, where this copy keeps its settings and SoundFonts, the licences of the app and of
everything bundled with it (SoundFonts, rain recordings, typefaces, interface design) with
their full texts, and every third-party crate compiled in, with its licence. **Check for
Updates…** asks GitHub's public releases API for a newer version (nothing about you is
sent); with **Check at startup** on (the default) it asks when the app opens, and a ⬆
button appears in the top bar when there is one.

## Built from the other projects

Much of Kibble-481 is lifted from [Ambient MIDI Generator](https://github.com/catskinroyale/ambient-midi-generator)
(MIT) and, through it, [BW Darkroom](https://github.com/catskinroyale/bnw-darkroom) and
LightCraft: the reverb, echo and ensemble; the city kit and the Ambient Electronic font; the
mixer's channel strips, fader law and meters; the slideshow planner and Resolve exporter;
the theme, widgets and screenshot mode. Each copied file says where it came from and what
changed. The workspace follows Film Darkroom's `crates/` + `apps/` layout so the repos can
be combined later; see `docs/decisions.md` (D2).

## Tests

```sh
cargo test --release --workspace
cargo clippy --release --all-targets -- -D warnings
KIBBLE_SCREENSHOT=window.bmp cargo run --release -p dd-desktop   # picture of the window, then quit
```

The tests cover the composer (40 seeds at five tempos, metres and lengths: it fills the
length exactly, keeps the rain falling throughout and uses only bundled sounds; every style
likewise; Noir without any rain or city sound; intros; chords about six seconds apart;
edits detected), the track cuts (on a SoundFont too), the recorded rain (steady for far longer than its
recording, heavier when harder, dying away when released), every factory patch (it sounds, stays in level and dies away), glide, sag,
sustain and voice stealing; the kit; event order and chasing; recording and quantise; MIDI
round trips and import; deterministic, correctly levelled renders through the bundled
SoundFonts; loop and count-in; and the gallery landing on the score's sections.

## Licence

The code is under the [MIT License](LICENSE). The SoundFonts keep their own licences
(`soundfonts/`). The interface theme and widgets are adapted from BW Darkroom and LightCraft
(MIT, `assets/LICENSE-LightCraft-MIT`). The rain recordings are CC0 (`assets/samples/`). The typefaces (Michroma, Share Tech Mono, VT323,
Cutive Mono, Sorts Mill Goudy, Jost and Inter) are under the SIL Open Font License 1.1
(`assets/fonts/OFL-*.txt`).

Kibble-481 is an independent project. Its sounds are original and written in the spirit of
early-1980s polysynth film scores; no film, composer or instrument maker is affiliated with it.
