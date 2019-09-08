use std::collections::HashMap;
use crate::generate::Instruction;
use crate::model::{ DefStore, Register, StructDef, EnumDef };
use crate::typeinf::{ BaseType, MemberType, RouteExpr };
use super::codegen::GenContext;

fn allocate_registers(context: &mut GenContext, member_types: &Vec<MemberType>, with_index: bool) -> Vec<Register> {
    let mut out = Vec::new();
    if with_index {
        let reg = context.regalloc.allocate();
        context.types.add(&reg,&MemberType::Base(BaseType::NumberType));
        out.push(reg);
    }
    for member_type in member_types.iter() {
        let reg = context.regalloc.allocate();
        context.types.add(&reg,member_type);
        out.push(reg);
    }
    out
}

fn extend_vertical<F>(in_: Vec<&Register>, mapping: &HashMap<Register,Vec<Register>>,cb: F) -> Result<Vec<Instruction>,()>
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

fn build_nil(context: &mut GenContext, defstore: &DefStore, reg: &Register, type_: &MemberType) -> Result<Vec<Instruction>,()> {
    let mut out = Vec::new();
    match type_ {
        MemberType::Vec(_) => out.push(Instruction::List(reg.clone())),
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
                    out.extend(build_nil(context,defstore,&r,member_type)?.iter().cloned());
                    subregs.push(r);
                }
                out.push(Instruction::CtorStruct(name.to_string(),reg.clone(),subregs));
            },
            BaseType::EnumType(name) => {
                let decl = defstore.get_enum(name).ok_or_else(|| ())?;
                let branch_type = decl.get_branch_types().get(0).ok_or_else(|| ())?;
                let field_name = decl.get_names().get(0).ok_or_else(|| ())?;
                let subreg = context.regalloc.allocate();
                context.types.add(&subreg,branch_type);
                out.extend(build_nil(context,defstore,&subreg,branch_type)?.iter().cloned());
                out.push(Instruction::CtorEnum(name.to_string(),field_name.clone(),reg.clone(),subreg));
            }
        }
    }
    Ok(out)
}

fn extend_common(instr: &Instruction, mapping: &HashMap<Register,Vec<Register>>) -> Result<Vec<Instruction>,()> {
    Ok(match instr {
        Instruction::Nil(_) => panic!("Impossible instruction #nil!"),
        Instruction::NumberConst(_,_) |
        Instruction::BooleanConst(_,_) |
        Instruction::StringConst(_,_) |
        Instruction::BytesConst(_,_) |
        Instruction::CtorStruct(_,_,_) |
        Instruction::CtorEnum(_,_,_,_) |
        Instruction::SValue(_,_,_,_) |
        Instruction::EValue(_,_,_,_) |
        Instruction::ETest(_,_,_,_) |
        Instruction::RefSValue(_,_,_,_) |
        Instruction::RefEValue(_,_,_,_) |
        Instruction::NumEq(_,_,_) => {
            vec![instr.clone()]
        },
        Instruction::Ref(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Ref(regs[0].clone(),regs[1].clone())
            })?
        },
        Instruction::Copy(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Copy(regs[0].clone(),regs[1].clone())
            })?
        },  
        Instruction::Push(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Push(regs[0].clone(),regs[1].clone())
            })?
        },  
        Instruction::List(reg) => {
            extend_vertical(vec![reg],mapping,|regs| {
                Instruction::List(regs[0].clone())
            })?
        },  
        Instruction::Square(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Square(regs[0].clone(),regs[1].clone())
            })?
        },
        Instruction::RefSquare(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::RefSquare(regs[0].clone(),regs[1].clone())
            })?
        },
        Instruction::Star(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Star(regs[0].clone(),regs[1].clone())
            })?
        },
        Instruction::Filter(dst,src,filter) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Filter(regs[0].clone(),regs[1].clone(),filter.clone())
            })?
        },
        Instruction::RefFilter(dst,src,filter) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::RefFilter(regs[0].clone(),regs[1].clone(),filter.clone())
            })?
        },
        Instruction::At(dst,src) => {
            if let Some(srcs) = mapping.get(&src) {
                vec![Instruction::At(dst.clone(),srcs[0].clone())]
            } else {
                vec![Instruction::At(dst.clone(),src.clone())]
            }
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
        }
    })
}

fn extend_struct_instr(obj_name: &str, decl: &StructDef, instr: &Instruction, mapping: &HashMap<Register,Vec<Register>>) -> Result<Vec<Instruction>,()> {
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
        Instruction::SValue(field,name,dst,src) if name == obj_name => {
            let dests = mapping.get(src).ok_or_else(|| ())?;
            print!("{:?} find {:?}\n",decl.get_names(),field);
            let pos = decl.get_names().iter().position(|n| n==field).ok_or_else(|| ())?;
            vec![Instruction::Copy(dst.clone(),dests[pos].clone())]
        },
        Instruction::RefSValue(field,name,dst,src) if name == obj_name => {
            let dests = mapping.get(src).ok_or_else(|| ())?;
            print!("{:?} find {:?}\n",decl.get_names(),field);
            let pos = decl.get_names().iter().position(|n| n==field).ok_or_else(|| ())?;
            vec![Instruction::Copy(dst.clone(),dests[pos].clone())]
        },
        instr => extend_common(instr,mapping)?
    })
}

