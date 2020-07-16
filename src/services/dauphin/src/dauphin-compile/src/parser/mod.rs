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

mod lexutil;
mod node;
mod parsedecl;
mod parseexpr;
mod parsestmt;
mod parser;
mod declare;

pub use lexutil::not_reserved;
pub use node::{ ParseError, Statement, Expression };
pub use parser::Parser;

pub use parsedecl::parse_signature;

// For nosey tests in other crates
pub use parsedecl::{ parse_type, parse_typesig };
