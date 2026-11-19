# Potion — Android Wavetable Synth

Potion is a first-party Honeydew rack instrument (module type 53). It is a native Honeydew audio-engine instrument, not a standalone CLAP/VST plugin and not a desktop plugin-host dependency.

## Implemented in the current branch

### Oscillators and tables
- Three independently enabled oscillators with level, pan, semitone pitch and wavetable position controls.
- Factory morph bank: sine, triangle, saw and square. When a custom source is loaded, its imported cycle is appended as a fifth frame.
- Wavetable frame interpolation and FFT-derived mip levels for frequency-dependent anti-aliasing.
- Import of one audio file as a single-cycle table using the module's WAV table picker. WAV, FLAC, OGG and MP3 are listed by the picker; the existing audio decoder averages stereo and resamples the beginning of the file into one periodic cycle. The imported cycle is selected by moving that oscillator's Wave Morph control to the end of its range.
- The source path is stored in module CSTM data and the table is prepared through Honeydew's sample cache before the engine is built.

### Voice, filter and performance
- Up to 32 voices, with user-selectable polyphony, plus poly, mono and legato modes.
- Per-oscillator unison up to seven voices, detune and stereo spread.
- Glide, pitch bend, host aftertouch, mod wheel and host slide offsets.
- Amp attack, decay, sustain and release.
- A state-variable filter with low-pass, band-pass and high-pass modes, resonance, key tracking and envelope modulation.
- Sub oscillator and seeded noise source.
- Two free-running LFOs (sine, triangle or square) with pitch, filter or pan destinations.
- Four modulation routes. Sources are LFO 1, LFO 2, amp envelope, velocity, mod wheel or aftertouch. Destinations are pitch, filter, pan or oscillator 1/2/3 level.
- Oscillator 2 frequency-modulates oscillator 1; a ring-modulation blend and three basic warp modes (bend, fold and phase remap) are provided.

### UI and presets
- A dedicated touch-friendly rack editor with oscillator, table import, filter, envelope, modulation, voice, mix and matrix pages.
- Native Honeydew module parameters remain the source of truth, so normal project save/undo/automation and module preset flows continue to apply.
- Module presets use Honeydew's `.flmpst` format. Imported audio files are referenced from the module's CSTM path; keep the original file available when moving a song or preset to another device.

## Current limitations

- Audio import creates one single-cycle frame. It does not parse a multiframe wavetable container or a bank of frames from one file.
- There is no drawing, harmonic editor, frame management UI, or table export yet.
- The waveform graphic is a UI preview derived from the morph control; it is not a live oscilloscope or a live tap of DSP output.
- The filter is one SVF per voice, not a dual-filter/serial/parallel routing section. Drive and additional filter models are not implemented.
- LFOs are free-running; there is no tempo sync, one-shot mode, random/chaos source, route curve or dedicated modulation smoothing editor.
- The warp choices are basic approximations, not a complete spectral/formant/harmonic warp suite.
- Host pitch bend, channel aftertouch, mod wheel and slide are supported, but full MPE per-note expression is not.
- Microtuning, arpeggiator/phrase sequencer, granular and spectral resynthesis, and an internal reorderable effects rack are not implemented in this branch.
- Serum 1 `.fxp` and Serum 2 `.SerumPreset` import is not implemented. Preset compatibility is limited to Honeydew's native module preset format plus imported audio cycles; no Xfer factory content or proprietary preset decoding is bundled.

## Real-time and verification requirements

Wavetable construction, audio decoding and table preparation happen outside the audio callback. The renderer uses fixed voice storage and does not allocate, lock or perform file I/O while rendering. The Android ARM64 CI workflow is the build acceptance check; auditioning and CPU profiling on a physical Android device are still required before claiming device-wide performance validation.
