use std::collections::{ HashMap };
use crate::generate::InstructionType;
use crate::generate::GenContext;
use crate::model::{ offset, DefStore, LinearPath, Register, RegisterPurpose };
use crate::typeinf::{ MemberType, MemberMode, MemberDataFlow };

struct HarnessInterp {
    pending: Option<HashMap<Register,Vec<usize>>>,
    values: HashMap<Option<Register>,Vec<usize>>,
}

impl HarnessInterp {
    fn new() -> HarnessInterp {
        let mut out = HarnessInterp {
            pending: None,
            values: HashMap::new()
        };
        out.values.insert(None,vec![]);
        out
    }

    fn insert(&mut self, r: &Register, v: Vec<usize>) {
        if let Some(ref mut pending) = self.pending {
            pending.insert(*r,v);
        } else {
            self.values.insert(Some(*r),v);
        }
    }

    fn begin(&mut self) {
        self.pending = Some(HashMap::new());
    }

    fn commit(&mut self) {
        for (r,v) in self.pending.take().unwrap().drain() {
            self.insert(&r,v);
        }
    }

    fn get_mut<'a>(&'a mut self, r: &Register) -> &'a mut Vec<usize> {
        self.values.entry(Some(*r)).or_insert(vec![])
    }

    fn get<'a>(&'a self, r: &Register) -> &'a Vec<usize> {
        self.values.get(&Some(*r)).unwrap_or(self.values.get(&None).unwrap())
    }

    fn dump(&mut self) -> HashMap<Register,Vec<usize>> {
        self.values.drain().filter(|(k,_)| k.is_some()).map(|(k,v)| (k.unwrap(),v)).collect()
    }

    fn copy(&mut self, dst: &Register, src: &Register) {
        self.values.insert(Some(*dst),self.get(src).to_vec());
    }

    fn blit(&mut self, dst: &Register, src: &Register, filter: Option<&Register>) {
        let filter_pos_all = filter.map(|r| self.get(r)).cloned();
        let src_val = self.get(&src).to_vec();
        let dst_val = self.get_mut(&dst);
        let src_len = src_val.len();
        if let Some(filter_pos_all) = filter_pos_all {
            for (i,filter_pos) in filter_pos_all.iter().enumerate() {
                dst_val[*filter_pos] = src_val[i%src_len];
            }
        } else {
            let new_values : Vec<_> = src_val.iter().cloned().collect();
            dst_val.extend(new_values.iter());     
        }
    }

    fn blit_number(&mut self, dst: &Register, src: &Register, filter: Option<&Register>, offset: usize, stride: usize) {
        let filter_pos_all = filter.map(|r| self.get(r)).cloned();
        let src_val = self.get(&src).to_vec();
        let dst_val = self.get_mut(&dst);
        let src_len = src_val.len();
        if let Some(filter_pos_all) = filter_pos_all {
            for (i,filter_pos) in filter_pos_all.iter().enumerate() {
                dst_val[*filter_pos] = src_val[i%src_len] + offset + (i*stride);
            }
        } else {
            let new_values : Vec<_> = src_val.iter().map(|x| *x+offset).collect();
            dst_val.extend(new_values.iter());     
        }
    }
}

fn assign_unfiltered(harness: &mut HarnessInterp, regs: &Vec<Register>) {
    let n = regs.len()/2;
    for i in 0..n {                            
        harness.insert(&regs[i],harness.get(&regs[i+n]).to_vec()); // XXX don't copy when we have CoW!
    }        
}

fn assign(defstore: &DefStore, harness: &mut HarnessInterp, types: &Vec<(MemberMode,MemberType,MemberDataFlow)>, regs: &Vec<Register>) {
    if types[0].0 == MemberMode::LValue {
        assign_unfiltered(harness,regs);
    } else {
        assign_filtered(defstore,harness,types,regs);
    }
}

