#ifndef THEVISUALIZER_PLUGIN_H
#define THEVISUALIZER_PLUGIN_H

#include <stdint.h>

#define THEVISUALIZER_ABI_VERSION 1u
#define THEVISUALIZER_PLUGIN_OK 0

#if defined(_WIN32)
#define THEVISUALIZER_EXPORT __declspec(dllexport)
#else
#define THEVISUALIZER_EXPORT __attribute__((visibility("default")))
#endif

typedef struct TheVisualizerFeatureSnapshotV1 {
    uint32_t struct_size;
    uint32_t abi_version;
    float time_seconds;
    float delta_seconds;
    const float *waveform;
    uint32_t waveform_len;
    const float *spectrum;
    uint32_t spectrum_len;
    float rms;
    float peak;
    float low;
    float mid;
    float high;
} TheVisualizerFeatureSnapshotV1;

typedef struct TheVisualizerPluginOutputV1 {
    uint32_t struct_size;
    uint32_t abi_version;
    float response_multiplier;
} TheVisualizerPluginOutputV1;

typedef int32_t (*TheVisualizerInitializeFn)(void);
typedef int32_t (*TheVisualizerProcessFn)(
    const TheVisualizerFeatureSnapshotV1 *features,
    TheVisualizerPluginOutputV1 *output
);
typedef void (*TheVisualizerShutdownFn)(void);

typedef struct TheVisualizerPluginV1 {
    uint32_t struct_size;
    uint32_t abi_version;
    TheVisualizerInitializeFn initialize;
    TheVisualizerProcessFn process;
    TheVisualizerShutdownFn shutdown;
} TheVisualizerPluginV1;

THEVISUALIZER_EXPORT const TheVisualizerPluginV1 *thevisualizer_plugin_v1(void);

#endif
