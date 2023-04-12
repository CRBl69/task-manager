use egui::DragValue;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub update_interval_ms: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            update_interval_ms: 1000,
        }
    }
}

impl Settings {
    pub fn settings_view(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Update interval (in milliseconds)");
                ui.add(DragValue::new(&mut self.update_interval_ms)
                    .speed(1.0)
                    .suffix("ms")
                );
            })
        });
    }
}
