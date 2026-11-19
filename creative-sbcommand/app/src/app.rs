use crate::pages;
use crate::state::{AppState, Page, PROFILES};
use crate::theme::{self, *};
use aurora_audio_command::eq_core;
use eframe::egui::{
    self, Align2, Color32, Context, FontId, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2,
    ViewportCommand,
};

/// Default output path for the equalizer's WAV render: `<stem>_eq.wav` next to
/// the input file.
pub fn default_eq_output(input: &str) -> String {
    let path = std::path::Path::new(input.trim());
    match (path.parent(), path.file_stem()) {
        (Some(dir), Some(stem)) => dir
            .join(format!("{}_eq.wav", stem.to_string_lossy()))
            .to_string_lossy()
            .into_owned(),
        _ => String::new(),
    }
}

pub struct ScreenshotJob {
    pub path: String,
    pub frame: u64,
    pub requested: bool,
    pub saved: bool,
}

pub struct AuroraApp {
    pub state: AppState,
    pub screenshot: Option<ScreenshotJob>,
    pub toast: Option<(String, f64)>,
    pub now: f64,
    pub dirty: bool,
    pub last_save: f64,
    pub eq_selected: Option<usize>,
    pub profile_scroll: usize,
    pub show_save_eq: bool,
    pub save_eq_name: String,
    pub show_save_profile: bool,
    pub save_profile_name: String,
    pub confirm_reset: bool,
    pub confirm_reset_eq: bool,
    pub show_eq_info: bool,
    pub show_direct_info: bool,
    pub show_eq_render: bool,
    pub eq_wav_in: String,
    pub eq_wav_out: String,
    pub eq_render_status: Option<(String, bool)>,
    pub page_accent: Color32,
}

