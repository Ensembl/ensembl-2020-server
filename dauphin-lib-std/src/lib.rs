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

/* interp */
pub mod interp;
pub mod stream {
    pub mod stream;
    pub use self::stream::{ Stream, StreamFactory };
}

pub use interp::{ InterpBinBoolOp, InterpBinNumOp, InterpNumModOp };
pub use interp::make_std_interp;
pub use self::stream::{ Stream, StreamFactory };


/* compile */
#[cfg(any(feature = "compile",test))]
mod compile {
    mod assign;
    mod eq;
    mod extend;
    mod numops;
    mod print;
    mod vector;
    pub mod library;
}

#[cfg(test)]
mod test;

#[cfg(any(feature = "compile",test))]
pub use compile::library::make_std;
