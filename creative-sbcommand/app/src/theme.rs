use crate::state::{EQ_FREQS, EQ_MAX_DB, EQ_MIN_DB};
use eframe::egui::{
    self, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Pos2,
    Rect, Response, Rounding, Sense, Shape, Stroke, TextStyle, Ui, Vec2,
};

// ------------------------------------------------------------------ palette

pub const BG_TOP: Color32 = Color32::from_rgb(43, 45, 73);
pub const BG_BOTTOM: Color32 = Color32::from_rgb(36, 38, 63);
pub const NAV_BG: Color32 = Color32::from_rgb(28, 32, 50);
pub const TITLE_BG: Color32 = Color32::from_rgb(22, 26, 40);
pub const BOTTOM_BG: Color32 = Color32::from_rgb(22, 27, 42);
pub const PANEL: Color32 = Color32::from_rgb(50, 53, 82);
pub const PANEL_SOFT: Color32 = Color32::from_rgb(47, 50, 78);
pub const CARD: Color32 = Color32::from_rgb(55, 58, 88);
pub const FIELD: Color32 = Color32::from_rgb(44, 47, 70);
pub const BUTTON: Color32 = Color32::from_rgb(58, 61, 89);
pub const BUTTON_HOVER: Color32 = Color32::from_rgb(69, 73, 104);
pub const BUTTON_ACTIVE: Color32 = Color32::from_rgb(78, 82, 116);
pub const STROKE: Color32 = Color32::from_rgb(72, 76, 108);
pub const STROKE_SOFT: Color32 = Color32::from_rgb(61, 65, 94);
pub const TEXT: Color32 = Color32::from_rgb(241, 243, 250);
pub const TEXT_DIM: Color32 = Color32::from_rgb(186, 191, 211);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(133, 139, 166);
pub const ACCENT: Color32 = Color32::from_rgb(0, 200, 230);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(84, 221, 240);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0, 142, 163);
pub const DANGER: Color32 = Color32::from_rgb(255, 96, 118);
pub const GREEN: Color32 = Color32::from_rgb(88, 222, 162);

pub const CONTENT_LEFT: f32 = 245.0;
pub const TITLE_H: f32 = 35.0;
pub const BOTTOM_H: f32 = 46.0;

// ------------------------------------------------------------------ setup

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let mut add_font = |name: &str, path: &str| -> bool {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.to_owned(), FontData::from_owned(bytes));
            true
        } else {
            false
        }
    };

    let has_barlow = add_font("bahnschrift", "C:/Windows/Fonts/bahnschrift.ttf");
    let has_bold = add_font("segoeuib", "C:/Windows/Fonts/segoeuib.ttf")
        || add_font("segoeuib", "C:/Windows/Fonts/segoeui.ttf");

    let bold_family = fonts
        .families
        .entry(FontFamily::Name("bold".into()))
        .or_default();
    if has_bold {
        bold_family.insert(0, "segoeuib".to_owned());
    }
    // "Ubuntu-Light" is one of egui's bundled fonts; unlike "Ubuntu-Bold" it
    // always exists and keeps the bold family usable on machines without the
    // Windows Segoe UI files.
    bold_family.push("Ubuntu-Light".to_owned());

    if has_barlow {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "bahnschrift".to_owned());
    }

    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(20.0, FontFamily::Name("bold".into()))),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ]
    .into();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(11.0, 6.0);
    style.spacing.slider_width = 150.0;
    style.visuals = dark_visuals();
    ctx.set_style(style);
}

fn dark_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(TEXT);
    v.panel_fill = BG_TOP;
    v.window_fill = Color32::from_rgb(39, 42, 67);
    v.extreme_bg_color = FIELD;
    v.faint_bg_color = CARD;
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0, 200, 230, 70);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.window_rounding = Rounding::same(10.0);
    v.menu_rounding = Rounding::same(8.0);
    v.window_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 8.0),
        blur: 28.0,
        spread: 0.0,
        color: Color32::from_black_alpha(130),
    };
    v.widgets.noninteractive.bg_fill = PANEL_SOFT;
    v.widgets.noninteractive.weak_bg_fill = PANEL_SOFT;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, STROKE_SOFT);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    v.widgets.noninteractive.rounding = Rounding::same(6.0);
    v.widgets.inactive.bg_fill = BUTTON;
    v.widgets.inactive.weak_bg_fill = BUTTON;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.inactive.rounding = Rounding::same(6.0);
    v.widgets.hovered.bg_fill = BUTTON_HOVER;
    v.widgets.hovered.weak_bg_fill = BUTTON_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT_DARK);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.hovered.rounding = Rounding::same(6.0);
    v.widgets.active.bg_fill = BUTTON_ACTIVE;
    v.widgets.active.weak_bg_fill = BUTTON_ACTIVE;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.active.rounding = Rounding::same(6.0);
    v.widgets.open.bg_fill = BUTTON_ACTIVE;
    v.widgets.open.weak_bg_fill = BUTTON_ACTIVE;
    v.widgets.open.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.open.rounding = Rounding::same(6.0);
    v
}

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}
pub fn small_caps(ui: &mut Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .font(font(10.5))
            .color(TEXT_MUTED),
    );
}

// ------------------------------------------------------------------ background

pub fn page_background(ui: &Ui, accent: Color32) {
    let rect = ui.max_rect();
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), BG_TOP);
    mesh.colored_vertex(rect.right_top(), Color32::from_rgb(48, 49, 80));
    mesh.colored_vertex(rect.right_bottom(), BG_BOTTOM);
    mesh.colored_vertex(rect.left_bottom(), Color32::from_rgb(39, 41, 68));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(Shape::mesh(mesh));

    // A soft coloured bloom in the upper right, like the original's lighting wash.
    let center = Pos2::new(rect.right() - 120.0, rect.top() + 40.0);
    for i in (1..=10).rev() {
        let r = 260.0 * (i as f32 / 10.0);
        let a = (7.0 * (1.0 - i as f32 / 12.0)) as u8;
        ui.painter()
            .circle_filled(center, r, Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), a));
    }

    // Thin top rule seen under the title bar in the original.
    ui.painter().line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 200, 230, 70)),
    );
}

