# Potion UI Architecture

Potion is a native Honeydew rack instrument with its own custom panel tree. It does not use the desktop CLAP editor path or the Lemon plugin embedding path.

## Current editor

The panel is registered as instrument type 53 and rendered by Honeydew's normal rack panel system. It keeps stable module parameter indices underneath the custom interface, preserving the host's existing project state, undo, automation and module preset behavior.

The current pages are:
- **OSC 1 / 2 / 3:** level, pan, pitch, wavetable morph and enable for each oscillator.
- **TABLE:** a momentary LOAD WAV TABLE action that opens Honeydew's existing module sample picker.
- **FILTER:** cutoff, resonance, low/band/high-pass selection and filter-envelope depth.
- **ENVELOPE:** amp attack, decay, sustain and release.
- **MOD:** two LFOs, FM, ring modulation and warp controls.
- **VOICE:** unison count, detune, stereo spread, glide, voice mode and polyphony.
- **MIX:** sub, noise, velocity sensitivity and master level.
- **MATRIX:** four modulation routes, each with source, destination and signed depth.

The three oscillator traces and table preview are drawn in the panel from the oscillator morph values. They are design previews, not live DSP waveform captures. A real oscilloscope, spectral view, animated modulation ranges and an interactive frame editor remain future work.

## WAV table import path

The TABLE page uses a momentary, unbound text button, following the same action pattern as Honeydew's Slicer LOAD SAMPLE control. The rack recognizes that action and opens the module sample picker. The picker decodes the selected audio file off the audio thread, then the selected file is stored as the module's CSTM PATH. The sample cache provides the prepared periodic cycle when the engine is rebuilt.

The current importer converts one audio file into one cycle. It does not yet support multi-frame table banks, drawing, harmonic editing or WAV export.

## UI-to-audio contract

- All audible controls are normal Honeydew instrument parameters, not CLAP parameter IDs.
- The rack sends changed normalized values through the native Instrument interface. Note-on, note-off, reset, controller and slide events use the same interface.
- The WAV picker and sample cache handle file I/O and decode away from the audio callback. The Wavetable object and its mip levels are constructed as the instrument engine is built.
- The render path uses bounded voice/oscillator arrays and performs no allocation, blocking lock or file I/O.
- Device-specific CPU and audio-glitch behavior must still be checked on physical Android hardware.
