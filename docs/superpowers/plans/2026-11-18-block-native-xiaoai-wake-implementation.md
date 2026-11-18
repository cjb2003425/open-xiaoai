# Block Native “小爱同学” Wake Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the processed-PCM tap used by “你好小黑” while making the native “小爱同学” detector receive silence.

**Architecture:** The preload wrapper continues to call the firmware DNN first, publishes a byte-for-byte copy of valid output to the existing ring, and only then conditionally zeros the firmware-facing buffer. The L06A procd overlay enables this behavior with a process-scoped environment switch; removing the overlay restores native behavior.

**Tech Stack:** C11-compatible GNU C, `LD_PRELOAD`, OpenWrt procd shell, ARMv7 Linaro GCC, existing Rust Client and mmap ring.

**Spec:** `docs/superpowers/specs/2026-11-18-block-native-xiaoai-wake-design.md`

## Global Constraints

- The original `xaudio_wrapper_dnn` must still be called exactly once and its return code/output byte count must be preserved.
- Publish real PCM before modifying the native output buffer.
- Enable blocking only for exact environment value `OPEN_XIAOAI_BLOCK_NATIVE_WAKE=1`.
- Never zero invalid ABI calls or failed DNN calls.
- Preserve the ring ABI and do not introduce ALSA fallback.
- Keep deployment temporary until device validation succeeds; rollback must restore native PNS.
- Preserve all unrelated working-tree changes and `packages/client-rust/target-linaro/`.

## Review Focus

- A null output pointer must increment the existing ring error count and must never be written.
- Null `output_bytes`, non-640-byte input/output, and nonzero DNN results must remain unmodified.
- An unset, empty, or non-`1` switch must preserve the current pass-through behavior.
- Ring publication must retain the original samples even when native output is zeroed immediately afterward.
- Repeated manager starts/stops must retain one PNS process and restore an unhooked native process on stop.

---

### Task 1: Publish-Then-Block Hook Behavior

**Files:**
- Modify: `packages/client-rust/tap/xaudio_pcm_tap.c`
- Modify: `packages/client-rust/tap/test_pcm_tap.c`
- Modify: `packages/client-rust/tap/build.sh`

**Interfaces:**
- Consumes: existing `publish_pcm(const void *, int, int, int)` and 640-byte validated DNN ABI.
- Produces: internal `process_output(void *output, int input_bytes, int output_bytes, int result)` and test-only setters/wrappers under `PCM_TAP_TEST`.

- [ ] **Step 1: Extend the host test with failing pass-through and blocking cases**

Add assertions that test mode can set blocking off/on; `process_output` preserves the caller buffer when off, publishes original nonzero samples then zeros the caller buffer when on, and leaves null/invalid/error outputs untouched while retaining existing error accounting.

- [ ] **Step 2: Run the host test and confirm the new test API is missing**

Run: `rm -rf /tmp/open-xiaoai-tap-build && packages/client-rust/tap/build.sh /tmp/open-xiaoai-tap-build`

Expected: compilation/link failure for the new test-only setter or processor.

- [ ] **Step 3: Implement switch parsing and publish-then-block processing**

In `xaudio_pcm_tap.c`, read the switch once during initialization with exact string comparison to `1`. Route the real wrapper through `process_output`; it must call `publish_pcm` first and then zero exactly 640 output bytes only for a successful validated call. Expose only the minimal test API under `PCM_TAP_TEST`.

- [ ] **Step 4: Run hook tests and static checks**

Run: `rm -rf /tmp/open-xiaoai-tap-build && packages/client-rust/tap/build.sh /tmp/open-xiaoai-tap-build && git diff --check`

Expected: host C test passes, ARM hook/capture build succeeds with `-Werror`, and diff check is clean.

---

### Task 2: L06A Overlay Configuration and Temporary Device Validation

**Files:**
- Modify: `packages/client-rust/tap/l06a_pcm_tap.sh`
- Verify: `/tmp/open-xiaoai-tap-stage/xaudio_pcm_tap.so` on `10.0.0.5`
- Verify: `/etc/rc.d/S55pns` temporary bind overlay on `10.0.0.5`

**Interfaces:**
- Consumes: Task 1 hook honoring `OPEN_XIAOAI_BLOCK_NATIVE_WAKE=1`.
- Produces: a PNS instance environment containing both the preload library and native-wake blocking switch.

- [ ] **Step 1: Write the failing overlay assertion**

Extend the packaging check to require the overlay renderer to emit exactly one `procd_set_param env OPEN_XIAOAI_BLOCK_NATIVE_WAKE=1` line beside the preload assignment.

- [ ] **Step 2: Run the build and confirm the assertion fails**

Run: `rm -rf /tmp/open-xiaoai-tap-build && packages/client-rust/tap/build.sh /tmp/open-xiaoai-tap-build`

Expected: manager-switch assertion fails.

- [ ] **Step 3: Inject the blocking switch into the mipns procd instance**

Update `render_overlay()` so its existing single insertion point prints the blocking assignment immediately after `LD_PRELOAD`. Do not alter ownership, firmware hash, health, or rollback behavior.

- [ ] **Step 4: Rebuild and inspect artifacts**

Run: `rm -rf /tmp/open-xiaoai-tap-build && packages/client-rust/tap/build.sh /tmp/open-xiaoai-tap-build && file /tmp/open-xiaoai-tap-build/xaudio_pcm_tap.so && sha256sum -c /tmp/open-xiaoai-tap-build/manifest.sha256`

Expected: ARM 32-bit shared object, passing host tests, and valid manifest.

- [ ] **Step 5: Replace only temporary speaker assets and restart the temporary tap**

Upload the rebuilt hook and manager to `/tmp/open-xiaoai-tap-stage`, run `l06a_pcm_tap.sh start`, and verify exactly one `mipns-xiaomi` PID has the hook mapped, the generated overlay contains both environment assignments, and `capture_pcm --check` passes.

- [ ] **Step 6: Verify custom audio remains non-silent**

Capture five seconds from the ring and verify 16 kHz mono S16LE data has nonzero RMS/peak; confirm the temporary Client reconnects to Bridge and local KWS continues receiving audio.

- [ ] **Step 7: Perform the spoken acceptance test**

Ask the user to say “小爱同学” three times and then “你好小黑”. Expected: no native wake/answer for all three native attempts, followed by one custom KWS trigger.

- [ ] **Step 8: Prove rollback before persistence**

Run the manager `stop`; verify one unhooked native PNS process, no ring/overlay, and native “小爱同学” works again. Re-enable the validated temporary blocking setup only after rollback succeeds.

- [ ] **Step 9: Commit the implementation after acceptance**

Stage only the tap hook, tests, build script, manager, plan, and directly required existing PCM-tap implementation files. Exclude `target-linaro/` and unrelated changes. Commit with `feat: block native XiaoAI wake behind PCM tap`.