pub fn nav_background(ui: &Ui) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, NAV_BG);
    ui.painter().line_segment(
        [rect.right_top(), rect.right_bottom()],
        Stroke::new(1.0_f32, Color32::from_rgb(20, 24, 38)),
    );
}

// ------------------------------------------------------------------ primitives

pub fn hline(ui: &mut Ui, color: Color32) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 1.0), Sense::hover());
    ui.painter().line_segment([rect.left_center(), rect.right_center()], Stroke::new(1.0_f32, color));
}

pub fn gap(ui: &mut Ui, h: f32) {
    ui.add_space(h);
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(PANEL_SOFT)
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::same(14.0))
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(CARD)
        .rounding(Rounding::same(7.0))
        .inner_margin(egui::Margin::same(12.0))
}

pub fn brand_mark(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, Rounding::same(6.0), Color32::from_rgb(25, 30, 45));
    let bar_w = rect.width() * 0.13;
    let gapx = rect.width() * 0.08;
    let start = rect.center().x - (bar_w * 4.0 + gapx * 3.0) * 0.5;
    let heights = [0.35, 0.68, 0.9, 0.52];
    for (i, h) in heights.iter().enumerate() {
        let top = rect.center().y - rect.height() * h * 0.5;
        let bar = Rect::from_min_size(
            Pos2::new(start + i as f32 * (bar_w + gapx), top),
            Vec2::new(bar_w, rect.height() * *h),
        );
        painter.rect_filled(bar, Rounding::same(bar_w * 0.5), ACCENT);
    }
}

pub fn pill(ui: &mut Ui, text: &str, color: Color32) {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(10.5), color);
    let size = Vec2::new(galley.size().x + 16.0, 20.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(10.0), Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 30));
    ui.painter().rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0_f32, color.gamma_multiply(0.55)));
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, color);
}

pub fn status_dot(painter: &egui::Painter, center: Pos2, color: Color32) {
    painter.circle_filled(center, 6.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 45));
    painter.circle_filled(center, 3.0, color);
}

// ------------------------------------------------------------------ buttons

pub fn primary_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let text_color = if enabled { Color32::from_rgb(9, 40, 47) } else { TEXT_MUTED };
    let fill = if enabled { ACCENT } else { BUTTON };
    let btn = egui::Button::new(egui::RichText::new(text).font(bold(12.0)).color(text_color))
        .fill(fill)
        .rounding(Rounding::same(5.0))
        .min_size(Vec2::new(0.0, 28.0));
    ui.add_enabled(enabled, btn)
}

pub fn ghost_button(ui: &mut Ui, text: &str) -> Response {
    let btn = egui::Button::new(egui::RichText::new(text).font(font(12.0)).color(TEXT_DIM))
        .fill(BUTTON)
        .rounding(Rounding::same(5.0))
        .min_size(Vec2::new(0.0, 26.0));
    ui.add(btn)
}

pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, enabled: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let color = if !enabled {
        TEXT_MUTED.gamma_multiply(0.55)
    } else if response.hovered() {
        TEXT
    } else {
        TEXT_DIM
    };
    if response.hovered() && enabled {
        ui.painter().rect_filled(rect, Rounding::same(4.0), BUTTON_HOVER);
    }
    draw_icon(ui.painter(), rect.shrink(size * 0.25), icon, color, 1.35);
    response
}

// ------------------------------------------------------------------ toggle

pub fn toggle_switch(ui: &mut Ui, on: &mut bool, enabled: bool) -> Response {
    let desired = Vec2::new(34.0, 18.0);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());
    let id = response.id;
    let anim = ui.ctx().animate_bool_with_time(id, *on && enabled, 0.12);
    let track = Rect::from_center_size(rect.center(), desired);
    let off = Color32::from_rgb(74, 78, 107);
    let fill = if !enabled {
        off.gamma_multiply(0.7)
    } else if *on {
        ACCENT
    } else {
        off
    };
    ui.painter().rect_filled(track, Rounding::same(9.0), fill);
    let thumb_x = egui::lerp((track.left() + 9.0)..=(track.right() - 9.0), anim);
    let thumb = Pos2::new(thumb_x, track.center().y);
    ui.painter().circle_filled(thumb, 6.5, if enabled { Color32::WHITE } else { TEXT_MUTED });
    if response.clicked() && enabled {
        *on = !*on;
        response.mark_changed();
    }
    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

// ------------------------------------------------------------------ segmented

pub fn segmented(ui: &mut Ui, labels: &[&str], selected: &mut usize, enabled: bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (i, label) in labels.iter().enumerate() {
            let active = *selected == i;
            let text_color = if !enabled {
                TEXT_MUTED.gamma_multiply(0.7)
            } else if active {
                TEXT
            } else {
                TEXT_DIM
            };
            let btn = egui::Button::new(egui::RichText::new(*label).font(font(11.5)).color(text_color))
                .fill(if active { BUTTON_ACTIVE } else { Color32::TRANSPARENT })
                .rounding(Rounding::same(4.0))
                .min_size(Vec2::new(0.0, 24.0));
            if ui.add_enabled(enabled, btn).clicked() {
                *selected = i;
            }
        }
    });
}

pub fn text_tab(ui: &mut Ui, label: &str, selected: bool, enabled: bool) -> Response {
    let color = if !enabled {
        TEXT_MUTED.gamma_multiply(0.7)
    } else if selected {
        ACCENT
    } else {
        TEXT_DIM
    };
    let btn = egui::Button::new(egui::RichText::new(label).font(bold(13.0)).color(color))
        .fill(Color32::TRANSPARENT)
        .rounding(Rounding::same(3.0))
        .min_size(Vec2::new(0.0, 26.0));
    let response = ui.add_enabled(enabled, btn);
    if selected {
        let r = response.rect;
        ui.painter().line_segment(
            [Pos2::new(r.left() + 2.0, r.bottom()), Pos2::new(r.right() - 2.0, r.bottom())],
            Stroke::new(2.0_f32, ACCENT),
        );
    }
    response
}

