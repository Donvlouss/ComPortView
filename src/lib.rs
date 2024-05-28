mod serial_app;
mod serial_control;
mod range_control;

pub use serial_app::*;
pub use serial_control::*;


#[derive(PartialEq, Clone, Copy)]
pub enum BaudRating {
    B9_600=9600,
    B57_600=57_600,
    B115_200=115_200,
    B256_000=256_000,
    B512_000=512_000,
    B921_600=921_600,
    B3_000_000=3_000_000,
}

pub struct ComSetting {
    pub port: String,
    pub baud_rate: u32,
}

const FACTOR: f32 = 16. / 9.;
