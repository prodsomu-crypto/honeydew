//! Potion's visual wavetable-synth workspace (type 53).
//!
//! The editor is drawn as one touch-first surface rather than a stack of FL-style
//! parameter widgets. Each graph is tied to the same normalized module parameters
//! used by the DSP engine, so the visual controls, automation and playback agree.

use eframe::egui::{Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};

use crate::panels::build::*;
use crate::song::ModuleInstance;
use crate::ui::UiCtx;

const LOAD_WAV_TABLE: u32 = 41600;
const SELECT_OSC: u32 = 90901;
const BG: Color32 = Color32::from_rgb(18, 21, 26);
const CARD: Color32 = Color32::from_rgb(29, 34, 41);
const CARD_HI: Color32 = Color32::from_rgb(39, 46, 56);
const TRACK: Color32 = Color32::from_rgb(10, 13, 17);
const GRID: Color32 = Color32::from_rgb(54, 63, 75);
const MUTED: Color32 = Color32::from_rgb(137, 149, 164);
const LABEL: Color32 = Color32::from_rgb(222, 229, 238);

fn value(module: &ModuleInstance, index: usize) -> f32 {
    module
        .params
        .get(index)
        .copied()
        .filter(|v| v.is_finite())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0)
}

fn put(module: &mut ModuleInstance, index: usize, next: f32, changed: &mut Vec<usize>) {
    if let Some(current) = module.params.get_mut(index) {
        let next = next.clamp(0.0, 1.0);
        if (*current - next).abs() > 0.0005 {
            *current = next;
            if !changed.contains(&index) {
                changed.push(index);
            }
        }
    }
}

fn text(ui: &Ui, rect: Rect, label: &str, size: f32, color: Color32, align: Align2) {
    ui.painter().text(
        rect.center(),
        align,
        label,
        FontId::proportional(size),
        color,
    );
}

fn card(ui: &Ui, rect: Rect, title: &str, ink: Color32) {
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, CARD);
    painter.rect_filled(
        Rect::from_min_max(rect.left_top(), pos2(rect.right(), rect.top() + 2.0)),
        2.0,
        ink.gamma_multiply(0.72),
    );
    painter.text(
        pos2(rect.left() + 10.0, rect.top() + 10.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(10.0),
        MUTED,
    );
}

fn graph_grid(ui: &Ui, rect: Rect) {
    let painter = ui.painter();
    for i in 1..4 {
        let x = rect.left() + rect.width() * i as f32 / 4.0;
        let y = rect.top() + rect.height() * i as f32 / 4.0;
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(0.7, GRID),
        );
        painter.line_segment(
            [pos2(rect.left(), y), pos2(rect.right(), y)],
            Stroke::new(0.7, GRID),
        );
    }
    painter.line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(0.8, GRID),
    );
}

