# L06A DNN PCM Tap Implementation Plan

## Scope

Implement the approved L06A DNN PCM tap without changing the Bridge protocol
or disabling Xiaomi's native assistant. Deployment remains temporary until
the device ABI, audio format, native behavior, and rollback are verified.

## Tasks

1. Add a versioned C ring layout and an ARM32 preload probe/tap that always
   calls the original `xaudio_wrapper_dnn` unchanged.
2. Add host-side C tests for publication, invalid block shapes, pass-through
   return values, and bounded ring size.
3. Add a Rust `pcm_tap` reader with tests for header validation, latest-frame
   start, wraparound, overwrite detection, stale producer detection, and
   Bridge-sized aggregation.
4. Select the recording backend explicitly with
   `OPEN_XIAOAI_AUDIO_SOURCE=pcm_tap`; never fall back to ALSA.
5. Add reversible L06A manager and launcher templates with exact firmware
   hashes, PNS overlay ownership checks, health checks, disabled-marker
   circuit breaking, and rollback.
6. Build the ARM32 hook and ARMv7 Client with the existing Linaro toolchain;
   run all local tests and inspect binaries.
7. Install only temporary assets on the speaker, activate the overlay, and
   verify ring health plus a short PCM capture. Restore automatically on any
   failure.
8. Verify native XiaoAI after the temporary load. If audio validation passes,
   install persistent assets with timestamped backups and an idempotent
   launcher.
9. Verify no Client-owned `arecord`, Client/PNS restart behavior, Bridge
   connection, custom wake-to-Hermes flow, native XiaoAI, and rollback.

## Stop conditions

- Do not persist if firmware hashes, DNN ABI, sample format, or native XiaoAI
  checks fail.
- Do not substitute ALSA for a failed tap.
- Restore the original PNS service immediately if the hook fails to load or
  the ring does not advance.
- Preserve the existing untracked `packages/client-rust/target-linaro/`
  directory and all unrelated user changes.
