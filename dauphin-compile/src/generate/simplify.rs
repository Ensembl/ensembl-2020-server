/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::collections::HashMap;
use crate::command::{ Instruction, InstructionType };
use crate::util::DFloat;
use crate::model::{ DefStore, StructDef, EnumDef };
use crate::typeinf::{ ContainerType, MemberType };
use super::gencontext::GenContext;
use dauphin_interp::command::Identifier;
use dauphin_interp::runtime::Register;
use dauphin_interp::types::BaseType;

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

macro_rules! instr {
    ($context:expr,$type:ident,$($regs:expr),*) => {
        $context.add(Instruction::new(InstructionType::$type,vec![$($regs),*]));
    };
}

macro_rules! allocate {
    ($context:expr,$type:ident) => {
        $context.allocate_register(Some(&MemberType::Base(BaseType::$type)))
    };
}

macro_rules! instr_f {
    ($context:expr,$type:ident,$itype:ident,$($regs:expr),*) => {
        {
            let x = allocate!($context,$type);
            instr!($context,$itype,x,$($regs),*);
            x
        }
    };
}

fn allocate_registers(context: &mut GenContext, member_types: &Vec<MemberType>, with_index: bool, container_type: ContainerType) -> Vec<Register> {
    let mut out = Vec::new();
    if with_index {
        let reg = context.allocate_register(Some(&container_type.construct(MemberType::Base(BaseType::NumberType))));
        out.push(reg);
    }
    for member_type in member_types.iter() {
        let reg = context.allocate_register(Some(&container_type.construct(member_type.clone())));
        out.push(reg);
    }
    out
}

fn extend_vertical<F>(in_: &Vec<Register>, mapping: &HashMap<Register,Vec<Register>>,mut cb: F) -> Result<(),String>
        where F: FnMut(Vec<Register>) -> Result<(),String> {
    let mut expanded = Vec::new();
    let mut len = None;
    for in_reg in in_.iter() {
        let in_reg = in_reg.clone().clone();
        let map = mapping.get(&in_reg).unwrap_or(&vec![in_reg]).clone();
        if len.is_none() { len = Some(map.len()); }
        if map.len() != len.unwrap() { return Err("mismatched register lengths".to_string()); }
        expanded.push(map);
    }
    for i in 0..len.unwrap() {
        let here_regs : Vec<Register> = expanded.iter().map(|v| v[i].clone()).collect();
        cb(here_regs)?;
    }
    Ok(())
}

/* Some easy value for unused enum branches */
fn build_nil(context: &mut GenContext, defstore: &DefStore, reg: &Register, type_: &MemberType) -> Result<(),String> {
    match type_ {
        MemberType::Vec(m) =>  {
            let subreg = context.allocate_register(Some(m));
            instr!(context,Nil,subreg);
            instr!(context,Star,*reg,subreg);
        },
        MemberType::Base(b) => match b {
            BaseType::BooleanType => context.add(Instruction::new(InstructionType::BooleanConst(false),vec![*reg])),
            BaseType::StringType => context.add(Instruction::new(InstructionType::StringConst(String::new()),vec![*reg])),
            BaseType::NumberType => context.add(Instruction::new(InstructionType::NumberConst(DFloat::new_usize(0)),vec![*reg])),
            BaseType::BytesType => context.add(Instruction::new(InstructionType::BytesConst(vec![]),vec![*reg])),
            BaseType::Invalid => return Err("cannot build nil".to_string()),
            BaseType::StructType(name) => {
                let decl = defstore.get_struct_id(name)?;
                let mut subregs = vec![*reg];
                for member_type in decl.get_member_types() {
                    let r = context.allocate_register(Some(member_type));
                    build_nil(context,defstore,&r,member_type)?;
                    subregs.push(r);
                }
                context.add(Instruction::new(InstructionType::CtorStruct(name.clone()),subregs));
            },
            BaseType::EnumType(name) => {
                let decl = defstore.get_enum_id(name)?;
                let branch_type = decl.get_branch_types().get(0).ok_or_else(|| "cannot build nil".to_string())?;
                let field_name = decl.get_names().get(0).ok_or_else(|| "cannot build nil".to_string())?;
                let subreg = context.allocate_register(Some(branch_type));
                build_nil(context,defstore,&subreg,branch_type)?;
                context.add(Instruction::new(InstructionType::CtorEnum(name.clone(),field_name.clone()),vec![*reg,subreg]));
            }
        }
    }
    Ok(())
}

