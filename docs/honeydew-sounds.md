# Honeydew Sounds

## Goal

Add an in-app catalogue for discovering, auditioning, and importing legitimately reusable sounds from online libraries. The first release is a sound browser, not a marketplace, social feature, or new instrument.

The browser should feel native to Honeydew: search and filter in the app, audition before importing, and put a downloaded file directly into the existing playlist/sample workflows. It must work on desktop and Android.

## Existing Honeydew infrastructure to reuse

- `src/ui/modules_browser.rs` and the shared side-panel/picker patterns provide an established searchable browser UI.
- `src/ui/picker_preview.rs` already implements sample audition controls and waveform/progress presentation.
- `src/sample_cache.rs` and `src/ui/clips.rs` provide waveform/sample handling used by existing audio clips.
- `src/content/bundle.rs`, `src/content/install.rs`, and `src/content/web.rs` already handle the shipped sound library and downloads.
- `src/content/install.rs` owns and synchronizes its managed `content` directory. User-downloaded catalogue sounds must **not** be stored there, because sync intentionally removes files not in its manifest.

Before wiring the browser into the UI, inspect the current sample picker and project-file import path and reuse their established import behavior.

## Phase 1 scope

1. A dedicated **Honeydew Sounds** entry in the existing browser/navigation system.
2. Search by sound name and filter by category: Drums, Loops, Vocals, Instruments, FX, and Textures.
3. Result rows show the title, source/creator, duration, licence, and available tags.
4. Preview a result in-app without adding it to the project; stop preview cleanly when selection changes or the browser closes.
5. Download/import a result into a user-owned sound cache, then offer the existing playlist/sample import path.
6. Show attribution and licence details at the point of download and retain the metadata alongside the cached file.
7. Handle offline state, failed downloads, cancellation, and cache reuse without blocking the audio thread or UI.

## Provider and licensing rules

- Start with one provider behind a small provider interface; do not hardwire provider-specific JSON into UI code.
- Freesound is a candidate first provider, subject to confirming its current API authentication requirements and terms before implementation.
- Initially include only assets whose licence clearly permits the intended use. Prefer CC0 for the first pass; do not silently treat every free-to-download file as royalty-free or commercially reusable.
- Preserve the provider's canonical asset URL, creator attribution, licence identifier/URL, and any required attribution text.
- Never scrape provider pages or bypass API limits/authentication. Keep API credentials out of source control and avoid shipping a private secret in the app binary.
- If a provider's API key cannot safely be used by a standalone client, make the provider configurable or choose a source with a suitable public API rather than embedding a secret.

## Storage and audio behavior

- Store downloads in a dedicated user cache/library location, separate from the managed bundled `content` directory and from project files.
- Use safe filenames derived from stable asset IDs, validate file size/type, write to a temporary file, and atomically move into place after a successful download.
- Cache provider metadata with the file so attribution survives offline use and future provider API changes.
- Reuse Honeydew's supported audio decoding and existing sample import path. Do not introduce a second playback engine.
- Network and file work must run off the real-time audio thread. Preview should use the existing audio/sample preview architecture where possible.
- Do not promise BPM/key matching in Phase 1 unless the existing preview path can do it accurately without destructive changes. It can be a later enhancement.

## Explicit non-goals for Phase 1

- Community uploads, accounts, comments, likes, or a marketplace.
- Paid subscriptions or a proprietary sound-pack storefront.
- A new sampler or synthesizer.
- Automatic stem separation, AI sound generation, or cloud project storage.
- Downloading arbitrary files from arbitrary URLs without provider metadata and licence checks.

## Suggested implementation sequence

1. Confirm provider API/authentication and the exact licence filter; write provider metadata types and tests.
2. Add a provider client with pagination, search, category mapping, cancellation, and bounded responses.
3. Add a user-owned cache with metadata persistence, atomic downloads, and offline listing.
4. Build the browser using Honeydew's existing side-panel/search/preview conventions.
5. Connect Import to the existing audio-file import flow.
6. Add tests for licence filtering, metadata persistence, interrupted downloads, cache hits, and provider failures; then build/test desktop and Android targets.

## Acceptance criteria

- A user can search and filter catalogue results without leaving Honeydew.
- A result can be previewed and stopped without importing it.
- A permitted result can be downloaded once, imported into a project, and reused offline.
- Every cached item retains its source and licence/attribution information.
- No unlicensed/restricted result is presented as unrestricted.
- The feature does not modify the managed bundled sound library or block audio playback while network operations run.
