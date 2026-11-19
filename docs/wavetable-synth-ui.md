# Wavetable Synth UI Architecture

Honeydew's wavetable synth will use a custom embedded editor, not the generic parameter-knob page.

## Host integration

Use the same built-in CLAP integration path already used by Lemon Trio and the other bundled Lemon Studio instruments:
- Compile the synth into Honeydew as a linked built-in CLAP instrument.
- Use the embedded editor path from os-plugkit so the editor is painted inside Honeydew's own rack/editor surface on Android.
- Do not depend on desktop plugin scanning or third-party CLAP hosting.
- Keep a normal CLAP parameter model underneath the custom editor. This preserves host automation, parameter gestures, project state, preset recall and accessibility to generic host controls.

The custom editor is the primary interface. CLAP parameters are the stable control/state contract, not a replacement for the visual interface.

## Main screen

- Header: preset name, save, undo/redo, browser and master output.
- Oscillator area: three large waveform/wavetable displays with animated play position, frame-position control, enable, pitch, unison, warp and level.
- Modulation area: draggable or tap-to-assign source badges, animated modulation ranges and a matrix view.
- Filter area: interactive response curve, routing, cutoff, resonance, drive and filter selection.
- Bottom navigation: Voice, Mod, FX, Matrix and Global pages.
- On phones, use scrollable stacked panels and full-screen editors for wavetable editing. On tablets, show oscillator, filter and modulation areas together.
- Render visualizations from cached UI snapshots. Never run FFTs, wavetable construction, file parsing or allocations in the audio callback.

## UI-to-audio contract

- Every audible control maps to a stable normalized CLAP parameter ID.
- The editor sends parameter begin/value/end gestures through the host context.
- DSP reads atomically published parameter state and performs smoothing at audio rate where needed.
- Meter/waveform/spectrum data is published through bounded, non-blocking snapshots; the UI never locks the audio thread.
- Large assets (wavetable frames, samples, presets) are parsed and prepared off-thread, then swapped at safe block boundaries.
