use crate::codegen::Instruction;
use super::typeinf::{ TypeInf };
use super::typestep::try_apply_command;
use crate::codegen::Register;
use crate::codegen::DefStore;
use super::uniquifier::Uniquifier;
use crate::lexer::{ FileResolver, Lexer };
use crate::parser::{ Sig, TypeSig, TypeSigExpr, parse_signature, BaseType };

fn sig_gen(sig: &str) -> Result<Sig,String> {
    let resolver = FileResolver::new();
    let mut lexer = Lexer::new(resolver);
    lexer.import(&format!("data: {}",sig)).ok();
    parse_signature(&mut lexer).map_err(|_| "internal sig parsing failed".to_string())
}

#[derive(Clone)]
pub struct TypePass {
    uniquifier: Uniquifier,
    typeinf: TypeInf
}

impl TypePass {
    pub fn new() -> TypePass {
        TypePass {
            uniquifier: Uniquifier::new(),
            typeinf: TypeInf::new()
        }
    }

    pub fn get_typeinf(&self) -> &TypeInf { &self.typeinf }

    // TODO move Result into get methods
    fn extract_proc_sig_regs(name: &str, defstore: &DefStore, regs: &Vec<Register>) -> Result<Vec<(Sig,Register)>,String> {
        let procdecl = defstore.get_proc(name).ok_or_else(|| format!("No such procedure {:?}",name))?;
        let sigs = procdecl.sigs();
        if regs.len() != sigs.len() {
            return Err(format!("Incorrect number of arguments to {}",name));
        }
        let mut out = Vec::new();
        for (i,s) in sigs.iter().enumerate() {
            out.push((s.clone(),regs[i].clone()));
        }
        Ok(out)
    }

    fn extract_sig_regs(instr: &Instruction, defstore: &DefStore) -> Result<Vec<(Sig,Register)>,String> {
        match instr {
            Instruction::Proc(name,regs) => TypePass::extract_proc_sig_regs(name,defstore,regs),
            Instruction::NumberConst(reg,_) => Ok(vec![(sig_gen("out number")?,reg.clone())]),
            Instruction::BooleanConst(reg,_) => Ok(vec![(sig_gen("out boolean")?,reg.clone())]),
            Instruction::StringConst(reg,_) => Ok(vec![(sig_gen("out string")?,reg.clone())]),
            Instruction::BytesConst(reg,_) => Ok(vec![(sig_gen("out bytes")?,reg.clone())]),
            Instruction::List(reg) => Ok(vec![(sig_gen("out vec(_)")?,reg.clone())]),
            Instruction::Star(dst,src) => Ok(vec![
                (sig_gen("out vec(_A)")?,dst.clone()),
                (sig_gen("_A")?,src.clone())
            ]),
            Instruction::Square(dst,src) => Ok(vec![
                (sig_gen("out _A")?,dst.clone()),
                (sig_gen("vec(_A)")?,src.clone())
            ]),
            Instruction::Star(dst,src) => Ok(vec![
                (sig_gen("out _A")?,dst.clone()),
                (sig_gen("vec(_A)")?,src.clone())
            ]),
            Instruction::Filter(dst,src,filter) => Ok(vec![
                (sig_gen("out _A")?,dst.clone()),
                (sig_gen("_A")?,src.clone()),
                (sig_gen("boolean")?,filter.clone()),
            ]),
            Instruction::Push(dst,src) => Ok(vec![
                (sig_gen("out vec(_A)")?,dst.clone()),
                (sig_gen("_A")?,src.clone()),
            ]),
            Instruction::CtorEnum(name,branch,dst,src) => {
                let exprdecl = defstore.get_enum(name).ok_or_else(|| format!("No such enum {:?}",name))?;
                let base = BaseType::IdentifiedType(name.to_string());
                let expr = TypeSigExpr::Base(base);
                let branch_typedef = exprdecl.get_branch_type(branch)
                        .ok_or_else(|| format!("No such enum branch {:?}",name))?
                        .to_typesigexpr();
                let dst_sig = Sig {
                    lvalue: Some(expr),
                    out: true,
                    typesig: TypeSig::Right(TypeSigExpr::Placeholder("_".to_string()))
                };
                let src_sig = Sig {
                    lvalue: None,
                    out: false,
                    typesig: TypeSig::Right(branch_typedef)
                };
                Ok(vec![
                    (dst_sig,dst.clone()),
                    (src_sig,src.clone()),
                ])
            },
            Instruction::CtorStruct(name,dst,srcs) => {
                let mut out = Vec::new();
                let exprdecl = defstore.get_struct(name).ok_or_else(|| format!("No such struct {:?}",name))?;
                let base = BaseType::IdentifiedType(name.to_string());
                let expr = TypeSigExpr::Base(base);
                let dst_sig = Sig {
                    lvalue: Some(expr),
                    out: true,
                    typesig: TypeSig::Right(TypeSigExpr::Placeholder("_".to_string()))
                };
                let intypes : Vec<TypeSigExpr> = exprdecl.get_member_types().iter()
                                .map(|x| x.to_typesigexpr()).collect();
                if srcs.len() != intypes.len() {
                    return Err("Incorrect number of arguments".to_string());
                }
                for (i,intype) in intypes.iter().enumerate() {
                    out.push((
                        Sig {
                            lvalue: None, out: false,
                            typesig: TypeSig::Right(intype.clone())
                        },
                        srcs.get(i).unwrap().clone()
                    ));
                }
                out.push((dst_sig,dst.clone()));
                Ok(out)
            },
            Instruction::Operator(name,dst,srcs) => {
                let mut out = Vec::new();
                let exprdecl = defstore.get_func(name).ok_or_else(|| format!("No such function {:?}",name))?;
                let dst_sig = Sig {
                    lvalue: Some(exprdecl.get_dst().clone()),
                    out: true,
                    typesig: TypeSig::Right(TypeSigExpr::Placeholder("_".to_string()))
                };
                let intypes : &Vec<TypeSigExpr> = exprdecl.get_srcs();
                if srcs.len() != intypes.len() {
                    return Err("Incorrect number of arguments".to_string());
                }
                for (i,intype) in intypes.iter().enumerate() {
                    out.push((
                        Sig {
                            lvalue: None, out: false,
                            typesig: TypeSig::Right(intype.clone())
                        },
                        srcs.get(i).unwrap().clone()
                    ));
                }
                out.push((dst_sig,dst.clone()));
                Ok(out)
            },
            _ => Err(format!("no signature for {:?}",instr))
        }
    }

