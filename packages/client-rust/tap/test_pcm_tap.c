#include "pcm_ring.h"

#include <assert.h>
#include <stdint.h>
#include <string.h>

void pcm_tap_test_attach(struct pcm_ring *test_ring);
void pcm_tap_test_publish(const void *output, int input_bytes, int output_bytes, int result);
void pcm_tap_test_set_block_native(int enabled);
void pcm_tap_test_process_output(void *output, int input_bytes, int output_bytes, int result);

static int all_zero(const int16_t *samples, int count) {
    for (int i = 0; i < count; ++i) {
        if (samples[i] != 0) return 0;
    }
    return 1;
}

int main(void) {
    assert(sizeof(struct pcm_ring) == 82984);
    assert(sizeof(struct pcm_ring) < 128 * 1024);

    struct pcm_ring ring;
    memset(&ring, 0, sizeof(ring));
    pcm_tap_test_attach(&ring);

    int16_t output[320];
    for (int i = 0; i < 320; ++i) output[i] = (int16_t)(i - 160);

    int16_t expected[320];
    memcpy(expected, output, sizeof(expected));

    pcm_tap_test_set_block_native(0);
    pcm_tap_test_process_output(output, 640, 640, 0);
    assert(ring.published == 1);
    assert(ring.errors == 0);
    assert(ring.blocks[0].sequence == 2);
    assert(!memcmp(ring.blocks[0].samples, expected, sizeof(ring.blocks[0].samples)));
    assert(!memcmp(output, expected, sizeof(expected)));

    for (int i = 0; i < 320; ++i) output[i] = (int16_t)(300 - i);
    memcpy(expected, output, sizeof(expected));
    pcm_tap_test_set_block_native(1);
    pcm_tap_test_process_output(output, 640, 640, 0);
    assert(ring.published == 2);
    assert(ring.errors == 0);
    assert(!memcmp(ring.blocks[1].samples, expected, sizeof(ring.blocks[1].samples)));
    assert(all_zero(output, 320));

    for (int i = 0; i < 320; ++i) output[i] = (int16_t)(i + 1);
    memcpy(expected, output, sizeof(expected));
    pcm_tap_test_process_output(output, 320, 640, 0);
    assert(ring.published == 2);
    assert(ring.errors == 1);
    assert(!memcmp(output, expected, sizeof(expected)));

    pcm_tap_test_process_output(output, 640, 640, -1);
    assert(ring.published == 2);
    assert(ring.errors == 1);
    assert(!memcmp(output, expected, sizeof(expected)));

    pcm_tap_test_process_output(NULL, 640, 640, 0);
    assert(ring.published == 2);
    assert(ring.errors == 2);
    return 0;
}
