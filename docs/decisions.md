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

Added for Homeward (D23), bundled, CC0: FreePats Concert Harp 2020-07-02 (Versilian
VCSL samples; archive `ConcertHarp-SF2-20200702.tar.xz`, SHA-256
`a328faa1996369154f20f960dde2e255b96e823574d5cf80eb9e7f00aed6a150`, 19 MB unpacked) and
Synth Strings 1 2020-05-28 (`SynthStrings1-SF2-20200528.7z`,
`1fed457941efbd6e1deca6937d1859db7920946709732265426785e9e11bcf55`, 6 MB). FreePats has
no CC0 cello, viola, double bass, flute, clarinet or horn; GeneralUser GS plays those.

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

## D15 — No adding tracks, for now (2026-11-11; reversed by D22)

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

## D19 — Overcity (2026-11-13)

A sixth style written to the user's brief: a slow, vast synthesizer cue of a rain-soaked,
neon-lit city seen from above at night, cold, melancholic and faintly sacred, as an
evocative homage to a classic early-80s analogue film score and not a copy of it. Per D3
the score, film and composer the brief names appear nowhere in the code, patches, titles or
docs; the name, "Overcity", is our own. No melody is copied: the theme is generated each
time from the brief's shape (a rising leap of a fifth or sixth, a stepwise fall in the
minor scale to a held note, long silences, each statement higher), not from any tune.

The brief's form maps onto the five parts: Intro (near-silence, a low drone, horns rising
on an open fifth), Neon (the theme), Swell (development: heartbeat, rippling glass, a high
choir), Night shift (the great swell) and Fade (dissolving to the drone and a last bell).
The horn swells are written as expression (CC11) and brightness (CC74) curves, which the
built-in synth and the SoundFonts both follow. Unlike Noir, the brief asks for wind, rumble,
thunder and a wash of rain, so Overcity keeps the City track, quieter. Its pieces measure
about -15 to -18 LUFS with a loudness range of 9 to 12 LU (large dynamics, as asked). The
other styles are written exactly as before (each style's recipes recomposed and compared
note for note with 0.4.0).

## D20 — Gallery timing, calculator and transitions (2026-11-16)

The gallery used to plan either the images a track holds or a track for a number of
images, and in the second case it sized the plan to a length the score might not have (it
set the length only sometimes, and never shrank an edited piece), so the export and the
music could disagree. Now the plan always follows the score as it stands, capped at the
number of photos (the time the rest would have had goes to the outro), and the reverse sum
lives in a **calculator** that changes nothing until asked: the images the score holds and
the time over; the music a number of photos needs (intro + photos × time + outro, rounded
up to whole bars); and how many bars to trim or add, with a button that sets the length
(a composed piece is written again; an edited one is trimmed or extended).

**Exact seconds** makes every image the same length, off the bar grid if need be, for an
even pace; the bar-line timing (and following the music's accents) stays as the other
choice. **Transitions**: crossfade (as before), fade to black, or cut. Fade to black is
written into the FCPXML as each photo ending one fade early, a black gap in its place,
and cross dissolves out to the gap and in from it, so each change still centres on its
beat; this relies on Resolve treating a dissolve against a gap as a fade to black, which
is how it handles an edit next to empty track. It is not yet checked in Resolve itself.

## D21 — Derelict (2026-11-16)

A seventh style written to the user's brief: an instrumental deep-space drone score,
1970s analogue sci-fi horror meeting industrial noir. Dread, isolation, claustrophobia
and awe; free tempo or very slow; very little melody; long silences and sudden swells; no
vocals, no drum beat, nothing of modern EDM. The tone references are for feel only: no
film, score or composer is named in the code, sounds or docs (D3), and the name,
"Derelict", is an ordinary word. No melody is copied; the few fragments there are come
from the brief's shape (a horn call turning on a semitone, a reed phrase that never
resolves, a ghost's blues phrase), not from any tune.

The brief's arc maps onto the five parts: Intro (the quiet void), Neon (creeping
tension), Swell (its tightening), Night shift (the brief orchestral eruption, about a
sixth of the piece, never longer than the collapse) and Fade (the collapse back into the
drone). The sounds it asks for are met from what is bundled, with no new downloads:
GeneralUser GS's tremolo strings, contrabass, cor anglais, oboe, muted trumpet, French
horns, celesta and harp (the first five added to the palette), the synth's drones and
pads, and new synthesised sounds in the city kit for the ship (hull groan, pipe clang,
steam hiss, metal scrape, ticking, tape hiss); `kit.rs`'s provenance note says so. The
pitched ship sounds sit on keys 0–23, where their octaves line up with pitch classes; the
others on 116–119, past the rain. "Free tempo" is written as a slow 48 bpm with most
entries placed a little off the beat and nothing that marks one; the tape is the
master's drift, detune and echo wow.

To keep every other style's recipes as they were, the new palette sounds are left out of
their choosing (`DERELICT_SOUNDS`), and the shared swell curve was only moved into a
function: all six were recomposed for 40 seeds at five tempos and lengths, default and
random arrangements, and compared with 0.6.0, identical. Derelict's pieces measure about
-18 to -21 LUFS with a loudness range of 12 to 16 LU and peaks around -0.5 dBFS: quiet and
wide, like Noir's level, for a cue with a void in it. The City track stays (the ship's
sounds live there) but has no rain or thunder; there is no Drums track.

The swells breathe from a floor rather than from silence. At first the pad (and the
double bass) climbed from nothing to nearly full on every chord, 25 to 40 dB over six
seconds, every fifteen: heard as one sound coming back louder each time, like a reverb
running backwards. Now they swell a few dB from a steady level, the pad's sudden swell
comes once in each tightening, and Derelict's expression curves are written finely
enough (an eighth of a beat apart) to climb smoothly instead of in stairs. The reverb was
shortened (size 0.8, about a 30 s tail in the black hole) so the tail doesn't pile up.

