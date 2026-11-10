#!/bin/sh
# Download optional, licence-clean SoundFonts into soundfonts/extra/, where DDream loads
# them alongside the bundled ones. Every FreePats archive is pinned by SHA-256 and is
# CC0 (public domain); MuseScore_General is MIT (its licence file is fetched with it).
#
#   tools/fetch_soundfonts.sh            the FreePats set (about 130 MB unpacked)
#   tools/fetch_soundfonts.sh --musescore  also MuseScore_General (216 MB): koto,
#                                          shamisen, shakuhachi, taiko, choir, celesta…
set -eu
cd "$(dirname "$0")/.."
DEST=soundfonts/extra
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$DEST"

fetch() { # url sha256 name
  url=$1; sum=$2; name=$3
  if ls "$DEST"/"$name"*.sf2 >/dev/null 2>&1; then echo "have $name"; return; fi
  echo "fetching $name"
  curl -fL --progress-bar -o "$WORK/archive" "$url"
  got=$(shasum -a 256 "$WORK/archive" | cut -d' ' -f1)
  if [ "$got" != "$sum" ]; then echo "checksum mismatch for $name: $got" >&2; exit 1; fi
  mkdir -p "$WORK/x" && rm -rf "$WORK/x"/*
  bsdtar -xf "$WORK/archive" -C "$WORK/x"
  find "$WORK/x" -iname '*.sf2' -exec cp {} "$DEST/FreePats-$name.sf2" \;
  find "$WORK/x" -iname 'readme*' -exec cp {} "$DEST/FreePats-$name-README.txt" \;
}

GH=https://github.com/freepats
fetch "$GH/fm-piano1/releases/download/2019-09-16/FM-Piano1-SF2-20190916.7z" 543774e268e00eaf13968c53d9a42eada7f13564e559d51534ed552507c971ad FM-Piano1
fetch "$GH/tubular-bells1/releases/download/2024-11-30/TubularBells-SF2-20241130.7z" e62e827ccfc11d843d91d6d54048627c02dbbc5ae56e2cdc1cb8dc5b765ea337 TubularBells
fetch "$GH/timpani/releases/download/2024-08-10/Timpani-SF2-20240810.7z" 6a8dff96d9ad109c7ba6155b11f2c2dcc635850cdd43fc1db460f80c45a8e066 Timpani
fetch "$GH/synth-pad-choir/releases/download/2020-05-16/SynthPadChoir-SF2-20200516.7z" 4a6024721615d15bedff030a7e8259d380fdcef51f2a223961c69dfbbbdb39f1 SynthPadChoir
fetch "https://freepats.zenvoid.org/Piano/UprightPianoKW/UprightPianoKW-SF2-20220221.7z" 17c084c6e4205233dc49b34e4bc44a9b2d7c7a2c02b04729ecda77079b07c826 UprightPianoKW

if [ "${1:-}" = "--musescore" ]; then
  MS=https://ftp.osuosl.org/pub/musescore/soundfont/MuseScore_General
  if [ ! -f "$DEST/MuseScore_General.sf2" ]; then
    echo "fetching MuseScore_General (216 MB)"
    curl -fL --progress-bar -o "$WORK/ms.sf2" "$MS/MuseScore_General.sf2"
    size=$(wc -c < "$WORK/ms.sf2" | tr -d ' ')
    if [ "$size" != "215614036" ]; then echo "unexpected size $size for MuseScore_General.sf2" >&2; exit 1; fi
    echo "SHA-256 $(shasum -a 256 "$WORK/ms.sf2" | cut -d' ' -f1) (record it in docs/decisions.md to pin it)"
    mv "$WORK/ms.sf2" "$DEST/MuseScore_General.sf2"
    curl -fsL -o "$DEST/MuseScore_General-LICENSE.md" "$MS/MuseScore_General_License.md"
  fi
fi
echo "Done: $(ls "$DEST"/*.sf2 | wc -l | tr -d ' ') SoundFonts in $DEST"
