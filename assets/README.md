# Bundled media provenance

`living-photograph-greenhouse.png` is an original project asset generated on 2026-07-28 and revised with OpenAI's built-in image-generation tool for the `Living Photograph` Studio prototype. No external photograph, logo, trademark, or artist reference was supplied.

Prompt:

> A hyperreal, believable wide photograph of an old glass greenhouse immediately after a rainstorm, with wet iron frames, fogged and cracked panes, dense ordinary plants, a muddy floor, small puddles, hanging condensation, and one worn wooden worktable; no people. Photorealistic environmental documentary photography, 16:9 landscape, asymmetric off-center perspective through multiple depth planes, physically plausible overcast daylight, real wear, rust, dirt, algae, wet glass, peeling paint, water residue, imperfect pavement, and wood grain. Preserve useful dark and light regions for reactive overlays. No text, signage, logos, trademarks, watermark, neon lighting, fantasy objects, symmetrical poster composition, excessive bloom, fake volumetric fog, or plastic-clean surfaces.

Revision prompt:

> Edit this exact photorealistic rainy greenhouse photograph non-destructively. Preserve the camera position, 16:9 framing, greenhouse structure, panes, plants, muddy floor, worn table, all object placement, wet overcast lighting, natural colors, documentary realism, and image resolution as closely as possible. Add exactly one clearly visible, scene-appropriate real tree frog perched naturally on the near front-right edge of the worn wooden worktable, facing slightly toward camera, about palm-sized relative to the pots. The frog should be unmistakable but not oversized, with anatomically correct limbs and toes, subtle natural green-brown coloration, wet skin, physically accurate contact shadow, reflections, depth of field, and lighting matching the scene. Add no other animals, no insects, no bird, no fantasy elements, no text, no watermark, no dramatic spotlight, no saturated colors, and no compositional rearrangement. The result must still look like an unstaged real environmental photograph.

The image is distributed as original project content under the repository's Apache-2.0 license.

## Motion asset

`living-photograph-greenhouse-motion.webp` was generated locally on 2026-07-28 from the bundled still through ArtForgeStudio's ComfyUI image-to-video lane using LTX 2 19B FP8 and the LTX 2 distilled LoRA. The accepted source render is 49 frames at 24 FPS and 1024×576. The app plays those frames forward and backward for a seamless loop.

Prompt:

> Static locked-off documentary shot of the supplied rainy greenhouse, preserving its exact composition, architecture, table, pots, plants, visible tree frog, wet overcast light, perspective, scale, and natural textures. A strong natural gust makes the large foreground plants dance with broad rhythmic stem and frond motion. Small leaves flutter rapidly and hanging vines sway and rebound with realistic weight and inertia. Rain streams down the glass and puddles ripple. The frog stays perched on the table, breathing and blinking. The table and metal greenhouse frame remain completely rigid. Photoreal environmental footage, stable geometry, continuous realistic motion.

Negative prompt:

> subtitles, captions, typography, letters, words, writing, symbols, numbers, credits, labels, UI overlay, HUD, watermark, logo, camera movement, dolly, pan, zoom, cut, scene change, new objects, extra animals, missing frog, frozen foliage, static leaves, melting, morphing, rubbery warping, geometry drift, illustration, cartoon, fantasy effects, blurry

The accepted render removed the frog despite the prompt. The original frog and its immediate rigid table contact area were therefore restored with a small feathered source-image composite before animated WebP encoding. No external media was used.
