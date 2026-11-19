//! The nine control pages.
//!
//! Visible values come from the investigation findings rather than invented
//! numbers:
//!
//! * fnd_1 — nine pages, ten EQ bands (31 Hz – 16 kHz), −12..+12 dB in 0.1 dB
//!   steps, bass crossover defaults to 80 Hz (10..1000 Hz).
//! * fnd_2 — the eight factory profiles and their five 0..1 effect levels,
//!   plus Night's SVM mode.
//! * fnd_3 — Encoder offers No Encoder / Dolby Digital / DTS with a
//!   Cinema/Music mode, a gain slider and a preferred 24-bit / 48 kHz output;
//!   Scout Mode is a single toggle with a user-captured hotkey.
//!
//! Every control writes back into `AuroraApp::state` and calls
//! `mark_dirty()` so the debounced save in `app.rs` persists it.

use crate::app::{
    self, accent_for_page, big_number, breadcrumb, default_eq_output, header_line, icon_label,
    page_header, small_note, vertical_divider, AuroraApp,
};
use aurora_audio_command::eq_core;
use crate::state::{
    eq_presets, OutputMode, Page, AUDIO_QUALITIES, DIALOG_MODES, EQ_FREQS, EQ_MAX_DB, EQ_MIN_DB,
    EQ_STEP_DB, HEADPHONE_LOADS, LIGHT_COLORS, LIGHT_DIRECTIONS, LIGHT_PATTERNS, MIC_BOOST_MAX,
    PROFILES, SPEAKER_CONFIGS, SPEAKER_TYPES, SURROUND_MODES, SVM_MODES, VOICE_FX,
};
use crate::theme::{self, *};
use eframe::egui::{
    self, Align, Align2, Color32, Layout, Pos2, Rect, Response, RichText, Rounding, Sense, Stroke,
    Ui, Vec2,
};

/// Microphone EQ choices: not present in the recovered state struct, so the
/// four labels are a neutral placeholder set (estimated).
const MIC_EQ: [&str; 4] = ["Flat", "Vocal", "Broadcast", "Warm"];
const ENCODER_MODES: [&str; 3] = ["No Encoder", "Dolby Digital", "DTS"];

// ---------------------------------------------------------------- helpers

fn txt(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    ui.label(RichText::new(text).font(app::font_id(size)).color(color));
}

fn strong(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    ui.label(RichText::new(text).font(theme::bold(size)).color(color));
}

fn page_top(ui: &mut Ui, crumb: &str, title: &str, subtitle: &str) {
    breadcrumb(ui, &["Aurora X5", crumb]);
    page_header(ui, title, Some(subtitle));
}

fn card(ui: &mut Ui, title: &str, hint: Option<&str>, add: impl FnOnce(&mut Ui)) {
    theme::card_frame().show(ui, |ui| {
        app::set_width(ui, ui.available_width());
        ui.horizontal(|ui| {
            theme::small_caps(ui, title);
            if let Some(hint) = hint {
                txt(ui, hint, 10.0, TEXT_MUTED);
            }
        });
        theme::gap(ui, 6.0);
        add(ui);
    });
}

fn kv(ui: &mut Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        txt(ui, key, 11.0, TEXT_MUTED);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            strong(ui, value, 11.0, TEXT_DIM);
        });
    });
}

fn fixed_label(ui: &mut Ui, text: &str, width: f32) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 18.0),
        Layout::left_to_right(Align::Center),
        |ui| txt(ui, text, 11.0, TEXT_DIM),
    );
}

fn meter(ui: &mut Ui, frac: f32, color: Color32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 8.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(3.0), Color32::from_rgb(40, 43, 64));
    let w = rect.width() * frac.clamp(0.0, 1.0);
    ui.painter().rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(w, rect.height())),
        Rounding::same(3.0),
        color,
    );
}

fn level_meter(ui: &mut Ui, frac: f32, accent: Color32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 14.0), Sense::hover());
    let segments = 32usize;
    let gap = 2.0;
    let seg_w = (rect.width() - gap * (segments as f32 - 1.0)) / segments as f32;
    for i in 0..segments {
        let t = i as f32 / segments as f32;
        let on = t <= frac;
        let color = if !on {
            Color32::from_rgb(46, 50, 74)
        } else if t > 0.86 {
            DANGER
        } else if t > 0.7 {
            Color32::from_rgb(255, 190, 80)
        } else {
            accent
        };
        let seg = Rect::from_min_size(
            Pos2::new(rect.left() + i as f32 * (seg_w + gap), rect.top()),
            Vec2::new(seg_w, rect.height()),
        );
        ui.painter().rect_filled(seg, Rounding::same(1.0), color);
    }
}

fn chip(ui: &mut Ui, text: &str, selected: bool, accent: Color32, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::click());
    let fill = if selected {
        Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 42)
    } else if response.hovered() {
        BUTTON_HOVER
    } else {
        BUTTON
    };
    ui.painter().rect_filled(rect, Rounding::same(5.0), fill);
    ui.painter().rect_stroke(
        rect,
        Rounding::same(5.0),
        Stroke::new(
            if selected { 1.4_f32 } else { 1.0_f32 },
            if selected { accent } else { STROKE_SOFT },
        ),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        if selected {
            theme::bold(11.0)
        } else {
            theme::font(11.0)
        },
        if selected { TEXT } else { TEXT_DIM },
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn chip_toggle(ui: &mut Ui, text: &str, value: &mut bool, accent: Color32) -> bool {
    let w = (ui
        .painter()
        .layout_no_wrap(text.to_owned(), theme::font(10.5), TEXT_DIM)
        .size()
        .x
        + 18.0)
        .max(48.0);
    if chip(ui, text, *value, accent, w).clicked() {
        *value = !*value;
        true
    } else {
        false
    }
}

fn toggle_row(
    ui: &mut Ui,
    title: &str,
    hint: Option<&str>,
    value: &mut bool,
    enabled: bool,
) -> Response {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            txt(ui, title, 11.5, if enabled { TEXT } else { TEXT_MUTED });
            if let Some(hint) = hint {
                txt(ui, hint, 9.5, TEXT_MUTED);
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            theme::toggle_switch(ui, value, enabled)
        })
        .inner
    })
    .inner
}

fn combo_row(
    ui: &mut Ui,
    title: &str,
    id: &str,
    options: &[&str],
    index: &mut usize,
    enabled: bool,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        txt(ui, title, 11.0, if enabled { TEXT_DIM } else { TEXT_MUTED });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let current = options.get(*index).copied().unwrap_or("");
            if let Some(picked) = theme::combo(ui, id, current, options, 150.0, enabled) {
                if picked != *index {
                    *index = picked;
                    changed = true;
                }
            }
        });
    });
    changed
}

fn param_row(
    ui: &mut Ui,
    title: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    suffix: &str,
    enabled: bool,
) -> Response {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            txt(ui, title, 11.0, if enabled { TEXT_DIM } else { TEXT_MUTED });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                strong(
                    ui,
                    &format!("{}{}", app::formatted_value(*value), suffix),
                    11.5,
                    if enabled { TEXT } else { TEXT_MUTED },
                );
            });
        });
        let width = ui.available_width();
        theme::cyan_slider(ui, width, value, min, max, enabled)
    })
    .inner
}

fn band_name(freq: u32) -> String {
    if freq >= 1000 {
        format!("{} kHz", freq / 1000)
    } else {
        format!("{} Hz", freq)
    }
}

/// A segmented control right-aligned on the same row as its label.
///
/// `theme::segmented` follows the layout direction of its parent, so nesting it
/// in a right-to-left row would reverse the button order. The labels are
/// reversed here and the selected index mapped back, which keeps the visual
/// order intact while still hugging the right edge exactly.
fn segmented_row(
    ui: &mut Ui,
    title: &str,
    labels: &[&str],
    index: &mut usize,
    enabled: bool,
) -> bool {
    let mut changed = false;
    let reversed: Vec<&str> = labels.iter().rev().copied().collect();
    let mut reversed_index = labels.len().saturating_sub(1).saturating_sub(*index);
    ui.horizontal(|ui| {
        txt(ui, title, 11.0, if enabled { TEXT_DIM } else { TEXT_MUTED });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let before = reversed_index;
            theme::segmented(ui, &reversed, &mut reversed_index, enabled);
            if reversed_index != before {
                *index = labels.len().saturating_sub(1).saturating_sub(reversed_index);
                changed = true;
            }
        });
    });
    changed
}

// ---------------------------------------------------------------- shared art

