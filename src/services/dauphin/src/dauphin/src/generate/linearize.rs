// TODO Copy for registers
use std::collections::BTreeMap;

use crate::model::Register;
use crate::typeinf::{ BaseType, MemberType, RouteExpr };
use super::codegen::GenContext;
use super::intstruction::Instruction;

use super::optimise::remove_unused_registers;

#[derive(Debug)]
struct Linearized {
    index: Vec<(Register,Register)>,
    data: Register
}

impl Linearized {
    fn new(context: &mut GenContext, type_: &MemberType, depth: usize) -> Linearized {
        let mut indices = Vec::new();
        for _ in 0..depth {
            let start = context.regalloc.allocate();
            let len = context.regalloc.allocate();
            context.types.add(&start,&MemberType::Base(BaseType::NumberType));
            context.types.add(&len,&MemberType::Base(BaseType::NumberType));
            indices.push((start,len));
        }
        let data = context.regalloc.allocate();
        context.types.add(&data,&MemberType::Base(type_.get_base()));
        Linearized {
            index: indices,
            data
        }
    }
}

fn allocate_subregs(context: &mut GenContext) -> BTreeMap<Register,Linearized> {
    let mut targets = Vec::new();
    for (reg,type_) in context.types.each_register() {
        let depth = type_.depth();
        if depth > 0 {
            targets.push((reg.clone(),type_.clone(),depth));
        }
    }
    let mut out = BTreeMap::new();
    for (reg,type_,depth) in &targets {
        out.insert(reg.clone(),Linearized::new(context,type_,*depth));
    }
    out
}

fn tmp_number_reg(context: &mut GenContext) -> Register {
    let r = context.regalloc.allocate();
    context.types.add(&r,&MemberType::Base(BaseType::NumberType));
    r
}

fn lower_seq_length(out: &mut Vec<Instruction>, context: &mut GenContext, lin: &Linearized, level: usize) -> Register {
    let reg = tmp_number_reg(context);
    if level == 0 {
        out.push(Instruction::Length(reg.clone(),lin.data.clone()));
    } else {
        out.push(Instruction::Length(reg.clone(),lin.index[level-1].0.clone()));
    }
    reg
}

fn push_copy_level(out: &mut Vec<Instruction>, context: &mut GenContext, lin_dst: &Linearized, lin_src: &Linearized, level: usize) {
    /* offset is offset in next layer down (be it index or data) */
    let offset = lower_seq_length(out,context,lin_dst,level);
    let tmp = tmp_number_reg(context);
    out.push(Instruction::Copy(tmp.clone(),lin_src.index[level].0.clone()));
    out.push(Instruction::Add(tmp.clone(),offset.clone()));
    out.push(Instruction::Append(lin_dst.index[level].0.clone(),tmp));
    out.push(Instruction::Append(lin_dst.index[level].1.clone(),lin_src.index[level].1.clone()));
}

fn push_top(out: &mut Vec<Instruction>, context: &mut GenContext, lin_dst: &Linearized, lin_src: &Linearized, level: usize) {
    /* top level offset is current length of next level down plus offset in source */
    let src_len = lower_seq_length(out,context,lin_dst,level);
    let tmp = tmp_number_reg(context);
    out.push(Instruction::Copy(tmp.clone(),lin_src.index[level].0.clone()));
    out.push(Instruction::Add(tmp.clone(),src_len.clone()));
    out.push(Instruction::Append(lin_dst.index[level].0.clone(),tmp));
    /* top level lengths are copied over */
    out.push(Instruction::Append(lin_dst.index[level].1.clone(),lin_src.index[level].1.clone()));
}

fn linear_extend<F>(subregs: &BTreeMap<Register,Linearized>, dst: &Register, src: &Register, mut cb: F)
        where F: FnMut(&Register,&Register,bool) {
    if let Some(lin_src) = subregs.get(src) {
        let lin_dst = subregs.get(dst).unwrap();
        cb(&lin_dst.data,&lin_src.data,true);
        for level in 0..lin_src.index.len() {
            cb(&lin_dst.index[level].0,&lin_src.index[level].0,false);
            cb(&lin_dst.index[level].1,&lin_src.index[level].1,false);
        }
    } else {
        cb(dst,src,true);
    }
}

