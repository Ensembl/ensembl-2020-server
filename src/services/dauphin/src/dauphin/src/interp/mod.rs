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

mod context;
mod harness;
mod stream;
mod compilelink;
mod interplink;
mod interpret;
mod commandsets {
    pub mod command;
    pub mod commandset;
    pub mod commandsetid;
    pub mod interpretsuite;
    mod member;
    pub mod timetrial;
    pub mod compilesuite;
    pub mod suitebuilder;

    pub use command::{ Command, CommandSchema, CommandTrigger, CommandType, PreImageOutcome };
    pub use commandset::CommandSet;
    pub use commandsetid::CommandSetId;
    pub use interpretsuite::CommandInterpretSuite;
    pub use compilesuite::CommandCompileSuite;
    pub use suitebuilder::{ LibrarySuiteBuilder, make_librarysuite_builder };
    pub use timetrial::{ TimeTrialCommandType, TimeTrial, regress };
}
mod values {
    pub mod registers;
    pub mod supercow;
    pub mod value;

}

pub use self::compilelink::CompilerLink;
pub use self::harness::{  xxx_test_config, find_testdata,stream_strings };
#[cfg(test)]
pub use self::harness::{ mini_interp,  mini_interp_run, interpret, comp_interpret, serialize };
pub use self::values::value::{ to_index, InterpValue, InterpNatural, InterpValueNumbers, InterpValueIndexes, numbers_to_indexes };
pub use self::values::supercow::SuperCow;
pub use self::values::registers::RegisterFile;
pub use self::stream::{ Stream, StreamContents, StreamFactory };
pub use self::commandsets::{
    CommandSet, CommandSetId, CommandInterpretSuite, CommandCompileSuite, LibrarySuiteBuilder, Command, CommandSchema, CommandTrigger, CommandType, 
    PreImageOutcome, make_librarysuite_builder, TimeTrialCommandType, TimeTrial, regress
};
pub use self::context::{ InterpContext, PayloadFactory };
pub use self::interpret::{ InterpretInstance, interpreter };
pub use self::interplink::InterpreterLink;