fn extend_common(context: &mut GenContext, instr: &Instruction, mapping: &HashMap<Register,Vec<Register>>) -> Result<(),String> {
    Ok(match &instr.itype {
        InstructionType::Proc(_,_) |
        InstructionType::Operator(_) |
        InstructionType::Run |
        InstructionType::Length |
        InstructionType::Add |
        InstructionType::SeqFilter |
        InstructionType::Pause(_) |
        InstructionType::SeqAt =>
            panic!("Impossible instruction! {:?}",instr),

        InstructionType::CtorStruct(_) |
        InstructionType::CtorEnum(_,_) |
        InstructionType::SValue(_,_) |
        InstructionType::RefSValue(_,_) |
        InstructionType::EValue(_,_) |
        InstructionType::RefEValue(_,_) |
        InstructionType::FilterEValue(_,_) |
        InstructionType::ETest(_,_) |
        InstructionType::NumEq |
        InstructionType::ReFilter |
        InstructionType::Const(_) |
        InstructionType::NumberConst(_) |
        InstructionType::BooleanConst(_) |
        InstructionType::StringConst(_) |
        InstructionType::BytesConst(_) |
        InstructionType::LineNumber(_) =>
            context.add(instr.clone()),

        InstructionType::Nil |
        InstructionType::Alias |
        InstructionType::Copy |
        InstructionType::Append |
        InstructionType::Square |
        InstructionType::RefSquare |
        InstructionType::Star => {
            extend_vertical(&instr.regs,mapping,|regs| {
                context.add(Instruction::new(instr.itype.clone(),regs));
                Ok(())
            })?
        },

        InstructionType::FilterSquare => {
            if let Some(srcs) = mapping.get(&instr.regs[1]) {
                instr!(context,FilterSquare,instr.regs[0],srcs[0]);
            } else {
                instr!(context,FilterSquare,instr.regs[0],instr.regs[1]);
            }
        },

        InstructionType::At => {
            if let Some(srcs) = mapping.get(&instr.regs[1]) {
                instr!(context,At,instr.regs[0],srcs[0]);
            } else {
                context.add(instr.clone());
            }
        },

        InstructionType::Filter => {
            extend_vertical(&vec![instr.regs[0],instr.regs[1]],mapping,|r| {
                instr!(context,Filter,r[0],r[1],instr.regs[2]);
                Ok(())
            })?
        },
        InstructionType::Call(name,impure,type_,flow) => {
            let mut new_regs = Vec::new();
            for reg in &instr.regs {
                if let Some(dests) = mapping.get(&reg) {
                    new_regs.extend(dests.iter().cloned());
                } else {
                    new_regs.push(reg.clone());
                }
            }
            context.add(Instruction::new(InstructionType::Call(name.clone(),*impure,type_.clone(),flow.clone()),new_regs));
        }
    })
}

fn extend_struct_instr(obj_name: &Identifier, context: &mut GenContext, decl: &StructDef, instr: &Instruction, mapping: &HashMap<Register,Vec<Register>>) -> Result<(),String> {
    /* because types topologically ordered and non-recursive
    * we know there's nothing to expand in the args in the else branches.
    */
    Ok(match &instr.itype {
        InstructionType::CtorStruct(name) => {
            if name == obj_name {
                let dests = mapping.get(&instr.regs[0]).ok_or_else(|| "missing register".to_string())?;
                for i in 1..instr.regs.len() {
                    instr!(context,Copy,dests[i-1],instr.regs[i]);
                }
            } else {
                context.add(instr.clone());
            }
        },

        InstructionType::SValue(name,field) if name == obj_name => {
            let dests = mapping.get(&instr.regs[1]).ok_or_else(|| "missing register".to_string())?;
            let pos = decl.get_names().iter().position(|n| n==field).ok_or_else(|| "missing register".to_string())?;
            instr!(context,Copy,instr.regs[0],dests[pos]);
        },

        InstructionType::RefSValue(name,field) if name == obj_name => {
            let dests = mapping.get(&instr.regs[1]).ok_or_else(|| "missing register".to_string())?;
            let pos = decl.get_names().iter().position(|n| n==field).ok_or_else(|| "missing register".to_string())?;
            instr!(context,Alias,instr.regs[0],dests[pos]);
        },

        _ => extend_common(context,instr,mapping)?
    })
}

