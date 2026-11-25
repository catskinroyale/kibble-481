# Kibble-481

(Formerly DDream. Projects saved as `.ddream` still open, and settings carry over.)

A synthesiser and sequencer in one window, for writing neon-noir film scores in MIDI:
rain on dark streets, sagging polysynth brass, bells falling like water, a lone sax at
3 a.m. You play it, record it, edit it on a piano roll, mix it, and hand the soundtrack
and a gallery of your photos to DaVinci Resolve as a ready-made timeline, or as a
finished HD video.

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
rustup fetches it. The nine bundled SoundFonts are in the repo.

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
Intro); an Intro can open a piece in any style. In the Score tab the
sections are a swim lane, with the parts to add on the right: drag a part into the lane
to add it there (or click it to add one at the end), drag a section by its ≡ grip to move
it, × to remove one; drag
the end of a section on the Sections strip above the roll to make it a bar or more longer
or shorter. Sections are numbered within their part (**Intro 1**, **Intro 2**, **Neon 1**…)
unless you name them. Click a section's name in the Score tab, or right-click it on the
Sections strip, for its menu: its **name**, its **part** (it is written again as the new
one), **new notes** for the whole section or for **one track** in it, **duplicate** (a copy
after it, notes and all), copy before, and delete. On the Sections strip, hover over a
section for buttons too: **↻** new notes, **‹+** and **+›** copies, **×** out. A composed
piece that hasn't been edited is written again whole to follow the arrangement. Once
edited (and after any of these section changes, which keep the rest of the piece as it
is; ⌘Z undoes them), each section's notes move with it, and a section that is new, made
another part or another length is written again on its own, to its length, the rest as
they were. In a piece that wasn't composed, a longer section plays its notes again to
fill it. Set the key and
mode (minor, Dorian, Phrygian, harmonic minor), the seed (◀ ▶ to step through) and the
density in the Score tab; the same settings always write the same piece.

There are sixteen **styles**, each playing the same kinds of section its own way, all
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
- **Overcity** (60 bpm): a vast, rain-soaked city seen from above at night, cold and faintly
  sacred. Out of near-silence and a sub drone felt more than heard, horns rise on an open
  fifth and fall away like a breath. Hollow voicings (root, fifth and octave, the third a
  tenth above) hover on the tonic, drifting for a moment to the relative major, the sixth
  or a suspension. A sparse, glassy theme of long notes leaps up a fifth or sixth and falls
  by step to a held note, then leaves a long silence, each time a little higher. Under the
  development a slow heartbeat on the sub and rippling glass high up (each seed shaping it its own
  way: up, down, arching, closing in, opening out or scattered); then one great swell
  with everything open, deep booms and distant thunder; then it dissolves into echo over
  the drone, ending on a last distant bell. The horn swells are written as expression
  (CC11) and brightness (CC74). Soft rain and the city's rumble sit far below; drums only
  mark the great swell. It opens on an Intro.
- **Derelict** (48 bpm): a ship adrift in deep space, cold, vast and almost free of time,
  with very little melody and no beat. Out of a quiet void (a detuned analogue drone, the
  ship's engines throbbing far below, faint mechanical ticking and tape hiss, the hull
  groaning and a pipe ringing somewhere far off), a life-support pump breathes, a sub
  pulses in uneven groups, and a worn pad (soundtrack, metal or space-voice pads among
  others) breathes in and out with long silences between. A lone horn or trombone calls,
  a celesta, tubular bell or ring-modulated bell answers with a tritone, steam vents, and
  an oboe, cor anglais or bassoon surfaces with a fragment that never comes home.
  Tremolo strings creep in one note at a time over a bowed double bass, metal scrapes,
  and once in each tightening a swell rises hard and cuts off into silence. A harp or
  pizzicato glissando (its notes, range, shape and pace new with each seed) sweeps up into one brief orchestral
  eruption (horns, strings, pad, timpani and a deep boom), then everything collapses back
  into the drone, the engines winding down, the pump and the pulse failing, the ticking
  stopping, a muted trumpet (or a tenor or baritone sax)
  drifting past like a ghost, and a last celesta note. Harmony barely moves: bare triads,
  clusters, a tritone in place of the fifth. Chords last about fourteen seconds; most
  entries fall a little off the beat. The reverb is cavernous and the whole mix wanders
  like old tape, with no chorus on anything. It uses only the sounds written for it (no
  koto or shakuhachi). It opens on an Intro and measures about -18 to -21 LUFS with a wide
  loudness range (12 to 16 LU), like a film cue.
