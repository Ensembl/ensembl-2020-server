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
use super::command::CommandType;
use super::commandset::CommandSet;

#[derive(Clone)]
pub(super) struct CommandSuiteMember {
    real_opcode: u32,
    local_opcode: u32,
    set: Rc<CommandSet>
}

impl CommandSuiteMember {
    pub(super) fn new(local_opcode: u32, set: Rc<CommandSet>, offset: u32) -> CommandSuiteMember {
        CommandSuiteMember {
            real_opcode: offset + local_opcode,
            local_opcode,
            set
        }
    }

    pub(super) fn get_object(&self) -> Result<&Box<dyn CommandType>,String> {
        self.set.get(self.local_opcode)
    }

    pub(super) fn opcode(&self) -> u32 { self.real_opcode }
    pub(super) fn set_offset(&mut self, offset: u32) { self.real_opcode = self.local_opcode + offset; }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{ CommandSetId, CommandTrigger };
    use crate::commands::ConstCommandType;
    use crate::generate::InstructionSuperType;

    #[test]
    fn member_smoke() {
        let csi = CommandSetId::new("test",(1,2),0x1E139093D228F8FF);
        let mut cs = CommandSet::new(&csi);
        cs.push("test1",5,ConstCommandType()).expect("a");
        let cs = Rc::new(cs);
        let mut m = CommandSuiteMember::new(5,cs.clone(),10);
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::Const),m.get_object().expect("b").get_schema().trigger);
        assert!(CommandSuiteMember::new(6,cs.clone(),10).get_object().is_err());
        assert_eq!(15,m.opcode());
        m.set_offset(20);
        assert_eq!(25,m.opcode());
    }
}