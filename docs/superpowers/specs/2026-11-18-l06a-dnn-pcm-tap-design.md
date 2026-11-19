# L06A DNN PCM Tap Design

## Goal

Feed the existing Open-XiaoAI Client with the continuously available,
front-end-processed microphone signal from the L06A speaker at `10.0.0.5`.
The Bridge will continue to perform local keyword spotting and ASR, then send
the recognized request to Hermes. Xiaomi's native wake word, cloud assistant,
and home-control path must continue to work normally.

The custom wake phrases remain `你好小黑` and `小黑你好`. The Bridge host is
`10.0.0.22`, and the Client continues to connect to
`ws://10.0.0.22:4399`.

## Confirmed device state

- Model: L06A
- ROM: 1.88.55, boot1/system1
- Kernel: aarch64; Xiaomi audio processes: ARM32 EABI5
- Native audio process: `/usr/bin/mipns-xiaomi`
- Native DNN library: `/usr/lib/libxaudio_engine.so`
- Exported function: `xaudio_wrapper_dnn`
- Existing Client launcher: `/data/user.sh`
- Existing Client binary: `/data/open-xiaoai/client`
- Root filesystem is read-only squashfs

The deployment must allow only these verified firmware hashes:

```text
a41638756e0a3cd557cb106f08aa7b4311cfc3ddf8569413f8754c920a474829  /usr/bin/mipns-xiaomi
2898fea85a38c15169ffe63d8534fed06c4c3298eb32668e9fbdb3f78d7199ca  /usr/lib/libxaudio_engine.so
911e6326b7862f832fc037e04eb2236b1ed38968b417647ae00cbdad0052d0ac  /etc/init.d/pns
```

## Architecture

Use an ARM32 `LD_PRELOAD` library to wrap `xaudio_wrapper_dnn()`. The wrapper
calls the original function with the original arguments, then copies the
verified processed mono PCM plane into a shared-memory ring. It does not
modify the arguments, output, return value, timing decisions, or the native
Xiaomi socket protocol.

```text
mipns-xiaomi
  |-- original xaudio_wrapper_dnn --> native Xiaomi path (unchanged)
  `-- fixed-size copy of processed PCM
          |
          v
  /tmp/open-xiaoai-pcm.ring (0600)
          |
          v
  Rust Client pcm_tap backend
          |
          v
  WebSocket record stream --> Bridge --> KWS/ASR --> Hermes --> XiaoAI TTS