fn profile_card(
    ui: &mut Ui,
    profile: &'static crate::state::ProfileDef,
    selected: bool,
    now: f64,
) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 74.0), Sense::click());
    let accent = Color32::from_rgb(profile.accent[0], profile.accent[1], profile.accent[2]);
    let background = if selected {
        Color32::from_rgb(46, 52, 78)
    } else if response.hovered() {
        Color32::from_rgb(51, 55, 84)
    } else {
        Color32::from_rgb(43, 46, 71)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(7.0), background);
    painter.rect_stroke(
        rect,
        Rounding::same(7.0),
        Stroke::new(
            if selected { 1.5_f32 } else { 1.0_f32 },
            if selected { accent } else { STROKE_SOFT },
        ),
    );
    let thumb = Rect::from_min_size(rect.min + Vec2::new(7.0, 7.0), Vec2::splat(60.0));
    theme::paint_profile_art(painter, thumb, profile.art, profile.accent, now, selected);
    let tx = thumb.right() + 9.0;
    painter.text(
        Pos2::new(tx, rect.top() + 19.0),
        Align2::LEFT_CENTER,
        profile.name,
        theme::bold(12.5),
        if selected { TEXT } else { TEXT_DIM },
    );
    painter.text(
        Pos2::new(tx, rect.top() + 35.0),
        Align2::LEFT_CENTER,
        profile.tagline,
        theme::font(9.0),
        TEXT_MUTED,
    );
    // Five level ticks mirror the Surround / Crystalizer / Bass / SVM / Dialog
    // levels of this profile (fnd_2).
    let bar_w = 9.0;
    for (i, level) in profile.levels.iter().enumerate() {
        let base = Rect::from_min_size(
            Pos2::new(tx + i as f32 * (bar_w + 3.0), rect.top() + 51.0),
            Vec2::new(bar_w, 3.0),
        );
        painter.rect_filled(base, Rounding::same(1.5), Color32::from_rgb(60, 64, 94));
        let filled = Rect::from_min_size(base.min, Vec2::new(bar_w * (level / 100.0), 3.0));
        painter.rect_filled(filled, Rounding::same(1.5), accent.gamma_multiply(0.9));
    }
    if selected {
        painter.text(
            Pos2::new(rect.right() - 10.0, rect.top() + 12.0),
            Align2::RIGHT_CENTER,
            "APPLIED",
            theme::font(8.0),
            accent,
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn output_card(
    ui: &mut Ui,
    icon: Icon,
    title: &str,
    detail: &str,
    selected: bool,
    accent: Color32,
) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 58.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(7.0),
        if selected {
            Color32::from_rgb(48, 54, 82)
        } else {
            CARD
        },
    );
    painter.rect_stroke(
        rect,
        Rounding::same(7.0),
        Stroke::new(
            if selected { 1.5_f32 } else { 1.0_f32 },
            if selected { accent } else { STROKE_SOFT },
        ),
    );
    let icon_rect = Rect::from_min_size(rect.min + Vec2::new(12.0, 15.0), Vec2::splat(28.0));
    theme::draw_icon(
        painter,
        icon_rect,
        icon,
        if selected { accent } else { TEXT_DIM },
        1.4,
    );
    painter.text(
        Pos2::new(icon_rect.right() + 12.0, rect.center().y - 8.0),
        Align2::LEFT_CENTER,
        title,
        theme::bold(12.5),
        TEXT,
    );
    painter.text(
        Pos2::new(icon_rect.right() + 12.0, rect.center().y + 9.0),
        Align2::LEFT_CENTER,
        detail,
        theme::font(9.5),
        TEXT_MUTED,
    );
    if selected {
        painter.circle_filled(Pos2::new(rect.right() - 18.0, rect.center().y), 5.0, accent);
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn radar(ui: &mut Ui, time: f64, active: bool, accent: Color32) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 150.0),
        Sense::hover(),
    );
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(6.0), Color32::from_rgb(35, 39, 61));
    let center = rect.center();
    let radius = rect.height() * 0.42;
    for ring in 1..=3 {
        painter.circle_stroke(
            center,
            radius * ring as f32 / 3.0,
            Stroke::new(1.0_f32, Color32::from_rgb(58, 64, 92)),
        );
    }
    let faint = Color32::from_rgb(54, 60, 88);
    painter.line_segment(
        [center - Vec2::new(radius, 0.0), center + Vec2::new(radius, 0.0)],
        Stroke::new(1.0_f32, faint),
    );
    painter.line_segment(
        [center - Vec2::new(0.0, radius), center + Vec2::new(0.0, radius)],
        Stroke::new(1.0_f32, faint),
    );
    let angle = time as f32 * 1.6;
    let sweep = center + Vec2::new(angle.cos() * radius, angle.sin() * radius);
    painter.line_segment(
        [center, sweep],
        Stroke::new(1.5_f32, accent.gamma_multiply(if active { 0.9 } else { 0.22 })),
    );
    let contacts = [(-0.42_f32, 0.22_f32), (0.3, -0.35), (0.52, 0.4), (-0.2, -0.55)];
    for (i, (dx, dy)) in contacts.iter().enumerate() {
        let point = center + Vec2::new(dx * radius, dy * radius);
        let pulse = ((time as f32 * 2.0 + i as f32).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        painter.circle_filled(
            point,
            3.0,
            accent.gamma_multiply(if active { 0.3 + 0.65 * pulse } else { 0.14 }),
        );
    }
    painter.text(
        Pos2::new(rect.right() - 10.0, rect.top() + 10.0),
        Align2::RIGHT_TOP,
        if active { "SCANNING" } else { "STANDBY" },
        theme::font(8.5),
        if active { accent } else { TEXT_MUTED },
    );
}

fn lighting_preview(
    ui: &mut Ui,
    on: bool,
    pattern: usize,
    color: [u8; 3],
    brightness: f32,
    speed: f32,
    time: f64,
) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 150.0),
        Sense::hover(),
    );
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(7.0), Color32::from_rgb(33, 36, 57));
    if !on {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "LIGHTING OFF",
            theme::bold(12.0),
            TEXT_MUTED,
        );
        return;
    }
    let base = Color32::from_rgb(color[0], color[1], color[2]);
    let bright = (brightness / 100.0).clamp(0.05, 1.0);
    for ring in (1..=6).rev() {
        let radius = rect.height() * 0.55 * (ring as f32 / 6.0);
        painter.circle_filled(
            rect.center(),
            radius,
            Color32::from_rgba_unmultiplied(
                color[0],
                color[1],
                color[2],
                (7.0 * bright * ring as f32 / 6.0) as u8,
            ),
        );
    }
    let count = 14usize;
    let phase_speed = 0.4 + speed / 100.0 * 1.8;
    for i in 0..count {
        let f = (i as f32 + 0.5) / count as f32;
        let phase = time as f32 * phase_speed + i as f32 * 0.5;
        let (dx, dy, alpha) = match pattern {
            0 => (f, 0.5, 0.3 + 0.7 * (phase.sin() * 0.5 + 0.5)),
            1 => (f, 0.5 + 0.16 * (phase * 0.7).sin(), 0.85),
            2 => (f, 0.5 + 0.28 * (f * 6.0 - phase).sin(), 0.8),
            3 => (0.5, 0.5, 0.2 + 0.8 * phase.sin().abs()),
            4 => (f, 0.5 + 0.1 * (phase * 0.4).sin(), 0.35 + 0.65 * (f * 3.0).sin().abs()),
            _ => (f, 0.5, 0.45 + 0.55 * ((phase + f * 6.28).sin() * 0.5 + 0.5)),
        };
        let pos = rect.min + Vec2::new(rect.width() * dx, rect.height() * dy);
        painter.circle_filled(pos, 5.5, base.gamma_multiply(bright * alpha));
    }
    painter.text(
        Pos2::new(rect.left() + 10.0, rect.top() + 10.0),
        Align2::LEFT_TOP,
        LIGHT_PATTERNS[pattern.min(LIGHT_PATTERNS.len() - 1)],
        theme::font(9.0),
        TEXT_MUTED,
    );
}

// ---------------------------------------------------------------- SBX

