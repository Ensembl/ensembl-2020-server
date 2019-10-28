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
        match purpose.get_linear() {
            LinearPath::Offset(0) | LinearPath::Length(0) => {}, /* top */
            _ => { /* non-top */
                let r = context.route.get(&regs[j]).expect(&format!("missing route for {:?}",regs[j]));
                let mut x = harness.get(&regs[j+n]).to_vec();
                let v = harness.get_mut(&r.0);
                v.append(&mut x);
            }
        }
    }
    print!("     (nontop-vec)\n");
}

fn assign(defstore: &DefStore, context: &GenContext, harness: &mut HarnessInterp, name: &str, types: &Vec<MemberType>, regs: &Vec<Register>) {
    let type_ = &types[0]; // signature ensures type match
    let mut filtered = false;
    /* filtered assignment or full replacement? */
    print!("-> {} registers\n",regs.len());
    print!("      -> {:?}\n",type_);
    let offsets = offset(defstore,type_).expect("resolving to registers");
    for (j,v) in offsets.iter().enumerate() {
        print!("          {:?}\n",v);
        if j == 0 {
            let r = context.route.get(&regs[0]).expect(&format!("missing route for {:?}",regs[0]));
            if r.1.len() > 0 {
                filtered = true;
            }
            print!("             route path {:?}\n",r);
        }
    }
    print!("     ({}filtered)\n",if !filtered { "un" } else { "" } );    
    if !filtered {
        assign_unfiltered(defstore,context,harness,name,types,regs);
        return;
    }
    /* append non-top data */
    if let MemberType::Vec(_) = type_ {
        append_nontop_vec(context,harness,&offsets,regs);
    }
    return;

    let mut reglist = Vec::new();
    for (i,type_) in types.iter().enumerate() {
        let offsets = offset(defstore,type_).expect("revolving to registers");
        for v in &offsets {
            reglist.push((i,v.clone()));
        }
    }
    let n = regs.len()/2;
    for i in 0..n {                            
        let r = context.route.get(&regs[i]).expect(&format!("missing route for {:?}",regs[i]));
        let v_new = harness.get(&regs[n+i]).to_vec();
        let x = harness.get(&regs[i]).to_vec();
        let mut v = harness.get(&r.0).to_vec();
        if r.1.len() > 0 {
            let mut filters = (0..v.len()).collect::<Vec<usize>>();
            for step in &r.1 {
                let mut new_filters = vec![];
                if let RouteExpr::SeqFilter(offsets,lens) = step {
                    let offsets = harness.get(offsets);
                    let lens = harness.get(lens);
                    let mut lens_iter = lens.iter();
                    for offset in offsets {
                        let len = lens_iter.next().unwrap();
                        for i in 0..*len {
                            if offset+i < filters.len() {
                                new_filters.push(filters[offset+i]);
                            }
                        }
                    }
                    print!("{:?} route {:?} step {:?}:{:?}/{:?} filter now {:?}\n",regs[i],r.0,r.1,offsets,lens,filters);
                    filters = new_filters;
                }
            }
            print!("{:?} route {:?} filter {:?} data {:?} was {:?} is {}\n",regs[i],r.0,filters,v_new,v,reglist[i].1);
            let mut v_new_iter = v_new.iter().cycle();
            for idx in &filters {
                print!("update {:?} at {:?} to {:?}\n",v,idx,v_new_iter.next().unwrap());
                *v.get_mut(*idx).unwrap() = *v_new_iter.next().unwrap();
            }
            print!("{:?} should be {:?}\n",r.0,v);
            harness.insert(&r.0,v);
        } else {
            harness.insert(&r.0,v_new);
        }
    }
}

pub fn mini_interp(defstore: &DefStore, context: &GenContext) -> (Vec<Vec<Vec<usize>>>,HashMap<Register,Vec<usize>>) {
    let mut printed = Vec::new();
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
                    _ => { panic!("Bad mini-interp instruction {:?}",instr); }        
                }
            },
            _ => { panic!("Bad mini-interp instruction {:?}",instr); }
        }
        for r in instr.get_registers() {
            //print!("{:?}={:?}\n",r,mi_get(&values,&r));
        }
    }
    (printed,harness.dump())
}
