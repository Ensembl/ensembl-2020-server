use std::collections::{ HashMap, BTreeMap };
use crate::generate::Instruction;
use crate::generate::GenContext;
use crate::model::{ offset, DefStore, LinearPath, Register, RegisterPurpose };
use crate::typeinf::{ BaseType, MemberType, RouteExpr };

struct HarnessInterp {
    values: HashMap<Option<Register>,Vec<usize>>
}

impl HarnessInterp {
    fn new() -> HarnessInterp {
        let mut out = HarnessInterp {
            values: HashMap::new()
        };
        out.values.insert(None,vec![]);
        out
    }

    fn insert(&mut self, r: &Register, v: Vec<usize>) {
        self.values.insert(Some(r.clone()),v);
    }

    fn get_mut<'a>(&'a mut self, r: &Register) -> &'a mut Vec<usize> {
        self.values.entry(Some(r.clone())).or_insert(vec![])
    }

    fn get<'a>(&'a self, r: &Register) -> &'a Vec<usize> {
        self.values.get(&Some(r.clone())).unwrap_or(self.values.get(&None).unwrap())
    }

    fn dump(&mut self) -> HashMap<Register,Vec<usize>> {
        self.values.drain().filter(|(k,_)| k.is_some()).map(|(k,v)| (k.unwrap(),v)).collect()
    }
}

fn assign_unfiltered(defstore: &DefStore, context: &GenContext, harness: &mut HarnessInterp, name: &str, types: &Vec<MemberType>, regs: &Vec<Register>) {
    let n = regs.len()/2;
    for i in 0..n {                            
        let r = context.route.get(&regs[i]).expect(&format!("missing route for {:?}",regs[i]));
        harness.insert(&r.0,harness.get(&regs[i+n]).to_vec()); // XXX don't copy when we have CoW!
    }        
}

fn append_nontop_vec(context: &GenContext, harness: &mut HarnessInterp, offsets: &Vec<RegisterPurpose>, regs: &Vec<Register>) {
    let n = regs.len()/2;
    for (j,purpose) in offsets.iter().enumerate() {
        if !purpose.is_top() {
            let r = context.route.get(&regs[j]).expect(&format!("missing route for {:?}",regs[j]));
            let mut x = harness.get(&regs[j+n]).to_vec();
            let v = harness.get_mut(&r.0);
            v.append(&mut x);
        }
    }
    print!("     (nontop-vec)\n");
}

// XXX efficiently via iterators
fn build_filter(harness: &mut HarnessInterp, seq: &Vec<RouteExpr>) -> Vec<usize> {
    let mut source : Option<Vec<usize>> = None;
    for r in seq {
        match r {
            RouteExpr::SeqFilter(off_reg,len_reg) => {
                let offs = harness.get(off_reg);
                let lens = harness.get(len_reg);
                let mut len_iter = lens.iter();
                let mut new_source = Vec::new();
                for off in offs {
                    let len = len_iter.next().unwrap();
                    match source {
                        None => {
                            for i in 0..*len {
                                new_source.push(off+i);
                            }
                        },
                        Some(ref mut src) => {
                            for i in 0..*len {
                                new_source.push(src[off+i]);
                            }
                        }
                    }
                }
                source = Some(new_source);
            },
            _ => {}
        }
    }
    source.unwrap()
}

fn copy_non_top(context: &GenContext, harness: &mut HarnessInterp, offsets: &Vec<RegisterPurpose>, regs: &Vec<Register>) -> usize {
    /* relies on ordering of innter-to-outer */
    let mut offset = 0;
    let n = regs.len()/2;
    for (j,purpose) in offsets.iter().enumerate() {
        if !purpose.is_top() {
            if let Some(r) = context.route.get(&regs[j]) {
                let applied_offset = match purpose.get_linear() {
                    LinearPath::Offset(_) => offset,
                    _ => 0
                };
                let mut new_values = harness.get(&regs[j+n]).iter().map(|x| x+applied_offset).collect();
                let new_offset = harness.get(&r.0).len();
                print!("      non-top copy to {:?} offset={} (old len {}) from {:?} containing {:?}\n",r.0,offset,new_offset,&regs[j+n],new_values);
                harness.get_mut(&r.0).append(&mut new_values); // XXX no-dup flag
                offset = new_offset;
            }            
        }
    }
    offset
}

