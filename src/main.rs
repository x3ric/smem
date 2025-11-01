mod scan; mod types; mod ui;
fn main(){
    let pid=std::env::args().nth(1).and_then(|s|s.parse::<i32>().ok()).unwrap_or(0);
    let opt=eframe::NativeOptions::default();
    eframe::run_native("smem",opt,Box::new(move|_|Box::new(ui::Smem::new(pid)))).ok();
}