impl AuroraApp {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<String>) -> Self {
        theme::install(&cc.egui_ctx);
        let (mut state, screenshot) = if screenshot.is_some() {
            (AppState::default(), screenshot.map(|path| ScreenshotJob {
                path,
                frame: 0,
                requested: false,
                saved: false,
            }))
        } else {
            (AppState::load(), None)
        };
        if screenshot.is_some() {
            if let Ok(preset) = std::env::var("AURORA_EQ_PRESET") {
                state.eq.apply_preset(&preset, state.playback.output);
            }
            if let Ok(mode) = std::env::var("AURORA_OUTPUT") {
                state.playback.output = if mode.eq_ignore_ascii_case("headphones") {
                    crate::state::OutputMode::Headphones
                } else {
                    crate::state::OutputMode::Speakers
                };
                let preset = state.eq.preset.clone();
                state.eq.apply_preset(&preset, state.playback.output);
            }
            if let Ok(page) = std::env::var("AURORA_PAGE") {
                state.page = match page.to_ascii_lowercase().as_str() {
                    "equalizer" | "eq" => Page::Equalizer,
                    "playback" => Page::Playback,
                    "recording" => Page::Recording,
                    "scout" => Page::Scout,
                    "encoder" => Page::Encoder,
                    "mixer" => Page::Mixer,
                    "lighting" => Page::Lighting,
                    "settings" => Page::Settings,
                    _ => Page::Sbx,
                };
            }
        }
        let mut app = Self {
            state,
            screenshot,
            toast: None,
            now: 0.0,
            dirty: false,
            last_save: 0.0,
            eq_selected: None,
            profile_scroll: 0,
            show_save_eq: false,
            save_eq_name: String::new(),
            show_save_profile: false,
            save_profile_name: String::new(),
            confirm_reset: false,
            confirm_reset_eq: false,
            show_eq_info: false,
            show_direct_info: false,
            show_eq_render: false,
            eq_wav_in: String::new(),
            eq_wav_out: String::new(),
            eq_render_status: None,
            page_accent: ACCENT,
        };
        if app.screenshot.is_some() {
            let page = app.state.page;
            app.set_page(page);
            if std::env::var("AURORA_EQ_RENDER").is_ok() {
                app.eq_wav_in = "C:\\audio\\input.wav".to_string();
                app.eq_wav_out = default_eq_output(&app.eq_wav_in);
                app.show_eq_render = true;
            }
        }
        app
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), self.now));
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn set_page(&mut self, page: Page) {
        self.state.page = page;
        self.page_accent = match page {
            Page::Sbx => Color32::from_rgb(0, 200, 230),
            Page::Equalizer => Color32::from_rgb(84, 221, 240),
            Page::Playback => Color32::from_rgb(76, 170, 255),
            Page::Recording => Color32::from_rgb(255, 130, 160),
            Page::Scout => Color32::from_rgb(0, 210, 190),
            Page::Encoder => Color32::from_rgb(130, 170, 255),
            Page::Mixer => Color32::from_rgb(100, 200, 255),
            Page::Lighting => Color32::from_rgb(180, 120, 255),
            Page::Settings => Color32::from_rgb(150, 160, 200),
        };
    }

    pub fn apply_profile(&mut self, name: &str) {
        self.state.effects.apply_profile(name);
        self.state.eq.apply_preset(name, self.state.playback.output);
        self.toast(format!("{} profile applied", name));
        self.mark_dirty();
    }

    fn title_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("title_bar")
            .exact_height(theme::TITLE_H)
            .frame(egui::Frame::none().fill(theme::TITLE_BG))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 0.0, theme::TITLE_BG);
                ui.painter().line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(1.0_f32, Color32::from_rgb(15, 19, 31)),
                );
                ui.horizontal_centered(|ui| {
                    ui.add_space(16.0);
                    let (mark_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                    theme::brand_mark(ui.painter(), mark_rect);
                    ui.add_space(9.0);
                    ui.label(
                        egui::RichText::new("Aurora Audio Command")
                            .font(theme::font(13.0))
                            .color(Color32::from_rgb(203, 207, 222)),
                    );
                    ui.add_space(18.0);
                    ui.label(
                        egui::RichText::new("X5 AUDIO CARD")
                            .font(theme::font(9.5))
                            .color(TEXT_MUTED),
                    );

                    let avail = ui.available_width();
                    let _ = avail;
                    let drag_w = (ui.available_width() - 100.0).max(40.0);
                    let (drag_rect, drag) =
                        ui.allocate_exact_size(Vec2::new(drag_w, theme::TITLE_H), Sense::click_and_drag());
                    if drag.drag_started() {
                        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                    let _ = drag_rect;

                    let min = ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("\u{2013}").font(theme::font(15.0)).color(TEXT_DIM),
                            )
                            .fill(Color32::TRANSPARENT)
                            .rounding(Rounding::same(3.0))
                            .min_size(Vec2::splat(28.0)),
                        )
                        .on_hover_text("Minimize");
                    if min.clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                    let close = ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("\u{00d7}").font(theme::font(17.0)).color(TEXT_DIM),
                            )
                            .fill(Color32::TRANSPARENT)
                            .rounding(Rounding::same(3.0))
                            .min_size(Vec2::splat(28.0)),
                        )
                        .on_hover_text("Close");
                    if close.clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                    ui.add_space(6.0);
                });
            });
    }

    fn nav(&mut self, ctx: &Context) {
        egui::SidePanel::left("nav")
            .exact_width(theme::CONTENT_LEFT)
            .resizable(false)
            .frame(egui::Frame::none().fill(theme::NAV_BG))
            .show(ctx, |ui| {
                theme::nav_background(ui);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(22.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("AURORA X5")
                                .font(theme::bold(17.0))
                                .color(ACCENT),
                        );
                        ui.add_space(1.0);
                        ui.horizontal(|ui| {
                            theme::status_dot(ui.painter(), ui.cursor().left_center() + Vec2::new(4.0, 0.0), GREEN);
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new("CONNECTED  \u{00b7}  PCIe")
                                    .font(theme::font(9.5))
                                    .color(TEXT_MUTED),
                            );
                        });
                    });
                });
                ui.add_space(10.0);
                theme::hline(ui, Color32::from_rgb(43, 47, 69));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(22.0);
                    theme::small_caps(ui, "Sound Experience");
                });
                ui.add_space(4.0);
                self.nav_item(ui, Page::Sbx, Icon::Sbx, "SBX Profile");
                self.nav_item(ui, Page::Equalizer, Icon::Eq, "Equalizer");
                ui.add_space(4.0);
                theme::hline(ui, Color32::from_rgb(43, 47, 69));
                ui.add_space(4.0);
                self.nav_item(ui, Page::Playback, Icon::Playback, "Playback");
                self.nav_item(ui, Page::Recording, Icon::Recording, "Recording");
                self.nav_item(ui, Page::Scout, Icon::Scout, "Scout Mode");
                self.nav_item(ui, Page::Encoder, Icon::Encoder, "Encoder");
                self.nav_item(ui, Page::Mixer, Icon::Mixer, "Mixer");

                let remaining = ui.available_height();
                if remaining > 160.0 {
                    ui.add_space(remaining - 158.0);
                } else {
                    ui.add_space(8.0);
                }
                theme::hline(ui, Color32::from_rgb(43, 47, 69));
                ui.add_space(4.0);
                self.nav_item(ui, Page::Lighting, Icon::Lighting, "Lighting");
                self.sign_in_item(ui);
                self.nav_item(ui, Page::Settings, Icon::Settings, "Settings");
                ui.add_space(8.0);
            });
    }

    fn nav_item(&mut self, ui: &mut Ui, page: Page, icon: Icon, label: &str) {
        let selected = self.state.page == page;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
        if selected {
            ui.painter().rect_filled(
                Rect::from_min_size(Pos2::new(rect.left() + 6.0, rect.top() + 1.0), Vec2::new(rect.width() - 12.0, rect.height() - 2.0)),
                Rounding::same(5.0),
                Color32::from_rgb(38, 43, 65),
            );
            ui.painter().rect_filled(
                Rect::from_min_size(Pos2::new(rect.left(), rect.top() + 4.0), Vec2::new(3.0, rect.height() - 8.0)),
                Rounding::same(1.5),
                ACCENT,
            );
        } else if response.hovered() {
            ui.painter().rect_filled(
                Rect::from_min_size(Pos2::new(rect.left() + 6.0, rect.top() + 1.0), Vec2::new(rect.width() - 12.0, rect.height() - 2.0)),
                Rounding::same(5.0),
                Color32::from_rgb(34, 39, 59),
            );
        }
        let icon_color = if selected { ACCENT } else { TEXT_MUTED };
        theme::draw_icon(
            ui.painter(),
            Rect::from_center_size(Pos2::new(rect.left() + 27.0, rect.center().y), Vec2::splat(18.0)),
            icon,
            icon_color,
            1.35,
        );
        ui.painter().text(
            Pos2::new(rect.left() + 50.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            theme::font(12.5),
            if selected { ACCENT } else { TEXT_DIM },
        );
        if response.clicked() {
            self.set_page(page);
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    fn sign_in_item(&mut self, ui: &mut Ui) {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
        theme::draw_icon(
            ui.painter(),
            Rect::from_center_size(Pos2::new(rect.left() + 27.0, rect.center().y), Vec2::splat(18.0)),
            Icon::User,
            TEXT_MUTED.gamma_multiply(0.8),
            1.25,
        );
        ui.painter().text(
            Pos2::new(rect.left() + 50.0, rect.center().y),
            Align2::LEFT_CENTER,
            "Sign In",
            theme::font(12.5),
            TEXT_MUTED.gamma_multiply(0.8),
        );
        ui.painter().text(
            Pos2::new(rect.left() + 50.0, rect.center().y + 11.0),
            Align2::LEFT_CENTER,
            "Online only",
            theme::font(8.5),
            TEXT_MUTED.gamma_multiply(0.6),
        );
        if response.clicked() {
            self.toast("Account sign-in is not part of this offline version");
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    fn bottom_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("bottom_bar")
            .exact_height(theme::BOTTOM_H)
            .frame(egui::Frame::none().fill(theme::BOTTOM_BG))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 0.0, theme::BOTTOM_BG);
                ui.painter().line_segment(
                    [rect.left_top(), rect.right_top()],
                    Stroke::new(1.0_f32, Color32::from_rgb(14, 18, 29)),
                );
                ui.horizontal_centered(|ui| {
                    ui.add_space(14.0);
                    let (avatar, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
                    ui.painter()
                        .circle_filled(avatar.center(), 15.0, Color32::from_rgb(38, 43, 64));
                    ui.painter()
                        .circle_stroke(avatar.center(), 15.0, Stroke::new(1.0_f32, Color32::from_rgb(54, 60, 86)));
                    theme::draw_icon(
                        ui.painter(),
                        Rect::from_center_size(avatar.center(), Vec2::splat(17.0)),
                        Icon::Sbx,
                        ACCENT,
                        1.3,
                    );
                    ui.add_space(8.0);
                    let mode = if self.state.playback.direct {
                        "DIRECT"
                    } else if self.state.scout.on {
                        "SCOUT"
                    } else {
                        "SBX"
                    };
                    ui.label(
                        egui::RichText::new(mode)
                            .font(theme::bold(18.0))
                            .color(TEXT),
                    );
                    ui.label(
                        egui::RichText::new("|")
                            .font(theme::font(15.0))
                            .color(TEXT_MUTED),
                    );
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(self.state.effects.active_profile.clone())
                                    .font(theme::font(16.0))
                                    .color(TEXT_DIM),
                            )
                            .fill(Color32::TRANSPARENT)
                            .rounding(Rounding::same(3.0)),
                        )
                        .on_hover_text("Sound profile")
                        .clicked()
                    {
                        self.set_page(Page::Sbx);
                    }

                    let right_w = 310.0;
                    let space = (ui.available_width() - right_w).max(0.0);
                    ui.add_space(space);

                    if theme::icon_button(ui, Icon::Mic, 24.0, true)
                        .on_hover_text("Recording")
                        .clicked()
                    {
                        self.set_page(Page::Recording);
                    }
                    let mute_icon = if self.state.mixer.master_muted { Icon::Mute } else { Icon::Speaker };
                    if theme::icon_button(ui, mute_icon, 24.0, true)
                        .on_hover_text(if self.state.mixer.master_muted { "Unmute" } else { "Mute" })
                        .clicked()
                    {
                        self.state.mixer.master_muted = !self.state.mixer.master_muted;
                        self.mark_dirty();
                    }
                    let w = 170.0;
                    ui.scope(|ui| {
                        ui.add_space(4.0);
                        if theme::cyan_slider(ui, w, &mut self.state.mixer.master, 0.0, 100.0, true).changed() {
                            self.state.mixer.master_muted = false;
                            self.mark_dirty();
                        }
                    });
                    ui.add_space(10.0);
                });
            });
    }

    fn handle_screenshot(&mut self, ctx: &Context) {
        let mut request = false;
        let mut done = false;
        if let Some(job) = &mut self.screenshot {
            job.frame += 1;
            ctx.request_repaint();
            if job.frame >= 120 && !job.requested {
                ctx.send_viewport_cmd(ViewportCommand::Screenshot);
                job.requested = true;
            }
            if job.saved {
                done = true;
            }
            request = job.requested;
        }
        if request {
            let events = ctx.input(|i| i.events.clone());
            for event in events {
                if let egui::Event::Screenshot { image, .. } = event {
                    if let Some(job) = &mut self.screenshot {
                        let size = image.size;
                        let mut bytes = Vec::with_capacity(size[0] * size[1] * 4);
                        for px in &image.pixels {
                            let [r, g, b, a] = px.to_array();
                            bytes.extend_from_slice(&[r, g, b, a]);
                        }
                        let saved = image::save_buffer_with_format(
                            &job.path,
                            &bytes,
                            size[0] as u32,
                            size[1] as u32,
                            image::ColorType::Rgba8,
                            image::ImageFormat::Png,
                        );
                        if saved.is_ok() {
                            job.saved = true;
                        } else if let Err(err) = saved {
                            eprintln!("screenshot save failed: {err}");
                            job.saved = true;
                        }
                    }
                }
            }
        }
        if done {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn draw_modals(&mut self, ctx: &Context) {
        if self.show_save_eq {
            let mut open = true;
            egui::Window::new("Save preset")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(360.0, 150.0))
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Name this equalizer preset")
                            .font(theme::font(12.5))
                            .color(TEXT_DIM),
                    );
                    ui.add_space(8.0);
                    let edit = egui::TextEdit::singleline(&mut self.save_eq_name)
                        .hint_text("My preset")
                        .desired_width(f32::INFINITY);
                    ui.add(edit);
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if theme::primary_button(ui, "Save preset", !self.save_eq_name.trim().is_empty()).clicked()
                        {
                            let name = self.save_eq_name.trim().to_string();
                            let bands = self.state.eq.bands;
                            self.state.eq.custom.retain(|p| p.name != name);
                            self.state.eq.custom.push(crate::state::CustomEqPreset { name: name.clone(), bands });
                            self.state.eq.preset = name.clone();
                            self.toast(format!("Preset \"{name}\" saved"));
                            self.show_save_eq = false;
                            self.save_eq_name.clear();
                            self.mark_dirty();
                        }
                        if theme::ghost_button(ui, "Cancel").clicked() {
                            self.show_save_eq = false;
                        }
                    });
                });
            if !open {
                self.show_save_eq = false;
            }
        }

        if self.show_save_profile {
            let mut open = true;
            egui::Window::new("Save profile")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(360.0, 150.0))
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Save the current sound as a profile")
                            .font(theme::font(12.5))
                            .color(TEXT_DIM),
                    );
                    ui.add_space(8.0);
                    let edit = egui::TextEdit::singleline(&mut self.save_profile_name)
                        .hint_text("My profile")
                        .desired_width(f32::INFINITY);
                    ui.add(edit);
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if theme::primary_button(ui, "Save profile", !self.save_profile_name.trim().is_empty()).clicked()
                        {
                            let name = self.save_profile_name.trim().to_string();
                            let mut effects = self.state.effects.clone();
                            effects.active_profile = name.clone();
                            self.state.custom_profiles.push((name.clone(), effects));
                            self.state.effects.active_profile = name.clone();
                            self.toast(format!("Profile \"{name}\" saved"));
                            self.show_save_profile = false;
                            self.save_profile_name.clear();
                            self.mark_dirty();
                        }
                        if theme::ghost_button(ui, "Cancel").clicked() {
                            self.show_save_profile = false;
                        }
                    });
                });
            if !open {
                self.show_save_profile = false;
            }
        }

        if self.confirm_reset {
            let mut open = true;
            egui::Window::new("Restore defaults")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(390.0, 150.0))
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Reset all sound settings to their factory values? This cannot be undone.")
                            .font(theme::font(12.0))
                            .color(TEXT_DIM),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if theme::primary_button(ui, "Restore defaults", true).clicked() {
                            let page = self.state.page;
                            self.state = AppState::default();
                            self.state.page = page;
                            self.toast("Factory defaults restored");
                            self.confirm_reset = false;
                            self.mark_dirty();
                        }
                        if theme::ghost_button(ui, "Cancel").clicked() {
                            self.confirm_reset = false;
                        }
                    });
                });
            if !open {
                self.confirm_reset = false;
            }
        }

        if self.confirm_reset_eq {
            let mut open = true;
            egui::Window::new("Reset equalizer")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(360.0, 140.0))
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Set all ten bands back to 0 dB?")
                            .font(theme::font(12.0))
                            .color(TEXT_DIM),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if theme::primary_button(ui, "Reset curve", true).clicked() {
                            self.state.eq.bands = [0.0; 10];
                            self.state.eq.preamp = 0.0;
                            self.state.eq.preset = "Flat".into();
                            self.toast("Equalizer reset to flat");
                            self.confirm_reset_eq = false;
                            self.mark_dirty();
                        }
                        if theme::ghost_button(ui, "Cancel").clicked() {
                            self.confirm_reset_eq = false;
                        }
                    });
                });
            if !open {
                self.confirm_reset_eq = false;
            }
        }

        if self.show_eq_render {
            let mut open = true;
            egui::Window::new("Render EQ to WAV")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(470.0, 250.0))
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(
                            "Applies the current ten bands and preamp with the same kernel the graph draws. Output is a float32 WAV; an existing file is never overwritten.",
                        )
                        .font(theme::font(11.0))
                        .color(TEXT_DIM),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Input").font(theme::font(11.0)).color(TEXT_MUTED),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.eq_wav_in)
                                .hint_text("C:\\audio\\input.wav")
                                .desired_width(f32::INFINITY),
                        );
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Output").font(theme::font(11.0)).color(TEXT_MUTED),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.eq_wav_out)
                                .hint_text("C:\\audio\\input_eq.wav")
                                .desired_width(f32::INFINITY),
                        );
                    });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let ready = !self.eq_wav_in.trim().is_empty();
                        if theme::primary_button(ui, "Render", ready).clicked() {
                            let input = std::path::PathBuf::from(self.eq_wav_in.trim());
                            let output = if self.eq_wav_out.trim().is_empty() {
                                default_eq_output(&self.eq_wav_in)
                            } else {
                                self.eq_wav_out.trim().to_string()
                            };
                            let mut gains = [0.0f64; 10];
                            for (slot, value) in gains.iter_mut().zip(self.state.eq.bands.iter()) {
                                *slot = *value as f64;
                            }
                            let curve = eq_core::Curve::from_bands(&gains, self.state.eq.preamp as f64);
                            match eq_core::render_wav(
                                &input,
                                std::path::Path::new(&output),
                                &curve,
                                0.0,
                            ) {
                                Ok(()) => {
                                    self.eq_render_status =
                                        Some((format!("Wrote {output}"), true));
                                    self.toast("EQ render finished");
                                }
                                Err(err) => {
                                    self.eq_render_status = Some((err.to_string(), false))
                                }
                            }
                        }
                        if theme::ghost_button(ui, "Cancel").clicked() {
                            self.show_eq_render = false;
                        }
                    });
                    if let Some((text, ok)) = &self.eq_render_status {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(text)
                                .font(theme::font(10.5))
                                .color(if *ok { GREEN } else { Color32::from_rgb(255, 128, 138) }),
                        );
                    }
                });
            if !open {
                self.show_eq_render = false;
            }
        }

        // toast
        if let Some((text, started)) = self.toast.clone() {
            let age = self.now - started;
            if age > 3.0 {
                self.toast = None;
            } else {
                let alpha = if age > 2.4 { ((3.0 - age) / 0.6).clamp(0.0, 1.0) } else { 1.0 };
                let screen = ctx.screen_rect();
                let pos = Pos2::new(screen.center().x - 130.0, screen.bottom() - 86.0);
                let rect = Rect::from_min_size(pos, Vec2::new(260.0, 30.0));
                let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("toast"));
                let painter = ctx.layer_painter(layer);
                painter.rect_filled(
                    rect,
                    Rounding::same(6.0),
                    Color32::from_rgba_unmultiplied(20, 24, 37, (225.0 * alpha) as u8),
                );
                painter.rect_stroke(
                    rect,
                    Rounding::same(6.0),
                    Stroke::new(1.0_f32, ACCENT.gamma_multiply((0.35 * alpha) as f32)),
                );
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    text,
                    theme::font(11.5),
                    Color32::from_rgba_unmultiplied(235, 238, 246, (245.0 * alpha) as u8),
                );
            }
        }
    }
}

