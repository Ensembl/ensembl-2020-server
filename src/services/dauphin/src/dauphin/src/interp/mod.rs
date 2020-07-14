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

mod harness;
mod compilelink;
mod interplink;
mod interpret;
mod commandsets {
    pub mod command;
    pub mod commandtypestore;
    mod deserializer;
    pub mod interpretsuite;
    pub mod timetrial;
    pub mod compilesuite;
    pub mod suite;
    pub mod suitebuilder;

    pub use command::{ Command, CommandSchema, CommandTrigger, CommandType, PreImageOutcome, PreImagePrepare };
    pub use interpretsuite::CommandInterpretSuite;
    pub use compilesuite::CommandCompileSuite;
    pub use suitebuilder::{ make_compiler_suite, make_interpret_suite };
    pub use suite::{ CompLibRegister };
    pub use timetrial::{ TimeTrialCommandType, TimeTrial, regress, trial_write, trial_signature };
    pub use deserializer::Deserializer;
    pub use commandtypestore::{ CommandTypeStore, CommandTypeId };
}

pub use self::compilelink::CompilerLink;


pub use self::harness::{ stream_strings };
#[cfg(test)]
pub use self::harness::{ xxx_test_config, mini_interp,  mini_interp_run, interpret, comp_interpret };
pub use self::commandsets::{
    CommandInterpretSuite, CommandCompileSuite, Command, CommandSchema, CommandTrigger, CommandType, Deserializer, CompLibRegister,
    PreImagePrepare, PreImageOutcome, make_compiler_suite, make_interpret_suite, TimeTrialCommandType, TimeTrial, regress, trial_write, trial_signature,
    CommandTypeStore, CommandTypeId
};
pub use self::interpret::{ InterpretInstance, interpreter };
pub use self::interplink::InterpreterLink;
