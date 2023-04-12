use std::sync::{Mutex, Arc};

use arboard::Clipboard;
use egui::{Label, RichText, ScrollArea, Sense};
use egui_extras::{Column, TableBuilder};
use nom::error::VerboseError;
use regex::Regex;
use sysinfo::{Pid, Process, ProcessExt, Signal, System, SystemExt, UserExt};

use crate::parse_labels::{self, Labels};

pub struct ProcessListState {
    search: String,
    regex: bool,
    label_search: bool,
    first: bool,
    sort: Columns,
    order: Order,
    case_sensitive: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Asc,
    Desc,
}

impl std::ops::Not for Order {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Order::Asc => Order::Desc,
            Order::Desc => Order::Asc,
        }
    }
}

pub enum Columns {
    Pid,
    Owner,
    Name,
}

impl Default for ProcessListState {
    fn default() -> Self {
        Self {
            search: String::new(),
            regex: false,
            label_search: false,
            first: true,
            sort: Columns::Pid,
            order: Order::Asc,
            case_sensitive: false,
        }
    }
}

impl ProcessListState {
    fn filtered_processes<'a>(&self, system: &'a System) -> Vec<(&'a Pid, &'a Process)> {
        let Self {
            search,
            regex,
            label_search,
            ..
        } = self;
        let sensitiveness = |s: &str| {
            if self.case_sensitive {
                s.to_string()
            } else {
                s.to_lowercase()
            }
        };
        if *regex && !*label_search {
            let re = Regex::new(search).unwrap();

            system
                .processes()
                .iter()
                .filter(|(_, process)| re.is_match(process.name()))
                .collect::<Vec<_>>()
        } else if *label_search {
            let labels = parse_labels::parse_input::<VerboseError<&str>>(search);
            let mut processes = system.processes().into_iter().collect::<Vec<_>>();
            if let Err(_) = labels {
                vec![]
            } else {
                let labels = labels.unwrap();
                if labels.0 != "" {
                    vec![]
                } else {
                    for label in labels.1 {
                        match label {
                            Labels::Pid(pid) => {
                                processes = processes
                                    .into_iter()
                                    .filter(|(_, process)| process.pid() == Pid::from(pid))
                                    .collect::<Vec<_>>();
                            }
                            Labels::Owner(name) => {
                                processes = processes
                                    .into_iter()
                                    .filter(|(_, process)| {
                                        sensitiveness(
                                            system
                                                .get_user_by_id(process.user_id().unwrap())
                                                .unwrap()
                                                .name(),
                                        )
                                        .contains(&sensitiveness(&name))
                                    })
                                    .collect::<Vec<_>>();
                            }
                            Labels::Name(name) => {
                                if *regex {
                                    let re = Regex::new(&name).unwrap();
                                    processes = processes
                                        .into_iter()
                                        .filter(|(_, process)| re.is_match(process.name()))
                                        .collect::<Vec<_>>();
                                } else {
                                    processes = processes
                                        .into_iter()
                                        .filter(|(_, process)| {
                                            sensitiveness(process.name())
                                                .contains(&sensitiveness(&name))
                                        })
                                        .collect::<Vec<_>>();
                                }
                            }
                        }
                    }
                    processes
                }
            }
        } else {
            system
                .processes()
                .iter()
                .filter(|(_, process)| {
                    sensitiveness(process.name()).contains(&sensitiveness(&search))
                })
                .collect::<Vec<_>>()
        }
    }

    fn sorted_processes<'a>(&self, system: &'a System) -> Vec<(&'a Pid, &'a Process)> {
        let mut processes = self.filtered_processes(system);
        let sensitiveness = |s: &str| {
            if self.case_sensitive {
                s.to_string()
            } else {
                s.to_lowercase()
            }
        };
        match self.sort {
            Columns::Pid => {
                if self.order == Order::Asc {
                    processes.sort_by_key(|(pid, _)| *pid);
                } else {
                    processes.sort_by_key(|(pid, _)| std::cmp::Reverse(*pid));
                }
            }
            Columns::Owner => {
                if self.order == Order::Asc {
                    processes.sort_by_key(|(_, process)| {
                        sensitiveness(
                            system
                                .get_user_by_id(process.user_id().unwrap())
                                .unwrap()
                                .name(),
                        )
                    });
                } else {
                    processes.sort_by_key(|(_, process)| {
                        std::cmp::Reverse(sensitiveness(
                            system
                                .get_user_by_id(process.user_id().unwrap())
                                .unwrap()
                                .name(),
                        ))
                    });
                }
            }
            Columns::Name => {
                if self.order == Order::Asc {
                    processes.sort_by_key(|(_, process)| sensitiveness(process.name()));
                } else {
                    processes.sort_by_key(|(_, process)| {
                        std::cmp::Reverse(sensitiveness(process.name()))
                    });
                }
            }
        };
        processes
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui, processes: &Vec<(&Pid, &Process)>) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            let text_edit = ui.text_edit_singleline(&mut self.search);
            if self.first {
                text_edit.request_focus();
                self.first = false;
            }
            ui.checkbox(&mut self.regex, "Regex");
            ui.checkbox(&mut self.label_search, "Label search").on_hover_ui(|ui| {
                ui.label(RichText::new("Search using labels").strong());
                ui.horizontal_wrapped(|ui| {
                    ui.label("You can use any column label to perform a label search. If both regex and label search are enabled, name will be regexed. Example :");
                    ui.code("pid:643 owner:root name:\"firefox\"");
                });
            });
            ui.checkbox(&mut self.case_sensitive, "Case sensitive").on_hover_ui(|ui| {
                ui.strong("Case sensitive search");
                ui.horizontal_wrapped(|ui| {
                    ui.label("If enabled, the search will be case sensitive. If disabled, the search will be case insensitive. If regex is turned on, this will be ignored. If you wish to do a case insensitive search, you can add");
                    ui.code("/i");
                    ui.label("at the end of your regex string.");
                });
            });
            if ui.button("Kill all").on_hover_ui(|ui| {
                ui.label("Send KILL to all processes matching the search. If the process of task manager is included, some processes might not be killed.");
            }).clicked() {
                for (_, process) in processes.iter() {
                    process.kill();
                }
            }
            ui.menu_button("Kill all with", |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    for signal in System::SUPPORTED_SIGNALS {
                        if ui.button(format!("Kill with {:?}", signal)).clicked() {
                            for (_, process) in processes.iter() {
                                process.kill_with(*signal);
                            }
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Cancel").clicked() {
                        ui.close_menu();
                    }
                });
            })
        });
    }

    fn context_menu(ui: &mut egui::Ui, _pid: &sysinfo::Pid, process: &sysinfo::Process) {
        ui.label(format!("{}", process.name()));
        ui.separator();
        if ui.button("Kill").clicked() {
            process.kill();
            ui.close_menu();
        }
        if ui.button("Terminate").clicked() {
            process.kill_with(Signal::Term);
            ui.close_menu();
        }
        ui.menu_button("More options", |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for signal in System::SUPPORTED_SIGNALS {
                    if ui.button(format!("Kill with {:?}", signal)).clicked() {
                        process.kill_with(*signal);
                        ui.close_menu();
                    }
                }
            });
        });
        ui.separator();
        if ui.button("Copy name").clicked() {
            let mut clipboard = Clipboard::new().unwrap();
            clipboard.set_text(process.name()).unwrap();
            ui.close_menu();
        }
        if ui.button("Copy PID").clicked() {
            let mut clipboard = Clipboard::new().unwrap();
            clipboard.set_text(process.pid().to_string()).unwrap();
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Cancel").clicked() {
            ui.close_menu();
        }
    }

    fn table(&mut self, ui: &mut egui::Ui, processes: &Vec<(&Pid, &Process)>, system: &System) {
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

        let table = TableBuilder::new(ui)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .striped(true)
            .column(Column::auto().at_least(64.0))
            .column(Column::auto().at_least(128.0))
            .column(Column::remainder())
            .min_scrolled_height(0.0);

        let table = table.header(20.0, |mut header| {
            header.col(|ui| {
                if ui
                    .add(Label::new(RichText::new("pid").strong()).sense(Sense::click()))
                    .clicked()
                {
                    if matches!(self.sort, Columns::Pid) {
                        self.order = !self.order;
                    } else {
                        self.order = Order::Asc;
                    }
                    self.sort = Columns::Pid;
                }
            });
            header.col(|ui| {
                if ui
                    .add(Label::new(RichText::new("owner").strong()).sense(Sense::click()))
                    .clicked()
                {
                    if matches!(self.sort, Columns::Owner) {
                        self.order = !self.order;
                    } else {
                        self.order = Order::Asc;
                    }
                    self.sort = Columns::Owner;
                }
            });
            header.col(|ui| {
                if ui
                    .add(Label::new(RichText::new("name").strong()).sense(Sense::click()))
                    .clicked()
                {
                    println!("test");
                    if matches!(self.sort, Columns::Name) {
                        self.order = !self.order;
                    } else {
                        self.order = Order::Asc;
                    }
                    self.sort = Columns::Name;
                }
            });
        });

        table.body(|body| {
            body.rows(text_height, processes.len(), |row_index, mut row| {
                let (pid, process) = processes[row_index];
                row.col(|ui| {
                    ui.label(pid.to_string());
                })
                .1
                .context_menu(|ui| Self::context_menu(ui, pid, process));
                row.col(|ui| {
                    ui.label(
                        system
                            .get_user_by_id(process.user_id().unwrap())
                            .unwrap()
                            .name(),
                    );
                })
                .1
                .context_menu(|ui| Self::context_menu(ui, pid, process));
                row.col(|ui| {
                    ui.label(process.name());
                })
                .1
                .context_menu(|ui| Self::context_menu(ui, pid, process));
            });
        });
    }

    pub fn process_list_view(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        system: Arc<Mutex<System>>,
    ) {
        let system = system.lock().unwrap();
        egui::CentralPanel::default().show(ctx, |ui| {
            let processes = self.sorted_processes(&system);

            self.menu_bar(ui, &processes);

            self.table(ui, &processes, &system);
        });
    }
}
