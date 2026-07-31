#ifndef THEVISUALIZER_PLUGIN_H
#define THEVISUALIZER_PLUGIN_H

#include <stdint.h>

#define THEVISUALIZER_ABI_VERSION 1u
#define THEVISUALIZER_ABI_VERSION_V2 2u
#define THEVISUALIZER_PLUGIN_OK 0
#define THEVISUALIZER_MAX_PLUGIN_COMMANDS 24u

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

typedef struct TheVisualizerPluginInitV2 {
    uint32_t struct_size;
    uint32_t abi_version;
    const uint8_t *profile_id;
    uint32_t profile_id_len;
} TheVisualizerPluginInitV2;

typedef struct TheVisualizerFeatureSnapshotV2 {
    uint32_t struct_size;
    uint32_t abi_version;
    TheVisualizerFeatureSnapshotV1 base;
    float onset;
    float transient;
    const float *controls;
    uint32_t controls_len;
} TheVisualizerFeatureSnapshotV2;

typedef struct TheVisualizerPluginCommandV2 {
    uint32_t target;
    uint32_t index;
    uint32_t operation;
    float value;
} TheVisualizerPluginCommandV2;

typedef struct TheVisualizerPluginOutputV2 {
    uint32_t struct_size;
    uint32_t abi_version;
    uint32_t command_count;
    uint32_t event_flags;
    TheVisualizerPluginCommandV2 commands[THEVISUALIZER_MAX_PLUGIN_COMMANDS];
} TheVisualizerPluginOutputV2;

typedef int32_t (*TheVisualizerInitializeFnV2)(const TheVisualizerPluginInitV2 *init);
typedef int32_t (*TheVisualizerProcessFnV2)(
    const TheVisualizerFeatureSnapshotV2 *features,
    TheVisualizerPluginOutputV2 *output
);

typedef struct TheVisualizerPluginV2 {
    uint32_t struct_size;
    uint32_t abi_version;
    TheVisualizerInitializeFnV2 initialize;
    TheVisualizerProcessFnV2 process;
    TheVisualizerShutdownFn shutdown;
} TheVisualizerPluginV2;

THEVISUALIZER_EXPORT const TheVisualizerPluginV2 *thevisualizer_plugin_v2(void);

#endif
