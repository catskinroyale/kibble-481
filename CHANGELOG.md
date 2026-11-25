# Changelog

## 0.13.2 — 2026-11-25

- **Reorder images**: a new step in Plan and export opens a pane of the folder's photos as
  thumbnails; drag them into the order you want (your strongest first, say), or
  right-click one to send it to the front or the end, and **Save and close**. The Resolve
  timeline and the HD video both use that order. **Your order** joins By name and By date
  under Photos; photos added to the folder later go at the end, and choosing another
  folder starts again from its own order.

## 0.13.1 — 2026-11-25

- **Fade out: the first image can fade in.** With the Fade out transition, a **First
  image fade in** slider (shown only then) fades the first photo up from black; the
  others still cut in.
- **Stretch final fade**: the last photo's fade out runs on to the end of the soundtrack,
  so the picture reaches black as the reverb tail dies away (Fade out, and Fade in and
  out). In the video and the Resolve timeline alike. On the command line:
  `--first-fade-in <secs>` and `--stretch-final-fade`.

## 0.13.0 — 2026-11-25

- **Gallery as HD video**: export the photos over the soundtrack as a finished 1920×1080
  H.264 video (`.mp4` or `.mov`) at the gallery's frame rate, from the Export menu or
  Plan and export (and `kibble-481-cli video`). Each change and fade lands on the frame
  the Resolve timeline gives, and the video is exactly as long as the soundtrack. Needs
  ffmpeg installed.
- **Three new transitions**: every photo **fades in** from black, **fades out** to
  black, or **fades in and out**, each fade with its own length. The Resolve timeline
  has them too. On the command line: `--transition`, `--fade-in`, `--fade-out`.

## 0.12.0 — 2026-11-24

- **Pachinko**, a note gate tracks can be routed through (Effects tab), after a eurorack
  random-gate module: each step (1/16 to a bar) a ball falls left or right, **Chance**
  of the time left. A track routed to a unit's left output plays the steps whose ball
  fell left; one on its right output, the rest, so two tracks take turns. **Déjà vu**
  loops the last **Length** balls (1-16), all the way round for ever; **Pattern** drops a
  different set. Two units, A and B. The same settings always play the same notes, live,
  rendered and in the exported MIDI; the roll fades the notes it holds back. The engine
  now routes each track's notes on their own; the reverb and echo sends are unchanged.
- **Density and complexity dials on every track**, beside mute and solo (now stacked):
  density leaves out the weakest notes or fills the gaps; complexity plays fewer notes of
  each chord and arpeggio or adds octaves (ghost notes on drums). The written notes stay
  as they are; the roll shows what's left out and added, and export plays it as heard.
- **Sections on the ruler**: right-click a section for its menu again (dragging it had
  taken over every button); left-drag moves it, a click moves the playhead.
- **A new drone score**, a sixteenth style:
  - **Strata** (50 bpm, Phrygian): vast, slow and heavy, no beat. A drone from far below
    that never stops, a second drone whose overtones wander like a slow whistle, a choir
    of drones, a swelling sub, the earth groaning, distant metal, a bell far off, and one
    great slow climb.
- **Five new synth drones** (Drones, and alternatives for any drone track): Basalt Drone
  (deep, dark and heaving), Overtone Drone (a resonant filter that sings the harmonics as
  brightness sweeps it), Radiant Drone (warm, beating sines), Prism Drone (high, bell-like
  FM tones) and Buzz Drone (a plucked string that keeps buzzing).

## 0.11.0 — 2026-11-24

- **Cinder**, a fourteenth style: dark ambient electronica after the end of the world,
  80 bpm. Tape hiss and a far-off radio, a long drone, distant voices, wide detuned pads,
  a sparse half-time broken beat with a muffled snare over a deep sub, hollow synth
  stabs, and a few long notes of melody with a lot of empty space.
- **Filmstrip**, a fifteenth: nostalgic downtempo electronica through a worn video tape,
  90 bpm in Dorian. Birdsong and a playground, a music box, a lazy swung beat, electric
  piano and tape strings, a wobbling analogue lead, glockenspiel, and children's voices
  singing a chant; the tape winds down at the end.
