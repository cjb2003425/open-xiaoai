#!/bin/sh
set -eu

[ "$#" -eq 1 ] || { echo "usage: $0 output-directory" >&2; exit 2; }
src=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
toolchain=/opt/gcc-linaro-7.5.0-2019.12-x86_64_arm-linux-gnueabihf/bin
cc="$toolchain/arm-linux-gnueabihf-gcc"
[ -x "$cc" ] || { echo "missing Linaro compiler: $cc" >&2; exit 1; }
mkdir -p "$1"

[ "$(grep -Fc 'print "    procd_append_param env OPEN_XIAOAI_BLOCK_NATIVE_WAKE=" block' "$src/l06a_pcm_tap.sh")" -eq 1 ] || {
    echo "manager must append exactly one validated native-wake mode" >&2
    exit 1
}

"$cc" -shared -fPIC -Os -Wall -Wextra -Werror \
    "$src/xaudio_pcm_tap.c" -ldl -o "$1/xaudio_pcm_tap.so"
"$cc" -Os -Wall -Wextra -Werror \
    "$src/capture_pcm.c" -o "$1/capture_pcm"

host_test="$1/test_pcm_tap"
cc -DPCM_TAP_TEST -Os -Wall -Wextra -Werror \
    "$src/xaudio_pcm_tap.c" "$src/test_pcm_tap.c" -ldl -o "$host_test"
"$host_test"

cp "$src/l06a_pcm_tap.sh" "$src/user.sh" "$src/rollback.sh" \
    "$src/install_l06a.sh" "$1/"
chmod 700 "$1/capture_pcm" "$1/l06a_pcm_tap.sh" "$1/user.sh" \
    "$1/rollback.sh" "$1/install_l06a.sh"
chmod 600 "$1/xaudio_pcm_tap.so"
(
    cd "$1"
    sha256sum xaudio_pcm_tap.so capture_pcm l06a_pcm_tap.sh user.sh rollback.sh > manifest.sha256
)
