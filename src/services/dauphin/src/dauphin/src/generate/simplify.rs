use std::collections::HashMap;
use crate::generate::Instruction;
use crate::model::{ DefStore, Register, StructDef, EnumDef };
use crate::typeinf::{ BaseType, MemberType };
use super::codegen::GenContext;

pub struct Extension(Vec<(Register,MemberType)>);

pub struct Extender {
}

impl Extender {
    pub fn new() -> Extender {
        Extender {
        }
    }

    fn allocate_registers(&mut self, context: &mut GenContext, register: &Register, member_types: &Vec<MemberType>, with_index: bool) -> Vec<Register> {
        let mut out = Vec::new();
        for member_type in member_types.iter() {
            let reg = context.regalloc.allocate();
            context.types.add(&reg,member_type);
            out.push(reg);
        }
        out
    }

    fn extend_vertical<F>(&mut self, in_: Vec<&Register>, mapping: &HashMap<Register,Vec<Register>>,cb: F) -> Result<Vec<Instruction>,()>
            where F: Fn(Vec<Register>) -> Instruction {
        let mut expanded = Vec::new();
        let mut len = None;
        for in_reg in in_.iter() {
            let in_reg = in_reg.clone().clone();
            let map = mapping.get(&in_reg).unwrap_or(&vec![in_reg]).clone();
            if len.is_none() { len = Some(map.len()); }
            if map.len() != len.unwrap() { return Err(()); }
            expanded.push(map);
        }
        let mut out = Vec::new();
        for i in 0..len.unwrap() {
            let here_regs : Vec<Register> = expanded.iter().map(|v| v[i].clone()).collect();
            out.push(cb(here_regs));
        }
        Ok(out)
    }

    fn extend_common(&mut self,instr: &Instruction, mapping: &HashMap<Register,Vec<Register>>) -> Result<Option<Vec<Instruction>>,()> {
        Ok(Some(match instr {
            Instruction::Ref(dst,src) => {
                self.extend_vertical(vec![dst,src],mapping,|regs| {
                    Instruction::Ref(regs[0].clone(),regs[1].clone())
                })?
            },  
            Instruction::Copy(dst,src) => {
                self.extend_vertical(vec![dst,src],mapping,|regs| {
                    Instruction::Copy(regs[0].clone(),regs[1].clone())
                })?
            },  
            Instruction::Push(dst,src) => {
                self.extend_vertical(vec![dst,src],mapping,|regs| {
                    Instruction::Push(regs[0].clone(),regs[1].clone())
                })?
            },  
            Instruction::List(reg) => {
                self.extend_vertical(vec![reg],mapping,|regs| {
                    Instruction::List(regs[0].clone())
                })?
            },  
            Instruction::Proc(name,regs) => {
                let mut new_regs = Vec::new();
                for reg in regs {
                    if let Some(dests) = mapping.get(reg) {
                        new_regs.extend(dests.iter().cloned());
                    } else {
                        new_regs.push(reg.clone());
                    }
                }
                vec![Instruction::Proc(name.clone(),new_regs)]
            },
            Instruction::Operator(name,dests,srcs) => {
                let mut new_dests = Vec::new();
                for reg in dests {
                    if let Some(dests) = mapping.get(reg) {
                        new_dests.extend(dests.iter().cloned());
                    } else {
                        new_dests.push(reg.clone());
                    }
                }
                let mut new_srcs = Vec::new();
                for reg in srcs {
                    if let Some(srcs) = mapping.get(reg) {
                        new_srcs.extend(srcs.iter().cloned());
                    } else {
                        new_srcs.push(reg.clone());
                    }
                }
                vec![Instruction::Operator(name.clone(),new_dests,new_srcs)]
            },
            _ => { return Ok(None); }
        }))
    }