pub fn sbx_page(app: &mut AuroraApp, ui: &mut Ui) {
    let accent = accent_for_page(Page::Sbx);
    let direct = app.state.is_direct();
    let processing = app.state.processing_allowed();
    let sbx_on = app.state.effects.sbx_on;
    let sbx_active = app.state.sbx_active();
    let mut want_on = sbx_on;
    let mut toggle = None;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            page_top(
                ui,
                "SBX Profile",
                "SBX Profile",
                "Factory-tuned profiles, each with five effect levels",
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            toggle = Some(theme::toggle_switch(ui, &mut want_on, processing));
            theme::pill(
                ui,
                if sbx_active { "SBX ACTIVE" } else { "BYPASSED" },
                if sbx_active { GREEN } else { TEXT_MUTED },
            );
            txt(ui, "SBX processing", 11.0, TEXT_DIM);
        });
    });
    if let Some(response) = toggle {
        if response.changed() {
            app.state.effects.sbx_on = want_on;
            app.mark_dirty();
            app.toast(if want_on {
                "SBX processing enabled"
            } else {
                "SBX processing bypassed"
            });
        }
    }
    header_line(ui);

    if direct {
        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(255, 168, 64, 22))
            .rounding(Rounding::same(6.0))
            .inner_margin(egui::Margin::same(9.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), Sense::hover());
                    theme::draw_icon(
                        ui.painter(),
                        icon_rect,
                        Icon::Info,
                        Color32::from_rgb(255, 190, 90),
                        1.2,
                    );
                    txt(
                        ui,
                        "Direct Mode bypasses SBX, EQ and Scout processing.",
                        11.0,
                        Color32::from_rgb(255, 205, 140),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::ghost_button(ui, "Turn off Direct Mode").clicked() {
                            app.state.toggle_direct();
                            app.mark_dirty();
                            app.toast("Direct Mode off — SBX restored");
                        }
                    });
                });
            });
        ui.add_space(8.0);
    }

    let active = app.state.effects.active_profile.clone();
    let now = app.now;
    let mut picked: Option<usize> = None;
    ui.columns(4, |cols| {
        for column in cols.iter_mut() {
            column.spacing_mut().item_spacing.y = 6.0;
        }
        for (index, profile) in PROFILES.iter().enumerate() {
            let column = &mut cols[index % 4];
            let selected = active.eq_ignore_ascii_case(profile.name);
            if profile_card(column, profile, selected, now).clicked() {
                picked = Some(index);
            }
        }
    });
    if let Some(index) = picked {
        app.profile_scroll = index;
        app.apply_profile(PROFILES[index].name);
    }

    ui.add_space(10.0);
    ui.columns(2, |cols| {
        // ---------------- left: five effect levels
        let mut level_changed = false;
        theme::card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Effect levels");
            ui.add_space(4.0);
            let enabled = app.state.sbx_active();
            {
                let effects = &mut app.state.effects;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    level_changed |= theme::knob(
                        ui,
                        "sbx_surround",
                        54.0,
                        &mut effects.surround,
                        0.0,
                        100.0,
                        "Surround",
                        "%",
                        enabled && effects.surround_on,
                        accent,
                    )
                    .changed();
                    level_changed |= theme::knob(
                        ui,
                        "sbx_crystalizer",
                        54.0,
                        &mut effects.crystalizer,
                        0.0,
                        100.0,
                        "Crystal",
                        "%",
                        enabled && effects.crystalizer_on,
                        accent,
                    )
                    .changed();
                    level_changed |= theme::knob(
                        ui,
                        "sbx_bass",
                        54.0,
                        &mut effects.bass,
                        0.0,
                        100.0,
                        "Bass",
                        "%",
                        enabled && effects.bass_on,
                        accent,
                    )
                    .changed();
                    level_changed |= theme::knob(
                        ui,
                        "sbx_svm",
                        54.0,
                        &mut effects.smart_vol,
                        0.0,
                        100.0,
                        "Smart",
                        "%",
                        enabled && effects.smart_on,
                        accent,
                    )
                    .changed();
                    level_changed |= theme::knob(
                        ui,
                        "sbx_dialog",
                        54.0,
                        &mut effects.dialog,
                        0.0,
                        100.0,
                        "Dialog",
                        "%",
                        enabled && effects.dialog_on,
                        accent,
                    )
                    .changed();
                });
            }
            if level_changed {
                app.state.effects.mark_personal();
                app.mark_dirty();
            }
            ui.add_space(16.0);
            theme::hline(ui, STROKE_SOFT);
            ui.add_space(6.0);
            theme::small_caps(ui, "Enabled effects");
            ui.add_space(5.0);
            let mut effect_toggle = false;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let enabled = app.state.sbx_active();
                effect_toggle |= chip_toggle(ui, "Surround", &mut app.state.effects.surround_on, accent);
                effect_toggle |=
                    chip_toggle(ui, "Crystalizer", &mut app.state.effects.crystalizer_on, accent);
                effect_toggle |= chip_toggle(ui, "Bass", &mut app.state.effects.bass_on, accent);
                effect_toggle |= chip_toggle(ui, "Smart Vol", &mut app.state.effects.smart_on, accent);
                effect_toggle |= chip_toggle(ui, "Dialog+", &mut app.state.effects.dialog_on, accent);
                let _ = enabled;
            });
            if effect_toggle {
                app.state.effects.mark_personal();
                app.mark_dirty();
            }
        });

        // ---------------- right: applied profile detail
        theme::card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            let name = app.state.effects.active_profile.clone();
            let personal = name == "Personal";
            let definition = app::profile_def(&name);
            ui.horizontal(|ui| {
                strong(
                    ui,
                    if personal { "Personal" } else { definition.name },
                    16.0,
                    TEXT,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    theme::pill(
                        ui,
                        if app.state.effects.sbx_on { "APPLIED" } else { "PAUSED" },
                        if app.state.effects.sbx_on { GREEN } else { TEXT_MUTED },
                    );
                    let active_index = PROFILES
                        .iter()
                        .position(|candidate| candidate.name.eq_ignore_ascii_case(&name))
                        .unwrap_or(app.profile_scroll);
                    txt(
                        ui,
                        &format!("profile {} / {}", active_index.min(PROFILES.len() - 1) + 1, PROFILES.len()),
                        9.0,
                        TEXT_MUTED,
                    );
                });
            });
            ui.add_space(2.0);
            txt(
                ui,
                if personal {
                    "Your own combination of the five effect levels."
                } else {
                    definition.description
                },
                10.5,
                TEXT_DIM,
            );
            ui.add_space(7.0);
            let enabled = app.state.sbx_active();
            let mut surround_mode = app.state.effects.surround_mode as usize;
            let mut svm_mode = app.state.effects.svm_mode as usize;
            let mut dialog_mode = app.state.effects.dialog_mode as usize;
            theme::small_caps(ui, "Surround mode");
            theme::segmented(ui, &SURROUND_MODES, &mut surround_mode, enabled);
            ui.add_space(3.0);
            theme::small_caps(ui, "Smart Volume mode");
            theme::segmented(ui, &SVM_MODES, &mut svm_mode, enabled);
            ui.add_space(3.0);
            theme::small_caps(ui, "Dialog mode");
            theme::segmented(ui, &DIALOG_MODES, &mut dialog_mode, enabled);
            let modes_after = (
                surround_mode.min(SURROUND_MODES.len() - 1),
                svm_mode.min(SVM_MODES.len() - 1),
                dialog_mode.min(DIALOG_MODES.len() - 1),
            );
            if modes_after
                != (
                    app.state.effects.surround_mode as usize,
                    app.state.effects.svm_mode as usize,
                    app.state.effects.dialog_mode as usize,
                )
            {
                app.state.effects.surround_mode = modes_after.0 as u8;
                app.state.effects.svm_mode = modes_after.1 as u8;
                app.state.effects.dialog_mode = modes_after.2 as u8;
                app.mark_dirty();
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if theme::primary_button(ui, "Save as profile", true).clicked() {
                    app.show_save_profile = true;
                    app.save_profile_name = if personal {
                        "My profile".to_string()
                    } else {
                        format!("{} custom", definition.name)
                    };
                }
                if theme::ghost_button(ui, "Restore defaults").clicked() {
                    app.confirm_reset = true;
                }
            });
        });
    });

    if !app.state.custom_profiles.is_empty() {
        ui.add_space(10.0);
        theme::small_caps(ui, "Saved profiles");
        ui.add_space(5.0);
        let names: Vec<String> = app
            .state
            .custom_profiles
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
            for name in &names {
                let selected = app.state.effects.active_profile == *name;
                if chip(ui, name, selected, accent, 118.0).clicked() {
                    let restored = app
                        .state
                        .custom_profiles
                        .iter()
                        .find(|(candidate, _)| candidate == name)
                        .map(|(_, effects)| effects.clone());
                    if let Some(effects) = restored {
                        app.state.effects = effects;
                        app.mark_dirty();
                        app.toast(format!("Profile \"{name}\" applied"));
                    }
                }
            }
        });
    }
}

// ---------------------------------------------------------------- Equalizer

/// dB response of the current curve on a log axis, computed with the same
/// kernel that renders WAV files, so the graph never drifts from the audio.
fn eq_response_curve(bands: &[f32; 10], preamp: f32) -> Vec<f32> {
    let mut gains = [0.0f64; 10];
    for (slot, value) in gains.iter_mut().zip(bands.iter()) {
        *slot = *value as f64;
    }
    let curve = eq_core::Curve::from_bands(&gains, preamp as f64);
    match eq_core::Chain::design(&curve, 48_000, 0.0) {
        Ok(chain) => {
            let lo = eq_core::BAND_FREQS[0];
            let hi = eq_core::BAND_FREQS[9];
            (0..theme::EQ_RESPONSE_POINTS)
                .map(|i| {
                    let t = i as f64 / (theme::EQ_RESPONSE_POINTS - 1) as f64;
                    chain.response_db(lo * (hi / lo).powf(t), 48_000.0) as f32
                })
                .collect()
        }
        Err(_) => vec![0.0; theme::EQ_RESPONSE_POINTS],
    }
}