impl eframe::App for AuroraApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.now = ctx.input(|i| i.time);
        self.handle_screenshot(ctx);

        let saved_visuals = ctx.style().visuals.clone();
        let mut visuals = saved_visuals.clone();
        visuals.override_text_color = Some(TEXT);
        visuals.panel_fill = BG_TOP;
        ctx.set_visuals(visuals);

        self.title_bar(ctx);
        self.nav(ctx);
        self.bottom_bar(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                theme::page_background(ui, self.page_accent);
                let content_height = (ui.available_height() - 14.0).max(120.0);
                ui.add_space(14.0);
                ui.horizontal_top(|ui| {
                    ui.add_space(22.0);
                    let content_width = (ui.available_width() - 22.0).max(200.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(content_width, content_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                .show(ui, |ui| {
                                    match self.state.page {
                                        Page::Sbx => pages::sbx_page(self, ui),
                                        Page::Equalizer => pages::equalizer_page(self, ui),
                                        Page::Playback => pages::playback_page(self, ui),
                                        Page::Recording => pages::recording_page(self, ui),
                                        Page::Scout => pages::scout_page(self, ui),
                                        Page::Encoder => pages::encoder_page(self, ui),
                                        Page::Mixer => pages::mixer_page(self, ui),
                                        Page::Lighting => pages::lighting_page(self, ui),
                                        Page::Settings => pages::settings_page(self, ui),
                                    }
                                });
                        },
                    );
                });
            });

        self.draw_modals(ctx);

        if self.dirty && self.now - self.last_save > 0.45 && self.screenshot.is_none() {
            self.state.save();
            self.dirty = false;
            self.last_save = self.now;
        }
        if self.screenshot.is_none() {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
        let _ = ctx.style().visuals.clone();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.screenshot.is_none() {
            self.state.save();
        }
    }
}

