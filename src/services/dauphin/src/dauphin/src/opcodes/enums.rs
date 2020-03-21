use std::collections::HashMap;
use std::rc::Rc;
use crate::generate::{ Instruction, Instruction2, Instruction2Core, GenContext };
use crate::model::{ DefStore, Register };
use super::CtorStruct;
use crate::typeinf::{ ArgumentConstraint, ArgumentExpressionConstraint, BaseType, MemberType };

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
                    out.extend(build_nil(context,defstore,&r,member_type)?);
                    subregs.push(r);
                }
                out.push(Instruction::New(Rc::new(CtorStruct::new(name.to_string(),reg.clone(),subregs))));
            },
            BaseType::EnumType(name) => {
                let decl = defstore.get_enum(name).ok_or_else(|| ())?;
                let branch_type = decl.get_branch_types().get(0).ok_or_else(|| ())?;
                let field_name = decl.get_names().get(0).ok_or_else(|| ())?;
                let subreg = context.regalloc.allocate();
                context.types.add(&subreg,branch_type);
                out.extend(build_nil(context,defstore,&subreg,branch_type)?);
                out.push(Instruction::New(Rc::new(CtorEnum::new(name.to_string(),field_name.clone(),reg.clone(),subreg))));
            }
        }
    }
    Ok(out)
}

#[derive(PartialEq,Clone)]
pub struct CtorEnum(Instruction2Core<()>);

impl CtorEnum {
    pub fn new(typename: String, branch: String, dst: Register, src: Register) -> CtorEnum {
        CtorEnum(Instruction2Core::new(vec!["enum".to_string(),typename,branch],vec![dst,src],vec![]))
    }
}

impl Instruction2 for CtorEnum {
    fn get_registers(&self) -> Vec<Register> { self.0.get_registers() }

    fn format(&self) -> String { self.0.format(|_| None) }

    fn get_constraint(&self, defstore: &DefStore) -> Result<Vec<(ArgumentConstraint,Register)>,String> {
        let name = &self.0.prefixes[1];
        let branch = &self.0.prefixes[2];
        let dst = &self.0.registers[0];
        let src = &self.0.registers[1];
        let mut out = Vec::new();
        out.push((ArgumentConstraint::NonReference(
            ArgumentExpressionConstraint::Base(
                BaseType::EnumType(name.to_string())
            )
        ),dst.clone()));
        let exprdecl = defstore.get_enum(name).ok_or_else(|| format!("No such enum {:?}",name))?;
        out.push((ArgumentConstraint::NonReference(
            exprdecl.get_branch_type(branch).ok_or_else(|| format!("No such enum branch {:?}",name))?
                .to_argumentexpressionconstraint()
        ),src.clone()));
        Ok(out)
    }

    fn simplify(&self, defstore: &DefStore, context: &mut GenContext, obj_name: &str, mapping: &HashMap<Register,Vec<Register>>, branch_names: &Vec<String>) -> Result<Vec<Instruction>,()> {
        let name = &self.0.prefixes[1];
        let branch = &self.0.prefixes[2];
        let dst = &self.0.registers[0];
        let src = &self.0.registers[1];
        if name == obj_name {
            let pos = branch_names.iter().position(|v| v==branch).ok_or_else(|| ())?;
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
            Ok(out)
        } else {
            Ok(vec![Instruction::New(Rc::new(self.clone()))])
        }
    }
}

#[derive(PartialEq,Clone)]
pub struct EValue(Instruction2Core<()>);

impl EValue {
    pub fn new(typename: String, branch: String, dst: Register, src: Register) -> EValue {
        EValue(Instruction2Core::new(vec!["evalue".to_string(),typename,branch],vec![dst,src],vec![]))
    }
}

impl Instruction2 for EValue {
    fn get_registers(&self) -> Vec<Register> { self.0.get_registers() }
    fn format(&self) -> String { self.0.format(|_| None) }

