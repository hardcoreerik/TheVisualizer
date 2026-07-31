# GOAL: Continue TheVisualizer as a Best-in-Class Audio-Reactive Visual Instrument

You are continuing development of **TheVisualizer** in:

`F:\Ai\TheVisualizer`

GitHub repository:

`https://github.com/hardcoreerik/TheVisualizer`

The product is a modern-retro, Windows-first, standalone music and sound visualization application inspired by Winamp and its visualization ecosystem. It captures system audio or microphone input, analyzes it in real time, and renders interactive GPU-driven visuals. It is not a media player, generic dashboard, business-intelligence system, charting toolkit, or arbitrary-data analytics platform.

Treat this as an ongoing implementation goal. Do not merely return a plan when safe, useful work is available.

## Operating rules

Follow the repository's current `AGENTS.md` exactly.

In particular:

- Scout before editing.
- Protect `main`.
- Never merge, force-push, rebase a shared branch, delete a branch, or close a PR without explicit approval.
- Keep diffs narrow.
- Run the narrowest meaningful validation first.
- Ask before high-usage work such as full test suites, large package cycles, broad reviews, or multi-branch cleanup.
- Never claim a feature, platform, device, package, or runtime behavior works unless it was actually observed.
- Keep `PROJECT_TRUTH.md` synchronized with verified evidence.
- Keep `ROADMAP.md` authoritative for milestone status.
- Keep native-plugin trust warnings explicit: approval is not sandboxing.

Begin by checking the current branch, working tree, remote state, and the specific files relevant to the next task. Do not repeat broad repository archaeology unless current evidence shows it is necessary.

## Current implemented baseline

Before changing anything, verify this baseline against the current repository rather than assuming this handoff is perfectly current:

- Rust desktop application using `winit`, `wgpu`, and `egui`.
- Windows WASAPI system-loopback and microphone capture.
- Shared normalized waveform, spectrum, RMS, peak, low, mid, and high-frequency features.
- Automatic Windows default-output following plus explicit pinned-device selection.
- Clear waiting, live, silent, and capture-error states.
- Windowed, borderless, and fullscreen presentation.
- Display-synchronized, 60 FPS, and 30 FPS frame-pacing choices.
- Ten supplied visualization modes:
  - Neon Scope
  - Particle Forge
  - Feedback Tunnel
  - Solar Bloom
  - Neon Horizon
  - Gravity Wells
  - Ripple Garden
  - Kaleido Reactor
  - Aurora Flow
  - TheVisualCityScape
- Eight declarative format-2 WGSL presets with mode-specific grouped controls.
- Each GPU preset currently exposes approximately 10–35 relevant controls rather than one generic control set.
- Per-band Full, Bass, Mid, and Treble color routing.
- Multiple palettes and Neon, Glossy, Matte, Metallic, and Glass material treatments.
- Slider and color-gradient controls in the Instrument panel.
- Up to eight interactive sound zones with position, band, radius, strength, and pin state.
- Compact diamond zone anchors with sparse audio-reactive range ticks instead of large enclosing circles.
- Camera yaw, pitch, and zoom controls.
- Named `.tvscene` saved views containing mode, controls, colors, materials, zones, and camera state.
- A versioned trusted native-plugin ABI with one example plugin; native plugins remain disabled until explicitly approved.
- A local Visual Director prompt composer with structured Visual DNA, concept novelty, capture profiles, preflight critique, history, and Markdown export.
- The Visual Director is experimental and must remain outside the primary Instrument workflow. The Instrument panel should contain only one button that opens it in a separate window.
- No paid image-generation request path is currently connected.

The current feature branch may contain work not merged into `main`. Verify branch truth before continuing and never describe feature-branch work as landed on `main`.

## Primary objective

Continue transforming TheVisualizer into an unusually capable, polished, and memorable **audio-reactive visual instrument**.

Prioritize:

- Strong visual identity.
- Immediate audio responsiveness.
- Fluid interaction.
- Mode-specific creative control.
- Discoverable direct manipulation.
- Progressive disclosure.
- High-quality silence and error behavior.
- Stable performance.
- A compact player experience.
- Extensibility without speculative frameworks.
- Visual surprise without incoherence.

The application should feel enjoyable when simply left running and rewarding when actively manipulated.

Do not optimize for feature count. Improve the weakest part of the actual experience.

## Product boundary

The core input is live sound:

- music
- games
- browsers
- video playback
- microphones

These sources are treated uniformly through system-loopback or microphone capture. TheVisualizer does not need to play media.