fn assign(defstore: &DefStore, context: &GenContext, harness: &mut HarnessInterp, name: &str, types: &Vec<MemberType>, regs: &Vec<Register>) {
    let type_ = &types[0]; // signature ensures type match
    print!("-> {} registers\n",regs.len());
    print!("      -> {:?}\n",type_);
    let offsets = offset(defstore,type_).expect("resolving to registers");
    for (j,v) in offsets.iter().enumerate() {
        print!("          {:?}\n",v);
        if let Some(r) = context.route.get(&regs[j]) {
            print!("             route path {:?}\n",r);
        } 
    }
    /* build filter */
    let mut filter = None;
    for (j,purpose) in offsets.iter().enumerate() {
        if purpose.is_top() {
            if let Some(r) = context.route.get(&regs[j]) {
                if r.1.len() > 0 {
                    print!("      build filter {:?}\n",r.1);
                    filter = Some(build_filter(harness,&r.1));
                }
            }
        }
    }
    if filter.is_none() {
        print!("      (unfiltered)\n");
        assign_unfiltered(defstore,context,harness,name,types,regs);
        return;
    }
    let filter = filter.unwrap();
    print!("      (filter: {:?})\n",filter);
    let n = regs.len()/2;
    for (j,purpose) in offsets.iter().enumerate() {
        if purpose.is_top() {
            if let Some(r) = context.route.get(&regs[j]) {
                let src_data_len =  harness.get(&regs[j+n]).len();
                for (src_pos,dst_pos) in filter.iter().enumerate() {
                    print!("      assigning to top level {:?}/{} from {:?}/{}\n",r.0,dst_pos,&regs[j+n],src_pos);
                    let offset = match purpose.get_linear() {
                        LinearPath::Offset(_) => copy_non_top(context,harness,&offsets,regs),
                        _ => 0
                    };
                    harness.get_mut(&r.0)[*dst_pos] =  harness.get(&regs[j+n])[src_pos % src_data_len]+offset;
                }
            }
        }
    }
}

fn print_vec_bottom(out: &mut String, harness: &mut HarnessInterp, offsets: &Vec<RegisterPurpose>, regs: &Vec<Register>, restrict: Option<(usize,usize)>) {
    for (j,purpose) in offsets.iter().enumerate() {
        match purpose.get_linear() {
            LinearPath::Data => {
                let mut values = harness.get(&regs[j]).to_vec();
                if let Some((a,b)) = restrict {
                    values = values[a..(a+b)].to_vec();
                }
                out.push_str(
                    &values.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")
                );
            },
            _ => {}
        }
    }
}

fn print_vec_level(out: &mut String, harness: &mut HarnessInterp, offsets: &Vec<RegisterPurpose>, regs: &Vec<Register>, level: usize, restrict: Option<(usize,usize)>) {
    /* find registers for level */
    let mut offset_reg = None;
    let mut len_reg = None;
    for (j,purpose) in offsets.iter().enumerate() {
        match purpose.get_linear() {
            LinearPath::Offset(v) if level == *v => { offset_reg = Some(j); },
            LinearPath::Length(v) if level == *v => { len_reg = Some(j); },
            _ => {}
        }
    }
    let mut starts = harness.get(&regs[offset_reg.unwrap()]).clone();
    let mut lens = harness.get(&regs[len_reg.unwrap()]).clone();
    if let Some((a,b)) = restrict {
        starts = starts[a..(a+b)].to_vec();
        lens = lens[a..(a+b)].to_vec();
    }
    let mut len_iter = lens.iter();
    for (j,offset) in starts.iter().enumerate() {
        if j > 0 { out.push_str(","); }
        let len = len_iter.next().unwrap();
        out.push_str("[");
        if level > 0 {
            print_vec_level(out,harness,offsets,regs,level-1,Some((*offset,*len)));            
        } else {
            print_vec_bottom(out,harness,offsets,regs,Some((*offset,*len)));
        }
            out.push_str("]");
    }
}