    fn extend_struct_instr(&mut self, obj_name: &str, decl: &StructDef, instr: &Instruction, mapping: &HashMap<Register,Vec<Register>>) -> Result<Vec<Instruction>,()> {
        if let Some(instrs) = self.extend_common(instr,mapping)? {
            Ok(instrs)
        } else {
            /* because types topologically ordered and non-recursive
            * we know there's nothing to expand in the args in the else branches.
            */
            Ok(match instr {
                Instruction::CtorStruct(name,dst,srcs) => {
                    if name == obj_name {
                        let dests = mapping.get(dst).ok_or_else(|| ())?;
                        if dests.len() != srcs.len() { return Err(()); }
                        let mut out = Vec::new();
                        for i in 0..srcs.len() {
                            out.push(Instruction::Copy(dests[i].clone(),srcs[i].clone()));
                        }
                        out
                    } else {
                        vec![instr.clone()]
                    }
                },
                Instruction::SValue(field,name,dst,src) => {
                    if name == obj_name {
                        let dests = mapping.get(src).ok_or_else(|| ())?;
                        print!("{:?} find {:?}\n",decl.get_names(),field);
                        let pos = decl.get_names().iter().position(|n| n==field).ok_or_else(|| ())?;
                        vec![Instruction::Copy(dst.clone(),dests[pos].clone())]
                    } else {
                        vec![instr.clone()]
                    }
                },
                Instruction::RefSValue(field,name,dst,src) => {
                    if name == obj_name {
                        let dests = mapping.get(src).ok_or_else(|| ())?;
                        print!("{:?} find {:?}\n",decl.get_names(),field);
                        let pos = decl.get_names().iter().position(|n| n==field).ok_or_else(|| ())?;
                        vec![Instruction::Copy(dst.clone(),dests[pos].clone())]
                    } else {
                        vec![instr.clone()]
                    }
                },
                instr => vec![instr.clone()]
            })
        }
    }

    fn build_nil(&mut self, context: &mut GenContext, defstore: &DefStore, reg: &Register, type_: &MemberType) -> Result<Vec<Instruction>,()> {
        let mut out = Vec::new();
        print!("nil for {:?} (type {:?})\n",reg,type_);
        match type_ {
            MemberType::Vec(v) => out.push(Instruction::List(reg.clone())),
            MemberType::Base(b) => match b {
                BaseType::BooleanType => out.push(Instruction::BooleanConst(reg.clone(),false)),
                BaseType::StringType => out.push(Instruction::StringConst(reg.clone(),String::new())),
                BaseType::NumberType => out.push(Instruction::NumberConst(reg.clone(),0.)),
                BaseType::BytesType => out.push(Instruction::BytesConst(reg.clone(),vec![])),
                BaseType::Invalid => return Err(()),
                BaseType::StructType(name) => {
                    let decl = defstore.get_struct(name).ok_or_else(|| ())?;
                    let mut subregs = Vec::new();
                    for member_type in decl.get_member_types() {
                        let r = context.regalloc.allocate();
                        context.types.add(&r,member_type);
                        out.extend(self.build_nil(context,defstore,&r,member_type)?.iter().cloned());
                        subregs.push(r);
                    }
                    out.push(Instruction::CtorStruct(name.to_string(),reg.clone(),subregs));
                },
                BaseType::EnumType(name) => {
                    // XXX index reg
                    let decl = defstore.get_enum(name).ok_or_else(|| ())?;
                    /* TODO choose best */
                    let branch_type = decl.get_branch_types().get(0).ok_or_else(|| ())?;
                    let field_name = decl.get_names().get(0).ok_or_else(|| ())?;
                    let subreg = context.regalloc.allocate();
                    context.types.add(&subreg,branch_type);
                    out.extend(self.build_nil(context,defstore,&subreg,branch_type)?.iter().cloned());
                    out.push(Instruction::CtorEnum(name.to_string(),field_name.clone(),reg.clone(),subreg));
                }
            }
        }
        print!("nil for {:?} (type {:?}) is {:?}\n",reg,type_,out);
        Ok(out)
    }