// ------------------------------------------------------------------ combo

pub fn combo(
    ui: &mut Ui,
    id: &str,
    current: &str,
    options: &[&str],
    width: f32,
    enabled: bool,
) -> Option<usize> {
    let mut picked = None;
    ui.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(egui::RichText::new(current).font(font(12.0)).color(TEXT))
            .width(width)
            .show_ui(ui, |ui| {
                for (i, option) in options.iter().enumerate() {
                    if ui
                        .selectable_label(option.eq_ignore_ascii_case(current), *option)
                        .clicked()
                    {
                        picked = Some(i);
                    }
                }
            });
    });
    picked
}

// ------------------------------------------------------------------ sliders

pub fn cyan_slider(
    ui: &mut Ui,
    width: f32,
    value: &mut f32,
    min: f32,
    max: f32,
    enabled: bool,
) -> Response {
    let height = 18.0;
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
    let track = Rect::from_center_size(rect.center(), Vec2::new(width, 3.0));
    let base = Color32::from_rgb(76, 80, 108);
    ui.painter().rect_filled(track, Rounding::same(2.0), base);
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let x = egui::lerp((track.left())..=(track.right()), t);
    if t > 0.0 {
        let fill = Rect::from_min_max(
            Pos2::new(track.left(), track.top()),
            Pos2::new(x, track.bottom()),
        );
        ui.painter().rect_filled(fill, Rounding::same(2.0), if enabled { ACCENT } else { TEXT_MUTED });
    }
    ui.painter().circle_filled(
        Pos2::new(x, rect.center().y),
        6.5,
        if enabled { Color32::WHITE } else { TEXT_MUTED },
    );
    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if enabled && (response.dragged() || response.clicked()) {
        if let Some(pos) = ui.ctx().pointer_interact_pos() {
            let nt = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            *value = (min + nt * (max - min)).clamp(min, max);
            if max - min > 0.0 {
                *value = (*value * 2.0).round() / 2.0;
            }
            response.mark_changed();
        }
    }
    response
}

// ------------------------------------------------------------------ knob

pub fn knob(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    size: f32,
    value: &mut f32,
    min: f32,
    max: f32,
    label: &str,
    unit: &str,
    enabled: bool,
    accent: Color32,
) -> Response {
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(size, size), Sense::click_and_drag());
    let id = ui.make_persistent_id(id);
    let center = rect.center();
    let radius = size * 0.44;
    let start = 135.0_f32.to_radians();
    let sweep = 270.0_f32.to_radians();
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);

    let col = if enabled { accent } else { accent.gamma_multiply(0.35) };
    let track_col = if enabled { Color32::from_rgb(84, 88, 118) } else { Color32::from_rgb(68, 71, 95) };

    if response.hovered() && enabled {
        ui.painter()
            .circle_filled(center, radius + 7.0, Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 14));
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }

    // track arc
    let steps = 96;
    for i in 0..steps {
        let a0 = start + sweep * (i as f32 / steps as f32);
        let a1 = start + sweep * ((i + 1) as f32 / steps as f32);
        let p0 = center + Vec2::new(a0.cos(), a0.sin()) * radius;
        let p1 = center + Vec2::new(a1.cos(), a1.sin()) * radius;
        ui.painter().line_segment([p0, p1], Stroke::new(5.0_f32, track_col));
    }
    // active arc
    let active_steps = (steps as f32 * t).round() as usize;
    for i in 0..active_steps {
        let f0 = i as f32 / steps as f32;
        let f1 = (i + 1) as f32 / steps as f32;
        let a0 = start + sweep * f0;
        let a1 = start + sweep * f1;
        let p0 = center + Vec2::new(a0.cos(), a0.sin()) * radius;
        let p1 = center + Vec2::new(a1.cos(), a1.sin()) * radius;
        let c = lerp_color(col, ACCENT_SOFT, f0);
        ui.painter().line_segment([p0, p1], Stroke::new(5.0_f32, c));
    }
    // end marker
    let end_angle = start + sweep * t;
    let end = center + Vec2::new(end_angle.cos(), end_angle.sin()) * radius;
    ui.painter().circle_filled(end, 3.2, Color32::WHITE);

    // value text
    let text = format!("{}", value.round() as i32);
    ui.painter().text(
        center + Vec2::new(0.0, 4.0),
        Align2::CENTER_CENTER,
        text,
        bold(size * 0.28),
        if enabled { TEXT } else { TEXT_MUTED },
    );
    if !unit.is_empty() {
        ui.painter().text(
            center + Vec2::new(0.0, size * 0.19),
            Align2::CENTER_CENTER,
            unit,
            font(size * 0.09),
            TEXT_MUTED,
        );
    }
    ui.painter().text(
        Pos2::new(center.x, rect.bottom() + 11.0),
        Align2::CENTER_CENTER,
        label,
        font(12.5),
        if enabled { TEXT_DIM } else { TEXT_MUTED },
    );

    if enabled && (response.dragged() || response.clicked()) {
        let delta = response.drag_delta();
        if response.dragged() {
            *value = (*value - delta.y * (max - min) / 160.0).clamp(min, max);
            *value = (*value * 2.0).round() / 2.0;
            response.mark_changed();
        } else if response.clicked() {
            // A click on the arc jumps to the clicked angle.
            if let Some(pos) = ui.ctx().pointer_interact_pos() {
                let v = pos - center;
                let mut a = v.y.atan2(v.x);
                let mut rel = a - start;
                while rel < 0.0 {
                    rel += std::f32::consts::TAU;
                }
                if rel <= sweep {
                    *value = (min + (rel / sweep) * (max - min)).clamp(min, max);
                    response.mark_changed();
                }
                a = 0.0;
                let _ = a;
            }
        }
    }
    let _ = id;
    response
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

