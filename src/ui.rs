use eframe::{egui, App, Frame};
use std::{collections::HashMap, error::Error, fs, io::Read, sync::{Arc, Mutex}};
use crate::types::{ValueType, RegionGroup};
use crate::scan::MemoryScanner;

const TYPES:&[&str]=&["Auto","Int8","Int16","Int32","Int64","UInt8","UInt16","UInt32","UInt64","Float32","Float64","Pointer","Size","Bool"];
const MODES:&[&str]=&["Exact","Changed","Unchanged","Increased","Increased or Greater","Increased by","Decreased","Decreased or Less","Decreased by"];

pub struct Smem{
    scanner:Arc<Mutex<MemoryScanner>>,
    groups:Vec<RegionGroup>,
    err:Option<String>,
    zoom:f32,
    selected_region:Option<String>,
    scan_value:String,
    scan_mode:String,
    sel_type:Option<String>,
    scan_history:Vec<HashMap<usize,ValueType>>,
    scan_types_history:Vec<HashMap<usize,(ValueType,String)>>,
    scan_results:Vec<usize>,
    pid_query:String,
    pid_items:Vec<(i32,String)>,
    pid_selected:Option<i32>,
}
impl Smem{
    pub fn new(pid:i32)->Self{
        let scanner=Arc::new(Mutex::new(MemoryScanner::new(pid)));
        let mut this=Self{
            scanner,
            groups:vec![],
            err:None,
            zoom:1.0,
            selected_region:None,
            scan_value:"0".into(),
            scan_mode:"Exact".into(),
            sel_type:None,
            scan_history:vec![],
            scan_types_history:vec![],
            scan_results:vec![],
            pid_query:String::new(),
            pid_items:vec![],
            pid_selected:None,
        };
        let _=this.init();
        this
    }

    fn init(&mut self)->Result<(),Box<dyn Error>>{
        self.refresh_pids();
        let mut s=self.scanner.lock().unwrap();
        if s.pid>0{
            self.groups=s.load_maps()?;
            let _=s.attach();
        }
        Ok(())
    }

    #[inline] fn color(b:u8)->egui::Color32{egui::Color32::from_gray((b as f32*0.8) as u8)}
    #[inline] fn ipx(x:f32)->f32{x.round().max(1.0)}
    #[inline] fn ibox(x:f32,y:f32,w:f32,h:f32)->egui::Rect{let(x,y,w,h)=(x.floor(),y.floor(),w.floor(),h.floor()); egui::Rect::from_min_size(egui::pos2(x,y),egui::vec2(w.max(1.0),h.max(1.0)))}
    #[inline] fn chosen_types(&self)->Option<Vec<ValueType>>{self.sel_type.as_deref().and_then(ValueType::string_to_type).map(|v|vec![v])}

