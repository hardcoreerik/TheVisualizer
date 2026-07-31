# Event Horizon motion sources

Status: three runtime motion clips plus seven high-resolution source frames. The PNGs are visual
development assets; the player renders the animated WebP clips, not panned still backgrounds.

The source frames were generated locally for TheVisualizer on 2026-07-29. The motion clips were
then generated locally from those frames with the installed ArtForge/ComfyUI LTX-2 image-to-video
pipeline and converted to bounded 1024×576, 12 FPS animated WebP assets. No network API is used at
runtime.

| Runtime clip | Musical role |
| --- | --- |
| `event-horizon-orbit-motion.webp` | Calm default orbit with moving plasma, stars, nebula gas, and lensing |
| `event-horizon-events-motion.webp` | Onset/transient state with a moving comet, separate ion/dust tails, and gravity distortion |
| `event-horizon-crossing-motion.webp` | Rare sustained-energy climax approaching and crossing the horizon |

Only one texture is advanced and uploaded at a time. The existing GPU preset remains layered above
the footage for live spectrum detail, waveform comet tails, gravity waves, and continuous musical
control. Event and crossing clips play forward and hold their resolved frame; the calm orbit
ping-pongs to avoid a visible loop cut.

## Source-frame roles

| Source | Intended role |
| --- | --- |
| `event-horizon-hero.png` | Stable observer view and black-hole optical anchor |
| `nebula-depth-field.png` | Gas and dust environment |
| `comet-waveform-field.png` | Comet event and waveform-tail direction |
| `gravity-wave-field.png` | Coherent star-field lens distortion |
| `horizon-crossing-pov.png` | First-person crossing composition |
| `cosmic-object-parallax.png` | Recognizable space objects and depth reference |
| `astronomical-object-atlas.png` | Future additive-object reference sheet |

## Scientific visual anchors

- A far-side accretion disk lenses into arcs above and below the black-hole shadow.
- Relativistic Doppler beaming makes the approaching disk side brighter.
- A comet's ion tail is narrower, straighter, and bluer than its curved dust tail.
- Gravity waves distort the star field coherently rather than appearing as water or neon rings.
- A crossing compresses and aberrates the external sky; it is not a wormhole or fantasy portal.

## Research references

- [NASA black-hole accretion disk visualization](https://svs.gsfc.nasa.gov/13326/)
- [NASA 2024 black hole with accretion disk visualization](https://svs.gsfc.nasa.gov/14619/)
- [LIGO gravitational-wave science](https://ligo.org/gravitational-wave-science/)
- [NASA gravitational-wave visualization](https://svs.gsfc.nasa.gov/20367/)
- [NASA comet tail notes](https://science.nasa.gov/solar-system/comets/nov2024-night-sky-notes/)
- [ESA Hubble Carina Nebula](https://www.esa.int/ESA_Multimedia/Images/2010/04/Hubble_captures_spectacular_landscape_in_the_Carina_Nebula)
