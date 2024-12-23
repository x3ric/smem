#![allow(unused)]
use eframe::{egui, App, Frame, NativeOptions};
use std::{
    collections::HashMap,
    error::Error,
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

#[derive(Clone)]
pub struct MemoryRegion { pub start: usize, pub end: usize }

#[derive(Clone)]
pub struct RegionGroup { pub name: String, pub enabled: bool, pub regions: Vec<MemoryRegion> }

pub struct MemoryScanner {
    pid: i32,
    mem_file: Option<File>,
}

impl MemoryScanner {
    pub fn new(pid: i32) -> Self {
        Self { pid, mem_file: None }
        }

    pub fn attach(&mut self) -> io::Result<()> {
        self.mem_file = Some(File::options().read(true).write(true).open(format!("/proc/{}/mem", self.pid))?);
        Ok(())
    }
    
    pub fn detach(&mut self) {
        self.mem_file = None;
}

    pub fn read_memory(&mut self, start: usize, size: usize) -> io::Result<Vec<u8>> {
        if let Some(ref mut file) = self.mem_file {
            file.seek(SeekFrom::Start(start as u64))?;
            let mut buffer = vec![0; size];
            file.read_exact(&mut buffer)?;
            Ok(buffer)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Memory file is not attached"))
        }
    }

    pub fn set_memory(&mut self, addr: usize, value: &[u8]) -> io::Result<()> {
        if let Some(ref mut file) = self.mem_file {
            file.seek(SeekFrom::Start(addr as u64))?;
            file.write_all(value)?;
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Memory file is not attached"))
        }
    }

    pub fn load_maps(&self) -> Result<Vec<RegionGroup>, Box<dyn Error>> {
        let rdr = BufReader::new(File::open(format!("/proc/{}/maps", self.pid))?);
        let mut grouped: HashMap<String, Vec<MemoryRegion>> = HashMap::new();
        for line in rdr.lines().flatten() {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let range: Vec<_> = parts[0].split('-').collect();
                if range.len() == 2 && parts[1].contains('r') {
                    let start = usize::from_str_radix(range[0], 16)?;
                    let end = usize::from_str_radix(range[1], 16)?;
                    let name = parts.get(5).unwrap_or(&"[Anonymous]").to_string();
                    grouped.entry(name).or_default().push(MemoryRegion { start, end });
                }
            }
        }
        let mut groups: Vec<RegionGroup> = grouped
            .into_iter()
            .map(|(name, regions)| RegionGroup {
                enabled: name == "[stack]",
                name,
                regions,
            })
            .collect();
        groups.sort_by(|a, b| match (a.name.as_str(), b.name.as_str()) {
            (_, _) if a.name == b.name => std::cmp::Ordering::Equal,
            ("[Anonymous]", _) => std::cmp::Ordering::Less,
            (_, "[Anonymous]") => std::cmp::Ordering::Greater,
            ("[heap]", _) => std::cmp::Ordering::Less,
            (_, "[heap]") => std::cmp::Ordering::Greater,
            (_, _) if a.name.starts_with('[') && !b.name.starts_with('[') => std::cmp::Ordering::Less,
            (_, _) if !a.name.starts_with('[') && b.name.starts_with('[') => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
        Ok(groups)
    }
    }

#[derive(Clone, Copy)]
enum ValueType { Int(i32), Float(f32) }

impl ValueType {
    fn from_bytes(bytes: [u8;4], as_float: bool) -> Self {
        if as_float { Self::Float(f32::from_ne_bytes(bytes)) }
        else { Self::Int(i32::from_ne_bytes(bytes)) }
    }

    fn to_bytes(&self) -> [u8;4] {
        match *self {
            Self::Int(x) => x.to_ne_bytes(),
            Self::Float(x) => x.to_ne_bytes(),
        }
    }

    fn equals(&self, other: &ValueType) -> bool {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => (a - b).abs() < f32::EPSILON,
            _ => false
        }
    }

    fn greater(&self, other: &ValueType) -> bool {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => a > b,
            (Self::Float(a), Self::Float(b)) => a > b,
            _ => false
        }
    }

    fn less(&self, other: &ValueType) -> bool {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => a < b,
            (Self::Float(a), Self::Float(b)) => a < b,
            _ => false
        }
    }

    fn add(&self, other: &ValueType) -> ValueType {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => Self::Int(a + b),
            (Self::Float(a), Self::Float(b)) => Self::Float(a + b),
            _ => *self
        }
    }

    fn sub(&self, other: &ValueType) -> ValueType {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => Self::Int(a - b),
            (Self::Float(a), Self::Float(b)) => Self::Float(a - b),
            _ => *self
        }
    }
}

fn parse_user_value(input: &str) -> Option<ValueType> {
    let trimmed = input.trim();
    if trimmed.starts_with("0x") {
        if let Ok(i) = i32::from_str_radix(&trimmed[2..], 16) {
            return Some(ValueType::Int(i));
        }
    }
    if let Ok(f) = trimmed.parse::<f32>() {
        if trimmed.contains('.') { Some(ValueType::Float(f)) }
        else { Some(ValueType::Int(f as i32)) }
    } else { None }
}

