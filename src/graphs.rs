use std::{sync::{Arc, Mutex}, thread::JoinHandle};

use egui::plot::{Line, Plot, PlotBounds};
use sysinfo::{System, SystemExt, CpuExt};

pub struct GraphsState {
    points: Arc<Mutex<Vec<[f64;2]>>>,
    thread: Option<JoinHandle<()>>,
    secs: Arc<Mutex<usize>>,
    plot_clicked: bool,
}

impl Default for GraphsState {
    fn default() -> Self {
        Self {
            points: Arc::new(Mutex::new(Vec::default())),
            thread: None,
            secs: Default::default(),
            plot_clicked: false,
        }
    }
}

impl GraphsState {
    pub fn graphs_view(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        system: Arc<Mutex<System>>,
    ) {
        if self.thread.is_none() {
            let system = system.clone();
            let points = self.points.clone();
            let secs = self.secs.clone();
            let ctx = ctx.clone();
            let help = move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let system = system.lock().unwrap();
                    let mut points = points.lock().unwrap();
                    let mut secs = secs.lock().unwrap();
                    let plot_point = [
                        secs.to_owned() as f64,
                        system.global_cpu_info().cpu_usage() as f64
                    ];
                    points.push(plot_point);
                    ctx.request_repaint();
                    *secs += 1;
                }
            };
            self.thread = Some(std::thread::spawn(help));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            let points = self.points.lock().unwrap();
            let line = Line::new(points.iter().cloned().collect::<Vec<[f64;2]>>());
            let secs = self.secs.lock().unwrap().to_owned();
            let plot_bounds = if secs > 60 {
                PlotBounds::from_min_max([-60.0 + (secs as f64), 0.0], [0.0 + (secs as f64), 100.0])
            } else {
                PlotBounds::from_min_max([0.0, 0.0], [60.0, 100.0])
            };
            Plot::new("CPU usage").view_aspect(2.0).show(ui, |plot_ui| {
                if !self.plot_clicked {
                    plot_ui.set_plot_bounds(plot_bounds);
                }
                plot_ui.line(line);
                if plot_ui.plot_clicked() {
                    self.plot_clicked = true;
                }
            })
        });
    }
}
