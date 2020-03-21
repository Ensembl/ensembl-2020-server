use std::collections::HashMap;
use std::rc::Rc;
use crate::generate::{ Instruction, Instruction2, Instruction2Core, GenContext };
use crate::model::{ DefStore, Register };
use crate::typeinf::{ ArgumentConstraint, ArgumentExpressionConstraint, BaseType };

#[derive(PartialEq,Clone)]
pub struct CtorStruct(Instruction2Core<()>);

impl CtorStruct {
    pub fn new(typename: String, dst: Register, mut src: Vec<Register>) -> CtorStruct {    
        let mut regs = vec![dst];
        regs.append(&mut src);
        CtorStruct(Instruction2Core::new(vec!["struct".to_string(),typename],regs,vec![]))
    }
}

impl Instruction2 for CtorStruct {
    fn get_registers(&self) -> Vec<Register> { self.0.get_registers() }

    fn format(&self) -> String { self.0.format(|_| None) }

    fn get_constraint(&self, defstore: &DefStore) -> Result<Vec<(ArgumentConstraint,Register)>,String> {
        let name = &self.0.prefixes[1];
        let dst = &self.0.registers[0];
        let srcs = &self.0.registers[1..];
        let mut out = Vec::new();
        out.push((ArgumentConstraint::NonReference(
            ArgumentExpressionConstraint::Base(
                BaseType::StructType(name.to_string())
            )
        ),dst.clone()));
        let exprdecl = defstore.get_struct(name)?;
        let intypes = exprdecl.get_member_types();
        if intypes.len() != srcs.len() {
            return Err(format!("Incorrect number of arguments: got {} expected {}",srcs.len(),intypes.len()));
        }
        for (i,intype) in intypes.iter().enumerate() {
            out.push((ArgumentConstraint::NonReference(
                intype.to_argumentexpressionconstraint()
            ),srcs[i].clone()));
        }
        Ok(out)
    }

    fn simplify(&self, _defstore: &DefStore, _context: &mut GenContext, obj_name: &str, mapping: &HashMap<Register,Vec<Register>>, _branch_names: &Vec<String>) -> Result<Vec<Instruction>,String> {
        let name = &self.0.prefixes[1];
        if name == obj_name {
            let dst = &self.0.registers[0];
            let srcs = &self.0.registers[1..];
            let dests = mapping.get(dst).ok_or_else(|| "internal error: bad mapping".to_string())?;
            if dests.len() != srcs.len() { return Err("internal error: mismatching lengths".to_string()); }
            let mut out = Vec::new();
            for i in 0..srcs.len() {
                out.push(Instruction::Copy(dests[i].clone(),srcs[i].clone()));
            }
            Ok(out)
        } else {
            Ok(vec![Instruction::New(Rc::new(self.clone()))])
        }
    }
}

#[derive(PartialEq,Clone)]
pub struct SValue(Instruction2Core<()>);

impl SValue {
    pub fn new(typename: String, member: String, dst: Register, src: Register) -> SValue {
        SValue(Instruction2Core::new(vec!["svalue".to_string(),typename,member],vec![dst,src],vec![]))
    }
}

impl Instruction2 for SValue {
    fn get_registers(&self) -> Vec<Register> { self.0.get_registers() }

    fn format(&self) -> String { self.0.format(|_| None) }

    fn get_constraint(&self, defstore: &DefStore) -> Result<Vec<(ArgumentConstraint,Register)>,String> {
        let stype = &self.0.prefixes[1];
        let field = &self.0.prefixes[2];
        let dst = &self.0.registers[0];
        let src = &self.0.registers[1];
        let exprdecl = defstore.get_struct(stype)?;
        let dtype = exprdecl.get_member_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
        Ok(vec![
            (ArgumentConstraint::NonReference(
                dtype.to_argumentexpressionconstraint()
            ),dst.clone()),
            (ArgumentConstraint::NonReference(
                ArgumentExpressionConstraint::Base(
                    BaseType::StructType(stype.to_string())
                )
            ),src.clone())
        ])
    }

    fn simplify(&self, _defstore: &DefStore, _context: &mut GenContext, obj_name: &str, mapping: &HashMap<Register,Vec<Register>>, names: &Vec<String>) -> Result<Vec<Instruction>,String> {
        let name = &self.0.prefixes[1];
        let field = &self.0.prefixes[2];
        let dst = &self.0.registers[0];
        let src = &self.0.registers[1];
        if name == obj_name {
            let dests = mapping.get(src).ok_or_else(|| "internal error: bad mapping".to_string())?;
            let pos = names.iter().position(|n| n==field).ok_or_else(|| "internal error: no such field".to_string())?;
            Ok(vec![Instruction::Copy(dst.clone(),dests[pos].clone())])
        } else {
            Ok(vec![Instruction::New(Rc::new(self.clone()))])
        }
    }
}