// ------------------------------------------------------------------ eq fader

pub fn eq_fader(
    ui: &mut Ui,
    freq: u32,
    value: &mut f32,
    enabled: bool,
    selected: bool,
) -> Response {
    let width = 43.0;
    let height = 126.0;
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
    let track_x = rect.center().x;
    let top = rect.top() + 6.0;
    let bottom = rect.top() + 96.0;
    let track = Rect::from_min_max(
        Pos2::new(track_x - 2.0, top),
        Pos2::new(track_x + 2.0, bottom),
    );
    ui.painter().rect_filled(track, Rounding::same(2.0), Color32::from_rgb(78, 82, 110));
    let mid = (top + bottom) * 0.5;
    ui.painter().line_segment(
        [Pos2::new(track_x - 9.0, mid), Pos2::new(track_x + 9.0, mid)],
        Stroke::new(1.0_f32, Color32::from_rgb(110, 115, 145)),
    );

    let t = ((*value - EQ_MIN_DB) / (EQ_MAX_DB - EQ_MIN_DB)).clamp(0.0, 1.0);
    let y = egui::lerp(bottom..=top, t);
    let fill = Rect::from_min_max(
        Pos2::new(track_x - 2.0, y.min(mid)),
        Pos2::new(track_x + 2.0, y.max(mid)),
    );
    ui.painter().rect_filled(fill, Rounding::same(2.0), if enabled { ACCENT } else { TEXT_MUTED });
    let handle = Rect::from_center_size(Pos2::new(track_x, y), Vec2::new(22.0, 9.0));
    ui.painter().rect_filled(handle, Rounding::same(4.0), if enabled { Color32::WHITE } else { TEXT_MUTED });
    if selected {
        ui.painter()
            .rect_stroke(handle.expand(2.0), Rounding::same(5.0), Stroke::new(1.0_f32, ACCENT));
    }

    let label = if freq >= 1000 {
        format!("{}k", freq / 1000)
    } else {
        format!("{}", freq)
    };
    ui.painter().text(
        Pos2::new(track_x, rect.bottom() - 10.0),
        Align2::CENTER_CENTER,
        label,
        font(10.5),
        if selected { TEXT } else { TEXT_MUTED },
    );
    ui.painter().text(
        Pos2::new(track_x, rect.top() + 0.0),
        Align2::CENTER_CENTER,
        format!("{:+.1}", *value),
        font(9.5),
        if enabled { TEXT_DIM } else { TEXT_MUTED },
    );

    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    if enabled && (response.dragged() || response.clicked()) {
        if let Some(pos) = ui.ctx().pointer_interact_pos() {
            let nt = ((bottom - pos.y) / (bottom - top)).clamp(0.0, 1.0);
            *value = EQ_MIN_DB + nt * (EQ_MAX_DB - EQ_MIN_DB);
            *value = (*value * 10.0).round() / 10.0;
            response.mark_changed();
        }
    }
    response
}

// ------------------------------------------------------------------ icons

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Icon {
    Sbx,
    Eq,
    Playback,
    Recording,
    Scout,
    Encoder,
    Mixer,
    Lighting,
    Settings,
    User,
    Speaker,
    Headphones,
    Mic,
    Plus,
    Dots,
    ChevronLeft,
    ChevronRight,
    Power,
    Info,
    Close,
    Minimize,
    Mute,
    Gear,
    Check,
    Reset,
    Play,
}

