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

use crate::interp::InterpNatural;
use crate::model::{ Register, VectorRegisters, RegisterSignature, cbor_array, ComplexPath, Identifier, cbor_make_map, ComplexRegisters };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, CommandSetId, InterpContext, StreamContents, PreImageOutcome, Stream, PreImagePrepare, InterpValue, RegisterFile };
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use serde_cbor::Value as CborValue;
use super::numops::library_numops_commands;
use super::eq::library_eq_command;
use super::assign::library_assign_commands;
use super::vector::library_vector_commands;
use crate::cli::Config;
use crate::typeinf::{ MemberMode, BaseType };
use crate::interp::{ CompilerLink, TimeTrialCommandType, trial_write, trial_signature, TimeTrial };
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

fn print_simple(file: &RegisterFile, vr: &VectorRegisters, regs: &[Register], path: &[usize], first: usize) -> Result<String,String> {
    let (reg,offset) = vr_lookup_data(file,regs,vr,path,first)?;
    let data = file.get(&reg);
    let data = data.borrow();
    let data = data.get_shared()?;
    Ok(match data.get_natural() {
        InterpNatural::Empty => "".to_string(),
        InterpNatural::Indexes => format!("{}",data.to_rc_indexes()?.0[offset]),
        InterpNatural::Numbers => format!("{}",data.to_rc_numbers()?.0[offset]),
        InterpNatural::Boolean => format!("{}",data.to_rc_boolean()?.0[offset]),
        InterpNatural::Strings => format!("\"{}\"",data.to_rc_strings()?.0[offset]),
        InterpNatural::Bytes => format!("\'{}\'",data.to_rc_bytes()?.0[offset].iter().map(|x| format!("{:02x}",x)).collect::<Vec<_>>().join("")),
    })
}

fn any_vr(xs: &XStructure) -> &VectorRegisters {
    match xs {
        XStructure::Vector(inner) => any_vr(inner),
        XStructure::Struct(_,kvs) => any_vr(kvs.iter().next().unwrap().1),
        XStructure::Enum(_,_,kvs,_) => any_vr(kvs.iter().next().unwrap().1),
        XStructure::Simple(vr) => vr
    }
}

fn vr_lookup_data(file: &RegisterFile, regs: &[Register], vr: &VectorRegisters, path: &[usize], first: usize) -> Result<(Register,usize),String> {
    let mut position = first;
    for (i,index) in path.iter().enumerate() {
        let offset_reg = &regs[vr.offset_pos(vr.depth()-1-i)?];
        let offset_val = file.get_indexes(offset_reg)?;
        position = offset_val[position] + index;
    }
    Ok((regs[vr.data_pos()].clone(),position))
}

fn vr_lookup_len(file: &RegisterFile, regs: &[Register], vr: &VectorRegisters, path: &[usize], first: usize) -> Result<usize,String> {
    let mut position = first;
    for (i,index) in path.iter().enumerate() {
        let offset_reg = &regs[vr.offset_pos(vr.depth()-1-i)?];
        let offset_val = file.get_indexes(offset_reg)?;
        position = offset_val[position] + index;
    }
    let len_reg = &regs[vr.length_pos(vr.depth()-1-path.len())?];
    let len_val = file.get_indexes(len_reg)?;
    Ok(len_val[position])
}

// TODO
/*
* multival print
* sharedvec etc
* types
*/
fn print(file: &RegisterFile, xs: &XStructure, regs: &[Register], path: &[usize], first: usize) -> Result<String,String> {
    Ok(match xs {
        XStructure::Vector(xs_inner) => {
            let vr = any_vr(xs);
            let len = vr_lookup_len(file,regs,vr,path,first)?;
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
            let kvs : Vec<(String,XStructure)> = subs.drain(..).map(|k| (k.clone(),kvs.get(&k).unwrap().clone())).collect();
            let out = kvs.iter().map(|(name,xs_inner)| 
                Ok(format!("{}: {}",name,print(file,xs_inner,regs,path,first)?))
            ).collect::<Result<Vec<_>,String>>()?;
            format!("{} {{ {} }}",id.to_string(),out.join(", "))
        },
        XStructure::Enum(id,order,kvs,disc) => {
            let (disc_reg,offset) = vr_lookup_data(file,regs,disc,path,first)?;
            let disc_val = file.get_indexes(&disc_reg)?[offset];
            let inner_xs = kvs.get(&order[disc_val]).ok_or_else(|| format!("bad enum"))?;
            format!("{}:{} {}",id,order[disc_val],print(file,inner_xs,regs,path,first)?)
        },
        XStructure::Simple(vr) => print_simple(file,vr,regs,path,first)?,
    })
}

pub struct PrintCommand(Vec<Register>,RegisterSignature);

impl Command for PrintCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let xs = to_xstructure(&self.1[0])?;
        let vr = any_vr(&xs);
        let pos_reg = if vr.depth() > 0 { vr.offset_pos(vr.depth()-1)? } else { vr.data_pos() };
        let registers = context.registers();
        let num = registers.len(&self.0[pos_reg])?;
        let mut out = vec![];
        for i in 0..num {
            out.push(print(&registers,&xs,&self.0,&vec![],i)?);
        }
        for s in &out {
            std_stream(context)?.add(StreamContents::String(s.to_string()));
        }
        Ok(())
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