fn wave_sample(x: f32, morph: f32) -> f32 {
    let phase = std::f32::consts::TAU * x;
    let sine = phase.sin();
    let triangle = 1.0 - 4.0 * (x - 0.5).abs();
    let saw = 2.0 * x - 1.0;
    let pulse = if x < 0.5 { -1.0 } else { 1.0 };
    let (a, b, t) = if morph < 0.333 {
        (sine, triangle, morph * 3.0)
    } else if morph < 0.666 {
        (triangle, saw, (morph - 0.333) * 3.0)
    } else {
        (saw, pulse, (morph - 0.666) * 3.0)
    };
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn wave_points(rect: Rect, morph: f32) -> Vec<eframe::egui::Pos2> {
    (0..=192)
        .map(|i| {
            let x = i as f32 / 192.0;
            pos2(
                rect.left() + x * rect.width(),
                rect.center().y - wave_sample(x, morph) * rect.height() * 0.39,
            )
        })
        .collect()
}

fn draw_wave(ui: &Ui, rect: Rect, morph: f32, ink: Color32) {
    let painter = ui.painter();
    painter.rect_filled(rect, 5.0, TRACK);
    graph_grid(ui, rect);
    painter.add(Shape::line(
        wave_points(rect.shrink(4.0), morph),
        Stroke::new(1.7, ink),
    ));
}

fn slider(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    param: usize,
    label: &str,
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let v = value(module, param);
    let painter = ui.painter();
    painter.rect_filled(rect, 5.0, CARD_HI);
    painter.text(
        pos2(rect.left() + 7.0, rect.top() + 5.0),
        Align2::LEFT_TOP,
        label,
        FontId::proportional(9.5),
        MUTED,
    );
    painter.text(
        pos2(rect.right() - 7.0, rect.top() + 5.0),
        Align2::RIGHT_TOP,
        format!("{v:.2}"),
        FontId::monospace(9.0),
        LABEL,
    );
    let bar = Rect::from_min_max(
        pos2(rect.left() + 8.0, rect.bottom() - 11.0),
        pos2(rect.right() - 8.0, rect.bottom() - 7.0),
    );
    painter.rect_filled(bar, 2.0, TRACK);
    let filled = Rect::from_min_max(
        bar.left_top(),
        pos2(bar.left() + bar.width() * v, bar.bottom()),
    );
    painter.rect_filled(filled, 2.0, ink.gamma_multiply(0.88));
    painter.circle_filled(pos2(bar.left() + bar.width() * v, bar.center().y), 4.0, ink);

    let response = ui.interact(
        rect,
        id.with(("potion-slider", param)),
        Sense::click_and_drag(),
    );
    if (response.dragged() || response.clicked())
        && let Some(at) = response.interact_pointer_pos()
    {
        put(
            module,
            param,
            ((at.x - bar.left()) / bar.width().max(1.0)).clamp(0.0, 1.0),
            changed,
        );
    }
}

fn toggle(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    param: usize,
    label: &str,
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let on = value(module, param) >= 0.5;
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        5.0,
        if on {
            ink.gamma_multiply(0.24)
        } else {
            CARD_HI
        },
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(9.5),
        if on { ink } else { MUTED },
    );
    if ui
        .interact(rect, id.with(("potion-toggle", param)), Sense::click())
        .clicked()
    {
        put(module, param, if on { 0.0 } else { 1.0 }, changed);
    }
}

fn choice(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    param: usize,
    labels: &[&str],
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let current = (value(module, param) * labels.len() as f32)
        .floor()
        .min(labels.len().saturating_sub(1) as f32) as usize;
    let gap = 3.0;
    let width =
        (rect.width() - gap * (labels.len().saturating_sub(1) as f32)) / labels.len().max(1) as f32;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(rect.left() + i as f32 * (width + gap), rect.top()),
            vec2(width, rect.height()),
        );
        let active = i == current;
        ui.painter().rect_filled(
            r,
            4.0,
            if active {
                ink.gamma_multiply(0.26)
            } else {
                CARD_HI
            },
        );
        text(
            ui,
            r,
            label,
            8.5,
            if active { ink } else { MUTED },
            Align2::CENTER_CENTER,
        );
        if ui
            .interact(r, id.with(("potion-choice", param, i)), Sense::click())
            .clicked()
        {
            put(
                module,
                param,
                i as f32 / labels.len().saturating_sub(1).max(1) as f32,
                changed,
            );
        }
    }
}

fn osc_page(
    ui: &mut Ui,
    id: Id,
    area: Rect,
    module: &mut ModuleInstance,
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let gap = 8.0;
    let width = (area.width() - gap * 2.0) / 3.0;
    for osc in 0..3 {
        let x = area.left() + osc as f32 * (width + gap);
        let panel = Rect::from_min_size(pos2(x, area.top()), vec2(width, area.height()));
        card(ui, panel, &format!("OSCILLATOR {}", osc + 1), ink);
        let active = value(module, 13 + osc) >= 0.5;
        toggle(
            ui,
            id,
            Rect::from_min_size(
                pos2(panel.right() - 47.0, panel.top() + 6.0),
                vec2(39.0, 20.0),
            ),
            module,
            13 + osc,
            if active { "ACTIVE" } else { "BYPASS" },
            ink,
            changed,
        );
        let graph = Rect::from_min_max(
            pos2(panel.left() + 9.0, panel.top() + 36.0),
            pos2(panel.right() - 9.0, panel.top() + panel.height() * 0.48),
        );
        draw_wave(ui, graph, value(module, 10 + osc), ink);
        let slider_h = 43.0;
        let top = graph.bottom() + 7.0;
        for (row, (param, label)) in [
            (1 + osc, "LEVEL"),
            (4 + osc, "PAN"),
            (7 + osc, "PITCH"),
            (10 + osc, "TABLE MORPH"),
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::from_min_size(
                pos2(panel.left() + 8.0, top + row as f32 * (slider_h + 4.0)),
                vec2(panel.width() - 16.0, slider_h),
            );
            slider(ui, id, r, module, param, label, ink, changed);
        }
    }
}