fn response_peak(response: &[f32]) -> Option<(f32, f32)> {
    let (index, db) = response
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))?;
    let lo = eq_core::BAND_FREQS[0] as f32;
    let hi = eq_core::BAND_FREQS[9] as f32;
    let t = index as f32 / (response.len().max(2) - 1) as f32;
    Some((*db, lo * (hi / lo).powf(t)))
}

fn format_freq(f: f32) -> String {
    if f >= 1000.0 {
        format!("{:.1} kHz", f / 1000.0)
    } else {
        format!("{:.0} Hz", f)
    }
}

pub fn equalizer_page(app: &mut AuroraApp, ui: &mut Ui) {
    let accent = accent_for_page(Page::Equalizer);
    let enabled = app.state.eq.enabled;
    let eq_active = app.state.eq_active();
    let mut want_enabled = enabled;
    let mut toggle = None;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            page_top(
                ui,
                "Equalizer",
                "Equalizer",
                "10-band graphic EQ · 31 Hz – 16 kHz · ±12 dB · 0.1 dB steps",
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            toggle = Some(theme::toggle_switch(ui, &mut want_enabled, true));
            theme::pill(
                ui,
                if eq_active { "EQ ON" } else { "EQ OFF" },
                if eq_active { GREEN } else { TEXT_MUTED },
            );
        });
    });
    if let Some(response) = toggle {
        if response.changed() {
            app.state.eq.enabled = want_enabled;
            app.mark_dirty();
        }
    }
    header_line(ui);

    // Preset toolbar -------------------------------------------------
    let current_preset = app.state.eq.preset.clone();
    let custom_names: Vec<String> = app.state.eq.custom.iter().map(|p| p.name.clone()).collect();
    let mut options: Vec<String> = eq_presets().iter().map(|p| p.name.clone()).collect();
    options.push("Personal".to_string());
    options.extend(custom_names.iter().cloned());
    let option_refs: Vec<&str> = options.iter().map(|name| name.as_str()).collect();
    let factory_mode = app.state.playback.output;

    ui.horizontal(|ui| {
        theme::small_caps(ui, "Preset");
        if let Some(index) = theme::combo(ui, "eq_preset", &current_preset, &option_refs, 160.0, true)
        {
            let picked = options[index].clone();
            app.state.eq.apply_preset(&picked, factory_mode);
            app.mark_dirty();
            app.toast(format!("Preset \"{picked}\" loaded"));
        }
        ui.add_space(4.0);
        if theme::ghost_button(ui, "Save preset").clicked() {
            app.show_save_eq = true;
            app.save_eq_name = if current_preset == "Personal" {
                "My preset".to_string()
            } else {
                format!("{current_preset} custom")
            };
        }
        if theme::ghost_button(ui, "Reset curve").clicked() {
            app.confirm_reset_eq = true;
        }
        if theme::ghost_button(ui, "Render WAV…").clicked() {
            app.show_eq_render = true;
            app.eq_render_status = None;
            if app.eq_wav_out.trim().is_empty() && !app.eq_wav_in.trim().is_empty() {
                app.eq_wav_out = default_eq_output(&app.eq_wav_in);
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::icon_button(ui, Icon::Info, 20.0, true)
                .on_hover_text("About these bands")
                .clicked()
            {
                app.show_eq_info = !app.show_eq_info;
            }
            let mode_label = if app.state.playback.output == OutputMode::Speakers {
                "SPEAKER"
            } else {
                "HEADPHONE"
            };
            let sync_label = if app.state.eq.sync_outputs {
                "SYNCED"
            } else {
                "PER-OUTPUT"
            };
            txt(ui, &format!("{mode_label} · {sync_label}"), 9.5, TEXT_MUTED);
        });
    });

    if app.show_eq_info {
        ui.add_space(6.0);
        egui::Frame::none()
            .fill(PANEL)
            .rounding(Rounding::same(6.0))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                txt(
                    ui,
                    &format!(
                        "Bands: 31, 62, 125, 250, 500 Hz, 1, 2, 4, 8 and 16 kHz. Range −12 to +12 dB in {EQ_STEP_DB} dB steps. Bass crossover defaults to 80 Hz (10–1000 Hz)."
                    ),
                    10.5,
                    TEXT_DIM,
                );
                small_note(
                    ui,
                    "Click or drag the curve to move the nearest band; drag a fader for fine control.",
                );
            });
    }

    // Interactive curve ----------------------------------------------
    ui.add_space(8.0);
    let response_curve = eq_response_curve(&app.state.eq.bands, app.state.eq.preamp);
    let (rect, graph_response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 178.0),
        Sense::click_and_drag(),
    );
    theme::draw_eq_graph(
        ui.painter(),
        rect,
        &app.state.eq.bands,
        &response_curve,
        app.eq_selected,
        app.state.eq_active(),
    );
    if app.state.eq_active() && (graph_response.clicked() || graph_response.dragged()) {
        if let Some(pos) = ui.ctx().pointer_interact_pos() {
            let x0 = rect.left() + theme::EQ_GRAPH_PAD_L;
            let x1 = rect.right() - theme::EQ_GRAPH_PAD_R;
            let y0 = rect.top() + theme::EQ_GRAPH_PAD_T;
            let y1 = rect.bottom() - theme::EQ_GRAPH_PAD_B;
            if pos.x >= x0 - 14.0 && pos.x <= x1 + 14.0 && pos.y >= y0 && pos.y <= y1 {
                let index = (0..10)
                    .min_by(|a, b| {
                        let da = (theme::eq_band_x(rect, EQ_FREQS[*a] as f32) - pos.x).abs();
                        let db = (theme::eq_band_x(rect, EQ_FREQS[*b] as f32) - pos.x).abs();
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap_or(0);
                let t = ((y1 - pos.y) / (y1 - y0)).clamp(0.0, 1.0);
                let db = (((EQ_MIN_DB + t * (EQ_MAX_DB - EQ_MIN_DB)) * 10.0).round() / 10.0)
                    .clamp(EQ_MIN_DB, EQ_MAX_DB);
                app.state.eq.bands[index] = db;
                app.state.eq.preset = "Personal".to_string();
                app.eq_selected = Some(index);
                app.mark_dirty();
            }
        }
    }

    // Ten faders -----------------------------------------------------
    ui.add_space(2.0);
    let mut selected = app.eq_selected;
    let mut fader_changed = false;
    let fader_enabled = app.state.eq_active();
    ui.horizontal(|ui| {
        let total = 10.0 * 43.0 + 9.0 * 6.0;
        ui.add_space(((ui.available_width() - total) * 0.5).max(0.0));
        ui.spacing_mut().item_spacing.x = 6.0;
        for index in 0..10 {
            let response = theme::eq_fader(
                ui,
                EQ_FREQS[index],
                &mut app.state.eq.bands[index],
                fader_enabled,
                selected == Some(index),
            );
            if response.clicked() || response.dragged() {
                selected = Some(index);
            }
            fader_changed |= response.changed();
        }
    });
    app.eq_selected = selected;
    if fader_changed {
        app.state.eq.preset = "Personal".to_string();
        app.mark_dirty();
    }

    // Trim + band readout --------------------------------------------
    ui.add_space(6.0);
    ui.columns(2, |cols| {
        theme::card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Output trim");
            ui.add_space(4.0);
            let mut preamp = app.state.eq.preamp;
            let response =
                param_row(ui, "Preamp", &mut preamp, EQ_MIN_DB, EQ_MAX_DB, " dB", app.state.eq.enabled);
            if response.changed() {
                app.state.eq.preamp = preamp;
                app.mark_dirty();
            }
            if let Some((peak_db, peak_f)) = response_peak(&response_curve) {
                txt(
                    ui,
                    &format!("Computed peak {peak_db:+.1} dB @ {}", format_freq(peak_f)),
                    9.5,
                    TEXT_MUTED,
                );
            }
            let mut sync = app.state.eq.sync_outputs;
            if toggle_row(ui, "Sync to all outputs", None, &mut sync, true).changed() {
                app.state.eq.sync_outputs = sync;
                app.mark_dirty();
            }
            // The SBX bass crossover lives on the equalizer page in the
            // product data (default 80 Hz, 10–1000 Hz).
            let mut crossover = app.state.effects.bass_crossover;
            let response = param_row(
                ui,
                "Bass crossover",
                &mut crossover,
                10.0,
                1000.0,
                " Hz",
                app.state.processing_allowed(),
            );
            if response.changed() {
                app.state.effects.bass_crossover = crossover;
                app.mark_dirty();
            }
        });

        theme::card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Selected band");
            ui.add_space(2.0);
            if let Some(index) = app.eq_selected {
                ui.horizontal(|ui| {
                    strong(ui, &band_name(EQ_FREQS[index]), 19.0, TEXT);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        strong(
                            ui,
                            &format!("{:+.1} dB", app.state.eq.bands[index]),
                            14.0,
                            accent,
                        );
                    });
                });
            } else {
                txt(
                    ui,
                    "Click the curve or a fader to pick a band.",
                    10.5,
                    TEXT_MUTED,
                );
            }
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                if theme::ghost_button(ui, "Flat (0 dB)").clicked() {
                    app.state.eq.bands = [0.0; 10];
                    app.state.eq.preset = "Flat".to_string();
                    app.mark_dirty();
                    app.toast("All bands set to 0 dB");
                }
                if theme::ghost_button(ui, "Save preset").clicked() {
                    app.show_save_eq = true;
                }
            });
        });
    });
}