fn assign_filtered(defstore: &DefStore, harness: &mut HarnessInterp, types: &Vec<(MemberMode,MemberType,MemberDataFlow)>, regs: &Vec<Register>) {
    let len = (regs.len()-1)/2;
    let filter = regs[0];
    let left_all = &regs[1..len+1];
    let right_all = &regs[len+1..];
    let left_purposes = offset(defstore,&types[1].1).expect("resolving to registers");
    let right_purposes = offset(defstore,&types[2].1).expect("resolving to registers");
    /* get current lengths (to calculate offsets) */
    let mut left_len = HashMap::new();
    let mut right_len = HashMap::new();
    for (j,purpose) in right_purposes.iter().enumerate() {
        left_len.insert(purpose.get_linear(),harness.get(&left_all[j]).len());
        right_len.insert(purpose.get_linear(),harness.get(&right_all[j]).len());
    }
    let filter_pos_all = harness.get(&filter).clone();
    for (j,purpose) in left_purposes.iter().enumerate() {
        let left = left_all[j];
        let right = right_all[j];
        let initial_offset = purpose.get_linear().references()
            .and_then(|p| left_len.get(&p).cloned())
            .unwrap_or(0);
        let copy_offset = purpose.get_linear().references()
            .and_then(|p| right_len.get(&p).cloned())
            .unwrap_or(0);
        if purpose.is_top() {
            match purpose.get_linear() {
                LinearPath::Offset(_) => {
                    harness.blit_number(&left,&right,Some(&filter),initial_offset,copy_offset);
                },
                LinearPath::Length(_) => {
                    harness.blit_number(&left,&right,Some(&filter),0,0);
                },
                LinearPath::Data => {
                    harness.blit(&left,&right,Some(&filter));
                }
            }
        } else {
            match purpose.get_linear() {
                LinearPath::Offset(_) | LinearPath::Length(_) => {
                    for i in 0..filter_pos_all.len() {
                        harness.blit_number(&left,&right,None,initial_offset+i*copy_offset,0);
                    }
                },
                LinearPath::Data => {
                    for _ in 0..filter_pos_all.len() {
                        harness.blit(&left,&right,None);
                    }
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

fn vec_len(defstore: &DefStore, harness: &mut HarnessInterp, type_: &MemberType, regs: &Vec<Register>) {
    let mut top_reg = None;
    let offsets = offset(defstore,type_).expect("resolving to registers");
    for (j,offset) in offsets.iter().enumerate() {
        match offset.get_linear() {
            LinearPath::Length(_) => {
                if offset.is_top() {
                    top_reg = Some(&regs[j+1]);
                }
            },
            _ => {}
        }
    }
    let lens = harness.get(top_reg.unwrap()).clone();
    harness.insert(&regs[0],lens);
}

fn number_binop(harness: &mut HarnessInterp, name: &str, c: &Register, a: &Register, b: &Register) {
    let a_vals = harness.get(a).clone();
    let b_vals = harness.get(b).clone();
    let mut c_vals = Vec::new();
    if b_vals.len() > 0 {
        let mut b_iter = b_vals.iter().cycle();
        for a_val in &a_vals {
            let b_val = b_iter.next().unwrap();
            match name {
                "eq" => { c_vals.push(if a_val == b_val {1} else {0}); },
                "lt" => { c_vals.push(if a_val < b_val {1} else {0}); },
                "gt" => { c_vals.push(if a_val > b_val {1} else {0}); },
                _ => { panic!("Unknown binop {}",name); }
            }
        }
    }
    harness.insert(c,c_vals);
}

pub fn mini_interp(defstore: &DefStore, context: &GenContext) -> (Vec<Vec<Vec<usize>>>,HashMap<Register,Vec<usize>>,Vec<String>) {
    let mut printed = Vec::new();
    let mut strings = Vec::new();
    let mut harness = HarnessInterp::new();
    for instr in &context.get_instructions() {
        harness.begin();
        for r in instr.get_registers() {
            print!("{:?}={:?}",r,harness.get(&r));
            print!("\n");
        }
        print!("{:?}",instr);
        match &instr.itype {
            InstructionType::Nil => { harness.insert(&instr.regs[0],vec![]); },
            InstructionType::NumberConst(n) => { harness.insert(&instr.regs[0],vec![*n as usize]); },
            InstructionType::Const(nn) => { harness.insert(&instr.regs[0],nn.iter().map(|x| *x as usize).collect()); },
            InstructionType::BooleanConst(n) => { harness.insert(&instr.regs[0],vec![if *n {1} else {0}]); },
            InstructionType::StringConst(_) |
            InstructionType::BytesConst(_) =>
                panic!("Unimplemented"),
            InstructionType::Copy => { harness.copy(&instr.regs[0],&instr.regs[1]); },
            InstructionType::Append => { harness.blit(&instr.regs[0],&instr.regs[1],None); },
            InstructionType::Length => { let v = vec![harness.get(&instr.regs[1]).len()]; harness.insert(&instr.regs[0],v); }
            InstructionType::Add => {
                let h = &mut harness;
                let delta = h.get(&instr.regs[1])[0];
                let v = h.get(&instr.regs[0]).iter().map(|x| x+delta).collect();
                h.insert(&instr.regs[0],v);
            },
            InstructionType::At => {
                let mut v = vec![];
                for i in 0..harness.get(&instr.regs[1]).len() {
                    v.push(i);
                }
                harness.insert(&instr.regs[0],v);
            },
            InstructionType::NumEq => {
                let h = &mut harness;
                let mut v = vec![];
                let mut b_iter = h.get(&instr.regs[2]).iter().cycle();
                for a in h.get(&instr.regs[1]).iter() {
                    let b = b_iter.next().unwrap();
                    v.push(if *a == *b {1} else {0});
                }
                h.insert(&instr.regs[0],v);
            },
            InstructionType::Filter => {
                let h = &mut harness;
                let mut f = h.get(&instr.regs[2]).iter();
                let mut v = vec![];
                for u in h.get(&instr.regs[1]) {
                    if *f.next().unwrap() > 0 {
                        v.push(*u);
                    }
                }
                h.insert(&instr.regs[0],v);
            },
            InstructionType::Run => {
                let h = &mut harness;
                let mut v = vec![];
                let mut b_iter = h.get(&instr.regs[2]).iter();
                for a in h.get(&instr.regs[1]).iter() {
                    let b = b_iter.next().unwrap();
                    for i in 0..*b {
                        v.push(a+i);
                    }
                }
                h.insert(&instr.regs[0],v);
            },
            InstructionType::SeqFilter => {
                let h = &mut harness;
                let u = h.get(&instr.regs[1]);
                let mut v = vec![];
                let mut b_iter = h.get(&instr.regs[3]).iter();
                for a in h.get(&instr.regs[2]).iter() {
                    let b = b_iter.next().unwrap();
                    for i in 0..*b {
                        v.push(u[a+i]);
                    }
                }
                h.insert(&instr.regs[0],v);
            },
            InstructionType::SeqAt => {
                let mut v = vec![];
                let b_vals = harness.get(&instr.regs[1]);
                for b_val in b_vals {
                    for i in 0..*b_val {
                        v.push(i);
                    }
                }
                harness.insert(&instr.regs[0],v);
            },

            InstructionType::Call(name,_,types) => {
                match &name[..] {
                    "assign" => {
                        assign(defstore,&mut harness,&types,&instr.regs);
                    },
                    "print_regs" => {
                        let mut print = Vec::new();
                        for r in &instr.regs {
                            let v = harness.get(&r).to_vec();
                            print!("{:?} = {:?}\n",r,v);
                            print.push(v);
                        }
                        printed.push(print);
                    },
                    "print_vec" => {
                        let s = print_vec(defstore,&mut harness,&types[0].1,&instr.regs);
                        print!("{}\n",s);
                        strings.push(s);
                    },
                    "len" => {
                        vec_len(defstore,&mut harness,&types[1].1,&instr.regs);
                    },
                    "eq" | "lt" | "gt" => {
                        number_binop(&mut harness,&name,&instr.regs[0],&instr.regs[1],&instr.regs[2]);
                    },
                    _ => { panic!("Bad mini-interp instruction {:?}",instr); }        
                }
            },

            InstructionType::Alias |
            InstructionType::Proc(_,_) |
            InstructionType::Operator(_) |
            InstructionType::CtorStruct(_) |
            InstructionType::CtorEnum(_,_) |
            InstructionType::SValue(_,_) |
            InstructionType::EValue(_,_) |
            InstructionType::ETest(_,_) |
            InstructionType::List |
            InstructionType::Square |
            InstructionType::RefSquare |
            InstructionType::FilterSquare |
            InstructionType::Star =>
                panic!("Illegal instruction")
        }
        harness.commit();
        for r in instr.get_registers() {
            print!("{:?}={:?}",r,harness.get(&r));
            print!("\n");
        }
    }
    (printed,harness.dump(),strings)
}
