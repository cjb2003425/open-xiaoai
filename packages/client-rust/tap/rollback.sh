#!/bin/sh
set -eu

BASE=/data/open-xiaoai/tap
BACKUP_PATH=$BASE/backup.path
[ -r "$BACKUP_PATH" ] || { echo "missing $BACKUP_PATH" >&2; exit 1; }
BACKUP=$(cat "$BACKUP_PATH")
[ -r "$BACKUP/user.sh" ] || { echo "missing launcher backup" >&2; exit 1; }

"$BASE/l06a_pcm_tap.sh" stop
cp "$BACKUP/user.sh" /data/user.sh
chmod 700 /data/user.sh
rm -f "$BASE/disabled"
sync
/etc/init.d/chat_gpt restart
echo "restored $BACKUP/user.sh"