fn linearize_one(out: &mut Vec<Instruction>, context: &mut GenContext, subregs: &BTreeMap<Register,Linearized> , instr: &Instruction) -> Result<(),String> {
    match instr {
        Instruction::CtorStruct(_,_,_) |
        Instruction::CtorEnum(_,_,_,_) |
        Instruction::SValue(_,_,_,_) |
        Instruction::EValue(_,_,_,_) |
        Instruction::ETest(_,_,_,_) |
        Instruction::SeqFilter(_,_,_,_) |
        Instruction::RefSeqFilter(_,_,_,_) |
        Instruction::RefFilter(_,_,_) |
        Instruction::RefSValue(_,_,_,_) |
        Instruction::Proc(_,_) |
        Instruction::Operator(_,_,_) |
        Instruction::RefEValue(_,_,_,_) => {
            panic!("unexpected instruction {:?}",instr);
        },
        Instruction::NumberConst(_,_) |
        Instruction::BooleanConst(_,_) |
        Instruction::StringConst(_,_) | 
        Instruction::BytesConst(_,_) => out.push(instr.clone()),
        Instruction::List(r) => {
            let lin = subregs.get(r).ok_or_else(|| format!("Missing info for register {:?}",r))?;
            out.push(Instruction::Nil(lin.data.clone()));
            for (start,len) in &lin.index {
                out.push(Instruction::Nil(start.clone()));
                out.push(Instruction::Nil(len.clone()));
            }
        },
        Instruction::Call(name,type_,regs) => {
            let mut new = Vec::new();
            for r in regs {
                if let Some(lin_src) = subregs.get(r) {
                    for i in 0..lin_src.index.len() {
                        new.push(lin_src.index[i].0.clone());
                        new.push(lin_src.index[i].1.clone());
                    }
                    new.push(lin_src.data.clone());
                } else {
                    new.push(r.clone());
                }
            }
            out.push(Instruction::Call(name.clone(),type_.clone(),new));
        },
        Instruction::Append(dst,src) => {
            if let Some(lin_src) = subregs.get(src) {
                let lin_dst = subregs.get(dst).ok_or_else(|| format!("Missing info for register {:?} in push",dst))?;
                push_top(out,context,lin_dst,lin_src,lin_src.index.len()-1);
                for level in (0..lin_src.index.len()-1).rev() {
                    push_copy_level(out,context,lin_dst,lin_src,level);
                }
                out.push(Instruction::Append(lin_dst.data.clone(),lin_src.data.clone()));
            } else {
                out.push(Instruction::Append(dst.clone(),src.clone()));
            }
        },
        Instruction::Nil(_) => {
            out.push(instr.clone());
        },
        Instruction::Copy(dst,src) => {
            linear_extend(subregs,dst,src, |d,s,_| {
                out.push(Instruction::Copy(d.clone(),s.clone()));
            });
        },
        Instruction::Ref(dst,src) => {
            linear_extend(subregs,dst,src, |d,s,dr| {
                context.route.set_empty(d,s);
                if dr {
                    out.push(Instruction::Ref(d.clone(),s.clone()));
                } else {
                    out.push(Instruction::Copy(d.clone(),s.clone()));
                }
            });
        },
        Instruction::NumEq(_,_,_) => {
            out.push(instr.clone())
        },
        // XXX unfiltered tracking
        Instruction::Square(dst,src) => {
            let lin_src = subregs.get(src).ok_or_else(|| format!("Missing info for register {:?}",dst))?;
            if lin_src.index.len() > 1 {
                let lin_dst = subregs.get(dst).ok_or_else(|| format!("Missing info for register {:?}",dst))?;
                out.push(Instruction::Copy(lin_dst.data.clone(),lin_src.data.clone()));
                let top_level = lin_dst.index.len()-1;
                if top_level > 0 {
                    for level in 0..top_level {
                        out.push(Instruction::Copy(lin_dst.index[level].0.clone(),lin_src.index[level].0.clone()));
                        out.push(Instruction::Copy(lin_dst.index[level].1.clone(),lin_src.index[level].1.clone()));
                    }
                }
                out.push(Instruction::SeqFilter(lin_dst.index[top_level].0.clone(),lin_src.index[top_level].0.clone(),
                                                lin_src.index[top_level+1].0.clone(),lin_src.index[top_level+1].1.clone()));
                out.push(Instruction::SeqFilter(lin_dst.index[top_level].1.clone(),lin_src.index[top_level].1.clone(),
                                                lin_src.index[top_level+1].0.clone(),lin_src.index[top_level+1].1.clone()));
            } else {
                out.push(Instruction::SeqFilter(dst.clone(),lin_src.data.clone(),
                                                lin_src.index[0].0.clone(),lin_src.index[0].1.clone()));
            }
        },
        Instruction::RefSquare(dst,src) => {
            let lin_src = subregs.get(src).ok_or_else(|| format!("Missing info for register {:?}",dst))?;
            if lin_src.index.len() > 1 {
                let lin_dst = subregs.get(dst).ok_or_else(|| format!("Missing info for register {:?}",dst))?;
                out.push(Instruction::Copy(lin_dst.data.clone(),lin_src.data.clone()));
                let data_type = context.types.get(&lin_src.data).unwrap().clone();
                context.types.add(&lin_dst.data,&data_type);
                let top_level = lin_dst.index.len()-1;
                if top_level > 0 {
                    for level in 0..top_level {
                        out.push(Instruction::Copy(lin_dst.index[level].0.clone(),lin_src.index[level].0.clone()));
                        out.push(Instruction::Copy(lin_dst.index[level].1.clone(),lin_src.index[level].1.clone()));
                    }
                }
                out.push(Instruction::SeqFilter(lin_dst.index[top_level].0.clone(),lin_src.index[top_level].0.clone(),
                                                lin_src.index[top_level+1].0.clone(),lin_src.index[top_level+1].1.clone()));
                out.push(Instruction::SeqFilter(lin_dst.index[top_level].1.clone(),lin_src.index[top_level].1.clone(),
                                                lin_src.index[top_level+1].0.clone(),lin_src.index[top_level+1].1.clone()));
            } else {
                let tmp_a = context.regalloc.allocate();
                context.types.add(&tmp_a,&MemberType::Base(BaseType::NumberType));
                let tmp_b = context.regalloc.allocate();
                context.types.add(&tmp_b,&MemberType::Base(BaseType::NumberType));
                out.push(Instruction::Copy(tmp_a.clone(),lin_src.index[0].0.clone()));
                out.push(Instruction::Copy(tmp_b.clone(),lin_src.index[0].1.clone()));
                out.push(Instruction::RefSeqFilter(dst.clone(),lin_src.data.clone(),tmp_a.clone(),tmp_b.clone()));
                let data_type = context.types.get(&src).unwrap().clone();
                context.types.add(&dst,&data_type);
                context.route.set_derive(&dst,&lin_src.data,&RouteExpr::SeqFilter(tmp_a.clone(),tmp_b.clone()));
            }
        },
        Instruction::At(dst,src) => {
            if let Some(lin_src) = subregs.get(src) {
                out.push(Instruction::At(dst.clone(),lin_src.index[0].0.clone()));
            } else {
                out.push(Instruction::At(dst.clone(),src.clone()));
            }
        },
        Instruction::Filter(dst,src,f) => {
            if let Some(lin_src) = subregs.get(src) {
                let lin_dst = subregs.get(dst).ok_or_else(|| format!("Missing info for register {:?}",dst))?;
                let top_level = lin_dst.index.len()-1;
                out.push(Instruction::Filter(lin_dst.index[top_level].0.clone(),lin_src.index[top_level].0.clone(),f.clone()));
                out.push(Instruction::Filter(lin_dst.index[top_level].1.clone(),lin_src.index[top_level].1.clone(),f.clone()));
                out.push(Instruction::Copy(lin_dst.data.clone(),lin_src.data.clone()));
                if top_level > 0 {
                    for level in 0..top_level {
                        out.push(Instruction::Copy(lin_dst.index[level].0.clone(),lin_src.index[level].0.clone()));
                        out.push(Instruction::Copy(lin_dst.index[level].1.clone(),lin_src.index[level].1.clone()));
                    }
                }
            } else {
                out.push(instr.clone());
            }
        },
        Instruction::Length(_,_) | Instruction::Add(_,_) => {
            return Err(format!("Bad instruction {:?}",instr.clone()));
        },
        Instruction::Star(dst,src) => {
            let lin_dst = subregs.get(dst).ok_or_else(|| format!("Missing info for register {:?}",dst))?;
            let top_level = lin_dst.index.len()-1;
            out.push(Instruction::Nil(lin_dst.index[top_level].0.clone()));
            let src_len = if let Some(lin_src) = subregs.get(src) {
                let src_len = lower_seq_length(out,context,lin_src,top_level);
                if top_level > 0 {
                    for level in 0..top_level {
                        out.push(Instruction::Copy(lin_dst.index[level].0.clone(),lin_src.index[level].0.clone()));
                        out.push(Instruction::Copy(lin_dst.index[level].1.clone(),lin_src.index[level].1.clone()));
                    }
                }
                out.push(Instruction::Append(lin_dst.data.clone(),lin_src.data.clone()));
                src_len
            } else {
                let src_len = tmp_number_reg(context);
                out.push(Instruction::Length(src_len.clone(),src.clone()));
                out.push(Instruction::Append(lin_dst.data.clone(),src.clone()));
                src_len
            };
            let zero_reg = tmp_number_reg(context);
            out.push(Instruction::NumberConst(zero_reg.clone(),0.));
            out.push(Instruction::Append(lin_dst.index[top_level].0.clone(),zero_reg));
            out.push(Instruction::Append(lin_dst.index[top_level].1.clone(),src_len.clone()));
        },
        //_ => {} // XXX
    };
    Ok(())
}

