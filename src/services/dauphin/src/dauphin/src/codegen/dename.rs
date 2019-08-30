use std::collections::{ HashMap, HashSet };
use super::{ DefStore, Instruction, Register, RegisterAllocator };
use crate::types::TypePass;

/* Denaming early allows us to assume that registers only ever have a single type. This greatly
 * simplifies the type handling in the simplification steps.
 */

pub fn replace_regs(instr: &Instruction, new: &Vec<Register>) -> Result<Instruction,String> {
    match instr {
        Instruction::Proc(name,_) => Ok(Instruction::Proc(name.to_string(),new.clone())),
        Instruction::NumberConst(_,c) => Ok(Instruction::NumberConst(new[0].clone().clone(),*c)),
        Instruction::BooleanConst(_,c) => Ok(Instruction::BooleanConst(new[0].clone(),*c)),
        Instruction::StringConst(_,c) => Ok(Instruction::StringConst(new[0].clone(),c.clone())),
        Instruction::BytesConst(_,c) => Ok(Instruction::BytesConst(new[0].clone(),c.clone())),
        Instruction::List(_) => Ok(Instruction::List(new[0].clone())),
        Instruction::Star(_,_) => Ok(Instruction::Star(new[0].clone(),new[1].clone())),
        Instruction::Square(_,_) => Ok(Instruction::Square(new[0].clone(),new[1].clone())),
        Instruction::At(_,_) => Ok(Instruction::At(new[0].clone(),new[1].clone())),
        Instruction::Filter(_,_,_) => Ok(Instruction::Filter(new[0].clone(),new[1].clone(),new[2].clone())),
        Instruction::Push(_,_) => Ok(Instruction::Push(new[0].clone(),new[1].clone())),
        Instruction::CtorEnum(name,branch,_,_) => Ok(Instruction::CtorEnum(name.to_string(),branch.to_string(),new[0].clone(),new[1].clone())),
        Instruction::CtorStruct(name,_,_) => Ok(Instruction::CtorStruct(name.to_string(),new[0].clone(),new[1..].to_vec())),
        Instruction::SValue(field,stype,_,_) => Ok(Instruction::SValue(field.to_string(),stype.to_string(),new[0].clone(),new[1].clone())),
        Instruction::EValue(field,etype,_,_) => Ok(Instruction::EValue(field.to_string(),etype.to_string(),new[0].clone(),new[1].clone())),
        Instruction::ETest(field,etype,_,_) => Ok(Instruction::ETest(field.to_string(),etype.to_string(),new[0].clone(),new[1].clone())),
        Instruction::RefSValue(field,stype,_,_) => Ok(Instruction::RefSValue(field.to_string(),stype.to_string(),new[0].clone(),new[1].clone())),
        Instruction::RefSValue2(field,stype,_,_) => Ok(Instruction::RefSValue2(field.to_string(),stype.to_string(),new[0].clone(),new[1].clone())),
        Instruction::RefEValue(field,etype,_,_) => Ok(Instruction::RefEValue(field.to_string(),etype.to_string(),new[0].clone(),new[1].clone())),
        Instruction::RefSquare(_,_) => Ok(Instruction::RefSquare(new[0].clone(),new[1].clone())),
        Instruction::RefFilter(_,_,_) => Ok(Instruction::RefFilter(new[0].clone(),new[1].clone(),new[2].clone())),
        Instruction::Operator(name,_,_) => Ok(Instruction::Operator(name.to_string(),new[0].clone(),new[1..].to_vec())),
        Instruction::Copy(_,_) => Ok(Instruction::Copy(new[0].clone(),new[1].clone())),
        Instruction::Ref(_,_) => Ok(Instruction::Ref(new[0].clone(),new[1].clone())),
        Instruction::Set(_,_) => Ok(Instruction::Set(new[0].clone(),new[1].clone())),
    }
}