    /* ref is special as the root of all leftyness! */
    pub fn try_apply_ref(&mut self, dst: &Register, src: &Register) -> Result<(),String> {
        let dst_t = self.typeinf.new_register(dst);
        let dst_ph = TypeSigExpr::Placeholder(self.uniquifier.new_placeholder().clone());
        self.typeinf.add(&dst_t,&TypeSig::Left(dst_ph,src.clone()));
        Ok(())
    }

    /* everything that's not ref */
    pub fn try_apply_command(&mut self, instr: &Instruction, defstore: &DefStore) -> Result<(),String> {
        let sig_regs = TypePass::extract_sig_regs(instr,defstore)?;
        let typesig = self.uniquifier.uniquify_sig(&sig_regs);
        try_apply_command(&mut self.typeinf, &typesig)
    }

    pub fn apply_command(&mut self, instr: &Instruction, defstore: &DefStore) -> Result<(),String> {
        let x = match instr {
            Instruction::Ref(dst,src) => self.try_apply_ref(dst,src),
            instr => self.try_apply_command(instr,defstore)
        };
        match x {
            Ok(_) => self.typeinf.commit(),
            Err(_) => self.typeinf.rollback()
        };
        x
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::codegen::Generator;

    #[test]
    fn typepeass_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/typepass-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let gen = Generator::new();
        let instrs : Vec<Instruction> = gen.go(&defstore,stmts).expect("codegen");
        let instrs_str : Vec<String> = instrs.iter().map(|v| format!("{:?}",v)).collect();
        print!("{}\n",instrs_str.join(""));
        let mut tp = TypePass::new();
        for instr in &instrs {
            print!("=== {:?}",instr);
            let before = tp.clone();
            tp.apply_command(instr,&defstore).expect("ok");
            print!("{}",tp.typeinf.make_diff(&before.typeinf));
        }
    }
}