pub fn draw_icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32, width: f32) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let stroke = Stroke::new(width, color);
    let line = |a: Pos2, b: Pos2| painter.line_segment([a, b], stroke);
    match icon {
        Icon::Sbx => {
            let h = s * 0.42;
            let w = s * 0.7;
            let pts = vec![
                Pos2::new(c.x - w * 0.5, c.y + h * 0.5),
                Pos2::new(c.x - w * 0.5, c.y - h * 0.15),
                Pos2::new(c.x, c.y - h * 0.5),
                Pos2::new(c.x + w * 0.5, c.y - h * 0.15),
                Pos2::new(c.x + w * 0.5, c.y + h * 0.5),
            ];
            painter.add(Shape::line(pts, stroke));
            line(
                Pos2::new(c.x - w * 0.18, c.y + h * 0.5),
                Pos2::new(c.x - w * 0.18, c.y + h * 0.1),
            );
            line(
                Pos2::new(c.x + w * 0.18, c.y + h * 0.5),
                Pos2::new(c.x + w * 0.18, c.y + h * 0.1),
            );
        }
        Icon::Eq => {
            for (i, x) in [-0.26_f32, 0.0, 0.26].iter().enumerate() {
                let xx = c.x + s * x;
                line(Pos2::new(xx, c.y - s * 0.34), Pos2::new(xx, c.y + s * 0.34));
                let y = c.y + s * [0.10, -0.12, 0.16][i];
                painter.circle_filled(Pos2::new(xx, y), s * 0.055, color);
            }
        }
        Icon::Playback => {
            let w = s * 0.34;
            let h = s * 0.28;
            let poly = vec![
                Pos2::new(c.x - w, c.y - h * 0.55),
                Pos2::new(c.x - w * 0.45, c.y - h * 0.55),
                Pos2::new(c.x, c.y - h),
                Pos2::new(c.x, c.y + h),
                Pos2::new(c.x - w * 0.45, c.y + h * 0.55),
                Pos2::new(c.x - w, c.y + h * 0.55),
            ];
            painter.add(Shape::convex_polygon(poly, Color32::TRANSPARENT, stroke));
            painter.circle_stroke(Pos2::new(c.x + s * 0.22, c.y), s * 0.18, stroke);
        }
        Icon::Recording | Icon::Mic => {
            painter.rect_stroke(
                Rect::from_center_size(Pos2::new(c.x, c.y - s * 0.1), Vec2::new(s * 0.22, s * 0.42)),
                Rounding::same(s * 0.11),
                stroke,
            );
            let arc = vec![
                Pos2::new(c.x - s * 0.2, c.y - s * 0.02),
                Pos2::new(c.x - s * 0.15, c.y + s * 0.2),
                Pos2::new(c.x, c.y + s * 0.27),
                Pos2::new(c.x + s * 0.15, c.y + s * 0.2),
                Pos2::new(c.x + s * 0.2, c.y - s * 0.02),
            ];
            painter.add(Shape::line(arc, stroke));
            line(Pos2::new(c.x, c.y + s * 0.27), Pos2::new(c.x, c.y + s * 0.36));
        }
        Icon::Scout => {
            painter.circle_stroke(c, s * 0.3, stroke);
            painter.circle_stroke(c, s * 0.08, Stroke::new(width * 0.85, color));
            for (dx, dy) in [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)] {
                let a = Pos2::new(c.x + dx * s * 0.36, c.y + dy * s * 0.36);
                let b = Pos2::new(c.x + dx * s * 0.46, c.y + dy * s * 0.46);
                line(a, b);
            }
        }
        Icon::Encoder => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(s * 0.72, s * 0.52)),
                Rounding::same(3.0),
                stroke,
            );
            for i in 0..3 {
                let x = c.x - s * 0.22 + i as f32 * s * 0.22;
                line(Pos2::new(x, c.y - s * 0.26), Pos2::new(x, c.y + s * 0.26));
            }
        }
        Icon::Mixer => {
            for (i, y) in [-0.24_f32, 0.0, 0.24].iter().enumerate() {
                let yy = c.y + s * y;
                line(Pos2::new(c.x - s * 0.34, yy), Pos2::new(c.x + s * 0.34, yy));
                let x = c.x + s * [-0.13, 0.16, -0.02][i];
                painter.rect_filled(
                    Rect::from_center_size(Pos2::new(x, yy), Vec2::new(s * 0.09, s * 0.16)),
                    Rounding::same(2.0),
                    color,
                );
            }
        }
        Icon::Lighting => {
            painter.circle_stroke(Pos2::new(c.x, c.y - s * 0.1), s * 0.22, stroke);
            line(Pos2::new(c.x - s * 0.09, c.y + s * 0.17), Pos2::new(c.x + s * 0.09, c.y + s * 0.17));
            line(Pos2::new(c.x - s * 0.07, c.y + s * 0.25), Pos2::new(c.x + s * 0.07, c.y + s * 0.25));
            for i in 0..4 {
                let a = -2.4 + i as f32 * 0.5;
                let p = Pos2::new(c.x + a.cos() * s * 0.4, c.y - s * 0.1 + a.sin() * s * 0.4);
                let q = Pos2::new(c.x + a.cos() * s * 0.5, c.y - s * 0.1 + a.sin() * s * 0.5);
                line(p, q);
            }
        }
        Icon::Settings | Icon::Gear => {
            painter.circle_stroke(c, s * 0.22, stroke);
            painter.circle_stroke(c, s * 0.07, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let p = Pos2::new(c.x + a.cos() * s * 0.32, c.y + a.sin() * s * 0.32);
                let q = Pos2::new(c.x + a.cos() * s * 0.44, c.y + a.sin() * s * 0.44);
                line(p, q);
            }
        }
        Icon::User => {
            painter.circle_stroke(Pos2::new(c.x, c.y - s * 0.15), s * 0.16, stroke);
            let arc = vec![
                Pos2::new(c.x - s * 0.3, c.y + s * 0.36),
                Pos2::new(c.x - s * 0.24, c.y + s * 0.15),
                Pos2::new(c.x, c.y + s * 0.06),
                Pos2::new(c.x + s * 0.24, c.y + s * 0.15),
                Pos2::new(c.x + s * 0.3, c.y + s * 0.36),
            ];
            painter.add(Shape::line(arc, stroke));
        }
        Icon::Speaker => {
            let poly = vec![
                Pos2::new(c.x - s * 0.36, c.y - s * 0.14),
                Pos2::new(c.x - s * 0.18, c.y - s * 0.14),
                Pos2::new(c.x + s * 0.03, c.y - s * 0.31),
                Pos2::new(c.x + s * 0.03, c.y + s * 0.31),
                Pos2::new(c.x - s * 0.18, c.y + s * 0.14),
                Pos2::new(c.x - s * 0.36, c.y + s * 0.14),
            ];
            painter.add(Shape::convex_polygon(poly, color, Stroke::NONE));
            painter.circle_stroke(Pos2::new(c.x + s * 0.17, c.y), s * 0.16, stroke);
            painter.circle_stroke(Pos2::new(c.x + s * 0.17, c.y), s * 0.29, stroke);
        }
        Icon::Headphones => {
            let arc = vec![
                Pos2::new(c.x - s * 0.34, c.y + s * 0.08),
                Pos2::new(c.x - s * 0.3, c.y - s * 0.24),
                Pos2::new(c.x, c.y - s * 0.38),
                Pos2::new(c.x + s * 0.3, c.y - s * 0.24),
                Pos2::new(c.x + s * 0.34, c.y + s * 0.08),
            ];
            painter.add(Shape::line(arc, stroke));
            painter.rect_stroke(
                Rect::from_center_size(Pos2::new(c.x - s * 0.32, c.y + s * 0.16), Vec2::new(s * 0.16, s * 0.28)),
                Rounding::same(2.0),
                stroke,
            );
            painter.rect_stroke(
                Rect::from_center_size(Pos2::new(c.x + s * 0.32, c.y + s * 0.16), Vec2::new(s * 0.16, s * 0.28)),
                Rounding::same(2.0),
                stroke,
            );
        }
        Icon::Plus => {
            line(Pos2::new(c.x - s * 0.28, c.y), Pos2::new(c.x + s * 0.28, c.y));
            line(Pos2::new(c.x, c.y - s * 0.28), Pos2::new(c.x, c.y + s * 0.28));
        }
        Icon::Dots => {
            for dx in [-0.26_f32, 0.0, 0.26] {
                painter.circle_filled(Pos2::new(c.x + s * dx, c.y), s * 0.055, color);
            }
        }
        Icon::ChevronLeft => {
            line(Pos2::new(c.x + s * 0.12, c.y - s * 0.28), Pos2::new(c.x - s * 0.12, c.y));
            line(Pos2::new(c.x - s * 0.12, c.y), Pos2::new(c.x + s * 0.12, c.y + s * 0.28));
        }
        Icon::ChevronRight => {
            line(Pos2::new(c.x - s * 0.12, c.y - s * 0.28), Pos2::new(c.x + s * 0.12, c.y));
            line(Pos2::new(c.x + s * 0.12, c.y), Pos2::new(c.x - s * 0.12, c.y + s * 0.28));
        }
        Icon::Power => {
            painter.circle_stroke(c, s * 0.28, stroke);
            line(Pos2::new(c.x, c.y - s * 0.42), Pos2::new(c.x, c.y - s * 0.06));
        }
        Icon::Info => {
            painter.circle_stroke(c, s * 0.34, stroke);
            painter.circle_filled(Pos2::new(c.x, c.y - s * 0.15), s * 0.035, color);
            line(Pos2::new(c.x, c.y - s * 0.02), Pos2::new(c.x, c.y + s * 0.2));
        }
        Icon::Close => {
            line(Pos2::new(c.x - s * 0.28, c.y - s * 0.28), Pos2::new(c.x + s * 0.28, c.y + s * 0.28));
            line(Pos2::new(c.x + s * 0.28, c.y - s * 0.28), Pos2::new(c.x - s * 0.28, c.y + s * 0.28));
        }
        Icon::Minimize => {
            line(Pos2::new(c.x - s * 0.28, c.y + s * 0.18), Pos2::new(c.x + s * 0.28, c.y + s * 0.18));
        }
        Icon::Mute => {
            let poly = vec![
                Pos2::new(c.x - s * 0.34, c.y - s * 0.12),
                Pos2::new(c.x - s * 0.18, c.y - s * 0.12),
                Pos2::new(c.x + s * 0.02, c.y - s * 0.27),
                Pos2::new(c.x + s * 0.02, c.y + s * 0.27),
                Pos2::new(c.x - s * 0.18, c.y + s * 0.12),
                Pos2::new(c.x - s * 0.34, c.y + s * 0.12),
            ];
            painter.add(Shape::convex_polygon(poly, color, Stroke::NONE));
            line(Pos2::new(c.x + s * 0.16, c.y - s * 0.12), Pos2::new(c.x + s * 0.36, c.y + s * 0.12));
            line(Pos2::new(c.x + s * 0.36, c.y - s * 0.12), Pos2::new(c.x + s * 0.16, c.y + s * 0.12));
        }
        Icon::Check => {
            line(Pos2::new(c.x - s * 0.28, c.y), Pos2::new(c.x - s * 0.06, c.y + s * 0.22));
            line(Pos2::new(c.x - s * 0.06, c.y + s * 0.22), Pos2::new(c.x + s * 0.3, c.y - s * 0.24));
        }
        Icon::Reset => {
            painter.circle_stroke(c, s * 0.3, stroke);
            line(Pos2::new(c.x + s * 0.3, c.y - s * 0.3), Pos2::new(c.x + s * 0.3, c.y - s * 0.05));
            line(Pos2::new(c.x + s * 0.3, c.y - s * 0.05), Pos2::new(c.x + s * 0.06, c.y - s * 0.05));
        }
        Icon::Play => {
            let poly = vec![
                Pos2::new(c.x - s * 0.2, c.y - s * 0.28),
                Pos2::new(c.x + s * 0.3, c.y),
                Pos2::new(c.x - s * 0.2, c.y + s * 0.28),
            ];
            painter.add(Shape::convex_polygon(poly, color, Stroke::NONE));
        }
    }
}

