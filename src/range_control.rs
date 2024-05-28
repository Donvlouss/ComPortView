use egui::{DragValue, Ui};

pub struct RangeControl {
    pub name: String,
    pub start: f64,
    pub end: f64,
}

impl RangeControl {
    pub fn get(&self) -> [f64; 2] {
        [
            self.start, self.end
        ]
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        egui::Grid::new(self.name.clone())
            .num_columns(4)
            .show(ui, |ui| {
                ui.label(self.name.clone());
                ui.end_row();

                ui.label("Start");
                ui.add(DragValue::new(&mut self.start).clamp_range(f64::NEG_INFINITY..=self.end-0.01));

                ui.label("End");
                ui.add(DragValue::new(&mut self.end).clamp_range(self.start..=f64::INFINITY));
                ui.end_row()
            });
    }
}