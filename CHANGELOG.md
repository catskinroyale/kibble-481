# Changelog

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
