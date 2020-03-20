use std::collections::{ HashMap };
use crate::generate::Instruction;
use crate::generate::GenContext;
use crate::model::{ offset, DefStore, LinearPath, Register, RegisterPurpose };
use crate::typeinf::{ MemberType, MemberMode };

struct HarnessInterp {
    values: HashMap<Option<Register>,Vec<usize>>,
    alias: HashMap<Register,Register>
}

impl HarnessInterp {
    fn new() -> HarnessInterp {
        let mut out = HarnessInterp {
            values: HashMap::new(),
            alias: HashMap::new()
        };
        out.values.insert(None,vec![]);
        out
    }

    fn resolve(&self, reg: &Register) -> Register {
        match self.alias.get(reg) {
            Some(reg) => self.resolve(reg),
            None => *reg
        }
    }

    fn alias(&mut self, dst: &Register, src: &Register) {
        self.alias.insert(*dst,*src);
    }

    fn insert(&mut self, r: &Register, v: Vec<usize>) {
        let r = self.resolve(r);
        self.values.insert(Some(r),v);
    }

    fn get_mut<'a>(&'a mut self, r: &Register) -> &'a mut Vec<usize> {
        let r = self.resolve(r);
        self.values.entry(Some(r)).or_insert(vec![])
    }

    fn get<'a>(&'a self, r: &Register) -> &'a Vec<usize> {
        let r = self.resolve(r);
        self.values.get(&Some(r)).unwrap_or(self.values.get(&None).unwrap())
    }

    fn dump(&mut self) -> HashMap<Register,Vec<usize>> {
        self.values.drain().filter(|(k,_)| k.is_some()).map(|(k,v)| (k.unwrap(),v)).collect()
    }
}

fn assign_unfiltered(harness: &mut HarnessInterp, regs: &Vec<Register>) {
    let n = regs.len()/2;
    for i in 0..n {                            
        harness.insert(&regs[i],harness.get(&regs[i+n]).to_vec()); // XXX don't copy when we have CoW!
    }        
}

fn assign(defstore: &DefStore, harness: &mut HarnessInterp, types: &Vec<(MemberMode,MemberType)>, regs: &Vec<Register>) {
    if types[0].0 == MemberMode::LValue {
        assign_unfiltered(harness,regs);
    } else {
        assign_filtered(defstore,harness,types,regs);
    }
}

fn assign_filtered(defstore: &DefStore, harness: &mut HarnessInterp, types: &Vec<(MemberMode,MemberType)>, regs: &Vec<Register>) {
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
        let self_right_len = harness.get(&right).len();
        let initial_offset = purpose.get_linear().references()
            .and_then(|p| left_len.get(&p).cloned())
            .unwrap_or(0);
        let copy_offset = purpose.get_linear().references()
            .and_then(|p| right_len.get(&p).cloned())
            .unwrap_or(0);
        if purpose.is_top() {
            for (i,filter_pos) in filter_pos_all.iter().enumerate() {
                let value = harness.get(&right)[i%self_right_len];
                match purpose.get_linear() {
                    LinearPath::Offset(_) => {
                        let value_offset = initial_offset + (i*copy_offset);
                        harness.get_mut(&left)[*filter_pos] = value+value_offset;
                    },
                    LinearPath::Length(_) => {
                        harness.get_mut(&left)[*filter_pos] = value;
                    },
                    LinearPath::Data => {
                        harness.get_mut(&left)[*filter_pos] = value;
                    }
                }
            }
            
        } else {
            for i in 0..filter_pos_all.len() {
                let value_offset = initial_offset + (i*copy_offset);
                let new_values : Vec<_> = harness.get(&right).iter().map(|x| *x+value_offset).collect();
                harness.get_mut(&left).extend(new_values.iter());
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
    for instr in &context.instrs {
        for r in instr.get_registers() {
            print!("{:?}={:?}",r,harness.get(&r));
            print!("\n");
        }
        print!("{:?}",instr);
        match instr {
            Instruction::Nil(r) => { harness.insert(r,vec![]); },
            Instruction::Alias(d,s) => { harness.alias(d,s); },
            Instruction::Append(r,s) => { let mut x = harness.get_mut(s).to_vec(); harness.get_mut(r).append(&mut x); },
            Instruction::Add(r,v) => { let h = &mut harness; let delta = h.get(v)[0]; let v = h.get(r).iter().map(|x| x+delta).collect(); h.insert(&r,v); },
            Instruction::Length(r,s) => { let v = vec![harness.get(s).len()]; harness.insert(&r,v); }
            Instruction::NumberConst(r,n) => { harness.insert(&r,vec![*n as usize]); },
            Instruction::BooleanConst(r,n) => { harness.insert(&r,vec![if *n {1} else {0}]); },
            Instruction::Copy(r,s) => { let x = harness.get_mut(s).to_vec(); harness.insert(&r,x); },
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
            Instruction::SeqFilter(d,s,a,b) => {
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
            Instruction::Run(d,a,b) => {
                let h = &mut harness;
                let mut v = vec![];
                let mut b_iter = h.get(b).iter();
                for a in h.get(a).iter() {
                    let b = b_iter.next().unwrap();
                    for i in 0..*b {
                        v.push(a+i);
                    }
                }
                h.insert(d,v);
            },
            Instruction::At(d,s) | Instruction::LValue(d,s) => {
                let mut v = vec![];
                for i in 0..harness.get(s).len() {
                    v.push(i);
                }
                harness.insert(d,v);
            },
            Instruction::SeqAt(r,b) => {
                let mut v = vec![];
                let b_vals = harness.get(b);
                for b_val in b_vals {
                    for i in 0..*b_val {
                        v.push(i);
                    }
                }
                harness.insert(r,v);
            },
            Instruction::Call(name,types,regs) => {
                match &name[..] {
                    "assign" => {
                        assign(defstore,&mut harness,types,regs);
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
                        let s = print_vec(defstore,&mut harness,&types[0].1,regs);
                        print!("{}\n",s);
                        strings.push(s);
                    },
                    "len" => {
                        vec_len(defstore,&mut harness,&types[1].1,regs);
                    },
                    "eq" | "lt" | "gt" => {
                        number_binop(&mut harness,name,&regs[0],&regs[1],&regs[2]);
                    },
                    _ => { panic!("Bad mini-interp instruction {:?}",instr); }        
                }
            },
            _ => { panic!("Bad mini-interp instruction {:?}",instr); }
        }
        for r in instr.get_registers() {
            print!("{:?}={:?}",r,harness.get(&r));
            print!("\n");
        }
    }
    (printed,harness.dump(),strings)
}