- **Tape**: a new effect (Effects tab, beside Ensemble) puts the whole mix on worn tape:
  wow and flutter, saturation and a dulled top. Off unless turned up.
- **New sounds**: Ashen Pad, Hollow Stab, Wobble Lead and Tape Strings (synth), Voice
  Oohs (GM), and in the city kit Radio Static, Vinyl Crackle, Birdsong and Playground.
  Tape hiss now restarts when you seek into it.

## 0.10.2 — 2026-11-23

- **Swells glide**: expression (CC11) and brightness (CC74) now glide from one written
  point to the next instead of jumping. The composer writes each swell as a handful of
  points, so on the horns the filter snapped open at the swell's peak (up to 20 dB in an
  instant, heard as a sudden re-emphasis part way through a held chord) and the level rose
  in 3-5 dB steps. Pieces saved before glide too; other controllers still switch at once.

## 0.10.1 — 2026-11-23

- **Section buttons in their own row**: new notes, copy before, copy after and delete now
  sit in an Edit row above the section names, always shown, so dragging a section to move
  it no longer takes their clicks. From the top: playhead, Edit, Sections, Bars.
- **Doubled notes fixed**: Overcity's and Vigil's Night shift wrote some brass and choir
  notes twice at the same moment (an octave above a note the chord already held). The copy
  cut the note off almost at once, and on a SoundFont doubled it. Those notes are now
  written once, and playback treats any key struck twice at once on a track as one note,
  so pieces saved before play correctly too.

## 0.10.0 — 2026-11-23

- **Vigil**, a thirteenth style: Overcity watched over gently. Overcity's theme, chords,
  form and great swell on other instruments: soft sampled French horns for the brass, two
  drones (the second following the chords where the choir or strings were), and a grand
  piano for the rippling glass. `kibble-481-cli compose --style vigil`. Grand Piano is
  also a Pluck sound in the Sound tab.
- **Added tracks always have notes**: a role the style doesn't write, or any role in a
  piece that wasn't composed, gets a part written over the piece's own chords.
- **Gallery: bars per image.** Images can be a set number of bars each (2, 4, 8…). The
  calculator now weighs the tempo as well as the sections: the tempo that gives the time
  on screen you aim for, a table of bar counts, and, in seconds, a tempo at which the
  sections as they are hold every photo. `resolve --bars-per-image <n>` on the command line.
- **Piano roll**: zoom out, zoom in and whole-piece buttons beside the tracks' + Add; the
  playhead's handle in a strip of its own above the sections, to click or drag; **⏮ ⏭** in the transport
  (and **[** **]**) jump between sections; drag a section on the Sections strip to move
  it before or after another, notes and all.

- **Nebula**, a twelfth style: a 1970s analogue electronic ambient score, dreamy and
  cosmic, synthesisers only. Wind, breath and water-drop blips over a drone with a
  sweeping resonant filter; a rolling sequencer and a round pulsing bass; a simple melody
  of long notes that repeats and grows over string-machine chords; whooshes, sparkles and
  echoing blips across the stereo field; building in density and brightness, then
  dissolving back into the wind. 96 bpm, always minor; `kibble-481-cli compose --style
  nebula`. Eight new synth patches, also in the Sound tab: **Solar Wind** (Pad),
  **Bubble Drops** and **Cosmic Sequence** (Pluck), **Round Pulse Bass** (Bass),
  **Starlight Lead** and **Tape Flutes** (Lead), **String Machine** (Strings) and
  **Modular Drone** (Drone). The other styles are unchanged.

## 0.9.1 — 2026-11-19

- **Orbit** keeps the brief's "no sudden changes" in any arrangement: each layer's level
  can no longer move faster than a full fade in about four seconds, so sections in any
  order (🎲 New piece, or your own) flow into each other, and the peak's last tubular
  bell now rings on into the Fade instead of being cut off as it starts.

## 0.9.0 — 2026-11-19

