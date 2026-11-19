/* Read Scout GEQ property records from a single Windows audio-driver INF section. */
#include <windows.h>
#include <setupapi.h>

#include <float.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <wchar.h>

#pragma comment(lib, "setupapi.lib")

enum { BAND_COUNT = 10, VALUE_COUNT = BAND_COUNT + 1, RECORD_SIZE = 12 };
static const wchar_t scout_prefix[] = L"{156ac5b7-384a-4b15-a3a4-79350563fc88},";

_Static_assert(sizeof(float) == sizeof(uint32_t) &&
               FLT_RADIX == 2 && FLT_MANT_DIG == 24 && FLT_MAX_EXP == 128,
               "Scout INF properties require IEEE-754 binary32");

static int property_slot(const wchar_t *key)
{
    const size_t prefix_length = ARRAYSIZE(scout_prefix) - 1;
    if (_wcsnicmp(key, scout_prefix, prefix_length) != 0) {
        return -1;
    }

    const wchar_t *number = key + prefix_length;
    wchar_t *end;
    const unsigned long pid = wcstoul(number, &end, 10);
    if (end == number || *end != L'\0') {
        return -1;
    }
    if (pid == 5) {
        return 0;
    }
    return (pid >= 10 && pid <= 19) ? (int)(pid - 9) : -1;
}

static int read_float_record(PINFCONTEXT line, float *value)
{
    static const uint8_t header[] = { 4, 0, 0, 0, 1, 0, 0, 0 };
    uint8_t data[RECORD_SIZE];
    DWORD length;
    INT flags;

    if (!SetupGetIntField(line, 4, &flags) || flags != 1 ||
        !SetupGetBinaryField(line, 5, data, sizeof(data), &length) ||
        length != sizeof(data) || memcmp(data, header, sizeof(header)) != 0) {
        return 0;
    }

    const uint32_t bits = (uint32_t)data[8] | ((uint32_t)data[9] << 8) |
                          ((uint32_t)data[10] << 16) | ((uint32_t)data[11] << 24);
    memcpy(value, &bits, sizeof(bits));
    return isfinite(*value);
}

int wmain(int argc, wchar_t **argv)
{
    if (argc < 2 || argc > 3) {
        fwprintf(stderr, L"Usage: extract-g6-scout-geq.exe <driver.inf> [AddReg-section]\n");
        return 1;
    }

    const wchar_t *section = argc == 3 ? argv[2] : L"SysFx1770.10.AddReg";
    HINF inf = SetupOpenInfFileW(argv[1], NULL, INF_STYLE_WIN4, NULL);
    if (inf == INVALID_HANDLE_VALUE) {
        fwprintf(stderr, L"Could not open INF (error %lu).\n", GetLastError());
        return 1;
    }

    INFCONTEXT line;
    int more = SetupFindFirstLineW(inf, section, NULL, &line);
    if (!more) {
        fwprintf(stderr, L"Could not open section [%ls] (error %lu).\n", section, GetLastError());
        SetupCloseInfFile(inf);
        return 1;
    }

    float values[VALUE_COUNT];
    unsigned int found = 0;
    int status = 0;
    while (more) {
        wchar_t root[8], subkey[64], key[128];
        if (SetupGetStringFieldW(&line, 1, root, ARRAYSIZE(root), NULL) &&
            _wcsicmp(root, L"HKR") == 0 &&
            SetupGetStringFieldW(&line, 2, subkey, ARRAYSIZE(subkey), NULL) &&
            (_wcsicmp(subkey, L"FX\\0") == 0 || _wcsicmp(subkey, L"FX\\\\0") == 0) &&
            SetupGetStringFieldW(&line, 3, key, ARRAYSIZE(key), NULL)) {
            const int slot = property_slot(key);
            if (slot >= 0) {
                const unsigned int bit = 1u << slot;
                if ((found & bit) || !read_float_record(&line, &values[slot])) {
                    fwprintf(stderr, L"Invalid or repeated Scout record: %ls.\n", key);
                    status = 1;
                    break;
                }
                found |= bit;
            }
        }

        INFCONTEXT next;
        more = SetupFindNextLine(&line, &next);
        if (more) {
            line = next;
        }
    }
    SetupCloseInfFile(inf);

    if (status != 0) {
        return status;
    }
    if (found != (1u << VALUE_COUNT) - 1u) {
        fwprintf(stderr, L"[%ls] FX\\0: Scout preamp and ten gains are incomplete (mask 0x%03x).\n",
                 section, found);
        return 2;
    }

    wprintf(L"INF section [%ls], FX\\0\n", section);
    wprintf(L"preamp: %.9g\n", values[0]);
    for (size_t i = 0; i < BAND_COUNT; ++i) {
        wprintf(L"gain[%zu]: %.9g\n", i, values[i + 1]);
    }
    return 0;
}