pub struct MemVis {
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

impl MemVis {
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
        if let Err(e) = this.load_maps() {
            this.err = Some(format!("Failed to load memory maps: {}", e));
        }
        this
    }

    pub fn attach(&mut self) -> Result<(), String> {
        let mut scanner = self.scanner.lock().map_err(|e| e.to_string())?;
        scanner.attach().map_err(|e| e.to_string())
    }    

    pub fn detach(&mut self) {
        let mut scanner = self.scanner.lock().unwrap();
        scanner.detach();
    }

    fn load_maps(&mut self) -> Result<(), Box<dyn Error>> {
        let scanner = self.scanner.lock().unwrap();
        self.groups = scanner.load_maps()?;
        Ok(())
    }

    fn color(byte: u8) -> egui::Color32 { egui::Color32::from_gray((byte as f32 * 0.8) as u8) }

    fn get_target_regions(&self) -> Vec<MemoryRegion> {
        if let Some(sel) = &self.selected_region {
            self.groups.iter().filter(|g| g.enabled && g.name==*sel).flat_map(|g| g.regions.clone()).collect()
        } else {
            self.groups.iter().filter(|g| g.enabled).flat_map(|g| g.regions.clone()).collect()
        }
    }

    fn comparator(&self, old_val: &ValueType, new_val: &ValueType, inp: &ValueType) -> bool {
        match self.scan_mode.as_str() {
            "Exact" => new_val.equals(inp),
            "Changed" => !new_val.equals(old_val),
            "Unchanged" => new_val.equals(old_val),
            "Increased" => new_val.greater(old_val),
            "Increased or Greater" => new_val.greater(old_val) || new_val.equals(old_val),
            "Increased by" => new_val.equals(&old_val.add(inp)),
            "Decreased" => new_val.less(old_val),
            "Decreased or Less" => new_val.less(old_val) || new_val.equals(old_val),
            "Decreased by" => new_val.equals(&old_val.sub(inp)),
            _ => false
        }
    }

    fn first_scan(&mut self) -> Result<(), String> {
        let val = parse_user_value(&self.scan_value).ok_or("Parse error")?;
        let regs = self.get_target_regions();
        if regs.is_empty() { return Err("No enabled region selected.".into()); }
        let mut baseline = HashMap::new();
        for r in regs {
            if let Ok(buf) = self.scanner.lock().unwrap().read_memory(r.start, r.end.saturating_sub(r.start)) {
                for (i, chunk) in buf.chunks(4).enumerate() {
                    if chunk.len()==4 {
                        let v = ValueType::from_bytes([chunk[0],chunk[1],chunk[2],chunk[3]], matches!(val, ValueType::Float(_)));
                        let addr = r.start + i*4;
                        if self.scan_mode=="Exact" {
                            if v.equals(&val) { baseline.insert(addr, v); }
                        } else {
                            baseline.insert(addr, v);
                        }
                    }
                }
            }
        }
        self.scan_history.clear();
        self.scan_history.push(baseline.clone());
        self.scan_results.clear();
        self.scan_results.extend(baseline.keys().copied());
        Ok(())
    }

    fn next_scan(&mut self) -> Result<(), String> {
        if self.scan_history.is_empty() { return self.first_scan(); }
        let val = parse_user_value(&self.scan_value).ok_or("Parse error")?;
        let prev_map = self.scan_history.last().unwrap();
        let mut new_map = HashMap::new();
        for (&addr, old_val) in prev_map {
            if let Ok(buf) = self.scanner.lock().unwrap().read_memory(addr, 4) {
                if buf.len()==4 {
                    let nv = ValueType::from_bytes([buf[0],buf[1],buf[2],buf[3]], matches!(val, ValueType::Float(_)));
                    if self.comparator(old_val, &nv, &val) {
                        new_map.insert(addr, nv);
                    }
                }
            }
        }
        self.scan_history.push(new_map.clone());
        self.scan_results.clear();
        self.scan_results.extend(new_map.keys().copied());
        Ok(())
    }

