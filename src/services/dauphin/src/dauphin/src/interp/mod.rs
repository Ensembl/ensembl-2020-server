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
 *  
 *  vscode-fold=1
 */

mod context;
mod harness;
mod stream;
mod commands {
    pub mod common {
        pub mod commontype;
    }
    pub mod core {
        pub mod consts;
        pub mod core;
    }
    pub mod assign;
    pub mod library {
        pub mod library;
        pub mod numops;
        pub mod eq;
    }
}
mod commandsets {
    pub mod command;
    pub mod commandset;
    pub mod commandsetid;
    pub mod interpretsuite;
    mod member;
    pub mod compilesuite;
    pub mod suitebuilder;

    pub use command::{ Command, CommandSchema, CommandTrigger, CommandType };
    pub use commandset::CommandSet;
    pub use commandsetid::CommandSetId;
    pub use interpretsuite::CommandInterpretSuite;
    pub use compilesuite::CommandCompileSuite;
    pub use suitebuilder::CommandSuiteBuilder;
}
mod values {
    pub mod registers;
    pub mod supercow;
    pub mod value;

}
pub use self::harness::mini_interp;
pub use self::values::value::{ to_index, InterpValue, InterpNatural };
pub use self::values::registers::RegisterFile;
pub use self::stream::StreamContents;