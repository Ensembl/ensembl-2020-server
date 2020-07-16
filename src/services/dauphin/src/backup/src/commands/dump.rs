/* 
 *  This is the default license template.
 *  
 *  File: dump.rs
 *  Author: dan
 *  Copyright (c) 2020 dan
 *  
 *  To edit this license information: Press Ctrl+Shift+P and press 'Create new License Template...'.
 */

use dauphin_interp_common::common::{ Register, Identifier, RegisterSignature, InterpCommand };
use dauphin_interp_common::interp::{ InterpValue };
use dauphin_compile::model::{ Command, CommandSchema, CommandType, CommandTrigger, PreImageOutcome };
use dauphin_compile::model::{ PreImageContext, Instruction, InstructionType };
use serde_cbor::Value as CborValue;

pub struct DumpSigCommandType();

fn sig_string(sig: &RegisterSignature) -> String {
    format!("{:?}",sig[1])
}

impl CommandType for DumpSigCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 1,
            trigger: CommandTrigger::Command(Identifier::new("buildtime","dump_sig"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(DumpSigCommand(it.regs[0],sig_string(sig))))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
}

pub struct DumpSigCommand(Register,String);

impl Command for DumpSigCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(None)
    }

    fn preimage(&self, context: &mut PreImageContext, _ic: Option<Box<dyn InterpCommand>>) -> Result<PreImageOutcome,String> {
        context.context_mut().registers_mut().write(&self.0,InterpValue::Strings(vec![self.1.to_string()]));
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

#[cfg(test)]
mod test {
    use crate::test::{ compile, xxx_test_config };

    #[test]
    fn dump_sig() {
        let mut config = xxx_test_config();
        config.add_define(("yes".to_string(),"".to_string()));
        config.add_define(("hello".to_string(),"world".to_string()));
        let strings = compile(&config,"search:buildtime/dump_sig").expect("a");
        assert!(strings[0].contains("[0, 1]"));
        assert!(strings[1].contains("[1, 0]"));
        assert!(strings[2].contains("[1, 1]"));
    }
}