// ---------------------------------------------------------------- Playback

pub fn playback_page(app: &mut AuroraApp, ui: &mut Ui) {
    let accent = accent_for_page(Page::Playback);
    let speakers = app.state.playback.output == OutputMode::Speakers;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            page_top(
                ui,
                "Playback",
                "Playback",
                "Speakers, headphones and the output format",
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            theme::pill(
                ui,
                if speakers { "SPEAKERS" } else { "HEADPHONES" },
                accent,
            );
        });
    });
    header_line(ui);

    let mut picked_output = None;
    ui.columns(2, |cols| {
        if output_card(
            &mut cols[0],
            Icon::Speaker,
            "Speakers",
            "Desktop or home theatre set",
            speakers,
            accent,
        )
        .clicked()
        {
            picked_output = Some(OutputMode::Speakers);
        }
        if output_card(
            &mut cols[1],
            Icon::Headphones,
            "Headphones",
            "Front-panel headphone jack",
            !speakers,
            accent,
        )
        .clicked()
        {
            picked_output = Some(OutputMode::Headphones);
        }
    });
    if let Some(mode) = picked_output {
        app.state.playback.output = mode;
        app.mark_dirty();
        app.toast(match mode {
            OutputMode::Speakers => "Output switched to speakers",
            OutputMode::Headphones => "Output switched to headphones",
        });
    }

    ui.add_space(8.0);
    let mut dirty = false;
    ui.columns(2, |cols| {
        // Speakers -------------------------------------------------
        theme::card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Speakers");
            ui.add_space(4.0);
            dirty |= combo_row(
                ui,
                "Layout",
                "playback_layout",
                &SPEAKER_CONFIGS,
                &mut app.state.playback.speaker_config,
                true,
            );
            dirty |= combo_row(
                ui,
                "Speaker type",
                "playback_type",
                &SPEAKER_TYPES,
                &mut app.state.playback.speaker_type,
                true,
            );
            dirty |= param_row(
                ui,
                "Bass crossover",
                &mut app.state.playback.crossover,
                10.0,
                1000.0,
                " Hz",
                true,
            )
            .changed();
            dirty |= toggle_row(
                ui,
                "Bass redirection",
                None,
                &mut app.state.playback.bass_redirection,
                true,
            )
            .changed();
            dirty |= toggle_row(
                ui,
                "Subwoofer gain",
                None,
                &mut app.state.playback.subwoofer_gain,
                true,
            )
            .changed();
            dirty |= toggle_row(
                ui,
                "Full-range front",
                None,
                &mut app.state.playback.full_range_front,
                true,
            )
            .changed();
            dirty |= toggle_row(
                ui,
                "Full-range surround",
                None,
                &mut app.state.playback.full_range_surround,
                true,
            )
            .changed();
            ui.add_space(2.0);
            theme::small_caps(ui, "Test tone");
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                for channel in ["Left", "Right", "Center", "Sub"] {
                    if theme::ghost_button(ui, channel).clicked() {
                        app.toast(format!("{channel} channel test tone"));
                    }
                }
            });
        });

        // Headphones -----------------------------------------------
        theme::card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Headphones");
            ui.add_space(4.0);
            dirty |= combo_row(
                ui,
                "Load",
                "playback_hp_load",
                &HEADPHONE_LOADS,
                &mut app.state.playback.headphone_load,
                true,
            );
            dirty |= segmented_row(
                ui,
                "Gain",
                &["Low", "High"],
                &mut app.state.playback.headphone_gain,
                true,
            );
            dirty |= toggle_row(
                ui,
                "Virtualisation",
                None,
                &mut app.state.playback.hp_virtualization,
                true,
            )
            .changed();
            dirty |= toggle_row(
                ui,
                "Retain gain",
                None,
                &mut app.state.playback.retain_gain,
                true,
            )
            .changed();
        });

        // Output format ---------------------------------------------
        cols[1].add_space(8.0);
        theme::card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Output format");
            ui.add_space(4.0);
            dirty |= combo_row(
                ui,
                "Quality",
                "playback_quality",
                &AUDIO_QUALITIES,
                &mut app.state.playback.audio_quality,
                true,
            );
            dirty |= toggle_row(
                ui,
                "Set as default device",
                None,
                &mut app.state.playback.set_default,
                true,
            )
            .changed();
            let mut direct = app.state.is_direct();
            let response = toggle_row(
                ui,
                "Direct Mode",
                Some("Bypasses SBX, EQ and Scout"),
                &mut direct,
                true,
            );
            if response.changed() {
                app.state.toggle_direct();
                app.mark_dirty();
                app.toast(if app.state.playback.direct {
                    "Direct Mode on — processing bypassed"
                } else {
                    "Direct Mode off — processing enabled"
                });
            }
            ui.add_space(2.0);
            if theme::ghost_button(ui, "About Direct Mode").clicked() {
                app.show_direct_info = !app.show_direct_info;
            }
            if app.show_direct_info {
                txt(
                    ui,
                    "Direct Mode sends the output untouched: SBX, the equalizer and Scout Mode are switched off while it is active.",
                    9.5,
                    TEXT_MUTED,
                );
            }
        });
    });
    if dirty {
        app.mark_dirty();
    }
}

// ---------------------------------------------------------------- Recording

pub fn recording_page(app: &mut AuroraApp, ui: &mut Ui) {
    let accent = accent_for_page(Page::Recording);
    let inputs: Vec<String> = app
        .state
        .mixer
        .recording
        .iter()
        .map(|channel| channel.name.clone())
        .collect();

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            page_top(
                ui,
                "Recording",
                "Recording",
                "Input source, microphone level and voice processing",
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            theme::pill(
                ui,
                if app.state.recording.monitor { "MONITORING" } else { "READY" },
                accent,
            );
        });
    });
    header_line(ui);

    ui.horizontal(|ui| {
        theme::small_caps(ui, "Input source");
        let mut picked = None;
        for (index, name) in inputs.iter().enumerate() {
            if chip(
                ui,
                name,
                app.state.recording.selected_input == index,
                accent,
                122.0,
            )
            .clicked()
            {
                picked = Some(index);
            }
        }
        if let Some(index) = picked {
            app.state.recording.selected_input = index;
            app.state.mixer.selected_input = index;
            app.mark_dirty();
            app.toast(format!("Input: {}", inputs[index]));
        }
    });

    ui.add_space(8.0);
    let mut dirty = false;
    ui.columns(2, |cols| {
        // Microphone ------------------------------------------------
        theme::card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Microphone");
            ui.add_space(4.0);
            let level = if app.state.mixer.mic_muted {
                0.0
            } else {
                (app.state.mixer.mic_volume / 100.0)
                    * (0.55
                        + 0.45
                            * ((app.now as f32 * 2.7).sin() * 0.5 + 0.5).clamp(0.0, 1.0))
            };
            level_meter(ui, level, accent, ui.available_width());
            ui.add_space(2.0);
            dirty |= param_row(
                ui,
                "Mic volume",
                &mut app.state.mixer.mic_volume,
                0.0,
                100.0,
                " %",
                true,
            )
            .changed();
            dirty |= param_row(
                ui,
                "Mic boost",
                &mut app.state.mixer.mic_boost,
                0.0,
                MIC_BOOST_MAX,
                " dB",
                true,
            )
            .changed();
            dirty |= toggle_row(
                ui,
                "Mute microphone",
                None,
                &mut app.state.mixer.mic_muted,
                true,
            )
            .changed();
            dirty |=
                toggle_row(ui, "Monitor", None, &mut app.state.mixer.monitor, true).changed();
            dirty |= toggle_row(ui, "AGC", None, &mut app.state.mixer.agc, true).changed();
        });

        // Voice processing ------------------------------------------
        theme::card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Voice processing");
            ui.add_space(4.0);
            dirty |= toggle_row(
                ui,
                "Smart Volume",
                None,
                &mut app.state.recording.smart_volume,
                true,
            )
            .changed();
            dirty |= toggle_row(
                ui,
                "Noise reduction",
                None,
                &mut app.state.recording.noise_reduction,
                true,
            )
            .changed();
            dirty |= toggle_row(
                ui,
                "Acoustic echo cancellation",
                None,
                &mut app.state.recording.aec,
                true,
            )
            .changed();
            dirty |= toggle_row(
                ui,
                "Voice focus",
                None,
                &mut app.state.recording.voice_focus,
                true,
            )
            .changed();
            ui.add_space(2.0);
            dirty |= combo_row(
                ui,
                "Mic EQ",
                "recording_mic_eq",
                &MIC_EQ,
                &mut app.state.recording.mic_eq,
                true,
            );
            dirty |= combo_row(
                ui,
                "Voice Morph",
                "recording_voice_fx",
                &VOICE_FX,
                &mut app.state.recording.voice_fx,
                true,
            );
            ui.add_space(2.0);
            icon_label(
                ui,
                Icon::Info,
                "Voice Morph uses the product's effect list.",
                TEXT_MUTED,
            );
        });
    });

    ui.add_space(8.0);
    card(ui, "Input channel levels", None, |ui| {
        for index in 0..app.state.mixer.recording.len() {
            let changed = {
                let channel = &mut app.state.mixer.recording[index];
                let mut row_changed = false;
                ui.horizontal(|ui| {
                    let selected = app.state.recording.selected_input == index;
                    if chip(
                        ui,
                        if selected { "SEL" } else { "" },
                        selected,
                        accent,
                        32.0,
                    )
                    .clicked()
                    {
                        app.state.recording.selected_input = index;
                        app.state.mixer.selected_input = index;
                        row_changed = true;
                    }
                    fixed_label(ui, &channel.name, 104.0);
                    row_changed |= theme::cyan_slider(
                        ui,
                        180.0,
                        &mut channel.volume,
                        0.0,
                        100.0,
                        true,
                    )
                    .changed();
                    strong(
                        ui,
                        &format!("{}", channel.volume.round() as i32),
                        10.5,
                        TEXT_MUTED,
                    );
                    row_changed |= theme::toggle_switch(ui, &mut channel.muted, true).changed();
                    meter(
                        ui,
                        if channel.muted { 0.0 } else { channel.volume / 100.0 },
                        if channel.muted { TEXT_MUTED } else { accent },
                        60.0,
                    );
                });
                row_changed
            };
            if changed {
                dirty = true;
            }
        }
    });

    if dirty {
        app.mark_dirty();
    }
}

