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

## D24 — Return (2026-11-18)

A ninth style from the user's brief "Homeward (Dawn Return)": Homeward's form and theme
with the instrumentation fixed: a soft electric piano for the Intro, violas for the
Neon's theme, a clarinet for the Swell answered by cellos, full strings at the peak, the
solo piano home; E minor at 66 bpm. Asked for as a style of its own after a first
attempt changed Homeward's harmony and split one piece's tracks by hand; that change
was reversed, so Homeward writes exactly as in 0.8.0 (identical fingerprints, sounds,
levels and titles across 288 pieces of the eight earlier styles).

It is written by `homeward.rs` with two extra tracks (`Tracks::ep`, `Tracks::counter`):
the electric piano takes the Intro, and the cellos, apart from the theme's violas, take
the countermelody, the build's pulse, the peak's low octave and the Fade's held notes
(the violas join the peak in octaves). Its theme is Homeward's for the same seed, but the
peak bar prefers the III chord, so the lift reaches the relative major where the note
allows. Its plan shares 8:8:8:16:12 bars in proportion (`share_bars`), so the build and
peak keep their sixteen and the Fade holds its last chord; under 20 bars it takes
Homeward's. Its sounds are named, not drawn (its own stream, `return-strings`), and it
always uses the concert hall. `Style::key` gives it E minor, set when it's chosen, on each
new piece in it, and by the command line unless `--key` says otherwise; the other styles
have none and keep their keys.

Played with grace (2026-11-19, after "the piano and viola are a little plonky"): in Return
only, the theme on the electric piano, the violas and the piano is phrased: velocity
follows the theme's arc through each bar, short notes lighter, a gentle downbeat, the
cadences sighing; the peak's first note and a cadence's last arrive a sixteenth of a
beat late; notes are legato. The violas are bowed: each long note swells (CC11) and its
vibrato (CC1) blooms as it's held. The pianos' block chords become the bass on the
downbeat and the chord broken gently up and back in quavers, legato-pedalled (CC64
lifted at each chord, down just after; up for the build's pulse), and the last chord
rolled slowly. Homeward writes exactly as before (identical fingerprints and titles
for every other style across 1,080 pieces); Return stays about -20 LUFS with peaks under
-2.5 dBFS, its range a little wider.

## D25 — Chamber (2026-11-18)

A tenth style from the brief "a chamber orchestra score style with a sci-fi dark noir
twist". A small orchestra of named sounds: a solo violin (a new Strings alternative,
GM 40, trimmed -4.6 dB; left out of the other styles' choosing, `CHAMBER_SOUNDS`),
violas, cellos, contrabass, pizzicato, tremolo strings, a clarinet (or cor anglais), a
bassoon, a horn, a harp, a celesta and a grand piano. The twist sits under them, not in
place of them: a glass drone (tonic and fifth, breathing with each section), and a dry
sub that beats like a heart (two thumps a bar) and at the climax like an engine (quavers).

Harmony is its own (`chamber.rs`), two bars a chord, eight-bar progressions that always
turn on V7: i–♭VI–iv–V7, i–♭VImaj7–♭II–V7 (Neapolitan, preferred in Phrygian),
i–♭vi–i–V7 (the uncanny chromatic mediant), imaj7–iv–♭VI–V7, i–III–iv–V7 and
i–♭VII–♭VI–V7. Melody notes come from a chord scale: the mode's, with notes a
semitone from the chord's giving way to them. The theme is a motif (a leap, then steps
against it), its answer, the motif sequenced on the second chord, raised to a climax
(sometimes inverted), a fall on the dominant and the leading tone (or the dominant's
fifth) held; no bar leaps more than a sixth from the last.

Sections: Intro the void (heartbeat, motif on the celesta, a far violin harmonic, low
cellos); Neon the clarinet's theme over a ticking pizzicato ostinato; Swell the solo
violin, violas in contrary motion, a bassoon line (a semitone into each new root), horn
swells, celesta echoes, low piano octaves; Night shift a build (tremolo layering, the
engine, low piano clusters with a minor ninth, the horn calling the motif) to the theme
in violin, violas and cellos in octaves; Fade the clarinet then the cellos, thinning.
The last section ends on a held i(maj7, 9) chord, the clarinet home on the tonic and the
motif once more on the celesta. Default plan 4:8:8:16:10 shared to the length.

Mix: a hall (or the Noir hall), no ensemble, orchestral seating (violin left, low
strings right); about -19 LUFS, LRA about 15 LU, peaks around -2 dBFS. Checked: the
other nine styles write identical fingerprints, titles, sounds and levels across 1,440
pieces.

## D26 — Orbit (2026-11-19)

An eleventh style from a brief for a minimalist score: eerie, hypnotic and serious, a
steady pulse in 15/8 felt as seven and eight, A minor with a relative-major ambiguity,
soft felt piano, glockenspiel, a Hammond-style organ, bass guitar, clean acoustic guitar
and tubular bells, built on one short repeating piano figure with layers entering one at
a time, a peak with the bells, and the layers dropping away back to the piano. Nothing
from the brief's references is copied: the figure, the patterns and the bell melody are
written by the seed.

- **A style can have a metre** (`Style::meter`, like `Style::key`): choosing Orbit sets
  15/8 as well as 92 bpm and A minor, and choosing a style without one from a style's own
  metre goes back to 4/4 (`Style::meter_from`). 15/8 is in the Metre list. The CLI writes
  a style in its metre. Orbit works in any metre: its pulse is the quaver, and a bar is
  felt as two groups (fifteen as seven and eight).
