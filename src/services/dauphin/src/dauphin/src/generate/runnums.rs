use std::collections::{ HashMap, HashSet };
use super::gencontext::GenContext;
use crate::model::Register;
use crate::generate::{ Instruction, InstructionType };

fn update_values(values: &mut HashMap<Register,Vec<f64>>, changing: &[usize], instr: &Instruction) {
    match &instr.itype {
        InstructionType::Nil => {
            values.insert(instr.regs[0],vec![]);
        },

        InstructionType::Copy => {
            if let Some(src) = values.get(&instr.regs[1]).cloned() {
                values.insert(instr.regs[0],src.to_vec());
            }
        },

        InstructionType::Append => {
            if let Some(src) = values.get(&instr.regs[1]) {
                let value = src.to_vec();
                values.get_mut(&instr.regs[0]).unwrap().extend(value.iter());
            } else {
                values.remove(&instr.regs[0]);
            }
        }

        InstructionType::NumberConst(n) => {
            values.insert(instr.regs[0],vec![*n]);
        },

        InstructionType::Const(nn) => {
            values.insert(instr.regs[0],nn.to_vec());
        },

        InstructionType::At => {
            if let Some(src) = values.get(&instr.regs[1]) {
                let mut value = vec![];
                for i in 0..src.len() {
                    value.push(i as f64);
                }
                values.insert(instr.regs[0],value);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::Filter => {
            if let (Some(src),Some(filter)) = (values.get(&instr.regs[1]),values.get(&instr.regs[2])) {
                let mut dst = vec![];
                let mut f = filter.iter();
                for u in src {
                    if *f.next().unwrap() > 0. {
                        dst.push(*u);
                    }
                }
                values.insert(instr.regs[0],dst);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::Run => {
            if let (Some(src),Some(filter)) = (values.get(&instr.regs[1]),values.get(&instr.regs[2])) {
                let mut dst = vec![];
                let mut b_iter = filter.iter();
                for a in src.iter() {
                    let b = b_iter.next().unwrap();
                    for i in 0..*b as usize {
                        dst.push(a+i as f64);
                    }
                }
                values.insert(instr.regs[0],dst);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::SeqFilter => {
            if let (Some(src),Some(start),Some(len)) = (values.get(&instr.regs[1]),values.get(&instr.regs[2]),values.get(&instr.regs[2])) {
                let mut dst = vec![];
                let mut b_iter = len.iter();
                for a in start.iter() {
                    let b = b_iter.next().unwrap();
                    for i in 0..*b as usize {
                        dst.push(src[*a as usize+i]);
                    }
                }
                values.insert(instr.regs[0],dst);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::NumEq => {
            if let (Some(aa),Some(bb)) = (values.get(&instr.regs[1]),values.get(&instr.regs[2])) {
                let mut dst = vec![];
                let mut b_iter = bb.iter().cycle();
                for a in aa {
                    let b = b_iter.next().unwrap();
                    dst.push(if *a == *b {1.} else {0.});
                }
                values.insert(instr.regs[0],dst);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::Length => {
            if let Some(src) = values.get(&instr.regs[1]).cloned() {
                values.insert(instr.regs[0],vec![src.len() as f64]);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::Add => {
            if let (Some(dst),Some(delta)) = (values.get(&instr.regs[0]),values.get(&instr.regs[1])) {
                let mut out = vec![];
                for (i,input) in dst.iter().enumerate() {
                    out.push(input+delta[i%delta.len()]);
                }
                values.insert(instr.regs[0],out);
            } else {
                values.remove(&instr.regs[0]);
            }                
        },

        InstructionType::SeqAt => {
            if let Some(src) = values.get(&instr.regs[1]).cloned() {
                let mut out = vec![];
                for b_val in &src {
                    for i in 0..*b_val as usize {
                        out.push(i as f64);
                    }
                }
                values.insert(instr.regs[0],out);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        _ => {
            for idx in changing {
                values.remove(&instr.regs[*idx]);
            }
        }
    }
}

fn all_known(values: &HashMap<Register,Vec<f64>>, changing: &[usize], instr: &Instruction) -> bool {
    let mut out = true;
    for i in changing {
        if !values.contains_key(&instr.regs[*i]) {
            out = false;
        }
    }
    out
}

pub fn run_nums(context: &mut GenContext) {
    let mut values = HashMap::new();
    let mut suppressed = HashSet::new();
    for instr in &context.get_instructions() {
        let changing = instr.itype.changing_registers(context.get_defstore());
        print!("{:?}\n",instr);
        update_values(&mut values,&changing,instr);
        print!("{:?}\n",values);
        if all_known(&values,&changing,instr) && !instr.itype.self_justifying_call() {
            print!("ALL KNOWN\n");
            for i in changing {
                suppressed.insert(&instr.regs[i]);
            }
        } else {
            for reg in &instr.regs {
                if suppressed.contains(reg) {
                    context.add(Instruction::new(InstructionType::Const(values.get(reg).unwrap().to_vec()),vec![*reg]));
                    suppressed.remove(reg);
                }
            }
            context.add(instr.clone());
        }

    }
    context.phase_finished();
}


#[cfg(test)]
mod test {
    use super::*;
    use super::super::call;
    use super::super::simplify::simplify;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::generate::generate_code;
    use crate::interp::mini_interp;
    use super::super::linearize;
    use super::super::remove_aliases;

    // XXX test pruning, eg fewer lines
    #[test]
    fn runnums_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-refsquare.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        print!("{:?}",context);
        run_nums(&mut context);
        print!("RUN NUMS\n");
        print!("{:?}",context);
        let (_prints,values,strings) = mini_interp(&defstore,&mut context);
        print!("{:?}\n",values);
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }
}