// ------------------------------------------------------------------ profile art

pub fn paint_profile_art(
    painter: &egui::Painter,
    rect: Rect,
    art: crate::state::ArtKind,
    accent: [u8; 3],
    time: f64,
    selected: bool,
) {
    use crate::state::ArtKind;
    let ac = Color32::from_rgb(accent[0], accent[1], accent[2]);
    let dark = Color32::from_rgb(
        (accent[0] as f32 * 0.18) as u8,
        (accent[1] as f32 * 0.18) as u8,
        (accent[2] as f32 * 0.22) as u8,
    );
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), dark);
    mesh.colored_vertex(rect.right_top(), ac.gamma_multiply(0.55));
    mesh.colored_vertex(rect.right_bottom(), Color32::from_rgb(24, 26, 44));
    mesh.colored_vertex(rect.left_bottom(), Color32::from_rgb(18, 20, 34));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
    painter.rect_stroke(
        rect,
        Rounding::same(3.0),
        Stroke::new(if selected { 2.0_f32 } else { 1.0_f32 }, if selected { ACCENT } else { ac.gamma_multiply(0.35) }),
    );

    let c = rect.center();
    let s = rect.width().min(rect.height());
    match art {
        ArtKind::Cinema => {
            // a stylised screen and light cones
            let screen = Rect::from_center_size(Pos2::new(c.x, c.y - s * 0.08), Vec2::new(s * 0.62, s * 0.34));
            painter.rect_filled(screen, Rounding::same(2.0), Color32::from_rgb(246, 239, 222));
            painter.rect_filled(screen.shrink(2.0), Rounding::same(2.0), Color32::from_rgb(34, 42, 60));
            for i in 0..5 {
                let off = (i as f32 - 2.0) * s * 0.09;
                let p = Pos2::new(c.x + off, c.y + s * 0.24);
                painter.circle_filled(p, s * 0.045, Color32::from_rgb(120, 76, 55));
            }
            painter.add(Shape::line(
                vec![
                    Pos2::new(c.x - s * 0.32, c.y + s * 0.36),
                    Pos2::new(c.x, c.y + s * 0.46),
                    Pos2::new(c.x + s * 0.32, c.y + s * 0.36),
                ],
                Stroke::new(1.0_f32, ac.gamma_multiply(0.5)),
            ));
        }
        ArtKind::Gaming => {
            for i in 0..3 {
                let r = s * (0.18 + i as f32 * 0.08);
                painter.circle_stroke(c, r, Stroke::new(1.0_f32, ac.gamma_multiply(0.55)));
            }
            for a in [0.0_f32, 1.57, 3.14, 4.71] {
                let p1 = Pos2::new(c.x + a.cos() * s * 0.12, c.y + a.sin() * s * 0.12);
                let p2 = Pos2::new(c.x + a.cos() * s * 0.34, c.y + a.sin() * s * 0.34);
                painter.line_segment([p1, p2], Stroke::new(1.3_f32, ac));
            }
            painter.circle_filled(c, s * 0.04, Color32::WHITE);
        }
        ArtKind::Concert => {
            // stage lights and speaker circles
            for i in 0..5 {
                let x = rect.left() + rect.width() * (i as f32 + 0.5) / 5.0;
                let p = Pos2::new(x, rect.top() + s * 0.2 + (i as f32 * 0.7).sin() * 8.0);
                painter.line_segment([p, Pos2::new(x - s * 0.2, rect.bottom())], Stroke::new(9.0_f32, ac.gamma_multiply(0.10)));
                painter.circle_filled(p, s * 0.035, ac);
            }
            painter.circle_stroke(Pos2::new(c.x, c.y + s * 0.12), s * 0.19, Stroke::new(1.5_f32, ac));
            painter.circle_stroke(Pos2::new(c.x, c.y + s * 0.12), s * 0.10, Stroke::new(1.0_f32, ac.gamma_multiply(0.7)));
        }
        ArtKind::Night => {
            painter.circle_filled(Pos2::new(c.x + s * 0.08, c.y - s * 0.1), s * 0.2, Color32::from_rgb(230, 232, 245));
            painter.circle_filled(Pos2::new(c.x + s * 0.18, c.y - s * 0.16), s * 0.18, dark);
            for i in 0..10 {
                let x = rect.left() + rect.width() * ((i * 37 % 100) as f32 / 100.0);
                let y = rect.top() + rect.height() * ((i * 61 % 80 + 5) as f32 / 100.0);
                painter.circle_filled(Pos2::new(x, y), 1.0, Color32::from_white_alpha(120));
            }
        }
        ArtKind::Fps => {
            let r = s * 0.27;
            painter.circle_stroke(c, r, Stroke::new(2.0_f32, ac));
            painter.circle_stroke(c, s * 0.07, Stroke::new(1.0_f32, ac.gamma_multiply(0.7)));
            for a in [0.0_f32, 1.57, 3.14, 4.71] {
                let p1 = Pos2::new(c.x + a.cos() * (r + 4.0), c.y + a.sin() * (r + 4.0));
                let p2 = Pos2::new(c.x + a.cos() * (r + s * 0.15), c.y + a.sin() * (r + s * 0.15));
                painter.line_segment([p1, p2], Stroke::new(2.0_f32, ac));
            }
            painter.circle_filled(c, 2.0, Color32::WHITE);
        }
        ArtKind::Rpg => {
            // a shield silhouette
            let pts = vec![
                Pos2::new(c.x, c.y - s * 0.32),
                Pos2::new(c.x + s * 0.25, c.y - s * 0.18),
                Pos2::new(c.x + s * 0.2, c.y + s * 0.15),
                Pos2::new(c.x, c.y + s * 0.36),
                Pos2::new(c.x - s * 0.2, c.y + s * 0.15),
                Pos2::new(c.x - s * 0.25, c.y - s * 0.18),
            ];
            painter.add(Shape::convex_polygon(pts, ac.gamma_multiply(0.18), Stroke::new(1.5_f32, ac)));
            painter.add(Shape::line(
                vec![
                    Pos2::new(c.x, c.y - s * 0.2),
                    Pos2::new(c.x, c.y + s * 0.2),
                ],
                Stroke::new(1.0_f32, ac.gamma_multiply(0.6)),
            ));
        }
        ArtKind::Racing => {
            for i in 0..5 {
                let t = i as f32 / 5.0;
                let y = rect.top() + rect.height() * (0.2 + t * 0.7);
                let w = rect.width() * (0.18 + t * 0.22);
                painter.line_segment(
                    [Pos2::new(c.x - w * 0.5, y), Pos2::new(c.x + w * 0.5, y)],
                    Stroke::new(1.0_f32, ac.gamma_multiply(0.35 + t * 0.1)),
                );
            }
            painter.circle_stroke(Pos2::new(c.x, c.y + s * 0.1), s * 0.12, Stroke::new(2.0_f32, ac));
            painter.circle_filled(Pos2::new(c.x, c.y + s * 0.1), s * 0.03, ac);
        }
        ArtKind::Strategy => {
            for i in 0..4 {
                let y = rect.top() + rect.height() * (0.24 + i as f32 * 0.16);
                let x0 = rect.left() + rect.width() * 0.16;
                let x1 = rect.left() + rect.width() * (0.55 + (i % 2) as f32 * 0.24);
                painter.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], Stroke::new(1.0_f32, ac.gamma_multiply(0.55)));
                painter.circle_filled(Pos2::new(x0, y), 2.0, ac);
                painter.circle_filled(Pos2::new(x1, y), 2.0, ac);
            }
            painter.circle_stroke(c, s * 0.08, Stroke::new(1.0_f32, ac));
        }
        ArtKind::Personal => {
            let phase = (time * 0.8) as f32;
            for i in 0..3 {
                let r = s * (0.12 + i as f32 * 0.08);
                let a = 0.4 - i as f32 * 0.1;
                painter.circle_stroke(
                    c + Vec2::new((phase + i as f32).sin() * 3.0, 0.0),
                    r,
                    Stroke::new(1.2_f32, ac.gamma_multiply(a)),
                );
            }
            painter.circle_filled(c, s * 0.035, Color32::WHITE);
        }
    }
}