// ---------------------------------------------------------------- Scout

pub fn scout_page(app: &mut AuroraApp, ui: &mut Ui) {
    let accent = accent_for_page(Page::Scout);

    // Hotkey capture runs before drawing so the label reflects the new value.
    if app.state.scout.capturing {
        let mut captured: Option<String> = None;
        let mut cancel = false;
        ui.ctx().input(|input| {
            for event in &input.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                {
                    if *key == egui::Key::Escape {
                        cancel = true;
                    } else {
                        let mut parts: Vec<String> = Vec::new();
                        if modifiers.ctrl {
                            parts.push("Ctrl".to_string());
                        }
                        if modifiers.shift {
                            parts.push("Shift".to_string());
                        }
                        if modifiers.alt {
                            parts.push("Alt".to_string());
                        }
                        let name = format!("{key:?}");
                        if !matches!(name.as_str(), "Control" | "Shift" | "Alt" | "Super") {
                            parts.push(name);
                        }
                        let modifier_count = modifiers.ctrl as usize
                            + modifiers.shift as usize
                            + modifiers.alt as usize;
                        if parts.len() > modifier_count {
                            captured = Some(parts.join(" + "));
                        }
                    }
                }
            }
        });
        if cancel {
            app.state.scout.capturing = false;
            app.toast("Hotkey capture cancelled");
        } else if let Some(keys) = captured {
            app.state.scout.hotkey_text = keys;
            app.state.scout.hotkey_enabled = true;
            app.state.scout.capturing = false;
            app.mark_dirty();
            app.toast("Hotkey captured");
        }
    }

    let on = app.state.scout.on;
    let scout_active = app.state.scout_active();
    let direct = app.state.playback.direct;
    let mut want_on = on;
    let mut toggle = None;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            page_top(
                ui,
                "Scout Mode",
                "Scout Mode",
                "One feature toggle with a hotkey you capture yourself",
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            toggle = Some(theme::toggle_switch(ui, &mut want_on, true));
            theme::pill(
                ui,
                if scout_active { "SCOUT ON" } else { "SCOUT OFF" },
                if scout_active { GREEN } else { TEXT_MUTED },
            );
        });
    });
    if let Some(response) = toggle {
        if response.changed() {
            app.state.toggle_scout();
            app.mark_dirty();
            app.toast(if app.state.scout.on {
                "Scout Mode on — Direct Mode disabled"
            } else {
                "Scout Mode off"
            });
        }
    }
    header_line(ui);

    if direct {
        txt(
            ui,
            "Direct Mode is active, so Scout Mode will switch it off when enabled.",
            10.0,
            Color32::from_rgb(255, 190, 90),
        );
        ui.add_space(4.0);
    }

    ui.columns(2, |cols| {
        theme::card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                strong(ui, "Scout Mode", 17.0, TEXT);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    theme::pill(
                        ui,
                        if on { "ACTIVE" } else { "STANDBY" },
                        if on { accent } else { TEXT_MUTED },
                    );
                });
            });
            txt(
                ui,
                "Highlights distant cues such as footsteps and reloads so they sit in front of the mix.",
                10.5,
                TEXT_DIM,
            );
            ui.add_space(4.0);
            radar(ui, app.now, on, accent);
            ui.add_space(2.0);
            let mut radar_on = app.state.scout.radar;
            if toggle_row(ui, "Radar overlay", None, &mut radar_on, true).changed() {
                app.state.scout.radar = radar_on;
                app.mark_dirty();
            }
        });

        theme::card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Hotkey");
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut app.state.scout.hotkey_text)
                        .font(theme::font(12.5))
                        .desired_width(150.0),
                );
                if response.changed() {
                    app.mark_dirty();
                }
                let label = if app.state.scout.capturing {
                    "Listening…"
                } else {
                    "Capture"
                };
                if theme::primary_button(ui, label, true).clicked() {
                    app.state.scout.capturing = !app.state.scout.capturing;
                    app.toast(if app.state.scout.capturing {
                        "Press the key combination you want to use"
                    } else {
                        "Hotkey capture cancelled"
                    });
                }
            });
            ui.add_space(4.0);
            let mut hotkey_enabled = app.state.scout.hotkey_enabled;
            if toggle_row(ui, "Enable hotkey", None, &mut hotkey_enabled, true).changed() {
                app.state.scout.hotkey_enabled = hotkey_enabled;
                app.mark_dirty();
            }
            if app.state.scout.capturing {
                ui.add_space(4.0);
                theme::pill(ui, "WAITING FOR KEYS", accent);
            }
            ui.add_space(8.0);
            theme::hline(ui, STROKE_SOFT);
            ui.add_space(6.0);
            txt(
                ui,
                "No factory gesture is stored in the installed product data, so the hotkey starts unset and is captured by you.",
                9.5,
                TEXT_MUTED,
            );
            ui.add_space(6.0);
            kv(
                ui,
                "Direct Mode",
                if app.state.playback.direct { "On" } else { "Off" },
            );
            kv(
                ui,
                "Scout processing",
                if on { "Running" } else { "Idle" },
            );
        });
    });
}

// ---------------------------------------------------------------- Encoder

pub fn encoder_page(app: &mut AuroraApp, ui: &mut Ui) {
    let accent = accent_for_page(Page::Encoder);
    let mode = app.state.encoder.mode;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            page_top(
                ui,
                "Encoder",
                "Encoder",
                "Dolby Digital and DTS encoding for the digital output",
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            theme::pill(
                ui,
                ENCODER_MODES[mode.min(ENCODER_MODES.len() - 1)],
                if mode == 0 { TEXT_MUTED } else { accent },
            );
        });
    });
    header_line(ui);

    ui.horizontal(|ui| {
        for (index, label) in ENCODER_MODES.iter().enumerate() {
            if chip(ui, label, mode == index, accent, 150.0).clicked() && mode != index {
                app.state.encoder.mode = index;
                app.mark_dirty();
                app.toast(format!("Encoder set to {label}"));
            }
        }
    });

    ui.add_space(8.0);
    let enabled = app.state.encoder.mode != 0;
    let mut dirty = false;
    ui.columns(2, |cols| {
        theme::card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Encoder settings");
            ui.add_space(4.0);
            // `neo_mode` carries the Cinema / Music choice; 0 = Cinema,
            // 1 = Music (estimated mapping — the recovered field predates the
            // Cinema/Music split).
            if app.state.encoder.neo_mode > 1 {
                app.state.encoder.neo_mode = 0;
            }
            dirty |= segmented_row(
                ui,
                "Mode",
                &["Cinema", "Music"],
                &mut app.state.encoder.neo_mode,
                enabled,
            );
            dirty |= param_row(
                ui,
                "Gain",
                &mut app.state.encoder.width,
                0.0,
                100.0,
                " %",
                enabled,
            )
            .changed();
            ui.add_space(4.0);
            if !enabled {
                txt(
                    ui,
                    "No encoder is selected, so the output passes through unencoded.",
                    9.5,
                    TEXT_MUTED,
                );
            } else {
                txt(
                    ui,
                    "The encoded stream follows the selected digital output.",
                    9.5,
                    TEXT_MUTED,
                );
            }
        });

        theme::card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Output");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                strong(ui, "24-bit / 48 kHz", 19.0, TEXT);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    theme::pill(ui, "PREFERRED", GREEN);
                });
            });
            txt(
                ui,
                "Preferred encoder output format recorded from the product data.",
                9.5,
                TEXT_MUTED,
            );
            ui.add_space(6.0);
            kv(
                ui,
                "Current playback format",
                AUDIO_QUALITIES[app
                    .state
                    .playback
                    .audio_quality
                    .min(AUDIO_QUALITIES.len() - 1)],
            );
            kv(ui, "Encoder", ENCODER_MODES[mode.min(ENCODER_MODES.len() - 1)]);
            kv(
                ui,
                "Mode",
                if enabled {
                    if app.state.encoder.neo_mode == 0 {
                        "Cinema"
                    } else {
                        "Music"
                    }
                } else {
                    "—"
                },
            );
            kv(
                ui,
                "Gain",
                &format!("{} %", app::formatted_value(app.state.encoder.width)),
            );
            ui.add_space(6.0);
            if theme::ghost_button(ui, "Open Playback settings").clicked() {
                app.set_page(Page::Playback);
            }
        });
    });
    if dirty {
        app.mark_dirty();
    }
}

