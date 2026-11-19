# Potion — Android Wavetable Synth

Potion is a first-party Honeydew rack instrument (module type 53). It is implemented in Honeydew's native audio engine and is available on Android; it is not a standalone CLAP/VST plugin.

## Implemented

### Oscillators and wavetable
- Three independently enabled wavetable oscillators with level, pan, semitone pitch and table-position controls.
- Four factory frames: sine, triangle, saw and pulse/square-like.
- Import of one audio file as a single-cycle source. The imported cycle is expanded into a 16-frame harmonic-scan bank, giving a 20-frame bank in total (four factory frames plus sixteen imported spectral frames).
- Frame interpolation and FFT-derived mip levels for frequency-dependent anti-aliasing.
- WAV, FLAC, OGG and MP3 sources are accepted by the existing module sample picker. Audio decoding, resampling and table preparation happen outside the render callback; the source path is retained in the module's CSTM data.

### Voice, filter and performance
- Up to 32 voices with adjustable polyphony and poly, mono or legato modes.
- Per-oscillator unison up to seven voices, detune, stereo spread and glide.
- Pitch bend with adjustable range, host aftertouch, mod wheel and host slide offsets.
- Independent amplitude, filter and bipolar pitch envelopes.
- A state-variable filter with low-pass, band-pass and high-pass modes, resonance, notch, keyboard tracking, velocity tracking and envelope modulation.
- Sub oscillator and seeded noise source.
- Two LFOs with sine, rising saw, triangle, falling saw and square shapes. They can free-run or synchronize to host tempo at seven beat divisions, and target pitch, filter or pan.
- Four modulation routes. Sources are LFO 1, LFO 2, amp envelope, velocity, mod wheel or aftertouch. Destinations are pitch, filter, pan or oscillator 1/2/3 level.
- Oscillator 2 frequency-modulates oscillator 1; ring modulation and four basic warp choices (phase, fold, sine drive and sync) are available.

### Editor and project integration
- Eight-page touch-oriented editor: OSC, TABLE, FILTER, ENV, MOD, VOICE, MIX and MATRIX.
- The editor is available in the rack and in a dedicated fullscreen overlay. Both edit the same native module parameters and send changes through Honeydew's normal live audio command path.
- Project save, undo, automation and Honeydew module preset flows use the native module parameter state. Imported audio remains an external file reference, so keep the source file available when moving a project to another device.

## Known limitations
- Import currently creates one single-cycle source bank; multiframe wavetable containers and banks are not parsed.
- There is no freehand frame drawing, per-frame harmonic editor, frame-bank management UI or wavetable export.
- The waveform graphic is a preview derived from the morph control, not a live oscilloscope tap of DSP output.
- The filter is one state-variable filter per voice, not a dual-filter or serial/parallel routing section.
- The warp modes are basic shapes, not a full spectral/formant/harmonic warp suite.
- Full MPE per-note expression, microtuning, arpeggiator/phrase sequencing, granular or spectral resynthesis, and an internal reorderable effects rack are not implemented.
- Serum 1 .fxp and Serum 2 .SerumPreset import is not implemented. Preset compatibility is Honeydew's native module preset format plus imported audio cycles; no proprietary factory content or preset decoding is bundled.

## Verification
The latest Android CI run for this branch (0dd4b5034f9b61d40579eb7f78a2126a5cdee112) passed both cargo fmt --check and the Android ARM64 APK build. This confirms build integration, not physical-device audio quality or CPU performance; those still require listening and profiling on Android hardware.