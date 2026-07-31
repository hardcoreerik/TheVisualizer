# Visual Director

Status: local brief composer implemented; paid image generation, editing, critique, and asset import are not connected.

The Visual Director converts the active visualization, a rolling audio window, routed colors, material finish, and explicit creative controls into an inspectable image brief. It opens in a separate experimental window from the single `Visual Director (Experimental)…` button in `Instrument [I]` and sends no network request.

## Current local pipeline

```text
live waveform/spectrum/energy + 12-second history
        |
        v
Visual DNA --> structured candidates --> novelty scoring --> scene specification
                                                             |
                                                             v
                                                  preflight --> prompt
```

The analyzer measures RMS/peak energy, low/mid/high energy, spectral centroid, 85% spectral rolloff, spectral flatness, crest factor, and positive spectral flux. A bounded 12-second history derives energy movement, onset density, dynamic contrast, and a passage label (`quiet`, `impact`, `building`, `receding`, or `sustained`). The system does not claim tempo, BPM, key, genre, lyrics, or mood.

The director exposes ten controls:

- Music influence
- Photorealism
- Abstraction
- Chaos
- Weirdness
- Human presence
- Era freedom
- Color freedom
- Environmental complexity
- Novelty

The director constructs candidates from independent subject, environment, era, event, scale, weather, composition, and lighting vocabularies. The current combination space is deliberately much larger than a list of complete prompts. The active visualization contributes one formal influence—such as cylindrical depth, traveling material disturbance, or traceable reflection—without directly dictating a cliché subject.

Each candidate is compared with a bounded history that survives application restarts. Subject and environment matches count more strongly than lighting or composition matches, so changing surface treatment alone does not make a repeated concept “new.” The Novelty control changes how many candidates compete, and the highest-scoring candidate is selected.

Composing a brief appends its mode, concept, capture profile, output intent, aspect, requested size, novelty ID, and full prompt to a versioned local history file. The Instrument panel shows the eight most recent records and can copy any saved prompt. `Export brief .md` writes the full Visual DNA, scene specification, preflight scores, metadata, and prompt beside that history. On Windows the default path is `%LOCALAPPDATA%\TheVisualizer\visual-director-history.tsv`; `THEVISUALIZER_DIRECTOR_HISTORY` can select another file. Malformed records are skipped and reported without preventing visualization.

## Capture and realism

The panel offers Auto plus 16 intentional capture profiles: documentary, street, concert, editorial, amateur, smartphone, disposable camera, 35mm film, archival, surreal photorealism, industrial documentary, nature documentary, macro, aerial, architectural, and night photography.

The realism engine chooses only two or three physically plausible imperfections per brief—such as mixed color temperature, limited-access framing, residue, direct-flash falloff, restrained grain, or a foreground obstruction. It does not stack every photographic defect into every prompt.

Cliché suppression is contextual. For example, neon palettes require visible practical light sources, human scenes add anatomy and contact constraints, reflective modes require traceable surfaces, and photographic profiles reject concept-art polish. The system builds a specific ordinary environment and event first instead of appending one universal negative-prompt wall.

## Preflight critique

Before any provider call, the local preflight reports:

- Structured-concept novelty
- Physical plausibility
- AI-cliché risk
- Concrete construction notes

This is a deterministic prompt/concept review, not an image critic. Anatomy, reflections, lighting, material defects, and image similarity can only be evaluated after an image provider and vision-capable critic are connected.

## Output intent

The panel separates `Preview` and `Final` intent and supports Square, Landscape, Portrait, and Cinematic frames. Preview uses low quality; Final uses high quality. These are preparation settings only in the current build.

The intended first provider is OpenAI's Image API with `gpt-image-2`. The official API currently supports low, medium, high, or automatic quality; flexible bounded resolutions; and PNG, JPEG, or WebP output. It returns base64 image data and does not currently support transparent backgrounds for `gpt-image-2`. See the official [Image generation guide](https://developers.openai.com/api/docs/guides/image-generation).

## Cost and trust boundary

- No request runs automatically or in response to audio.
- No API secret is stored in source, presets, or scene briefs.
- One deliberate Generate action will equal one image when a provider is implemented.
- Preview should remain the default and use low quality.
- Final generation, edits, critique, and retries require separate explicit actions.
- The current build disables Generate because no validated provider connection exists and `OPENAI_API_KEY` was absent during implementation.

## Still open

- API provider connection and response decoding
- User-visible cost estimate immediately before each request
- Saved generated-image history and reusable scene assets
- Reference-image editing and selective masks
- Optional image critique and bounded revision loop
- Perceptual-hash or embedding-based image similarity after real assets exist
- Generated-asset provenance and package policy
- Mapping generated 2D assets into the live CityScape renderer

The Responses API may later support conversational image editing, but it adds a mainline model step and its associated usage. The simpler Image API is the preferred first connection for one-shot generation; multi-turn editing should be added only when the interaction warrants it.
