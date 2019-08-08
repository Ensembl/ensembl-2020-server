use std::collections::HashMap;
use super::{ DefStore, Instruction, Register, RegisterAllocator };

struct Simplify {
    name: String,
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
            output: Vec::new()
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

    fn ctor_struct(&mut self, ds: &DefStore, out: &Register, ins: &Vec<Register>) {
        let map = self.get_regs(ds,out).to_vec();
        let mut map_iter = map.iter();
        for in_reg in ins {
            let out_reg = map_iter.next().unwrap();
            self.output.push(Instruction::Copy(out_reg.clone(),in_reg.clone()));
        }
    }

    fn add(&mut self, ds: &DefStore, input: &Instruction) {
        match input {
            Instruction::CtorStruct(name,out,ins) if name == &self.name =>
                self.ctor_struct(ds,out,ins),
            default =>
                self.output.push(default.clone())
        };
    }

    fn output(self) -> Vec<Instruction> { self.output }
}

pub fn simplify(regalloc: &RegisterAllocator, ds: &DefStore, input: &Vec<Instruction>) -> Vec<Instruction> {
    let mut val = input.to_vec();
    for elim in ds.get_structenum_order().rev() {
        let mut s = Simplify::new(regalloc,elim);
        for instr in val {
            s.add(ds,&instr);
        }
        val = s.output();
    }
    val
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
        let outstrs = simplify(&regalloc,&defstore,&instrs);
        let outstrs_str : Vec<String> = outstrs.iter().map(|v| format!("{:?}",v)).collect();
        print!("=====\n\n{}\n",outstrs_str.join(""));
    }
}
