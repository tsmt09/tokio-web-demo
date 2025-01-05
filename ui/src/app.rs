use poll_promise::Promise;
use serde::{Deserialize, Serialize};
use std::iter::repeat_with;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TokioWebDemoUi {
    // Example stuff:
    label: String,
    selected_function: DemoFunctions,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,
    #[serde(skip)] // This how you opt-out of serialization of a field
    promise: Option<Promise<ehttp::Result<ehttp::Response>>>,
    #[serde(skip)] // This how you opt-out of serialization of a field
    sysinfo: Option<serde_json::Value>,
}
#[derive(PartialEq, Serialize, Deserialize)]
enum DemoFunctions {
    None,
    Blockers,
    Channel,
    CpuLoadGen,
    RedisKeys,
    Sleeper,
}

impl Default for TokioWebDemoUi {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            promise: None,
            selected_function: DemoFunctions::None,
            sysinfo: None,
        }
    }
}

impl TokioWebDemoUi {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TokioWebDemoUi {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
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
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            powered_by_egui_and_eframe(ui);
            egui::warn_if_debug_build(ui);
        });

        egui::SidePanel::left("function menu").show(ctx, |ui| {
            ui.selectable_value(&mut self.selected_function, DemoFunctions::Sleeper, "Sleep");
            ui.selectable_value(
                &mut self.selected_function,
                DemoFunctions::RedisKeys,
                "Redis Keys",
            );
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::TopBottomPanel::bottom("info")
                .resizable(true)
                .default_height(250.0)
                .show(ctx, |ui| {
                    ui.heading("tokio web demo");
                    if let Some(sysinfo) = &self.sysinfo {
                        ui.label(sysinfo["cpu"]["brand"].as_str().unwrap_or("does not work"));
                    } else {
                        if self.promise.is_none() {
                            let (sender, promise) = poll_promise::Promise::new();
                            self.promise = Some(promise);
                            ehttp::fetch(ehttp::Request::get("/api/sysinfo"), move |resp| {
                                sender.send(resp);
                            });
                        }
                    }

                    if let Some(promise) = &mut self.promise {
                        if let Some(result) = promise.ready() {
                            match result {
                                Ok(response) => {
                                    let Some(text) = response.text() else {
                                        log::error!("response does not contain a text");
                                        return;
                                    };
                                    let Ok(value) = serde_json::from_str(text) else {
                                        log::error!("cannot parse to json: {}", text);
                                        return;
                                    };
                                    self.sysinfo = Some(value);
                                }
                                Err(error) => {
                                    log::error!("Error in request: {}", error);
                                }
                            }
                        } else {
                            ui.spinner();
                        }
                    }
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