fn table_page(
    ui: &mut Ui,
    id: Id,
    area: Rect,
    module: &mut ModuleInstance,
    state: &mut PanelState,
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let selected = state
        .settings
        .get(&SELECT_OSC)
        .copied()
        .unwrap_or(0.0)
        .round()
        .clamp(0.0, 2.0) as usize;
    let selector = Rect::from_min_size(area.left_top(), vec2(area.width(), 29.0));
    for osc in 0..3 {
        let r = Rect::from_min_size(
            pos2(
                selector.left() + osc as f32 * (selector.width() / 3.0),
                selector.top(),
            ),
            vec2(selector.width() / 3.0 - 3.0, 27.0),
        );
        let on = osc == selected;
        ui.painter()
            .rect_filled(r, 5.0, if on { ink.gamma_multiply(0.25) } else { CARD });
        text(
            ui,
            r,
            &format!("OSCILLATOR {}", osc + 1),
            9.5,
            if on { ink } else { MUTED },
            Align2::CENTER_CENTER,
        );
        if ui
            .interact(r, id.with(("table-osc", osc)), Sense::click())
            .clicked()
        {
            state.settings.insert(SELECT_OSC, osc as f32);
        }
    }

    let content_top = selector.bottom() + 8.0;
    let left_w = area.width() * 0.69;
    let graph = Rect::from_min_size(
        pos2(area.left(), content_top),
        vec2(left_w, area.bottom() - content_top - 65.0),
    );
    card(ui, graph, "SINGLE-CYCLE WAVETABLE VIEW", ink);
    let wave_rect = graph.shrink2(vec2(13.0, 27.0));
    draw_wave(ui, wave_rect, value(module, 10 + selected), ink);

    let side = Rect::from_min_max(pos2(graph.right() + 8.0, content_top), area.right_bottom());
    card(ui, side, "TABLE CONTROLS", ink);
    let morph = Rect::from_min_size(
        pos2(side.left() + 8.0, side.top() + 31.0),
        vec2(side.width() - 16.0, 50.0),
    );
    slider(
        ui,
        id,
        morph,
        module,
        10 + selected,
        "FRAME / WAVE MORPH",
        ink,
        changed,
    );
    let tiles = ["SINE", "TRIANGLE", "SAW", "PULSE"];
    for (i, label) in tiles.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let r = Rect::from_min_size(
            pos2(
                side.left() + 8.0 + col as f32 * ((side.width() - 19.0) / 2.0 + 3.0),
                morph.bottom() + 8.0 + row as f32 * 34.0,
            ),
            vec2((side.width() - 19.0) / 2.0, 30.0),
        );
        ui.painter().rect_filled(r, 4.0, CARD_HI);
        text(ui, r, label, 8.2, LABEL, Align2::CENTER_CENTER);
        if ui
            .interact(r, id.with(("wave-preset", selected, i)), Sense::click())
            .clicked()
        {
            let v = [0.0, 0.333, 0.666, 0.999][i];
            put(module, 10 + selected, v, changed);
        }
    }
    let import = Rect::from_min_size(
        pos2(side.left() + 8.0, side.bottom() - 39.0),
        vec2(side.width() - 16.0, 30.0),
    );
    ui.painter().rect_filled(import, 5.0, CARD_HI);
    text(
        ui,
        import,
        "IMPORT WAVETABLE",
        9.0,
        ink,
        Align2::CENTER_CENTER,
    );
    if ui
        .interact(
            import,
            id.with(("import-wavetable", selected)),
            Sense::click(),
        )
        .clicked()
    {
        state.settings.insert(LOAD_WAV_TABLE, 1.0);
    }
    ui.painter().text(
        pos2(area.left() + 3.0, area.bottom() - 44.0),
        Align2::LEFT_TOP,
        "Wave preview follows this oscillator's table position.",
        FontId::proportional(9.0),
        MUTED,
    );
}