Future MIDI, OSC, sensors, telemetry, and network values remain adapter possibilities, not permission to turn the application into a generic data-visualization suite today.

Do not implement maps, business dashboards, graph databases, tables, timelines, Sankey diagrams, or generic chart systems unless a later, explicitly approved input-adapter milestone creates a real requirement for them.

Retain useful principles from professional visualization systems:

- clear hierarchy
- linked controls and visuals
- progressive disclosure
- direct manipulation
- context-sensitive controls
- smooth transitions
- useful empty and error states
- responsiveness
- consistent visual language

Apply those principles to audio-reactive art, not enterprise analytics.

## Immediate priority order

Use repository evidence to confirm or adjust this order:

1. Stabilize and package the current ten-mode experience before adding more modes.
2. Run the expanded Windows package smoke test only with approval because it is a high-usage task.
3. Improve shared musical timing with a bounded onset/beat event signal that can benefit every mode. Do not claim BPM tracking until it is measured and validated.
4. Refine the ten existing modes for composition, motion, control relevance, silence behavior, performance, and visual distinctness.
5. Continue polishing TheVisualCityScape as the flagship interactive environment without importing copyrighted city imagery or promising true 3D assets that do not exist.
6. Improve saved-scene usability only where actual use demonstrates a need, such as rename, delete, or share workflows.
7. Treat image generation as a separate experimental production pipeline, not part of the normal playback controls.

Do not add visualization mode eleven merely because ten exist. Add another only when it offers a genuinely distinct visual and interaction model.

## Interaction and control direction

Each mode should expose controls that belong to that visual.

Examples:

- A particle mode may expose count, trails, turbulence, attraction, collision behavior, spread, and decay.
- A waveform mode may expose persistence, thickness, smoothing, echo layers, symmetry, and displacement.
- A city mode may expose traffic, windows, crowd activity, rooftop events, portal behavior, weather, and camera movement.

Do not force every mode into the same arbitrary list.

Controls should remain:

- grouped by purpose
- labeled plainly
- bounded
- resettable
- keyboard accessible where practical
- visible only when relevant

Use sliders, gradient bars, compact selectors, and direct manipulation where they communicate the value naturally.

Sound zones should feel like part of the visual language. Keep their normal representation restrained. Selection may reveal additional context, but debugging geometry must not dominate the artwork.

## Color and material direction

Continue supporting intentional per-band routing:

- Bass can be red while Treble is blue.
- Each audio band can have its own color.
- Palettes can establish coordinated starting points.
- Users can still override individual colors.
- Material finish can alter glow, highlight response, surface softness, reflection character, or contrast.

Avoid treating finish as a text label with no meaningful visual effect.

Preserve enough contrast for controls and selection markers to remain usable against varied scenes.

## TheVisualCityScape direction

TheVisualCityScape is the flagship tenth mode.

Its foundation is **The Neverending Party Block** combined with impossible geometry and an evolving **Pocket Metropolis** sky.

Its signature interaction remains:

- Bass routed to the hotel and architecture.
- Mids routed to the street party and crowd.
- Treble routed to the rooftop and sky detail.
- A fourth route placed in the sky as a hyperspace portal.
- The user can rotate, zoom, reposition routes, recolor bands, and change scene behavior.

The city should feel alive even when viewed passively, but it must not become visually unreadable.

Prefer procedural, original scene construction. Do not download or redistribute city photography, recognizable trademarked assets, or third-party artwork without a recorded licensing decision.

Do not claim true polygonal 3D, panoramic photography, generated city assets, or landmark fidelity unless those capabilities are actually implemented and verified.

## Visual Director and image-generation direction

The Visual Director began from a goal to create memorable, diverse, photorealistic imagery influenced intelligently by music while avoiding obvious AI-image clichés.

The current local implementation is only a prompt-composition laboratory. Preserve that truth.

It currently performs local, deterministic work such as:

- rolling audio observation
- Visual DNA derivation
- structured concept invention
- capture-profile selection
- controlled-imperfection selection
- prompt construction
- novelty comparison against local history
- preflight critique
- prompt copy
- Markdown export

It does not currently:

- call OpenAI or another image provider
- decode generated image responses
- store generated image assets
- inspect images with a vision model
- revise images
- ingest generated assets into a visualization

Keep Visual Director controls in a separate experimental window. Do not place its DNA table, scene specification, critique meters, long prompt, or history inside the normal Instrument panel.

The normal player must remain understandable without knowing Visual Director exists.

### Long-term staged image pipeline