- **The loop**: the figure is a quaver on every pulse for two bars, each group cut into
  cells of two and three, each cell a small shape on the tonic triad plus one colour (the
  ninth or the seventh), the second bar answering the first in its last group. It plays
  unchanged from the first bar to the last (a test checks two-bar cycles match).
  Under it the harmony moves two bars a chord: i, ♭VI, III, iv and ♭VII suspended
  (no leading note), and ♭VII never goes to III, so the relative major is heard but
  never arrived at. The other loops: the bass on each group, the glockenspiel's five-note
  cell in dotted crotchets (five to a bar of fifteen, across the seven and eight), the
  organ's chords voiced close to the last, the guitar's counter-line a fixed shape on the
  current chord's notes.
- **Accumulation, no jumps**: each layer's presence is a curve over its sections (bass,
  then glockenspiel in the Neon; organ, then guitar in the Swell; all at the peak; out in
  the same order through the Fade), written as expression (CC11) a step a quaver, so
  layers fade in and out; velocities follow an intensity that rises section by section
  and falls through the Fade. Both are worked out a quaver at a time over the whole
  piece first and slew-limited (a full fade takes about four seconds at least), so any
  arrangement of sections flows without a jump (0.9.1; a test checks orders and random
  plans). The bells are never faded, so their last note rings on past the peak. The tubular bells play only in the Night shift, a slow
  arc on the chord's notes, a note a group (some held over), the loudest notes in the
  piece. The last bar is the piano's figure gathered into a rolled A minor seventh (also
  C major's sixth), held.
- Sounds: GeneralUser GS (bundled) Grand Piano with its top cut at 2.6 kHz and a soft
  touch for the felt piano, Glockenspiel, Tubular Bells, and three new alternatives
  left out of the other styles' choosing (`ORBIT_SOUNDS`): Drawbar Organ (GM 16, Keys),
  Finger Bass (GM 33, Bass) and Steel Guitar (GM 25, Pluck, trimmed +9.6 dB to match the
  koto). A concert hall, dark reverb, a little tape drift and echo wow, no ensemble, no
  drums or city. About -20 LUFS, LRA about 12 LU, peaks -2 to -4 dBFS; short-term
  loudness climbs steadily from about -28 to -16 LUFS at the bells and falls away.
  Checked: the other ten styles write identical fingerprints and titles across 1,200
  pieces.

## D27 — Nebula (2026-11-19)

A twelfth style from a brief for a 1970s analogue electronic ambient score: dreamy,
spacious and cosmic, about 90-100 bpm in 4/4, vintage analogue synthesisers only (a
modular synth, string-machine pads, a vintage organ, a tape keyboard), opening on
filtered-noise wind and breath, water-like blips and a sweeping resonant filter over a
drone; a rolling sequencer and a soft round bass; a simple, naive melody in long notes,
repeating and evolving; string-machine pads swelling between a few warm minor and major
chords; whooshes, sparkling arpeggios and echoing blips across the stereo field;
building in density and brightness, then dissolving back into the wind. No drums. No
named instrument, piece or artist is imitated: the patches and the melody are our own.

- **Synth only.** Eight new factory patches (`patch.rs`, D27 section), each also a
  palette alternative trimmed to its role (and left out of the other styles' choosing,
  `NEBULA_SOUNDS`): Solar Wind (noise through a key-tracked resonant low-pass swept by
  a slow LFO, a breathier band above), Bubble Drops (a resonant low-pass snapping shut
  on a soft pulse), Cosmic Sequence (saw and pulse, resonant, short filter envelope),
  Round Pulse Bass, Starlight Lead (narrow pulse and sine, gliding, late vibrato), Tape
  Flutes (breathy sines with tape wow), String Machine (thin detuned saws through the
  ensemble) and Modular Drone (resonant, swept). With Velour Organ, Glass Chimes and
  Hover Sweep from the factory. The only sample is the tape hiss (the City FX kit's,
  held faint, as in Derelict).
- **Filters move with brightness (CC74)**: the drone sweeps on a sixteen-bar sine, the
  sequencer on eight, both opening as the piece builds; the whooshes sweep up and down;
  the lead and strings brighten with the build.
- **Harmony** four bars a chord, sixteen-bar progressions from i, ♭VI(maj7), III, iv
  and ♭VII; the final four bars return to i. Always natural minor (`Style::mode`,
  applied with the key by `Style::settle` wherever a style is chosen or a piece drawn).
- **Melody**: four four-bar segments A B A' C, one to a chord; long notes (rhythms in
  sixteenths of a segment, the shortest a beat, mean at least three beats), A' the same
  rhythm and steps as A from a note of its own chord, long notes resting on the chord.
  Each time round it evolves: from the second, the longest notes of B and C lean a step
  up on their last beat; from the third, B's peak is a step higher; at the build the tape
  flutes double it an octave up.
- **Build**: per-layer presence and intensity curves by part, a quaver at a time over the
  piece and slew-limited (as Orbit's, 0.9.1), the expression itself never stepping more
  than 8 a quaver, so any arrangement flows; the sequencer plays quavers, fills seeded
  semiquavers past half intensity and all of them near the peak; sparkles every four bars,
  every two at the build; blips more often in the air.
- Mix: a plate for the spring reverb, a long dotted-crotchet ping-pong tape echo (0.55
  feedback, wow 0.25), tape drift 0.2, the ensemble on strings, flutes and organ; about
  -19 LUFS, LRA about 9-10 LU, peaks around -4 dBFS; short-term loudness climbing from
  about -26 to -15 LUFS and dissolving to about -34. Checked: the older styles write
  identical fingerprints and titles across 1,200 pieces.

## D28 — Vigil, filled tracks, bars per image, section moves (2026-11-23)

Asked for: a score based on Overcity with a gentle brass instrument, the strings swapped
for a drone (two drone tracks in all) and the pluck replaced by a grand piano; images set
to a number of bars on the gallery timeline, the calculator considering a tempo change as
well as changing sections; added tracks always populated; zoom on the piano roll; a
transport control to skip to the next or previous section; sections reordered by dragging.

- **Vigil is Overcity re-voiced, not a new composer** (`vigil.rs`): `compose_plan` writes
  the piece as Overcity and `adapt` changes the instruments, so a seed plays the same
  theme and form in both, and every Overcity fix reaches Vigil. `Style::base` maps it to
  Overcity for the plan and chord length. Its own stream (`vigil`) picks only the second
  drone, so the Overcity draws are untouched.
- **Gentle brass: sampled French Horns** (GM 60), played at 85% velocity with the
  swells' expression at 90%, no ensemble or echo (which doubled sampled brass in
  Derelict), a 3.5 kHz high cut; the fader 5 dB up from the trim, as the sampled horns
  under Overcity's swells measured 10 dB under the synth brass. Stem about -28 LUFS,
  against Overcity's brass at -25.
