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

mod assignregs;
mod call;
mod codegen;
mod cow;
mod dealias;
mod gencontext;
mod instruction;
mod linearize;
mod prune;
mod reusedead;
mod runnums;
mod simplify;

pub use self::assignregs::assign_regs;
pub use self::call::call;
pub use self::cow::{ copy_on_write, reuse_const };
pub use self::dealias::remove_aliases;
pub use self::gencontext::{ GenContext, generate_and_optimise };
pub use self::codegen::generate_code;
pub use self::instruction::{ Instruction, InstructionType, InstructionSuperType };
pub use self::linearize::linearize;
pub use self::prune::prune;
pub use self::reusedead::reuse_dead;
pub use self::runnums::run_nums;
pub use self::simplify::simplify;
