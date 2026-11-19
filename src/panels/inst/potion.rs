//! Potion's custom-painted wavetable instrument panel (type 53).
//! The graph is a responsive shape preview; all controls are normal host parameters.

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

/// Three layered waveform previews following each oscillator's frame-morph control.
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
                let custom = (phase * 2.0 + (phase * 5.0).sin() * 0.35).sin();
                let (a, b, t) = if morph < 0.25 {
                    (sine, triangle, morph * 4.0)
                } else if morph < 0.5 {
                    (triangle, saw, (morph - 0.25) * 4.0)
                } else if morph < 0.75 {
                    (saw, square, (morph - 0.5) * 4.0)
                } else {
                    (square, custom, (morph - 0.75) * 4.0)
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

/// The import button is an ordinary module parameter so it works with the rack's existing sample-picker action.
pub fn custom_picked(module: &ModuleInstance, changed: &[i32]) -> bool {
    module.type_id == 53 && changed.contains(&46)
}

pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        tabs(0, "OSC 1 · OSC 2 · OSC 3 · TABLE · FILTER · ENVELOPE · MOD · VOICE · MIX · MATRIX")
            .h(Size::U(1.25)),
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
        hstack([toggle(41600, "LOAD WAV TABLE").p(46)]).show(Show::Tab(3)),
        hstack([
            knob(41100, "CUTOFF").p(19),
            knob(41200, "RESONANCE").p(22),
            knob(41300, "FILTER MODE").p(23),
            knob(41400, "FILTER ENV").p(44),
        ])
        .show(Show::Tab(4)),
        hstack([
            knob(51100, "ATTACK").p(16),
            knob(51200, "DECAY").p(21),
            knob(51300, "SUSTAIN").p(20),
            knob(51400, "RELEASE").p(17),
        ])
        .show(Show::Tab(5)),
        hstack([
            knob(71100, "LFO 1 RATE").p(29),
            knob(71200, "LFO 1 AMT").p(30),
            knob(71300, "LFO 1 TARGET").p(31),
            knob(71400, "LFO 1 SHAPE").p(32),
        ])
        .show(Show::Tab(6)),
        hstack([
            knob(72100, "LFO 2 RATE").p(33),
            knob(72200, "LFO 2 AMT").p(34),
            knob(72300, "LFO 2 TARGET").p(35),
            knob(72400, "LFO 2 SHAPE").p(36),
        ])
        .show(Show::Tab(6)),
        hstack([
            knob(73100, "FM").p(37),
            knob(73200, "RING").p(38),
            knob(73300, "WARP").p(39),
            knob(73400, "WARP MODE").p(40),
        ])
        .show(Show::Tab(6)),
        hstack([
            knob(81100, "UNISON").p(24),
            knob(81200, "DETUNE").p(25),
            knob(81300, "SPREAD").p(26),
            knob(81400, "GLIDE").p(27),
        ])
        .show(Show::Tab(7)),
        hstack([
            knob(81500, "VOICE MODE").p(28),
            knob(81600, "POLYPHONY").p(45),
        ])
        .show(Show::Tab(7)),
        hstack([
            knob(91100, "SUB").p(41),
            knob(91200, "NOISE").p(42),
            knob(91300, "VELOCITY").p(43),
            knob(91400, "MASTER").p(18),
        ])
        .show(Show::Tab(8)),
        hstack([
            knob(101100, "SRC 1").p(47),
            knob(101200, "TARGET 1").p(48),
            knob(101300, "DEPTH 1").p(49),
        ])
        .show(Show::Tab(9)),
        hstack([
            knob(102100, "SRC 2").p(50),
            knob(102200, "TARGET 2").p(51),
            knob(102300, "DEPTH 2").p(52),
        ])
        .show(Show::Tab(9)),
        hstack([
            knob(103100, "SRC 3").p(53),
            knob(103200, "TARGET 3").p(54),
            knob(103300, "DEPTH 3").p(55),
        ])
        .show(Show::Tab(9)),
        hstack([
            knob(104100, "SRC 4").p(56),
            knob(104200, "TARGET 4").p(57),
            knob(104300, "DEPTH 4").p(58),
        ])
        .show(Show::Tab(9)),
    ])))
}

#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [16.96, 14.0],
    settings: &[],
    cases: &[],
};
