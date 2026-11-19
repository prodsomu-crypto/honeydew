# Honeydew Sounds

## Goal

Honeydew Sounds is the in-app sound discovery and auditioning feature. It keeps catalogue browsing inside Honeydew, makes licence terms visible before use, and imports audio into the user's existing sample workflow. It is a sound-finding tool, not a social network, marketplace, or content mirror.

## Phase 1 implementation

- **Entry point:** Help → Honeydew Sounds opens the native overlay in the editor.
- **Catalogue:** Search Freesound and page through results; filter by Drums, Loops, Vocals, Instruments, FX, or Textures. Result rows show title, creator, category, duration, and licence.
- **Licence policy:** accept CC0, CC BY and CC BY-NC (including recognised 2.0/3.0/4.0 identifiers and canonical URLs); reject ND, unknown and malformed entries. CC BY and CC BY-NC attribution is retained. CC BY-NC is clearly marked for non-commercial/personal use.
- **Credential handling:** the user's API key is entered in the Honeydew Sounds panel, saved separately from ordinary Settings and songs, and sent as an Authorization header rather than as a URL query parameter. Desktop stores it in a private configuration file (0600 on Unix); Android stores it inside the app-private data directory. The app never embeds a shared Freesound key.
- **Network:** search runs on a worker thread. Native builds use Honeydew's existing HTTPS curl downloader; Android uses its existing HTTPS `HttpURLConnection` bridge. Authenticated search is restricted to the Freesound API origin and its response is size-limited.
- **Preview:** selected preview files are fetched away from the audio callback, decoded using Honeydew's existing sample decoder, and played through the existing preview command. Stop/close clears preview playback.
- **Import:** the preview copy can be imported into the user's `My Samples` folder. A text sidecar records title, creator, source page, licence name, and licence URL. Imported files remain outside the managed bundled `content` directory and can be used by the existing picker workflow.
- **Local sounds:** Browse installed sounds opens the existing sample picker, showing user samples, recordings, and available bundled/imported sample content. Picking a file uses the existing decode/waveform preview.

## Important boundaries

- The current integration imports **Freesound preview copies**, not the original-quality source downloads. Freesound's original download endpoint has different OAuth requirements and is not part of this phase.
- The online Freesound catalogue is enabled for desktop and Android. It is not enabled in web/iOS builds yet; their local sample browser remains available.
- Search requires the user to provide their own Freesound API key. Honeydew does not create accounts, proxy credentials, embed a project-wide secret, or bypass provider authentication.
- Results link to their canonical Freesound pages; Honeydew is not intended to mirror Freesound or replace its service.
- The preview import persists source/licence details alongside the file. It is the user's responsibility to respect the specific licence conditions in their own projects.

## Not yet in Phase 1

- Persistent offline catalogue/search cache and automatic cache reuse.
- Tempo/key analysis or automatic tempo/key matching.
- A searchable preset library for Honeydew instruments.
- Original-quality Freesound downloads and OAuth flow.
- Online provider support in web/iOS builds.
- Additional online catalogue providers.

## Architecture

- `src/content/sounds.rs`: provider-neutral categories, licences, attribution and metadata validation.
- `src/content/freesound.rs`: Freesound search URL builder, API response types, preview-host validation and conversion to neutral assets.
- `src/update.rs`: authenticated search transport reusing the existing native downloader.
- `src/android.rs` and `HoneydewActivity.java`: Android bridge for authenticated search requests.
- `src/settings.rs`: private local API-key storage, separate from serialised user preferences.
- `src/project_files/samples.rs`: imports validated preview files into `My Samples` with a provenance/licence sidecar.
- `src/ui/views/honeydew_sounds.rs`: catalogue UI, asynchronous search/download/decode state, preview and local-library view.
- `src/ui/views/mod.rs` and `src/ui/main_menu/mod.rs`: overlay registration and Help menu entry.

## Validation and testing

The Freesound adapter and metadata model include unit tests for search URL encoding, pagination bounds, accepted licences, attribution, invalid metadata, insecure URLs, unknown/ND licences, missing previews, and disallowed preview hosts. The repository's Android CI workflow runs formatting checks and builds the Android libraries/APK. The status of that workflow must be checked on the feature branch before treating this integration as build-verified.
