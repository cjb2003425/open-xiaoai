/*
 * L06A 1.88.55 processed-PCM tap. The original function is always called
 * first. Its return value and output size are preserved; when explicitly
 * enabled, the native output is silenced only after the real PCM is tapped.
 *
 * The ring and callback structure are adapted from xiaomi_ai_llm's MIT
 * licensed PCM tap by open-speaker-llm (Copyright 2026).
 */
#define _GNU_SOURCE
#include "pcm_ring.h"

#include <dlfcn.h>
#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

typedef int (*process_fn)(void *, void *, int, void *, int *);

static process_fn original;
static struct pcm_ring *ring;
static int block_native_wake;

static int valid_pcm_call(const void *output, int input_bytes, int output_bytes, int result) {
    return output && result == 0 && input_bytes == 640 && output_bytes == 640;
}

static void publish_pcm(const void *output, int input_bytes, int output_bytes, int result) {
    if (!ring || result != 0) return;

    __atomic_store_n(&ring->input_bytes, (uint32_t)input_bytes, __ATOMIC_RELAXED);
    __atomic_store_n(&ring->output_bytes, (uint32_t)output_bytes, __ATOMIC_RELAXED);
    if (!valid_pcm_call(output, input_bytes, output_bytes, result)) {
        __atomic_add_fetch(&ring->errors, 1, __ATOMIC_RELAXED);
        return;
    }

    uint32_t frame = __atomic_load_n(&ring->published, __ATOMIC_RELAXED);
    struct pcm_block *block = &ring->blocks[frame % PCM_SLOTS];
    __atomic_store_n(&block->sequence, frame * 2u + 1u, __ATOMIC_SEQ_CST);
    memcpy(block->samples, output, sizeof(block->samples));
    __atomic_store_n(&block->sequence, frame * 2u + 2u, __ATOMIC_RELEASE);
    __atomic_store_n(&ring->published, frame + 1u, __ATOMIC_RELEASE);
}

static void process_output(void *output, int input_bytes, int output_bytes, int result) {
    publish_pcm(output, input_bytes, output_bytes, result);
    if (block_native_wake && valid_pcm_call(output, input_bytes, output_bytes, result)) {
        memset(output, 0, (size_t)output_bytes);
    }
}

__attribute__((constructor)) static void initialize(void) {
    original = (process_fn)dlsym(RTLD_NEXT, "xaudio_wrapper_dnn");

    char executable[256];
    ssize_t size = readlink("/proc/self/exe", executable, sizeof(executable) - 1);
    if (size < 0) return;
    executable[size] = '\0';
    if (!original || strcmp(executable, "/usr/bin/mipns-xiaomi") != 0) return;

    const char *block_value = getenv("OPEN_XIAOAI_BLOCK_NATIVE_WAKE");
    block_native_wake = block_value && strcmp(block_value, "1") == 0;

    unsetenv("LD_PRELOAD");
    int fd = open(PCM_RING_PATH, O_RDWR | O_CREAT | O_CLOEXEC | O_NOFOLLOW, 0600);
    if (fd < 0) return;
    struct stat status;
    if (fstat(fd, &status) || !S_ISREG(status.st_mode) || status.st_uid != getuid() ||
        fchmod(fd, 0600) || ftruncate(fd, sizeof(struct pcm_ring))) {
        close(fd);
        return;
    }

    void *mapping = mmap(NULL, sizeof(struct pcm_ring), PROT_READ | PROT_WRITE,
                         MAP_SHARED, fd, 0);
    close(fd);
    if (mapping == MAP_FAILED) return;

    ring = mapping;
    memset(ring, 0, sizeof(*ring));
    ring->version = PCM_VERSION;
    ring->rate = PCM_RATE;
    ring->block_samples = PCM_SAMPLES;
    ring->slots = PCM_SLOTS;
    ring->producer_pid = (uint32_t)getpid();
    __atomic_store_n(&ring->magic, PCM_MAGIC, __ATOMIC_RELEASE);
}

int xaudio_wrapper_dnn(void *handle, void *input, int input_bytes,
                       void *output, int *output_bytes) {
    if (!original) return -1;
    int result = original(handle, input, input_bytes, output, output_bytes);
    process_output(output, input_bytes, output_bytes ? *output_bytes : -1, result);
    return result;
}

#ifdef PCM_TAP_TEST
void pcm_tap_test_attach(struct pcm_ring *test_ring) { ring = test_ring; }
void pcm_tap_test_publish(const void *output, int input_bytes, int output_bytes, int result) {
    publish_pcm(output, input_bytes, output_bytes, result);
}
void pcm_tap_test_set_block_native(int enabled) { block_native_wake = enabled != 0; }
void pcm_tap_test_process_output(void *output, int input_bytes, int output_bytes, int result) {
    process_output(output, input_bytes, output_bytes, result);
}
#endif
