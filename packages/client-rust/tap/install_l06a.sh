#!/bin/sh
# Run on the speaker only after temporary ABI/audio validation passes.
set -eu

STAGE=${1:-/tmp/open-xiaoai-tap-stage}
DEST=/data/open-xiaoai/tap
[ -d "$STAGE" ] || { echo "missing stage directory: $STAGE" >&2; exit 1; }
for file in xaudio_pcm_tap.so capture_pcm l06a_pcm_tap.sh user.sh rollback.sh manifest.sha256; do
    [ -f "$STAGE/$file" ] || { echo "missing staged $file" >&2; exit 1; }
done
(cd "$STAGE" && sha256sum -c manifest.sha256)

timestamp=$(date +%Y%m%d-%H%M%S)
backup="$DEST/backup/$timestamp"
mkdir -p "$backup"
chmod 700 "$backup"
cp /data/user.sh "$backup/user.sh"

mkdir -p "$DEST"
chmod 700 "$DEST"
for file in xaudio_pcm_tap.so capture_pcm l06a_pcm_tap.sh user.sh rollback.sh manifest.sha256; do
    cp "$STAGE/$file" "$DEST/$file"
done
chmod 600 "$DEST/xaudio_pcm_tap.so" "$DEST/manifest.sha256"
chmod 700 "$DEST/capture_pcm" "$DEST/l06a_pcm_tap.sh" "$DEST/user.sh" "$DEST/rollback.sh"
echo "$backup" > "$DEST/backup.path"
printf '%s\n' blocked > "$DEST/native-wake.mode"
chmod 600 "$DEST/backup.path"
chmod 600 "$DEST/native-wake.mode"

cp "$DEST/user.sh" /data/user.sh
chmod 700 /data/user.sh
rm -f "$DEST/disabled"
if ! "$DEST/l06a_pcm_tap.sh" start; then
    cp "$backup/user.sh" /data/user.sh
    chmod 700 /data/user.sh
    "$DEST/l06a_pcm_tap.sh" stop || true
    echo "install failed; launcher restored from $backup" >&2
    exit 1
fi
sync
/etc/init.d/chat_gpt restart
echo "BACKUP=$backup"