pub fn page_header(ui: &mut Ui, title: &str, subtitle: Option<&str>) {
    ui.label(
        egui::RichText::new(title)
            .font(theme::bold(22.0))
            .color(TEXT),
    );
    if let Some(sub) = subtitle {
        ui.add_space(1.0);
        ui.label(
            egui::RichText::new(sub)
                .font(theme::font(11.5))
                .color(TEXT_DIM),
        );
    }
}

pub fn header_line(ui: &mut Ui) {
    ui.add_space(8.0);
    theme::hline(ui, Color32::from_rgb(56, 60, 89));
    ui.add_space(10.0);
}

pub fn small_note(ui: &mut Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .font(theme::font(10.5))
            .color(TEXT_MUTED),
    );
}

pub fn breadcrumb(ui: &mut Ui, parts: &[&str]) {
    ui.horizontal(|ui| {
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                ui.label(
                    egui::RichText::new("/")
                        .font(theme::font(11.0))
                        .color(TEXT_MUTED),
                );
            }
            ui.label(
                egui::RichText::new(*part)
                    .font(theme::font(11.0))
                    .color(if i + 1 == parts.len() { TEXT_DIM } else { TEXT_MUTED }),
            );
        }
    });
}

pub fn profile_def(name: &str) -> &'static crate::state::ProfileDef {
    PROFILES
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .unwrap_or(&PROFILES[1])
}

