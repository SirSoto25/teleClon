#!/usr/bin/env bash
# Regenera los ficheros de prueba de tc-audio (tono de 440 Hz, 1 s). Requiere ffmpeg con libopus,
# libmp3lame y libvorbis. Los resultados se versionan: las pruebas no necesitan ffmpeg.
set -euo pipefail
out="$(dirname "$0")/../../crates/tc-audio/tests/fixtures"
mkdir -p "$out"
sine() { echo "sine=frequency=440:duration=1:sample_rate=$1"; }
ff() { ffmpeg -v error -y -f lavfi "$@"; }
# Nota de voz como las de Telegram: OGG/Opus mono.
ff -i "$(sine 48000)" -c:a libopus -b:a 24k -application voip -ac 1 "$out/voice.ogg"
ff -i "$(sine 48000)" -ac 2 -c:a libopus -b:a 48k "$out/stereo.opus"
ff -i "$(sine 22050)" -c:a libmp3lame -b:a 32k -ac 1 "$out/tone.mp3"
ff -i "$(sine 22050)" -c:a aac -b:a 32k -ac 1 "$out/tone.m4a"
ff -i "$(sine 8000)" -c:a flac -ac 1 -sample_fmt s16 "$out/tone.flac"
ff -i "$(sine 22050)" -c:a libvorbis -q:a 0 -ac 1 "$out/tone.ogg"
ls -la "$out"
