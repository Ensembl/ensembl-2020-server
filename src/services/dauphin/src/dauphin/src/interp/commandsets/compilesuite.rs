use std::collections::HashMap;
use std::rc::Rc;
use super::command::{ CommandTrigger, CommandType };
use super::commandset::CommandSet;
use super::member::CommandSuiteMember;
use serde_cbor::Value as CborValue;

pub struct CommandCompileSuite {
    sets: Vec<(Rc<CommandSet>,u32)>,
    mapping: HashMap<CommandTrigger,CommandSuiteMember>
}

impl CommandCompileSuite {
    pub(super) fn new() -> CommandCompileSuite {
        CommandCompileSuite {
            sets: vec![],
            mapping: HashMap::new()
        }
    }

    pub(super) fn add_set(&mut self, set: Rc<CommandSet>, offset: u32) {
        self.sets.push((set,offset));
    }

    pub(super) fn add_member(&mut self, trigger: CommandTrigger, member: &CommandSuiteMember) {
        self.mapping.insert(trigger,member.clone());
    }

    pub(super) fn check_traces(&self) -> Result<(),String> {
        for members in &self.sets {
            members.0.check_trace()?;
        }
        Ok(())
    }

    pub fn serialize(&self) -> CborValue {
        let mut out = vec![];
        for members in &self.sets {
            out.push(CborValue::Integer(members.1 as i128));
            out.push(members.0.id().serialize());
        }
        CborValue::Array(out)
    }

    pub fn get_by_trigger(&self, trigger: &CommandTrigger) -> Result<(&Box<dyn CommandType>,u32),String> {
        let member = self.mapping.get(trigger).ok_or(format!("Unknown command {}",trigger))?;
        let cmdtype = member.get_object()?;
        Ok((cmdtype,member.opcode()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{ CommandSetId, CommandTrigger };
    use crate::interp::commands::core::consts::{ ConstCommandType, NumberConstCommandType };
    use crate::generate::InstructionSuperType;
    use crate::test::cbor::cbor_cmp;

    #[test]
    fn test_compile_smoke() {
        let mut ccs = CommandCompileSuite::new();
        //
        let csi1 = CommandSetId::new("test",(1,2),0x2A9E7C72C8628854);
        let mut cs1 = CommandSet::new(&csi1);
        cs1.push("test1",5,ConstCommandType()).expect("a");
        let cs1 = Rc::new(cs1);
        ccs.add_set(cs1.clone(),10);
        let m = CommandSuiteMember::new(5,cs1.clone(),10);
        ccs.add_member(CommandTrigger::Instruction(InstructionSuperType::Const),&m);
        //
        let csi2 = CommandSetId::new("test2",(1,2),0x284E7C72C8628854);
        let mut cs2 = CommandSet::new(&csi2);
        cs2.push("test2",5,NumberConstCommandType()).expect("a");
        let cs2 = Rc::new(cs2);
        ccs.add_set(cs2.clone(),20);
        let m = CommandSuiteMember::new(5,cs2.clone(),20);
        ccs.add_member(CommandTrigger::Instruction(InstructionSuperType::NumberConst),&m);
        //
        let (cmd,opcode) = ccs.get_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::Const)).expect("b");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::Const),cmd.get_schema().trigger);
        assert_eq!(15,opcode);
        let (cmd,opcode) = ccs.get_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::NumberConst)).expect("c");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::NumberConst),cmd.get_schema().trigger);
        assert_eq!(25,opcode);
        ccs.check_traces().expect("c");
        cbor_cmp(&ccs.serialize(),"compilesuite.out");
    }
}