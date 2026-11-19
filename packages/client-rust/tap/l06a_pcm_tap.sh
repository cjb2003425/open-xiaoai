#!/bin/sh
# Reversible L06A 1.88.55 procd overlay. Never writes the squashfs firmware.
set -u

PNS=/etc/rc.d/S55pns
PNS_CTL=$PNS
BASE=${OPEN_XIAOAI_TAP_DIR:-/data/open-xiaoai/tap}
TAP=$BASE/xaudio_pcm_tap.so
CAPTURE=$BASE/capture_pcm
MANIFEST=$BASE/manifest.sha256
OVERLAY=/tmp/open-xiaoai-tap.managed.pns
RING=/tmp/open-xiaoai-pcm.ring
DISABLED=$BASE/disabled
MODE=$BASE/native-wake.mode
TAG=OPEN_XIAOAI_PCM_TAP_L06A_V1

MIPNS_HASH=a41638756e0a3cd557cb106f08aa7b4311cfc3ddf8569413f8754c920a474829
XAUDIO_HASH=2898fea85a38c15169ffe63d8534fed06c4c3298eb32668e9fbdb3f78d7199ca
PNS_HASH=911e6326b7862f832fc037e04eb2236b1ed38968b417647ae00cbdad0052d0ac

log() { echo "[PCM-TAP] $*"; }
hash_is() { [ "$(sha256sum "$1" 2>/dev/null | awk '{print $1}')" = "$2" ]; }
is_boot1() { [ "$(awk '$2 == "/" {print $1; exit}' /proc/mounts)" = /dev/mtdblock5 ]; }
mounted() { awk -v target="$PNS" '$2 == target {found=1} END {exit !found}' /proc/mounts; }
owned() { mounted && grep -q "$TAG" "$PNS" 2>/dev/null; }

read_mode() {
    mode=$(sed -n '1p' "$MODE" 2>/dev/null || true)
    case "$mode" in
        takeover) echo takeover ;;
        *) echo blocked ;;
    esac
}

write_mode() {
    umask 077
    printf '%s\n' "$1" > "$MODE"
}

verify_firmware() {
    is_boot1 &&
    hash_is /usr/bin/mipns-xiaomi "$MIPNS_HASH" &&
    hash_is /usr/lib/libxaudio_engine.so "$XAUDIO_HASH"
}

verify_components() {
    [ -r "$TAP" ] && [ -x "$CAPTURE" ] && [ -r "$MANIFEST" ] || return 1
    (cd "$BASE" && sha256sum -c manifest.sha256 >/dev/null 2>&1)
}

healthy() {
    set -- $(pidof mipns-xiaomi 2>/dev/null)
    [ "$#" -eq 1 ] || return 1
    grep -q "$TAP" "/proc/$1/maps" 2>/dev/null || return 1
    "$CAPTURE" --check >/dev/null 2>&1
}

stop_all_pns() {
    "$PNS_CTL" stop >/dev/null 2>&1 || true
    attempt=0
    while [ -n "$(pidof mipns-xiaomi 2>/dev/null)" ] && [ "$attempt" -lt 5 ]; do
        sleep 1
        attempt=$((attempt + 1))
    done
    for pid in $(pidof mipns-xiaomi 2>/dev/null); do
        kill -TERM "$pid" 2>/dev/null || true
    done
    attempt=0
    while [ -n "$(pidof mipns-xiaomi 2>/dev/null)" ] && [ "$attempt" -lt 5 ]; do
        sleep 1
        attempt=$((attempt + 1))
    done
    [ -z "$(pidof mipns-xiaomi 2>/dev/null)" ]
}

wait_native() {
    attempt=0
    while [ "$attempt" -lt 8 ]; do
        set -- $(pidof mipns-xiaomi 2>/dev/null)
        if [ "$#" -eq 1 ] && ! grep -q "$TAP" "/proc/$1/maps" 2>/dev/null; then
            return 0
        fi
        sleep 1
        attempt=$((attempt + 1))
    done
    return 1
}