    fn extend_enum_instr(&mut self, defstore: &DefStore, context: &mut GenContext, obj_name: &str, decl: &EnumDef, instr: &Instruction, mapping: &HashMap<Register,Vec<Register>>) -> Result<Vec<Instruction>,()> {
        if let Some(instrs) = self.extend_common(instr,mapping)? {
            Ok(instrs)
        } else {
            Ok(match instr {
                Instruction::CtorEnum(name,field,dst,src) => {
                    if name == obj_name {
                        let pos = decl.get_names().iter().position(|v| v==field).ok_or_else(|| ())?;
                        let dests = mapping.get(dst).ok_or_else(|| ())?;
                        let mut out = Vec::new();
                        for i in 0..dests.len() {
                            if i == pos {
                                out.push(Instruction::Copy(dests[i].clone(),src.clone()));
                            } else {
                                let type_ = context.types.get(&dests[i]).ok_or_else(|| ())?.clone();
                                out.extend(self.build_nil(context,defstore,&dests[i],&type_)?.iter().cloned());
                            }
                        }
                        out
                    } else {
                        /* because types topologically ordered and non-recursive
                        * we know there's nothing to expand in the args
                        */
                        vec![instr.clone()]
                    }
                },
                instr => vec![instr.clone()]
            })
        }
    }

    fn make_new_registers(&mut self, context: &mut GenContext, member_types: &Vec<MemberType>, names: &Vec<String>, base: BaseType, with_index: bool) -> Result<HashMap<Register,Vec<Register>>,()> {
        let mut target_registers = Vec::new();
        /* which registers will we be expanding? */
        for (reg,reg_type) in context.types.each_register() {
            if reg_type.get_base() == base {
                target_registers.push(reg.clone());
            }
        }
        /* create some new subregisters for them */
        let mut new_registers = HashMap::new();
        for reg in &target_registers {
            new_registers.insert(reg.clone(),self.allocate_registers(context,reg,member_types,with_index));
        }
        /* make sure all the ref registers we're splitting get updated to use the split non-ref targets */
        for ref_reg in &target_registers {
            if let Some((origin_reg,_)) = context.route.get(ref_reg) {
                let ref_subregs = &new_registers[ref_reg];
                let origin_subregs = &new_registers[origin_reg];
                if origin_subregs.len() != ref_subregs.len() {
                    return Err(());
                }
                for i in 0..origin_subregs.len() {
                    context.route.split_origin(&ref_subregs[i],&origin_subregs[i],&ref_reg);
                }
            }
        }
        /* move any refs which include our member forward to new origin */
        for ref_reg in &target_registers {
            let ref_subregs = &new_registers[ref_reg];
            for i in 0..names.len() {
                context.route.quantify_member(ref_reg,&ref_subregs[i],&names[i]);
            }
        }
        print!("{:?}\n",new_registers);
        Ok(new_registers)
    }

    fn extend_one(&mut self, defstore: &DefStore, context: &mut GenContext, name: &str) -> Result<(),()> {
        let mut new_instrs : Vec<Instruction> = Vec::new();
        if let Some(decl) = defstore.get_struct(name) {
            let member_types = decl.get_member_types();
            let names = decl.get_names();
            let base = BaseType::StructType(name.to_string());
            let new_registers = self.make_new_registers(context,member_types,names,base,false)?;
            for instr in &context.instrs {
                new_instrs.extend(self.extend_struct_instr(name,decl,instr,&new_registers)?.iter().cloned());
            }
        } else if let Some(decl) = defstore.get_enum(name) {
            let member_types = decl.get_branch_types();
            let names = decl.get_names();
            let base = BaseType::EnumType(name.to_string());
            let new_registers = self.make_new_registers(context,member_types,names,base,true)?;
            print!("new_registers {:?}\n",new_registers);
            for instr in &context.instrs.clone() {
                new_instrs.extend(self.extend_enum_instr(defstore,context,name,decl,instr,&new_registers)?.iter().cloned());
            }
        } else {
            return Err(());                
        };
        context.instrs = new_instrs;
        Ok(())
    }

    fn extend(&mut self, defstore: &DefStore, context: &mut GenContext) -> Result<(),()> {
        for name in defstore.get_structenum_order().rev() {
            print!("extend {:?}\n",name);
            self.extend_one(defstore,context,name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::generate::generate_code;

    #[test]
    fn typepeass_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/extension-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        print!("{:?}\n",context);
        let mut xt = Extender::new();
        xt.extend(&defstore,&mut context).expect("k");
        print!("\n===\n\n{:?}\n",context);
    }
}
