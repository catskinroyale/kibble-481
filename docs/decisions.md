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

## Open

- Bake the factory patches into a `Kibble-481-Synths.sf2` (as midigen's `make_soundfont.rs` does)
  so exported MIDI sounds like the app in any DAW.
- Tempo changes within a piece; per-note pitch curves (MPE); automation of synth parameters.
- Ken Burns moves on gallery stills in the FCPXML.
