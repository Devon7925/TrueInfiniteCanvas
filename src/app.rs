use ron::Options;
use serde::{Deserialize, Serialize};

use crate::painting::Painting;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Deserialize, Serialize)]
#[derive(Default)]
pub struct TemplateApp {
    // Example stuff:
    painting: Painting,
}



impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return {
                let key = eframe::APP_KEY;
                storage.get_string(key).and_then(|value| {
                    let mut deserializer = ron::de::Deserializer::from_str_with_options(
                        &value,
                        Options::default().without_recursion_limit(),
                    )
                    .unwrap();
                    let deserializer = serde_stacker::Deserializer::new(&mut deserializer);
                    match TemplateApp::deserialize(deserializer) {
                        Ok(value) => Some(value),
                        Err(err) => {
                            // This happens on when we break the format, e.g. when updating egui.
                            log::debug!("Failed to decode RON: {err}");
                            None
                        }
                    }
                })
            }
            .unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let key = eframe::APP_KEY;
        let mut out = Vec::new();
        let mut serializer = ron::ser::Serializer::with_options(
            &mut out,
            None,
            Options::default().without_recursion_limit(),
        )
        .unwrap();
        let serializer = serde_stacker::Serializer::new(&mut serializer);
        match self.serialize(serializer) {
            Ok(_) => storage.set_string(key, String::from_utf8(out).expect("Ron should be utf-8")),
            Err(err) => log::error!("eframe failed to encode data using ron: {}", err),
        }
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            self.painting.ui_control(ui);
            self.painting.ui_content(ui);

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
