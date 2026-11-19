#define _POSIX_C_SOURCE 200809L
#include "pcm_ring.h"

#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

static volatile sig_atomic_t active = 1;
static void stop_capture(int signal_number) { (void)signal_number; active = 0; }

static double monotonic_seconds(void) {
    struct timespec time;
    clock_gettime(CLOCK_MONOTONIC, &time);
    return time.tv_sec + time.tv_nsec / 1e9;
}

static void put16(FILE *file, uint32_t value) {
    fputc(value & 255, file);
    fputc((value >> 8) & 255, file);
}

static void put32(FILE *file, uint32_t value) {
    put16(file, value);
    put16(file, value >> 16);
}

static int write_wav(const char *path, int16_t *samples, uint32_t count) {
    int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC | O_NOFOLLOW, 0600);
    if (fd < 0) return 1;
    FILE *file = fdopen(fd, "wb");
    if (!file) { close(fd); return 1; }
    fwrite("RIFF", 1, 4, file); put32(file, 36 + count * 2);
    fwrite("WAVEfmt ", 1, 8, file); put32(file, 16); put16(file, 1); put16(file, 1);
    put32(file, PCM_RATE); put32(file, PCM_RATE * 2); put16(file, 2); put16(file, 16);
    fwrite("data", 1, 4, file); put32(file, count * 2);
    int failed = fwrite(samples, 2, count, file) != count;
    failed |= fclose(file) != 0;
    return failed;
}

int main(int argc, char **argv) {
    int check = argc == 2 && !strcmp(argv[1], "--check");
    if ((!check && argc != 3) || (check && argc != 2)) {
        fprintf(stderr, "usage: capture_pcm --check | output.wav seconds\n");
        return 2;
    }
    double seconds = check ? 0.1 : strtod(argv[2], NULL);
    if (seconds < 0.1 || seconds > 30) return 2;

    int fd = open(PCM_RING_PATH, O_RDONLY | O_NOFOLLOW);
    if (fd < 0) return 1;
    struct stat status;
    if (fstat(fd, &status) || !S_ISREG(status.st_mode) ||
        status.st_size != (off_t)sizeof(struct pcm_ring)) { close(fd); return 1; }
    struct pcm_ring *ring = mmap(NULL, sizeof(*ring), PROT_READ, MAP_SHARED, fd, 0);
    close(fd);
    if (ring == MAP_FAILED) return 1;
    if (__atomic_load_n(&ring->magic, __ATOMIC_ACQUIRE) != PCM_MAGIC ||
        ring->version != PCM_VERSION || ring->rate != PCM_RATE ||
        ring->block_samples != PCM_SAMPLES || ring->slots != PCM_SLOTS) return 1;

    uint32_t producer = ring->producer_pid;
    uint32_t read_frame = __atomic_load_n(&ring->published, __ATOMIC_ACQUIRE);
    uint32_t target = (uint32_t)(seconds * 100) * PCM_SAMPLES;
    uint32_t used = 0;
    int16_t *samples = calloc(target, sizeof(int16_t));
    if (!samples) return 1;
    signal(SIGTERM, stop_capture); signal(SIGINT, stop_capture);
    double start = monotonic_seconds(), last = start;
    int failed = 0;

    while (active && used < target && monotonic_seconds() - start < seconds + 3) {
        uint32_t next = __atomic_load_n(&ring->published, __ATOMIC_ACQUIRE);
        if (ring->producer_pid != producer || next - read_frame > PCM_SLOTS) { failed = 1; break; }
        if (next == read_frame) {
            if (monotonic_seconds() - last > 2) { failed = 1; break; }
            struct timespec pause = {0, 5000000}; nanosleep(&pause, NULL); continue;
        }
        struct pcm_block *block = &ring->blocks[read_frame % PCM_SLOTS];
        uint32_t expected = read_frame * 2u + 2u;
        if (__atomic_load_n(&block->sequence, __ATOMIC_ACQUIRE) != expected) { failed = 1; break; }
        memcpy(samples + used, block->samples, sizeof(block->samples));
        __atomic_thread_fence(__ATOMIC_SEQ_CST);
        if (__atomic_load_n(&block->sequence, __ATOMIC_ACQUIRE) != expected) { failed = 1; break; }
        read_frame++; used += PCM_SAMPLES; last = monotonic_seconds();
    }
    if (!active || used != target) failed = 1;
    if (!failed && !check) failed = write_wav(argv[1], samples, used);
    fprintf(stderr, "PCM_TAP producer=%u input=%u output=%u errors=%u samples=%u status=%s\n",
            producer, ring->input_bytes, ring->output_bytes, ring->errors, used,
            failed ? "error" : "ok");
    free(samples); munmap(ring, sizeof(*ring));
    return failed;
}
