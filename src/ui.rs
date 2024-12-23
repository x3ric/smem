use eframe::{egui, App, Frame};
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use crate::types::{ValueType, MemoryRegion, RegionGroup};
use crate::scan::MemoryScanner;

pub struct Smem {
    scanner: Arc<Mutex<MemoryScanner>>,
    groups: Vec<RegionGroup>,
    err: Option<String>,
    zoom: f32,
    selected_region: Option<String>,
    scan_value: String,
    scan_mode: String,
    scan_history: Vec<HashMap<usize, ValueType>>,
    scan_results: Vec<usize>,
}

impl Smem {
    pub fn new(pid: i32) -> Self {
        let scanner = Arc::new(Mutex::new(MemoryScanner::new(pid)));
        let mut this = Self {
            scanner,
            groups: vec![],
            err: None,
            zoom: 1.0,
            selected_region: None,
            scan_value: "0".to_string(),
            scan_mode: "Changed".to_string(),
            scan_history: vec![],
            scan_results: vec![],
        };
        if let Err(e) = this.init() {
            this.err = Some(format!("Failed to load memory maps: {}", e));
        }
        this
    }
    
    fn init(&mut self) -> Result<(), Box<dyn Error>> {
        let mut scanner = self.scanner.lock().unwrap();
        self.groups = scanner.load_maps()?;
        scanner.attach()?;
        Ok(())
    }

    fn color(byte: u8) -> egui::Color32 { egui::Color32::from_gray((byte as f32 * 0.8) as u8) }
    