fn filter_page(
    ui: &mut Ui,
    id: Id,
    area: Rect,
    module: &mut ModuleInstance,
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let graph = Rect::from_min_size(
        area.left_top(),
        vec2(area.width() * 0.62, area.height() - 4.0),
    );
    card(ui, graph, "FILTER RESPONSE", ink);
    let plot = graph.shrink2(vec2(12.0, 27.0));
    ui.painter().rect_filled(plot, 4.0, TRACK);
    graph_grid(ui, plot);
    let cutoff = value(module, 19);
    let resonance = value(module, 22);
    let filter_mode = (value(module, 23) * 2.0).round() as usize;
    let points = (0..=120)
        .map(|i| {
            let x = i as f32 / 120.0;
            let ratio = (x / (cutoff * 0.95 + 0.025)).max(0.001);
            let low = 1.0 / (1.0 + ratio.powf(3.7)).sqrt();
            let high = 1.0 - low;
            let band = (2.0 * ratio / (1.0 + ratio * ratio)).clamp(0.0, 1.0);
            let base = match filter_mode {
                1 => band,
                2 => high,
                _ => low,
            };
            let peak = (-((x - cutoff).powi(2)) / 0.0035).exp() * resonance * 0.32;
            let y = (base + peak).clamp(0.0, 1.0);
            pos2(
                plot.left() + x * plot.width(),
                plot.bottom() - y * plot.height(),
            )
        })
        .collect::<Vec<_>>();
    ui.painter().add(Shape::line(points, Stroke::new(2.0, ink)));
    let marker = pos2(
        plot.left() + cutoff * plot.width(),
        plot.center().y - resonance * plot.height() * 0.22,
    );
    ui.painter().line_segment(
        [pos2(marker.x, plot.top()), pos2(marker.x, plot.bottom())],
        Stroke::new(1.0, ink.gamma_multiply(0.55)),
    );
    ui.painter().circle_filled(marker, 5.0, ink);
    ui.painter().circle_stroke(marker, 8.0, Stroke::new(1.0, BG));
    let handle = Rect::from_center_size(marker, vec2(34.0, 34.0));
    let response = ui.interact(handle, id.with("filter-cutoff-q"), Sense::click_and_drag());
    if (response.dragged() || response.clicked())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let next_cutoff =
            ((pointer.x - plot.left()) / plot.width().max(1.0)).clamp(0.0, 1.0);
        let next_resonance =
            ((plot.center().y - pointer.y) / (plot.height() * 0.22).max(1.0))
                .clamp(0.0, 1.0);
        put(module, 19, next_cutoff, changed);
        put(module, 22, next_resonance, changed);
    }

    let side = Rect::from_min_max(pos2(graph.right() + 8.0, area.top()), area.right_bottom());
    card(ui, side, "FILTER DESIGN", ink);
    let sh = 48.0;
    let controls = [(19, "CUTOFF"), (22, "RESONANCE"), (44, "ENVELOPE AMOUNT")];
    for (i, (param, label)) in controls.into_iter().enumerate() {
        slider(
            ui,
            id,
            Rect::from_min_size(
                pos2(side.left() + 8.0, side.top() + 30.0 + i as f32 * (sh + 5.0)),
                vec2(side.width() - 16.0, sh),
            ),
            module,
            param,
            label,
            ink,
            changed,
        );
    }
    choice(
        ui,
        id,
        Rect::from_min_size(
            pos2(side.left() + 8.0, side.bottom() - 42.0),
            vec2(side.width() - 16.0, 32.0),
        ),
        module,
        23,
        &["LOW PASS", "BAND PASS", "HIGH PASS"],
        ink,
        changed,
    );
}

