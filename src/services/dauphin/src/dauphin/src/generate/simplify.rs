use std::collections::HashMap;
use crate::generate::Instruction;
use crate::model::{ DefStore, Register, StructDef, EnumDef };
use crate::typeinf::{ BaseType, ContainerType, MemberType, RouteExpr };
use super::codegen::GenContext;

/* simplification is the process of converting arbitrary assemblies of structs, enums and vecs into sets of vecs of
 * simple values. To achieve this, vecs of structured types are converted to sets of vecs of simpler types.
 * 
 * dauphin datastructures cannot be defined recursively, so they can be ordered such that within the ordering
 * containment occurs in only one direction. With such an order and starting at the largest type, data structures
 * are simplified iteratively. After the complete elimination of one structure to generate new code, the code is
 * considered completely afresh to eliminate the next.
 * 
 * For each elimination, first registers are made and then each instruction updated in turn.
 * 
 * Registers are made in two stages. First they are allocated and then refs are updated. refs refer to some origin
 * register of the same type. Because registers have just been allocated for the whole type, there are now matching
 * sets of registers to replace the ref and non-ref types. For each reference, the origin is updated to point to the
 * relevant sub-register, copying the path. Any of those reference registers which refer to the type currently being 
 * split are then replaced with a new origin and this part removed from the expression.
 * 
 * Extension proceeds slightly differently depending on whether a struct or an enum is extended. However, most
 * instructions are extended in the same way and are handled in a common function.
 */

fn allocate_registers(context: &mut GenContext, member_types: &Vec<MemberType>, with_index: bool, container_type: ContainerType) -> Vec<Register> {
    let mut out = Vec::new();
    if with_index {
        let reg = context.regalloc.allocate();
        context.types.add(&reg,&container_type.construct(MemberType::Base(BaseType::NumberType)));
        out.push(reg);
    }
    for member_type in member_types.iter() {
        let reg = context.regalloc.allocate();
        context.types.add(&reg,&container_type.construct(member_type.clone()));
        out.push(reg);
    }
    out
}

/* extend_vertical applies the given operation to all subregisters. It's the operation to apply to most instrctions. */
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

/* Some easy value for unused enum branches */
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
        Instruction::SeqFilter(_,_,_,_) |
        Instruction::Length(_,_) |
        Instruction::Add(_,_) |
        Instruction::Operator(_,_,_) |
        Instruction::Proc(_,_) |
        Instruction::SeqAt(_,_) => {
            panic!("Impossible instruction! {:?}",instr);
        },
        Instruction::NumberConst(_,_) |
        Instruction::BooleanConst(_,_) |
        Instruction::StringConst(_,_) |
        Instruction::BytesConst(_,_) |
        Instruction::CtorStruct(_,_,_) |
        Instruction::CtorEnum(_,_,_,_) |
        Instruction::SValue(_,_,_,_) |
        Instruction::EValue(_,_,_,_) |
        Instruction::ETest(_,_,_,_) |
        Instruction::Alias(_,_) |
        Instruction::Run(_,_,_) |
        Instruction::NumEq(_,_,_) => {
            vec![instr.clone()]
        },
        Instruction::Nil(r) => {
            extend_vertical(vec![r],mapping,|regs| {
                Instruction::Nil(regs[0])
            })?
        },
        Instruction::LValue(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::LValue(regs[0],regs[1])
            })?
        },
        Instruction::Copy(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Copy(regs[0],regs[1])
            })?
        },  
        Instruction::Append(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Append(regs[0],regs[1])
            })?
        },  
        Instruction::List(reg) => {
            extend_vertical(vec![reg],mapping,|regs| {
                Instruction::List(regs[0])
            })?
        },  
        Instruction::Square(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Square(regs[0],regs[1])
            })?
        },
        Instruction::RefSquare(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::RefSquare(regs[0],regs[1])
            })?
        },
        Instruction::FilterSquare(dst,src) => {
            if let Some(srcs) = mapping.get(&src) {
                vec![Instruction::FilterSquare(*dst,srcs[0])]
            } else {
                vec![Instruction::FilterSquare(*dst,*src)]
            }
        },
        Instruction::Star(dst,src) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Star(regs[0],regs[1])
            })?
        },
        Instruction::Filter(dst,src,filter) => {
            extend_vertical(vec![dst,src],mapping,|regs| {
                Instruction::Filter(regs[0],regs[1],filter.clone())
            })?
        },
        Instruction::At(dst,src) => {
            if let Some(srcs) = mapping.get(&src) {
                vec![Instruction::At(*dst,srcs[0])]
            } else {
                vec![Instruction::At(*dst,*src)]
            }
        },
        Instruction::Call(name,type_,regs) => {
            let mut new_regs = Vec::new();
            for reg in regs {
                if let Some(dests) = mapping.get(reg) {
                    new_regs.extend(dests.iter().cloned());
                } else {
                    new_regs.push(reg.clone());
                }
            }
            vec![Instruction::Call(name.clone(),type_.clone(),new_regs)]
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
        let type_ = context.types.get(reg).ok_or_else(|| ())?.clone();
        new_registers.insert(reg.clone(),allocate_registers(context,member_types,with_index,type_.get_container()));
    }
    /* move any refs which include our member forward to new origin */
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
    use super::super::call;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::generate::generate_code;
    use crate::testsuite::load_testdata;

    // XXX common
    fn compare_instrs(a: &Vec<String>,b: &Vec<String>) {
        print!("compare:\nLHS\n{:?}\n\nRHS\n{:?}\n",a.join("\n"),b.join("\n"));
        let mut a_iter = a.iter();
        for (i,b) in b.iter().enumerate() {
            if let Some(a) = a_iter.next() {
                let a = a.trim();
                let b = b.trim();
                assert_eq!(a,b,"mismatch {:?} {:?} line {}",a,b,i);
            } else if b != "" {
                panic!("premature eof lhs");
            }
        }
        if a_iter.next().is_some() {
            panic!("premature eof rhs");
        }
    }

    #[test]
    fn simplify_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/simplify-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        print!("{:?}\n",context);
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        let outdata = load_testdata(&["codegen","simplify-smoke.out"]).ok().unwrap();
        let cmds : Vec<String> = context.instrs.iter().map(|e| format!("{:?}",e)).collect();
        compare_instrs(&cmds,&outdata.split("\n").map(|x| x.to_string()).collect());
        print!("{:?}\n",context);
    }

    #[test]
    fn simplify_enum_nest() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/simplify-enum-nest.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        print!("{:?}\n",context);
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
    }
}
