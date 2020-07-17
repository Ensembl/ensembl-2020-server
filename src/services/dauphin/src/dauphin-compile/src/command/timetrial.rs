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

use std::time::{ SystemTime, Duration };
use crate::cli::Config;
use crate::command::{ CompilerLink, Instruction };
use serde_cbor::Value as CborValue;
use dauphin_interp::command::{ InterpCommand };
use dauphin_interp::runtime::{ InterpContext, InterpValue, Register };
use dauphin_interp::types::{ RegisterSignature, ComplexPath, FullType, VectorRegisters, MemberMode, BaseType };
use dauphin_interp::util::cbor::{ cbor_array, cbor_float };

pub fn regress(input: &[(f64,f64)]) -> Result<(f64,f64),String> {
    if input.len() == 0 {
        return Err("no data to regress".to_string());
    }
    let total_x : f64 = input.iter().map(|x| x.0).sum();
    let total_y : f64 = input.iter().map(|x| x.1).sum();
    let mean_x = total_x / input.len() as f64;
    let mean_y = total_y / input.len() as f64;
    let mut numer = 0.;
    let mut denom = 0.;
    for (x,y) in input {
        let x_delta = *x as f64 - mean_x;
        let y_delta = y         - mean_y;
        numer += x_delta*y_delta;
        denom += x_delta*x_delta;
    }
    if denom == 0. {
        return Err("no x-variance to regress".to_string());
    }
    let grad = numer/denom;
    let icept = mean_y - grad * mean_x;
    Ok((grad,icept))
}

fn run_time_trial(command_type: &dyn TimeTrialCommandType, icom: &Box<dyn InterpCommand>, linker: &CompilerLink, _config: &Config, t: i64, loops: i64, dry: bool) -> Result<f64,String> {
    let mut context = linker.new_context();
    command_type.global_prepare(&mut context,t);
    let start_time = SystemTime::now();
    if dry {
        for _ in 0..loops {
            command_type.local_prepare(&mut context,t);
            context.registers_mut().commit();
        }
    } else {
        for _ in 0..loops {
            command_type.local_prepare(&mut context,t);
            context.registers_mut().commit();
            icom.execute(&mut context)?;
            context.registers_mut().commit();
        }
    }
    Ok(start_time.elapsed().unwrap_or(Duration::new(0,0)).as_secs_f64()*1000.)
}

fn generate_one_timing(trial: &dyn TimeTrialCommandType, linker: &CompilerLink, config: &Config, param: i64, block: i64) -> Result<f64,String> {
    let mut data = vec![];
    for i in 0..5 {
        let instr = trial.timetrial_make_command(param,linker,config)?;
        let interp_command = linker.instruction_to_interp_command(&instr)?.ok_or_else(|| format!("cannot run time trial"))?;
        let run = run_time_trial(trial,&interp_command,linker,config,param,i*block,false)?;
        let dry = run_time_trial(trial,&interp_command,linker,config,param,i*block,true)?;
        data.push(((i*block) as f64,run-dry));
        if config.get_verbose() > 2 {
            print!("loops={} time={:.2}ms\n",i*block,run-dry);
        }
    }
    Ok(regress(&data)?.0)
}

#[derive(Debug,Clone)]
pub struct TimeTrial(pub f64,pub f64);

impl TimeTrial {
    pub fn run(command: &dyn TimeTrialCommandType, linker: &CompilerLink, config: &Config) -> Result<TimeTrial,String> {
        let block = if config.get_unit_test() { 10 } else { 100 };
        let trial = command.timetrial_make_trials();
        let mut data = vec![];
        for axis_val in (trial.0)..(trial.1) {
            if config.get_verbose() > 2 {
                print!("param {:?}\n",axis_val);
            }
            let r = generate_one_timing(command,linker,config,axis_val,block)?;
            if config.get_verbose() > 2 {
                print!("takes {:.3}ms with param={:?}\n",r,axis_val);
            }
            data.push((axis_val as f64,r));
        }
        let (m,c) = if data.len() > 1 { regress(&data)? } else { (0.,data[0].1) };
        if config.get_verbose() > 1 {
            print!("trend m={:.4} c={:.4}\n",m,c);
        }
        Ok(TimeTrial(m,c))
    }

    pub fn evaluate(&self, x: f64) -> f64 { self.0*x+self.1 }

    pub fn serialize(&self) -> CborValue {
        CborValue::Array(vec![CborValue::Float(self.0),CborValue::Float(self.1)])
    }

    pub fn deserialize(value: &CborValue) -> Result<TimeTrial,String> {
        let data = cbor_array(value,2,false)?;
        Ok(TimeTrial(cbor_float(&data[0])?,cbor_float(&data[1])?))
    }
}

pub trait TimeTrialCommandType {
    fn timetrial_make_trials(&self) -> (i64,i64);
    fn local_prepare(&self, _context: &mut InterpContext, _: i64) {}
    fn global_prepare(&self, _context: &mut InterpContext, _: i64) {}

    fn timetrial_make_command(&self, instance: i64, linker: &CompilerLink, config: &Config) -> Result<Instruction,String>;
}


pub fn trial_write<F>(context: &mut InterpContext, i: usize, t: usize, cb: F) where F: Fn(usize) -> usize {
    let a : Vec<usize> = (0..t).map(|x| cb(x as usize)).collect();
    context.registers_mut().write(&Register(i),InterpValue::Indexes(a));
}

pub fn trial_signature(layout: &[(MemberMode,usize,BaseType)]) -> RegisterSignature {
    let mut sigs = RegisterSignature::new();
    for (mode,depth,base) in layout {
        let mut cr = FullType::new_empty(*mode);
        cr.add(ComplexPath::new_empty().add_levels(*depth),VectorRegisters::new(*depth,base.clone()));
        sigs.add(cr);
    }
    sigs
}
