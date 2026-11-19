#include "g6_surround.h"

#include <math.h>
#include <string.h>

static float clamp_unit(float value)
{
    return value > 1.0f ? 1.0f : value;
}

void g6_surround_set(G6SurroundState *state, unsigned mask, int strength)
{
    float u;
    float front;
    float extra;
    float a = 0.0f;
    float b = 0.0f;
    float c = 0.0f;
    float d = 0.0f;
    float e = 0.0f;

    if (state == NULL) {
        return;
    }
    if (strength < 0) {
        strength = 0;
    }
    if (strength > 100) {
        strength = 100;
    }
    u = (float)strength / 100.0f;
    front = clamp_unit(1.0f + u);
    extra = clamp_unit(1.0f - u);

    switch (mask & 0x7ffu) {
    case 0x03b:
    case 0x033:
    case 0x603:
    case 0x60b:
        a = front;
        b = extra;
        d = 0.0f;
        e = 0.0f;
        break;
    case 0x007:
    case 0x00f:
        a = 0.9535f;
        d = 0.3015f;
        break;
    case 0x037:
    case 0x03f:
    case 0x607:
    case 0x60f:
        a = 0.2875f;
        b = extra;
        d = 0.3015f;
        e = 0.3015f;
        break;
    case 0x637:
    case 0x63b:
    case 0x63f:
    case 0x633:
    case 0x70f:
        a = 0.2875f;
        b = extra * 0.9535f;
        d = 0.3015f;
        e = 0.3015f;
        break;
    default:
        break;
    }

    state->mask = mask;
    state->strength = strength;
    state->target[0] = a;
    state->target[1] = b;
    state->target[2] = c;
    state->target[3] = d;
    state->target[4] = e;
    state->transition = 1;
}

void g6_surround_process(G6SurroundState *state,
                         float *front_l, float *front_r,
                         float *extra_l, float *extra_r,
                         const float *center, const float *lfe,
                         float *rear_l, float *rear_r,
                         size_t frames)
{
    float gains[5];
    float steps[5];
    int ramping;
    size_t i;
    size_t n;

    if (state == NULL || frames == 0 || (front_l == NULL && front_r == NULL)) {
        return;
    }
    for (i = 0; i < 5; ++i) {
        gains[i] = state->current[i];
        steps[i] = 0.0f;
    }
    ramping = state->transition;
    if (ramping) {
        const float count = (float)frames;
        for (i = 0; i < 5; ++i) {
            steps[i] = (state->target[i] - state->current[i]) / count;
        }
    } else {
        for (i = 0; i < 5; ++i) {
            gains[i] = state->target[i];
        }
    }

    for (n = 0; n < frames; ++n) {
        const float a = gains[0];
        const float b = gains[1];
        const float d = gains[3];
        const float e = gains[4];
        float left = (front_l != NULL) ? front_l[n] * a : 0.0f;
        float right = (front_r != NULL) ? front_r[n] * a : 0.0f;
        float el = (extra_l != NULL) ? extra_l[n] * b : 0.0f;
        float er = (extra_r != NULL) ? extra_r[n] * b : 0.0f;

        if (center != NULL) {
            left += center[n] * d;
            right += center[n] * d;
        }
        if (lfe != NULL) {
            el += lfe[n] * e;
            er += lfe[n] * e;
        }
        if (front_l != NULL) {
            front_l[n] = left;
        }
        if (front_r != NULL) {
            front_r[n] = right;
        }
        if (extra_l != NULL) {
            extra_l[n] = el;
        }
        if (extra_r != NULL) {
            extra_r[n] = er;
        }
        if (rear_l != NULL && (state->mask & 0x600u) != 0) {
            rear_l[n] = el;
        }
        if (rear_r != NULL && (state->mask & 0x600u) != 0) {
            rear_r[n] = er;
        }
        if (ramping) {
            for (i = 0; i < 5; ++i) {
                gains[i] += steps[i];
            }
        }
    }

    if (ramping) {
        for (i = 0; i < 5; ++i) {
            state->current[i] = state->target[i];
        }
        state->transition = 0;
    }
}
