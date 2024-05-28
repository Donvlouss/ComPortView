use std::{sync::{atomic::AtomicBool, Arc, Mutex}, thread, time::Duration};

use egui::{Color32, Ui};

use crate::{range_control::RangeControl, BaudRating, ComSetting};

pub struct SerialControl {
    pub data: Arc<Mutex<Vec<f64>>>,
    pub look_behind: usize,
    pub is_running: bool,
    pub setting: ComSetting,
    pub thread_handle: Option<thread::JoinHandle<()>>,
    pub should_run: Option<Arc<AtomicBool>>,
    pub is_custom_baud_rate: bool,
    pub color: Color32,
    pub mark_to_remove: bool,

    pub enable_convert: bool,
    pub input_range: RangeControl,
    pub output_range: RangeControl,
}

impl SerialControl {
    pub fn new(port: String, look_behind: usize) -> Self {
        Self {
            data: Arc::new(Mutex::new(vec![])),
            is_running: false,
            look_behind,
            setting: ComSetting {
                port,
                baud_rate: BaudRating::B115_200 as u32,
            },
            thread_handle: None,
            should_run: None,
            is_custom_baud_rate: false,
            color: Color32::RED,
            mark_to_remove: false,

            enable_convert: false,
            input_range: RangeControl {
                name: "Input".to_owned(),
                start: 0.,
                end: 1024.,
            },
            output_range: RangeControl {
                name: "Output".to_owned(),
                start: 0.,
                end: 3.3,
            },
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.add_enabled_ui(!self.is_running, |ui| {
                egui::Grid::new("Panel")
                    .num_columns(2)
                    .show(ui, |ui| {

                        ui.label(self.setting.port.clone());

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

                        ui.label("Line Color:");
                        ui.color_edit_button_srgba(&mut self.color);
                        ui.end_row();
                    });

                ui.separator();
                self.input_range.ui(ui);
                ui.add(egui::Checkbox::new(&mut self.enable_convert, "Convert"));
                if self.enable_convert {
                    self.output_range.ui(ui);
                }
                ui.separator();
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
        });
    }
}

impl SerialControl {
    fn start(&mut self) {
        let data_clone = self.data.clone();
        let look_behind = self.look_behind;
        let port = self.setting.port.clone();
        let baud = self.setting.baud_rate.clone();

        let should_run = Arc::new(AtomicBool::new(true));
        let should_run_clone = Arc::clone(&should_run);

        let range_in = self.input_range.get();
        let range_out = self.output_range.get();
        let enable_convert = self.enable_convert;

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

                                        let value= if enable_convert {
                                            (value-range_in[0]) / (range_in[1] - range_in[0])
                                            * (range_out[1] - range_out[0]) + range_out[0]
                                        } else {
                                            value
                                        };
                                        data.push(value);
                                        
                                        if data.len() > look_behind {
                                            data.remove(0);
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

    pub fn stop(&mut self) {
        if let Some(should_run) = self.should_run.take() {
            should_run.store(false, std::sync::atomic::Ordering::Relaxed);
            if let Some(handle) = self.thread_handle.take() {
                handle.join().unwrap();
            }
        }
    }
}