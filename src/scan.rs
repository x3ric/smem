use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use crate::types::{MemoryRegion, RegionGroup, ValueType};

pub struct MemoryScanner{
    mem_file:Option<File>,
    pub is_attached:bool,
    pub pid:i32,
    pub scan_history:Vec<HashMap<usize,ValueType>>,
    pub scan_types_history:Vec<HashMap<usize,(ValueType,String)>>,
    pub scan_results:Vec<usize>,
}
impl MemoryScanner{
    pub fn new(pid:i32)->Self{Self{mem_file:None,is_attached:false,pid,scan_history:vec![],scan_types_history:vec![],scan_results:vec![]}}
    pub fn set_pid(&mut self,pid:i32){self.detach(); self.pid=pid;}
    pub fn attach(&mut self)->io::Result<()>{
        if self.is_attached||self.pid<=0{return Ok(());}
        self.mem_file=Some(File::options().read(true).write(true).open(format!("/proc/{}/mem",self.pid))?);
        self.is_attached=true; Ok(())
    }
    pub fn detach(&mut self){self.mem_file=None; self.is_attached=false;}
    fn file(&mut self)->io::Result<&mut File>{self.mem_file.as_mut().ok_or_else(||io::Error::new(io::ErrorKind::Other,"not attached"))}
    pub fn read_memory(&mut self,addr:usize,len:usize)->io::Result<Vec<u8>>{
        let f=self.file()?; f.seek(SeekFrom::Start(addr as u64))?;
        let mut b=vec![0;len]; f.read_exact(&mut b)?; Ok(b)
    }
    pub fn write_memory(&mut self,addr:usize,data:&[u8])->io::Result<()>{
        let f=self.file()?; f.seek(SeekFrom::Start(addr as u64))?; f.write_all(data)
    }
    pub fn load_maps(&self)->Result<Vec<RegionGroup>,Box<dyn Error>>{
        if self.pid<=0{return Ok(vec![]);}
        let rdr=BufReader::new(File::open(format!("/proc/{}/maps",self.pid))?);
        let mut g:HashMap<String,Vec<MemoryRegion>>=HashMap::new();
        for line in rdr.lines().flatten(){
            let p:Vec<_>=line.split_whitespace().collect(); if p.len()<2{continue;}
            let mut r=p[0].split('-'); let (s,e)=(r.next(),r.next()); if s.is_none()||e.is_none(){continue;}
            if !p[1].contains('r'){continue;}
            let start=usize::from_str_radix(s.unwrap(),16)?; let end=usize::from_str_radix(e.unwrap(),16)?;
            if end<=start{continue;}
            let name=if p.len()>=6{p[5..].join(" ")}else{"[Anonymous]".to_string()};
            g.entry(name).or_default().push(MemoryRegion{start,end});
        }
        let mut v:Vec<RegionGroup>=g.into_iter().map(|(name,regions)|RegionGroup{name,enabled:true,regions}).collect();
        v.sort_by(|a,b|match(a.name.as_str(),b.name.as_str()){
            (x,y) if x==y=>std::cmp::Ordering::Equal,
            ("[Anonymous]",_)=>std::cmp::Ordering::Less,(_, "[Anonymous]")=>std::cmp::Ordering::Greater,
            ("[heap]",_)=>std::cmp::Ordering::Less,(_, "[heap]")=>std::cmp::Ordering::Greater,
            (x,y) if x.starts_with('[')&&!y.starts_with('[')=>std::cmp::Ordering::Less,
            (x,y) if !x.starts_with('[')&&y.starts_with('[')=>std::cmp::Ordering::Greater,
            _=>a.name.cmp(&b.name),
        });
        Ok(v)
    }
    pub fn reset_scan(&mut self){self.scan_history.clear(); self.scan_types_history.clear(); self.scan_results.clear();}
    pub fn previous_scan(&mut self){
        if self.scan_history.len()>1{
            self.scan_history.pop();
            self.scan_types_history.pop();
            let prev=self.scan_history.last().unwrap();
            self.scan_results=prev.keys().copied().collect();
        }
    }