fn envelope_page(
    ui: &mut Ui,
    id: Id,
    area: Rect,
    module: &mut ModuleInstance,
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let main = Rect::from_min_size(area.left_top(), vec2(area.width(), area.height() * 0.54));
    card(ui, main, "AMPLITUDE ENVELOPE", ink);
    let plot = main.shrink2(vec2(12.0, 27.0));
    ui.painter().rect_filled(plot, 4.0, TRACK);
    graph_grid(ui, plot);
    let a = value(module, 16);
    let d = value(module, 21);
    let s = value(module, 20);
    let r = value(module, 17);
    let p0 = plot.left_bottom();
    let p1 = pos2(
        plot.left() + (0.05 + a * 0.28) * plot.width(),
        plot.top() + 5.0,
    );
    let p2 = pos2(
        p1.x + (0.05 + d * 0.27) * plot.width(),
        plot.bottom() - s * plot.height(),
    );
    let p3 = pos2(p2.x + r * (plot.right() - p2.x) * 0.72, p2.y);
    let p4 = plot.right_bottom();
    let points = [p0, p1, p2, p3, p4];
    ui.painter()
        .add(Shape::line(points.to_vec(), Stroke::new(2.2, ink)));
    for (handle, point) in [(0usize, p1), (1usize, p2), (2usize, p3)] {
        ui.painter().circle_filled(point, 5.0, ink);
        ui.painter()
            .circle_stroke(point, 8.0, Stroke::new(1.0, BG));
        let hit = Rect::from_center_size(point, vec2(28.0, 28.0));
        let response = ui.interact(
            hit,
            id.with(("amp-envelope-handle", handle)),
            Sense::click_and_drag(),
        );
        if (response.dragged() || response.clicked())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let x = pointer.x.clamp(plot.left(), plot.right());
            let y = pointer.y.clamp(plot.top(), plot.bottom());
            match handle {
                0 => {
                    let attack = (((x - plot.left()) / plot.width().max(1.0) - 0.05) / 0.28)
                        .clamp(0.0, 1.0);
                    put(module, 16, attack, changed);
                }
                1 => {
                    let decay = (((x - p1.x) / plot.width().max(1.0) - 0.05) / 0.27)
                        .clamp(0.0, 1.0);
                    let sustain = ((plot.bottom() - y) / plot.height().max(1.0))
                        .clamp(0.0, 1.0);
                    put(module, 21, decay, changed);
                    put(module, 20, sustain, changed);
                }
                _ => {
                    let release = ((x - p2.x) / ((plot.right() - p2.x) * 0.72).max(1.0))
                        .clamp(0.0, 1.0);
                    put(module, 17, release, changed);
                }
            }
        }
    }
    let controls_top = main.bottom() + 7.0;
    let gap = 7.0;
    let w = (area.width() - gap * 3.0) / 4.0;
    for (i, (param, label)) in [
        (16, "ATTACK"),
        (21, "DECAY"),
        (20, "SUSTAIN"),
        (17, "RELEASE"),
    ]
    .into_iter()
    .enumerate()
    {
        slider(
            ui,
            id,
            Rect::from_min_size(
                pos2(area.left() + i as f32 * (w + gap), controls_top),
                vec2(w, 50.0),
            ),
            module,
            param,
            label,
            ink,
            changed,
        );
    }

    let filter_top = controls_top + 58.0;
    let filter_h = (area.bottom() - filter_top).max(48.0);
    let filter = Rect::from_min_size(
        area.left_bottom() - vec2(0.0, filter_h),
        vec2(area.width(), filter_h),
    );
    card(ui, filter, "FILTER ENVELOPE · INDEPENDENT SHAPE", ink);
    let sub_w = (area.width() - gap * 3.0) / 4.0;
    for (i, (param, label)) in [
        (59, "F-ATTACK"),
        (60, "F-DECAY"),
        (61, "F-SUSTAIN"),
        (62, "F-RELEASE"),
    ]
    .into_iter()
    .enumerate()
    {
        slider(
            ui,
            id,
            Rect::from_min_size(
                pos2(
                    filter.left() + i as f32 * (sub_w + gap),
                    filter.top() + 30.0,
                ),
                vec2(sub_w, (filter.height() - 35.0).max(42.0)),
            ),
            module,
            param,
            label,
            ink,
            changed,
        );
    }
}

