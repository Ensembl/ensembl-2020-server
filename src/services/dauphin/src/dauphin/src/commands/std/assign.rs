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

use crate::model::{ Register, RegisterSignature, cbor_make_map };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, InterpContext, PreImageOutcome, PreImagePrepare, TimeTrialCommandType, TimeTrial, regress, trial_write, trial_signature };
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use serde_cbor::Value as CborValue;
use crate::model::{ cbor_array, cbor_bool };
use crate::typeinf::MemberMode;
use super::super::common::vectorcopy::{ vector_update_offsets, vector_update_lengths, vector_update_poly, vector_push, vector_register_copy, vector_append, append_data };
use super::super::common::vectorsource::RegisterVectorSource;
use super::super::common::sharedvec::{ SharedVec };
use super::super::common::writevec::WriteVec;
use super::library::std;
use crate::cli::Config;
use crate::interp::CompilerLink;

fn assign_unfiltered(context: &mut InterpContext, regs: &Vec<Register>) -> Result<(),String> {
    let registers = context.registers();
    let n = regs.len()/2;
    for i in 0..n {
        registers.copy(&regs[i],&regs[i+n])?;
    }
    Ok(())
}

fn preimage_instrs(regs: &Vec<Register>) -> Result<Vec<Instruction>,String> {
    let mut instrs = vec![];
    let n = regs.len()/2;
    for i in 0..n {
        instrs.push(Instruction::new(InstructionType::Copy,vec![regs[i],regs[i+n]]));
    }
    Ok(instrs)
}

fn preimage_sizes(context: &PreImageContext, regs: &Vec<Register>, offset: usize) -> Result<Vec<(Register,usize)>,String> {
    let mut out = vec![];
    let n = (regs.len()-offset)/2;
    for i in 0..n {
        if let Some(a) = context.get_reg_size(&regs[offset+i+n]) {
            out.push((regs[offset+i],a));
        }
    }
    Ok(out)
}

fn copy_deep<'d>(left: &mut WriteVec<'d>, right: &SharedVec, filter: &[usize]) -> Result<(),String> {
    if filter.len() > 0 {
        let offsets = vector_push(left,right,filter.len())?;
        let depth = left.depth();
        vector_update_offsets(left.get_offset_mut(depth-1)?,right.get_offset(depth-1)?,filter,offsets);
        vector_update_lengths(left.get_length_mut(depth-1)?,right.get_length(depth-1)?,filter);
    }
    Ok(())
}

fn copy_shallow<'d>(left: &mut WriteVec<'d>, right: &SharedVec, filter: &[usize]) -> Result<(),String> {
    let rightval = right.get_data();
    let mut leftval = left.take_data()?;
    leftval = vector_update_poly(leftval,rightval,filter)?;
    left.replace_data(leftval)?;
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

struct UnfilteredAssignTimeTrial();

impl TimeTrialCommandType for UnfilteredAssignTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        for i in 0..10 {
            trial_write(context,i,t*100,|x| x);
        }    
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sigs = trial_signature(&vec![(MemberMode::LValue,2),(MemberMode::RValue,2)]);
        let regs : Vec<Register> = (0..10).map(|x| Register(x)).collect();
        Ok(Box::new(AssignCommand(false,sigs,regs)))
    }
}

struct FilteredNumberAssignTimeTrial();

impl TimeTrialCommandType for FilteredNumberAssignTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn local_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,0,t*10,|x| x);    /* 10t writes */
        trial_write(context,1,t*100,|x| x);   /* 100t data */
        trial_write(context,2,t*10,|x| x*10); /* 10t arrays (offset 10x) */
        trial_write(context,3,t*10,|_| 10);   /* 10t arrays (len 10) */
        trial_write(context,4,t*10,|x| x);   /* 10tm data */
        trial_write(context,5,t*10,|x| x); /* 10t arrays (offset xm) */
        trial_write(context,6,t*10,|_| 1);   /* 10t arrays (len m) */
        context.registers().commit();
    }

    fn global_prepare(&self, _context: &mut InterpContext, _t: i64) {}

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sigs = trial_signature(&vec![(MemberMode::FValue,0),(MemberMode::LValue,1),(MemberMode::RValue,1)]);
        let regs : Vec<Register> = (0..7).map(|x| Register(x)).collect();
        Ok(Box::new(AssignCommand(true,sigs,regs)))
    }
}

struct FilteredDepthAssignTimeTrial();

