# Changelog

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
