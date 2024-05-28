use egui::Button;
use egui_plot::{Line, Plot, PlotPoints};

use crate::{SerialControl, FACTOR};

pub struct SerialApp {
    pub look_behind: usize,
    pub components: Vec<SerialControl>,
    pub port_choice: Vec<String>,
    pub current_select: String,
}

impl Default for SerialApp {
    fn default() -> Self {
        let choice = serialport::available_ports().unwrap();
        let choice = choice.into_iter().map(|c| c.port_name).collect::<Vec<String>>();
        Self {
            look_behind: 2000,
            components: vec![],
            current_select: if choice.is_empty() {"".to_owned()} else {choice[0].clone()},
            port_choice: choice,
        }
    }
}

impl eframe::App for SerialApp {

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {

        let mut removed = vec![];
        egui::Window::new("Control Panel")
        .default_pos([1000., 10.])
        .default_width(250.)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
            .show(ui, |ui| {
                if ui.button("Refresh").clicked() {
                    let choice = serialport::available_ports().unwrap();
                    let choice = choice.into_iter().map(|c| c.port_name).collect::<Vec<String>>();
                    if choice.is_empty() {
                        self.components.iter_mut().for_each(|comp| {
                            comp.stop();
                        });
                        self.components.clear();
                        self.current_select = "".to_owned();
                    } else if self.current_select.is_empty() {
                        self.current_select = choice[0].clone();
                    }
                    self.port_choice = choice;
                }
                egui::Grid::new("Panel")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Max data points:");
                    ui.add(egui::DragValue::new(&mut self.look_behind).clamp_range(10..=5000));
                    ui.end_row();

                    ui.label("Device Choose");
                    egui::ComboBox::from_id_source("DeviceChoose")
                        .selected_text(format!("{:?}", self.current_select))
                        .show_ui(ui, |ui| {
                            self.port_choice.clone()
                            .into_iter()
                            .for_each(|c| {
                                ui.selectable_value(&mut self.current_select, c.clone(), c);
                            });
                        });
                    if !self.port_choice.is_empty() 
                        && self.port_choice.len() != self.components.len()
                        && ui.add(Button::new("+").small()).clicked()
                    {
                        self.components.push(
                            SerialControl::new(
                                self.port_choice[self.components.len()].clone(),
                                self.look_behind
                            )
                        );
                    }
                });

                self.components.iter_mut().enumerate().for_each(|(i, comp)| {
                    ui.separator();
                    ui.separator();
                    if !comp.is_running {
                        if ui.button("Remove").clicked() {
                            comp.mark_to_remove = true;
                            removed.push(i);
                        }
                    }
                    comp.ui(ui);
                });
            
                ui.separator();
                egui::Grid::new("Hint")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Right Click");
                        ui.label("Box-Resize.");
                        ui.end_row();

                        ui.label("CTRL + Scroll");
                        ui.label("Zoom");
                        ui.end_row();

                        ui.label("Double-Click");
                        ui.label("Reset");
                        ui.end_row();
                    });
            });
        });

        removed.reverse();
        removed.into_iter().for_each(|i| {
            self.components.remove(i);
        });

        
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut min: Option<f64> = None;
            let mut max: Option<f64> = None;
            let lines = self.components.iter()
                .map(|comp| {
                    let data = comp.data.clone();
                    let data = data.lock().unwrap();
                    let points: PlotPoints = data.iter().enumerate().map(|(i, &y)| [i as f64, y]).collect();
                    let range = if !comp.enable_convert {
                        comp.input_range.get()
                    } else {
                        comp.output_range.get()
                    };

                    match min {
                        Some(v) => {
                            if v > range[0] {
                                min = Some(range[0]);
                            }
                        } ,
                        None => {
                            min = Some(range[0]);
                        },
                    };
                    match max {
                        Some(v) => {
                            if v < range[0] {
                                max = Some(range[1]);
                            }
                        } ,
                        None => {
                            max = Some(range[1]);
                        },
                    };

                    Line::new(points).color(comp.color)
                })
                .collect::<Vec<Line>>();

            let min = match min {
                Some(v) => v,
                None => 0.,
            };
            let max = match max {
                Some(v) => v,
                None => 1024.,
            };

            let mut ratio = FACTOR;
            ctx.input(|i| {
                match i.viewport().inner_rect {
                    Some(rect) => {
                        ratio = rect.width() / rect.height();
                    },
                    None => {},
                } ;
            });

            Plot::new("serial_plot")
                .view_aspect(ratio)
                .include_y(min)
                .include_y(max)
                .allow_drag(false)
                .allow_scroll(false)
                .y_axis_label("Voltage")
                .show(ui, |plot_ui| {
                    lines.into_iter().for_each(|line| plot_ui.line(line));
                });
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(7)); // 60 FPS

    }
}