- **The second drone follows the chords**: each chord's lowest note and its fifth (else
  its next note) moved together into the octave from D3, held on through chords that keep
  them. One of Glass Drone, Choir Drone, Ozone Drone or Bowed Glass, never the first
  drone's sound, each with its own fader measured in that register (the drones' trims are
  measured two octaves lower, and differ by up to 10 dB up there). Stem about -26 LUFS,
  where Overcity's pad was about -20.
- **Grand Piano as a Pluck** (trim -4.3 dB, level-matched), left out of every other
  style's choosing (`VIGIL_SOUNDS`), so their pieces are unchanged; struck 10 velocity
  harder and 8 dB up, as the soft glass velocities are far quieter on a sampled piano.
- Measured over five seeds at 48 bars: -16 to -19.6 LUFS, peaks under -0.4 dBFS (Overcity
  -15 to -18); the master left as Overcity's. The ten older styles write identical
  fingerprints and titles across 1,200 pieces.
- **Added tracks always have notes** (`fill.rs`): when neither the style nor its next 15
  seeds write a role (or the piece wasn't composed), a part is written over the piece's
  own harmony, read bar by bar by matching triads to the notes sounding (the low notes
  weighted, staying on the last chord on a tie), and the key's tonic minor where nothing
  plays. Each role in its own way: held voicings (pad, strings, horn swells), roots
  (bass), the commonest root held a section at a time (drone), a rolled chord a half bar
  (keys), sparse chord tones (bells), an arpeggio (pluck), two-bar phrases on chord tones
  (lead, the answer in the gaps), a soft kit groove, and rain for the city; quieter in an
  Intro or Fade. "New notes for one track" on such a track writes it the same way.
- **Bars per image** (`Slideshow::by_bars`, `image_bars`, default 4): every image the same
  whole number of bars, whatever the tempo; following accents is off in it. The tempo
  then only sets the time on screen and the sections only the count, so the calculator
  offers both: `tempo_for` (the tempo at which N bars last the time aimed for; 30-220
  bpm), a table of 2-16 bars, and the existing section shares. In seconds and on bar
  lines, `tempo_to_fit` searches tenths of a bpm outward from the current tempo for one at
  which the score's bars hold every photo, offered beside the section shares. A tempo
  change follows the app's rules: an untouched piece is written again, an edited one
  keeps its notes.
- **Roll zoom buttons** (− + and whole piece) beside the tracks' + Add, zooming about the
  middle; ⌘-scroll and pinch stay. Not over the roll (where they hid notes and a missed
  click added one), and not ⌘= and ⌘−, which egui uses to scale the window.
- **The playhead's own strip** (16 px) above the Sections strip, with a larger handle:
  click or drag there to move or scrub it. Dragging on the sections had moved or resized
  them instead; scrubbing on the bar numbers still works.
- **Section buttons in a row of their own** (Edit, 19 px) between the playhead strip and
  the section names, shown for every section wide enough (a narrow one while the pointer
  is over it). Drawn over the names, they lost their clicks to the drag that moves a
  section. From the top: playhead, Edit, Sections, Bars.
- **Skip by section**: ⏮ ⏭ in the transport, **[** and **]** on the keyboard; back goes
  to this section's start, or the one before when already there (or playing within a
  second of it); the roll scrolls to show it.
- **Section moves by dragging** a section's body on the Sections strip (its end still
  resizes it): a ghost follows the pointer, a line marks the gap; on release it goes
  through `apply_arrangement` like any reordering, so an untouched piece is written again
  in the new order and an edited one keeps every section's notes. A lead-in stays first.
- **Doubled keys and stepped swells (0.10.1, then after)**: a key struck twice at once on a
  track plays as one note (the timeline merges them; Overcity's Night shift no longer
  writes them). Expression and brightness glide between their written points in
  playback, a unit at a time at most every 20 ms, so the composer's few-point swells don't
  snap the filter open or step the level: chosen in the timeline, not the composer, so
  saved pieces and drawn curves are heard the same way and the fingerprints don't change.
  Other controllers (pedal, bank, etc.) still switch at once.

## D29 — Cinder and Filmstrip, worn tape, four new field sounds (2026-11-24)

Two styles from two briefs. **Cinder**: dark ambient electronica, post-apocalyptic,
desolate and cinematic, about 80 bpm; a sparse half-time broken beat with a dry, muffled
snare and deep sub-bass; wide detuned analogue pads, hollow brass-like synth stabs, long
minor drones; worn tape (saturation, wow and flutter, hiss); distant processed voices and
faint radio static; minimal melody, lots of empty space, slow-evolving, ominous but
quiet. **Filmstrip**: nostalgic downtempo electronica, warm and hazy, a 1970s children's
educational film heard through a worn video tape; a lazy, swung hip-hop-influenced beat
about 90 bpm with soft dusty drums; detuned analogue melodies with wobbly, drifting pitch;
mellotron-style pads, glockenspiel and music-box accents, soft electric piano;
children's voices, filtered and pitch-warped; tape degradation, vinyl crackle, field
recordings of playgrounds and birdsong; bittersweet, innocent, slightly eerie. No named
film, programme, artist or instrument maker is imitated or named: the patches, sounds,
tunes and titles are our own.