```

The shared ring is preferred over a Unix socket or FIFO because the producer
must never block on, or depend on, the Client. Restarting or disconnecting the
Client must not affect `mipns-xiaomi`.

## Probe and firmware validation

No persistent loading is allowed until a temporary probe succeeds. The probe
must establish the L06A 1.88.55 ABI and data layout rather than assuming the
layout previously observed on another Xiaomi speaker model.

The temporary validation must confirm:

- the wrapper resolves and calls the real `xaudio_wrapper_dnn`;
- the observed input and output lengths are stable;
- the selected output plane is S16LE, 16 kHz, mono;
- the stream advances while the native assistant is idle;
- a captured sample contains intelligible speech with no material clipping;
- native wake, native answers, and native home control still work;
- unloading the probe restores the exact original PNS startup path.

Temporary assets live under `/tmp/open-xiaoai-tap/`. A temporary bind mount
may shadow `/etc/rc.d/S55pns`; the squashfs file itself must never be modified.
This firmware stores `S55pns` as an independent copy rather than a symlink to
`/etc/init.d/pns`, and procd registers the service under the invoked basename,
so both injection and lifecycle control must use the `S55pns` path.
The temporary test must restore the original PNS service whether validation
succeeds or fails.

## Ring format and producer behavior

The ring has a fixed, versioned header containing at least:

- magic and format version;
- producer PID;
- sample rate;
- samples per block;
- slot count;
- monotonically increasing published sequence;
- producer error counter.

Each slot uses a publish sequence that lets the consumer detect an in-progress
write, overwrite, or stale frame. The file is owned by root, has mode `0600`,
and is stored in `/tmp`. Its total mapped size should remain below about
128 KiB.

The wrapper performs only bounded validation, a fixed-size copy, and atomic
sequence publication. It never waits for a consumer and never performs
network or filesystem I/O in the audio callback after initialization. If the
ABI or block shape differs from the validated values, it increments an error
counter and passes the native call through unchanged without publishing data.

## Client recording backend

Add a `pcm_tap` backend to the Rust Client while retaining the existing ALSA
code for other devices. The L06A launcher explicitly selects `pcm_tap`.

For this backend the Client:

- accepts only 16 kHz, mono, 16-bit requests;
- opens and memory-maps the validated ring;
- starts at the latest published sequence, excluding historical audio;
- aggregates ring blocks to the buffer size requested by the Bridge;
- sends the existing `record` stream format without changing the WebSocket
  protocol;
- never starts `arecord` and never falls back to ALSA;
- stops sending and returns an error when the producer dies, the ring stalls,
  the format changes, frames are overwritten, or validation fails.

A fatal recording-source error causes the Client connection/process to end so
the existing `chat_gpt` procd service can restart it. A Client-only restart
must attach to a healthy existing ring without restarting PNS.

## Loading, persistence, and fail-closed behavior

After temporary validation succeeds, persistent components are installed
under `/data/open-xiaoai/tap/`. A manager performs the following idempotently:

1. Verify all firmware and installed-component hashes.
2. Prepare the ring and an exact PNS init-script overlay.
3. Add `LD_PRELOAD` only to the `mipns-xiaomi` procd instance.
4. Restart PNS only when the overlay or hook actually needs activation.
5. Verify the library appears in the process maps and ring sequences advance.
6. Start the Client only after the tap health check passes.

`/data/user.sh` invokes the manager before executing the Client. Its previous
contents are backed up before replacement. Repeated invocations are safe and
do not restart a healthy PNS process.

This deployment is fail-closed for Hermes audio. If tap startup validation
fails, the manager removes the PNS overlay, restarts the unmodified native PNS
service, records a disabled marker and diagnostic log, and does not start the
Hermes audio Client. It must not use the existing ALSA `noop` path as a
fallback. The disabled marker prevents procd respawn from repeatedly
restarting PNS.

Failure of the tap must not disable or degrade Xiaomi's native assistant.

## Rollback

Before persistent installation, save private timestamped backups of
`/data/user.sh` and every replaced file under `/data/open-xiaoai/tap/backup/`.
Provide one rollback command that:

1. stops the Client;
2. removes the temporary PNS bind mount;
3. restores the original PNS service and restarts it;
4. restores the previous `/data/user.sh`;
5. removes active tap state and its disabled marker;
6. keeps the backup and diagnostic logs for inspection.

No Xiaomi firmware binary or squashfs file is overwritten.

## Testing and acceptance

Automated tests must cover ring publication, wraparound, in-progress slots,
overwrites, stale producers, invalid headers, unsupported audio formats,
buffer aggregation, and the explicit absence of ALSA fallback. The wrapper
tests use a fake original DNN function and verify that its inputs, outputs,
return value, and call count are unchanged.

Live acceptance requires all of the following:

1. No `arecord` process is started by the Client.
2. `你好小黑` and `小黑你好` trigger Bridge to Hermes and produce a spoken
   response on the speaker.
3. `小爱同学` still handles native questions and home control normally.
4. A temporary PCM sample is 16 kHz/S16LE/mono, intelligible, and not
   materially clipped; silence does not cause a false wake.
5. Removing or invalidating the ring stops Hermes input without starting
   ALSA.
6. Restarting the Client reattaches without restarting PNS.
7. Restarting PNS recreates the ring and the Client subsequently reconnects.
8. A full speaker reboot restores both native XiaoAI and the Hermes path.
9. The callback adds only a fixed memory copy and bounded atomic operations,
   and the ring remains below about 128 KiB.

Persistent boot activation occurs only after the temporary ABI, audio, native
assistant, and rollback checks have all passed.
