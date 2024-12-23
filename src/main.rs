#![allow(unused)]

use eframe::NativeOptions;    

mod types;
mod scanner;
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