- **Orbit**, an eleventh style: a hypnotic minimalist score in 15/8 (seven and eight
  quavers) in A minor. One seeded two-bar figure on a soft, felted piano loops all the
  way through; a bass guitar, a glockenspiel, a drawbar organ and an acoustic guitar
  join one at a time, each looping its own pattern, while the harmony drifts towards C
  major without resolving. It builds by accumulation to a peak of tubular bells over the
  full ensemble, then the layers leave one by one, back to the piano alone. Choosing it
  sets 92 bpm, A minor and 15/8; 15/8 joins the Metre list, and choosing another style
  afterwards puts the metre back to 4/4. `kibble-481-cli compose --style orbit` (the CLI
  now writes a style in its own metre). New sounds: **Drawbar Organ** (Keys), **Finger
  Bass** (Bass) and **Steel Guitar** (Pluck). The other styles are unchanged.
- With the mixer open and nothing playing, the app no longer redraws 60 times a second
  (its meters ask for frames only while they move): idle CPU falls from about a third of
  a core to about a tenth.
- **Return** plays with more grace: the piano and violas phrase the theme (swelling
  towards its peak, lighter on short notes, easing into the cadences, a breath before
  the climax), the violas swell on long notes with a vibrato that blooms, and the piano
  breaks its chords into gentle pedalled arpeggios instead of repeated block chords.
  Other styles are unchanged.
- **Chamber**, a tenth style: a chamber orchestra with a dark, sci-fi noir twist. A solo
  violin, violas, cellos, double bass, pizzicato, clarinet (or cor anglais), bassoon,
  horn, harp, celesta and piano play over a glass drone and a sub that beats like a
  heart. The harmony is chromatic noir (Neapolitan, chromatic mediants, the flat sixth)
  turning on a dominant seventh, and the piece ends on a minor chord with a major seventh
  and a ninth. One seeded theme passes from the clarinet to the violin to three octaves
  of strings at the climax. 62 bpm; `kibble-481-cli compose --style chamber`. A solo
  **Violin** joins the strings' sounds in the Sound tab. The other styles are unchanged.
- **Return**, a ninth style from the brief "Homeward (Dawn Return)": Homeward's theme
  and story at 66 bpm in E minor, with its instruments chosen: a soft electric piano
  opens, violas carry the theme, a clarinet varies it answered by cellos, full strings
  peak, and the solo piano brings it home to a long-held E minor chord with a ninth. Its
  lift reaches G major (the relative major). Choosing it sets E minor as well as its
  tempo. `kibble-481-cli compose --style return`. Homeward and the other styles are
  unchanged.
- The gallery calculator lists the sections, how long each is now and how long it would
  be to fit the photos: the change shared among them in proportion, in whole 4-bar
  phrases where they're made of them, the bars left over going to the last section. Each
  proposal can be changed by hand (the total shows against what the photos need), or
  left while you drag sections in the piano roll. **Set the sections to these** applies
  them; before, lengthening an edited piece only held its last notes on to the new end.
- Following the music's accents no longer holds fewer photos than the calculator counted:
  with the photos known, the plan places all the score holds and the accents only move
  where each change falls.

## 0.8.0 — 2026-11-17

- **Homeward**, an eighth composer style: a slow, warm, quietly hopeful cinematic score
  built on one singable eight-bar theme in a minor key, written by the seed, that comes
  back in every section: on solo piano, then legato cellos, then reharmonised for a
  flute, clarinet or French horn with a countermelody in the strings, then building
  under layered strings and a warm analogue pad to one peak in full strings, and home on
  the piano to a held tonic chord. No drums or city sounds; 69 bpm;
  `kibble-481-cli compose --style homeward`. The other styles are unchanged.
- Two more CC0 SoundFonts from FreePats, bundled: a sampled **Concert Harp** (Versilian
  VCSL samples) and **Synth Strings**, a warm analogue string pad. New sounds to choose
  from: Concert Harp (Pluck), Synth Strings (Pad), Viola (Strings), Clarinet and French
  Horn (Lead).
- Tracks can be **moved** by dragging their ≡ grip in the track list, and **deleted** with
  the × by their name (⌘Z brings one back). The last track can no longer be deleted.

- **Sections are numbered within their part** (Intro 1, Intro 2, Neon 1…) unless you give
  them a name. Click a section's name in the Score tab (or right-click it on the Sections
  strip above the roll) for its menu: rename it, **change its part** (it is written again
  as the new one), new notes for it, **new notes for one track in it**, duplicate it (a
  copy after it, notes and all), copy before, delete.
