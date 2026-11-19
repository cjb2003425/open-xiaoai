#!/bin/sh

TAP_DIR=/data/open-xiaoai/tap
LOG=/tmp/open-xiaoai-tap.log

if ! "$TAP_DIR/l06a_pcm_tap.sh" start >> "$LOG" 2>&1; then
    echo "PCM tap unavailable; external conversation audio remains disabled" >> "$LOG"
    exec sleep 86400
fi

export OPEN_XIAOAI_AUDIO_SOURCE=pcm_tap
export OPEN_XIAOAI_PCM_RING=/tmp/open-xiaoai-pcm.ring
exec /data/open-xiaoai/client ws://10.0.0.22:4399 > /tmp/client_log.txt 2>&1