- **Worn tape** (`effects.rs` `Tape`, `Fx::tape`, the Effects tab's Tape dial beside
  Ensemble): over the whole mix, before the master gain: one transport for both sides,
  so the pitch wows (0.43 Hz), flutters (6.3 Hz) and drags (a random wander) together,
  about ±12, ±1.5 and ±6 cents at full; saturation untouched below a knee that falls to
  0.55 of full scale, rounded above it, never past full scale (a first try normalised
  `tanh`, which raised quiet signals by 8 dB: rejected); the top dulled from 20 kHz to
  7 kHz. It fades in from the dry signal, and 0 (the default, so every saved piece and
  every other style) bypasses it entirely. Cinder uses 0.45, Filmstrip 0.7. (`Fx::drift`
  and `Fx::detune`, inherited from midigen, are still stored and still do nothing.)
- **Field sounds** in the City FX kit (`kit.rs`, synthesised, looping seamlessly): Radio
  Static (122: band-limited static whose reception fades and sputters, interference
  crackles, a heterodyne whistle drifting), Vinyl Crackle (123: ticks, pops, faint hiss,
  a warp's thump once a turn at 33 rpm), Birdsong (124: five birds of four kinds of song,
  near and far) and Playground (125: a murmur of children at play from formant-filtered
  noise, and calls and laughs sung on a sawtooth through children's vowel formants, all
  muffled by distance). They restart on a seek, as rain does (and now tape hiss too).
- **New patches**, each a palette alternative trimmed to its role and left out of the
  other styles' choosing (`CINDER_SOUNDS`, `FILMSTRIP_SOUNDS`): Ashen Pad (two saws ±14
  cents apart, panned wide, slow), Hollow Stab (a square with a brassy filter blat and a
  saw an octave down, gone in a second or two), Wobble Lead (a saw and pulse ±9 cents,
  16 cents of drift and a 0.9 Hz sway, gliding), Tape Strings (each key's tape runs out
  after about nine seconds, wavering). And from the GM font, Voice Oohs (Pad).
- **Cinder** (`cinder.rs`): natural minor, four bars a chord from i, ♭III, iv, v, ♭VI,
  ♭VII (no leading note). Layers fade in and out by section as Nebula's do (the shared
  `nebula::envelopes`, at most 0.06 a quaver). The beat: the dull kick on the downbeat
  and a broken pattern held four bars; the big snare on the bar's middle beat, its top
  cut to 2.2 kHz and nearly dry; ticks only as it fills; the sub (Pure Sub) under each
  kick. Stabs: root, fifth and octave, on a new chord (and again two bars on at the
  fullest). Voices: the GM choir thinned to 350 Hz-2.4 kHz, deep in reverb and echo,
  bending down about a third of a semitone through each. Melody (Glass Lead): three or
  four long notes over five of eight bars, rising and sighing onto the next chord, played
  every other phrase only.
- **Filmstrip** (`filmstrip.rs`): Dorian, two bars a chord from i9, IV9 (the mode's
  major sixth), ♭IIImaj7, ♭VIImaj7, v7, ii7 and the half-diminished vi. The beat (GM Room
  kit, top cut to 4.5 kHz): semiquaver offbeats swung 24% of a semiquaver late, the
  snare laid back 18 ticks on two and four, everything else up to 14 ticks late, never
  early (a jitter either side put a kick into the section before). The tune: A B A C over
  four chords, on the music box at the start and the end, the lead between (resting every
  third time round in the Swell), the glockenspiel doubling its long notes at the
  fullest. The children's chant is the playground's sol-mi-la-sol-mi, set on the tonic's
  seventh, fifth and octave so it sits in the mode; the voices sway ±900 in bend and now
  and then sink. At the end the music box, strings and piano bend down two semitones
  over the last bar: the tape winding down.
- **Titles** avoid well-known titles and names (a first list made "Secret Garden",
  "Paper Kites" and "Last Shelter" possible: changed).
- **Level**: Cinder about -17.5 LUFS, Filmstrip about -16.5, peaks about -0.5 to
  -1 dBFS. The other thirteen styles' notes are unchanged (fingerprints of 1,560 pieces
  identical to 0.10.2's).

## D30 — Strata, Lumen and Mantra: three drone scores, five new drones (2026-11-24)

Asked for three new drone-based scores with new synth drone sounds. Three different
answers to "drone": one deep and dark with no beat, one bright, one rhythmic. Each is
written by its own module in the manner of Cinder (layers fading in and out by section
through the shared `nebula::envelopes`, CC11 at most 8 a quaver, the `Writer`), four
bars a chord over a drone that holds the tonic (and fifth) through every section and a
bar into the next, so in any order of sections the drone never stops.

