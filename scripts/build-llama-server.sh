#!/usr/bin/env bash
# Compila llama-server (llama.cpp) come unico eseguibile statico e lo mette dove Tauri cerca i sidecar:
# src-tauri/binaries/llama-server-<target triple>. Serve al modello di pulizia locale (src-tauri/src/llm/local.rs).
# Uso: scripts/build-llama-server.sh [target triple]   (di default quello di rustc)
set -euo pipefail

TAG=v1.0.0 # llama.cpp: aggiornare insieme, dopo aver rifatto l'esame del modello
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TRIPLE=${1:-$(rustc -vV | sed -n 's/^host: //p')}
SRC=${LLAMA_CPP_DIR:-$ROOT/src-tauri/target/llama.cpp-$TAG}
OUT=$ROOT/src-tauri/binaries
EXE=""
[[ $TRIPLE == *windows* ]] && EXE=.exe

# Build universale del Mac (release): le due architetture unite con lipo, come vuole Tauri
if [[ $TRIPLE == universal-apple-darwin ]]; then
  "$0" aarch64-apple-darwin
  "$0" x86_64-apple-darwin
  lipo -create -output "$OUT/llama-server-$TRIPLE" "$OUT/llama-server-aarch64-apple-darwin" \
    "$OUT/llama-server-x86_64-apple-darwin"
  ls -lh "$OUT/llama-server-$TRIPLE"
  exit 0
fi

[ -d "$SRC" ] || git clone -q --depth 1 --branch "$TAG" https://github.com/ggml-org/llama.cpp "$SRC"

ARGS=(-DBUILD_SHARED_LIBS=OFF -DLLAMA_CURL=OFF -DLLAMA_OPENSSL=OFF -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_EXAMPLES=OFF
  -DLLAMA_BUILD_SERVER=ON -DLLAMA_USE_PREBUILT_UI=OFF -DLLAMA_BUILD_UI=OFF -DGGML_NATIVE=OFF
  -DCMAKE_BUILD_TYPE=Release)
# niente HTTPS né interfaccia web (il server ascolta solo in locale e l'app usa solo l'API) e niente -march=native: il file gira sui PC degli utenti
case $TRIPLE in
  # cmake di Homebrew può girare sotto Rosetta: architettura e processore vanno forzati, altrimenti
  # ggml compila per x86_64
  aarch64-apple-darwin) ARGS+=(-DCMAKE_OSX_ARCHITECTURES=arm64 -DCMAKE_SYSTEM_PROCESSOR=arm64 -DGGML_CPU_ARM_ARCH=armv8.4-a -DGGML_METAL=ON
    -DGGML_METAL_EMBED_LIBRARY=ON -DCMAKE_OSX_DEPLOYMENT_TARGET=11.0) ;;
  x86_64-apple-darwin) ARGS+=(-DCMAKE_OSX_ARCHITECTURES=x86_64 -DCMAKE_SYSTEM_PROCESSOR=x86_64 -DGGML_METAL=OFF
    -DCMAKE_OSX_DEPLOYMENT_TARGET=11.0) ;;
  # ponytail: Windows e Linux solo CPU per ora (con ggml che sceglie le istruzioni a runtime);
  # Vulkan quando il pacchetto G porta la GPU anche a Whisper
  *) ARGS+=(-DGGML_BACKEND_DL=OFF -DGGML_CPU_ALL_VARIANTS=OFF) ;;
esac

cmake -S "$SRC" -B "$SRC/build-$TRIPLE" "${ARGS[@]}" >/dev/null
cmake --build "$SRC/build-$TRIPLE" --config Release --target llama-server -j >/dev/null
mkdir -p "$OUT"
BIN=$(find "$SRC/build-$TRIPLE" -name "llama-server$EXE" -type f | head -1)
cp "$BIN" "$OUT/llama-server-$TRIPLE$EXE"
ls -lh "$OUT/llama-server-$TRIPLE$EXE"