After listening, the user found koto and shakuhachi in it and the bed too much like
noisy tape hiss. So Derelict now chooses only from its own sounds (no weight at all for
the rest), more of them 1970s orchestral and synth (trombone, bassoon, baritone sax,
pizzicato, tubular and ring-modulated bells, GeneralUser's soundtrack, metal and
space-voice pads; the bassoon and baritone written an octave and a half or an octave
lower); the mains hum is gone and the tape hiss far down, and the bed is two new
synthesised loops, an engine throbbing at a stroke and a half a second with a knock on
each stroke, and a life-support pump that breathes and clanks, both failing in the
collapse. And choosing a style now always brings its own sounds, Keep sounds or not:
the koto had come from the previous style's piece through Keep sounds.

The brass plays without the ensemble chorus or the echo: on sampled horns and
trombones the chorus's wavering copies (and an echo a quaver behind) sounded like a
second player doubling, slightly out of tune. The pad goes without it too, and the
master ensemble is off in Derelict. Its swells (written finely) now move the brightness
(CC74) step by step with the expression rather than in three jumps: a filter thrown two
octaves open at a swell's peak was heard as a stab. A swell that peaks at its end holds
there, and the eruption takes up the brass and strings where the build left them,
instead of dropping and kicking back. The other styles' three-point curves are unchanged.

## D22 — Sections by name, moving with their notes; adding tracks again (2026-11-17)

Sections show as their part and a number within it (Intro 1, Intro 2) unless given a name
of their own (`Block::own_name`, `block_labels`); a numbered name reads back as its part,
so MIDI markers round-trip. A section's menu (its name in the Score tab, or right-click on
the Sections strip) renames it, changes its part, writes new notes for it or for one track
in it, and duplicates it.

Rearranging an edited piece used to move the bar levels but not the notes: a section made
longer showed empty bars, and the sections after it slid out from under their notes. Now
(`Project::rearrange_with_notes`) the notes and controllers go with their sections, and in
a composed piece a section that is new, another part or another length is written again on
its own (`compose::rewrite_sections`), with the seed it was last written with
(`Section::seed`, set by New notes for a section), the rest kept as they are. An untouched
composed piece is still written again whole. A piece that wasn't composed fills a longer
section by playing its notes again. Notes held right through (the rain, held drones) stay
held through copies, deletions and rewritten sections instead of being cut.

**+ Add** is back (reversing D15): a track of any role gets the sound and notes the piece's
style writes for it (`compose::add_track`); a second track of a role the piece has is
written with a seed of its own (from its name), so it plays something else; a role the
style never writes comes empty; Noir offers no City (D18). Added tracks are marked
(`Track::added`) and written again for their roles by a new piece; adding one to an
untouched piece leaves it untouched.

## D23 — Homeward (2026-11-17)

An eighth style written to the user's brief: an instrumental cinematic score, slow, warm,
melodic and serious, built around one memorable theme that returns in varied forms;
66–72 bpm in a soft, felt pulse; reflective, dignified, quietly hopeful; nothing sudden.
It lives in `homeward.rs`, called from `compose_plan` like Derelict's writer.

The seed writes an eight-bar theme in the piece's mode: rhythms from gentle patterns per
metre (the second phrase repeats the first's, to be remembered), a first phrase rising
and resting on the dominant (or the second), a second lifting towards the relative major,
peaking and falling home (to the octave when the way down starts high, the leading tone
rising to it), never a leap past a fifth. A chord a bar holds each bar's main note, with
a major dominant at the cadences, and a reharmonisation (other chords holding the same
notes, with sevenths, the cadences kept) for the middle statement.

Each section states the theme in whole statements from its start, and what is left over
its last bars, so every section ends on its cadence (an Intro, or a section opening the
piece, too short for a statement opens the theme instead). Intro: the piano alone. Neon:
the cellos (or violas), with soft piano chords, harp, bowed bass, sub, a soft string bed.
Swell: the lead (flute, clarinet or horn) varied with passing notes, the cellos' contrary
countermelody, the reharmonised chords. Night shift: a build (a beat-by-beat pulse in
cellos, bass and piano, strings adding voices, the pad entering, expression rising, the
dominant just before the peak) to a peak of up to eight bars, the theme in the strings in
octaves over the cellos. Fade: the theme on the piano again, then a held tonic chord with
a ninth above, everything gliding down. The usual arrangement is Intro, Neon, Swell,
Night shift, Fade in eight-bar units (four or two when short); new pieces always open
with an Intro.

Sounds: the bundled GM cello, viola, strings, contrabass, flute, clarinet, horn, grand and
electric piano, plus two new CC0 FreePats banks (D4): Concert Harp and Synth Strings (the
analogue pad). The new alternatives are left out of the other styles' choosing
(`HOMEWARD_SOUNDS`), so every earlier recipe sounds as before (checked: identical
fingerprints, titles and sounds across 168 pieces). Mix: a natural hall, no ensemble,
the piano high-cut like felt, the sub low-passed and quiet; about -21 LUFS, LRA 11–12 LU,
peaks under -2 dBFS.

## Open

- Bake the factory patches into a `Kibble-481-Synths.sf2` (as midigen's `make_soundfont.rs` does)
  so exported MIDI sounds like the app in any DAW.
- Tempo changes within a piece; per-note pitch curves (MPE); automation of synth parameters.
- Ken Burns moves on gallery stills in the FCPXML.