// ---------------------------------------------------------------- Mixer

pub fn mixer_page(app: &mut AuroraApp, ui: &mut Ui) {
    let accent = accent_for_page(Page::Mixer);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            page_top(
                ui,
                "Mixer",
                "Mixer",
                "Playback and recording levels, microphone and monitoring",
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            theme::pill(
                ui,
                if app.state.mixer.master_muted { "MASTER MUTED" } else { "MASTER LIVE" },
                if app.state.mixer.master_muted { DANGER } else { GREEN },
            );
        });
    });
    header_line(ui);

    // Master ---------------------------------------------------------
    theme::panel_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
            theme::draw_icon(
                ui.painter(),
                icon_rect,
                if app.state.mixer.master_muted {
                    Icon::Mute
                } else {
                    Icon::Speaker
                },
                if app.state.mixer.master_muted { DANGER } else { accent },
                1.4,
            );
            strong(ui, "Master", 14.0, TEXT);
            let mut muted = app.state.mixer.master_muted;
            if theme::toggle_switch(ui, &mut muted, true).changed() {
                app.state.mixer.master_muted = muted;
                app.mark_dirty();
            }
            vertical_divider(ui, 18.0);
            // `big_number` is a two-widget row, so it must stay in a
            // left-to-right layout or the value and its unit swap places.
            ui.add_space((ui.available_width() - 64.0).max(0.0));
            big_number(
                ui,
                &format!("{}", app.state.mixer.master.round() as i32),
                "%",
            );
        });
        let mut master = app.state.mixer.master;
        if theme::cyan_slider(ui, ui.available_width(), &mut master, 0.0, 100.0, true).changed() {
            app.state.mixer.master = master;
            app.state.mixer.master_muted = false;
            app.mark_dirty();
        }
    });

    ui.add_space(8.0);
    let mut dirty = false;
    ui.columns(2, |cols| {
        // Playback channels -----------------------------------------
        theme::card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Playback");
            ui.add_space(5.0);
            for index in 0..app.state.mixer.playback.len() {
                let changed = {
                    let channel = &mut app.state.mixer.playback[index];
                    let mut row_changed = false;
                    ui.horizontal(|ui| {
                        fixed_label(ui, &channel.name, 92.0);
                        row_changed |= theme::cyan_slider(
                            ui,
                            132.0,
                            &mut channel.volume,
                            0.0,
                            100.0,
                            true,
                        )
                        .changed();
                        strong(
                            ui,
                            &format!("{}", channel.volume.round() as i32),
                            10.5,
                            TEXT_MUTED,
                        );
                        row_changed |=
                            theme::toggle_switch(ui, &mut channel.muted, true).changed();
                    });
                    meter(
                        ui,
                        if channel.muted { 0.0 } else { channel.volume / 100.0 },
                        if channel.muted { TEXT_MUTED } else { accent },
                        ui.available_width(),
                    );
                    row_changed
                };
                if changed {
                    dirty = true;
                }
                ui.add_space(3.0);
            }
        });

        // Recording channels ----------------------------------------
        theme::card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Recording");
            ui.add_space(5.0);
            for index in 0..app.state.mixer.recording.len() {
                let changed = {
                    let channel = &mut app.state.mixer.recording[index];
                    let selected = app.state.recording.selected_input == index;
                    let mut row_changed = false;
                    ui.horizontal(|ui| {
                        if chip(ui, if selected { "SEL" } else { "" }, selected, accent, 30.0)
                            .clicked()
                        {
                            app.state.recording.selected_input = index;
                            app.state.mixer.selected_input = index;
                            row_changed = true;
                        }
                        fixed_label(ui, &channel.name, 84.0);
                        row_changed |= theme::cyan_slider(
                            ui,
                            122.0,
                            &mut channel.volume,
                            0.0,
                            100.0,
                            true,
                        )
                        .changed();
                        strong(
                            ui,
                            &format!("{}", channel.volume.round() as i32),
                            10.5,
                            TEXT_MUTED,
                        );
                        row_changed |=
                            theme::toggle_switch(ui, &mut channel.muted, true).changed();
                    });
                    meter(
                        ui,
                        if channel.muted { 0.0 } else { channel.volume / 100.0 },
                        if channel.muted { TEXT_MUTED } else { accent },
                        ui.available_width(),
                    );
                    row_changed
                };
                if changed {
                    dirty = true;
                }
                ui.add_space(3.0);
            }
            ui.add_space(4.0);
            theme::hline(ui, STROKE_SOFT);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                txt(ui, "Balance", 10.5, TEXT_MUTED);
                ui.add_space(6.0);
                txt(ui, "L", 10.0, TEXT_MUTED);
                dirty |= theme::cyan_slider(
                    ui,
                    84.0,
                    &mut app.state.mixer.balance_l,
                    0.0,
                    100.0,
                    true,
                )
                .changed();
                dirty |= theme::cyan_slider(
                    ui,
                    84.0,
                    &mut app.state.mixer.balance_r,
                    0.0,
                    100.0,
                    true,
                )
                .changed();
                txt(ui, "R", 10.0, TEXT_MUTED);
            });
        });
    });

    ui.add_space(8.0);
    card(ui, "Microphone", Some("shared with the Recording page"), |ui| {
        ui.columns(2, |cols| {
            cols[0].set_width(cols[0].available_width());
            dirty |= param_row(
                &mut cols[0],
                "Mic volume",
                &mut app.state.mixer.mic_volume,
                0.0,
                100.0,
                " %",
                true,
            )
            .changed();
            dirty |= param_row(
                &mut cols[0],
                "Mic boost",
                &mut app.state.mixer.mic_boost,
                0.0,
                MIC_BOOST_MAX,
                " dB",
                true,
            )
            .changed();
            dirty |= toggle_row(
                &mut cols[1],
                "Mic mute",
                None,
                &mut app.state.mixer.mic_muted,
                true,
            )
            .changed();
            dirty |= toggle_row(
                &mut cols[1],
                "Monitor",
                None,
                &mut app.state.mixer.monitor,
                true,
            )
            .changed();
            dirty |= toggle_row(&mut cols[1], "AGC", None, &mut app.state.mixer.agc, true).changed();
        });
    });

    if dirty {
        app.mark_dirty();
    }
}

// ---------------------------------------------------------------- Lighting

