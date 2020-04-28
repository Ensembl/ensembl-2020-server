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

use crate::interp::{ InterpNatural, InterpValue };
use crate::model::{ Register, RegisterSignature };
use super::super::common::blit::coerce_to;
use super::super::common::vectorcmp::{ SharedVec, compare };
use super::super::common::vectorsource::RegisterVectorSource;
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, InterpContext };
use crate::generate::{ Instruction, InstructionType };
use serde_cbor::Value as CborValue;
use crate::model::{ cbor_array, cbor_bool };

pub struct EqCommandType();

impl CommandType for EqCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command("eq".to_string())
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
        let sig = RegisterSignature::deserialize(&value[0],false,false)?;
        Ok(Box::new(EqCommand(sig,regs)))
    }
}

pub struct EqCommand(pub(crate) RegisterSignature, pub(crate) Vec<Register>);

impl Command for EqCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let vs = RegisterVectorSource::new(&self.1);
        let cr_a = &self.0[1];
        let cr_b = &self.0[2];
        // XXX need info on vec/struct ordering
        for (vr_a,vr_b) in cr_a.iter().zip(cr_b.iter()) {
            let a = SharedVec::new(context,&vs,vr_a.1)?;
            let b = SharedVec::new(context,&vs,vr_b.1)?;
            let out = compare(&a,&b)?;
            context.registers().write(&self.1[0],InterpValue::Boolean(out));
        }
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        let regs = CborValue::Array(self.1.iter().map(|x| x.serialize()).collect());
        Ok(vec![self.0.serialize(false,false)?,regs])
    }
}

pub(super) fn library_eq_command(set: &mut CommandSet) -> Result<(),String> {
    set.push("eq",0,EqCommandType())?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::generate::{ generate_code, generate };
    use crate::interp::mini_interp;

    #[test]
    fn eq_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:library/eq.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        generate(&mut context,&defstore).expect("j");
        let (_,strings) = mini_interp(&mut context).expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
    }
}