# README Refresh Design

## Goal

Refresh the fork's root README so visitors can understand the added L06A
audio and wake-takeover capabilities without presenting the Server as tied to
specific hardware or an operating system.

## Scope

- Remove the archive or stopped-maintenance warning from the top of the README.
- Preserve the original Open-XiaoAI introduction, demonstrations, related
  projects, disclaimer, agreement, and upstream author attribution.
- Do not mention N100 or any other Server hardware model.
- Do not claim that the upstream repository has stopped maintenance.
- Describe the Server as runnable on any host that satisfies its dependencies.

## Fork Capabilities

Add a concise section describing the fork's material differences:

- an L06A-specific `LD_PRELOAD` PCM tap for processed 16 kHz mono S16LE audio;
- a Rust Client `pcm_tap` recording backend that forwards audio through the
  existing WebSocket protocol;
- preservation of the speaker's native `小爱同学` KWS and native `我在`;
- immediate suppression of the remaining Xiaomi conversation after the native
  wake marker;
- Server-side VAD and local ASR, with Bridge-side KWS disabled for this flow;
- fail-closed `blocked` and validated `takeover` modes with health checks and
  rollback support.

The README must not imply that Xiaomi ASR supplies the user's question in this
flow.  The question is captured from the PCM tap and transcribed on the Server.

## Device Compatibility

Keep the existing LX06/OH2P quick-start path intact.  Present the L06A work as
a separate, firmware-specific path verified only for L06A firmware 1.88.55.
Do not suggest flashing LX06/OH2P firmware or instructions onto L06A.

## README Structure

1. Project title and original introduction.
2. A new fork-capabilities section near the introduction.
3. A compact architecture flow for the L06A path.
4. Existing demonstrations.
5. Quick start split into the original LX06/OH2P path and the L06A custom path.
6. Existing related projects, references, disclaimer, agreement, and license.

The L06A path links to repository-native source and design files.  It does not
publish unverified one-command deployment instructions.

## Validation

- Search the README for `N100`; there must be no match.
- Confirm the stopped-maintenance warning and equivalent wording are absent.
- Confirm the Server is not described as requiring a particular OS or machine.
- Confirm native KWS/native `我在`, Server VAD/local ASR, and PCM tap ownership
  are stated consistently.
- Confirm all added relative links resolve to tracked repository files.
- Run `git diff --check` before committing.