fn extend_enum_instr(defstore: &DefStore, context: &mut GenContext, obj_name: &Identifier, decl: &EnumDef, instr: &Instruction, mapping: &HashMap<Register,Vec<Register>>) -> Result<(),String> {
    /* because types topologically ordered and non-recursive we know there's nothing to expand in the args */
    Ok(match &instr.itype {
        InstructionType::CtorEnum(name,field) => {
            if name == obj_name {
                let pos = decl.get_names().iter().position(|v| v==field).ok_or_else(|| "missing register".to_string())?;
                let dests = mapping.get(&instr.regs[0]).ok_or_else(|| "missing register".to_string())?;
                for i in 1..dests.len() {
                    if i-1 == pos {
                        context.add(Instruction::new(InstructionType::NumberConst(DFloat::new_usize(i-1)),vec![dests[0]]));
                        instr!(context,Copy,dests[i],instr.regs[1]);
                    } else {
                        let type_ = context.xxx_types().get(&dests[i]).ok_or_else(|| "missing register".to_string())?.clone();
                        build_nil(context,defstore,&dests[i],&type_)?;
                    }
                }
            } else {
                context.add(instr.clone());
            }
        },

        InstructionType::FilterEValue(name,field) if name == obj_name => {
            let pos = decl.get_names().iter().position(|v| v==field).ok_or_else(|| "missing register".to_string())?;
            let srcs = mapping.get(&instr.regs[1]).ok_or_else(|| "missing register".to_string())?;
            let posreg = allocate!(context,NumberType);
            context.add(Instruction::new(InstructionType::NumberConst(DFloat::new_usize(pos)),vec![posreg]));
            let seq = instr_f!(context,NumberType,At,srcs[0]);
            let filter = instr_f!(context,BooleanType,NumEq,srcs[0],posreg);
            instr!(context,Filter,instr.regs[0],seq,filter);
        },

        InstructionType::EValue(name,field) if name == obj_name => {
            let pos = decl.get_names().iter().position(|v| v==field).ok_or_else(|| "missing register".to_string())?;
            let srcs = mapping.get(&instr.regs[1]).ok_or_else(|| "missing register".to_string())?;
            let posreg = allocate!(context,NumberType);
            context.add(Instruction::new(InstructionType::NumberConst(DFloat::new_usize(pos)),vec![posreg]));
            let filter = instr_f!(context,BooleanType,NumEq,srcs[0],posreg);
            instr!(context,Filter,instr.regs[0],srcs[pos+1],filter);
        },

        InstructionType::RefEValue(name,field) if name == obj_name => {
            let pos = decl.get_names().iter().position(|v| v==field).ok_or_else(|| "missing register".to_string())?;
            let srcs = mapping.get(&instr.regs[1]).ok_or_else(|| "missing register".to_string())?;
            instr!(context,Alias,instr.regs[0],srcs[pos+1]);
        },

        InstructionType::ETest(name,field) if name == obj_name => {
            let pos = decl.get_names().iter().position(|v| v==field).ok_or_else(|| "missing register".to_string())?;
            let srcs = mapping.get(&instr.regs[1]).ok_or_else(|| "missing register".to_string())?;
            let posreg = allocate!(context,NumberType);
            context.add(Instruction::new(InstructionType::NumberConst(DFloat::new_usize(pos)),vec![posreg]));
            instr!(context,NumEq,instr.regs[0],srcs[0],posreg);
        },

        _ => extend_common(context,instr,mapping)?
    })
}

fn make_new_registers(context: &mut GenContext, member_types: &Vec<MemberType>, base: BaseType, with_index: bool) -> Result<HashMap<Register,Vec<Register>>,String> {
    let mut target_registers = Vec::new();
    /* which registers will we be expanding? */
    for (reg,reg_type) in context.xxx_types().each_register() {
        if reg_type.get_base() == base {
            target_registers.push(reg.clone());
        }
    }
    target_registers.sort();
    /* create some new subregisters for them */
    let mut new_registers = HashMap::new();
    for reg in &target_registers {
        let type_ = context.xxx_types().get(reg).ok_or_else(|| "Missing register")?.clone();
        new_registers.insert(reg.clone(),allocate_registers(context,member_types,with_index,type_.get_container()));
    }
    /* move any refs which include our member forward to new origin */
    Ok(new_registers)
}

fn extend_one(defstore: &DefStore, context: &mut GenContext, name: &Identifier) -> Result<(),String> {
    if let Some(decl) = defstore.get_struct_id(name).ok() {
        let member_types = decl.get_member_types();
        let base = BaseType::StructType(name.clone());
        let new_registers = make_new_registers(context,member_types,base,false)?;
        for instr in &context.get_instructions() {
            extend_struct_instr(name,context,decl,instr,&new_registers)?;
        }
    } else if let Some(decl) = defstore.get_enum_id(name).ok() {
        let member_types = decl.get_branch_types();
        let base = BaseType::EnumType(name.clone());
        let new_registers = make_new_registers(context,member_types,base,true)?;
        print!("new_registers {:?}\n",new_registers);
        for instr in &context.get_instructions() {
            extend_enum_instr(defstore,context,name,decl,instr,&new_registers)?;
        }
    } else {
        return Err("can only extend structs/enums".to_string());                
    };
    context.phase_finished();
    Ok(())
}

pub fn simplify(defstore: &DefStore, context: &mut GenContext) -> Result<(),String> {
    for name in defstore.get_structenum_order().rev() {
        extend_one(defstore,context,name)?;
    }
    Ok(())
}