- **Homeward** (69 bpm): slow, warm, serious and quietly hopeful, built on one singable
  eight-bar theme in the minor key that the seed writes: a first phrase that rises and
  rests on the dominant, a second that lifts towards the relative major, peaks and
  falls home to the tonic over a raised leading tone. Its harmony follows the theme, a
  chord a bar. The theme returns in every section, each statement ending on its cadence:
  on solo piano (or soft electric piano) in the Intro; on legato cellos (or violas) in
  the Neon, over soft piano chords, a sampled concert harp, a bowed double bass and a
  quiet sub; reharmonised in the Swell for a flute, clarinet or French horn, with a
  countermelody in the cellos; in the Night shift a soft pulse builds under layered
  strings and a warm analogue pad to a single peak, the theme in full strings in octaves;
  and in the Fade on the piano as it began, settling on a held tonic chord with a ninth
  on top. A natural hall, no chorus, no drums or city. It measures about -21 LUFS with
  peaks under -2 dBFS. Its tracks: Piano, Cellos, Strings, Double Bass, Sub, Lead, Harp
  and Pad.
- **Return** (66 bpm, E minor): Homeward at dawn, with its instruments chosen. The theme
  opens on a soft electric piano; violas carry it in the Neon; a clarinet varies it in
  the Swell over new chords, answered by cellos of their own; the build peaks with the
  theme in full strings in octaves (the violas and cellos with them); and the solo piano
  brings it home to a long-held E minor chord with an added ninth, fading to silence.
  Its lift reaches the relative major (G major under the theme's peak where the note
  allows). Choosing it sets E minor as well as its tempo, and each new piece keeps it.
  The concert hall, no drums or city; about -20 LUFS with peaks under -2 dBFS. Its
  tracks: Electric Piano, Piano, Violas, Cellos, Strings, Double Bass, Sub, Clarinet,
  Harp and Pad.
- **Chamber** (62 bpm): a chamber orchestra in a dark, sci-fi noir. A small orchestra
  plays over a cold machine: a glass drone, and a sub that beats like a heart (and, at
  the climax, like an engine). Its harmony is noir and a little uncanny, two bars a
  chord, always turning on a dominant seventh: the flat sixth, the Neapolitan, a minor
  chord a major third down or a major one a major third up. The seed writes one
  eight-bar theme from a motif: stated, sequenced on the next chord, raised to a climax
  and falling to the leading tone. The Intro is the void, with the heartbeat, the motif
  on a celesta and a far violin harmonic. In the Neon a clarinet (or cor anglais) has
  the theme over a ticking pizzicato, held low strings and a harp. In the Swell a solo
  violin takes it, with the violas against it, a bassoon walking below, horn swells and
  the celesta echoing. The Night shift builds on tremolo strings, the engine and low
  piano clusters to the theme in three octaves of strings. In the Fade the clarinet
  hands it to the cellos, settling on a held minor chord with a major seventh and a
  ninth. A natural hall, no drums or city; about -19 LUFS with peaks under -1.5 dBFS.
  Its tracks: Celesta, Clarinet, Bassoon, Horn, Harp, Piano, Violin, Violas, Cellos,
  Double Bass, Pizzicato, Tremolo Strings, Glass and Pulse.
