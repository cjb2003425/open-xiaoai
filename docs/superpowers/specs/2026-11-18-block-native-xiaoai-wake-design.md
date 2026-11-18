# Block Native “小爱同学” Wake Design

## Goal

On the L06A speaker, saying “小爱同学” must produce no wake response while the custom “你好小黑” path continues receiving processed 16 kHz mono PCM from `xaudio_wrapper_dnn`.

## Scope

- Extend the existing reversible `LD_PRELOAD` PCM tap.
- Preserve the original DNN call, return code, ABI checks, ring format, and Client/Bridge data path.
- Block the native wake path only when an explicit environment switch is enabled.
- Do not stop or disable the PNS service, `mipns-xiaomi`, or the DNN engine.
- Do not add an ALSA capture fallback.

## Data Flow

For each successful `xaudio_wrapper_dnn` call with the validated 640-byte input/output ABI:

1. Call the firmware's original `xaudio_wrapper_dnn` implementation.
2. Copy the unmodified processed PCM output into the shared ring.
3. If `OPEN_XIAOAI_BLOCK_NATIVE_WAKE=1`, overwrite the firmware-facing output buffer with zero samples.
4. Return the original function result and output byte count.

The ordering is mandatory: the custom Client sees the real processed PCM, while the native downstream wake detector sees silence.

## Configuration

The hook reads `OPEN_XIAOAI_BLOCK_NATIVE_WAKE` once during initialization. Blocking is enabled only for the exact value `1`; an absent or different value preserves the current tap-only behavior.

The L06A manager overlay injects both:

- `LD_PRELOAD=<tap library>`
- `OPEN_XIAOAI_BLOCK_NATIVE_WAKE=1`

The environment remains scoped to `mipns-xiaomi`. The hook continues to remove `LD_PRELOAD` from its process environment so child processes do not inherit the library.

## Failure and Rollback

- Firmware and component hash checks remain mandatory.
- Invalid DNN calls are counted as ring errors and are not modified.
- If the tap health check fails, the manager restores the native PNS service as it does today.
- `l06a_pcm_tap.sh stop` removes the overlay and restarts native PNS without the hook or blocking switch, fully restoring “小爱同学”.
- The blocking switch does not change the shared-ring ABI, so the existing Client remains compatible.

## Tests

### Host-side hook test

- Feed a known non-zero 640-byte output through the test hook.
- Verify the ring contains the original bytes.
- With blocking disabled, verify the caller output remains unchanged.
- With blocking enabled, verify the caller output is all zero.
- Verify invalid ABI/error calls are not zeroed.

### Shell/overlay test

- Verify the generated PNS overlay contains exactly one preload assignment and one blocking assignment in the `mipns-xiaomi` instance.
- Preserve existing syntax, manifest, firmware-hash, health-check, and rollback tests.

### Temporary device validation

Before persistence:

1. Start the temporary tap and confirm the ring advances with non-silent PCM.
2. Say “小爱同学” three times; the speaker must not wake or answer.
3. Say “你好小黑”; the custom KWS must trigger and enter its configured direct-AI path.
4. Stop the temporary tap and verify native “小爱同学” is restored, demonstrating rollback.

Persistence is allowed only after steps 1–3 pass. The rollback check may then be followed by re-enabling the validated blocking configuration.