    pub fn first_scan(&mut self) -> Result<(), String> {
        let val = ValueType::parse_user_value(&self.scan_value)
            .ok_or_else(|| "Failed to parse scan value.".to_string())?;
        let scan_types = ValueType::scan_types(&val);
        let regions: Vec<MemoryRegion> = if let Some(sel) = &self.selected_region {
            self.groups.iter().filter(|g| g.enabled && g.name == *sel).flat_map(|g| g.regions.clone()).collect()
        } else {
            self.groups.iter().filter(|g| g.enabled).flat_map(|g| g.regions.clone()).collect()
        };
        if regions.is_empty() {
            return Err("No enabled memory regions selected.".to_string());
        }
        let mut baseline = HashMap::new();
        if let Ok(mut scanner) = self.scanner.lock() {
            for region in regions {
                let memory_size = region.end.saturating_sub(region.start);
                if let Ok(buffer) = scanner.read_memory(region.start, memory_size) {
                    for type_hint in &scan_types {
                        let byte_size = ValueType::type_size(type_hint);
                        for i in 0..(buffer.len() / byte_size) {
                            let chunk_start = i * byte_size;
                            let chunk_end = chunk_start + byte_size;
                            if chunk_end <= buffer.len() {
                                let chunk = &buffer[chunk_start..chunk_end];
                                let value = ValueType::from_bytes(chunk.to_vec(), type_hint.clone());
                                let address = region.start + chunk_start;
                                if self.scan_mode == "Exact" {
                                    if value.equals(&val) {
                                        baseline.insert(address, value);
                                    }
                                } else {
                                    baseline.insert(address, value);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            return Err("Failed to lock scanner.".to_string());
        }
        self.scan_history.clear();
        self.scan_history.push(baseline.clone());
        self.scan_results = baseline.keys().copied().collect();
        Ok(())
    }    

    pub fn next_scan(&mut self) -> Result<(), String> {
        if self.scan_history.is_empty() { return self.first_scan(); }
        let val = ValueType::parse_user_value(&self.scan_value).ok_or("Failed to parse scan value.")?;
        let scan_types = ValueType::scan_types(&val);
        let prev_map = self.scan_history.last().unwrap().clone();
        let mut new_map = HashMap::new();
        if let Ok(mut scanner) = self.scanner.lock() {
            for (&addr, old_val) in &prev_map {
                for type_hint in &scan_types {
                    let byte_size = ValueType::type_size(type_hint);
                    if let Ok(buffer) = scanner.read_memory(addr, byte_size) {
                        if buffer.len() == byte_size {
                            let new_value = ValueType::from_bytes(buffer.to_vec(), type_hint.clone());
                            if ValueType::comparator(&self.scan_mode, old_val, &new_value, &val) {
                                new_map.insert(addr, new_value);
                            }
                        }
                    }
                }
            }
        } else {
            return Err("Failed to lock scanner".to_string());
        }
        self.scan_history.push(new_map.clone());
        self.scan_results = new_map.keys().copied().collect();
        Ok(())
    }

    fn scan(&mut self) {
        let r = if self.scan_history.is_empty() { self.first_scan() } else { self.next_scan() };
        if let Err(e) = r { self.err = Some(e); }
    } 

    fn previous_scan(&mut self) {
        if self.scan_history.len()>1 {
            self.scan_history.pop();
            let prev = self.scan_history.last().unwrap();
            self.scan_results = prev.keys().copied().collect();
        }
    }

    fn reset_scan(&mut self) {
        self.scan_history.clear();
        self.scan_results.clear();
    }

    pub fn address_set(&mut self) {
        let val = match ValueType::parse_user_value(&self.scan_value) {
            Some(v) => v,
            None => {
                self.err = Some("Invalid value in scan_value.".into());
                return;
            }
        };
        let bytes = val.to_bytes();
        for &addr in &self.scan_results {
            match self.scanner.lock().unwrap().write_memory(addr, &bytes) {
                Ok(_) => {}
                Err(e) => {
                    self.err = Some(format!("Failed to set memory at 0x{:x}: {}", addr, e));
                    break;
                }
            }
        }
    }

    pub fn address_set_lock(&mut self) {
        let scanner = Arc::clone(&self.scanner);
        let scan_results = self.scan_results.clone();
        let scan_value = self.scan_value.clone();
        thread::spawn(move || {
            let val = match ValueType::parse_user_value(&scan_value) {
                Some(v) => v,
                None => {
                    eprintln!("Invalid value in scan_value.");
                    return;
                }
            };
            let bytes = val.to_bytes();
            loop {
                for &addr in &scan_results {
                    if let Ok(mut scanner) = scanner.lock() {
                        if let Err(e) = scanner.write_memory(addr, &bytes) {
                            eprintln!("Failed to set memory at 0x{:x}: {}", addr, e);
                        }
                    }
                }
                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    fn is_scanned(&self) -> bool { !self.scan_history.is_empty() }

    fn handle_key_input(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F1) {
                if let Ok(mut scanner) = self.scanner.lock() {
                    if scanner.is_attached { scanner.detach(); } else { scanner.attach(); }
                }
            }
            if i.key_pressed(egui::Key::F2) { self.scan(); }
            if i.key_pressed(egui::Key::F3) { self.previous_scan(); }
            if i.key_pressed(egui::Key::F4) { self.reset_scan(); }
            if i.key_pressed(egui::Key::F5) { self.address_set(); }
            if i.key_pressed(egui::Key::F7) { self.address_set_lock(); }
            if i.key_pressed(egui::Key::F8) { self.scan_mode = "Changed".to_string(); self.scan(); }
            if i.key_pressed(egui::Key::F9) { self.scan_mode = "Increased".to_string(); self.scan(); }
            if i.key_pressed(egui::Key::F10) { self.scan_mode = "Decreased".to_string(); self.scan(); }
            if i.key_pressed(egui::Key::F11) { self.zoom = (self.zoom / 1.1).clamp(0.2, 8.0); }
            if i.key_pressed(egui::Key::F12) { self.zoom = (self.zoom * 1.1).clamp(0.2, 8.0); }
        });
    }

    fn draw_tooltip(&self, ctx: &egui::Context, id: egui::Id, row_start: usize, rect: egui::Rect, resp: &egui::Response, buf: &[u8], zoom: f32) {
        if resp.hovered() {
            egui::show_tooltip(ctx, id, |ui| {
                ui.label(format!("Base: 0x{:X}", row_start));
                if let Some(pos) = resp.hover_pos() {
                    let rel_x = pos.x - rect.min.x;
                    let col = (rel_x / (6.0 * zoom)).floor() as usize;
                    if col < buf.len() {
                        let val = buf[col];
                        ui.label(format!("Hex: 0x{:02X}", val));
                        ui.label(format!("Dec: {}", val));
                        ui.label(format!("Char: {}", if val.is_ascii_graphic() { val as char } else { '.' }));
                    }
                }
            });
        }
    }

    fn draw_scan(&mut self, ctx: &egui::Context) {
        egui::Window::new("Scan").anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 8.0)).resizable(false).default_open(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized([ui.available_width() * 0.4, 0.0], egui::TextEdit::singleline(&mut self.scan_value));
                ui.horizontal(|ui| {
                    if ui.button("Set").clicked() { self.address_set(); }
                    if ui.button("Lock").clicked() { self.address_set_lock(); }
                    if let Ok(mut scanner) = self.scanner.lock() {
                        if scanner.is_attached {
                            if ui.button("Detach").clicked() {
                                scanner.detach();
                            }
                        } else {
                            if ui.button("Attach").clicked() {
                                scanner.attach();
                            }
                        }
                    }
                });
            });
            ui.horizontal(|ui| {
                egui::ComboBox::new("scan_mode", "")
                    .width(135.0)
                    .selected_text(&self.scan_mode)
                    .show_ui(ui, |ui| {
                        for mode in [ "Exact", "Changed", "Unchanged", "Increased", "Increased or Greater", "Increased by", "Decreased", "Decreased or Less", "Decreased by" ] {
                            ui.selectable_value(&mut self.scan_mode, mode.to_string(), mode);
                        }
                    });
                ui.add_space(-7.5);
                if ui.button("Next").clicked() { self.scan(); }
                if ui.button("Prev").clicked() { self.previous_scan(); }
                if ui.button("Reset").clicked() { self.reset_scan(); }
            });
            if let Some(e) = &self.err { ui.colored_label(egui::Color32::RED, e); }
        });
    }

    fn draw_regions(&mut self, ctx: &egui::Context) {
        egui::Window::new("Regions").anchor(egui::Align2::RIGHT_TOP, egui::vec2(-25.0, 8.0)).resizable(false).default_open(false).show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for g in &mut self.groups {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut g.enabled, "");
                            if ui.selectable_label(self.selected_region.as_deref() == Some(&g.name), &g.name).clicked()
                            {
                                self.selected_region = Some(g.name.clone());
                            }
                        });
                    }
                });
            });
    }

    fn draw_maps(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(e) = &self.err {
                ui.colored_label(egui::Color32::RED, e);
                return;
            }
            ui.style_mut().spacing.item_spacing.y = 0.0;
            egui::ScrollArea::vertical().drag_to_scroll(true).show(ui, |ui| {
                if self.is_scanned() && !self.scan_results.is_empty() {
                    let mut sorted = self.scan_results.clone();
                    sorted.sort_unstable();
                    let mut merged = Vec::new();
                    let mut start = sorted[0];
                    let mut end = start + 4;
                    for &a in &sorted[1..] {
                        if a <= end {
                            end = a + 4;
                        } else {
                            merged.push(MemoryRegion { start, end });
                            start = a;
                            end = a + 4;
                        }
                    }
                    merged.push(MemoryRegion { start, end });
                    ui.label(format!("Results: {}", merged.len()));
                    let bytes_per_row = (ui.available_width() / (6.0 * self.zoom)).floor() as usize;
                    let cell_height = 6.0 * self.zoom;
                    let mut current_x = 0.0;
                    let mut current_y = 0.0;
                    for m in &merged {
                        let size = m.end.saturating_sub(m.start);
                        let rows = (size + bytes_per_row - 1) / bytes_per_row;
                        for row in 0..rows {
                            let row_start = m.start + row * bytes_per_row;
                            let row_end = (row_start + bytes_per_row).min(m.end);
                            let width_px = (row_end - row_start) as f32 * (6.0 * self.zoom);   
                            if current_x + width_px > ui.available_width() {
                                current_x = 0.0;
                                current_y += cell_height;
                            }
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(width_px, cell_height), egui::Sense::click());
                            if let Ok(buf) = self.scanner.lock().unwrap().read_memory(row_start, row_end - row_start) {
                                let paint = ui.painter_at(rect);
                                for (i, &byte) in buf.iter().enumerate() {
                                    let x = rect.min.x + i as f32 * (6.0 * self.zoom);
                                    paint.rect_filled(egui::Rect::from_min_size(egui::pos2(x, rect.min.y), egui::vec2(6.0 * self.zoom, 6.0 * self.zoom)), 0.0, Self::color(byte));
                                }
                                if resp.clicked_by(egui::PointerButton::Secondary) { ui.ctx().output_mut(|o| o.copied_text = format!("0x{:x}", row_start)); }
                                self.draw_tooltip(ui.ctx(), egui::Id::new(row_start), row_start, rect, &resp, &buf, self.zoom);
                            }
                            current_x += width_px;
                        }
                        current_x = 0.0;
                        current_y += cell_height;
                    }
                    if current_x > 0.0 {
                        current_y += cell_height;
                    }
                } else {
                    let enabled_groups: Vec<_> = self.groups.iter().filter(|g| g.enabled).cloned().collect();
                    for g in enabled_groups {
                        ui.heading(&g.name);
                        for r in &g.regions {
                            ui.label(format!("{:x}-{:x}", r.start, r.end));
                            let size = r.end.saturating_sub(r.start);
                            let bpr = (ui.available_width() / (6.0 * self.zoom)).floor() as usize;
                            let rows = (size + bpr - 1) / bpr;
                            for row in 0..rows {
                                let row_start = r.start + row * bpr;
                                let row_end = (row_start + bpr).min(r.end);
                                let width_px = (row_end - row_start) as f32 * (6.0 * self.zoom);
                                let (rect, resp) = ui.allocate_exact_size(egui::vec2(width_px, 6.0 * self.zoom), egui::Sense::click());
                                if !ui.is_rect_visible(rect) { continue; }
                                if let Ok(buf) = self.scanner.lock().unwrap().read_memory(row_start, row_end - row_start) {
                                    let paint = ui.painter_at(rect);
                                    for (i, &byte) in buf.iter().enumerate() {
                                        let x = rect.min.x + i as f32 * (6.0 * self.zoom);
                                        paint.rect_filled(egui::Rect::from_min_size(egui::pos2(x, rect.min.y), egui::vec2(6.0 * self.zoom, 6.0 * self.zoom)), 0.0, Self::color(byte));
                                    }
                                    if resp.clicked_by(egui::PointerButton::Secondary) { ui.ctx().output_mut(|o| o.copied_text = format!("0x{:x}", row_start)); }
                                    self.draw_tooltip(ui.ctx(), egui::Id::new(row_start), row_start, rect, &resp, &buf, self.zoom);
                                }
                            }
                        }
                    }
                }
            });
        });
    }
}

impl App for Smem {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        ctx.request_repaint_after(Duration::from_millis(100));
        self.handle_key_input(ctx);
        self.draw_scan(ctx);
        self.draw_regions(ctx);
        self.draw_maps(ctx);       
    }
}