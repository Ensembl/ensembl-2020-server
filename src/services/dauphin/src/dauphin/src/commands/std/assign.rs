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

use crate::model::{ Register, RegisterSignature };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, InterpContext, PreImageOutcome };
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use serde_cbor::Value as CborValue;
use crate::model::{ cbor_array, cbor_bool };
use crate::typeinf::MemberMode;
use super::super::common::vectorcopy::{ vector_update, vector_update_poly, vector_push, vector_register_copy, vector_append, append_data };
use super::super::common::vectorsource::RegisterVectorSource;
use super::super::common::sharedvec::SharedVec;
use super::super::common::writevec::WriteVec;
use super::library::std;

fn assign_unfiltered(context: &mut InterpContext, regs: &Vec<Register>) -> Result<(),String> {
    let registers = context.registers();
    let n = regs.len()/2;
    for i in 0..n {
        registers.copy(&regs[i],&regs[i+n])?;
    }
    Ok(())
}

fn preimage_possible(context: &mut PreImageContext, regs: &Vec<Register>) -> Result<bool,String> {
    let n = regs.len()/2;
    for i in 0..n {
        if !context.get_reg_valid(&regs[i+n]) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn preimage_instrs(regs: &Vec<Register>) -> Result<Vec<Instruction>,String> {
    let mut instrs = vec![];
    let n = regs.len()/2;
    for i in 0..n {
        instrs.push(Instruction::new(InstructionType::Copy,vec![regs[i],regs[i+n]]));
    }
    Ok(instrs)
}

fn copy_deep<'d>(left: &mut WriteVec<'d>, right: &SharedVec, filter: &[usize]) -> Result<(),String> {
    if filter.len() > 0 {
        let offsets = vector_push(left,right,filter.len())?;
        let depth = left.depth();
        let off_len = offsets.len();
        let mut i = 0;
        vector_update(left.get_offset_mut(depth-1)?,right.get_offset(depth-1)?,filter,|v| {
            i += 1;
            *v+offsets[i%off_len]
        });
        vector_update(left.get_length_mut(depth-1)?,right.get_length(depth-1)?,filter,|v| *v);
    }
    Ok(())
}

fn copy_shallow<'d>(left: &mut WriteVec<'d>, right: &SharedVec, filter: &[usize]) -> Result<(),String> {
    for _ in 0..filter.len() {
        let data = vector_update_poly(left.take_data()?,right.get_data(),filter)?;
        left.replace_data(data)?;
    }
    Ok(())
}

pub fn copy_vector<'d>(left: &mut WriteVec<'d>, right: &SharedVec, filter: &[usize]) -> Result<(),String> {
    if left.depth() > 0 {
        copy_deep(left,right,filter)?;
    } else {
        copy_shallow(left,right,filter)?;
    }
    Ok(())
}

/// XXX ban multi-Lvalue
fn assign_filtered(context: &mut InterpContext, sig: &RegisterSignature, regs: &Vec<Register>) -> Result<(),String> {
    let filter_reg = context.registers().get_indexes(&regs[0])?;
    let vrs = RegisterVectorSource::new(&regs);
    /* build rhs then lhs (to avoid cow panics) */
    let rights = sig[2].iter().map(|vr| SharedVec::new(context,&vrs,vr.1)).collect::<Result<Vec<_>,_>>()?;
    let mut lefts = sig[1].iter().map(|vr| WriteVec::new(context,&vrs,vr.1)).collect::<Result<Vec<_>,_>>()?;
    /* copy */
    for (left,right) in lefts.iter_mut().zip(rights.iter()) {
        copy_vector(left,right,&filter_reg)?;
        left.write(context)?;
    }
    Ok(())
}

fn assign(context: &mut InterpContext, filtered: bool, purposes: &RegisterSignature, regs: &Vec<Register>) -> Result<(),String> {
    if filtered {
        assign_filtered(context,purposes,regs)?;
    } else {
        assign_unfiltered(context,regs)?;
    }
    Ok(())
}