pub fn lighting_page(app: &mut AuroraApp, ui: &mut Ui) {
    let accent = accent_for_page(Page::Lighting);
    let on = app.state.lighting.on;
    let mut want_on = on;
    let mut toggle = None;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            page_top(
                ui,
                "Lighting",
                "Lighting",
                "Patterns, colours and brightness for the card lighting",
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            toggle = Some(theme::toggle_switch(ui, &mut want_on, true));
            theme::pill(
                ui,
                if on { "LIGHTS ON" } else { "LIGHTS OFF" },
                if on { accent } else { TEXT_MUTED },
            );
        });
    });
    if let Some(response) = toggle {
        if response.changed() {
            app.state.lighting.on = want_on;
            app.mark_dirty();
        }
    }
    header_line(ui);

    let pattern = app.state.lighting.pattern.min(LIGHT_PATTERNS.len() - 1);
    let color = LIGHT_COLORS[app.state.lighting.color.min(LIGHT_COLORS.len() - 1)].1;
    let brightness = app.state.lighting.brightness;
    let speed = app.state.lighting.speed;

    let mut dirty = false;
    ui.columns(2, |cols| {
        theme::card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Preview");
            ui.add_space(4.0);
            lighting_preview(ui, on, pattern, color, brightness, speed, app.now);
        });

        theme::card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            theme::small_caps(ui, "Output");
            ui.add_space(4.0);
            dirty |= param_row(
                ui,
                "Brightness",
                &mut app.state.lighting.brightness,
                0.0,
                100.0,
                " %",
                on,
            )
            .changed();
            dirty |= param_row(
                ui,
                "Speed",
                &mut app.state.lighting.speed,
                0.0,
                100.0,
                " %",
                on,
            )
            .changed();
            dirty |= combo_row(
                ui,
                "Direction",
                "lighting_direction",
                &LIGHT_DIRECTIONS,
                &mut app.state.lighting.direction,
                on,
            );
            dirty |= toggle_row(
                ui,
                "Apply to all zones",
                None,
                &mut app.state.lighting.all_zones,
                true,
            )
            .changed();
            ui.add_space(2.0);
            txt(
                ui,
                "Patterns and directions come from the product's lighting definitions.",
                9.5,
                TEXT_MUTED,
            );
        });
    });

    ui.add_space(8.0);
    card(ui, "Pattern", None, |ui| {
        ui.columns(4, |cols| {
            for (index, name) in LIGHT_PATTERNS.iter().enumerate() {
                let column = &mut cols[index % 4];
                let selected = app.state.lighting.pattern == index;
                if chip(column, name, selected, accent, column.available_width()).clicked() {
                    app.state.lighting.pattern = index;
                    app.mark_dirty();
                }
                column.add_space(4.0);
            }
        });
    });

    ui.add_space(8.0);
    card(ui, "Colour", None, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let mut picked = None;
            for (index, (name, rgb)) in LIGHT_COLORS.iter().enumerate() {
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(72.0, 42.0), Sense::click());
                let swatch = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                let selected = app.state.lighting.color == index;
                ui.painter().circle_filled(
                    Pos2::new(rect.center().x, rect.top() + 13.0),
                    9.0,
                    swatch,
                );
                if selected {
                    ui.painter().circle_stroke(
                        Pos2::new(rect.center().x, rect.top() + 13.0),
                        13.0,
                        Stroke::new(1.5_f32, TEXT),
                    );
                }
                ui.painter().text(
                    Pos2::new(rect.center().x, rect.bottom() - 6.0),
                    Align2::CENTER_CENTER,
                    *name,
                    theme::font(8.5),
                    if selected { TEXT } else { TEXT_MUTED },
                );
                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if response.clicked() {
                    picked = Some(index);
                }
            }
            if let Some(index) = picked {
                app.state.lighting.color = index;
                app.mark_dirty();
            }
        });
    });

    if dirty {
        app.mark_dirty();
        app.state.lighting.pattern = app.state.lighting.pattern.min(LIGHT_PATTERNS.len() - 1);
    }
}

// ---------------------------------------------------------------- Settings

pub fn settings_page(app: &mut AuroraApp, ui: &mut Ui) {
    let accent = accent_for_page(Page::Settings);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            page_top(
                ui,
                "Settings",
                "Settings",
                "Application, startup and build information",
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            theme::pill(ui, "OFFLINE BUILD", accent);
        });
    });
    header_line(ui);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        for (index, name) in ["General", "Startup", "About"].iter().enumerate() {
            if theme::text_tab(ui, name, app.state.settings.tab == index, true).clicked() {
                app.state.settings.tab = index;
            }
        }
    });
    ui.add_space(8.0);

    match app.state.settings.tab {
        0 => {
            let mut dirty = false;
            ui.columns(2, |cols| {
                theme::card_frame().show(&mut cols[0], |ui| {
                    ui.set_width(ui.available_width());
                    theme::small_caps(ui, "Language");
                    ui.add_space(4.0);
                    // Estimated list: the recovered settings struct stores only
                    // an index, with no packaged language table.
                    dirty |= combo_row(
                        ui,
                        "Language",
                        "settings_language",
                        &["English", "System default", "Deutsch", "日本語"],
                        &mut app.state.settings.language,
                        true,
                    );
                    ui.add_space(6.0);
                    theme::small_caps(ui, "Current sound");
                    ui.add_space(4.0);
                    kv(ui, "Profile", &app.state.effects.active_profile.clone());
                    kv(ui, "EQ preset", &app.state.eq.preset.clone());
                    kv(
                        ui,
                        "Output",
                        match app.state.playback.output {
                            OutputMode::Speakers => "Speakers",
                            OutputMode::Headphones => "Headphones",
                        },
                    );
                    kv(
                        ui,
                        "SBX",
                        if app.state.effects.sbx_on { "On" } else { "Bypassed" },
                    );
                    kv(
                        ui,
                        "Direct Mode",
                        if app.state.is_direct() { "On" } else { "Off" },
                    );
                });

                theme::card_frame().show(&mut cols[1], |ui| {
                    ui.set_width(ui.available_width());
                    theme::small_caps(ui, "Maintenance");
                    ui.add_space(6.0);
                    txt(
                        ui,
                        "Restoring defaults resets every page to its factory values. Saved presets and custom profiles are cleared.",
                        10.0,
                        TEXT_DIM,
                    );
                    ui.add_space(8.0);
                    if theme::primary_button(ui, "Restore defaults", true).clicked() {
                        app.confirm_reset = true;
                    }
                    ui.add_space(8.0);
                    theme::hline(ui, STROKE);
                    ui.add_space(6.0);
                    txt(
                        ui,
                        "Settings are saved automatically to your application data folder after each change.",
                        9.5,
                        TEXT_MUTED,
                    );
                });
            });
            if dirty {
                app.mark_dirty();
            }
        }
        1 => {
            let mut dirty = false;
            ui.columns(2, |cols| {
                theme::card_frame().show(&mut cols[0], |ui| {
                    ui.set_width(ui.available_width());
                    theme::small_caps(ui, "At sign-in");
                    ui.add_space(6.0);
                    let mut startup = app.state.settings.startup;
                    if toggle_row(
                        ui,
                        "Run at Windows sign-in",
                        None,
                        &mut startup,
                        true,
                    )
                    .changed()
                    {
                        app.state.settings.startup = startup;
                        dirty = true;
                    }
                    let mut minimized = app.state.settings.start_minimized;
                    if toggle_row(
                        ui,
                        "Start minimised",
                        None,
                        &mut minimized,
                        app.state.settings.startup,
                    )
                    .changed()
                    {
                        app.state.settings.start_minimized = minimized;
                        dirty = true;
                    }
                });

                theme::card_frame().show(&mut cols[1], |ui| {
                    ui.set_width(ui.available_width());
                    theme::small_caps(ui, "Updates");
                    ui.add_space(6.0);
                    txt(
                        ui,
                        "This build is fully offline and makes no network requests.",
                        10.0,
                        TEXT_DIM,
                    );
                    ui.add_space(4.0);
                    kv(ui, "Last check", &app.state.settings.last_check.clone());
                    ui.add_space(6.0);
                    if theme::ghost_button(ui, "Check for updates").clicked() {
                        app.state.settings.last_check = "Checked just now".to_string();
                        app.mark_dirty();
                        app.toast("You are running the latest offline build");
                    }
                });
            });
            if dirty {
                app.mark_dirty();
            }
        }
        _ => {
            ui.columns(2, |cols| {
                theme::card_frame().show(&mut cols[0], |ui| {
                    ui.set_width(ui.available_width());
                    theme::small_caps(ui, "About");
                    ui.add_space(6.0);
                    strong(ui, "Aurora Audio Command", 18.0, TEXT);
                    txt(ui, "Neutral-branded audio control panel", 10.5, TEXT_DIM);
                    ui.add_space(8.0);
                    kv(ui, "Version", env!("CARGO_PKG_VERSION"));
                    kv(ui, "UI engine", "eframe 0.29 / egui");
                    kv(ui, "Fidelity", "Offline replica");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let (dot, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                        theme::status_dot(ui.painter(), dot.center(), GREEN);
                        txt(ui, "No account, network or telemetry", 10.0, TEXT_MUTED);
                    });
                });

                theme::card_frame().show(&mut cols[1], |ui| {
                    ui.set_width(ui.available_width());
                    theme::small_caps(ui, "Pages");
                    ui.add_space(6.0);
                    for page in Page::ALL {
                        let selected = page == app.state.page;
                        ui.horizontal(|ui| {
                            let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                            theme::status_dot(
                                ui.painter(),
                                dot.center(),
                                if selected { accent } else { TEXT_MUTED },
                            );
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(page.title())
                                            .font(theme::font(11.5))
                                            .color(if selected { TEXT } else { TEXT_DIM }),
                                    )
                                    .fill(Color32::TRANSPARENT)
                                    .rounding(Rounding::same(3.0)),
                                )
                                .clicked()
                            {
                                app.set_page(page);
                            }
                        });
                    }
                });
            });
        }
    }
}