    pub fn first_scan(
        &mut self,
        groups:&[RegionGroup],
        selected_region:&Option<String>,
        scan_value:&str,
        scan_mode:&str,
        type_filter:Option<&[ValueType]>,
        scan_hist:&mut Vec<HashMap<usize,ValueType>>,
        scan_types_hist:&mut Vec<HashMap<usize,(ValueType,String)>>,
        scan_results:&mut Vec<usize>
    )->Result<(),String>{
        let val=ValueType::parse_user_value(scan_value).ok_or_else(||"Failed to parse value".to_string())?;
        let scan_types=match type_filter{Some(t)=>t.to_vec(),None=>ValueType::scan_types(&val)};
        let regions:Vec<MemoryRegion>=if let Some(sel)=selected_region{
            groups.iter().filter(|g|g.enabled&&g.name==*sel).flat_map(|g|g.regions.clone()).collect()
        }else{
            groups.iter().filter(|g|g.enabled).flat_map(|g|g.regions.clone()).collect()
        };
        if regions.is_empty(){return Err("No enabled regions".into());}
        let mut baseline:HashMap<usize,(ValueType,String)>=HashMap::new();
        for region in regions{
            let size=region.end.saturating_sub(region.start); if size==0{continue;}
            if let Ok(buf)=self.read_memory(region.start,size){
                for t in &scan_types{
                    let ts=ValueType::type_size(t); if ts==0{continue;}
                    let tname=ValueType::type_to_string(t).to_string();
                    let mut i=0usize;
                    while i+ts<=buf.len(){
                        let a=region.start+i;
                        let v=ValueType::from_bytes(buf[i..i+ts].to_vec(),t.clone());
                        if scan_mode=="Exact"{ if v.equals(&val){baseline.insert(a,(v,tname.clone()));}}
                        else{ baseline.insert(a,(v,tname.clone()));}
                        i+=ts;
                    }
                }
            }
        }
        scan_types_hist.push(baseline.clone());
        scan_hist.push(baseline.iter().map(|(&k,(v,_))|(k,v.clone())).collect());
        *scan_results=baseline.keys().copied().collect();
        Ok(())
    }

    pub fn next_scan(
        &mut self,
        scan_value:&str,
        scan_mode:&str,
        type_filter:Option<&[ValueType]>,
        scan_hist:&mut Vec<HashMap<usize,ValueType>>,
        scan_types_hist:&mut Vec<HashMap<usize,(ValueType,String)>>,
        scan_results:&mut Vec<usize>
    )->Result<(),String>{
        if scan_hist.is_empty(){return Err("No baseline".into());}
        let val=ValueType::parse_user_value(scan_value).ok_or("Failed to parse value")?;
        let scan_types=match type_filter{Some(t)=>t.to_vec(),None=>ValueType::scan_types(&val)};
        let prev=scan_types_hist.last().unwrap().clone();
        let mut new_map:HashMap<usize,(ValueType,String)>=HashMap::new();
        for (&addr,(old,_)) in &prev{
            for t in &scan_types{
                let ts=ValueType::type_size(t); if ts==0{continue;}
                if let Ok(buf)=self.read_memory(addr,ts){
                    if buf.len()==ts{
                        let nv=ValueType::from_bytes(buf,t.clone());
                        if ValueType::comparator(scan_mode,&old,&nv,&val){
                            new_map.insert(addr,(nv,ValueType::type_to_string(t).to_string()));
                        }
                    }
                }
            }
        }
        scan_types_hist.push(new_map.clone());
        scan_hist.push(new_map.iter().map(|(&k,(v,_))|(k,v.clone())).collect());
        *scan_results=new_map.keys().copied().collect();
        Ok(())
    }

    pub fn address_set(&mut self,scan_value:&str,scan_results:&mut Vec<usize>)->Result<(),String>{
        let val=ValueType::parse_user_value(scan_value).ok_or("Bad value")?;
        let bytes=val.to_bytes();
        for a in scan_results.clone(){ self.write_memory(a,&bytes).map_err(|e|format!("0x{a:x} {e}"))?; }
        Ok(())
    }

    pub fn address_set_lock(self_arc:Arc<Mutex<Self>>,scan_value:String){
        thread::spawn(move||{
            loop{
                let (bytes,addrs)={
                    let s=self_arc.lock().unwrap();
                    (ValueType::parse_user_value(&scan_value).map(|v|v.to_bytes()),s.scan_results.clone())
                };
                if let Some(b)=bytes{
                    for a in addrs{
                        if let Ok(mut s)=self_arc.lock(){ let _=s.write_memory(a,&b); }
                    }
                }
                thread::sleep(Duration::from_millis(100));
            }
        });
    }
}