pub struct AssignCommandType();

impl CommandType for AssignCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std("assign"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(AssignCommand(sig[0].get_mode() != MemberMode::LValue,sig.clone(),it.regs.to_vec())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[2],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        let sig = RegisterSignature::deserialize(&value[1],false,false)?;
        Ok(Box::new(AssignCommand(cbor_bool(&value[0])?,sig,regs)))
    }
}

pub struct AssignCommand(pub(crate) bool, pub(crate) RegisterSignature, pub(crate) Vec<Register>);

impl Command for AssignCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        assign(context,self.0,&self.1,&self.2)?;
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        let regs = CborValue::Array(self.2.iter().map(|x| x.serialize()).collect());
        Ok(vec![CborValue::Bool(self.0),self.1.serialize(false,false)?,regs])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> {
        if !context.get_reg_valid(&self.2[0]) {
            return Ok(false);
        }
        for idx in self.1[2].all_registers() {
            if !context.get_reg_valid(&self.2[idx]) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(self.1[1].all_registers().iter().map(|x| self.2[*x]).collect()))
    }

    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> { 
        if !self.0 && preimage_possible(context,&self.2)? {
            Ok(PreImageOutcome::Replace(preimage_instrs(&self.2)?))
        } else {
            Ok(if self.simple_preimage(context)? {
                self.execute(context.context())?;
                self.preimage_post(context)?
            } else {
                PreImageOutcome::Skip
            })
        }
    }
}

fn extend(context: &mut InterpContext, sig: &RegisterSignature, regs: &Vec<Register>) -> Result<(),String> {
    let vrs = RegisterVectorSource::new(&regs);
    let bb = sig[2].iter().map(|vr| SharedVec::new(context,&vrs,vr.1)).collect::<Result<Vec<_>,_>>()?;
    let mut zz = vec![];
    for (vr_z,vr_a) in sig[0].iter().zip(sig[1].iter()) {
        vector_register_copy(context,&vrs,vr_z.1,vr_a.1)?;
    }
    for vr_z in sig[0].iter() {
        zz.push(WriteVec::new(context,&vrs,vr_z.1)?);
    }
    for (z,b) in zz.iter_mut().zip(bb.iter()) {
        let depth = z.depth();
        if depth > 0 {
            let offset = vector_push(z,b,1)?[0];
            vector_append(z.get_offset_mut(depth-1)?,b.get_offset(depth-1)?, |v| *v+offset);
            vector_append(z.get_length_mut(depth-1)?,b.get_length(depth-1)?, |v| *v);
        } else {
            let data = append_data(z.take_data()?,b.get_data())?.0;
            z.replace_data(data)?;
        }
        z.write(context)?;
    }
    Ok(())
}

pub struct ExtendCommandType();

impl CommandType for ExtendCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(std("extend"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(ExtendCommand(sig.clone(),it.regs.to_vec())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[1],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        let sig = RegisterSignature::deserialize(&value[0],false,false)?;
        Ok(Box::new(ExtendCommand(sig,regs)))
    }
}

pub struct ExtendCommand(pub(crate) RegisterSignature, pub(crate) Vec<Register>);

impl Command for ExtendCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        extend(context,&self.0,&self.1)?;
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        let regs = CborValue::Array(self.1.iter().map(|x| x.serialize()).collect());
        Ok(vec![self.0.serialize(false,false)?,regs])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> {
        for pos in 1..3 {
            for idx in self.0[pos].all_registers() {
                if !context.get_reg_valid(&self.1[idx]) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(self.0[0].all_registers().iter().map(|x| self.1[*x]).collect()))
    }
}

// TODO filtered-assign rewrite
pub(super) fn library_assign_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("assign",9,AssignCommandType())?;
    set.push("extend",10,ExtendCommandType())?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::generate;
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_librarysuite_builder };

    #[test]
    fn extend_smoke() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:std/extend").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&linker,&config).expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
    }
}