    fn refresh_pids(&mut self){
        let mut v=Vec::new();
        if let Ok(rd)=fs::read_dir("/proc"){
            for e in rd.flatten(){
                if let Ok(pid)=e.file_name().to_string_lossy().parse::<i32>(){
                    let mut comm=String::new();
                    if let Ok(mut f)=fs::File::open(format!("/proc/{pid}/comm")){let _=f.read_to_string(&mut comm);}
                    let n=comm.trim();
                    if !n.is_empty(){v.push((pid,n.to_string()));}
                }
            }
        }
        v.sort_by(|a,b|a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
        self.pid_items=v;
    }

    fn attach_selected(&mut self){
        if let Some(pid)=self.pid_selected{
            let mut s=self.scanner.lock().unwrap();
            s.set_pid(pid);
            self.groups=s.load_maps().unwrap_or_default();
            let _=s.attach();
            s.reset_scan();
            self.scan_history.clear();
            self.scan_types_history.clear();
            self.scan_results.clear();
        }
    }

    fn quick_attach_query(&mut self){
        if let Ok(pid)=self.pid_query.trim().parse::<i32>(){
            self.pid_selected=Some(pid);
            self.attach_selected();
        }
    }

    fn scan(&mut self){
        let tf=self.chosen_types();
        let r=if self.scan_history.is_empty(){
            self.scanner.lock().unwrap().first_scan(
                &self.groups,
                &self.selected_region,
                &self.scan_value,
                &self.scan_mode,
                tf.as_deref(),
                &mut self.scan_history,
                &mut self.scan_types_history,
                &mut self.scan_results
            )
        }else{
            self.scanner.lock().unwrap().next_scan(
                &self.scan_value,
                &self.scan_mode,
                tf.as_deref(),
                &mut self.scan_history,
                &mut self.scan_types_history,
                &mut self.scan_results
            )
        };
        if let Err(e)=r{self.err=Some(e);}
    }

    fn previous_scan(&mut self){self.scanner.lock().unwrap().previous_scan();}
    fn reset_scan(&mut self){
        self.scanner.lock().unwrap().reset_scan();
        self.scan_history.clear();
        self.scan_types_history.clear();
        self.scan_results.clear();
    }

    fn topbar(&mut self,ctx:&egui::Context){
        let (pid,attached)={{let s=self.scanner.lock().unwrap();(s.pid,s.is_attached)}};
        egui::TopBottomPanel::top("top")
            .frame(egui::Frame::default().inner_margin(egui::Margin::symmetric(6.0,6.0)))
            .show(ctx,|ui|{
                if pid>0 && attached{
                    ui.horizontal_wrapped(|ui|{
                        ui.strong(format!("PID {pid}"));
                        if ui.button("Detach").clicked(){self.scanner.lock().unwrap().detach();}
                        if ui.button("Reload Maps").clicked(){if let Ok(v)=self.scanner.lock().unwrap().load_maps(){self.groups=v;}}
                        ui.separator();
                        let mut t=self.sel_type.clone().unwrap_or_else(||"Auto".into());
                        egui::ComboBox::from_id_source("type").selected_text(&t).width(120.0).show_ui(ui,|ui|{
                            for &v in TYPES{if ui.selectable_label(t==v,v).clicked(){t=v.to_string();}}
                        });
                        self.sel_type=if t=="Auto"{None}else{Some(t)};
                        egui::ComboBox::from_id_source("mode").selected_text(&self.scan_mode).width(200.0).show_ui(ui,|ui|{
                            for &m in MODES{ui.selectable_value(&mut self.scan_mode,m.to_string(),m);}
                        });
                        ui.add(egui::TextEdit::singleline(&mut self.scan_value).hint_text("value").desired_width(160.0));
                        if ui.button("Scan").clicked(){self.scan();}
                        if ui.button("Prev").clicked(){self.previous_scan();}
                        if ui.button("Reset").clicked(){self.reset_scan();}
                        ui.separator();
                        ui.add(egui::Slider::new(&mut self.zoom,0.5..=16.0).logarithmic(true).text("Zoom"));
                        ui.separator();
                        if ui.button("Set").clicked(){if let Err(e)=self.scanner.lock().unwrap().address_set(&self.scan_value,&mut self.scan_results){self.err=Some(e);}}
                        if ui.button("Lock").clicked(){MemoryScanner::address_set_lock(self.scanner.clone(),self.scan_value.clone());}
                        if let Some(e)=self.err.take(){ui.colored_label(egui::Color32::RED,e);}
                    });
                }else{
                    ui.horizontal_wrapped(|ui|{
                        ui.strong("Attach");
                        let w=Self::ipx(ui.available_width().min(520.0)*0.45);
                        let r=ui.add(egui::TextEdit::singleline(&mut self.pid_query).hint_text("name or PID").desired_width(w));
                        if (r.lost_focus()&&ui.input(|i|i.key_pressed(egui::Key::Enter)))||ui.button("↩ Attach PID").clicked(){self.quick_attach_query();}
                        if ui.button("Refresh").clicked(){self.refresh_pids();}
                        let q=self.pid_query.to_lowercase();
                        let sel=self.pid_selected.map(|p|p.to_string()).unwrap_or_else(||"select".into());
                        egui::ComboBox::from_id_source("pid_pick").selected_text(sel).width(280.0).show_ui(ui,|ui|{
                            let mut shown=0;
                            for (p,n) in &self.pid_items{
                                if q.is_empty()||n.to_lowercase().contains(&q)||format!("{p}").contains(&q){
                                    if ui.selectable_label(self.pid_selected==Some(*p),format!("{p}  {n}")).clicked(){self.pid_selected=Some(*p);}
                                    shown+=1; if shown>=200{break;}
                                }
                            }
                        });
                        if ui.button("Attach").clicked(){self.attach_selected();}
                        if let Some(e)=self.err.take(){ui.colored_label(egui::Color32::RED,e);}
                    });
                }
            });
    }

    fn tooltip(&self,ctx:&egui::Context,id:egui::Id,row:usize,rect:egui::Rect,resp:&egui::Response,buf:&[u8],bsz:f32){
        if !resp.hovered(){return;}
        egui::show_tooltip(ctx,id,|ui|{
            ui.monospace(format!("0x{row:016X}"));
            if let Some(pos)=resp.hover_pos(){
                let col=((pos.x-rect.min.x)/bsz).floor() as usize;
                if col<buf.len(){
                    let addr=row+col;
                    if let Some((_,tstr))=self.scan_types_history.last().and_then(|m|m.get(&addr)){
                        if let Some(t)=ValueType::string_to_type(tstr){
                            let sz=ValueType::type_size(&t);
                            if col+sz<=buf.len(){
                                let v=ValueType::from_bytes(buf[col..col+sz].to_vec(),t.clone());
                                ui.monospace(format!("{v:?}"));
                                return;
                            }
                        }
                    }
                    ui.monospace(format!("hex 0x{:02X}  dec {}",buf[col],buf[col]));
                }
            }
        });
    }

    fn results_view(&mut self,ui:&mut egui::Ui){
        let latest=self.scan_types_history.last().unwrap();
        let row_h=Self::ipx((6.0*self.zoom).clamp(4.0,24.0));
        let full_w=Self::ipx(ui.available_width());
        egui::ScrollArea::vertical().auto_shrink([false;2]).show(ui,|ui|{
            let mut addrs:Vec<_>=latest.keys().cloned().collect(); addrs.sort_unstable();
            for addr in addrs{
                let sz=ValueType::type_size(&latest[&addr].0);
                let (r,resp)=ui.allocate_exact_size(egui::vec2(full_w,row_h),egui::Sense::click());
                let rect=Self::ibox(r.min.x,r.min.y,full_w,row_h);
                if let Ok(buf)=self.scanner.lock().unwrap().read_memory(addr,sz){
                    ui.painter().rect_filled(rect,0.0,Self::color(buf[0]));
                    ui.painter().text(rect.left_top()+egui::vec2(6.0,0.0),egui::Align2::LEFT_TOP,format!("0x{addr:016x}  [{}]",sz),egui::FontId::monospace(11.0),egui::Color32::LIGHT_GRAY);
                    if resp.clicked_by(egui::PointerButton::Secondary){ui.ctx().output_mut(|o|o.copied_text=format!("0x{:x}",addr));}
                    self.tooltip(ui.ctx(),egui::Id::new(addr),addr,rect,&resp,&buf,row_h);
                }
            }
        });
    }

    fn maps_view(&mut self,ui:&mut egui::Ui){
        egui::ScrollArea::vertical().drag_to_scroll(true).auto_shrink([false;2]).show(ui,|ui|{
            for g in self.groups.iter().filter(|g|g.enabled){
                ui.collapsing(&g.name,|ui|{
                    for region in &g.regions{
                        let size=region.end.saturating_sub(region.start); if size==0{continue;}
                        ui.small(format!("{:x}-{:x}  {} bytes",region.start,region.end,size));
                        let bsz=Self::ipx((6.0*self.zoom).clamp(3.0,18.0));
                        let avail_w=Self::ipx(ui.available_width());
                        let bpr=(avail_w/bsz).floor() as usize; if bpr==0{continue;}
                        let rows=(size+bpr-1)/bpr;
                        for row in 0..rows{
                            let start=region.start+row*bpr; let end=(start+bpr).min(region.end);
                            let width=Self::ipx((end-start) as f32*bsz);
                            let (r,resp)=ui.allocate_exact_size(egui::vec2(width,bsz),egui::Sense::click());
                            let rect=Self::ibox(r.min.x,r.min.y,width,bsz);
                            if !ui.is_rect_visible(rect){continue;}
                            if let Ok(buf)=self.scanner.lock().unwrap().read_memory(start,end-start){
                                let p=ui.painter_at(rect);
                                for (i,&b) in buf.iter().enumerate(){
                                    let x=Self::ipx(rect.min.x+(i as f32)*bsz);
                                    p.rect_filled(Self::ibox(x,rect.min.y,bsz,bsz),0.0,Self::color(b));
                                }
                                if resp.clicked_by(egui::PointerButton::Secondary){ui.ctx().output_mut(|o|o.copied_text=format!("0x{:x}",start));}
                                self.tooltip(ui.ctx(),egui::Id::new(start),start,rect,&resp,&buf,bsz);
                            }
                        }
                    }
                });
            }
        });
    }
}

impl App for Smem{
    fn update(&mut self,ctx:&egui::Context,_:&mut Frame){
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        self.topbar(ctx);
        egui::CentralPanel::default().show(ctx,|ui|{
            if let Some(e)=&self.err{ui.colored_label(egui::Color32::RED,e); return;}
            if !self.scan_types_history.is_empty() && !self.scan_results.is_empty(){self.results_view(ui);}else{self.maps_view(ui);}
        });
    }
}