fn print_vec(defstore: &DefStore, harness: &mut HarnessInterp, type_: &MemberType, regs: &Vec<Register>) -> String {
    let mut out = String::new();
    let offsets = offset(defstore,type_).expect("resolving to registers");
    if offsets.len() > 1 {
        print_vec_level(&mut out,harness,&offsets,regs,(offsets.len()-3)/2,None);
    } else {
        print_vec_bottom(&mut out,harness,&offsets,regs,None);
    }
    out
}

pub fn mini_interp(defstore: &DefStore, context: &GenContext) -> (Vec<Vec<Vec<usize>>>,HashMap<Register,Vec<usize>>,Vec<String>) {
    let mut printed = Vec::new();
    let mut strings = Vec::new();
    let mut harness = HarnessInterp::new();
    for instr in &context.instrs {
        for r in instr.get_registers() {
            //print!("{:?}={:?}\n",r,mi_get(&values,&r));
        }
        print!("{:?}",instr);
        match instr {
            Instruction::Nil(r) => { harness.insert(r,vec![]); },
            Instruction::Append(r,s) => { let mut x = harness.get_mut(s).to_vec(); harness.get_mut(r).append(&mut x); },
            Instruction::Add(r,v) => { let h = &mut harness; let delta = h.get(v)[0]; let v = h.get(r).iter().map(|x| x+delta).collect(); h.insert(&r,v); },
            Instruction::Length(r,s) => { let v = vec![harness.get(s).len()]; harness.insert(&r,v); }
            Instruction::NumberConst(r,n) => { harness.insert(&r,vec![*n as usize]); },
            Instruction::BooleanConst(r,n) => { harness.insert(&r,vec![if *n {1} else {0}]); },
            Instruction::Copy(r,s) | Instruction::Ref(r,s) => { let x = harness.get_mut(s).to_vec(); harness.insert(&r,x); },
            Instruction::Filter(d,s,f) => {
                let h = &mut harness;
                let mut f = h.get(f).iter();
                let mut v = vec![];
                for u in h.get(s) {
                    if *f.next().unwrap() > 0 {
                        v.push(*u);
                    }
                }
                h.insert(d,v);
            },
            Instruction::SeqFilter(d,s,a,b) | Instruction::RefSeqFilter(d,s,a,b) => {
                let h = &mut harness;
                let u = h.get(s);
                let mut v = vec![];
                let mut b_iter = h.get(b).iter();
                for a in h.get(a).iter() {
                    let b = b_iter.next().unwrap();
                    for i in 0..*b {
                        v.push(u[a+i]);
                    }
                }
                h.insert(d,v);
            },
            Instruction::At(d,s) => {
                let mut v = vec![];
                for i in 0..harness.get(s).len() {
                    v.push(i);
                }
                harness.insert(d,v);
            },
            Instruction::Call(name,types,regs) => {
                match &name[..] {
                    "assign" => {
                        assign(defstore,context,&mut harness,name,types,regs);
                    },
                    "print_regs" => {
                        let mut print = Vec::new();
                        for r in regs {
                            let v = harness.get(&r).to_vec();
                            print!("{:?} = {:?}\n",r,v);
                            print.push(v);
                        }
                        printed.push(print);
                    },
                    "print_vec" => {
                        let s = print_vec(defstore,&mut harness,&types[0],regs);
                        print!("{}\n",s);
                        strings.push(s);
                    },
                    _ => { panic!("Bad mini-interp instruction {:?}",instr); }        
                }
            },
            _ => { panic!("Bad mini-interp instruction {:?}",instr); }
        }
        for r in instr.get_registers() {
            //print!("{:?}={:?}\n",r,mi_get(&values,&r));
        }
    }
    (printed,harness.dump(),strings)
}
