# Replace `chat` with Open-XiaoAI Client

## Goal

Replace the speaker's current `/data/chat -p` process with the locally built
Open-XiaoAI Rust Client while retaining the existing OpenWrt `chat_gpt` procd
service and its boot-time/respawn behavior.

## Current state

- Speaker: `root@10.0.0.5`, L06A firmware 1.88.55.
- `/etc/rc.d/S96chat_gpt` enables `/etc/init.d/chat_gpt` at boot.
- The service runs `/data/user.sh` under procd supervision.
- `/data/user.sh` currently executes `/data/chat -p`.
- The Linaro-built Client is compatible with the speaker's ARMv7 userland and
  glibc 2.25.
- The Bridge host is `10.0.0.22`; its WebSocket endpoint is port 4399.

## Design

1. Create `/data/open-xiaoai` on the speaker.
2. Back up `/data/user.sh` into `/data/open-xiaoai/backup/` with a timestamp.
3. Install the verified Linaro build as `/data/open-xiaoai/client` and make it
   executable.
4. Replace `/data/user.sh` with a small launcher that executes:

   ```sh
   exec /data/open-xiaoai/client ws://10.0.0.22:4399 > /tmp/client_log.txt 2>&1
   ```

5. Keep `/etc/init.d/chat_gpt` and its `S96chat_gpt` boot link unchanged, so
   procd continues to provide boot startup and automatic respawn.
6. Restart the `chat_gpt` service, then verify that `/data/chat -p` is gone and
   the new Client is running with the expected WebSocket URL.

## Safety and rollback

- Do not delete `/data/chat` or the original `chat_gpt` service.
- Preserve the prior `/data/user.sh` before replacement.
- If validation fails, restore the backup launcher and restart `chat_gpt`.
- The Bridge is not started as part of this change. If it is offline, the
  Client remains running and retries the WebSocket connection.

## Validation

- Compare local and remote SHA-256 hashes of the Client binary.
- Confirm the process command line contains
  `/data/open-xiaoai/client ws://10.0.0.22:4399`.
- Confirm `/data/chat -p` is no longer running.
- Inspect `/tmp/client_log.txt` for startup, connection, or compatibility
  errors.
- Confirm `/etc/rc.d/S96chat_gpt` remains enabled for the next boot.
