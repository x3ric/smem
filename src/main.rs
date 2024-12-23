#![allow(unused)]

//TODO left click to set value single address
//TODO allinone scanning for all types
//TODO move ui.rs scan fn into scan.rs
//TODO Implement all types
// Integer types (signed) Int8(i8),Int16(i16),Int32(i32),Int64(i64),
// Integer types (unsigned) UInt8(u8),UInt16(u16),UInt32(u32),UInt64(u64),
// Floating point types Float32(f32),Float64(f64),
// Special numeric types Size(usize),Pointer(usize),
// Array types ByteArray(Vec<u8>),String(String),
// Boolean type Bool(bool),

use eframe::NativeOptions;    

mod types;
mod scan;
mod ui;
use crate::ui::Smem;

fn main() {
    let pid = std::env::args().nth(1).and_then(|x| x.parse().ok()).unwrap_or_else(|| {
        eprintln!("Usage: {} <pid>", std::env::args().next().unwrap());
        std::process::exit(1);
    });
    let app = Smem::new(pid);
    eframe::run_native("Smem", NativeOptions::default(), Box::new(|_cc| Box::new(app)));
}
