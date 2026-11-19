# Potion visual editor

Potion is a native Honeydew rack instrument with a dedicated, touch-interactive graphical workspace. It is registered as instrument type 53 and rendered in the rack using Honeydew's egui panel system. The visual controls edit the same normalized module parameters consumed by the audio engine, preserving project state, automation and host undo behavior.

## Workspace

The editor is arranged as eight full-width pages rather than exposing the instrument as a list of generic parameter controls:

- **OSC:** three oscillator cards with waveform previews, enable/bypass, level, pan, pitch and table-position controls.
- **TABLE:** oscillator selection, single-cycle preview, morph control, factory wave regions and an import button in the table editor itself.
- **FILTER:** visual response curve, cutoff, resonance, envelope amount and low-pass / band-pass / high-pass mode.
- **ENV:** plotted amplitude ADSR and independent filter-envelope controls.
- **MOD:** two LFO shape previews, rate, depth and named destinations; FM, ring modulation and warp controls.
- **VOICE:** visual unison spread, detune, stereo width, glide, poly/mono/legato and polyphony.
- **MIX:** oscillator level bars, sub, noise, velocity response and master.
- **MATRIX:** four source/destination/depth modulation routes.

The graphs and hit targets are drawn in one custom workspace. Sliders accept direct click/drag and buttons accept taps, so the same controls work with mouse, pen and touch input. The rack's expand button opens a Lemon-style fullscreen overlay: a dimmed backdrop, full-width editor canvas, eight large page tabs and a close control in the upper corner. The fullscreen view edits the same module parameters and sends live parameter updates to the running engine. The wavetable import action opens the existing module sample picker over the fullscreen editor.

## Wavetable and sample limits

The current factory table contains four generated frames (sine, triangle, saw and pulse/square-like). The waveform plot is a visual model of the morph position, not an oscilloscope reading the DSP voice. The current import route loads a single audio cycle through the existing module sample picker and prepares it for the oscillator engine off the audio callback.

This is not yet a full multi-frame wavetable authoring suite: freehand frame drawing, per-frame harmonic/spectral editing, frame-bank management, wavetable export and animated live spectrum are not implemented. The TABLE page's factory wave selections set morph positions; they do not create or edit new arbitrary frames.

## Audio and modulation

The native DSP currently provides three wavetable oscillators, mipmapped wavetable sampling, oscillator level/pan/pitch/morph, unison/detune/spread, FM, ring modulation, warp modes, sub/noise, a state-variable low/band/high-pass filter, amplitude and filter ADSRs, two LFOs, four modulation routes, poly/mono/legato behavior, glide, pitch bend, aftertouch, mod wheel and slide response.

The processing path uses bounded voice/oscillator arrays and avoids file I/O and allocation on the audio callback. Physical-device CPU and audio-glitch behavior still requires verification on actual Android hardware.

## Not included

MPE and preset formats are intentionally out of scope. Full Serum/Vital feature parity is not claimed: advanced wavetable authoring and spectral processing, broader filter/oscillator models, a full effect rack, advanced modulators/sequencers and physical-device performance verification remain separate work.
