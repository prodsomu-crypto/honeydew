# Potion visual editor

Potion is a native Honeydew rack instrument with a dedicated, touch-interactive graphical workspace. It is registered as instrument type 53 and rendered in the rack using Honeydew's egui panel system. The visual controls edit the same normalized module parameters consumed by the audio engine, preserving project state, automation and host undo behavior.

## Workspace

The editor is arranged as eight full-width pages rather than exposing the instrument as a list of generic parameter controls:

- **OSC:** three oscillator cards with waveform previews, enable/bypass, level, pan, pitch and table-position controls.
- **TABLE:** oscillator selection, morph control, factory wave regions and a table import button. Imported single-cycle files are expanded into a 16-frame harmonic-scan bank, from a fundamental-only frame to the full source spectrum.
- **FILTER:** visual response curve, cutoff, resonance, envelope amount, signed keyboard tracking, velocity tracking and low-pass / band-pass / high-pass / notch modes.
- **ENV:** plotted amplitude ADSR and independent filter-envelope controls.
- **MOD:** two free-running or host-tempo-synced LFOs with seven beat divisions, shape, rate, depth and destinations including per-oscillator pitch; FM, ring modulation and warp controls.
- **VOICE:** visual unison spread, detune, stereo width, glide, adjustable 1–24 semitone pitch-bend range, poly/mono/legato and polyphony.
- **MIX:** oscillator level bars, sub, noise, velocity response and master.
- **MATRIX:** four source/destination/depth modulation routes.

The graphs and hit targets are drawn in one custom workspace. Sliders accept direct click/drag and buttons accept taps, so the same controls work with mouse, pen and touch input. The rack's expand button opens a Lemon-style fullscreen overlay: a dimmed backdrop, full-width editor canvas, eight large page tabs and a close control in the upper corner. The fullscreen view edits the same module parameters and sends live parameter updates to the running engine. The wavetable import action opens the existing module sample picker over the fullscreen editor.

## Wavetable and sample limits

The factory table contains four generated frames (sine, triangle, saw and pulse/square-like). Importing a custom single-cycle file adds 16 spectral frames derived with an FFT harmonic-cutoff sweep; the first is fundamental-only and the final frame retains the full imported spectrum. All table analysis and mip generation happen during the module build, not on the render callback. The waveform plot is still a visual model of the morph position, not a live oscilloscope reading of the DSP voice.

This is not yet a full multi-frame wavetable authoring suite: freehand frame drawing, per-frame harmonic/spectral editing, frame-bank management, wavetable export and animated live spectrum are not implemented. The TABLE page's factory wave selections set morph positions; they do not create or edit new arbitrary frames.

## Audio and modulation

The native DSP currently provides three wavetable oscillators, mipmapped wavetable sampling, oscillator level/pan/pitch/morph, unison/detune/spread, FM, ring modulation, warp modes, sub/noise, a state-variable low/band/high-pass/notch filter, amplitude and filter ADSRs, two free-running or tempo-synced LFOs, four modulation routes (including per-oscillator pitch), filter key/velocity tracking, adjustable pitch-bend range, poly/mono/legato behavior, glide, aftertouch, mod wheel and slide response.

The processing path uses bounded voice/oscillator arrays and avoids file I/O and allocation on the audio callback. Physical-device CPU and audio-glitch behavior still requires verification on actual Android hardware.

## Not included

MPE and preset formats are intentionally out of scope. Full Serum/Vital feature parity is not claimed: advanced wavetable authoring and spectral processing, broader filter/oscillator models, a full effect rack, advanced modulators/sequencers and physical-device performance verification remain separate work.
