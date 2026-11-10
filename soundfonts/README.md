# SoundFonts

Every font here may be redistributed with the app, and music made with it may be used
commercially. Each licence travels with its font.

| File | What it gives Kibble-481 | By | Licence |
|---|---|---|---|
| `GeneralUser-GS.sf2` | the General MIDI set: piano, koto, shakuhachi, shamisen, choir, strings, timpani, taiko, drum kits | S. Christian Collins | GeneralUser GS licence v2.0: free for any music, private or commercial; may be bundled (`GeneralUser-GS-LICENSE.txt`) |
| `Ambient-Electronic.sf2` | CS-style brass, pads, Smoky Lead, Crystal Bells, and the Trailer Kit and Trailer FX, which share the city kit's keys | synthesised by [Ambient MIDI Generator](https://github.com/catskinroyale/ambient-midi-generator); no recorded samples | MIT (`Ambient-Electronic-LICENSE.txt`) |
| `FreePats-SynthBrass1.sf2` | analogue synth brass | FreePats (Roberto Gordo Sáez) | CC0 1.0 (`CC0-1.0.txt`) |
| `FreePats-SynthBrass2.sf2` | FM ("DX7-style") synth brass | FreePats | CC0 1.0 |
| `FreePats-SweepPad.sf2` | a sweeping analogue pad | FreePats | CC0 1.0 |
| `FreePats-NewAge.sf2` | a bright bell pad | FreePats | CC0 1.0 |
| `FreePats-TenorSaxophone.sf2` | a sampled tenor saxophone (Versilian VCSL samples) | FreePats, Versilian Studios | CC0 1.0 |

Each FreePats font's own readme is beside it. Their archives' SHA-256s are in
`docs/decisions.md` (D4).

## Optional extras

`tools/fetch_soundfonts.sh` downloads more into `soundfonts/extra/` (not committed), all
pinned by SHA-256: FreePats FM-Piano1, Tubular Bells, Timpani, Synth Pad Choir and the
Kawai upright piano (all CC0). With `--musescore` it also fetches MuseScore_General.sf2
(MIT, 216 MB, with its licence file), the most complete licence-clean set of Japanese
instruments, gongs and choirs.

## Rejected

Timbres of Heaven (no redistribution without permission, non-commercial), Arachno and
SGM (no clear licence, samples of unknown origin), Sonatina Symphonic Orchestra (SFZ,
CC Sampling Plus), anything in SF3 (rustysynth can't read it) and the Salamander grand
piano SF2 (CC-BY, but 296 MB).
