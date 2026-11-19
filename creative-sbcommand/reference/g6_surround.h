#ifndef G6_SURROUND_H
#define G6_SURROUND_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * CMSS3D surround matrix reference, reconstructed from KSUSBAPO64.dll
 * 1.2.109.0:
 *
 *   parameter update   0x180093990 (mask -> five target gains)
 *   process entry      0x180093940 (transition or steady)
 *   steady path        0x180093b30
 *   transition path    0x180093dd0
 *
 * The object keeps five current gains (+0x24..+0x34) and five target gains
 * (+0x38..+0x48). The setup constants 0.9535f and 0.3015f are cos/sin of
 * 17.5 degrees. Strength u = strength/100 enters as front = min(1+u,1) and
 * extra = min(1-u,1).
 */

typedef struct {
    unsigned mask;
    int strength;
    float current[5]; /* front, extra, unused, center, lfe */
    float target[5];
    int transition;
} G6SurroundState;

/* Recomputes the target gains and arms the transition, as 0x180093990. */
void g6_surround_set(G6SurroundState *state, unsigned mask, int strength);

/*
 * Processes one block in place. front_l/front_r and extra_l/extra_r are the
 * two input pairs and also receive the matrix output. center/lfe are optional
 * inputs; rear_l/rear_r are optional outputs for the extra pair. When a
 * pointer is NULL the corresponding path is skipped.
 */
void g6_surround_process(G6SurroundState *state,
                         float *front_l, float *front_r,
                         float *extra_l, float *extra_r,
                         const float *center, const float *lfe,
                         float *rear_l, float *rear_r,
                         size_t frames);

#ifdef __cplusplus
}
#endif

#endif
