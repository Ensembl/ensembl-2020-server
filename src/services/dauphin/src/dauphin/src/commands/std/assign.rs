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

use dauphin_interp_common::common::{ Register, RegisterSignature, VectorRegisters, InterpCommand, MemberMode };
use dauphin_interp_common::interp::InterpLibRegister;
use crate::interp::{
    Command, CommandSchema, CommandType, CommandTrigger, PreImageOutcome,
    CompLibRegister
};
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use serde_cbor::Value as CborValue;
use super::super::common::vectorcopy::{ vector_push_instrs, vector_update_offsets, vector_update_lengths, vector_copy };
use super::extend::ExtendCommandType;
use super::library::std;

fn preimage_instrs(regs: &Vec<Register>) -> Result<Vec<Instruction>,String> {
    let mut instrs = vec![];
    let n = regs.len()/2;
    for i in 0..n {
        instrs.push(Instruction::new(InstructionType::Copy,vec![regs[i],regs[i+n]]));
    }
    Ok(instrs)
}

fn copy_deep_instrs<'d>(context: &mut PreImageContext, left: &VectorRegisters, right: &VectorRegisters, filter: &Register, regs: &[Register]) -> Result<Vec<Instruction>,String> {
    let mut out = vec![];
    let depth = left.depth();
    let start = context.new_register();
    let reg_off = if depth > 1 { left.offset_pos(depth-2)? } else { left.data_pos() };
    out.push(Instruction::new(InstructionType::Length,vec![start,regs[reg_off]]));
    let stride = context.new_register();
    let reg_off = if depth > 1 { right.offset_pos(depth-2)? } else { right.data_pos() };
    out.push(Instruction::new(InstructionType::Length,vec![stride,regs[reg_off]]));
    let filter_len = context.new_register();
    out.push(Instruction::new(InstructionType::Copy,vec![filter_len,filter.clone()]));
    out.append(&mut vector_push_instrs(context,left,right,&filter_len,regs)?);
    let zero = context.new_register();
    out.push(Instruction::new(InstructionType::Const(vec![0]),vec![zero]));
    out.push(vector_update_offsets(left,right,&start,&stride,filter,regs,depth-1)?);
    out.push(vector_update_lengths(left,right,&zero,filter,regs,depth-1)?);
    Ok(out)
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
            Ok(Box::new(AssignCommand(sig[0].get_mode() == MemberMode::Filter,sig.clone(),it.regs.to_vec())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }    
}

pub struct AssignCommand(bool,RegisterSignature,Vec<Register>);

impl AssignCommand {
    fn replace_shallow(&self, context: &mut PreImageContext) -> Result<Vec<Instruction>,String> {
        let mut out = vec![];
        for (left,right) in self.1[1].iter().zip(self.1[2].iter()) {
            if left.1.depth() > 0 {
                /* deep */
                out.append(&mut copy_deep_instrs(context,left.1,right.1, &self.2[0],&self.2)?);
            } else {
                /* shallow */
                out.push(vector_copy(left.1,right.1,&self.2[0],&self.2)?);
            }
        }
        Ok(out)
    }
}

impl Command for AssignCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Err(format!("compile-side command"))
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(self.1[1].all_registers().iter().map(|x| self.2[*x]).collect()))
    }

    fn preimage(&self, context: &mut PreImageContext, _ic: Option<Box<dyn InterpCommand>>) -> Result<PreImageOutcome,String> { 
        Ok(if !self.0 {
            /* unfiltered */
            PreImageOutcome::Replace(preimage_instrs(&self.2)?)
        } else {
            /* filtered */
            PreImageOutcome::Replace(self.replace_shallow(context)?)
        })
    }
}

pub(super) fn library_assign_commands(set: &mut CompLibRegister) -> Result<(),String> {
    set.push("assign",None,AssignCommandType());
    set.push("extend",None,ExtendCommandType::new());
    Ok(())
}

pub(super) fn library_assign_commands_interp(_set: &mut InterpLibRegister) -> Result<(),String> {
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::generate;
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_compiler_suite };

    #[test]
    fn assign_filtered() {
        let config = xxx_test_config();
        let mut linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:std/filterassign").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,_strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        // XXX todo test it!
    }

    #[test]
    fn assign_shallow() {
        let mut config = xxx_test_config();
        config.set_debug_run(true);
        let mut linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:std/assignshallow").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        print!("{:?}\n",strings);
        assert_eq!("[0, 0]",strings[0]);
    }
}