impl TimeTrialCommandType for FilteredDepthAssignTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn local_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,0,10,|x| x);    /* 10t writes */
        trial_write(context,1,100,|x| x);   /* 100t data */
        trial_write(context,2,10,|x| x*10); /* 10t arrays (offset 10x) */
        trial_write(context,3,10,|_| 10);   /* 10t arrays (len 10) */
        trial_write(context,4,t*1000,|x| x);   /* 10tm data */
        trial_write(context,5,10,|x| t*100*x); /* 10t arrays (offset xm) */
        trial_write(context,6,10,|_| t*100);   /* 10t arrays (len m) */
        context.registers().commit();
    }

    fn global_prepare(&self, _context: &mut InterpContext, _t: i64) {}

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sigs = trial_signature(&vec![(MemberMode::FValue,0),(MemberMode::LValue,1),(MemberMode::RValue,1)]);
        let regs : Vec<Register> = (0..7).map(|x| Register(x)).collect();
        Ok(Box::new(AssignCommand(true,sigs,regs)))
    }
}

struct ShallowAssignTimeTrial();

impl TimeTrialCommandType for ShallowAssignTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn local_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,0,t*100,|x| x);    /* 100t writes */
        trial_write(context,1,t*1000,|x| x);   /* 1000t data */
        trial_write(context,2,t*100,|x| x);   /* 100t reads */
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sigs = trial_signature(&vec![(MemberMode::FValue,0),(MemberMode::LValue,0),(MemberMode::RValue,0)]);
        let regs : Vec<Register> = (0..3).map(|x| Register(x)).collect();
        Ok(Box::new(AssignCommand(true,sigs,regs)))
    }
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

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let unfiltered = TimeTrial::run(&UnfilteredAssignTimeTrial(),linker,config)?;
        let shallow = TimeTrial::run(&ShallowAssignTimeTrial(),linker,config)?;
        let filtered_number = TimeTrial::run(&FilteredNumberAssignTimeTrial(),linker,config)?;
        let filtered_depth = TimeTrial::run(&FilteredDepthAssignTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["tu","tfn","ts","tfd"],vec![unfiltered.serialize(),filtered_number.serialize(),shallow.serialize(),filtered_depth.serialize()])?)
    }
}

pub struct AssignCommand(bool,RegisterSignature,Vec<Register>);

impl AssignCommand {
    fn can_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> {
        if !context.is_reg_valid(&self.2[0]) {
            return Ok(false);
        }
        for idx in self.1[2].all_registers() {
            if !context.is_reg_valid(&self.2[idx]) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl Command for AssignCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        assign(context,self.0,&self.1,&self.2)?;
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        let regs = CborValue::Array(self.2.iter().map(|x| x.serialize()).collect());
        Ok(Some(vec![CborValue::Bool(self.0),self.1.serialize(false,false)?,regs]))
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(self.1[1].all_registers().iter().map(|x| self.2[*x]).collect()))
    }

    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> { 
        Ok(if !self.0 {
            /* unfiltered */
            PreImageOutcome::Replace(preimage_instrs(&self.2)?)
        } else {
            /* filtered */
            if self.can_preimage(context)? {
                self.execute(context.context())?;
                self.preimage_post(context)?
            } else {
                PreImageOutcome::Skip(preimage_sizes(context,&self.2,1)?)
            }
        })
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
            let offset = vector_push(z,b,1)?.0;
            vector_append(z.get_offset_mut(depth-1)?,b.get_offset(depth-1)?, |v| *v+offset);
            vector_append(z.get_length_mut(depth-1)?,b.get_length(depth-1)?, |v| *v);
        } else {
            let data = append_data(z.take_data()?,b.get_data(),1)?.0;
            z.replace_data(data)?;
        }
        z.write(context)?;
    }
    Ok(())
}

fn extend_sizes(context: &mut PreImageContext, sig: &RegisterSignature, regs: &Vec<Register>) -> Result<Vec<(Register,usize)>,String> {
    let mut out = vec![];
    for (((_,a),(_,b)),(_,z)) in sig[1].iter().zip(sig[2].iter()).zip(sig[0].iter()) {
        let depth = z.depth();
        if depth > 0 {
            for level in 0..depth {
                if let (Some(x),Some(y)) = (context.get_reg_size(&regs[a.offset_pos(level)?]),context.get_reg_size(&regs[b.offset_pos(level)?])) {
                    out.push((regs[z.offset_pos(level)?],x+y));
                }
                if let (Some(x),Some(y)) = (context.get_reg_size(&regs[a.length_pos(level)?]),context.get_reg_size(&regs[b.length_pos(level)?])) {
                    out.push((regs[z.length_pos(level)?],x+y));
                }
            }
        }
        if let (Some(x),Some(y)) = (context.get_reg_size(&regs[a.data_pos()]),context.get_reg_size(&regs[b.data_pos()])) {
            out.push((regs[z.data_pos()],x+y));
        }
    }
    Ok(out)
}