- **Orbit** (92 bpm, A minor, 15/8): a hypnotic minimalist score, eerie and serious. The
  bar is fifteen quavers felt as seven and eight, and everything turns on one short piano
  figure of two bars, written by the seed, that loops from the first bar to the last on
  a soft, felted grand. The Intro is the figure alone. Layers join it one at a time, each
  looping a simple pattern of its own: a rounded bass guitar, a glockenspiel figure in
  dotted crotchets (five to the bar against the piano's seven and eight), a warm drawbar
  organ holding the chords, a clean acoustic guitar's counter-line. Under the loop the
  harmony moves two bars a chord, leaning towards C major (the flat sixth, the relative
  major, a suspended seventh) without ever resolving there. It builds by accumulation
  only, each layer fading in, to a peak where tubular bells ring a slow melody over the
  full ensemble, the loudest moment; then the layers leave in the order they came, back
  to the piano alone, fading onto a held A minor seventh. Choosing it sets A minor and
  15/8 as well as its tempo (15/8 is also in the Metre list). A concert hall with a
  little tape drift, no drums or city; about -20 LUFS with peaks under -2 dBFS. Its
  tracks: Piano, Bass, Glockenspiel, Organ, Guitar and Tubular Bells.
- **Nebula** (96 bpm, minor): a 1970s analogue electronic ambient score, dreamy,
  spacious and cosmic, made only of the built-in synthesiser. It opens on air:
  filtered-noise wind and breath, bubbling water-drop blips, and a soft modular drone
  under a slowly sweeping resonant filter. A gentle rolling sequencer rises out of it,
  then a soft, round pulsing bass. Over them a simple, naive melody of long notes,
  written by the seed, repeats every sixteen bars and grows a little each time round (a
  passing note, a higher peak, tape-keyboard flutes doubling it an octave up).
  String-machine chords swell and fade beneath it, four bars a chord, between a few warm
  minor and major chords, with a vintage organ at the build. Whooshes, sparkling
  arpeggios and echoing blips drift across the stereo field. It builds in density (the
  sequencer filling out from quavers to semiquavers) and brightness (the filters
  opening), then dissolves back into the wind and fades. A plate standing in for a spring
  reverb, a long wobbling tape echo, tape drift and a little hiss; no drums. Always in a
  minor key; about -19 LUFS with peaks under -4 dBFS. Its tracks: Wind, Blips, Drone,
  Sequence, Bass, String Machine, Lead, Tape Flutes, Organ, Sparkle, Whoosh and Tape Hiss.
- **Vigil** (60 bpm): Overcity watched over gently. The same vast city from above at
  night, its theme, chords, form and one great swell written as Overcity writes them, on
  other instruments: soft sampled **French horns** for the hollow fifths instead of the
  synth brass; **two drones**, the first as Overcity holds it and a second, an octave and
  more above, following the chords where the choir or strings held them (its root and
  fifth, held on through the chords that keep them); and a **grand piano** rippling where
  the glass did. A seed in Vigil plays the same notes as in Overcity. About -16 to -19
  LUFS. Its tracks: Horns, Drone 1, Drone 2, Lead, Bass, Bells, Piano, City and Drums.
- **Cinder** (80 bpm, minor): dark ambient electronica after the end of the world,
  desolate and cinematic, ominous but quiet. Out of tape hiss and a far-off radio
  between stations (static that fades and sputters, a whistle drifting through it) a
  long drone rises, and distant voices drift in and out, thinned, far back in the reverb,
  sinking in pitch like a slowing tape. Wide, detuned analogue pad chords (the new
  **Ashen Pad**) move four bars a chord. A sparse half-time broken beat enters: a dull
  kick placed differently every four bars, a dry, muffled snare on the third beat, with a
  deep sub-bass under the kick; then hollow, brass-like stabs (the new **Hollow Stab**, a
  square's odd harmonics) echoing into the dark, and a minimal melody of a few long notes
  that plays and then rests for as long again, leaving a lot of empty space. It builds a
  little, never loud, and falls back to the drone and the static. A dark cathedral, a
  slow wobbling echo and worn **tape** over the whole mix. About -17.5 LUFS. Its tracks:
  Tape Hiss, Radio, Drone, Pad, Sub, Drums, Stabs, Voices and Lead.
- **Filmstrip** (90 bpm, Dorian): nostalgic downtempo electronica, warm and hazy, like
  an old children's educational film heard through a worn video tape: bittersweet,
  innocent and a little eerie. It opens on the room, birdsong, a playground across the
  field, a record's crackle and tape hiss, with a music box playing the tune alone over
  tape-keyboard strings (the new **Tape Strings**: each key a strip of tape that runs out
  after about nine seconds). A lazy, swung broken beat drops in, soft and dusty (a room
  kit with its top cut, the snare laid back, never ahead of the beat), with a round
  finger bass on the kick and a soft electric piano comping. A detuned analogue lead (the
  new **Wobble Lead**, its pitch drifting and swaying) takes the tune; glockenspiel and
  music-box accents chime around it, and children's voices sing a playground chant, high,
  filtered and warped in pitch. Then the beat leaves, the music box has the tune again,
  the playground comes back, and the tape winds down. Two bars a chord in Dorian (a minor
  key with a bright, major sixth: its IV chord is major); a small warm plate and heavily
  worn tape. About -16.5 LUFS. Its tracks: Field Recording, Vinyl & Tape, Drums, Bass,
  Electric Piano, Tape Strings, Lead, Music Box, Glockenspiel and Children's Voices.
- **Strata** (50 bpm, Phrygian): a deep drone score, vast, slow and heavy, with no beat.
  Out of a cold wind a drone rises from far below (the new **Basalt Drone**: saws and a
  pulse an octave down into a dark, resonant filter, a sine two octaves down) and never
  stops. Over it the new **Overtone Drone** holds the tonic and fifth while brightness
  sweeps its very resonant filter up and down through the harmonics, a slowly wandering
  whistle of overtones like a throat singer's, roaming further as it builds. A sub swells
  under each new chord, a choir of drones holds the Phrygian chords above (the flat second
  pressing down on the tonic, the flat sixth and seventh), the earth groans, distant metal
  clangs and scrapes, and a bell tolls far off. One great slow climb (a long riser landing
  on a boom) and everything sinks back into the depths and the wind. The deepest reverb
  there is and a long dark echo. About -18 LUFS. Its tracks: Wind, Bedrock, Overtones,
  Choir, Sub, Depths, Metal and Bell.

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
**Write again** brings the composed version back. The gallery's calculator can set the
length to fit your photos. New seeds choose their own instruments, unless **Keep sounds**
is lit; a new style always brings its own. Every step can be undone.

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
vibrato or bends, as the CS-80's second touch did. Sixteen voices. In the Sound tab the
patch is laid out in strips like the mixer's: Layer I and Layer II side by side (each
switched on and off by the lamp by its name), then Glide, LFO and Touch; every setting is
a small dial, straight up at its usual setting, and every choice (octave, LFO shape, mono
legato) a neon lamp.

64 factory patches, all original: eight brass, seven pads, five strings, nine leads,
seven basses, six bells, four keys, four plucks, eleven drones and three atmospheres. Among them are
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
metal, sub booms, braams, risers, reversed cymbals, glass grains, drones and timpani; a
derelict ship's hull groaning and pipes clanging (both tuned), steam hissing, metal
scraping, and throbbing engines, a life-support pump, ticking and tape hiss that loop for
as long as you hold them; a far-off radio's static, a record's crackle, birdsong and a
playground across the field, looping too; and a
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
- **Density and complexity**: two small dials on each track, beside its mute (over solo).
  Straight up plays the notes as written; the written notes never change. **Density**
  left leaves out the weakest notes first (off the beat, short and soft), down to about
  the strongest sixth; right fills the gaps, striking held notes again and echoing hits,
  on the quavers and then the semiquavers. **Complexity** left plays fewer notes of each
  chord (the bass and top kept longest) and fewer of the different notes in each bar's
  arpeggio or line (the lowest and most played kept longest); right adds the chord's top
  an octave up, then its bass an octave down, and doubles some single notes an octave up
  (on a drum kit: fewer drums, the kick and snare kept; then soft ghost notes). The roll
  shows notes left out faded and notes added as outlines; playback, WAV and MIDI export
  play them as heard. Double-click a dial to put it back.
- **Pachinko** (Effects tab): two note gates, A and B, after a eurorack random-gate
  module. Each drops a ball every step (1/16, 1/8, 1/4 or a bar); it lands left with the
  **Chance** set, else right. Route a track (the Routing grid beside them) to a unit's
  **L** output and it plays only the notes struck in steps whose ball fell left; to its
  **R** output, only the others. One track on L at 30% plays about three steps in ten; a
  second on R of the same unit plays exactly the steps the first leaves out. A chord's
  notes share their step's ball. **Déjà vu** at 0 drops a new ball every step; turned up,
  more of them repeat the ball from **Length** steps before, and all the way round the
  pattern loops for ever (turn the chance then to thin or fill that same pattern).
  **Pattern** drops a different set of balls. It is worked out from the step number, so
  the same settings play the same notes live, after a seek, in a WAV and in exported
  MIDI; the roll fades the notes it holds back. Live playing isn't gated.
- **Sections on the ruler**: drag a section's name to move it; right-click it for its
  menu (name, part, new notes, copies, delete, the loop); click it to move the playhead.
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
  (drag a section's name before or after another to move it, notes and all; drag its end
  to resize it, a bar at a time). In the **Edit** row above the names each section has its
  buttons: **↻** new notes, copy before, copy after, and **×** delete;
  also ordered in the Score tab. **⏮ ⏭** in the transport (or **[** and **]**) jump to the
  start of this section or the one before, and to the next, and bring it into view. The
  **playhead** has a strip of its own above the sections: click there to move it, or drag
  its handle to scrub, without touching a section.
- **Zoom**: the **− +** buttons beside the tracks' **+ Add** zoom the roll in and out
  about the middle, and **[ ]** shows the whole piece; ⌘-scroll or pinch zoom about the pointer, ⌥-scroll
  zooms the keys.
- **Tracks**: click anywhere on a track to select it; double-click its name (or Rename in
  its right-click menu) to rename it. Drag a track by its ≡ grip to move it up or down
  the list; its **×** deletes it (⌘Z brings it back; the last track stays). **+ Add** adds a track of any role, with its sound
  and the notes the piece's style writes for that role (a second track of a role plays a
  line of its own). A role the style doesn't write, or any role in a piece that wasn't
  composed, still comes with notes: a part for the role written over the chords the piece
  already has (read bar by bar from its notes), quieter in an Intro or Fade. New pieces
  write added tracks again for their roles.