// ------------------------------------------------------------------ eq curve

/// Graph insets shared by the curve, its nodes and the click mapping.
pub const EQ_GRAPH_PAD_L: f32 = 30.0;
pub const EQ_GRAPH_PAD_R: f32 = 16.0;
pub const EQ_GRAPH_PAD_T: f32 = 14.0;
pub const EQ_GRAPH_PAD_B: f32 = 20.0;
/// Number of log-spaced samples in a computed response curve.
pub const EQ_RESPONSE_POINTS: usize = 160;

/// Horizontal position of a band centre on the graph's log-frequency axis.
pub fn eq_band_x(rect: Rect, freq: f32) -> f32 {
    let x0 = rect.left() + EQ_GRAPH_PAD_L;
    let x1 = rect.right() - EQ_GRAPH_PAD_R;
    let lo = EQ_FREQS[0] as f32;
    let hi = EQ_FREQS[9] as f32;
    let t = (freq.max(lo) / lo).log2() / (hi / lo).log2();
    x0 + (x1 - x0) * t.clamp(0.0, 1.0)
}

pub fn eq_curve_points(rect: Rect, response: &[f32]) -> Vec<Pos2> {
    let x0 = rect.left() + EQ_GRAPH_PAD_L;
    let x1 = rect.right() - EQ_GRAPH_PAD_R;
    let y0 = rect.top() + EQ_GRAPH_PAD_T;
    let y1 = rect.bottom() - EQ_GRAPH_PAD_B;
    let y_for = |db: f32| {
        let t = ((db - EQ_MIN_DB) / (EQ_MAX_DB - EQ_MIN_DB)).clamp(0.0, 1.0);
        y1 - t * (y1 - y0)
    };
    let last = response.len().saturating_sub(1).max(1);
    response
        .iter()
        .enumerate()
        .map(|(i, db)| Pos2::new(x0 + (x1 - x0) * i as f32 / last as f32, y_for(*db)))
        .collect()
}