pub fn icon_label(ui: &mut Ui, icon: Icon, text: &str, color: Color32) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
        theme::draw_icon(ui.painter(), r, icon, color, 1.2);
        ui.label(egui::RichText::new(text).font(theme::font(11.0)).color(color));
    });
}

pub fn set_width(ui: &mut Ui, width: f32) {
    ui.set_width(width);
}

pub fn vertical_divider(ui: &mut Ui, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, height), Sense::hover());
    ui.painter()
        .line_segment([rect.center_top(), rect.center_bottom()], Stroke::new(1.0_f32, STROKE_SOFT));
}

pub fn big_number(ui: &mut Ui, value: &str, suffix: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(value)
                .font(theme::bold(26.0))
                .color(TEXT),
        );
        ui.label(
            egui::RichText::new(suffix)
                .font(theme::font(11.0))
                .color(TEXT_MUTED),
        );
    });
}

pub fn formatted_value(value: f32) -> String {
    if (value.fract()).abs() < 0.05 {
        format!("{}", value.round() as i32)
    } else {
        format!("{:.1}", value)
    }
}

pub fn accent_for_page(page: Page) -> Color32 {
    match page {
        Page::Sbx => ACCENT,
        Page::Equalizer => Color32::from_rgb(84, 221, 240),
        Page::Playback => Color32::from_rgb(76, 170, 255),
        Page::Recording => Color32::from_rgb(255, 130, 160),
        Page::Scout => Color32::from_rgb(0, 210, 190),
        Page::Encoder => Color32::from_rgb(130, 170, 255),
        Page::Mixer => Color32::from_rgb(100, 200, 255),
        Page::Lighting => Color32::from_rgb(180, 120, 255),
        Page::Settings => Color32::from_rgb(150, 160, 200),
    }
}