- Undo and redo (⌘Z, ⇧⌘Z) for everything.

## Mixing

Each track has a strip: a neon lamp, lit while the track is heard (not muted, nor silenced
by another track's solo; the track list has the same lamp), pan and width, the fader with
mute, solo and the ensemble chorus, and at the foot four small dials: reverb and echo sends over a **low cut** and a **high
cut**. Each dial points straight up at its neutral setting (reverb at the usual 35%, no
echo, both cuts off): turn the reverb either way, the echo and low cut right, the high cut
left. Drag up or right to turn clockwise; double-click for neutral. The cuts (12 dB an
octave, 20 Hz–20 kHz) filter the track's sound after its instrument, so they work the same
on the synth, the kits and SoundFonts: a SoundFont has no high-pass of its own and only
some presets answer the brightness controller, so the filter isn't left to it. They are
saved with the project but, having no standard MIDI message, aren't in an exported MIDI
file for other programs. Shared effects (from Ambient MIDI Generator): eight reverb spaces including
**Noir** (a vast, dark 1980s digital hall), Shimmer and Black Hole; a tempo-synced tape
echo; a string-machine ensemble; worn **tape** over the whole mix (wow and flutter,
saturation above a knee, a dulled top; off unless a style or you turn it up); A =
440/432/415 Hz. The Effects tab is three small strips like
the mixer's, Reverb, Echo and Ensemble: each choice (the reverb space, echo time, shimmer
interval, tuning, and Freeze and Ping-pong) is a neon lamp and its name, lit when chosen;
click one to choose it. Under them are the settings as small dials, two to a row, as on
the mixer (the shimmer's own are greyed out in the other spaces).

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
  get the soundtrack, an `.fcpxml` timeline with every photo at its change, the changes as
  `.edl` markers, and a cue list. In Resolve: **File › Import › Timeline**, pick the
  `.fcpxml`, let it import the media. Each image is on screen for **exact seconds** (every
  one the same, wherever the bar lines fall), **on bar lines** (the nearest whole bars, 4
  to 16 s, moved onto the music's accents if you like), or a set number of **bars per
  image** (2, 4, 6, 8, 12, 16 or any; usually even, as phrases come in twos), whatever
  the tempo. Between them: a **crossfade**, a **fade to black** and up again, or a
  **cut**, with the fade's length yours; or every photo can **fade in** from black as it
  starts, **fade out** to black as it ends, or **fade in and out**, each fade its own
  length (both fit within the shortest image). Fading out only, the first photo can still
  fade in (**First image fade in**); and **Stretch final fade** draws the last photo's
  fade out on to the very end of the soundtrack, so it reaches black as the reverb tail
  dies away. The plan
  always follows the score as it stands, with no more images than photos (the rest goes to
  the outro). The **calculator** in Plan and export works out, before you change
  anything, how many images the score holds and what's left over, and how many bars your
  photos need with the intro and outro, and says how far to trim or lengthen the score.
  It lists the sections with their lengths now and the lengths proposed, the change shared
  among them in proportion (whole 4-bar phrases where the sections are made of them, any
  bars over going to the last section); change any proposal there, or leave them and drag
  the sections' ends in the piano roll. **Set the sections to these** applies them: each
  section whose length changes is written again at its new length. The calculator weighs
  the **tempo** as well: in bars per image the tempo sets how long each image is on screen
  (the sections how many the score holds), so it offers the tempo that gives the time you
  aim for, with a table of each bar count's time now, the tempo for your time and the
  bars your photos would need; in seconds it offers, where one exists, the tempo at which
  the sections as they are hold every photo, instead of changing them. With the photos
  counted, the plan places every one the score holds, following the music only in where
  each change falls.
- **Gallery as HD video** (Export menu, or Plan and export): the same photos over the
  soundtrack as a finished 1920×1080 H.264 video (`.mp4` or `.mov`, AAC audio, Rec. 709)
  at the gallery's frame rate (24, 25, 30, 50 or 60). Every frame is drawn from the same
  schedule as the Resolve timeline, so each change and fade lands on the frame the
  timeline and markers give, and the video runs exactly as long as the soundtrack with
  its tail (to the next whole frame). Photos are scaled to fit, with black bars around
  portrait or odd-shaped ones. It needs **ffmpeg**, which isn't bundled (macOS:
  `brew install ffmpeg`; Windows: `ffmpeg.exe` beside Kibble-481 or on the `PATH`; or
  set `KIBBLE_FFMPEG`).
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
kibble-481-cli video rain.kibble rain.mp4 --photos ~/Pictures/tokyo --fps 25 --transition inout --fade-in 1.5 --fade-out 0.8
kibble-481-cli patches | fonts
```

## Layout

| Path | What |
|---|---|
| `crates/dd-dsp` | the polysynth (`poly.rs`, `patch.rs`), the city kit and its player, reverb, echo, ensemble. No UI. |
| `crates/dd-seq` | project model, timeline, the engine and mixer (live and offline), SoundFont instruments, recording and quantise, MIDI and WAV |
| `crates/dd-gallery` | slideshow timing, the frame schedule (`shots.rs`), Resolve FCPXML/EDL, the `.kibble` document, the Resolve package export, the HD video export (`video.rs`, via ffmpeg) |
| `apps/dd-desktop` | the egui app (`kibble-481`); `about.rs` (version, details, licences), `update.rs` (the update check) |
| `apps/dd-cli` | `kibble-481-cli` |

`./tools/bundle_macos.sh` builds `dist/Kibble-481.app` with the icon, both binaries and the
SoundFonts. After changing dependencies, `./tools/gen_licenses.py` rewrites the About
window's list of third-party crates and their licences (a test checks it matches
`Cargo.lock`).

## About

The ⓘ button opens About: the version and the git commit it was built from, a quick
guide to playing and recording (keys, MIDI, the ribbon, the roll), what the app
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