restore_native() {
    if owned; then
        stop_all_pns || { log "could not stop all tap PNS processes"; return 1; }
        umount "$PNS" || return 1
        rm -f "$OVERLAY" "$RING"
        "$PNS_CTL" start || return 1
        wait_native || { log "native PNS did not reach a single-instance state"; return 1; }
        log "restored native PNS service"
    elif mounted; then
        log "foreign PNS overlay detected; left untouched"
        return 1
    fi
}

render_overlay() {
    block_native=$1
    umask 077
    awk -v tag="$TAG" -v tap="$TAP" -v block="$block_native" '
        {print}
        /^_start_mipns_xiaomi\(\)/ {xiaomi=1}
        xiaomi && /^[[:space:]]*procd_open_instance[[:space:]]*$/ {
            print "    # " tag
            print "    procd_set_param env LD_PRELOAD=" tap
            print "    procd_append_param env OPEN_XIAOAI_BLOCK_NATIVE_WAKE=" block
            xiaomi=0; count++
        }
        END {if(count!=1) exit 1}
    ' "$PNS" > "$OVERLAY" || return 1
    sh -n "$OVERLAY" || return 1
    chmod 700 "$OVERLAY"
}

start_tap() {
    mode=$(read_mode)
    if [ "$mode" = takeover ]; then block_native=0; else block_native=1; fi
    [ ! -e "$DISABLED" ] || { log "disabled marker present; fail closed"; return 1; }
    verify_firmware || { log "firmware ABI hash mismatch; refusing injection"; return 1; }
    verify_components || { log "component manifest verification failed"; return 1; }

    if owned; then
        if healthy && grep -q "OPEN_XIAOAI_BLOCK_NATIVE_WAKE=$block_native" "$PNS"; then
            log "already healthy; native wake mode=$mode"
            return 0
        fi
        restore_native || return 1
    elif mounted; then
        log "foreign PNS overlay detected; refusing replacement"
        return 1
    fi

    hash_is "$PNS" "$PNS_HASH" || { log "PNS init hash mismatch"; return 1; }
    render_overlay "$block_native" || { log "failed to render PNS overlay"; return 1; }
    stop_all_pns || { log "could not stop existing PNS processes"; rm -f "$OVERLAY"; return 1; }
    mount --bind "$OVERLAY" "$PNS" || return 1

    if "$PNS_CTL" start; then
        attempt=0
        while [ "$attempt" -lt 8 ]; do
            sleep 1
            if healthy; then
                log "ready: continuous processed PCM; native wake mode=$mode"
                return 0
            fi
            attempt=$((attempt + 1))
        done
    fi

    log "health check failed; restoring native PNS"
    log "diagnostic pids: $(pidof mipns-xiaomi 2>/dev/null || echo none)"
    for pid in $(pidof mipns-xiaomi 2>/dev/null); do
        if grep -q "$TAP" "/proc/$pid/maps" 2>/dev/null; then
            log "diagnostic pid=$pid hook=loaded"
        else
            log "diagnostic pid=$pid hook=missing"
        fi
    done
    if [ -e "$RING" ]; then
        log "diagnostic ring: $(ls -ln "$RING" 2>/dev/null)"
        "$CAPTURE" --check 2>&1 || true
    else
        log "diagnostic ring: missing"
    fi
    restore_native || true
    if [ "$mode" = takeover ] && [ "${RECOVERING_BLOCKED:-0}" -eq 0 ]; then
        log "takeover activation failed; restoring blocked mode"
        write_mode blocked
        RECOVERING_BLOCKED=1 start_tap || true
        return 1
    fi
    : > "$DISABLED"
    return 1
}

set_native_wake_mode() {
    requested=$1
    mkdir -p "$BASE"
    write_mode "$requested"
    rm -f "$DISABLED"
    start_tap
}

case "${1:-status}" in
    start) start_tap ;;
    stop) restore_native ;;
    status) owned && healthy && { log "healthy"; exit 0; }; log "inactive or unhealthy"; exit 1 ;;
    clear-disabled) rm -f "$DISABLED" ;;
    block-native-wake) set_native_wake_mode blocked ;;
    allow-native-wake) set_native_wake_mode takeover ;;
    *) echo "usage: $0 start|stop|status|clear-disabled|block-native-wake|allow-native-wake" >&2; exit 2 ;;
esac
