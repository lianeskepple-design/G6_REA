#include "g6_chain_capi.h"

#include "g6_bass.h"
#include "g6_crystalizer.h"
#include "g6_dialog.h"
#include "g6_surround.h"
#include "g6_svm.h"

#include <stdlib.h>
#include <string.h>

#define G6_CHAIN_BLOCK_FRAMES 128

struct G6Chain {
    uint32_t sample_rate;
    uint32_t channels;
    int crystalizer_enabled;
    int bass_enabled;
    int dialog_enabled;
    int surround_enabled;
    int svm_enabled;
    G6CrystalizerState crystalizer;
    G6BassState bass;
    G6DialogState dialog;
    G6SurroundState surround;
    G6SvmState svm;
    float scratch[G6_SVM_MAX_CHANNELS][G6_CHAIN_BLOCK_FRAMES];
};

G6Chain *g6_chain_create(uint32_t sample_rate, uint32_t channels)
{
    G6Chain *chain;

    if (sample_rate == 0 || channels == 0 || channels > G6_SVM_MAX_CHANNELS) {
        return NULL;
    }
    chain = (G6Chain *)calloc(1, sizeof(*chain));
    if (chain == NULL) {
        return NULL;
    }
    chain->sample_rate = sample_rate;
    chain->channels = channels;
    if (!g6_crystalizer_init(&chain->crystalizer, sample_rate) ||
        !g6_bass_init(&chain->bass, sample_rate, channels) ||
        !g6_dialog_init(&chain->dialog, sample_rate, channels) ||
        !g6_svm_init(&chain->svm, sample_rate, channels)) {
        free(chain);
        return NULL;
    }
    /* A new chain is in steady state, so Smart Volume only runs when enabled. */
    g6_svm_settle(&chain->svm);
    return chain;
}

void g6_chain_destroy(G6Chain *chain)
{
    free(chain);
}

int g6_chain_set_crystalizer(G6Chain *chain, int enabled, uint32_t strength)
{
    if (chain == NULL) {
        return 0;
    }
    if (enabled) {
        if (!g6_crystalizer_set_strength(&chain->crystalizer, strength)) {
            return 0;
        }
    }
    chain->crystalizer_enabled = enabled != 0;
    return 1;
}

int g6_chain_set_bass(G6Chain *chain, int enabled, int strength,
                      float cutoff_hz)
{
    if (chain == NULL) {
        return 0;
    }
    g6_bass_set(&chain->bass, enabled ? strength : 0, cutoff_hz);
    chain->bass_enabled = enabled != 0;
    return 1;
}

int g6_chain_set_bass_mode(G6Chain *chain, int mode)
{
    if (chain == NULL) {
        return 0;
    }
    g6_bass_set_mode(&chain->bass, mode);
    return 1;
}

int g6_chain_set_dialog(G6Chain *chain, int enabled, int strength)
{
    if (chain == NULL) {
        return 0;
    }
    g6_dialog_set(&chain->dialog, enabled ? strength : 0);
    chain->dialog_enabled = enabled != 0;
    return 1;
}

int g6_chain_set_surround(G6Chain *chain, int enabled, uint32_t mask,
                          int strength)
{
    if (chain == NULL) {
        return 0;
    }
    if (enabled) {
        if (chain->channels != 6) {
            return 0;
        }
        g6_surround_set(&chain->surround, mask, strength);
    }
    chain->surround_enabled = enabled != 0;
    return 1;
}

int g6_chain_set_svm(G6Chain *chain, int enabled, float level_db, float mix)
{
    if (chain == NULL) {
        return 0;
    }
    if (!g6_svm_set_level(&chain->svm, level_db)) {
        return 0;
    }
    g6_svm_set_mix(&chain->svm, mix);
    /* 0x180093420: bypass is a wet/makeup crossfade, not a hard switch. */
    if (!g6_svm_set_enabled(&chain->svm, enabled != 0)) {
        return 0;
    }
    chain->svm_enabled = enabled != 0;
    return 1;
}

int g6_chain_set_svm_delay(G6Chain *chain, float seconds)
{
    if (chain == NULL) {
        return 0;
    }
    return g6_svm_set_delay_seconds(&chain->svm, seconds);
}

static void deinterleave(G6Chain *chain, const float *block, size_t frames,
                         size_t channels)
{
    size_t i;
    size_t channel;

    for (i = 0; i < frames; ++i) {
        for (channel = 0; channel < channels; ++channel) {
            chain->scratch[channel][i] = block[i * channels + channel];
        }
    }
}

static void interleave(G6Chain *chain, float *block, size_t frames,
                       size_t channels)
{
    size_t i;
    size_t channel;

    for (i = 0; i < frames; ++i) {
        for (channel = 0; channel < channels; ++channel) {
            block[i * channels + channel] = chain->scratch[channel][i];
        }
    }
}

int g6_chain_process_interleaved(G6Chain *chain, float *samples, size_t frames)
{
    size_t position = 0;

    if (chain == NULL || samples == NULL) {
        return 0;
    }
    while (position < frames) {
        size_t count = frames - position;
        float *block = samples + position * chain->channels;

        if (count > G6_CHAIN_BLOCK_FRAMES) {
            count = G6_CHAIN_BLOCK_FRAMES;
        }
        if (chain->crystalizer_enabled) {
            if (!g6_crystalizer_process_interleaved(
                    &chain->crystalizer, block, count, chain->channels)) {
                return 0;
            }
        }
        if (chain->bass_enabled) {
            if (!g6_bass_process_interleaved(&chain->bass, block, count,
                                             chain->channels)) {
                return 0;
            }
        }
        if (chain->dialog_enabled) {
            if (!g6_dialog_process_interleaved(&chain->dialog, block, count,
                                               chain->channels)) {
                return 0;
            }
        }
        if (chain->surround_enabled) {
            /*
             * 5.1 WAV order: FL, FR, FC, LFE, BL, BR.
             * The matrix treats BL/BR as the "extra" pair and reads FC/LFE.
             */
            deinterleave(chain, block, count, 6);
            g6_surround_process(&chain->surround,
                                chain->scratch[0], chain->scratch[1],
                                chain->scratch[4], chain->scratch[5],
                                chain->scratch[2], chain->scratch[3],
                                NULL, NULL, count);
            interleave(chain, block, count, 6);
        }
        if (chain->svm_enabled || g6_svm_active(&chain->svm)) {
            float *buffers[G6_SVM_MAX_CHANNELS];
            size_t channel;

            deinterleave(chain, block, count, chain->channels);
            for (channel = 0; channel < G6_SVM_MAX_CHANNELS; ++channel) {
                buffers[channel] = (channel < chain->channels)
                    ? chain->scratch[channel] : NULL;
            }
            if (!g6_svm_process(&chain->svm, buffers, count)) {
                return 0;
            }
            interleave(chain, block, count, chain->channels);
        }
        position += count;
    }
    return 1;
}