- Dragging a section's end, or moving, adding or changing sections, in an edited piece
  now moves the notes with their sections, and writes a section that is new, another part
  or another length again on its own, to its new length. It used to leave a longer
  section's new bars empty and the sections after it out of step with their notes. A
  section given new notes keeps its seed when it is written again.
- The rain, and anything else held right through, keeps going through copied, deleted and
  rewritten sections instead of stopping there.
- **+ Add** is back in the track list: a new track of any role comes with its sound and
  the notes the piece's style writes for it (a second track of a role plays a line of its
  own; a role the style doesn't write comes empty). New pieces write them again.

## 0.7.1 — 2026-11-17

- Fixed a crash on macOS when a folder was dragged onto the Choose photo folder… dialog.
  Every file and folder dialog (open, save, export, photos, patches, SoundFonts) now
  opens without holding up the window: on macOS as a sheet on it.

## 0.7.0 — 2026-11-16

- **Derelict**, a seventh composer style: a ship adrift in deep space, analogue drones and
  failing machinery out of which sparse, dissonant orchestral voices surface. A quiet void
  (drone, throbbing engines, ticking, a groaning hull), creeping tension (a sub pulsing like
  failing life support, a worn pad breathing in and out with long silences between, a lone horn,
  celesta, an oboe or cor anglais fragment), tremolo strings entering one note at a time
  over a bowed double bass, a harp glissando into one brief orchestral eruption, and a
  collapse back into the drone with a ghostly muted trumpet or sax. No beat; 48 bpm;
  `kibble-481-cli compose --style derelict`. The other styles are unchanged.
- The city kit has a derelict ship's sounds: **Hull Groan** and **Pipe Clang** (tuned, an
  octave of keys each), **Steam Hiss**, **Metal Scrape**, and **Engine Throb**, **Life
  Support** (a pump), **Ticking** and **Tape Hiss** that loop while held.
- New sounds to choose from, all from the bundled GeneralUser GS: Tremolo Strings
  (Strings), Cor Anglais, Solo Oboe and Bassoon (Lead), Muted Trumpet and Baritone Sax
  (Answer), Contrabass (Bass), Trombone (Brass), Soundtrack, Metal Pad and Space Voice
  (Pad), Pizzicato Strings (Pluck), Goblin (Drone).
- Each seed draws Derelict's harp (or pizzicato) glissandi its own way: the notes they
  run through (the scale, thirds, five notes of it, or the chord), where they lie and how
  far they go, their shape and pace, and whether they double an octave down.
- Derelict's swells breathe from a steady level instead of climbing out of silence on
  every chord, and its brightness moves smoothly with them, so nothing stabs; the
  brass, pad and master go without the ensemble chorus, and the brass without echo.
- Choosing another style always brings that style's own sounds, even with Keep sounds
  lit (which still keeps them through new seeds in the same style).

## 0.6.0 — 2026-11-16

- Gallery: **exact seconds**, every image on screen for exactly the time asked (no
  variation), as well as the nearest whole bars; a choice of **crossfade**, **fade to
  black** or **cut** between images on the exported timeline; and a **calculator** in Plan
  and export: how many images the score holds and the time over, the bars your photos need
  with the intro and outro, how far to trim or lengthen, and a button to set the score's
  length. The plan now always follows the score as it stands, with no more images than
  photos, rather than a track length the score may not have.

## 0.5.0 — 2026-11-13

- **Overcity**, a sixth composer style: a vast, rain-soaked city seen from above at night.
  Horns rising on open fifths out of near-silence, a sparse glassy theme that leaps and
  falls with long silences between, a heartbeat on the sub, rippling glass, one great
  swell with deep booms, and a fade to the drone and a last distant bell. Each seed
  shapes the rippling glass its own way in each section (up, down, up and down, down
  and up, closing in, opening out or scattered, in quavers, dotted quavers or
  triplets). 60 bpm;
  `kibble-481-cli compose --style overcity`. The other styles are unchanged.
