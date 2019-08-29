use std::collections::HashMap;
use super::{ DefStore, Instruction, Register, RegisterAllocator };
use crate::types::TypePass;

struct Simplify {
    name: String,
    types: TypePass,
    regalloc: RegisterAllocator,
    regmap: HashMap<Register,Vec<Register>>,
    output: Vec<Instruction>
}

impl Simplify {
    fn new(regalloc: &RegisterAllocator, name: &str) -> Simplify {
        Simplify {
            name: name.to_string(),
            regalloc: regalloc.clone(),
            regmap: HashMap::new(),
            output: Vec::new(),
            types: TypePass::new(false)
        }
    }

    fn get_regs(&mut self, ds: &DefStore, src: &Register) -> &Vec<Register> {
        if !self.regmap.contains_key(src) {
            let struct_ = ds.get_struct(&self.name).unwrap();
            let regs = (0..struct_.get_names().len()).map(|_|
                self.regalloc.allocate()
            ).collect();
            self.regmap.insert(src.clone(),regs);
        }
        self.regmap.get(src).unwrap()
    }

    fn ctor_struct(&mut self, ds: &DefStore, out: &Register, ins: &Vec<Register>) -> Vec<Instruction> {
        let mut ret = vec![];
        let map = self.get_regs(ds,out).to_vec();
        let mut map_iter = map.iter();
        for in_reg in ins {
            let out_reg = map_iter.next().unwrap();
            ret.push(Instruction::Copy(out_reg.clone(),in_reg.clone()));
        }
        ret
    }

    fn horizontal(&mut self, regs: &Vec<Register>) -> Vec<Register> {
        let mut output = Vec::new();
        for input in regs {
            if let Some(ref repls) = self.regmap.get(input) {
                output.extend(repls.iter().cloned());
            } else {
                output.push(input.clone());
            }
        }
        output
    }

    fn add(&mut self, ds: &DefStore, input: &Instruction) -> Result<(),String> {
        let mut instrs = match input {
            Instruction::CtorStruct(name,out,ins) if name == &self.name => {
                self.types.apply_command(input,ds)?;
                self.ctor_struct(ds,out,ins)
            },
            Instruction::SValue(field,name,out,in_) if *name == self.name => {
                let struct_ = ds.get_struct(&name).unwrap();
                let pos = struct_.get_names().iter().position(|v| v == field).unwrap();
                let inreg = self.get_regs(ds,in_).iter().enumerate().find(|(i,_)| *i==pos).map(|(_,v)| v).unwrap().clone();
                vec![Instruction::Copy(out.clone(),inreg)]
            },
            Instruction::Proc(name,ref regs) => {
                // XXX to avoid type checking of instruction revealing longer arg list
                let instr = Instruction::Proc(name.to_string(),self.horizontal(regs)).clone();
                self.output.push(instr);
                vec![]
            },
            Instruction::Copy(dest,src) => {
                if let Some(src_repls) = self.regmap.get(src).cloned() {
                    let mut out = Vec::new();
                    let mut dst_repls = self.get_regs(ds,dest).iter();
                    for sub_src in src_repls.iter() {
                        let sub_dst = dst_repls.next().unwrap();
                        out.push(Instruction::Copy(sub_dst.clone(),sub_src.clone()));
                    }
                    out
                } else {
                    self.types.apply_command(input,ds)?;
                    vec![Instruction::Copy(dest.clone(),src.clone())]
                }
            },
            default => {
                self.types.apply_command(input,ds)?;
                vec![default.clone()]
            }
        };
        for instr in &instrs {
            self.types.apply_command(instr,ds)?;
        }
        self.output.extend(instrs.drain(..));
        print!("\n{:?}\n{:?}\n",self.types,self.output);
        Ok(())
    }

    fn output(self) -> Vec<Instruction> { self.output }
}

pub fn simplify(regalloc: &RegisterAllocator, ds: &DefStore, input: &Vec<Instruction>) -> Result<Vec<Instruction>,String> {
    let mut val = input.to_vec();
    for elim in ds.get_structenum_order().rev() {
        let mut s = Simplify::new(regalloc,elim);
        for instr in val {
            s.add(ds,&instr)?;
        }
        val = s.output();
    }
    Ok(val)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::codegen::{ Generator, RegisterAllocator };

    #[test]
    fn simplify_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/simplify-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let regalloc = RegisterAllocator::new();
        let gen = Generator::new(&regalloc);
        let instrs : Vec<Instruction> = gen.go(&defstore,stmts).expect("codegen");
        let instrs_str : Vec<String> = instrs.iter().map(|v| format!("{:?}",v)).collect();
        print!("{}\n",instrs_str.join(""));
        let outstrs = simplify(&regalloc,&defstore,&instrs).expect("ok");
        let outstrs_str : Vec<String> = outstrs.iter().map(|v| format!("{:?}",v)).collect();
        print!("=====\n\n{}\n",outstrs_str.join(""));
    }
}
