/*
 * C side of the APO-exact EQ parity harness.
 *
 * Build (MSVC developer prompt):
 *   cl /nologo /TC /std:c11 /W4 /WX /fp:strict /I ..\..\reference
 *      apo_parity.c ..\..\reference\g6_eq_apo.c /Fe:apo_parity.exe
 *
 * Run and compare with `cargo run --example apo_parity` from app/.
 * Both programs print the five coefficient slots and a 256-sample impulse
 * response as hex float32, in the same order.
 */

#include "g6_eq_apo.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define FS 48000.0f
#define N 256

static uint32_t bits(float value)
{
    uint32_t word;
    memcpy(&word, &value, sizeof(word));
    return word;
}

static void report(const char *name, const G6EqApoCoefficients *coeff)
{
    G6EqApoState state = {0.0f, 0.0f};
    size_t n;

    printf("case %s slots %08x %08x %08x %08x %08x\n", name,
           bits(coeff->c8), bits(coeff->c12), bits(coeff->c16),
           bits(coeff->c20), bits(coeff->c24));
    printf("case %s ir", name);
    for (n = 0; n < N; ++n) {
        float x = (n == 0) ? 1.0f : 0.0f;
        float y = g6_eq_apo_process(coeff, &state, x);
        printf(" %08x", bits(y));
    }
    printf("\n");
}

int main(void)
{
    G6EqApoCoefficients coeff;
    int status;

    status = g6_eq_apo_peak_db(6.0f, FS, 1000.0f, 2.0f, &coeff);
    if (status != G6_EQ_APO_OK) {
        return 1;
    }
    report("peak_p6", &coeff);

    status = g6_eq_apo_peak_db(-12.0f, FS, 1000.0f, 2.0f, &coeff);
    if (status != G6_EQ_APO_OK) {
        return 2;
    }
    report("peak_m12", &coeff);

    status = g6_eq_apo_low_shelf_db(6.0f, FS, 31.0f, &coeff);
    if (status != G6_EQ_APO_OK) {
        return 3;
    }
    report("low_p6", &coeff);

    status = g6_eq_apo_low_shelf_db(-12.0f, FS, 31.0f, &coeff);
    if (status != G6_EQ_APO_OK) {
        return 4;
    }
    report("low_m12", &coeff);

    status = g6_eq_apo_high_shelf_db(6.0f, FS, 16000.0f, &coeff);
    if (status != G6_EQ_APO_OK) {
        return 5;
    }
    report("high_p6", &coeff);

    status = g6_eq_apo_high_shelf_db(-12.0f, FS, 16000.0f, &coeff);
    if (status != G6_EQ_APO_OK) {
        return 6;
    }
    report("high_m12", &coeff);

    {
        static const float freqs[10] = {
            31.0f, 62.0f, 125.0f, 250.0f, 500.0f,
            1000.0f, 2000.0f, 4000.0f, 8000.0f, 16000.0f
        };
        static const float gains[10] = {
            2.0f, 0.0f, -2.0f, -2.0f, 0.0f, 2.0f, 2.0f, -5.0f, 2.0f, 1.0f
        };
        G6EqApoCoefficients bands[10];
        G6EqApoState states[10];
        size_t band;
        size_t n;

        for (band = 0; band < 10; ++band) {
            states[band].w1 = 0.0f;
            states[band].w2 = 0.0f;
            if (band == 0) {
                status = g6_eq_apo_low_shelf_db(gains[band], FS, freqs[band],
                                                &bands[band]);
            } else if (band == 9) {
                status = g6_eq_apo_high_shelf_db(gains[band], FS, freqs[band],
                                                 &bands[band]);
            } else {
                status = g6_eq_apo_peak_db(gains[band], FS, freqs[band], 2.0f,
                                           &bands[band]);
            }
            if (status != G6_EQ_APO_OK) {
                return 7;
            }
        }

        printf("case fps_chain ir");
        for (n = 0; n < N; ++n) {
            float x = (n == 0) ? 1.0f : 0.0f;
            for (band = 0; band < 10; ++band) {
                x = g6_eq_apo_process(&bands[band], &states[band], x);
            }
            printf(" %08x", bits(x));
        }
        printf("\n");
    }

    return 0;
}
