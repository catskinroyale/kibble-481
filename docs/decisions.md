# Decisions

Product and scope decisions, newest last. Reference them as D1, D2…

## D1 — An instrument you play, not a generator (2026-11-10)

Ambient MIDI Generator already writes neon-noir pieces (Neon Noir, Megacity Rain, Neon
Glide). DDream is its complement: a hands-on synth, sequencer and recorder for writing a
score yourself. Generated pieces come in through MIDI import (D6) rather than by porting
the 5,000-line generator.

## D2 — Reuse by copying, laid out for a later merge (2026-11-10)

Code from Ambient MIDI Generator is copied with a provenance line rather than shared as a
dependency, since neither repo publishes crates. Copied as is: `reverb.rs`, `kit.rs`,
`resolve.rs` (one change: the soundtrack's audio rate), the effect types in `dd-dsp/fx.rs`,
`theme.rs`/`widgets.rs` (palette changed), `snapshot.rs`. Adapted: the mixer (strips per
track), the slideshow's `Accents` (read from a project), the player (now the engine).

The workspace uses Film Darkroom's `crates/` + `apps/` layout and `dd-` prefixes. When the
repos merge, `dd-dsp` (reverb, kit, effects) should become the shared audio crate that the
generator also depends on, `dd-gallery` its slideshow crate, and `fx.rs` the single home of
the effect types. Keep the copies in step until then.

## D3 — The synth is CS-80-shaped, and its sounds are original (2026-11-10)

Two complete layers per voice, 12 dB/octave high- and low-pass per layer, initial and
second touch, ribbon, sag. These are the architecture's ideas, not anyone's patches. Patch
names avoid film titles, characters, composers and trademarks; the README and About say so.

## D4 — SoundFonts: bundled, optional, rejected (2026-11-10)

Bundled (all redistributable, commercial use of music allowed): GeneralUser GS v2.0.3, Ambient
Electronic (MIT, ours), and FreePats CC0 banks: SynthBrass1 (archive SHA-256
`ce2876f93f7c3aded4a163dfa787cb708025023f80aeb01d178799a09f750e8e`), SynthBrass2
(`d094cd3067efd057fdbba0cc9ca831496fa9331580544ae943cb1dca9da476b1`), SweepPad
(`2a2eb595b47e780fc633391f756e972e87512d61adcb9564c3c27b9ffca3fb63`), NewAge
(`016c4b1c3053cdf1067dbe4a0c3a4a8ceb1ae5de31433edde27785f35c3387f4`), TenorSaxophone
(`f7640cf2039662b15b4521bb5615e1ddc426731559b4c31079cb5e9f50e16fa0`). About 58 MB for the
five, committed like the midigen fonts.

Optional, by `tools/fetch_soundfonts.sh` (hashes in the script): FreePats FM-Piano1,
TubularBells, Timpani, SynthPadChoir, UprightPianoKW (CC0); MuseScore_General (MIT, 216 MB;
size-checked only, as it wasn't downloaded while this was written; pin its hash when it is).

Rejected: Timbres of Heaven (no redistribution, non-commercial), Arachno and SGM (unclear
licence and sample origin), Sonatina (SFZ, CC Sampling Plus), all SF3 (rustysynth reads SF2
only), Salamander SF2 (296 MB).

## D5 — One engine for listening and export (2026-11-10)

The real-time engine and the WAV export are the same code, so the export is what you heard.
Exports run at 48 kHz/24-bit, Resolve's native format, whatever the device rate. Rendering is
deterministic (tested). An export rings on after the music until its tail has been below -84 dBFS
for half a second (at most a minute), so long halls aren't cut off.

## D6 — MIDI is the interchange format (2026-11-10)

Export carries GM programs so files play anywhere, the kit on the Ambient Electronic font's
Trailer Kit/FX slots (which share the kit's keys), and the full project as embedded JSON
(`ddream-project:`) for an exact round trip. Import recognises Ambient MIDI Generator files.

## D7 — Recording timestamps what was heard (2026-11-10)

Live input goes from the MIDI thread straight to the audio thread. Takes are stamped by
extrapolating the engine's last block position with the wall clock, less one output buffer,
so notes land where they were heard rather than where the next block will be. The timeline is
not swapped during a take (a swap releases held notes).

## D8 — A composer after all, and pieces that follow their shape (2026-11-10)

Asked for: a randomising regenerate button, and pieces that respond to tempo and length.
So D1 is relaxed: `dd-seq/compose.rs` writes a cue from a `Recipe` (seed, key, mode,
density) at the project's tempo, metre and length. It is small and DDream's own rather
than a port of midigen's generator, but it uses midigen's seeded streams (`rng.rs`).
A piece is "untouched" while its notes' fingerprint matches the one taken when it was
composed; untouched pieces are rewritten when the tempo, metre or length change, and
edited ones are kept (tick-based notes follow tempo; `resize` handles length). Changes
wait for the mouse to be released, and every rewrite is one undo step.

## D9 — Rain is real recordings, played as recorded (2026-11-10)

The synthesised rain loop sounded like noise, and a first granular version (short,
re-pitched grains) didn't convince either. Rain is now eight CC0 field recordings
(Freesound and Wikimedia Commons; `assets/samples/README.md`) played without processing,
moving between random 6–14 s stretches with 1.5 s equal-power crossfades. Ogg Vorbis keeps
them to 5.3 MB in the binary (decoded with `lewton`, MIT/Apache); they're decoded in the
background when a City FX track is made. Stereo files whose sides barely differ are folded
to mono and given independent stretches per side. The Ambient Electronic SoundFont still
has the synthesised rain, so exported MIDI on that font plays that one.

## D10 — Roles and alternatives (2026-11-10)

Each part has a role, and `dd-seq/palette.rs` lists the sounds that can play it, synth and
SoundFont alike, with trims measured by the loudest 400 ms (fair to hits and held sounds)
so that every alternative sits within 3 dB of its role's usual sound (a test holds this).
Swapping moves the fader by the trim difference, so the mix keeps its balance. The composer
picks from the same lists (never the stab for held chords). Fetched-only fonts are offered
but greyed out until present.

## D11 — Bells are tuned, echoes barely wow (2026-11-10)

Bells sounded detuned: non-integer FM ratios (3.5, 1.41, 7.07) put partials off the
harmonic series, and layer detune, drift and the echo's tape wow (0.35) bent them further.
Bell patches now use whole-number ratios and no detune, drift or vibrato; the house echo's
wow is 0.08. `poly::tests::tuned_percussion_is_on_pitch` checks every bell and key patch is
strongly periodic within 4 cents of A440, and that an inharmonic bell fails.

## D12 — Sections are an arrangement of four parts (2026-11-10)

Asked for: build a score from the sections Neon, Swell, Night shift and Fade in any order,
repeated, ordered in the side panel, with lengths set by dragging in the roll. So a
section has a `Part`, and the sections tile the piece (each lasts until the next). The
composer writes each part its own way wherever it falls (`compose_plan`): Neon is rain,
pad, drone and bells; Swell adds brass, bass and a heartbeat; Night shift adds the lead,
plucks and drums, with thunder and a braam into it; Fade drops brass and bass for the
answer. The arrangement is part of a piece's shape (D8): it's left out of the edit
fingerprint, and changing it rewrites an untouched piece; an edited one keeps its notes.
Changing the length shares the change among the sections in proportion. Older pieces'
section names (Rain, Brass, Lone lead, Recede and the other sets) read as the four parts;
MIDI markers with other names stay plain markers, and a piece with any of those is
written in the default arrangement.

## D13 — Kibble-481, and a look from the mood board (2026-11-10)

The app is renamed Kibble-481 (at first user-facing only; D14 renames the rest). The look
follows the mood board: each palette is one ground, one panel tone, two supporting colours
and a lead accent, and every colour in the interface is derived from them (`theme.rs`);
tracks keep their stored colour and show it in the palette's tones. The board's palette
names refer to a film and a novel, so under D3 they are renamed for what they look like:
Sodium Amber, Rain on Neon, Rust and Verdigris, Paperback, Phosphor. A test holds text,
labels and text on the accent to WCAG contrast in all five. Typefaces as the board
suggests, all SIL OFL and bundled: Michroma, Share Tech Mono, VT323, Jost, Sorts Mill Goudy,
Cutive Mono (OCR-A isn't bundled: its free builds' licences vary).

## D14 — The old name goes from everything but the crates (2026-11-10)

DDream is gone from the binaries (`kibble-481`, `kibble-481-cli`), the project extension
(`.kibble`), the MIDI file's embedded project (`kibble-481-project:`), the settings storage
(`Kibble-481`), the MIDI input's client name and the environment variables (`KIBBLE_*`).
Nothing saved is lost: `.ddream` projects open, MIDI files with the old marker open
exactly, `$DDREAM_SOUNDFONTS` is still read, and the settings stored under DDream are
copied across the first time the app runs. The crates keep their short `dd-` names
(internal, and renaming them would churn every `use`).

## D15 — No adding tracks, for now (2026-11-11)

The track list's **+ Add** menu is hidden: a piece comes with its tracks (the composer's
roles, the city and the drums), and the sounds are changed from each track's dropdown. The
menu's code stays behind `ADD_TRACKS` in `panels.rs`, to bring back when it is wanted.
Duplicate (the track's right-click menu) and File › Add tracks from a MIDI file still add
tracks.

## D16 — Volume bar by bar (2026-11-11)

Each track has a list of bar levels (`Track::levels`: a bar and a value, 100 = the fader),
kept apart from its notes and controllers so writing a piece again doesn't touch them and
setting them doesn't count as editing the notes. Between two set bars the volume ramps; it
holds before the first and after the last, so a new section's bars follow their
neighbours without being set. The engine scales each strip by the level at the position
(the timeline carries them), so live, offline and stems agree. Rearranging the sections
moves each section's levels with it (`Project::rearrange`); a shortened section keeps its
last level at its new end. The MIDI file carries them as CC7, the fader times the level, a
step a beat through ramps. Rejected: writing CC7 or CC11 events into the track's
controllers (the composer rewrites those, and instruments treat them differently).

## D17 — Composer styles (2026-11-11)

The composer has styles that play the same four parts their own way, so arrangements,
sections and the Score tab work alike for all of them: Nocturne (as before, note for note
for every recipe), Pursuit (driven), Blackout (dark, drone-led) and Momento (romantic,
after the feel of Ambient MIDI Generator's swaying neon cue, not any one piece). New
styles use only the bundled synth patches, kits and SoundFonts, to keep the download from
growing. Choosing a style sets its tempo; New piece keeps the style. Momento adds a Keys
track (electric piano), only to its own pieces, so the other styles' tracks and colours
are unchanged. Its name was the user's choice: an ordinary Italian and Spanish word,
noted here because it is one letter from a film title (D3).

## D18 — Noir, intros, track cuts and About (2026-11-12)

Noir was written to the user's brief: the melancholy of walking alone in the rain at
night, evoked by harmony, melody, texture, space and pacing alone, with no rain recordings,
weather or other literal sound effects, for YouTube video under voice-over. So a Noir piece
has no City track at all (no rain, hum, neon buzz or thunder), and a composer test checks
it. The visual references (neon reflections, alleys, taxis, crossings, lanterns,
flickering tubes, a figure under an umbrella) shape the music only: the flicker is an
arpeggio missing notes, the unease a faint tritone under the night shift. The tone
references (late-70s/80s analogue sci-fi scores, synthwave and darkwave, ambient jazz noir)
are for feel only; no melody or piece is copied, and none is named in the code (D3). To sit
under a voice it is mixed at about -20 LUFS (the other styles -13 to -18), the lead pulled
back and resting every other phrase, the pad's and arpeggio's tops cut. Its length is the
project's, as for every style. Its brushes are GeneralUser GS's Brush kit (bank 128,
program 40), already bundled.

The Intro is a part like the others, so the Score tab, arrangements and stretching handle
it unchanged. It draws from its own random streams ("intro", "intro-form"), so a piece with
no Intro is written exactly as before (checked against the 72-piece Nocturne fingerprint).

The track cuts are a filter on the track's output in the engine, not a message to the
instrument: SoundFonts have no high-pass, and their low-pass is per voice and moved by
CC74 only in some presets, so filtering the output is the one way that works the same for
every source. They aren't written to MIDI files (no standard message); the embedded project
keeps them.

About now follows Ambient MIDI Generator's (and Film Darkroom's): build commit, licence
texts, the full crate list (`tools/gen_licenses.py`) and an update check. The check contacts
only GitHub's public releases API with a generic user agent and is on at startup by
default, as in the other apps; it can be turned off in About.

## Open

- Bake the factory patches into a `Kibble-481-Synths.sf2` (as midigen's `make_soundfont.rs` does)
  so exported MIDI sounds like the app in any DAW.
- Tempo changes within a piece; per-note pitch curves (MPE); automation of synth parameters.
- Ken Burns moves on gallery stills in the FCPXML.