struct ExtendDataTimeTrial(bool); /* true = right, false = left */

impl TimeTrialCommandType for ExtendDataTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn local_prepare(&self, context: &mut InterpContext, t: i64) {
        let (left,right) = if self.0 { (1,t as usize*100) } else { (t as usize*100,1) };
        trial_write(context,1,left,|x| x);
        trial_write(context,2,right,|x| x);
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sigs = trial_signature(&vec![(MemberMode::RValue,0),(MemberMode::RValue,0),(MemberMode::RValue,0)]);
        let regs : Vec<Register> = (0..3).map(|x| Register(x)).collect();
        Ok(Box::new(ExtendCommand(sigs,regs)))
    }
}

struct ExtendBaseTimeTrial(bool); /* true = right, false = left */

impl TimeTrialCommandType for ExtendBaseTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let (left,right) = if self.0 { (1,t as usize*100) } else { (t as usize*100,1) };
        trial_write(context,3,left,|x| x);
        trial_write(context,4,1,|_| 0);
        trial_write(context,5,1,|_| left);
        trial_write(context,6,right,|x| x);
        trial_write(context,7,1,|_| 0);
        trial_write(context,8,1,|_| right);
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sigs = trial_signature(&vec![(MemberMode::RValue,1),(MemberMode::RValue,1),(MemberMode::RValue,1)]);
        let regs : Vec<Register> = (0..9).map(|x| Register(x)).collect();
        Ok(Box::new(ExtendCommand(sigs,regs)))
    }
}

struct ExtendHeightTimeTrial();

impl TimeTrialCommandType for ExtendHeightTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        for pos in 1..3 {
            let offset = (t*2+1)*pos;
            trial_write(context,offset,t*100,|x| x);
            trial_write(context,offset+1,1,|_| 0);
            trial_write(context,offset+2,1,|_| 100);
            for layer in 1..t {
                trial_write(context,offset+(2*layer)+1,1,|_| 0);
                trial_write(context,offset+(2*layer)+2,1,|_| 1);
            }
        }
        context.registers().commit();
    }

    fn timetrial_make_command(&self, t: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let t = t as usize;
        let sigs = trial_signature(&vec![(MemberMode::RValue,1),(MemberMode::RValue,t),(MemberMode::RValue,t)]);
        let regs : Vec<Register> = (0..(t*6+3)).map(|x| Register(x)).collect();
        Ok(Box::new(ExtendCommand(sigs,regs)))
    }
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

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let left_data_time = TimeTrial::run(&ExtendDataTimeTrial(false),linker,config)?;
        let right_data_time = TimeTrial::run(&ExtendDataTimeTrial(true),linker,config)?;
        let left_vec_time = TimeTrial::run(&ExtendBaseTimeTrial(false),linker,config)?;
        let right_vec_time = TimeTrial::run(&ExtendBaseTimeTrial(true),linker,config)?;
        let height_time = TimeTrial::run(&ExtendHeightTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["tdl","tdr","tvl","tvr","th"],vec![
            left_data_time.serialize(),right_data_time.serialize(),
            left_vec_time.serialize(),right_vec_time.serialize(),
            height_time.serialize()
        ])?)
    }
}

pub struct ExtendCommand(pub(crate) RegisterSignature, pub(crate) Vec<Register>);

impl ExtendCommand {
    fn can_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> {
        for pos in 1..3 {
            for idx in self.0[pos].all_registers() {
                if !context.is_reg_valid(&self.1[idx]) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

impl Command for ExtendCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        extend(context,&self.0,&self.1)?;
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        let regs = CborValue::Array(self.1.iter().map(|x| x.serialize()).collect());
        Ok(Some(vec![self.0.serialize(false,false)?,regs]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> {
        Ok(if self.can_preimage(context)? {
            PreImagePrepare::Replace
        } else {
            PreImagePrepare::Keep(extend_sizes(context,&self.0,&self.1)?)
        })
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
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:std/extend").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
    }

    #[test]
    fn assign_filtered() {
        let config = xxx_test_config();
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:std/filterassign").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        // XXX todo test it!
    }
}
