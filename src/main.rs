#![windows_subsystem = "windows"]

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(PartialEq, Clone, Copy)]
enum BaudRating {
    B9_600=9600,
    B57_600=57_600,
    B115_200=115_200,
    B256_000=256_000,
    B512_000=512_000,
    B921_600=921_600,
    B3_000_000=3_000_000,
}

struct ComSetting {
    pub port: String,
    pub baud_rate: u32,
}

struct SerialApp {
    data: Arc<Mutex<Vec<f64>>>,
    // look_behind: Arc<Mutex<usize>>,
    look_behind: usize,
    is_running: bool,
    setting: ComSetting,
    thread_handle: Option<thread::JoinHandle<()>>,
    port_choice: Vec<String>,
    should_run: Option<Arc<AtomicBool>>,
    is_custom_baud_rate: bool,
}

impl SerialApp {
    fn start(&mut self) {
        let data_clone = self.data.clone();
        let look_behind = self.look_behind;
        let port = self.setting.port.clone();
        let baud = self.setting.baud_rate.clone();

        let should_run = Arc::new(AtomicBool::new(true));
        let should_run_clone = Arc::clone(&should_run);

        let handle = thread::spawn(move || {
            let mut port = serialport::new(port, baud as u32)
                .timeout(Duration::from_millis(10))
                .open()
                .expect("Failed to open port");

            let mut serial_buf: Vec<u8> = vec![0; 32];
            let mut buffer = Vec::new();

            while should_run_clone.load(std::sync::atomic::Ordering::Relaxed) {
                match port.read(serial_buf.as_mut_slice()) {
                    Ok(t) => {
                        if t > 0 {
                            buffer.extend_from_slice(&serial_buf[..t]);

                            while let Some(pos) = buffer.iter().position(|&x| x == b'\n') {
                                let line = buffer.drain(..=pos).collect::<Vec<_>>();
                                if let Ok(text) = std::str::from_utf8(&line) {
                                    let trimmed_text = text.trim();
                                    if let Ok(value) = trimmed_text.parse::<f64>() {
                                        let mut data = data_clone.lock().unwrap();
                                        data.push(value);

                                        if data.len() > look_behind {
                                            data.remove(0); // 保持數據量不超過 100
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                    Err(e) => {
                        eprintln!("Error reading from serial port: {:?}", e);
                        break;
                    }
                }
            }
        });

        self.thread_handle = Some(handle);
        self.should_run = Some(should_run);
    }

    fn stop(&mut self) {
        if let Some(should_run) = self.should_run.take() {
            should_run.store(false, std::sync::atomic::Ordering::Relaxed);
            if let Some(handle) = self.thread_handle.take() {
                handle.join().unwrap();
            }
        }
    }
}

impl eframe::App for SerialApp {

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {

        egui::CentralPanel::default().show(ctx, |ui| {
            let data = self.data.clone();
            let data = data.lock().unwrap();
            let points: PlotPoints = data.iter().enumerate().map(|(i, &y)| [i as f64, y/1024.*3.3]).collect();
            let line = Line::new(points);
            Plot::new("serial_plot")
                // .view_aspect(2.0)
                .view_aspect(16. / 9.)
                .include_y(0.0)
                .include_y(3.3)
                .allow_drag(false)
                .allow_scroll(false)
                .y_axis_label("Voltage")
                .show(ui, |plot_ui| {
                    plot_ui.line(line);
                });
        });

        egui::Window::new("Control Panel")
        .default_pos([1000., 10.])
        .default_width(250.)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.add_enabled_ui(!self.is_running, |ui| {
                    if ui.button("Refresh").clicked() {
                        let choice = serialport::available_ports().unwrap();
                        self.port_choice = choice.into_iter().map(|c| c.port_name).collect::<Vec<String>>();
                        if self.port_choice.is_empty() {
                            self.setting.port = "".to_owned();
                        }
                    }
                    egui::Grid::new("Panel")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Max data points:");
                            ui.add(egui::DragValue::new(&mut self.look_behind).clamp_range(10..=5000));
                            ui.end_row();
            
                            ui.label("Device Choose");
                            egui::ComboBox::from_id_source("DeviceChoose")
                                .selected_text(format!("{:?}", self.setting.port))
                                .show_ui(ui, |ui| {
                                    self.port_choice.clone().into_iter().for_each(|c| {
                                        ui.selectable_value(&mut self.setting.port, c.clone(), c);
                                    });
                                });
                            ui.end_row();
            
                            ui.label("Baud-Rate");
                            ui.end_row();
                            ui.checkbox(&mut self.is_custom_baud_rate, "Custom Baud Rate");
                            if self.is_custom_baud_rate  {
                                ui.add(egui::DragValue::new(&mut self.setting.baud_rate).clamp_range(1..=BaudRating::B3_000_000 as u32));
                            } else {
                                egui::ComboBox::from_id_source("BaudRate")
                                    .selected_text(format!("{:?}", self.setting.baud_rate as u32))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.setting.baud_rate, BaudRating::B9_600 as u32, "9_600");
                                        ui.selectable_value(&mut self.setting.baud_rate, BaudRating::B57_600 as u32, "57_600");
                                        ui.selectable_value(&mut self.setting.baud_rate, BaudRating::B115_200 as u32, "115_200");
                                        ui.selectable_value(&mut self.setting.baud_rate, BaudRating::B256_000 as u32, "256_000");
                                        ui.selectable_value(&mut self.setting.baud_rate, BaudRating::B512_000 as u32, "512_000");
                                        ui.selectable_value(&mut self.setting.baud_rate, BaudRating::B921_600 as u32, "921_600");
                                        ui.selectable_value(&mut self.setting.baud_rate, BaudRating::B3_000_000 as u32, "3_000_000");
                                    });
                            }
                            ui.end_row();
                        });
                });
            
                egui::Grid::new("Button")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.add_enabled_ui(!self.is_running && !self.setting.port.is_empty(), |ui| {
                            if ui.button("Start").clicked() {
                                self.is_running = true;
                                self.start();
                            }
                        });
        
                        ui.add_enabled_ui(self.is_running, |ui| {
                            if ui.button("Stop").clicked() {
                                self.stop();
                                self.is_running = false;
                            }
                        });
                        ui.end_row();
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

        ctx.request_repaint_after(Duration::from_millis(7)); // 60 FPS
    }
}

fn main() {
    let data = Arc::new(Mutex::new(Vec::new()));

    let choice = serialport::available_ports().unwrap();
    let choice = choice.into_iter().map(|c| c.port_name).collect::<Vec<String>>();

    let app = SerialApp {
        data,
        look_behind: 2000,
        is_running: false,
        setting: ComSetting {
            port: if choice.is_empty() {"".to_owned()} else {choice[0].clone()} ,
            baud_rate: BaudRating::B115_200 as u32,
        },
        thread_handle: None,
        port_choice: choice,
        should_run: None,
        is_custom_baud_rate: false
    };
    
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = native_options.viewport
        // .with_inner_size([1200., 600.])
        .with_inner_size([1280., 720.])
        .with_resizable(false);
    eframe::run_native("Flow Monitor", native_options, Box::new(|_| Box::new(app))).unwrap();
}