When image generation becomes an approved active milestone, preserve this conceptual pipeline:

`MUSIC → AUDIO ANALYSIS → VISUAL DNA → CREATIVE DIRECTOR → SCENE DESIGN → CAPTURE/ART SPECIFICATION → PROMPT CONSTRUCTION → IMAGE GENERATION → OPTIONAL CRITIQUE → TARGETED REVISION → FINAL ASSET`

Do not collapse it into:

`music → one generic prompt → image`

### Visual DNA

Use measured audio information that genuinely exists:

- RMS and peak energy
- low, mid, and high-band balance
- spectral centroid
- spectral rolloff
- transient intensity
- onset density
- dynamics
- perceived movement
- quiet, sustained, building, impact, and receding passage states

Future tempo or BPM may be added only after a measured feasibility spike. Never invent unavailable genre, mood, lyric, or metadata.

Visual DNA should influence physical properties rather than select clichés.

Examples:

- Energy may affect motion, subject activity, distance, exposure pressure, or environmental disorder.
- Spectral brightness may affect reflectivity, illumination hardness, texture scale, or atmospheric clarity.
- Bass dominance may affect apparent mass, architecture, geology, machinery, or camera height.

Do not encode rules such as high BPM equals cyberpunk, bass equals red, or slow music equals sunset.

### Creative direction

Invent the concept before writing the final prompt.

Vary:

- subject
- environment
- era
- location
- scale
- event
- action
- camera position
- composition
- light source
- weather
- materials
- surface condition
- color relationships
- atmosphere
- realism level
- capture medium
- controlled imperfections
- visual story

Favor memorable and believable surprise over generic beauty.

Allow ordinary locations, awkward moments, industrial spaces, ugly weather, mundane objects, asymmetry, environmental wear, unusual occupations, and strange scale.

Avoid automatically producing neon cities, astronauts, robots, hooded hackers, glowing skulls, futuristic streets, or other repeated generated-image shorthand.

### Photographic realism

When photorealism is selected, construct a plausible photograph rather than generic concept art.

Possible capture profiles include:

- documentary
- street photography
- concert photography
- editorial
- amateur snapshot
- smartphone
- disposable camera
- 35mm film
- archival photograph
- surreal photorealism
- industrial documentary
- nature documentary
- macro
- aerial
- architectural
- night photography

Use believable materials, wear, dirt, weathering, fabric, skin texture, imperfect pavement, water residue, rust, stains, ordinary clutter, and physically plausible light.

Select only a small compatible subset of imperfections such as motion blur, imperfect framing, mixed color temperature, foreground obstruction, mild underexposure, clipped practical lights, high-ISO noise, grain, or missed focus.

Do not add every imperfection to every image.

### AI-cliché suppression

Avoid constructing scenes that naturally produce:

- plastic skin
- waxy faces
- excessive symmetry
- centered poster composition
- constant bokeh
- unnecessary volumetric fog
- excessive bloom
- permanent teal/orange grading
- impossibly clean environments
- impossible shadows or reflections
- repetitive architecture
- meaningless signs
- generic cyberpunk
- generic astronauts
- generic glowing eyes

Do not solve this by appending an enormous universal negative-prompt list.

### Composition

Support genuine variation:

- extreme wide
- environmental portrait
- medium
- close
- extreme close
- top-down
- low angle
- elevated
- aerial
- foreground-obstructed
- partially cropped
- off-center
- asymmetrical
- distant subject
- environmental dominance
- negative-space-heavy
- crowded
- compressed perspective

Do not center the primary subject by default.

### Novelty

Retain structured history so repeated subjects, places, eras, camera positions, palettes, compositions, lighting setups, and tropes can be discouraged.

Treat concept similarity and image similarity as separate future problems.

Do not add embeddings, perceptual hashes, or a vector database until generated images actually exist and a simpler method has measurably failed.

### Optional critic and revision

Future post-generation critique may evaluate:

- physical plausibility
- anatomy
- lighting
- materials
- environment
- composition originality
- concept originality
- AI-cliché probability
- similarity to recent work

Criticism should identify concrete defects and produce targeted edit instructions.

Do not regenerate a strong image merely because one repairable defect exists.

This remains future work until a provider-backed generation path exists.

### Provider integration gate

Before connecting any paid image provider:

