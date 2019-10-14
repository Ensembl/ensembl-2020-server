// TODO Copy for registers
use std::collections::HashMap;

use crate::model::Register;
use crate::typeinf::{ BaseType, MemberType };
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

fn allocate_subregs(context: &mut GenContext) -> HashMap<Register,Linearized> {
    let mut targets = Vec::new();
    for (reg,type_) in context.types.each_register() {
        let depth = type_.depth();
        if depth > 0 {
            targets.push((reg.clone(),type_.clone(),depth));
        }
    }
    let mut out = HashMap::new();
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

fn push_copy_level(out: &mut Vec<Instruction>, context: &mut GenContext, lin_dst: &Linearized, lin_src: &Linearized, level: usize) {
    let offset = tmp_number_reg(context);
    if level == 0 {
        out.push(Instruction::Length(offset.clone(),lin_dst.data.clone()));
    } else {
        out.push(Instruction::Length(offset.clone(),lin_dst.index[level-1].0.clone()));
    }
    let tmp = tmp_number_reg(context);
    out.push(Instruction::Copy(tmp.clone(),lin_src.index[level].0.clone()));
    out.push(Instruction::Add(tmp.clone(),offset.clone()));
    out.push(Instruction::Append(lin_dst.index[level].0.clone(),tmp));
    out.push(Instruction::Append(lin_dst.index[level].1.clone(),lin_src.index[level].1.clone()));
}

fn push_top(out: &mut Vec<Instruction>, context: &mut GenContext, lin_dst: &Linearized, lin_src: &Linearized, level: usize) {
    let dst_len = tmp_number_reg(context);
    out.push(Instruction::Length(dst_len.clone(),lin_dst.index[level-1].0.clone()));
    let src_len = tmp_number_reg(context);
    out.push(Instruction::Length(src_len.clone(),lin_src.index[level-1].0.clone()));
    out.push(Instruction::Push(lin_dst.index[level].0.clone(),dst_len));
    out.push(Instruction::Push(lin_dst.index[level].1.clone(),src_len));
}

fn linear_extend<F>(subregs: &HashMap<Register,Linearized>, dst: &Register, src: &Register, mut cb: F)
        where F: FnMut(&Register,&Register) {
    if let Some(lin_src) = subregs.get(src) {
        let lin_dst = subregs.get(dst).unwrap();
        cb(&lin_dst.data,&lin_src.data);
        for level in 0..lin_src.index.len() {
            cb(&lin_dst.index[level].0,&lin_src.index[level].0);
            cb(&lin_dst.index[level].1,&lin_src.index[level].1);
        }
    } else {
        cb(dst,src);
    }
}

fn linearize_one(out: &mut Vec<Instruction>, context: &mut GenContext, subregs: &HashMap<Register,Linearized> , instr: &Instruction) -> Result<(),String> {
    match instr {
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
        Instruction::Push(dst,src) => {
            let lin_dst = subregs.get(dst).ok_or_else(|| format!("Missing info for register {:?}",dst))?;
            if let Some(lin_src) = subregs.get(src) {
                push_top(out,context,lin_dst,lin_src,lin_src.index.len());
                for level in (0..lin_src.index.len()).rev() {
                    push_copy_level(out,context,lin_dst,lin_src,level);
                }
                out.push(Instruction::Append(lin_dst.data.clone(),lin_src.data.clone()));
            } else {
                let dst_len = tmp_number_reg(context);
                out.push(Instruction::Length(dst_len.clone(),lin_dst.data.clone()));
                out.push(Instruction::Push(lin_dst.index[0].0.clone(),dst_len));
                out.push(Instruction::Append(lin_dst.data.clone(),src.clone()));
                let src_len = tmp_number_reg(context);
                out.push(Instruction::Length(src_len.clone(),src.clone()));
                out.push(Instruction::Push(lin_dst.index[0].1.clone(),src_len));
            }
        },
        Instruction::Nil(_) => {
            out.push(instr.clone());
        },
        Instruction::Copy(dst,src) => {
            linear_extend(subregs,dst,src, |d,s| {
                out.push(Instruction::Copy(d.clone(),s.clone()));
            });
        },
        Instruction::Ref(dst,src) => {
            linear_extend(subregs,dst,src, |d,s| {
                out.push(Instruction::Ref(d.clone(),s.clone()));
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
        Instruction::Append(_,_) | Instruction::Length(_,_) | Instruction::Add(_,_) => {
            return Err(format!("Bad instruction {:?}",instr.clone()));
        }
        _ => {} // XXX
    };
    Ok(())
}

fn linearize_real(context: &mut GenContext) -> Result<HashMap<Register,Linearized>,String> {
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
    use super::*;
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

    fn mini_interp(context: &GenContext) -> HashMap<Register,Vec<usize>> {
        let mut values : HashMap<Option<Register>,Vec<usize>> = HashMap::new();
        values.insert(None,vec![]);
        for instr in &context.instrs {
            for r in instr.get_registers() {
                print!("{:?}={:?}\n",r,mi_get(&values,&r));
            }
            print!("{:?}\n",instr);
            match instr {
                Instruction::Nil(r) => { mi_ins(&mut values,r,vec![]); },
                Instruction::Push(r,s) => { let x = mi_mut(&mut values,s)[0]; mi_mut(&mut values,r).push(x); },
                Instruction::Append(r,s) => { let mut x = mi_mut(&mut values,s).to_vec(); mi_mut(&mut values,r).append(&mut x); },
                Instruction::Add(r,v) => { let v = mi_get(&values,r).iter().map(|x| x+mi_get(&values,v)[0]).collect(); mi_ins(&mut values,&r,v); },
                Instruction::Length(r,s) => { let v = vec![mi_get(&values,s).len()]; mi_ins(&mut values,&r,v); }
                Instruction::NumberConst(r,n) => { mi_ins(&mut values,&r,vec![*n as usize]); },
                Instruction::BooleanConst(r,n) => { mi_ins(&mut values,&r,vec![if *n {1} else {0}]); },
                Instruction::Copy(r,s) => { let x = mi_mut(&mut values,s).to_vec(); mi_ins(&mut values,&r,x); },
                Instruction::Ref(_,_) => { /* Hmmm, ok for now */ },
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
                Instruction::SeqFilter(d,s,a,b) => {
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
                _ => { panic!("Bad mini-interp instruction {:?}",instr); }
            }
            for r in instr.get_registers() {
                print!("{:?}={:?}\n",r,mi_get(&values,&r));
            }
        }
        values.drain().filter(|(k,_)| k.is_some()).map(|(k,v)| (k.unwrap(),v)).collect()
    }

    fn find_assigns<'a>( instrs: &Vec<Instruction>, subregs: &'a HashMap<Register,Linearized>) -> Vec<&'a Linearized> {
        let mut out = Vec::new();
        for instr in instrs {
            if let Instruction::Proc(s,vv) = instr {
                if s == "assign" {
                    out.push(&subregs[&vv[1]]);
                }
            }
        }
        out
    }

    #[test]
    fn linearize_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        linearize_real(&mut context).expect("linearize");
        print!("{:?}\n",context);
        let values = mini_interp(&mut context);
        print!("{:?}",values);
    }

    fn linearize_stable_pass() -> GenContext {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
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
        simplify(&defstore,&mut context).expect("k");
        let instrs = context.instrs.clone();
        let subregs = linearize_real(&mut context).expect("linearize");
        let lins = find_assigns(&instrs,&subregs);
        let values = mini_interp(&mut context);
        assert_eq!(Vec::<usize>::new(),values[&lins[0].data]);
        assert_eq!(Vec::<usize>::new(),values[&lins[0].index[0].0]);
        assert_eq!(Vec::<usize>::new(),values[&lins[0].index[0].1]);
        assert_eq!(vec![3],values[&lins[1].data]);
        assert_eq!(vec![0],values[&lins[1].index[0].0]);
        assert_eq!(vec![1],values[&lins[1].index[0].1]);
        assert_eq!(vec![0],values[&lins[1].index[1].0]);
        assert_eq!(vec![1],values[&lins[1].index[1].1]);
        assert_eq!(vec![1],values[&lins[2].data]);
        assert_eq!(vec![0],values[&lins[2].index[0].0]);
        assert_eq!(vec![1],values[&lins[2].index[0].1]);
        assert_eq!(vec![1,2,3,4,5,6],values[&lins[3].data]);
        assert_eq!(vec![0,1,2,3,4,5],values[&lins[3].index[0].0]);
        assert_eq!(vec![1,1,1,1,1,1],values[&lins[3].index[0].1]);
        assert_eq!(vec![0,2,3,6],values[&lins[3].index[1].0]);
        assert_eq!(vec![2,1,3,0],values[&lins[3].index[1].1]);
        assert_eq!(vec![0,2],values[&lins[3].index[2].0]);
        assert_eq!(vec![2,2],values[&lins[3].index[2].1]);
        assert_eq!(Vec::<usize>::new(),values[&lins[4].data]);
        assert_eq!(Vec::<usize>::new(),values[&lins[4].index[0].0]);
        assert_eq!(Vec::<usize>::new(),values[&lins[4].index[0].1]);
        assert_eq!(vec![0],values[&lins[4].index[1].0]);
        assert_eq!(vec![0],values[&lins[4].index[1].1]);
        //print!("{:?}",values);
    }
}
