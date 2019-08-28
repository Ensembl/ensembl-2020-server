use std::collections::HashMap;
use super::{ DefStore, Instruction, Register, RegisterAllocator };
use crate::types::TypePass;

/* Denaming early allows us to assume that registers only ever have a single type. This greatly
 * simplifies the type handling in the simplification steps.
 */

struct Dename {
    types: TypePass,
    regalloc: RegisterAllocator,
    regmap: HashMap<String,Register>,
    output: Vec<Instruction>
}

impl Dename {
    fn new(regalloc: &RegisterAllocator) -> Dename {
        Dename {
            types: TypePass::new(),
            regalloc: regalloc.clone(),
            regmap: HashMap::new(),
            output: Vec::new(),
        }
    }

    pub fn replace_regs(&mut self, instr: &Instruction, new: &Vec<Register>) -> Result<Instruction,String> {
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
            Instruction::RefEValue(field,etype,_,_) => Ok(Instruction::RefEValue(field.to_string(),etype.to_string(),new[0].clone(),new[1].clone())),
            Instruction::RefSquare(_,_) => Ok(Instruction::RefSquare(new[0].clone(),new[1].clone())),
            Instruction::RefFilter(_,_,_) => Ok(Instruction::RefFilter(new[0].clone(),new[1].clone(),new[2].clone())),
            Instruction::Operator(name,_,_) => Ok(Instruction::Operator(name.to_string(),new[0].clone(),new[1..].to_vec())),
            Instruction::Copy(_,_) => Ok(Instruction::Copy(new[0].clone(),new[1].clone())),
            Instruction::Ref(_,_) => Ok(Instruction::Ref(new[0].clone(),new[1].clone())),
        }
    }

    fn add(&mut self, ds: &DefStore, instr: &Instruction) -> Result<(),String> {
        self.types.apply_command(instr,ds)?;
        print!("\n{:?}\n",instr);
        let mut regalloc = self.regalloc.clone();
        let mut new_regs = Vec::new();
        for (sig,reg) in self.types.extract_sig_regs(instr,ds)? {
            new_regs.push(if let Register::Named(name) = reg {
                if sig.out {
                    let new = self.regalloc.allocate();
                    self.regmap.insert(name,new.clone());
                    new
                } else {
                    self.regmap.entry(name).or_insert_with(|| regalloc.allocate()).clone()
                }
            } else {
                reg
            })
        }
        let new = self.replace_regs(instr,&new_regs)?;
        self.output.push(new);
        Ok(())
    }

    fn output(self) -> Vec<Instruction> { self.output }
}

pub fn dename(regalloc: &RegisterAllocator, ds: &DefStore, input: &Vec<Instruction>) -> Result<Vec<Instruction>,String> {
    let val = input.to_vec();
    let mut s = Dename::new(regalloc);
    for instr in val {
        s.add(ds,&instr)?;
    }
    Ok(s.output())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::codegen::{ Generator, RegisterAllocator };

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
        let outstrs = dename(&regalloc,&defstore,&instrs).expect("ok");
        let outstrs_str : Vec<String> = outstrs.iter().map(|v| format!("{:?}",v)).collect();
        print!("=====\n\n{}\n",outstrs_str.join(""));
    }
}
