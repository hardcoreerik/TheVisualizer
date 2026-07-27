# Embedded Vision

Embedded support is a post-desktop-v0.1 research lane. Nothing in this document is implemented or hardware-verified.

## Goal

Explore standalone audio-reactive visualizers that capture local audio and render without requiring the desktop application.

Candidate target classes are:

- ESP32-class boards with integrated or attached displays
- M5Tab5 as a touch-display visualizer
- addressable LED strips or matrices with a dedicated controller

All three target classes remain in discovery. No exact board, microphone, codec, display, LED protocol, SDK, or performance floor has been selected.

## Standalone boundary

An embedded visualizer is expected to own:

- microphone or I²S audio input
- bounded local audio analysis
- target-appropriate visual features
- local rendering or LED output
- physical controls and calibration required by the exact device
- clear degraded behavior when audio or output hardware is unavailable

It must not depend on desktop system-audio loopback, the desktop native-plugin ABI, `wgpu`, or WGSL.

## Reuse without false compatibility

The desktop and embedded implementations may reuse concepts such as normalized waveform, energy, and broad frequency bands. They may later exchange compact feature packets if a real use case justifies desktop-assisted operation.

This does not promise:

- binary compatibility
- desktop plugin compatibility
- WGSL preset compatibility
- identical FFT sizes or band definitions
- identical frame rates, precision, colors, or feedback effects
- one firmware image across target classes

Each embedded renderer should use the smallest native graphics or LED facility supported by its measured hardware.

## Research sequence

1. Finish and measure Windows desktop v0.1.
2. Inventory exact available boards, microphones/codecs, displays, LEDs, memory, storage, and SDKs.
3. Select one target for a standalone audio-to-visual spike.
4. Measure capture stability, analysis cost, frame rate, memory, heat, and power.
5. Record which desktop feature concepts remain useful.
6. Decide whether M5Tab5, an ESP32 display, and an LED controller need separate runtimes.

## First proof

The first embedded proof should boot without a desktop connection, capture live microphone or I²S audio, distinguish silence from active input, and render one stable bounded visual on exact recorded hardware.

Networking, remote preset delivery, synchronized multi-device shows, desktop feature streaming, and embedded plugin systems remain deferred until that proof exists.
