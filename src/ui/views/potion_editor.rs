//! Potion's native visual synth editor in a Lemon-style fullscreen overlay.

use eframe::egui::{self, Align2, Area, FontId, Id, LayerId, Order, Rect, Sense, Vec2, pos2, vec2};
use honeydew_audio::device::AudioCommand;

use super::{AppCtx, Nav, Overlay};
use crate::icons::{IconId, draw_icon};
use crate::panels::{PanelState, inst::potion};
use crate::theme::channel_color;
use crate::ui::anim;
use crate::ui::buttons::filled_circle_button;

const CLOSE: f32 = 1.5;
const CLOSE_AT: (f32, f32) = (2.1, 0.2);

const PAGES: [&str; 8] = ["OSC", "TABLE", "FILTER", "ENV", "MOD", "VOICE", "MIX", "MATRIX"];

#[derive(Default)]
pub struct State {
    panel: PanelState,
}

/// Shows Potion's workspace across the screen. Parameter changes use the normal live audio command path.
pub fn show(
    state: &mut State,
    ui: &egui::Ui,
    app: &mut AppCtx,
    channel: usize,
    module_id: i32,
) -> Option<Nav> {
    let ctx = ui.ctx().clone();
    if crate::ui::close_pressed(&ctx) {
        *state = State::default();
        return Some(Nav::Close);
    }

    let screen = app.regions.screen;
    let m = app.cx.m;
    let colors = &app.cx.theme.colors;
    let id = Id::new("potion full screen");
    let bg = colors.background.to_color32();
    let text = colors.text.to_color32();
    crate::ui::backdrop(&ctx, id);
    ctx.layer_painter(LayerId::new(Order::Middle, id.with("backdrop")))
        .rect_filled(screen, 0.0, anim::dim(bg, anim::DIM_ALPHA));

    let mut changed_values = Vec::new();
    let mut imported = false;
    let mut closed = false;

    {
        let Some(ch) = app.song.channels.get_mut(channel) else {
            *state = State::default();
            return Some(Nav::Close);
        };
        let ink = channel_color(ch.hue, app.cx.theme).to_color32();
        let Some(module) = ch.rack.get_mut(module_id) else {
            *state = State::default();
            return Some(Nav::Close);
        };
        if module.type_id != 53 {
            *state = State::default();
            return Some(Nav::Close);
        }

        let disc = Rect::from_min_size(
            pos2(
                screen.right() - m.u(CLOSE_AT.0),
                screen.top() + m.u(CLOSE_AT.1),
            ),
            Vec2::splat(m.u(CLOSE)),
        );
        let area = screen
            .shrink2(vec2(m.u(CLOSE_AT.0), 0.0))
            .shrink2(vec2(0.0, m.u(0.55)));
        let tabs_h = m.u(1.65).max(36.0);
        let tab_rect = Rect::from_min_size(area.left_top(), vec2(area.width(), tabs_h));
        let body = Rect::from_min_max(
            pos2(area.left(), tab_rect.bottom() + m.u(0.22)),
            area.right_bottom(),
        );

        Area::new(id)
            .order(Order::Foreground)
            .fixed_pos(screen.min)
            .constrain(false)
            .fade_in(false)
            .show(&ctx, |ui| {
                ui.allocate_exact_size(screen.size(), Sense::hover());

                let tab_width = tab_rect.width() / PAGES.len() as f32;
                for (index, label) in PAGES.iter().enumerate() {
                    let cell = Rect::from_min_size(
                        pos2(tab_rect.left() + index as f32 * tab_width, tab_rect.top()),
                        vec2((tab_width - m.u(0.12)).max(1.0), tabs_h),
                    );
                    let active = state.panel.tab == index;
                    ui.painter().rect_filled(
                        cell,
                        5.0,
                        if active {
                            ink.gamma_multiply(0.28)
                        } else {
                            colors.control_fill.to_color32()
                        },
                    );
                    ui.painter().text(
                        cell.center(),
                        Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(10.5),
                        if active { ink } else { text },
                    );
                    if ui
                        .interact(cell, id.with(("potion-page", index)), Sense::click())
                        .clicked()
                    {
                        state.panel.tab = index;
                    }
                }

                let changed = potion::workspace(
                    ui,
                    &app.cx,
                    id.with("workspace"),
                    body,
                    module,
                    &mut state.panel,
                    ink,
                );
                changed_values = changed
                    .into_iter()
                    .filter_map(|index| {
                        module
                            .params
                            .get(index)
                            .copied()
                            .map(|value| (index, value))
                    })
                    .collect();
                imported = potion::asks_for_wavetable(&mut state.panel, &[]);
                let x = filled_circle_button(ui, id.with("close"), disc.center(), disc.width(), bg);
                draw_icon(ui.painter(), disc, IconId::Close, text, 0.7, m);
                closed = x.clicked();
            });
    }

    if !changed_values.is_empty() {
        app.channel_param_edited();
        for (index, value) in changed_values {
            let _ = app.audio.send(AudioCommand::SetParam {
                channel,
                instance_id: module_id,
                index: index as i32,
                value,
            });
        }
    }

    if imported {
        return Some(Nav::Open(Overlay::ModuleSamplePicker {
            channel,
            module: module_id,
        }));
    }
    if closed {
        *state = State::default();
        return Some(Nav::Close);
    }
    None
}
