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
use crate::model::{ cbor_array, cbor_bool, cbor_map };
use crate::typeinf::MemberMode;
use super::super::common::vectorcopy::{ vector_update_offsets, vector_update_lengths, vector_update_poly, vector_push, vector_register_copy, vector_append, append_data };
use super::super::common::vectorsource::RegisterVectorSource;
use super::super::common::sharedvec::{ SharedVec };
use super::super::common::writevec::WriteVec;
use super::library::std;
use crate::cli::Config;
use crate::interp::CompilerLink;

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

struct ExtendTimeTrial();

impl TimeTrialCommandType for ExtendTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize * 100;
        trial_write(context,3,t,|x| x);
        trial_write(context,4,1,|_| 0);
        trial_write(context,5,1,|_| t);
        trial_write(context,6,t,|x| x);
        trial_write(context,7,1,|_| 0);
        trial_write(context,8,1,|_| t);
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sigs = trial_signature(&vec![(MemberMode::RValue,1),(MemberMode::RValue,1),(MemberMode::RValue,1)]);
        let regs : Vec<Register> = (0..9).map(|x| Register(x)).collect();
        Ok(Box::new(ExtendCommand(sigs,regs,None)))
    }
}

pub struct ExtendCommandType(Option<TimeTrial>);

impl ExtendCommandType {
    pub fn new() -> ExtendCommandType { ExtendCommandType(None) }
}

impl CommandType for ExtendCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(std("extend"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(ExtendCommand(sig.clone(),it.regs.to_vec(),self.0.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[1],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        let sig = RegisterSignature::deserialize(&value[0],false,false)?;
        Ok(Box::new(ExtendCommand(sig,regs,self.0.clone())))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&ExtendTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct ExtendCommand(RegisterSignature,Vec<Register>,Option<TimeTrial>);

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

    fn calc_length(&self, context: &PreImageContext, sig: &RegisterSignature, regs: &[Register]) -> Result<Option<usize>,String> {
        let mut length = 0;
        for idx in 1..2 {
            for (_,a) in sig[idx].iter() {
                for level in 0..a.depth() {
                    let more = context.get_reg_size(&regs[a.offset_pos(level)?]);
                    if let Some(more) = more { length += more } else { return Ok(None); }
                    let more = context.get_reg_size(&regs[a.length_pos(level)?]);
                    if let Some(more) = more { length += more } else { return Ok(None); }
                }
                let more = context.get_reg_size(&regs[a.data_pos()]);
                if let Some(more) = more { length += more } else { return Ok(None); }
            }
        }
        Ok(Some(length))
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

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Ok(Some(length)) = self.calc_length(context,&self.0,&self.1) {
            self.2.as_ref().map(|x| x.evaluate(length as f64/200.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::{ generate, InstructionType, Instruction };
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
        let mut prev : Option<Instruction> = None;
        for instr in &instrs {
            if let InstructionType::Call(id,_,_,_) = &instr.itype {
                if id.name() == "extend" {
                    if let Some(prev) = prev {
                        assert_ne!(InstructionType::Pause,prev.itype);
                    }
                }
            }
            prev = Some(instr.clone());
        }
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
    }
}