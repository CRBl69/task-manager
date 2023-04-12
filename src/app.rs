use std::sync::{Arc, Mutex};

use egui::{Button, Stroke, Color32};
use serde::{Serialize, Deserialize};
use sysinfo::{System, SystemExt};

use crate::{process_list::ProcessListState, settings::Settings, graphs::GraphsState};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct TaskManager {
    settings: Arc<Mutex<Settings>>,

    #[serde(skip)]
    system: Arc<Mutex<System>>,

    #[serde(skip)]
    view: View,
}

pub enum View {
    Processes(ProcessListState),
    Graphs(GraphsState),
    Settings,
}

impl Default for View {
    fn default() -> Self {
        Self::Processes(ProcessListState::default())
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self {
            settings: Arc::new(Mutex::new(Settings::default())),
            system: Arc::new(Mutex::new(sysinfo::System::new_all())),
            view: View::Processes(ProcessListState::default()),
        }
    }
}

impl TaskManager {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let tm = TaskManager::default();

        std::thread::spawn({
            let system = tm.system.clone();
            let settings = tm.settings.clone();
            move || {
                loop {
                    let time = {
                        let settings = settings.lock().unwrap();
                        settings.update_interval_ms
                    };
                    std::thread::sleep(std::time::Duration::from_millis(time as u64));
                    system.lock().unwrap().refresh_all();
                }
            }
        });

        tm
    }
}

impl eframe::App for TaskManager {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.top_panel(ui, frame);
        });

        match &mut self.view {
            View::Processes(state) => state.process_list_view(ctx, frame, self.system.clone()),
            View::Graphs(state) => state.graphs_view(ctx, frame, self.system.clone()),
            View::Settings => self.settings.lock().unwrap().settings_view(ctx, frame),
        }
    }
}

impl TaskManager {
    fn top_panel(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Settings").clicked() {
                    self.view = View::Settings;
                }
                if ui.button("Quit").clicked() {
                    frame.close();
                }
            });
            ui.menu_button("Views", |ui| {
                let mut processes_btn = Button::new("Processes");
                let mut graphs_btn = Button::new("Graphs");
                match self.view {
                    View::Processes(_) => {
                        processes_btn = processes_btn.stroke(Stroke::new(2.0, Color32::DARK_GRAY));
                    },
                    View::Graphs(_) => {
                        graphs_btn = graphs_btn.stroke(Stroke::new(2.0, Color32::DARK_GRAY));
                    }
                    _ => {}
                };
                if ui.add(processes_btn).clicked() {
                    self.view = View::Processes(ProcessListState::default());
                    ui.close_menu();
                }
                if ui.add(graphs_btn).clicked() {
                    self.view = View::Graphs(GraphsState::default());
                    ui.close_menu();
                }
            });
            ui.menu_button("Help", |_| {
                frame.close();
            });
        });
    }
}
