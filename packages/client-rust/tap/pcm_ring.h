#ifndef OPEN_XIAOAI_PCM_RING_H
#define OPEN_XIAOAI_PCM_RING_H

#include <stdint.h>

#define PCM_MAGIC 0x4f585031u
#define PCM_VERSION 1u
#define PCM_RATE 16000u
#define PCM_SAMPLES 160u
#define PCM_SLOTS 256u
#define PCM_RING_PATH "/tmp/open-xiaoai-pcm.ring"

struct pcm_block {
    uint32_t sequence;
    int16_t samples[PCM_SAMPLES];
};

struct pcm_ring {
    uint32_t magic;
    uint32_t version;
    uint32_t rate;
    uint32_t block_samples;
    uint32_t slots;
    uint32_t producer_pid;
    uint32_t published;
    uint32_t errors;
    uint32_t input_bytes;
    uint32_t output_bytes;
    struct pcm_block blocks[PCM_SLOTS];
};

#endif