- The Score tab is in strips like the mixer's: the style as neon lamps; a Key strip with
  the scale (two across) and the mode as lamps side by side, the density dial and the seed
  (fixed width) under them; New piece, New notes and Write again as
  push buttons whose neon flares when pressed; Keep sounds a lamp that switches on and
  off; tempo and length as dials and the metre as lamps. The playing and recording guide
  is now in About (ⓘ).
- The Score tab's sections are a swim lane: the sections stacked on the left (grip, name,
  bars, ×; right-click to change the part) and the parts on the right to drag into it.

## 0.4.0 — 2026-11-13

- Each section on the Sections strip above the roll has its own buttons on hover (and a
  right-click menu): new notes for just that section in the piece's style and key, copy
  it with its notes before or after itself, and delete it with its notes. The rest of
  the piece stays as it is.
- The mixer's reverb, echo, low cut and high cut are now small dials, two by two at the
  foot of each strip, each pointing straight up at its neutral setting.
- A neon lamp on every mixer strip and track row, lit while the track is heard.
- The Effects tab is three small strips like the mixer's: Reverb, Echo and Ensemble.
  Each choice (reverb space, echo time, shimmer interval, tuning, Freeze, Ping-pong) is a
  neon lamp with its name, lit when chosen, and the settings are small dials two to a
  row as on the mixer, each straight up at its usual setting; the shimmer's own grey out
  in the other spaces.
- The synth editor is in strips like the mixer's too: Layer I and Layer II side by side
  (a lamp by each name switches it on), then Glide, LFO and Touch, every setting a small
  dial and every choice a neon lamp. The Effects strips no longer sit in a section that
  folds away.
- Tidier mixer strips (fader group centred, even spacing, a rule above the dials) and
  Composer section (a full-width style selector, aligned Key and Seed rows, equal buttons).

## 0.3.0 — 2026-11-12

- **Noir**, a fifth composer style: the melancholy of walking alone at night in music
  alone (no rain, city sounds or other effects). Minor ninths and major sevenths, an
  electric piano, a slow walking upright bass, brushes, a flickering arpeggio and a lead
  that leaves room for a voice; mixed quieter, to sit under voice-over. 64 bpm;
  `kibble-481-cli compose --style noir`.
- **Intro**, a new kind of section for every style: the drone, a thin pad and the
  theme's first notes heard alone. Noir opens with one, and 🎲 New piece now and then
  starts with one in the other styles. Pieces without one are written exactly as before.
- **Low cut** and **high cut** on every track's mixer strip (12 dB an octave), on the
  track's output so they work on SoundFont tracks too.
- A fuller **About** window, laid out like Ambient MIDI Generator's: version and build
  commit, what the app does, where its settings and SoundFonts are, every bundled licence
  with its full text, and the 494 third-party crates with theirs. **Check for Updates…**
  (and at startup, which can be turned off) asks GitHub for a newer release.
- The GM Brush Kit is one of the drum kits to choose from.

## 0.2.0 — 2026-11-12

- Composer styles: **Nocturne** (the piece as before), **Pursuit** (driven: a sequenced
  bass, an arpeggio, stabbing brass, a backbeat with taiko fills), **Blackout** (dark:
  kit drones, sub booms, distant metal, low brass and a far-off horn) and **Momento**
  (romantic: an electric piano, a swung groove and a sliding, leaping sax-like theme). They use only the
  sounds already bundled. Choosing one sets its tempo; `kibble-481-cli compose --style`.
- **Volume** bar by bar: a new lane draws each track's volume per bar, ramping between the
  bars set and holding after the last; the levels move with their sections, survive new
  seeds and styles, and are written to the MIDI file as CC7. While it plays, the mixer's
  fader shows where the volume has the track (an orange marker and a ▸ readout).
- Sections are dragged into order by their ≡ grip, and new ones dragged into place from
  the **+** buttons; the ▲ ▼ buttons are gone.
- **🎲 New piece** now writes a new arrangement of sections as well as new notes; **New
  notes** (beside it, in the Composer and in File) keeps the sections.
- Saves and exports are named with the title, the seed and a UTC time stamp.
- Clicking anywhere on a track selects it; its name is renamed by double-clicking it (or
  Rename in its menu).