- **New patches** (`patch.rs`, category Drone; palette Drone alternatives, trimmed to
  within ±3 dB; left out of the other styles' choosing as `strata::DRONE_SOUNDS`):
  Basalt Drone (saw and 30% pulse an octave down into a 260 Hz low-pass at 0.45
  resonance, driven, a sine two octaves down; LFO 0.03 Hz on the filter), Overtone Drone
  (a saw through a 700 Hz low-pass at 0.86 resonance tracking the keyboard fully, so CC74
  (±3 octaves) moves a whistling peak across the harmonics; a soft saw an octave down
  under it), Radiant Drone (sine and soft square ±5 cents, FM sines an octave up, 15%
  tremolo at 0.12 Hz), Prism Drone (FM sines at ratios 3.5 and 1.5, one and two octaves
  up, 30% tremolo) and Buzz Drone (a 12% pulse and a saw, plucked: the filter envelope
  closes over 2.8 s onto a resonant, driven buzz that sustains at 35%).
- **Strata** (`strata.rs`): Phrygian (i, ♭II, iv, ♭VI, ♭vii), no drums at all. The
  Overtone Drone's brightness is two slow sine waves (7 and 3 bars) against each other,
  reaching ±18 to ±44 as intensity rises, so the harmonic it sings wanders without
  repeating. One riser into the first Night shift and a boom as it lands; hull groans
  (tuned to the tonic) and distant metal or scrapes now and then; the bell every four
  bars from the Night shift. The Black Hole reverb, a half-note echo, no tape.
  The choir of drones was first the loudest layer (-20 LUFS of a -19 mix): turned down
  5 dB and the bedrock and overtones up, so the drones lead.
- **Lumen** (`lumen.rs`): Dorian, the tonic voiced as a suspended ninth (no third) with
  IV, ♭III and ♭VII major. The pulse is a four-note figure of chord degrees in quavers,
  every other quaver while quiet; chimes are 3-5 note falling runs on the minor
  pentatonic; the melody (Starlight Lead) four or five long notes over six of eight bars,
  an octave up every other time in the Night shift. The Shimmer reverb at 0.35.
- **Mantra** (`mantra.rs`): harmonic minor (i, iv, ♭VI, V). The cycle is a pluck every
  beat for the whole piece, the fifth below, the tonic twice, the tonic an octave down,
  as a drone lute is tuned and played (the repeated tonic cuts the one before, as a
  re-pluck does). Drums (kit): a frame drum on the low tom with the sub under it, the
  low and high tabla strokes, a shaker in semiquavers past 0.6 intensity; one of four
  loops per seed. The voice (Ribbon Lead, mono with a 0.28 s glide) holds each note a
  semiquaver into the next so it slides; every phrase ends on the tonic, fifth or octave,
  and in the Night shift begins a step higher each time (at most four).
- **Titles** avoid well-known names (a first list made "Molten Core", "Clear Sky" and
  "Burning Wheel" possible, and repeated the style name Vigil: changed).
- **Level**: Strata about -18 LUFS (its weight is low, where loudness counts least;
  peaks about -0.7 to -1.4 dBFS), Lumen about -16.3 (peaks about -2.5), Mantra about
  -16.7 (peaks about -0.7 to -1.4). The other fifteen styles' notes are unchanged
  (fingerprints of 1,800 pieces identical to 0.11.0's).

## D31 — Track density and complexity; Lumen and Mantra withdrawn; the ruler's menu (2026-11-24)

- **Lumen and Mantra** were heard and not liked; Strata stays. Their code, tests and
  sounds stay (`Style::offered` is false for them): the Score tab, the CLI and the
  screenshot names no longer offer them, and a piece already in one still opens, plays
  and shows its style while it's chosen. Their drones (Radiant, Prism, Buzz) stay in the
  palette.
- **Density and complexity** (`shape.rs`, `Track::density`/`complexity`, 0..1, 0.5 =
  as written, left out of the saved file at 0.5 so every saved piece and fingerprint is
  unchanged): applied when the timeline is built and when MIDI is exported, never to the
  written notes, so turning a dial back restores exactly what was there and editing still
  edits what's written. Chosen over rewriting notes because the dials then work on any
  track (composed, recorded, drawn, imported) and need nothing from the styles. Which
  notes go or come is decided by a hash of their tick and key, not a random stream, so
  one edit never reshuffles the rest. Density down ranks each chord (notes within 40
  ticks) by its beat (downbeat, beat, quaver, semiquaver), length, velocity and a little
  hash, keeping 15% to 100%; up strikes each chord again in the gap to the next on the
  semiquaver grid, quavers up to halfway and semiquavers beyond, each strike softer.
  Complexity down keeps `round(n × share)` notes of a chord (bass, top, then inner from
  the bottom) and of the distinct notes struck alone in each bar (where there are three
  or more: the lowest, then the most played); up adds the top an octave up, then the
  bass an octave down (single notes: some doubled an octave up). On a drum kit the same
  "lowest first" keeps kick and snare; up adds ghosts a semiquaver before hits; city
  effects are never changed by complexity (their keys are different sounds).
- **The ruler's sections**: making a section's name draggable (0.10.0) put a drag-only
  widget over it that took every button, so the ruler's right-click menu never opened
  there and a right-drag moved the section. Now the name takes left-drags (move), clicks
  (the playhead, as elsewhere on the ruler) and right-clicks (the same menu as the ruler:
  the section's name, part, new notes, copies and delete, then the loop).

## D32 — Pachinko: a routed note gate; per-track note routing in the engine (2026-11-24)

Asked for an effect, taking inspiration from a eurorack Pachinko module, that a track is
routed to and that decides whether each note plays, with the engine changed to route
per track for it (the reverb and echo routing left as they are). The Pachinko modules
sold are clones of a well-known random sampler whose gate section drops a ball left or
right each clock tick (its bias setting the odds) and whose déjà vu control loops recent
random values. Only that idea is taken; nothing is named after the original.

- **Model** (`pachinko.rs`): two units (`Project::pachinko`, `UNITS` = 2). Each step (a
  semiquaver, quaver, crotchet or bar in ticks: no tempo changes, so steps are even in
  seconds too) one ball u in [0, 1) falls; left if u < chance. `Track::route` names a
  unit and a side; a note is heard if its step's ball fell its side, so L and R of one
  unit are exact complements, and every note struck in a step (a chord) shares a ball.
  Déjà vu: a loop of `length` (1-16) balls; each step draws keep and fresh; if keep ≥
  déjà vu the loop slot takes fresh. So 0 is always new, 1 a locked loop, and the chance
  compares at play time, so turning it over a locked loop thins or fills that pattern
  (as the module's bias does).
- **Deterministic**: the balls are a function of the step number, seed, déjà vu and
  length only, worked out from step 0 (`Balls`, xorshift64*, allocation-free, fixed
  16-slot memory, moved forward as the engine plays and recomputed from 0 after a jump
  back or a change: thousands of steps take microseconds). Live, after a seek, a WAV and
  the exported MIDI (`pachinko::passed`) agree; the roll uses the same.
- **Engine routing**: `MixSettings` carries each track's route and the units (sent like
  the mix, so turning a dial or rerouting takes effect at once, without rebuilding the
  timeline); `Engine::play` sends a timeline note-on through its unit (offs, controllers
  and unrouted tracks pass straight through; a note-off for a note never struck is
  harmless). Notes picked up on a seek go through the gate at their own start time
  (`Timeline::chase` now returns it). Live input and auditions aren't gated (they're the
  player's, not the score's). The reverb and echo sends are unchanged.
- **Order**: a track's density and complexity (D31) shape its notes first; Pachinko
  gates what they leave and add.
- **Saved files**: units at their defaults and unrouted tracks are left out, so every
  saved piece, and every style's fingerprint, is unchanged.

## D33 — Fade-in/out transitions and an HD video export (2026-11-25)

Asked for transitions where every image just fades in, just fades out, or fades in and
out, each with its own timing; and a video export, in HD, with the gallery's timings and
frame rate honoured.

- **Transitions**: `Transition::{FadeIn, FadeOut, FadeInOut}` with `Slideshow::fade_in`
  and `fade_out` (seconds, default 1). Unlike the crossfade and the dip to black (centred
  on the change), these are each image's own: the fade in starts on the image's first
  frame from black, the fade out ends on its last. If both together are longer than the
  shortest image they shrink in proportion to fit it. Older documents open unchanged
  (the fields default).
- **One schedule** (`shots.rs`): each photo's place and fades in whole frames, every
  fade an even number of frames so it centres on a frame. The FCPXML (dissolves to and
  from black gaps, as the dip to black already did) and the video both follow it, so the
  timeline, the markers and the video agree to the frame. The existing transitions'
  timelines are unchanged (their tests pass as before).
- **Video** (`video.rs`): 1920×1080, H.264 (x264 at CRF 17, tuned for stills; else the
  system's encoder), yuv420p tagged Rec. 709, AAC 320k at 48 kHz, `.mp4` or `.mov`,
  constant frame rate at the gallery's fps. Every frame is composited here (frame `n` is
  the picture at `n / fps` s; linear fades; later images over earlier over black) and
  piped raw to ffmpeg, so timing is ours, not the encoder's. Its length is the
  soundtrack's (the music and its rung-out tail) rounded up to a whole frame. Photos are
  read and scaled to fit (black bars) by ffmpeg, which also reads HEIC; one it can't
  read shows as black and is reported.
- **ffmpeg isn't bundled**: an H.264 encoder can't be shipped under the project's
  licence terms without patent and GPL questions (D4's spirit), and the Resolve package
  remains the no-dependency route. It's found via `KIBBLE_FFMPEG`, beside the app, the
  `PATH`, or the usual install folders; without it, the export says how to get it. A
  test encodes a short gallery and checks the size, rate, frame count and fades on given
  frames (skipped where there's no ffmpeg).

- **First image fade in and stretched final fade** (0.13.1, asked for): with Fade out
  only, `Slideshow::first_fade_in` fades the first image up from black (within its time,
  after its fade out); 0 is none. `stretch_final_fade` (Fade out, Fade in and out) makes
  the last image's fade out start where it would have and end on the soundtrack's last
  frame (the WAV's, tail included), so `shots::shots` now takes that end frame: the
  video's frame count, or the FCPXML's soundtrack length. The picture then reaches black
  as the reverb dies away rather than sitting black through the outro.

## D34 — Photos in an order set by hand (2026-11-25)

Asked for a way to set the photos' order for the exports (a strongest image first, not
the folder's listing): a Reorder images step in Plan and export opening a pane of
thumbnails to drag into order, saved on closing; no removing photos yet.

- **Stored by file name** (`Slideshow::arranged`, with `PhotoOrder::Custom`), so the
  order survives the folder being copied or moved, and a `.kibble` stays readable.
  `Slideshow::photos` is the one listing every export and the calculator use: the
  folder's photos in the names' order, any it doesn't name (added since) after, by name;
  names of photos gone are passed over. Choosing another folder clears it. Older
  documents open as before (empty is left out of the file).
- **Thumbnails** (`reorder.rs`): decoded on up to four background threads to 320 px at
  most, turned as the camera's EXIF says, by the `image` crate (already in the build
  through eframe; JPEG, PNG, TIFF, WebP, BMP, GIF), or ffmpeg where it's there for the
  rest (HEIC, raw). The pane opens at once and fills in.
- **The pane** is a modal: tiles in rows (each tile a drag source; a line shows where a
  drop lands), a right-click menu and buttons for to the front or the end, arrow keys
  for the selected tile, Folder order to start again, Cancel or Save and close.

## D35 — Canopy and Tarn: two pastoral scores, new pads, drones and field sounds (2026-11-26)

Two briefs. **Canopy**: pastoral ambient electronica, serene and hopeful, beatless, about
60 bpm, D major or Lydian; warm analogue string-style pads with a slow four-second attack,
chorus and gentle detune; a soft low sine and triangle drone on the root and fifth,
slowly breathing; sparse electric piano and celesta phrases with long decay and plenty of
space; now and then a tape-wobbled glockenspiel; faint birdsong and wind; light tape,
soft reverb; sunrise in a forest. **Tarn**: warm pastoral ambient, nostalgic and
reflective, beatless, about 54 bpm, A Dorian; tape-keyboard breathy pads and a warm
filtered saw pad with long attack and tape wobble; a deep analogue drone on the root with
slight pitch drift; a soft felt piano playing sparse, simple motifs with long pauses;
distant reverb-soaked nylon guitar; faint crickets and breeze; tape, vinyl dust, wide
reverb; golden hour over a lake. Both: no drums, percussion or voices. Asked also to find
new pads and drones, synthesised or from SoundFonts. No instrument maker is named (the
brief's tape keyboard is "Breath Choir", "Tape Choir").

- **Lydian** (`Mode::Lydian`): added for Canopy, but not to `Mode::ALL`, so no other
  style, random recipe or test sweep meets it; the Score tab lists it after the four
  minor modes only while the chosen style is written in it. Canopy and Tarn set their
  keys as Return and Orbit do (`Style::key`: D Lydian, A Dorian); the key can be changed
  after.
- **New patches** (`patch.rs`): Dawn Strings (saws ±8-9 cents and a soft pulse an octave
  up, 4-4.5 s attack, filter opening with it; played through the ensemble), Breath Choir
  (a hollow pulse-and-sine tone with a layer of noise through a resonant, fully tracking
  low-pass, 7 cents of 0.55 Hz wobble and 7 of drift), Amber Pad (two saws ±11-12 cents
  behind a 520-600 Hz low-pass whose envelope takes five or six seconds to open, 6 cents of
  0.42 Hz wobble), Meadow Drone (a sine, and an octave down a sine and square through a
  380 Hz low-pass: a rounded, triangle-like tone; 25% tremolo at 0.08 Hz) and Loam Drone
  (saw and pulse into a 240 Hz low-pass, a sine an octave down, 14 cents of drift).
- **Found in the bundled fonts** and never offered before: the Ambient Electronic font's
  Warm Analog Pad, Airy Choir and Glass Pad (programs 88, 91, 89), and GeneralUser GS's
  variation banks: Orchestra Pad (8:48), Synth Strings 3 (8:50), Chorused Tine EP (8:4,
  offered as Chorused EP) and Bell Tine EP (12:4). No new font was added: these were
  already shipped and licensed (D4), and searching further afield for a font would need
  its licence checked and its checksum pinned first.
- All twelve are palette alternatives, trimmed to their roles (the slow-attack pads are
  measured in the level test's three seconds, so they read low there and carry larger
  trims), and left out of the other styles' choosing as `canopy::PASTORAL_SOUNDS`.
- **Field sounds** (City FX kit, synthesised, looping seamlessly, restarting on a seek):
  Crickets (126: seven crickets, each a chirp of three or four 24-38 Hz pulses of a
  3.9-5.3 kHz tone, repeated at its own rate, resting now and then; the far ones duller
  and quieter) and Breeze (127: low noise swelling in two slow gusts out of step, and a
  2.2-6.5 kHz band of leaves rustling with the cube of the gust).
- **Canopy** (`canopy.rs`): four bars a chord, every progression with the major II (the
  Lydian colour), the tonic as an add9 or major seventh, iii7, V add9, vi7. The drone
  holds root and fifth through every section and a bar into the next; its expression
  breathes on a three-bar sine. The piano's and celesta's phrases are drawn from three
  per seed (two to four notes, stepping, the last on the chord), so a piece's phrases are
  kin; the piano plays in a chord's first bar, the celesta answers in its third, each
  note gated by the layer's presence. The glockenspiel plays one or two notes in a
  chord's last bar; its wobble is pitch bend every demisemiquaver while it rings (a
  0.45 Hz sway and a 0.11 Hz drag, 15 cents at most): the tape over the mix (0.3) wobbles
  everything a little, this the glockenspiel much more. `canopy::wobble`, `phrases` and
  `realise` are shared with Tarn.
- **Tarn** (`tarn.rs`): i add9, IV add9, ♭III maj7, ♭VII, v7, four bars a chord. The
  felt piano is the GM grand cut at 1.9 kHz, played softly; it plays one motif (three
  times in four) or another, its second note now and then a step away, then rests
  through the chord. The guitar's broken chords (root, fifth, octave and the chord's own
  third, three-quarters of a beat apart) are far off: low and high cut, 0.85 reverb.
  The Cloud reverb, a half-note echo with wow, tape at 0.5.
- **Level**: Canopy about -16 LUFS (peaks -1.2 to -3.6 dBFS), Tarn about -16.8 (peaks
  -1.1 to -2.7) over six seeds at 48 bars. The pads were first 12 dB over the piano:
  turned down and the keys, bells and field sounds up. The other eighteen styles' notes
  are unchanged (fingerprints of 1,800 pieces identical to 0.13.2's).

## D36 — Ambience as beds, more from the dials, a panic button, VU meters (2026-11-26)

Asked (after hearing Canopy and Tarn): the tape hiss far too present, to be very subtle;
the field sounds (hiss, crickets, birds, rain…) not tracks but options with their own
levels, a separate kind of track for background ambience, per score; more variety from
New notes and much more from the density and complexity dials (changing intervals or the
scale's use), so more than one or two settings of each score are useful, rather than more
scores; the panic button at the far left with the transport, red, an exclamation mark;
the level controls to look like VU meters, red over 0 dB, amber under, green under that,
after a standard.

- **Beds** (`ambience.rs`, `Track::ambience`): each bed is a track holding one sound of
  the City FX kit, so the engine, timeline, render and MIDI export need nothing new. Chosen
  over a separate list of sounds in the project (which would have needed its own mixer
  channel, meter and chase in the engine). The fourteen kinds (`BEDS`): rain (any of the
  nine), thunder, machinery hum, neon buzz, tape hiss, vinyl crackle, radio static,
  birdsong, playground, crickets, breeze, engine throb, life support, ticking. Score
  effects (risers, booms, braams, metal, grains, drones, hull groans, pipes, timpani, steam,
  scrapes) stay music.
- **No style was rewritten**: `compose::finish` calls `ambience::adopt`, which splits each
  City FX track by bed (a copy of its strip, expression and bar levels for each, so the sum
  is the same), leaves the effects on the track (named Effects if it was City) and puts the
  beds after the music. A test compared 1,080 pieces of every style with 0.13.2's: every
  music track's notes, controllers, strip and sound identical, and the City notes the same
  set. (It found that Momento and Noir could choose the electric pianos added in D35 for
  their keys: they're now left out of that choice too, so D35's claim that no other style
  changed now holds for sounds as well as notes.)
- **Opening older files** (`Document::load` → `adopt_opened`): the same split; a piece that
  was unedited is still unedited (its fingerprint taken again). `fingerprint` leaves the
  beds out, so switching one on or off or adding one isn't an edit.
- **Kept through new notes**: a new piece in the same style (New notes, New piece, a new
  key, Write again, a tempo change) keeps the beds as set: the same set, each at its level
  and switch (`ambience::keep`). Another style brings its own. Section rewrites leave the
  beds alone. Callers now say whether the style is the same, since the Score tab writes
  the new style into the recipe before the piece is written again.
- **In the app**: the track list, the roll's overview, its ghosts, the section menu's
  tracks and the mixer leave the beds out; the selection never rests on one. Under the
  track list (always in view) an Ambience panel: each bed's lamp (on/off), name (rain: a
  menu of the nine), level slider (-40 to +6 dB), small meter and ×; **+ Add** with the
  kinds not there (none in Noir, D18). The City role leaves the + Add menu for tracks. The
  Effects track shows "Score effects" where the rain menu was.
- **Tape hiss** made 10.5 dB quieter in the kit itself (0.2 to 0.06 of the kit's level),
  so every style that uses it (Derelict, Nebula, Cinder, Filmstrip, Tarn) gets the faint
  version; the kit test's "not silent" floor moved from 0.1 to 0.03 for it.
- **The dials** (`shape.rs`), now knowing the piece's scale (`scale_of`: the recipe's key
  and mode, else the seven pitch classes most used):
  - complexity below 0.3 bares chords to their bass and the fifth above it in the top's
    octave (all chords by 0), as well as thinning them;
  - above about 0.62 it colours chords with the scale's next third above the top (a
    seventh or ninth, every chord at 1); above 0.6, a melody (single notes, median at or
    above A3) gets a third below in the scale on some notes (never a bass line); above 0.7,
    passing notes of the scale halfway through leaps of a third or more with a beat or more
    between, as well as the octaves as before;
  - density below 0.3 lets what's left ring on to the next note kept (up to two bars);
  - above 0.5 more and more of a chord's restrikes are an arpeggio (up, down, or up and
    down, chosen per chord) instead of the whole chord.
  All decided by hashes of where the notes are, as before. A test checks seven settings of
  one Canopy piece all play different notes.
- **Vary dials** (Score tab, on by default; `shape::vary`): New notes and New piece turn
  each track's dials somewhere new (a third of the time straight up, else 0.15 to 0.88),
  leaving drones, effects and beds straight up. Off, a new seed keeps the dials as set; a
  piece written again in the same style keeps them anyway (`keep_dials`; they used to
  return to straight up).
- **Panic**: the first control of the transport, a red rounded button with a white "!";
  it now stops playback (or ends a take) as well as silencing every voice and clearing the
  reverb, echo, ensemble and tape (`App::panic_stop`).
- **Meters**: no IEC or EBU document found defines colour zones for a digital meter
  (IEC TR 60268-18 sets the scale, 0 = full scale, not colours; the K-system puts green
  below its 0 and yellow above); the zones follow EBU R68's alignment: green below
  -18 dBFS (0 VU), amber from there, red at 0 dB, taken as -0.5 dBFS since nothing here
  passes full scale (the mixes' ceiling is 0.95). Drawn as lamps (a gap every 3 points),
  each zone faint beyond the level so the scale reads in silence; the same on the
  mixer, the track list and the beds. Faders' grooves carry the zones: red over 0 dB.
- **Level**: the demo -14.7 LUFS, Tarn -16.9 (about the same without the hiss), Tarn with
  every dial at an extreme -17.4; peaks -1.2 to -2.9 dBFS.

## D37 — Images with the chords; compact ambience rows (2026-11-27)

- **The problem**: on bar lines the gallery chose the nearest whole bars to the time asked,
  often three (12.9 s at 56 bpm), and following the accents could pick any count; three-bar
  images over four-bar chords and phrases feel like a waltz across the beat. Some
  styles also change chord every three bars at some tempos (`chord_bars` rounds seconds
  to bars), so even four-bar images could cut across them.
- **With the chords** (`Slideshow::by_chords`, the default for a new gallery; older
  documents, without the field, keep their mode): each image is the score's chord length
  (`compose::chord_bars_at`: the styles with their own modules keep theirs; a piece not
  composed counts a chord a bar) times 1, 2, 4, 8 or 16, or a half or quarter of a long
  chord (`phrase_choices`), the one chosen if it keeps to the chords, else the nearest the
  time aimed for (the longer on a tie). The intro rounds up to whole chords (or whole
  images, if shorter) so every change lands on a chord change or halfway through one.
  Exact seconds or bars per image, if set, come first (`Slideshow::chords`). The tempo
  offered for the time aimed for is one at which the chords still fit the images
  (`Grid::at`).
- **Square phrases** (`Recipe::square`, off by default and left out of files when off, so
  every piece written before is unchanged): `chord_bars` takes the nearest of 1, 2 or 4
  bars (log scale, no more than the style's most). The gallery offers it when the chords
  come in threes or sixes; the piece is written again with its sections moved to whole
  chords (`snap_plan`), sounds, dials and beds kept; ⌘Z undoes it.
- **Ambience rows**: M to mute (the lamp read as decoration); short rain names
  (`RainKind::short`); a long name scrolls slowly to its end and back while hovered
  (`mixer_ui::marquee`); the slider and its dB readout replaced by the mixer's fader lying
  down (`mixer_ui::hfader`): the zones in the groove, the level as a thin VU strip along
  the top, marks at -40, -18, -6, 0 and +6, down to silence at the far left.
- **CLI**: `--with-chords`.

## Open

- Bake the factory patches into a `Kibble-481-Synths.sf2` (as midigen's `make_soundfont.rs` does)
  so exported MIDI sounds like the app in any DAW.
- Tempo changes within a piece; per-note pitch curves (MPE); automation of synth parameters.
- Ken Burns moves on gallery stills in the FCPXML.
