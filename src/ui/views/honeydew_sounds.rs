//! Honeydew Sounds: a native Freesound catalogue browser and a doorway to the existing local sample library.
//!
//! The online catalogue keeps provider metadata small and links back to Freesound. Preview audio is decoded away from
//! the audio callback; imported previews are written into the user's My Samples folder with a licence/source sidecar.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};

use eframe::egui;
use honeydew_audio::device::AudioCommand;

use super::{AppCtx, Nav, sample_picker};
use crate::content::sounds::{Category, SoundAsset};
use crate::ui::preset_picker::{PickerAction, PresetPicker};

const SEARCH_PAGE_SIZE: u8 = 30;
const MAX_PREVIEW_BYTES: u64 = 32 * 1024 * 1024;

#[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
use std::sync::Arc;
#[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
use std::thread;

#[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
struct PreviewReady {
    sample: honeydew_audio::sample::SampleData,
    bytes: Vec<u8>,
    extension: String,
}

#[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
type SearchResult = Result<(u64, Vec<SoundAsset>), String>;
#[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
type PreviewResult = Result<PreviewReady, String>;

/// State for the catalogue and the local sample browser.
#[derive(Default)]
pub struct State {
    query: String,
    category: Option<Category>,
    results: Vec<SoundAsset>,
    selected: Option<usize>,
    page: u32,
    total: u64,
    status: String,
    local_browser: bool,
    local_picker: Option<PresetPicker>,
    local_loading: Option<(PathBuf, Receiver<sample_picker::Decoded>)>,
    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    api_key: String,
    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    key_loaded: bool,
    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    search: Option<Receiver<SearchResult>>,
    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    preview: Option<(String, Receiver<PreviewResult>)>,
    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    cached_preview: Option<(String, Vec<u8>, String)>,
    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    playing: bool,
}

impl State {
    pub(super) fn stop_sample(&mut self, app: &mut AppCtx) {
        #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
        if self.playing {
            app.audio.send(AudioCommand::Preview(None));
            self.playing = false;
        }
        if let Some(picker) = &mut self.local_picker {
            picker.stop_sample();
            sample_picker::preview(picker, app.audio);
        }
    }

    pub(super) fn leave(&mut self, app: &mut AppCtx) {
        self.stop_sample(app);
        self.close_local(app);
        #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
        {
            self.search = None;
            self.preview = None;
        }
    }

    fn close_local(&mut self, app: &mut AppCtx) {
        if let Some(picker) = &mut self.local_picker {
            picker.closing();
            sample_picker::preview(picker, app.audio);
        }
        self.local_picker = None;
        self.local_loading = None;
        self.local_browser = false;
    }

