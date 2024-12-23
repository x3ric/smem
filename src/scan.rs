use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::types::{ValueType, MemoryRegion, RegionGroup};

pub struct MemoryScanner {
    mem_file: Option<File>,
    pub is_attached: bool,
    pub pid: i32,
    err: Option<String>,
}

impl MemoryScanner {
    pub fn new(pid: i32) -> Self {
        Self {
            mem_file: None,
            is_attached: false,
            pid,
            err: None,
        }
    }

    pub fn attach(&mut self) -> io::Result<()> {
        self.is_attached = true;
        self.mem_file = Some(File::options().read(true).write(true).open(format!("/proc/{}/mem", self.pid))?);
        Ok(())
    }

    pub fn detach(&mut self) {
        self.is_attached = false;
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

    pub fn write_memory(&mut self, addr: usize, value: &[u8]) -> io::Result<()> {
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
