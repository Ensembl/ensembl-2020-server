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

#[macro_use]
extern crate dauphin_interp_common;

pub mod cli {
    pub mod config;
    pub use config::Config;
}

pub mod util {
    pub mod vectorcopy;
    pub use vectorcopy::{ 
        vector_append, vector_append_lengths, vector_append_offsets, vector_update_lengths, vector_update_offsets, vector_push_instrs, vector_copy,
        vector_register_copy_instrs
    };
}

pub mod commands;
pub mod generate;
pub mod lexer;
pub mod model;
pub mod parser;
pub mod resolver;
pub mod typeinf;

#[cfg(test)]
mod test;

#[macro_use]
extern crate lazy_static;