- The track list no longer offers **+ Add**: pieces come with their tracks (for now).

## 0.1.1 — 2026-11-10

- The old name, DDream, is gone: the binaries are `kibble-481` and `kibble-481-cli`,
  projects save as `.kibble`, and the environment variables are `KIBBLE_SOUNDFONTS`,
  `KIBBLE_PALETTE` and `KIBBLE_SCREENSHOT*`. `.ddream` projects and MIDI files saved
  before still open exactly, `DDREAM_SOUNDFONTS` still works, and settings carry over.
- A Windows build (64-bit): `tools/bundle_windows.sh` cross-compiles it from macOS and
  zips both programs with the SoundFonts.

## 0.1.0 — 2026-11-10

First release.

### The composer

- 🎲 New piece writes a seeded neon-noir cue (key, mode, seed, density in the Score tab);
  the app opens on a new one each time. `ddream-cli compose`.
- Sections are an arrangement: Neon, Swell, Night shift and Fade, added in any order and
  as often as you like, reordered and removed in the Score tab. Drag a section's end on
  the strip above the roll to lengthen or shorten it a bar at a time; later sections move
  with it. The composer writes each kind of section its own way wherever it falls.
- A composed piece is written again to fit when the tempo, metre, length or arrangement
  change; a change of length is shared among the sections. Edited pieces keep their
  notes, and a new length holds the notes that ran to the end to the new end.
- A drone in every composed piece.

### Sound

- Two-layer, CS-80-shaped polysynth with glide, sag, ribbon, drift and touch; patches save
  as `.ddpatch`. Original factory patches across brass, pads, leads, keys, bells, plucks,
  basses and drones (among them Chrome Fanfare, Velvet Horns, Silk Strings, Night Choir,
  Lonely Horn, Ring Bells, Rain Harp, Abyss and Sodium Drone), all level-matched.
- Bells in tune: whole-number FM ratios, no detune, drift or vibrato; a test measures it.
- Tracks played by the synth, any SoundFont preset, the city kit or the drum kit. Every
  track has a role, and its sound is a dropdown under it in the track list offering the
  role's alternatives. Swapping a sound while the music plays is heard at once; stopped,
  choosing one plays a short preview in the track's register and the piece's key.
- The City track's dropdown chooses the rain (or none) and turns thunder, the machinery
  hum and the neon buzz on or off. The City and drum tracks keep their kits.
- Rain from real CC0 recordings (Freesound and Wikimedia Commons), played as recorded and
  kept going with crossfades between random stretches: quiet pitter-patter beds (a
  window-sill, drips, an umbrella, a roof, a back street) at -26 LUFS, and a few denser
  ones.
- Seven bundled SoundFonts (GeneralUser GS, Ambient Electronic, five CC0 FreePats banks);
  `tools/fetch_soundfonts.sh` for five more and MuseScore_General.

### Playing, editing and mixing

- Recording from MIDI keyboards, the computer keyboard and on-screen keys, ribbon, mod
  wheel, touch and sustain; count-in, click, loop, quantise, replace.
- Piano roll, track overview, velocity and controller lane, sections strip, undo.
- Mixer with sends to the Noir reverb, tape echo and ensemble; mute, solo, faders and
  knobs on the mixer and the track list, with the same smoothed level meters in both.

### Getting it out

- MIDI export and import, WAV and stems at 48 kHz/24-bit, and a gallery package for DaVinci
  Resolve (FCPXML timeline, EDL markers, cue list).
- Exported WAVs ring on after the music until the reverb, echo and releases have died
  away (to -84 dBFS, at most a minute); the Resolve timeline's audio clip is as long as
  the file.
- `ddream-cli` for headless rendering and export.

### Look

- Five palettes from the mood board (Sodium Amber, the default, Rain on Neon, Rust and
  Verdigris, Paperback, a light one, and Phosphor), chosen from the swatch button in the
  top bar; tracks take their colours from the palette.
- Typefaces, all SIL OFL: Michroma for the wordmark and panel headings, Share Tech Mono
  for readouts and values, VT323 for the synth's patch display, Jost for the interface,
  Sorts Mill Goudy and Cutive Mono in About.