fn linearize_real(context: &mut GenContext) -> Result<BTreeMap<Register,Linearized>,String> {
    remove_unused_registers(context);
    let subregs = allocate_subregs(context);
    let mut instrs = Vec::new();
    for instr in &context.instrs.to_vec() {
        linearize_one(&mut instrs,context,&subregs,&instr)?;
    }
    context.instrs = instrs;
    print!("subregs {:?}\n",subregs);
    Ok(subregs)
}

pub fn linearize(context: &mut GenContext) -> Result<(),String> {
    linearize_real(context)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use super::*;
    use super::super::call;
    use super::super::simplify::simplify;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::generate::generate_code;

    fn mi_ins(values: &mut HashMap<Option<Register>,Vec<usize>>, r: &Register, v: Vec<usize>) {
        values.insert(Some(r.clone()),v);
    }

    fn mi_mut<'a>(values: &'a mut HashMap<Option<Register>,Vec<usize>>, r: &Register) -> &'a mut Vec<usize> {
        values.entry(Some(r.clone())).or_insert(vec![])
    }

    fn mi_get<'a>(values: &'a HashMap<Option<Register>,Vec<usize>>, r: &Register) -> &'a Vec<usize> {
        values.get(&Some(r.clone())).unwrap_or(values.get(&None).unwrap())
    }

    fn mini_interp(context: &GenContext) -> (Vec<Vec<Vec<usize>>>,HashMap<Register,Vec<usize>>) {
        let mut printed = Vec::new();
        let mut values : HashMap<Option<Register>,Vec<usize>> = HashMap::new();
        values.insert(None,vec![]);
        for instr in &context.instrs {
            for r in instr.get_registers() {
                //print!("{:?}={:?}\n",r,mi_get(&values,&r));
            }
            print!("{:?}",instr);
            match instr {
                Instruction::Nil(r) => { mi_ins(&mut values,r,vec![]); },
                Instruction::Append(r,s) => { let mut x = mi_mut(&mut values,s).to_vec(); mi_mut(&mut values,r).append(&mut x); },
                Instruction::Add(r,v) => { let v = mi_get(&values,r).iter().map(|x| x+mi_get(&values,v)[0]).collect(); mi_ins(&mut values,&r,v); },
                Instruction::Length(r,s) => { let v = vec![mi_get(&values,s).len()]; mi_ins(&mut values,&r,v); }
                Instruction::NumberConst(r,n) => { mi_ins(&mut values,&r,vec![*n as usize]); },
                Instruction::BooleanConst(r,n) => { mi_ins(&mut values,&r,vec![if *n {1} else {0}]); },
                Instruction::Copy(r,s) => { let x = mi_mut(&mut values,s).to_vec(); mi_ins(&mut values,&r,x); },
                Instruction::Ref(d,s) => {
                    let mut v = vec![];
                    for i in 0..mi_get(&values,s).len() {
                        v.push(i);
                    }
                    mi_ins(&mut values,d,v);
                },
                Instruction::Filter(d,s,f) => {
                    let mut f = mi_get(&values,f).iter();
                    let mut v = vec![];
                    for u in mi_get(&values,s) {
                        if *f.next().unwrap() > 0 {
                            v.push(*u);
                        }
                    }
                    mi_ins(&mut values,d,v);
                },
                Instruction::SeqFilter(d,s,a,b) | Instruction::RefSeqFilter(d,s,a,b) => {
                    let u = mi_get(&values,s);
                    let mut v = vec![];
                    let mut b_iter = mi_get(&values,b).iter();
                    for a in mi_get(&values,a).iter() {
                        let b = b_iter.next().unwrap();
                        for i in 0..*b {
                            v.push(u[a+i]);
                        }
                    }
                    mi_ins(&mut values,d,v);
                },
                Instruction::At(d,s) => {
                    let mut v = vec![];
                    for i in 0..mi_get(&values,s).len() {
                        v.push(i);
                    }
                    mi_ins(&mut values,d,v);
                },
                Instruction::Call(name,types,regs) => {
                    match &name[..] {
                        "assign" => {
                            let n = regs.len()/2;
                            for i in 0..n {
                                let r = context.route.get(&regs[i]).expect(&format!("missing route for {:?}",regs[i]));
                                let v_new = mi_get(&values,&regs[n+i]).to_vec();
                                let x = mi_get(&values,&regs[i]).to_vec();
                                let mut v = mi_get(&values,&r.0).to_vec();
                                mi_ins(&mut values,&r.0,v_new.clone());
                                print!("{:?} route {:?} indexes {:?} data {:?} was {:?}\n",regs[i],r.0,x,v_new,v);
                                let mut v_new_iter = v_new.iter().cycle();
                                for idx in &x {
                                    *v.get_mut(*idx).unwrap() = *v_new_iter.next().unwrap();
                                }
                            }
                        },
                        "print_regs" => {
                            let mut print = Vec::new();
                            for r in regs {
                                let v = mi_get(&values,&r).to_vec();
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
        (printed,values.drain().filter(|(k,_)| k.is_some()).map(|(k,v)| (k.unwrap(),v)).collect())
    }

    fn find_assigns<'a>( instrs: &Vec<Instruction>, subregs: &'a BTreeMap<Register,Linearized>) -> (Vec<&'a Linearized>,Vec<Register>) {
        let mut lin = Vec::new();
        let mut norm = Vec::new();
        for instr in instrs {
            if let Instruction::Call(s,_,vv) = instr {
                if s == "assign" {
                    if let Some(reg) = subregs.get(&vv[1]) {
                        lin.push(reg);
                    } else {
                        norm.push(vv[1].clone());
                    }
                }
            }
        }
        (lin,norm)
    }

    #[test]
    fn linearize_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        linearize_real(&mut context).expect("linearize");
        print!("{:?}\n",context);
        let values = mini_interp(&mut context);
        print!("{:?}",values);
    }

    #[test]
    fn linearize_filter_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke-filter.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        let instrs = context.instrs.clone();
        print!("{:?}\n",context);
        let subregs = linearize_real(&mut context).expect("linearize");
        print!("{:?}\n",context);
        let (_,values) = mini_interp(&mut context);
        let (lins,norms) = find_assigns(&instrs,&subregs);
        print!("{:?}",values);
        assert_eq!(vec![1,2],values[&lins[0].data]);
        assert_eq!(vec![0],values[&lins[0].index[0].0]);
        assert_eq!(vec![2],values[&lins[0].index[0].1]);
        assert_eq!(vec![2],values[&norms[0]]);
        assert_eq!(vec![1,2,3,4,5],values[&lins[1].data]);
        assert_eq!(vec![2],values[&lins[1].index[0].0]);
        assert_eq!(vec![3],values[&lins[1].index[0].1]);
        assert_eq!(vec![3,4,5],values[&norms[1]]);
        assert_eq!(vec![1,2,3,4,5],values[&norms[2]]);
        assert_eq!(Vec::<usize>::new(),values[&norms[3]]);
    }

    #[test]
    fn linearize_reffilter_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke-reffilter.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        let instrs = context.instrs.clone();
        print!("{:?}\n",context);
        let subregs = linearize_real(&mut context).expect("linearize");
        print!("{:?}\n",context);
        let (prints,values) = mini_interp(&mut context);
        let (lins,norms) = find_assigns(&instrs,&subregs);
        assert_eq!(vec![vec![vec![0],vec![2],vec![3,4]]],prints);
    }

    fn linearize_stable_pass() -> GenContext {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize_real(&mut context).expect("linearize");
        print!("{:?}\n",context);
        context
    }

    #[test]
    fn linearize_stable_allocs() {
        let a = linearize_stable_pass();
        let b = linearize_stable_pass();
        assert_eq!(a.instrs,b.instrs);
    }

   #[test]
    fn linearize_push_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke-push.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        let instrs = context.instrs.clone();
        print!("{:?}\n",instrs);
        let subregs = linearize_real(&mut context).expect("linearize");
        let (lins,_) = find_assigns(&instrs,&subregs);
        let (_,values) = mini_interp(&mut context);
        assert_eq!(Vec::<usize>::new(),values[&lins[0].data]);
        assert_eq!(vec![0],values[&lins[0].index[0].0]);
        assert_eq!(vec![0],values[&lins[0].index[0].1]);
        assert_eq!(vec![3],values[&lins[1].data]);
        assert_eq!(vec![0],values[&lins[1].index[0].0]);
        assert_eq!(vec![1],values[&lins[1].index[0].1]);
        assert_eq!(vec![0],values[&lins[1].index[1].0]);
        assert_eq!(vec![1],values[&lins[1].index[1].1]);
        assert_eq!(vec![1],values[&lins[2].data]);
        assert_eq!(vec![0],values[&lins[2].index[0].0]);
        assert_eq!(vec![1],values[&lins[2].index[0].1]);
        assert_eq!(vec![1,2,3,4,5,6],values[&lins[3].data]);
        assert_eq!(vec![0,2,3,6],values[&lins[3].index[0].0]);
        assert_eq!(vec![2,1,3,0],values[&lins[3].index[0].1]);
        assert_eq!(vec![0,2],values[&lins[3].index[1].0]);
        assert_eq!(vec![2,2],values[&lins[3].index[1].1]);
        assert_eq!(vec![0],values[&lins[3].index[2].0]);
        assert_eq!(vec![2],values[&lins[3].index[2].1]);
        assert_eq!(Vec::<usize>::new(),values[&lins[4].data]);
        assert_eq!(vec![0],values[&lins[4].index[0].0]);
        assert_eq!(vec![0],values[&lins[4].index[0].1]);
        assert_eq!(vec![0],values[&lins[4].index[1].0]);
        assert_eq!(vec![1],values[&lins[4].index[1].1]);
        //print!("{:?}",values);
    }
}
