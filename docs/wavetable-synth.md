# Android Wavetable Synthesizer

## Goal

Build a first-party Honeydew instrument for Android with a touch-first workflow and a feature set informed by Vital and Serum 2. This is an original implementation: no copied UI, source code, presets, factory wavetables, or proprietary assets from either product.

The synth is a native Honeydew instrument (not a desktop plugin host dependency), so it runs in the Android audio engine and its parameters are saved with the song.

## Feature targets

### Wavetable voice engine
- Three independently enabled wavetable oscillators with frame position, pitch, phase, level, pan, unison, detune, stereo spread and per-oscillator routing.
- High-quality band-limited wavetable playback, frame interpolation, mip levels and anti-aliasing.
- Wavetable import/export, table browsing, frame editing, drawing and harmonic editing.
- Wave and spectral warp families, including bend, sync, mirror/fold, quantize, phase, formant and harmonic/spectral operations.
- Oscillator cross-modulation: FM, phase modulation, ring/amplitude modulation and hard sync, with clear CPU-cost controls.
- Sub oscillator and noise source.

### Voice, filters and modulation
- Poly, mono and legato modes; configurable voice limit, glide/portamento, velocity, pitch bend, aftertouch and MPE expression.
- Multiple filter models with serial/parallel routing, key tracking and drive.
- Flexible multi-stage envelopes with curves/tension and looping.
- Editable tempo-synced/free-running LFOs, key tracking, one-shot and stereo split modes.
- Touch-friendly modulation source/destination assignment on phones and tablets.
- Modulation matrix with per-route amount, polarity, curve/remap and smoothing; audio-rate routing only where the DSP can safely support it.
- Random and chaos modulation sources.

### Sample-based synthesis and performance
- Sample oscillator with loop, slice and modulation controls.
- Multisample instrument support using SFZ as an open interchange format.
- Granular playback and spectral/resynthesis modes as later engine modules.
- Arpeggiator and phrase/clip sequencer.
- Microtuning import (.tun, .scl, .kbm) and MPE support.

### Effects, metering and preset workflow
- Internal effects rack with reorderable routing, including distortion, chorus, phaser, delay, reverb, compressor and EQ.
- Oscilloscope, spectrum and filter-response views designed for Android.
- Preset browser with search, tags, favorites, categories, save/import/export and versioned native preset format.
- Preset migration that reports unsupported or approximated features rather than silently substituting them.

## Implementation order

1. **Core playable instrument:** three oscillators, band-limited wavetable playback, unison, pitch/level/pan, amp envelope, voice allocation, filter and native Honeydew parameter/song integration.
2. **Sound design foundation:** wavetable import/export/editor, warp modes, dual filters, filter envelope, LFOs and modulation matrix.
3. **Performance and polish:** MPE, microtuning, preset browser, visual meters and Android tablet/phone layouts.
4. **Serum/Vital-adjacent synthesis modes:** sample, multisample/SFZ, granular and spectral engines; arpeggiator and phrase sequencer.
5. **Internal effects rack and compatibility expansion:** effect routing, preset migration, CPU/memory profiling and device testing.

Every stage must pass Android ARM64 build checks and audio-engine tests before the next stage is considered complete. Performance budgets will be measured on Android hardware; desktop results are not the acceptance target.

## Serum preset compatibility

Serum 1 presets commonly use VST2 .fxp chunks, while Serum 2 adds its own .SerumPreset format. Xfer has stated that the Serum preset encoding is not publicly documented. Therefore, full, lossless native loading cannot be promised up front.

The compatibility strategy is:
- First support interoperable assets such as wavetable WAV files and SFZ multisample definitions.
- Investigate a legally and technically supportable importer for Serum 1 .fxp and Serum 2 .SerumPreset.
- If a parser can be implemented and validated, map supported oscillator, envelope, filter, LFO, effect and modulation values into Honeydew's native patch format.
- Clearly list missing dependencies (for example, external wavetables/noise samples) and features that cannot be mapped. Never claim a converted preset is identical when it is only an approximation.
- Do not bundle Xfer factory content or bypass any product protection.

## Android constraints

The synth must be real-time safe: no allocation, file I/O, blocking locks or expensive table generation in the audio callback. Import, analysis, wavetable construction and preset parsing happen off the audio thread. The renderer uses fixed voice/resource limits, graceful quality settings and memory-aware caches suitable for phones and tablets.
