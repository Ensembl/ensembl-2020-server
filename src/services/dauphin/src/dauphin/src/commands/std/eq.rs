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

use crate::interp::InterpValue;
use crate::model::{ Register, RegisterSignature, cbor_make_map };
use super::super::common::vectorsource::RegisterVectorSource;
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, InterpContext, PreImageOutcome, trial_write, trial_signature, TimeTrialCommandType, TimeTrial };
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use serde_cbor::Value as CborValue;
use crate::typeinf::MemberMode;
use crate::model::cbor_array;
use crate::commands::common::sharedvec::SharedVec;
use super::library::std;
use crate::cli::Config;
use crate::interp::CompilerLink;


use std::fmt::Debug;
use crate::commands::common::polymorphic::arbitrate_type;

fn compare_work<T>(a: &SharedVec, a_off: (usize,usize), a_data: &[T], b: &SharedVec, b_off: (usize,usize), b_data: &[T], level: usize) -> Result<bool,String>
        where T: PartialEq {
    if a_off.1 != b_off.1 { return Ok(false); }
    if level > 0 {
        /* index with index below */
        let lower_a_off = a.get_offset(level-1)?;
        let lower_a_len = a.get_length(level-1)?;
        let lower_b_off = b.get_offset(level-1)?;
        let lower_b_len = b.get_length(level-1)?;
        for i in 0..a_off.1 {
            if !compare_work(a,(lower_a_off[a_off.0+i],lower_a_len[a_off.0+i]),a_data,
                                b,(lower_b_off[b_off.0+i],lower_b_len[b_off.0+i]),b_data,
                                level-1)? {
                return Ok(false);
            }
        }
    } else {
        /* index with data below */
        for i in 0..a_off.1 {
            if a_data[a_off.0+i] != b_data[b_off.0+i] {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn compare_indexed<T>(a: &SharedVec, b: &SharedVec, a_data: &[T], b_data: &[T]) -> Result<Vec<bool>,String> where T: PartialEq + Debug {
    let top_a_off = a.get_offset(a.depth()-1)?;
    let top_a_len = a.get_length(a.depth()-1)?;
    let top_b_off = b.get_offset(b.depth()-1)?;
    let top_b_len = b.get_length(b.depth()-1)?;
    let b_len = top_b_off.len();
    let mut out = vec![];
    for i in 0..top_a_off.len() {
        out.push(compare_work(a,(top_a_off[i],top_a_len[i]),a_data,
                              b,(top_b_off[i%b_len],top_b_len[i%b_len]),b_data,
                              a.depth()-1)?);
    }
    Ok(out)
}

fn compare_data<T>(a: &[T], b: &[T]) -> Vec<bool> where T: PartialEq {
    let b_len = b.len();
    a.iter().enumerate().map(|(i,av)| av == &b[i%b_len]).collect()
}

pub fn compare(a: &SharedVec, b: &SharedVec) -> Result<Vec<bool>,String> {
    if a.depth() != b.depth() {
        return Err(format!("unequal types in eq"));
    }
    let a_data = a.get_data();
    let b_data = b.get_data();
    if let Some(natural) = arbitrate_type(&a_data,&b_data,true) {
        Ok(polymorphic!([&a_data,&b_data],natural,(|d,s| {
            if a.depth() != 0 {
                compare_indexed(a,b,d,s)
            } else {
                Ok(compare_data(d,s))
            }
        })).transpose()?.ok_or_else(|| format!("unexpected empty in eq"))?)
    } else {
        Ok(vec![])
    }
}

struct EqDataTimeTrial();

impl TimeTrialCommandType for EqDataTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,3,t*100,|x| x);
        trial_write(context,4,t*100,|x| x);
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sigs = trial_signature(&vec![(MemberMode::RValue,1),(MemberMode::RValue,0),(MemberMode::RValue,0)]);
        let regs : Vec<Register> = (0..5).map(|x| Register(x)).collect();
        Ok(Box::new(EqCommand(sigs,regs)))
    }
}

struct EqWidthTimeTrial();

impl TimeTrialCommandType for EqWidthTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,3,t*100,|x| x);
        trial_write(context,4,1,|_| 0);
        trial_write(context,5,1,|_| t*100);
        trial_write(context,6,t*100,|x| x);
        trial_write(context,7,1,|_| 0);
        trial_write(context,8,1,|_| t*100);
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sigs = trial_signature(&vec![(MemberMode::RValue,1),(MemberMode::RValue,1),(MemberMode::RValue,1)]);
        let regs : Vec<Register> = (0..9).map(|x| Register(x)).collect();
        Ok(Box::new(EqCommand(sigs,regs)))
    }
}

struct EqHeightTimeTrial();

impl TimeTrialCommandType for EqHeightTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        for pos in 0..2 {
            let offset = 3 + (t*2+1)*pos;
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
        let regs : Vec<Register> = (0..(t*4+5)).map(|x| Register(x)).collect();
        Ok(Box::new(EqCommand(sigs,regs)))
    }
}

pub struct EqCommandType();

impl CommandType for EqCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(std("eq"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(EqCommand(sig.clone(),it.regs.to_vec())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[1],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        let sig = RegisterSignature::deserialize(&value[0],false,true)?;
        Ok(Box::new(EqCommand(sig,regs)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let eq_data = TimeTrial::run(&EqDataTimeTrial(),linker,config)?;
        let eq_width = TimeTrial::run(&EqWidthTimeTrial(),linker,config)?;
        let eq_height = TimeTrial::run(&EqHeightTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["td","tw","th"],vec![eq_data.serialize(),eq_width.serialize(),eq_height.serialize()])?)
    }
}

// TODO preimage
pub struct EqCommand(pub(crate) RegisterSignature, pub(crate) Vec<Register>);

impl Command for EqCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let vs = RegisterVectorSource::new(&self.1);
        let cr_a = &self.0[1];
        let cr_b = &self.0[2];
        let mut out : Option<Vec<bool>> = None;
        for (vr_a,vr_b) in cr_a.iter().zip(cr_b.iter()) {
            let a = SharedVec::new(context,&vs,vr_a.1)?;
            let b = SharedVec::new(context,&vs,vr_b.1)?;
            let more = compare(&a,&b)?;
            if let Some(ref mut out) = out {
                let out_len = out.len();
                for (i,value) in more.iter().enumerate() {
                    if !value {
                        out[i%out_len] = false;
                    }
                }
            } else {
                out = Some(more);
            }
        }
        context.registers().write(&self.1[0],InterpValue::Boolean(out.unwrap_or_else(|| vec![])));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        let regs = CborValue::Array(self.1.iter().map(|x| x.serialize()).collect());
        Ok(vec![self.0.serialize(false,true)?,regs])
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
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.1[0],true);
        Ok(PreImageOutcome::Constant(vec![self.1[0]]))
    }
}

pub(super) fn library_eq_command(set: &mut CommandSet) -> Result<(),String> {
    set.push("eq",0,EqCommandType())?;
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
    fn eq_smoke() {
        let mut config = xxx_test_config();
        config.set_generate_debug(false);
        config.set_verbose(2);
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:std/eq").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
    }
}