pub fn draw_eq_graph(
    painter: &egui::Painter,
    rect: Rect,
    bands: &[f32; 10],
    response: &[f32],
    selected: Option<usize>,
    enabled: bool,
) {
    painter.rect_filled(rect, Rounding::same(6.0), Color32::from_rgb(40, 43, 68));
    painter.rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, STROKE_SOFT));
    let x0 = rect.left() + EQ_GRAPH_PAD_L;
    let x1 = rect.right() - EQ_GRAPH_PAD_R;
    let y0 = rect.top() + EQ_GRAPH_PAD_T;
    let y1 = rect.bottom() - EQ_GRAPH_PAD_B;
    // horizontal grid
    for db in [-12.0_f32, -6.0, 0.0, 6.0, 12.0] {
        let t = (db - EQ_MIN_DB) / (EQ_MAX_DB - EQ_MIN_DB);
        let y = y1 - t * (y1 - y0);
        let col = if db == 0.0 {
            Color32::from_rgb(89, 94, 126)
        } else {
            Color32::from_rgb(61, 65, 94)
        };
        painter.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], Stroke::new(1.0_f32, col));
        painter.text(
            Pos2::new(x0 - 8.0, y),
            Align2::RIGHT_CENTER,
            format!("{:+}", db as i32),
            font(9.0),
            TEXT_MUTED,
        );
    }
    // vertical grid on the band centres
    for i in 0..10 {
        let x = eq_band_x(rect, EQ_FREQS[i] as f32);
        painter.line_segment(
            [Pos2::new(x, y0), Pos2::new(x, y1)],
            Stroke::new(1.0_f32, Color32::from_rgb(55, 59, 87)),
        );
    }
    let points = eq_curve_points(rect, response);
    if points.len() > 2 {
        let fill = if enabled {
            Color32::from_rgba_unmultiplied(0, 200, 230, 35)
        } else {
            Color32::from_rgba_unmultiplied(160, 165, 190, 18)
        };
        // One convex quad per sample pair: robust for any curve shape and
        // cheaper than tessellating one concave path.
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a.x == b.x {
                continue;
            }
            painter.add(Shape::convex_polygon(
                vec![
                    Pos2::new(a.x, y1),
                    a,
                    b,
                    Pos2::new(b.x, y1),
                ],
                fill,
                Stroke::NONE,
            ));
        }
        painter.add(Shape::line(
            points.clone(),
            Stroke::new(2.0_f32, if enabled { ACCENT } else { TEXT_MUTED }),
        ));
    }
    // nodes
    for i in 0..10 {
        let x = eq_band_x(rect, EQ_FREQS[i] as f32);
        let t = (bands[i] - EQ_MIN_DB) / (EQ_MAX_DB - EQ_MIN_DB);
        let y = y1 - t * (y1 - y0);
        let active = selected == Some(i);
        painter.circle_filled(
            Pos2::new(x, y),
            if active { 5.0 } else { 3.4 },
            if enabled { Color32::WHITE } else { TEXT_MUTED },
        );
        if active {
            painter.circle_stroke(Pos2::new(x, y), 8.0, Stroke::new(1.5_f32, ACCENT));
        }
    }
    // frequency labels
    for i in 0..10 {
        let x = eq_band_x(rect, EQ_FREQS[i] as f32);
        let label = if EQ_FREQS[i] >= 1000 {
            format!("{}k", EQ_FREQS[i] / 1000)
        } else {
            format!("{}", EQ_FREQS[i])
        };
        painter.text(
            Pos2::new(x, y1 + 12.0),
            Align2::CENTER_CENTER,
            label,
            font(8.5),
            TEXT_MUTED,
        );
    }
}