1. Research current official provider documentation.
2. Verify the current OpenAI image model name and capabilities; do not assume an older model label or parameter set is still correct.
3. Verify supported sizes, quality modes, editing behavior, response formats, rate limits, and pricing controls.
4. Keep provider-specific code isolated at the external boundary without building a speculative multi-provider framework.
5. Keep API secrets out of source and logs.
6. Add explicit preview/final cost controls.
7. Make critique and revision opt-in.
8. Handle rate limits and failures without affecting audio capture or rendering.
9. Record generation metadata.
10. Obtain explicit approval before exercising a paid request path.

For widescreen output, use a provider-supported native landscape size when available. Do not promise 2560×1440 generation unless current official documentation supports it.

## Audio and rendering engineering constraints

- Treat audio callbacks as real-time boundaries.
- Do not block, render, perform network activity, or allow unbounded allocation inside capture callbacks.
- Preserve one shared normalized feature path for built-ins, presets, and plugins.
- Keep platform-specific capture outside shared analysis and rendering.
- Bound histories and queues.
- Measure latency before fixing thresholds or buffer sizes.
- Keep rendering usable through source changes, silence, resize, and presentation transitions.
- Preserve the last working preset when a replacement fails validation.
- Reject malformed preset and scene files at their trust boundaries.

## Native-plugin safety

- Native plugins execute with the user's process privileges.
- Approval is not sandboxing.
- Unknown plugins remain disabled.
- The host owns audio capture, the window, GPU resources, and frame lifecycle.
- Do not expose raw GPU or window handles in the v0.1 ABI.
- Do not market manifests, hashes, or approval prompts as security isolation.

## Performance and visual validation

For visual changes:

1. Build the narrowest affected target.
2. Launch the application.
3. Navigate to the affected mode.
4. Exercise the interaction.
5. Inspect the actual rendering.
6. Check relevant viewport sizes.
7. Verify live, silent, waiting, and error behavior where relevant.
8. Check runtime errors.
9. Refine obvious weaknesses.

Do not declare visual work complete because it compiles.

Do not run repeated full suites or packaging loops without approval.

## Documentation discipline

Update existing documents rather than creating overlapping specifications.

Use:

- `PROJECT_TRUTH.md` for verified implementation and runtime evidence.
- `ROADMAP.md` for milestone status.
- `docs/ARCHITECTURE.md` for system boundaries.
- `docs/INTERACTIVE_MODES.md` for interactions and controls.
- `docs/VISUAL_DIRECTOR.md` for the image-brief laboratory and future provider boundary.
- `docs/PRESETS_AND_PLUGINS.md` for extension contracts and safety.
- `docs/OPEN_QUESTIONS.md` for unresolved research.

Never convert a proposal into an implemented claim without evidence.

## Quality standard

Evaluate meaningful changes against:

- **Audio response:** Does the visual respond clearly and musically?
- **Composition:** Is the frame visually coherent?
- **Interaction:** Can users discover and manipulate it naturally?
- **Control relevance:** Do controls belong to the active mode?
- **Performance:** Does it remain fluid on verified hardware?
- **Silence behavior:** Does it settle intentionally instead of looking broken?
- **Error behavior:** Does the user understand what failed?
- **Consistency:** Does it belong to TheVisualizer?
- **Novelty:** Does it offer something distinct without becoming random noise?
- **Truthfulness:** Are claims supported by tests or observation?

## Execution loop

Use:

`SCOUT → SELECT ONE HIGH-VALUE GAP → IMPLEMENT NARROWLY → RUN TARGETED CHECKS → LIVE-INSPECT WHEN VISUAL → REFINE → SYNC TRUTH → REPORT`

Operate autonomously inside the selected task.

Ask only when:

- a decision materially changes product direction
- required information cannot be discovered
- an action is destructive or difficult to reverse
- a paid external action is required
- AGENTS.md classifies the work as high usage
- a merge or other protected Git action needs approval

Do not create broad frameworks for hypothetical requirements.

Do not keep adding controls, modes, panels, or AI features merely because they are possible.

The standard is not the number of features. The standard is whether TheVisualizer feels like an exceptional audio-reactive instrument.

## Initial action for the new session

1. Read the repository `AGENTS.md`.
2. Scout the current branch, working tree, remote tracking, and relevant diffs.
3. Read `PROJECT_TRUTH.md`, `ROADMAP.md`, and only the task-relevant architecture documents.
4. Confirm whether the latest feature branch is merged or still separate from `main`.
5. Identify the highest-value unblocked gap in the core listening and interaction experience.
6. State the narrow task, risk, tests, and approval requirement.
7. Continue into implementation unless approval is required.

Do not begin with another broad feature expansion.

Begin by making the current experience more coherent, musical, reliable, and polished.
