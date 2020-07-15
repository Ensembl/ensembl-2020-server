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

extern crate serde_cbor;

pub mod cli {
    mod config;
    pub use config::Config;
}

pub mod command {
    mod command;
    pub use command::{ Command, CommandSchema, CommandTrigger, CommandType, PreImageOutcome, PreImagePrepare };
}

pub mod model {
    mod commandtypestore;
    mod compilelink;
    mod compilesuite;
    mod complibregister;
    mod dfloat;
    mod instruction;
    mod lexer;
    mod preimage;
    mod regalloc;
    mod resolvefile;
    mod timetrial;

    pub use complibregister::CompLibRegister;
    pub use compilelink::CompilerLink;
    pub use compilesuite::CommandCompileSuite;
    pub use commandtypestore::{ CommandTypeStore };
    pub use dfloat::DFloat;
    pub use instruction::{ Instruction, InstructionType, InstructionSuperType };
    pub use lexer::{ LexerPosition, FileContentsHandle };
    pub use preimage::PreImageContext;
    pub use regalloc::RegisterAllocator;
    pub use resolvefile::ResolveFile;
    pub use timetrial::{ TimeTrialCommandType, TimeTrial, trial_signature,trial_write };
}

pub mod util {
    mod vectorcopy;

    pub use vectorcopy::{
        vector_push_instrs, vector_update_offsets, vector_update_lengths, vector_copy, vector_register_copy_instrs, vector_append, vector_append_offsets,
        vector_append_lengths
    };
}

#[cfg(test)]
mod test;