/* A pure register is a direct reference to a named variable */
fn find_pure(input: &Vec<Instruction>) -> Result<HashSet<Register>,String> {
    let mut pures = HashSet::new();
    for instr in input {
        if let Instruction::Ref(dest,Register::Named(_)) = instr {
            pures.insert(dest.clone());
        }
    }
    Ok(pures)
}

/* A write-only register is a pure register used in an out context */
fn find_writeonly(ds: &DefStore, input: &Vec<Instruction>, pures: &HashSet<Register>) -> Result<HashSet<Register>,String> {
    let mut types = TypePass::new(true);
    let mut writeonly = HashSet::new();
    for instr in input {
        types.apply_command(instr,ds)?;
        print!("{:?} -> {:?}\n",instr,types.extract_sig_regs(instr,ds)?);
        for (sig,reg) in types.extract_sig_regs(instr,ds)?.iter() {
            if sig.out {
                if pures.contains(reg) {
                    writeonly.insert(reg.clone());
                }
            }
        }
    }
    Ok(writeonly)
}

/* A ref into a write-only register creates a new referee to allow type changing */
fn rename(ds: &DefStore, regalloc: &RegisterAllocator, input: &Vec<Instruction>, writeonly: &HashSet<Register>) -> Result<Vec<Instruction>,String> {
    let mut out = Vec::new();
    let mut types = TypePass::new(true);
    let mut mapping : HashMap<Register,Register> = HashMap::new();
    for instr in input {
        types.apply_command(instr,ds)?;
        let mut new_regs = Vec::new();
        for (_,reg) in types.extract_sig_regs(instr,ds)?.iter() {
            if let Instruction::Ref(referer,referee) = instr {
                if writeonly.contains(referer) {
                    mapping.remove(referee);
                }
            }
            if let Some(new) = mapping.get(reg) {
                new_regs.push(new.clone());
            } else {
                if let Register::Named(_) = reg {
                    let new_reg = regalloc.allocate();
                    mapping.insert(reg.clone(),new_reg.clone());
                    new_regs.push(new_reg);
                } else {
                    new_regs.push(reg.clone());
                }
            }
        }
        out.push(replace_regs(instr,&new_regs)?);
    }
    Ok(out)
}

pub fn dename(regalloc: &RegisterAllocator, ds: &DefStore, input: &Vec<Instruction>) -> Result<Vec<Instruction>,String> {
    let pures = find_pure(input)?;
    let writeonly = find_writeonly(ds,input,&pures)?;
    let out = rename(ds,regalloc,input,&writeonly)?;
    print!("pure={:?} writeonly={:?}\n",pures,writeonly);
    Ok(out)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::codegen::{ Generator, RegisterAllocator };

    fn run_types(instrs: &Vec<Instruction>, defstore: &DefStore, allow_typechange: bool) -> Result<(),String> {
        let mut pass = TypePass::new(allow_typechange);
        for instr in instrs {
            pass.apply_command(instr,defstore)?;
        }
        print!("{:?}",pass);
        Ok(())
    }

    //#[test]
    fn dename_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/dename-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let regalloc = RegisterAllocator::new();
        let gen = Generator::new(&regalloc);
        let instrs : Vec<Instruction> = gen.go(&defstore,stmts).expect("dename");
        let instrs_str : Vec<String> = instrs.iter().map(|v| format!("{:?}",v)).collect();
        print!("{}\n",instrs_str.join(""));
        run_types(&instrs,&defstore,true).expect("A");
        run_types(&instrs,&defstore,false).expect_err("B");
        let outstrs = dename(&regalloc,&defstore,&instrs).expect("ok");
        let outstrs_str : Vec<String> = outstrs.iter().map(|v| format!("{:?}",v)).collect();
        print!("=====\n\n{}\n",outstrs_str.join(""));
        run_types(&outstrs,&defstore,true).expect("C");
        run_types(&outstrs,&defstore,false).expect("D");
    }
}
