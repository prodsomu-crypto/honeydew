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

## Current implementation status

- Added `src/content/sounds.rs` with provider-neutral asset metadata, the six browser categories, CC0/CC-BY/CC-BY-NC licence parsing, attribution formatting, and validation of provider metadata/HTTPS URLs.
- Added `src/content/freesound.rs` with Freesound search-response DTOs, a bounded/encoded search URL builder, preview selection, tag-based category hints, and conversion into validated provider-neutral assets.
- Added asynchronous search transport using `ehttp` (native and wasm-compatible), with the API key sent in an Authorization header, a 20-second timeout, a 2 MiB response-body guard, and user-safe error messages. The caller supplies the key; this layer does not persist or log it.
- Exposed both modules as `crate::content::sounds` and `crate::content::freesound`.
- Added adapter tests for URL encoding/pagination bounds, CC-BY metadata conversion, attribution retention, and rejection of ND or preview-less results.
- Added unit tests for licence eligibility, attribution, insecure URLs, and invalid duration metadata.
- Licence parsing recognizes canonical Creative Commons CC0, CC-BY 2.0/3.0/4.0, and CC-BY-NC 2.0/3.0/4.0 URLs as well as short identifiers. NC assets are allowed for this personal-use project but always require attribution; ND variants remain excluded.
- Confirmed the existing `src/ui/views/sample_picker.rs` path already handles worker-thread decoding, waveform preview, and applying imported audio to pads, clips, and supported modules. Honeydew Sounds should hand downloaded files into that path rather than create another preview engine.
- Not yet implemented: local credential storage/settings UI, search UI, preview transport, download/cache, and playlist import. The asynchronous transport exists but has not been wired into the UI or verified with a build; this is still not a usable browser.

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
- Freesound is the selected first provider. Its API is documented as free for non-commercial use, requires an API credential for search, and exposes preview URLs without OAuth2; original-quality downloads require OAuth2. The app must use a locally configured user API key, never a key committed to source or embedded as a shared secret.
- Because this is a personal-use project, include CC0, CC-BY, and CC-BY-NC assets. Preserve attribution for both BY variants and clearly label NC assets as non-commercial. Continue rejecting ND and unknown licences; a free download is not automatically unrestricted.
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

1. Confirm provider API/authentication and the exact licence filter; write provider metadata types and tests. (Done as transport-free adapter groundwork; live search still requires a local API credential.)
2. Add private local credential handling and wire the async transport into a cancellable search UI. The API key is supplied at call time and is never put in the URL or persisted by this module.
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