    fn do_scan(&mut self) {
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
        let val = match parse_user_value(&self.scan_value) {
            Some(v) => v,
            None => {
                self.err = Some("Invalid value in scan_value.".into());
                return;
            }
        };
        let bytes = val.to_bytes();
        for &addr in &self.scan_results {
            match self.scanner.lock().unwrap().set_memory(addr, &bytes) {
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
            let val = match parse_user_value(&scan_value) {
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
                        if let Err(e) = scanner.set_memory(addr, &bytes) {
                            eprintln!("Failed to set memory at 0x{:x}: {}", addr, e);
                        }
                    }
                }
                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    fn is_scanned(&self) -> bool { !self.scan_history.is_empty() }
}

impl App for MemVis {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        ctx.request_repaint_after(Duration::from_millis(100));

        ctx.input(|i| {
            if i.key_pressed(egui::Key::F11) { self.zoom = (self.zoom * 1.1).clamp(0.2, 8.0); }
            if i.key_pressed(egui::Key::F12) { self.zoom = (self.zoom / 1.1).clamp(0.2, 8.0); }
        });

        egui::Window::new("Scan").anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 8.0)).resizable(false).default_open(false).show(ctx, |ui| {
            ui.add_sized([ui.available_width(), 0.0], egui::TextEdit::singleline(&mut self.scan_value));
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Mode").selected_text(&self.scan_mode).show_ui(ui, |ui| {
                    for mode in ["Exact", "Changed", "Unchanged", "Increased", "Increased or Greater", "Increased by", "Decreased", "Decreased or Less", "Decreased by"] {
                        ui.selectable_value(&mut self.scan_mode, mode.to_string(), mode);
                    }
                });
                if ui.button("Next").clicked() { self.do_scan(); }
                if ui.button("Previous").clicked() { self.previous_scan(); }
                if ui.button("Reset").clicked() { self.reset_scan(); }
                if ui.button("Set").clicked() { self.address_set(); }
                if ui.button("Lock").clicked() { self.address_set_lock(); }
                if ui.button("Attach").clicked() { self.attach(); }
                if ui.button("Detach").clicked() { self.detach(); }
            });
            if let Some(e) = &self.err { ui.colored_label(egui::Color32::RED, e); }
        });

        egui::Window::new("Regions").anchor(egui::Align2::RIGHT_TOP, egui::vec2(-25.0, 8.0)).resizable(false).default_open(false).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for g in &mut self.groups {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut g.enabled, "");
                        if ui.selectable_label(self.selected_region.as_deref() == Some(&g.name), &g.name).clicked() {
                            self.selected_region = Some(g.name.clone());
                        }
                    });
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(e) = &self.err {
                ui.colored_label(egui::Color32::RED, e);
                return;
            }
            ui.style_mut().spacing.item_spacing.y = 0.0;
            egui::ScrollArea::vertical().show(ui, |ui| {
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
                    for m in &merged {
                        ui.label(format!("{:x}-{:x}", m.start, m.end));
                        let size = m.end.saturating_sub(m.start);
                        let bpr = (ui.available_width() / (6.0 * self.zoom)).floor() as usize;
                        let rows = (size + bpr - 1) / bpr;
                        for row in 0..rows {
                            let row_start = m.start + row * bpr;
                            let row_end = (row_start + bpr).min(m.end);
                            let width_px = (row_end - row_start) as f32 * (6.0 * self.zoom);
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(width_px, 6.0 * self.zoom), egui::Sense::click());
                            if !ui.is_rect_visible(rect) {
                                continue;
                            }
                            if let Ok(buf) = self.scanner.lock().unwrap().read_memory(row_start, row_end - row_start) {
                                let paint = ui.painter_at(rect);
                                for (i, &byte) in buf.iter().enumerate() {
                                    let x = rect.min.x + i as f32 * (6.0 * self.zoom);
                                    paint.rect_filled(egui::Rect::from_min_size(egui::pos2(x, rect.min.y), egui::vec2(6.0 * self.zoom, 6.0 * self.zoom)), 0.0, Self::color(byte));
                                }
                                if resp.clicked_by(egui::PointerButton::Secondary) {
                                    ui.ctx().output_mut(|o| o.copied_text = format!("0x{:x}", row_start));
                                }
                                if resp.hovered() {
                                    egui::show_tooltip(ui.ctx(), egui::Id::new(row_start), |ui| {
                                        ui.label(format!("Base: 0x{:X}", row_start));
                                        if let Some(pos) = resp.hover_pos() {
                                            let rel_x = pos.x - rect.min.x;
                                            let col = (rel_x / (6.0 * self.zoom)).floor() as usize;
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
                        }
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
                                    if resp.clicked_by(egui::PointerButton::Secondary) {
                                        ui.ctx().output_mut(|o| o.copied_text = format!("0x{:x}", row_start));
                                    }
                                    if resp.hovered() {
                                        egui::show_tooltip(ui.ctx(), egui::Id::new(row_start), |ui| {
                                            ui.label(format!("Base: 0x{:X}", row_start));
                                            if let Some(pos) = resp.hover_pos() {
                                                let rel_x = pos.x - rect.min.x;
                                                let col = (rel_x / (6.0 * self.zoom)).floor() as usize;
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
                            }
                        }
                    }
                }
            });
            
        });
        
    }
}

fn main() {
    let pid = std::env::args().nth(1).and_then(|x| x.parse().ok()).unwrap_or_else(|| {
        eprintln!("Usage: {} <pid>", std::env::args().next().unwrap());
        std::process::exit(1);
    });
    let app = MemVis::new(pid);
    eframe::run_native("MemVis", NativeOptions::default(), Box::new(|_cc| Box::new(app)));
}
