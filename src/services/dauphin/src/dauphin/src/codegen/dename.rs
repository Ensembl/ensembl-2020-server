use std::collections::{ HashMap, HashSet };
use super::{ DefStore, Instruction, Register, RegisterAllocator };
use crate::types::TypePass;

/* Denaming early allows us to assume that registers only ever have a single type. This greatly
 * simplifies the type handling in the simplification steps.
 */

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
        for arg in types.extract_sig_regs(instr,ds)?.iter() {
            if arg.get_type().writeonly {
                if pures.contains(arg.get_register()) {
                    writeonly.insert(arg.get_register().clone());
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
        for arg in types.extract_sig_regs(instr,ds)?.iter() {
            if let Instruction::Ref(referer,referee) = instr {
                if writeonly.contains(referer) {
                    mapping.remove(referee);
                }
            }
            if let Some(new) = mapping.get(arg.get_register()) {
                new_regs.push(new.clone());
            } else {
                if let Register::Named(_) = arg.get_register() {
                    let new_reg = regalloc.allocate();
                    mapping.insert(arg.get_register().clone(),new_reg.clone());
                    new_regs.push(new_reg);
                } else {
                    new_regs.push(arg.get_register().clone());
                }
            }
        }
        out.push(instr.replace_regs(&new_regs)?);
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

    #[test]
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
