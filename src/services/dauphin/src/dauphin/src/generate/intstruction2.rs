use crate::codegen::{ DefStore, Register2 };
use crate::typeinf::{ ArgumentConstraint, ArgumentExpressionConstraint, BaseType, InstructionConstraint };

#[derive(Debug)]
pub enum Instruction2 {
    Proc(String,Vec<Register2>),
    NumberConst(Register2,f64),
    BooleanConst(Register2,bool),
    StringConst(Register2,String),
    BytesConst(Register2,Vec<u8>),
    List(Register2),
    Push(Register2,Register2),
    CtorStruct(String,Register2,Vec<Register2>),
    CtorEnum(String,String,Register2,Register2),
    SValue(String,String,Register2,Register2),
    Set(Register2,Register2),
    Copy(Register2,Register2),
    EValue(String,String,Register2,Register2),
    ETest(String,String,Register2,Register2),
    RefSValue(String,String,Register2,Register2),
}

impl Instruction2 {
    pub fn get_constraint(&self, defstore: &DefStore) -> Result<InstructionConstraint,String> {
        Ok(InstructionConstraint::new(&match self {
            Instruction2::Proc(name,regs) => {
                let procdecl = defstore.get_proc(name).ok_or_else(|| format!("No such procedure {:?}",name))?;
                let signature = procdecl.get_signature();
                let mut arguments = Vec::new();
                for (i,member) in signature.each_member().enumerate() {
                    arguments.push((member.to_argumentconstraint(),regs[i].clone()));
                }
                arguments
            },
            Instruction2::Set(dst,src) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone()),
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),src.clone())
                ]
            },
            Instruction2::Copy(dst,src) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),src.clone())
                ]
            },
            Instruction2::CtorStruct(name,dst,srcs) => {
                let mut out = Vec::new();
                out.push((ArgumentConstraint::NonReference(
                    ArgumentExpressionConstraint::Base(
                        BaseType::StructType(name.to_string())
                    )
                ),dst.clone()));
                let exprdecl = defstore.get_struct(name).ok_or_else(|| format!("No such struct {:?}",name))?;
                let intypes = exprdecl.get_member_types2();
                if intypes.len() != srcs.len() {
                    return Err(format!("Incorrect number of arguments: got {} expected {}",srcs.len(),intypes.len()));
                }
                for (i,intype) in intypes.iter().enumerate() {
                    out.push((ArgumentConstraint::NonReference(
                        intype.to_argumentexpressionconstraint()
                    ),srcs[i].clone()));
                }
                out
            },
            Instruction2::CtorEnum(name,branch,dst,src) => {
                let mut out = Vec::new();
                out.push((ArgumentConstraint::NonReference(
                    ArgumentExpressionConstraint::Base(
                        BaseType::EnumType(name.to_string())
                    )
                ),dst.clone()));
                let exprdecl = defstore.get_enum(name).ok_or_else(|| format!("No such enum {:?}",name))?;
                out.push((ArgumentConstraint::NonReference(
                    exprdecl.get_branch_type2(branch).ok_or_else(|| format!("No such enum branch {:?}",name))?
                        .to_argumentexpressionconstraint()
                ),src.clone()));
                out
            },
            Instruction2::SValue(field,stype,dst,src) => {
                let exprdecl = defstore.get_struct(stype).ok_or_else(|| format!("No such struct {:?}",stype))?;
                let dtype = exprdecl.get_member_type2(field).ok_or_else(|| format!("No such field {:?}",field))?;
                vec![
                    (ArgumentConstraint::NonReference(
                        dtype.to_argumentexpressionconstraint()
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::StructType(stype.to_string())
                        )
                    ),src.clone())
                ]
            },
            Instruction2::RefSValue(field,stype,dst,src) => {
                let exprdecl = defstore.get_struct(stype).ok_or_else(|| format!("No such struct {:?}",stype))?;
                let dtype = exprdecl.get_member_type2(field).ok_or_else(|| format!("No such field {:?}",field))?;
                vec![
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::StructType(stype.to_string())
                        )
                    ),dst.clone()),
                    (ArgumentConstraint::Reference(
                        dtype.to_argumentexpressionconstraint()
                    ),src.clone())
                ]
            },
            Instruction2::EValue(field,etype,dst,src) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                let dtype = exprdecl.get_branch_type2(field).ok_or_else(|| format!("No such branch {:?}",field))?;
                vec![
                    (ArgumentConstraint::NonReference(
                        dtype.to_argumentexpressionconstraint()
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::EnumType(etype.to_string())
                        )
                    ),src.clone())
                ]
            },
            Instruction2::ETest(field,etype,dst,src) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::BooleanType
                        )
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::EnumType(etype.to_string())
                        )
                    ),src.clone())
                ]
            },
            Instruction2::NumberConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::NumberType
                        )
                    ),r.clone())
                ]
            },
            Instruction2::BooleanConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::BooleanType
                        )
                    ),r.clone())
                ]
            },
            Instruction2::StringConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::StringType
                        )
                    ),r.clone())
                ]
            },
            Instruction2::BytesConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::BytesType
                        )
                    ),r.clone())
                ]
            },
            Instruction2::List(r) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Vec(Box::new(
                            ArgumentExpressionConstraint::Placeholder(String::new())
                        ))
                    ),r.clone())
                ]
            },
            Instruction2::Push(r,c) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Vec(Box::new(
                            ArgumentExpressionConstraint::Placeholder(String::new())
                        ))
                    ),r.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),c.clone())
                ]
            }
        }))
    }
}