    fn get_constraint(&self, defstore: &DefStore) -> Result<Vec<(ArgumentConstraint,Register)>,String> {
        let name = &self.0.prefixes[1];
        let branch = &self.0.prefixes[2];
        let dst = &self.0.registers[0];
        let src = &self.0.registers[1];
        let exprdecl = defstore.get_enum(name).ok_or_else(|| format!("No such enum {:?}",name))?;
        let dtype = exprdecl.get_branch_type(branch).ok_or_else(|| format!("No such branch {:?}",branch))?;
        Ok(vec![
            (ArgumentConstraint::NonReference(
                dtype.to_argumentexpressionconstraint()
            ),dst.clone()),
            (ArgumentConstraint::NonReference(
                ArgumentExpressionConstraint::Base(
                    BaseType::EnumType(name.to_string())
                )
            ),src.clone())
        ])
    }

    fn simplify(&self, _defstore: &DefStore, context: &mut GenContext, obj_name: &str, mapping: &HashMap<Register,Vec<Register>>, names: &Vec<String>) -> Result<Vec<Instruction>,()> {
        let name = &self.0.prefixes[1];
        if name == obj_name {
            let branch = &self.0.prefixes[2];
            let dst = &self.0.registers[0];
            let src = &self.0.registers[1];
            let pos = names.iter().position(|v| v==branch).ok_or_else(|| ())?;
            let srcs = mapping.get(src).ok_or_else(|| ())?;
            let mut out = Vec::new();
            let filter = context.regalloc.allocate();
            let posreg = context.regalloc.allocate();
            out.push(Instruction::NumberConst(posreg.clone(),pos as f64));
            context.types.add(&filter,&MemberType::Base(BaseType::BooleanType));
            out.push(Instruction::NumEq(filter.clone(),srcs[0].clone(),posreg));
            out.push(Instruction::Filter(dst.clone(),srcs[pos+1].clone(),filter));
            Ok(out)
        } else {
            Ok(vec![Instruction::New(Rc::new(self.clone()))])
        }
    }
}

#[derive(PartialEq,Clone)]
pub struct ETest(Instruction2Core<()>);

impl ETest {
    pub fn new(typename: String, branch: String, dst: Register, src: Register) -> ETest {
        ETest(Instruction2Core::new(vec!["etest".to_string(),typename,branch],vec![dst,src],vec![]))
    }
}

impl Instruction2 for ETest {
    fn get_registers(&self) -> Vec<Register> { self.0.get_registers() }
    fn format(&self) -> String { self.0.format(|_| None) }

    fn get_constraint(&self, _defstore: &DefStore) -> Result<Vec<(ArgumentConstraint,Register)>,String> {
        let name = &self.0.prefixes[1];
        let dst = &self.0.registers[0];
        let src = &self.0.registers[1];
        Ok(vec![
            (ArgumentConstraint::NonReference(
                ArgumentExpressionConstraint::Base(
                    BaseType::BooleanType
                )
            ),dst.clone()),
            (ArgumentConstraint::NonReference(
                ArgumentExpressionConstraint::Base(
                    BaseType::EnumType(name.to_string())
                )
            ),src.clone())
        ])
    }

    fn simplify(&self, _defstore: &DefStore, context: &mut GenContext, obj_name: &str, mapping: &HashMap<Register,Vec<Register>>, names: &Vec<String>) -> Result<Vec<Instruction>,()> {
        let name = &self.0.prefixes[1];
        let branch = &self.0.prefixes[2];
        let dst = &self.0.registers[0];
        let src = &self.0.registers[1];
        if name == obj_name {
            let pos = names.iter().position(|v| v==branch).ok_or_else(|| ())?;
            let srcs = mapping.get(src).ok_or_else(|| ())?;
            let mut out = Vec::new();
            let posreg = context.regalloc.allocate();
            out.push(Instruction::NumberConst(posreg.clone(),pos as f64));
            out.push(Instruction::NumEq(dst.clone(),srcs[0].clone(),posreg));
            Ok(out)
        } else {
            Ok(vec![Instruction::New(Rc::new(self.clone()))])
        }
    }
}
