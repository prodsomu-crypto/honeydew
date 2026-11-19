//! Potion's custom-painted wavetable instrument panel (type 53).
//! The waveform display is drawn from the live oscillator morph controls; all controls remain normal host parameters.

use eframe::egui::{Color32, Id, Rect, Shape, Stroke, Ui, pos2};

use crate::panels::build::*;
use crate::song::ModuleInstance;
use crate::ui::UiCtx;

fn value(module: &ModuleInstance, index: usize) -> f32 {
    module
        .params
        .get(index)
        .copied()
        .unwrap_or(0.0)
        .clamp(0.0, 1.0)
}

/// Three layered preview traces, with each trace following its oscillator's wavetable position.
fn waveform(
    ui: &mut Ui,
    cx: &UiCtx,
    _: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, cx.theme.colors.control_fill.to_color32());
    let traces = [(10, 1.0), (11, 0.72), (12, 0.48)];
    for (param, alpha) in traces {
        let morph = value(module, param);
        let points = (0..=160)
            .map(|i| {
            let x = i as f32 / 160.0;
            let phase = std::f32::consts::TAU * x;
            let sine = phase.sin();
            let triangle = (2.0 / std::f32::consts::PI) * phase.sin().asin();
            let saw = 2.0 * x - 1.0;
            let square = if x < 0.5 { 1.0 } else { -1.0 };
            let (a, b, t) = if morph < 1.0 / 3.0 {
                (sine, triangle, morph * 3.0)
            } else if morph < 2.0 / 3.0 {
                (triangle, saw, (morph - 1.0 / 3.0) * 3.0)
            } else {
                (saw, square, (morph - 2.0 / 3.0) * 3.0)
            };
            let sample = a + (b - a) * t;
            pos2(
                rect.left() + x * rect.width(),
                rect.center().y - sample * rect.height() * 0.34,
            )
            })
            .collect::<Vec<_>>();
        painter.add(Shape::line(
            points,
            Stroke::new(cx.m.t() * 1.5, ink.gamma_multiply(alpha)),
        ));
    }
    Vec::new()
}

pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        tabs(0, "OSC 1 · OSC 2 · OSC 3 · FILTER · ENVELOPE · MASTER").h(Size::U(1.25)),
        display(9000).draw(waveform).h(Size::U(2.6)),
        hstack([
            knob(11100, "LEVEL").p(1),
            knob(11200, "PAN").p(4),
            knob(11300, "PITCH").p(7),
            knob(11400, "WAVE MORPH").p(10),
            toggle(11500, "ON").p(13),
        ])
        .show(Show::Tab(0)),
        hstack([
            knob(21100, "LEVEL").p(2),
            knob(21200, "PAN").p(5),
            knob(21300, "PITCH").p(8),
            knob(21400, "WAVE MORPH").p(11),
            toggle(21500, "ON").p(14),
        ])
        .show(Show::Tab(1)),
        hstack([
            knob(31100, "LEVEL").p(3),
            knob(31200, "PAN").p(6),
            knob(31300, "PITCH").p(9),
            knob(31400, "WAVE MORPH").p(12),
            toggle(31500, "ON").p(15),
        ])
        .show(Show::Tab(2)),
        hstack([knob(41100, "CUTOFF").p(19)]).show(Show::Tab(3)),
        hstack([
            knob(51100, "ATTACK").p(16),
            knob(51200, "SUSTAIN").p(20),
            knob(51300, "RELEASE").p(17),
        ])
        .show(Show::Tab(4)),
        hstack([knob(61100, "MASTER").p(18)]).show(Show::Tab(5)),
    ])))
}

#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [16.96, 8.0],
    settings: &[],
    cases: &[],
};