fn extend_enum_instr(defstore: &DefStore, context: &mut GenContext, obj_name: &str, decl: &EnumDef, instr: &Instruction, mapping: &HashMap<Register,Vec<Register>>) -> Result<Vec<Instruction>,()> {
    /* because types topologically ordered and non-recursive we know
        * there's nothing to expand in the args
        */
    Ok(match instr {
        Instruction::CtorEnum(name,field,dst,src) => {
            if name == obj_name {
                let pos = decl.get_names().iter().position(|v| v==field).ok_or_else(|| ())?;
                let dests = mapping.get(dst).ok_or_else(|| ())?;
                let mut out = Vec::new();
                for i in 1..dests.len() {
                    if i-1 == pos {
                        out.push(Instruction::NumberConst(dests[0].clone(),(i-1) as f64));
                        out.push(Instruction::Copy(dests[i].clone(),src.clone()));
                    } else {
                        let type_ = context.types.get(&dests[i]).ok_or_else(|| ())?.clone();
                        out.extend(build_nil(context,defstore,&dests[i],&type_)?.iter().cloned());
                    }
                }
                out
            } else {
                vec![instr.clone()]
            }
        },
        Instruction::EValue(field,name,dst,src) if name == obj_name => {
            let pos = decl.get_names().iter().position(|v| v==field).ok_or_else(|| ())?;
            let srcs = mapping.get(src).ok_or_else(|| ())?;
            let mut out = Vec::new();
            let filter = context.regalloc.allocate();
            let posreg = context.regalloc.allocate();
            out.push(Instruction::NumberConst(posreg.clone(),pos as f64));
            context.types.add(&filter,&MemberType::Base(BaseType::BooleanType));
            out.push(Instruction::NumEq(filter.clone(),srcs[0].clone(),posreg));
            out.push(Instruction::Filter(dst.clone(),srcs[pos+1].clone(),filter));
            out
        },
        Instruction::ETest(field,name,dst,src) if name == obj_name => {
            let pos = decl.get_names().iter().position(|v| v==field).ok_or_else(|| ())?;
            let srcs = mapping.get(src).ok_or_else(|| ())?;
            let mut out = Vec::new();
            let posreg = context.regalloc.allocate();
            out.push(Instruction::NumberConst(posreg.clone(),pos as f64));
            out.push(Instruction::NumEq(dst.clone(),srcs[0].clone(),posreg));
            out
        },
        Instruction::RefEValue(field,name,dst,src) if name == obj_name => {
            let pos = decl.get_names().iter().position(|v| v==field).ok_or_else(|| ())?;
            let srcs = mapping.get(src).ok_or_else(|| ())?;
            let mut out = Vec::new();
            let filter = context.regalloc.allocate();
            let posreg = context.regalloc.allocate();
            out.push(Instruction::NumberConst(posreg.clone(),pos as f64));
            context.types.add(&filter,&MemberType::Base(BaseType::BooleanType));
            out.push(Instruction::NumEq(filter.clone(),srcs[0].clone(),posreg));
            context.route.set_derive(&dst,&srcs[pos+1],&RouteExpr::Filter(filter.clone()));
            out.push(Instruction::RefFilter(dst.clone(),srcs[pos+1].clone(),filter));
            out
        },
        instr => extend_common(instr,mapping)?
    })
}

fn make_new_registers(context: &mut GenContext, member_types: &Vec<MemberType>, names: &Vec<String>, base: BaseType, with_index: bool) -> Result<HashMap<Register,Vec<Register>>,()> {
    let mut target_registers = Vec::new();
    /* which registers will we be expanding? */
    for (reg,reg_type) in context.types.each_register() {
        if reg_type.get_base() == base {
            target_registers.push(reg.clone());
        }
    }
    target_registers.sort();
    /* create some new subregisters for them */
    let mut new_registers = HashMap::new();
    for reg in &target_registers {
        new_registers.insert(reg.clone(),allocate_registers(context,member_types,with_index));
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
        let offset = if with_index { 1 } else { 0 };
        for i in 0..names.len() {
            context.route.quantify_member(ref_reg,&ref_subregs[i+offset],&names[i]);
        }
    }
    print!("{:?}\n",new_registers);
    Ok(new_registers)
}

fn extend_one(defstore: &DefStore, context: &mut GenContext, name: &str) -> Result<(),()> {
    let mut new_instrs : Vec<Instruction> = Vec::new();
    if let Some(decl) = defstore.get_struct(name) {
        let member_types = decl.get_member_types();
        let names = decl.get_names();
        let base = BaseType::StructType(name.to_string());
        let new_registers = make_new_registers(context,member_types,names,base,false)?;
        for instr in &context.instrs {
            new_instrs.extend(extend_struct_instr(name,decl,instr,&new_registers)?.iter().cloned());
        }
    } else if let Some(decl) = defstore.get_enum(name) {
        let member_types = decl.get_branch_types();
        let names = decl.get_names();
        let base = BaseType::EnumType(name.to_string());
        let new_registers = make_new_registers(context,member_types,names,base,true)?;
        print!("new_registers {:?}\n",new_registers);
        for instr in &context.instrs.clone() {
            new_instrs.extend(extend_enum_instr(defstore,context,name,decl,instr,&new_registers)?.iter().cloned());
        }
    } else {
        return Err(());                
    };
    context.instrs = new_instrs;
    Ok(())
}

pub fn simplify(defstore: &DefStore, context: &mut GenContext) -> Result<(),()> {
    for name in defstore.get_structenum_order().rev() {
        print!("extend {:?}\n",name);
        extend_one(defstore,context,name)?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::generate::generate_code;
    use crate::testsuite::load_testdata;

    #[test]
    fn simplify_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/simplify-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        print!("{:?}\n",context);
        simplify(&defstore,&mut context).expect("k");
        let outdata = load_testdata(&["codegen","simplify-smoke.out"]).ok().unwrap();
        let cmds : Vec<String> = context.instrs.iter().map(|e| format!("{:?}",e)).collect();
        assert_eq!(outdata,cmds.join(""));
        print!("{:?}\n",context);
    }
}
