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

use std::rc::Rc;
use crate::interp::InterpNatural;
use crate::model::{ Register, VectorRegisters, RegisterSignature, cbor_array, ComplexPath, Identifier, cbor_make_map, ComplexRegisters };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, CommandSetId, InterpContext, StreamContents, PreImageOutcome, Stream, PreImagePrepare, InterpValue, RegisterFile, InterpCommand };
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use serde_cbor::Value as CborValue;
use super::numops::library_numops_commands;
use super::eq::library_eq_command;
use super::assign::library_assign_commands;
use super::vector::library_vector_commands;
use crate::cli::Config;
use crate::typeinf::{ MemberMode, BaseType };
use crate::interp::{ CompilerLink, TimeTrialCommandType, trial_write, trial_signature, TimeTrial };
use super::super::common::vectorsource::{ RegisterVectorSource };
use super::super::common::sharedvec::SharedVec;
use crate::commands::{ to_xstructure, XStructure };

// XXX dedup
pub fn std_stream(context: &mut InterpContext) -> Result<&mut Stream,String> {
    let p = context.payload("std","stream")?;
    Ok(p.downcast_mut().ok_or_else(|| "No stream context".to_string())?)
}

pub struct PrintCommandType();

impl CommandType for PrintCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(Identifier::new("std","print"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(PrintCommand(it.regs.clone(),sig.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[0],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        let sig = RegisterSignature::deserialize(value[1],true,true)?;
        Ok(Box::new(PrintCommand(regs,sig)))
    }
}

fn print_simple(sv: &SharedVec, path: &[usize], first: usize) -> Result<String,String> {
    let (data,offset) = vr_lookup_data(sv,path,first)?;
    Ok(match data.get_natural() {
        InterpNatural::Empty => "".to_string(),
        InterpNatural::Indexes => format!("{}",data.to_rc_indexes()?.0[offset]),
        InterpNatural::Numbers => format!("{}",data.to_rc_numbers()?.0[offset]),
        InterpNatural::Boolean => format!("{}",data.to_rc_boolean()?.0[offset]),
        InterpNatural::Strings => format!("\"{}\"",data.to_rc_strings()?.0[offset]),
        InterpNatural::Bytes => format!("\'{}\'",data.to_rc_bytes()?.0[offset].iter().map(|x| format!("{:02x}",x)).collect::<Vec<_>>().join("")),
    })
}

fn vr_lookup_data(sv: &SharedVec, path: &[usize], first: usize) -> Result<(Rc<InterpValue>,usize),String> {
    let mut position = first;
    for (i,index) in path.iter().enumerate() {
        let offset_val = sv.get_offset(sv.depth()-1-i)?;
        position = offset_val[position] + index;
    }
    Ok((sv.get_data().clone(),position))
}

fn vr_lookup_len(sv: &SharedVec, path: &[usize], first: usize) -> Result<usize,String> {
    let mut position = first;
    for (i,index) in path.iter().enumerate() {
        let offset_val = sv.get_offset(sv.depth()-1-i)?;
        position = offset_val[position] + index;
    }
    let len_val = sv.get_length(sv.depth()-1-path.len())?;
    Ok(len_val[position])
}

fn print(file: &RegisterFile, xs: &XStructure<SharedVec>, regs: &[Register], path: &[usize], first: usize) -> Result<String,String> {
    Ok(match xs {
        XStructure::Vector(xs_inner) => {
            let sv = xs.any();
            let len = vr_lookup_len(&sv,path,first)?;
            let mut out = vec![];
            for i in 0..len {
                let mut new_path = path.to_vec();
                new_path.push(i);
                out.push(print(file,xs_inner,regs,&new_path,first)?);
            }
            format!("[{}]",out.join(", "))
        },
        XStructure::Struct(id,kvs) => {
            let mut subs : Vec<String> = kvs.keys().cloned().collect();
            subs.sort();
            let kvs : Vec<(String,_)> = subs.drain(..).map(|k| (k.clone(),kvs.get(&k).unwrap().clone())).collect();
            let out = kvs.iter().map(|(name,xs_inner)| 
                Ok(format!("{}: {}",name,print(file,xs_inner,regs,path,first)?))
            ).collect::<Result<Vec<_>,String>>()?;
            format!("{} {{ {} }}",id.to_string(),out.join(", "))
        },
        XStructure::Enum(id,order,kvs,disc) => {
            let (data,offset) = vr_lookup_data(&disc.borrow(),path,first)?;
            let disc_val = data.to_rc_indexes()?.0[offset];
            let inner_xs = kvs.get(&order[disc_val]).ok_or_else(|| format!("bad enum"))?;
            format!("{}:{} {}",id,order[disc_val],print(file,inner_xs,regs,path,first)?)
        },
        XStructure::Simple(sv) => print_simple(&sv.borrow(),path,first)?,
    })
}

pub struct PrintInterpCommand(Vec<Register>,RegisterSignature);

impl InterpCommand for PrintInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let xs = to_xstructure(&self.1[0])?;
        let vs = RegisterVectorSource::new(&self.0);
        let xs2 = xs.derive(&mut (|vr: &VectorRegisters| SharedVec::new(context,&vs,vr)))?;
        let sv = xs2.any();
        let num = if sv.depth() > 0 { sv.get_offset(sv.depth()-1)?.len() } else { sv.get_data().len() };
        let registers = context.registers();
        let mut out = vec![];
        for i in 0..num {
            out.push(print(&registers,&xs2,&self.0,&vec![],i)?);
        }
        for s in &out {
            std_stream(context)?.add(StreamContents::String(s.to_string()));
        }
        Ok(())
    }
}

pub struct PrintCommand(Vec<Register>,RegisterSignature);

impl Command for PrintCommand {
    fn to_interp_command(&self) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(PrintInterpCommand(self.0.clone(),self.1.clone())))
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        let regs = CborValue::Array(self.0.iter().map(|x| x.serialize()).collect());
        Ok(Some(vec![regs,self.1.serialize(true,true)?]))
    }    
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::generate;
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_librarysuite_builder };

    #[test]
    fn print_smoke() {
        let mut config = xxx_test_config();
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:std/print").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(&vec![
            "[print::test3 { A: [[1, 1], [1, 2, 3], [4, 5, 6], [7, 8, 9], [1, 1]], B: [] }, print::test3 { A: [[7], [6], [5]], B: [[4]] }]",
            "[buildtime::version { major: 0, minor: 1 }, buildtime::version { major: 0, minor: 0 }, buildtime::version { major: 0, minor: 0 }]",
            "[print::test { x: [false, true] }, print::test { x: [true, false] }]",
            "[print::test2:A [true, true], print::test2:B [[0], [1, 2, 3]], print::test2:C false, print::test2:A [false]]",
            "1", "2", "3",
            "\'4241030040\'"
        ].iter().map(|x| x.to_string()).collect::<Vec<_>>(),&strings);
    }
}
