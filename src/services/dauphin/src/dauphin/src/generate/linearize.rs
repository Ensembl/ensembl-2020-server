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

fn linearize_one(out: &mut Vec<Instruction>, context: &mut GenContext, subregs: &HashMap<Register,Linearized> , instr: &Instruction) -> Result<(),()> {
    match instr {
        Instruction::NumberConst(_,_) |
        Instruction::BooleanConst(_,_) |
        Instruction::StringConst(_,_) | 
        Instruction::BytesConst(_,_) => out.push(instr.clone()),
        Instruction::List(r) => {
            let lin = subregs.get(r).ok_or_else(|| ())?;
            out.push(Instruction::Nil(lin.data.clone()));
            for (start,len) in &lin.index {
                out.push(Instruction::Nil(start.clone()));
                out.push(Instruction::Nil(len.clone()));
            }
        },
        Instruction::Push(dst,src) => {
            let lin_dst = subregs.get(dst).ok_or_else(|| ())?;
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
        _ => {} // XXX
    };
    Ok(())
}

fn linearize_real(context: &mut GenContext) -> Result<HashMap<Register,Linearized>,()> {
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

pub fn linearize(context: &mut GenContext) -> Result<(),()> {
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

    fn mini_interp(context: &GenContext) -> HashMap<Register,Vec<usize>> {
        let mut values : HashMap<Register,Vec<usize>> = HashMap::new();
        for instr in &context.instrs {
            match instr {
                Instruction::Nil(r) => { values.insert(r.clone(),vec![]); },
                Instruction::Push(r,s) => { let x = values[s][0]; values.get_mut(r).unwrap().push(x); },
                Instruction::Append(r,s) => { let mut x = values[s].to_vec(); values.get_mut(r).unwrap().append(&mut x); },
                Instruction::Add(r,v) => { values.insert(r.clone(),values[r].iter().map(|x| x+values[v][0]).collect()); },
                Instruction::Length(r,s) => { values.insert(r.clone(),vec![values[s].len()]); }
                Instruction::NumberConst(r,n) => { values.insert(r.clone(),vec![*n as usize]); },
                Instruction::Copy(r,s) => { let x = values[s].to_vec(); values.insert(r.clone(),x); }
                _ => ()
            }
        }
        values
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
        //print!("{:?}",values);
    }

   #[test]
    fn linearize_smoke_push() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke-push.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        let instrs = context.instrs.clone();
        let subregs = linearize_real(&mut context).expect("linearize");
        let lins = find_assigns(&instrs,&subregs);
        //print!("{:?}\n",context);
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