pub fn font_id(size: f32) -> FontId {
    theme::font(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_output_defaults_next_to_the_input() {
        assert_eq!(default_eq_output("C:\\audio\\take.wav"), "C:\\audio\\take_eq.wav");
        assert_eq!(default_eq_output("take.wav"), "take_eq.wav");
        assert_eq!(default_eq_output("  C:\\audio\\take.flac  "), "C:\\audio\\take_eq.wav");
        assert_eq!(default_eq_output(""), "");
    }

    /// The equalizer page's render action: current f32 bands + preamp go
    /// through the shared kernel into a float32 WAV, and never overwrite.
    #[test]
    fn render_uses_current_bands_and_refuses_overwrite() {
        use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("eq-render-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("in.wav");
        let spec = WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        {
            let mut writer = WavWriter::create(&input, spec).unwrap();
            for i in 0..4800 {
                let phase = i as f64 * std::f64::consts::TAU * 1000.0 / 48000.0;
                writer.write_sample((phase.sin() * 8000.0) as i16).unwrap();
            }
            writer.finalize().unwrap();
        }
        let mut gains = [0.0f64; 10];
        gains[5] = 6.0;
        let curve = eq_core::Curve::from_bands(&gains, 0.0);
        let output = dir.join("out.wav");
        eq_core::render_wav(&input, &output, &curve, 0.0).unwrap();

        let mut reader = WavReader::open(&output).unwrap();
        let out_spec = reader.spec();
        assert_eq!(out_spec.bits_per_sample, 32);
        assert_eq!(out_spec.sample_format, SampleFormat::Float);
        assert_eq!(out_spec.sample_rate, 48000);
        assert_eq!(out_spec.channels, 1);
        let samples: Vec<f32> = reader.samples::<f32>().map(|s| s.unwrap()).collect();
        assert_eq!(samples.len(), 4800);
        let peak = samples.iter().fold(0f32, |acc, s| acc.max(s.abs()));
        let ratio = peak / (8000.0 / 32768.0);
        assert!((ratio - 1.995).abs() < 0.1, "1 kHz band gain ratio {ratio}");
        assert!(eq_core::render_wav(&input, &output, &curve, 0.0).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