    fn poll_local(&mut self, app: &mut AppCtx) {
        // Existing local sample picks use the same worker decode and waveform preview as Honeydew's other pickers.
        let local_result = self.local_loading.as_ref().map(|(_, receiver)| receiver.try_recv());
        if let Some(result) = local_result {
            match result {
                Ok(decoded) => {
                    let path = self.local_loading.as_ref().map(|(path, _)| path.clone()).unwrap_or_default();
                    self.local_loading = None;
                    let wave = decoded.wave;
                    if let (Some(picker), Some(wave)) = (&mut self.local_picker, wave) {
                        sample_picker::show_strip(picker, &path, Some(wave));
                    } else if let Some(error) = decoded.error {
                        self.status = error;
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    self.local_loading = None;
                    self.status = "Could not decode the selected local sound.".into();
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(picker) = &mut self.local_picker {
            sample_picker::preview(picker, app.audio);
        }
    }

    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    fn begin_search(&mut self) {
        use crate::content::freesound::{authorization_value, search_url, validate_query};
        use crate::update;

        let query = self.query.trim().to_owned();
        if self.page <= 1 {
            self.results.clear();
            self.selected = None;
            self.total = 0;
            self.cached_preview = None;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let key = self.api_key.clone();
        let page = self.page.max(1);
        let started = (|| {
            validate_query(&query).map_err(|e| e.to_string())?;
            let authorization = authorization_value(&key).map_err(|e| e.to_string())?;
            Ok::<_, String>((authorization, search_url(&query, page, SEARCH_PAGE_SIZE)))
        })();
        match started {
            Ok((authorization, url)) => {
                self.status = "Searching Freesound…".into();
                self.search = Some(rx);
                thread::spawn(move || {
                    let result = (|| {
                        let path = temporary_file("search");
                        let downloaded = update::get_with_header(&url, &path, &authorization, &|_| ())
                            .map_err(|err| err.to_string());
                        let result = downloaded.and_then(|_| {
                            let size = std::fs::metadata(&path).map_err(|err| err.to_string())?.len();
                            if size > 4 * 1024 * 1024 {
                                return Err("Freesound returned a catalogue response that is too large.".into());
                            }
                            let bytes = std::fs::read(&path).map_err(|err| err.to_string())?;
                            let response: crate::content::freesound::SearchResponse =
                                serde_json::from_slice(&bytes).map_err(|err| format!("Could not read Freesound results: {err}"))?;
                            let assets = response.results.into_iter().filter_map(|result| result.into_asset()).collect();
                            Ok((response.count, assets))
                        });
                        let _ = std::fs::remove_file(&path);
                        result
                    })();
                    let _ = tx.send(result);
                });
            }
            Err(err) => self.status = err,
        }
    }

    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    fn begin_preview(&mut self, asset: &SoundAsset) {
        use crate::update;

        let id = asset.id.clone();
        let url = asset.download_url.clone();
        let extension = preview_extension(&url).to_owned();
        let (tx, rx) = std::sync::mpsc::channel();
        self.preview = Some((id, rx));
        self.cached_preview = None;
        self.status = format!("Loading preview from {}…", asset.creator);
        thread::spawn(move || {
            let result = (|| {
                let path = temporary_file("preview");
                let downloaded = update::get(&url, &path, &|_| ()).map_err(|err| err.to_string());
                let result = downloaded.and_then(|_| {
                    let size = std::fs::metadata(&path).map_err(|err| err.to_string())?.len();
                    if size == 0 || size > MAX_PREVIEW_BYTES {
                        return Err("Preview is empty or larger than 32 MiB.".into());
                    }
                    let bytes = std::fs::read(&path).map_err(|err| err.to_string())?;
                    let sample = honeydew_audio::sample::decode(&bytes)
                        .map_err(|err| format!("Honeydew could not decode this preview: {err:?}"))?;
                    Ok(PreviewReady { sample, bytes, extension })
                });
                let _ = std::fs::remove_file(&path);
                result
            })();
            let _ = tx.send(result);
        });
    }

    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    fn poll_workers(&mut self, app: &mut AppCtx) {
        let search_result = self.search.as_ref().map(|search| search.try_recv());
        if let Some(result) = search_result {
            match result {
                Ok(Ok((count, results))) => {
                    self.search = None;
                    self.total = count;
                    self.results.extend(results);
                    if self.selected.is_none() {
                        self.selected = (!self.results.is_empty()).then_some(0);
                    }
                    self.status = if self.results.is_empty() {
                        "No reusable previews matched. Try a broader search.".into()
                    } else {
                        format!("{} reusable results on this page · licences are shown per sound", self.results.len())
                    };
                }
                Ok(Err(err)) => {
                    self.search = None;
                    self.status = format!("Freesound search failed: {err}");
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.search = None;
                    self.status = "Freesound search stopped unexpectedly.".into();
                }
            }
        }

        let ready = self.preview.as_ref().and_then(|(id, rx)| match rx.try_recv() {
            Ok(result) => Some((id.clone(), result)),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some((id.clone(), Err("Preview worker stopped unexpectedly.".into()))),
        });
        if let Some((id, result)) = ready {
            self.preview = None;
            match result {
                Ok(ready) => {
                    let sample = Arc::new(ready.sample);
                    app.audio.send(AudioCommand::Preview(Some((sample, 0))));
                    self.playing = true;
                    self.cached_preview = Some((id, ready.bytes, ready.extension));
                    self.status = "Preview playing. Import saves it with a source/licence note.".into();
                }
                Err(err) => {
                    self.cached_preview = None;
                    self.status = err;
                }
            }
        }

    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
fn temporary_file(kind: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    std::env::temp_dir().join(format!(
        "honeydew-sounds-{}-{}-{kind}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

#[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
fn preview_extension(url: &str) -> &'static str {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    if path.ends_with(".ogg") { "ogg" } else { "mp3" }
}

/// Shows Honeydew Sounds over the editor. Search/download/decode work is asynchronous on native and Android builds.
pub fn show(state: &mut State, ui: &mut egui::Ui, app: &mut AppCtx) -> Option<Nav> {
    let ctx = ui.ctx().clone();

    if state.local_browser {
        let mut back = false;
        let at = ctx.content_rect().left_top() + egui::vec2(14.0, 14.0);
        egui::Area::new(egui::Id::new("honeydew_sounds_local_header"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(&ctx, |ui| {
                ui.horizontal(|ui| {
                    back = ui.button("← Honeydew Sounds").clicked();
                    ui.label("Your installed sounds · My Samples · My Recordings");
                });
            });
        if back {
            state.close_local(app);
        } else {
            let action = if let Some(picker) = &mut state.local_picker {
                let library = app.files.sample_library();
                picker.show(&ctx, &app.cx, &*library)
            } else {
                state.local_picker = Some(sample_picker::picker(app.files));
                None
            };
            match action {
                Some(PickerAction::Pick(path)) => {
                    state.local_loading = Some((path.clone(), sample_picker::decode_file(app.files, &path)));
                }
                Some(PickerAction::Import) => {
                    if let Some(user) = app.files.user_dir().map(Path::to_owned) {
                        app.files.import_audio(user.join(crate::project_files::SAMPLES_FOLDER));
                    }
                }
                Some(PickerAction::Close) => state.close_local(app),
                _ => {}
            }
            state.poll_local(app);
            #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
            state.poll_workers(app);
            return None;
        }
    }

    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    {
        if !state.key_loaded {
            state.api_key = crate::settings::load_sounds_api_key().unwrap_or_default();
            state.key_loaded = true;
        }
        state.poll_workers(app);
    }
    state.poll_local(app);
    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
    if state.search.is_some() || state.preview.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
    }

    let mut open = true;
    egui::Window::new("Honeydew Sounds")
        .id(egui::Id::new("honeydew_sounds_window"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size(egui::vec2(760.0, 620.0))
        .min_width(360.0)
        .show(&ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Find sounds. Preview them. Bring them into your project.").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Browse installed sounds").clicked() {
                        state.stop_sample(app);
                        state.local_picker = Some(sample_picker::picker(app.files));
                        state.local_browser = true;
                    }
                });
            });
            ui.separator();

            #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
            {
                ui.collapsing("Freesound connection", |ui| {
                    ui.label("Your API key is kept in a private file on this device. It is not written to songs or ordinary Honeydew settings.");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut state.api_key).password(true).hint_text("Freesound API key"));
                        if ui.button("Save key").clicked() {
                            match crate::settings::save_sounds_api_key(&state.api_key) {
                                Ok(()) => state.status = "API key saved privately on this device.".into(),
                                Err(err) => state.status = format!("Could not save API key: {err}"),
                            }
                        }
                        if ui.button("Forget").clicked() {
                            state.api_key.clear();
                            match crate::settings::save_sounds_api_key("") {
                                Ok(()) => state.status = "Saved API key removed from this device.".into(),
                                Err(err) => state.status = format!("Could not remove API key: {err}"),
                            }
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.query).desired_width(f32::INFINITY).hint_text("Search kicks, piano loops, textures…"));
                    let searching = state.search.is_some();
                    if ui.add_enabled(!searching && !state.query.trim().is_empty() && !state.api_key.trim().is_empty(), egui::Button::new(if searching { "Searching…" } else { "Search" })).clicked() {
                        state.page = 1;
                        state.begin_search();
                    }
                });
            }
            #[cfg(any(target_arch = "wasm32", target_os = "ios"))]
            {
                ui.label("Online catalogue search is not enabled in this build. Your installed and imported sounds remain available below.");
            }

            ui.horizontal_wrapped(|ui| {
                for (label, category) in [
                    ("All", None),
                    ("Drums", Some(Category::Drums)),
                    ("Loops", Some(Category::Loops)),
                    ("Vocals", Some(Category::Vocals)),
                    ("Instruments", Some(Category::Instruments)),
                    ("FX", Some(Category::Fx)),
                    ("Textures", Some(Category::Textures)),
                ] {
                    if ui.selectable_label(state.category == category, label).clicked() {
                        state.category = category;
                    }
                }
            });
            ui.separator();

            let mut selected = None;
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                let visible: Vec<(usize, SoundAsset)> = state.results.iter().cloned().enumerate()
                    .filter(|(_, asset)| state.category.is_none_or(|category| asset.category == category))
                    .collect();
                if visible.is_empty() {
                    ui.label("Search Freesound to see CC0, CC BY and CC BY-NC previews. Unknown and no-derivatives licences are filtered out.");
                }
                for (index, asset) in visible {
                    let is_selected = state.selected == Some(index);
                    let response = ui.group(|ui| {
                        if ui.selectable_label(is_selected, &asset.title).clicked() {
                            selected = Some(index);
                        }
                        ui.horizontal_wrapped(|ui| {
                            ui.weak(format!("{} · {}", asset.creator, category_name(asset.category)));
                            ui.weak(asset.accepted_license().map_or("Licence unavailable", |l| l.label()));
                            if let Some(seconds) = asset.duration_seconds {
                                ui.weak(format!("{seconds:.1}s"));
                            }
                        });
                    });
                    response.response.on_hover_text(&asset.tags.join(", "));
                }
            });
            if let Some(index) = selected {
                state.selected = Some(index);
            }

            if let Some(asset) = state.selected.and_then(|index| state.results.get(index)).cloned() {
                ui.separator();
                ui.label(egui::RichText::new(&asset.title).heading());
                ui.label(format!("{} · {} · {}", asset.creator, category_name(asset.category), asset.accepted_license().map_or("Licence unavailable", |l| l.label())));
                if asset.accepted_license().is_some_and(|license| license.is_non_commercial()) {
                    ui.colored_label(ui.visuals().warn_fg_color, "Non-commercial licence: personal-use projects only.");
                }
                ui.horizontal_wrapped(|ui| {
                    #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
                    {
                        if ui.button("Preview").clicked() {
                            state.stop_sample(app);
                            state.begin_preview(&asset);
                        }
                        let preview_ready = state.cached_preview.as_ref().is_some_and(|(id, _, _)| id == &asset.id);
                        if ui.add_enabled(preview_ready, egui::Button::new("Import preview to My Samples")).clicked() {
                            if let Some((id, bytes, extension)) = state.cached_preview.take() {
                                if id == asset.id {
                                    let license = asset.accepted_license().map_or("Unknown", |l| l.label());
                                    let attribution = format!(
                                        "Title: {}\nCreator: {}\nSource: {}\nLicence: {}\nLicence URL: {}\nHoneydew Sounds / Freesound preview copy.\n",
                                        asset.title,
                                        asset.creator,
                                        asset.page_url,
                                        license,
                                        asset.license_url.as_deref().unwrap_or("Not supplied"),
                                    );
                                    match app.files.keep_downloaded_sound(&asset.title, &extension, bytes, &attribution) {
                                        Ok(path) => state.status = format!("Imported to My Samples: {}", path.file_name().unwrap_or_default().to_string_lossy()),
                                        Err(err) => state.status = format!("Import failed: {err}"),
                                    }
                                }
                            }
                        }
                        if state.playing && ui.button("Stop preview").clicked() {
                            state.stop_sample(app);
                        }
                    }
                    let _ = ui.hyperlink_to("Open source page ↗", &asset.page_url);
                });
            }

            ui.separator();
            ui.label(&state.status);
            ui.horizontal(|ui| {
                ui.label(format!("Page {} · {} provider results", state.page, state.total));
                #[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
                {
                    let searching = state.search.is_some();
                    if ui.add_enabled(!searching && (state.page as u64) * u64::from(SEARCH_PAGE_SIZE) < state.total, egui::Button::new("Load more")).clicked() {
                        state.page += 1;
                        state.begin_search();
                    }
                }
            });
        });

    if !open {
        state.stop_sample(app);
        state.close_local(app);
        return Some(Nav::Close);
    }
    None
}

fn category_name(category: Category) -> &'static str {
    match category {
        Category::Drums => "Drums",
        Category::Loops => "Loops",
        Category::Vocals => "Vocals",
        Category::Instruments => "Instruments",
        Category::Fx => "FX",
        Category::Textures => "Textures",
    }
}
