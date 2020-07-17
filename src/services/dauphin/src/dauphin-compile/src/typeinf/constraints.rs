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

use crate::command::{ Instruction, InstructionType };
use crate::model::DefStore;
use crate::typeinf::{ ArgumentConstraint, ArgumentExpressionConstraint, InstructionConstraint };
use dauphin_interp::types::{ BaseType, MemberMode };

fn placeholder(ref_: bool) -> ArgumentConstraint {
    if ref_ {
        ArgumentConstraint::Reference(ArgumentExpressionConstraint::Placeholder(String::new()))
    } else {
        ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new()))
    }
}

fn array(ref_: bool) -> ArgumentConstraint {
    if ref_ {
        ArgumentConstraint::Reference(ArgumentExpressionConstraint::Vec(Box::new(ArgumentExpressionConstraint::Placeholder(String::new()))))
    } else {
        ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Vec(Box::new(ArgumentExpressionConstraint::Placeholder(String::new()))))
    }
}

fn fixed(bt: BaseType) -> ArgumentConstraint {
    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(bt))
}

pub fn get_constraints(it: &InstructionType, defstore: &DefStore) -> Result<Vec<ArgumentConstraint>,String> {
    match it {
        InstructionType::CtorStruct(identifier) => {
            let exprdecl = defstore.get_struct_id(identifier)?;
            let intypes = exprdecl.get_member_types();
            let mut out = vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::StructType(identifier.clone())))];
            out.extend(intypes.iter().map(|t| ArgumentConstraint::NonReference(t.to_argumentexpressionconstraint())));
            Ok(out)
        },

        InstructionType::CtorEnum(identifier,branch) => {
            let exprdecl = defstore.get_enum_id(identifier)?;
            let intype = exprdecl.get_branch_type(branch).ok_or_else(|| format!("No such enum branch {:?}",branch))?;
            Ok(vec![
                ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(identifier.clone()))),
                ArgumentConstraint::NonReference(intype.to_argumentexpressionconstraint())
            ])
        },

        InstructionType::SValue(identifier,field) => {
            let exprdecl = defstore.get_struct_id(identifier)?;
            let dtype = exprdecl.get_member_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
            Ok(vec![
                ArgumentConstraint::NonReference(dtype.to_argumentexpressionconstraint()),
                ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::StructType(identifier.clone())))
            ])
        },

        InstructionType::RefSValue(identifier,field) => {
            let exprdecl = defstore.get_struct_id(identifier)?;
            let dtype = exprdecl.get_member_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
            Ok(vec![
                ArgumentConstraint::Reference(dtype.to_argumentexpressionconstraint()),
                ArgumentConstraint::Reference(ArgumentExpressionConstraint::Base(BaseType::StructType(identifier.clone())))
            ])
        },

        InstructionType::EValue(identifier,field) => {
            let exprdecl = defstore.get_enum_id(identifier)?;
            let dtype = exprdecl.get_branch_type(field).ok_or_else(|| format!("No such branch {:?}",field))?;
            Ok(vec![
                ArgumentConstraint::NonReference(dtype.to_argumentexpressionconstraint()),
                ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(identifier.clone())))
            ])
        },

        InstructionType::FilterEValue(identifier,_) => {
            Ok(vec![
                ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType)),
                ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(identifier.clone())))
            ])
        },

        InstructionType::RefEValue(identifier,field) => {
            let exprdecl = defstore.get_enum_id(identifier)?;
            let dtype = exprdecl.get_branch_type(field).ok_or_else(|| format!("No such branch {:?}",field))?;
            Ok(vec![
                ArgumentConstraint::Reference(dtype.to_argumentexpressionconstraint()),
                ArgumentConstraint::Reference(ArgumentExpressionConstraint::Base(BaseType::EnumType(identifier.clone())))
            ])
        },

        InstructionType::ETest(identifier,_) => {
            Ok(vec![
                fixed(BaseType::BooleanType),
                ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(identifier.clone())))
            ])
        },

        InstructionType::Proc(identifier,modes) => {
            let procdecl = defstore.get_proc_id(identifier)?;
            let signature = procdecl.get_signature();
            let mut arguments = Vec::new();
            let mut member_index = 0;
            let members : Vec<_> = signature.each_member().collect();
            for mode in modes {
                let constraint = match mode {
                    MemberMode::In | MemberMode::InOut | MemberMode::Out => {
                        member_index += 1;
                        members[member_index-1].to_argumentconstraint()
                    },
                    MemberMode::Filter => ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType))
                };
                arguments.push(constraint);
            }
            Ok(arguments)
        },

        InstructionType::Operator(identifier) => {
            let mut out = Vec::new();
            let exprdecl = defstore.get_func_id(identifier)?;
            let signature = exprdecl.get_signature();
            for member_constraint in signature.each_member() {
                out.push(member_constraint.to_argumentconstraint());
            }
            Ok(out)
        },

        InstructionType::Nil   => Ok(vec![placeholder(false)]),
        InstructionType::Alias => Ok(vec![placeholder(true),placeholder(false)]),
        InstructionType::Copy =>  Ok(vec![placeholder(false),placeholder(false)]),
        InstructionType::Append => Ok(vec![placeholder(false),placeholder(false)]),
        InstructionType::Square => Ok(vec![placeholder(false),array(false)]),
        InstructionType::RefSquare => Ok(vec![placeholder(true),array(true)]),
        InstructionType::FilterSquare => Ok(vec![fixed(BaseType::NumberType),array(false)]),
        InstructionType::Star => Ok(vec![array(false),placeholder(false)]),
        InstructionType::At => Ok(vec![fixed(BaseType::NumberType),placeholder(false)]),
        InstructionType::Filter => Ok(vec![placeholder(false),placeholder(false),fixed(BaseType::BooleanType)]),
        InstructionType::Run => Ok(vec![fixed(BaseType::NumberType),fixed(BaseType::NumberType),fixed(BaseType::NumberType)]),
        InstructionType::NumberConst(_) | InstructionType::Const(_) => Ok(vec![fixed(BaseType::NumberType)]),
        InstructionType::BooleanConst(_) => Ok(vec![fixed(BaseType::BooleanType)]),
        InstructionType::StringConst(_) => Ok(vec![fixed(BaseType::StringType)]),
        InstructionType::BytesConst(_) => Ok(vec![fixed(BaseType::BytesType)]),
        InstructionType::ReFilter => Ok(vec![fixed(BaseType::NumberType),fixed(BaseType::NumberType),fixed(BaseType::NumberType)]),

        InstructionType::LineNumber(_) |
        InstructionType::Pause(_) |
        InstructionType::NumEq |
        InstructionType::Length |
        InstructionType::Add |
        InstructionType::SeqFilter |
        InstructionType::SeqAt |
        InstructionType::Call(_,_,_,_) =>
            Ok(vec![]),
    }
}

pub fn get_constraint(instr: &Instruction, defstore: &DefStore) -> Result<InstructionConstraint,String> {
    let mut out = Vec::new();
    for (i,c) in get_constraints(&instr.itype, defstore)?.drain(..).enumerate() {
        out.push((c,instr.regs[i]));
    }
    Ok(InstructionConstraint::new(&out))
}