fn lfo_points(rect: Rect, shape: usize) -> Vec<eframe::egui::Pos2> {
    (0..=96)
        .map(|i| {
            let x = i as f32 / 96.0;
            let phase = x.fract();
            let y = match shape {
                1 => 1.0 - 4.0 * (phase - 0.5).abs(),
                2 => {
                    if phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                _ => (std::f32::consts::TAU * phase).sin(),
            };
            pos2(
                rect.left() + x * rect.width(),
                rect.center().y - y * rect.height() * 0.35,
            )
        })
        .collect()
}

fn modulation_page(
    ui: &mut Ui,
    id: Id,
    area: Rect,
    module: &mut ModuleInstance,
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let gap = 8.0;
    let width = (area.width() - gap) / 2.0;
    for lfo in 0..2 {
        let panel = Rect::from_min_size(
            pos2(area.left() + lfo as f32 * (width + gap), area.top()),
            vec2(width, area.height() * 0.58),
        );
        card(ui, panel, &format!("LFO {}", lfo + 1), ink);
        let graph = Rect::from_min_size(
            pos2(panel.left() + 9.0, panel.top() + 28.0),
            vec2(panel.width() - 18.0, panel.height() * 0.28),
        );
        ui.painter().rect_filled(graph, 4.0, TRACK);
        graph_grid(ui, graph);
        let shape = (value(module, 32 + lfo) * 2.99).floor() as usize;
        ui.painter().add(Shape::line(
            lfo_points(graph, shape.min(2)),
            Stroke::new(1.8, ink),
        ));
        let control_top = graph.bottom() + 5.0;
        let control_width = panel.width() - 16.0;
        slider(
            ui,
            id,
            Rect::from_min_size(
                pos2(panel.left() + 8.0, control_top),
                vec2(control_width, 31.0),
            ),
            module,
            29 + lfo * 4,
            "RATE",
            ink,
            changed,
        );
        slider(
            ui,
            id,
            Rect::from_min_size(
                pos2(panel.left() + 8.0, control_top + 33.0),
                vec2(control_width, 31.0),
            ),
            module,
            30 + lfo * 4,
            "DEPTH",
            ink,
            changed,
        );
        choice(
            ui,
            id,
            Rect::from_min_size(
                pos2(panel.left() + 8.0, control_top + 66.0),
                vec2(control_width, 22.0),
            ),
            module,
            31 + lfo * 4,
            &["PITCH", "FILTER", "PAN"],
            ink,
            changed,
        );
        choice(
            ui,
            id,
            Rect::from_min_size(
                pos2(panel.left() + 8.0, control_top + 90.0),
                vec2(control_width, 22.0),
            ),
            module,
            32 + lfo * 4,
            &["SINE", "TRIANGLE", "SQUARE"],
            ink,
            changed,
        );
    }

    let top = area.top() + area.height() * 0.61;
    let h = (area.height() * 0.18).clamp(48.0, 68.0);
    let w = (area.width() - gap * 2.0) / 3.0;
    for (i, (param, label)) in [(37, "FM AMOUNT"), (38, "RING MOD"), (39, "WARP AMOUNT")]
        .into_iter()
        .enumerate()
    {
        slider(
            ui,
            id,
            Rect::from_min_size(
                pos2(area.left() + i as f32 * (w + gap), top),
                vec2(w, h.max(44.0)),
            ),
            module,
            param,
            label,
            ink,
            changed,
        );
    }
    choice(
        ui,
        id,
        Rect::from_min_size(
            pos2(area.left(), area.bottom() - 34.0),
            vec2(area.width(), 30.0),
        ),
        module,
        40,
        &["PHASE", "FOLD", "SINE DRIVE", "SYNC"],
        ink,
        changed,
    );
}

fn voice_page(
    ui: &mut Ui,
    id: Id,
    area: Rect,
    module: &mut ModuleInstance,
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let top = Rect::from_min_size(area.left_top(), vec2(area.width(), area.height() * 0.38));
    card(ui, top, "UNISON VOICE FIELD", ink);
    let count = 1 + (value(module, 24) * 6.0).round() as usize;
    let detune = value(module, 25);
    let spread = value(module, 26);
    let center = pos2(top.center().x, top.center().y + 12.0);
    for i in 0..count {
        let p = if count > 1 {
            i as f32 / (count - 1) as f32
        } else {
            0.5
        };
        let x = center.x + (p - 0.5) * top.width() * 0.72 * spread;
        let y = center.y + (p - 0.5) * 34.0 * detune;
        ui.painter()
            .circle_filled(pos2(x, y), 5.0, ink.gamma_multiply(0.9 - p * 0.25));
    }
    let gap = 7.0;
    let width = (area.width() - gap * 3.0) / 4.0;
    let controls = [
        (24, "UNISON"),
        (25, "DETUNE"),
        (26, "STEREO SPREAD"),
        (27, "GLIDE"),
    ];
    for (i, (param, label)) in controls.into_iter().enumerate() {
        slider(
            ui,
            id,
            Rect::from_min_size(
                pos2(area.left() + i as f32 * (width + gap), top.bottom() + 8.0),
                vec2(width, 50.0),
            ),
            module,
            param,
            label,
            ink,
            changed,
        );
    }
    let row = top.bottom() + 66.0;
    choice(
        ui,
        id,
        Rect::from_min_size(pos2(area.left(), row), vec2(area.width() * 0.58, 35.0)),
        module,
        28,
        &["POLY", "MONO", "LEGATO"],
        ink,
        changed,
    );
    slider(
        ui,
        id,
        Rect::from_min_size(
            pos2(area.left() + area.width() * 0.61, row - 7.0),
            vec2(area.width() * 0.39, 50.0),
        ),
        module,
        45,
        "POLYPHONY",
        ink,
        changed,
    );
}

fn mix_page(
    ui: &mut Ui,
    id: Id,
    area: Rect,
    module: &mut ModuleInstance,
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let meter = Rect::from_min_size(area.left_top(), vec2(area.width(), area.height() * 0.34));
    card(ui, meter, "OSCILLATOR MIX BUS", ink);
    for osc in 0..3 {
        let x = meter.left() + meter.width() * (0.18 + 0.32 * osc as f32);
        let level = value(module, 1 + osc);
        let r = Rect::from_min_max(
            pos2(x, meter.bottom() - 16.0),
            pos2(
                x + 28.0,
                meter.bottom() - 16.0 - level * (meter.height() - 42.0),
            ),
        );
        ui.painter().rect_filled(
            Rect::from_min_max(
                pos2(x, meter.top() + 27.0),
                pos2(x + 28.0, meter.bottom() - 16.0),
            ),
            4.0,
            TRACK,
        );
        ui.painter().rect_filled(r, 4.0, ink.gamma_multiply(0.9));
        text(
            ui,
            Rect::from_min_size(pos2(x - 10.0, meter.bottom() - 13.0), vec2(48.0, 12.0)),
            &format!("OSC {}", osc + 1),
            8.0,
            MUTED,
            Align2::CENTER_CENTER,
        );
    }

    let top = meter.bottom() + 8.0;
    let gap = 7.0;
    let width = (area.width() - gap * 3.0) / 4.0;
    for (i, (param, label)) in [(41, "SUB"), (42, "NOISE"), (43, "VELOCITY"), (18, "MASTER")]
        .into_iter()
        .enumerate()
    {
        slider(
            ui,
            id,
            Rect::from_min_size(
                pos2(area.left() + i as f32 * (width + gap), top),
                vec2(width, 54.0),
            ),
            module,
            param,
            label,
            ink,
            changed,
        );
    }
}

fn matrix_page(
    ui: &mut Ui,
    id: Id,
    area: Rect,
    module: &mut ModuleInstance,
    ink: Color32,
    changed: &mut Vec<usize>,
) {
    let gap = 8.0;
    let width = (area.width() - gap) / 2.0;
    let height = (area.height() - gap) / 2.0;
    for route in 0..4 {
        let x = area.left() + (route % 2) as f32 * (width + gap);
        let y = area.top() + (route / 2) as f32 * (height + gap);
        let panel = Rect::from_min_size(pos2(x, y), vec2(width, height));
        card(ui, panel, &format!("MOD ROUTE {}", route + 1), ink);
        let base = 47 + route * 3;
        let control_width = panel.width() - 16.0;
        choice(
            ui,
            id,
            Rect::from_min_size(
                pos2(panel.left() + 8.0, panel.top() + 31.0),
                vec2(control_width, 24.0),
            ),
            module,
            base,
            &["LFO 1", "LFO 2", "ENV", "VEL", "MOD", "TOUCH"],
            ink,
            changed,
        );
        choice(
            ui,
            id,
            Rect::from_min_size(
                pos2(panel.left() + 8.0, panel.top() + 61.0),
                vec2(control_width, 24.0),
            ),
            module,
            base + 1,
            &["PITCH", "FILTER", "PAN", "OSC 1", "OSC 2", "OSC 3"],
            ink,
            changed,
        );
        slider(
            ui,
            id,
            Rect::from_min_size(
                pos2(panel.left() + 8.0, panel.top() + 91.0),
                vec2(control_width, 40.0),
            ),
            module,
            base + 2,
            "MODULATION DEPTH",
            ink,
            changed,
        );
    }
}

pub(crate) fn workspace(
    ui: &mut Ui,
    cx: &UiCtx,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    state: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let mut changed = Vec::new();
    let painter = ui.painter();
    painter.rect_filled(rect, 7.0, BG);
    let title = [
        "OSCILLATORS",
        "WAVETABLE EDITOR",
        "FILTER DESIGNER",
        "ENVELOPES",
        "MODULATION",
        "VOICE / UNISON",
        "MIXER",
        "MODULATION MATRIX",
    ];
    painter.text(
        pos2(rect.left() + 12.0, rect.top() + 8.0),
        Align2::LEFT_TOP,
        "POTION  /  WAVETABLE SYNTH",
        FontId::proportional(11.0),
        ink,
    );
    painter.text(
        pos2(rect.right() - 12.0, rect.top() + 8.0),
        Align2::RIGHT_TOP,
        title[state.tab.min(7)],
        FontId::proportional(9.5),
        LABEL,
    );
    painter.line_segment(
        [
            pos2(rect.left() + 10.0, rect.top() + 26.0),
            pos2(rect.right() - 10.0, rect.top() + 26.0),
        ],
        Stroke::new(1.0, GRID),
    );
    let area = Rect::from_min_max(
        pos2(rect.left() + 9.0, rect.top() + 35.0),
        pos2(rect.right() - 9.0, rect.bottom() - 9.0),
    );
    match state.tab.min(7) {
        0 => osc_page(ui, id, area, module, ink, &mut changed),
        1 => table_page(ui, id, area, module, state, ink, &mut changed),
        2 => filter_page(ui, id, area, module, ink, &mut changed),
        3 => envelope_page(ui, id, area, module, ink, &mut changed),
        4 => modulation_page(ui, id, area, module, ink, &mut changed),
        5 => voice_page(ui, id, area, module, ink, &mut changed),
        6 => mix_page(ui, id, area, module, ink, &mut changed),
        _ => matrix_page(ui, id, area, module, ink, &mut changed),
    }
    let _ = cx;
    changed
}

/// The wavetable import action is a momentary panel button, routed to Honeydew's
/// existing module sample picker (decode stays outside the audio callback).
pub fn asks_for_wavetable(state: &mut PanelState, edited: &[u32]) -> bool {
    let requested = edited.contains(&LOAD_WAV_TABLE)
        || state
            .settings
            .get(&LOAD_WAV_TABLE)
            .is_some_and(|&value| value >= 0.5);
    if requested {
        state.settings.insert(LOAD_WAV_TABLE, 0.0);
    }
    requested
}

pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        tabs(0, "OSC · TABLE · FILTER · ENV · MOD · VOICE · MIX · MATRIX").h(Size::U(1.25)),
        display(9000).draw(workspace).h(Size::U(12.8)),
    ])))
}

#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [16.96, 15.2],
    settings: &[],
    cases: &[],
};
