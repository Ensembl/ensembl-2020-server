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

use std::collections::{ HashMap };
use std::rc::Rc;
use crate::commands::{ make_core, make_library };
use crate::generate::GenContext;
use crate::model::Register;
use crate::interp::context::InterpContext;
use crate::interp::CommandSuiteBuilder;
use crate::interp::{ InterpValue, StreamContents };
use super::compilelink::CompilerLink;
use super::interplink::InterpreterLink;
use crate::test::cbor::hexdump;

fn stream_strings(stream: &[StreamContents]) -> Vec<String> {
    let mut out = vec![];
    for s in stream {
        match s {
            StreamContents::String(s) => out.push(s.to_string()),
            _ => {}
        }
    }
    out
}

fn export_indexes(ic: &mut InterpContext) -> Result<HashMap<Register,Vec<usize>>,String> {
    let mut out = HashMap::new();
    for (r,iv) in ic.registers().export()?.iter() {
        let iv = Rc::new(iv.copy());
        let v = InterpValue::to_rc_indexes(&iv).map(|x| x.0.to_vec()).unwrap_or(vec![]);
        out.insert(*r,v);
    }
    Ok(out)
}

pub fn mini_interp_run(context: &GenContext, cl: &CompilerLink, ic: &mut InterpContext) -> Result<(HashMap<Register,Vec<usize>>,Vec<String>),String> {
    let program = cl.serialize(&context.get_instructions(),false)?;
    let mut buffer = Vec::new();
    serde_cbor::to_writer(&mut buffer,&program).expect("cbor b");
    print!("{}\n",hexdump(&buffer));

    let mut suite = CommandSuiteBuilder::new();
    suite.add(make_core()?)?;
    suite.add(make_library()?)?;
    let interpret_linker = InterpreterLink::new(suite,&program).map_err(|x| format!("{} while linking",x))?;

    
    let mut instrs = interpret_linker.get_instructions().unwrap().iter();
    for command in interpret_linker.get_commands() {
        let (instr,regs) = instrs.next().unwrap();
        print!("{}",ic.registers().dump_many(&regs)?);
        print!("{}",instr);
        command.execute(ic)?;
        ic.registers().commit();
        print!("{}",ic.registers().dump_many(&regs)?);
    }
    Ok((export_indexes(ic)?,stream_strings(&ic.stream_take())))
}

pub fn xxx_compiler_link() -> Result<CompilerLink,String> {
    let mut suite = CommandSuiteBuilder::new();
    suite.add(make_core()?)?;
    suite.add(make_library()?)?;
    CompilerLink::new(suite)
}

pub fn mini_interp(context: &GenContext, cl: &CompilerLink) -> Result<(HashMap<Register,Vec<usize>>,Vec<String>),String> {
    let mut ic = InterpContext::new();
    mini_interp_run(context,&cl,&mut ic).map_err(|x| {
        let line = ic.get_line_number();
        if line.1 != 0 {
            format!("{} at {}:{}",x,line.0,line.1)
        } else {
            x
        }
    })
}