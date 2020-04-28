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

use super::{ linearize, remove_aliases, prune, run_nums, reuse_dead, assign_regs, GenContext, call, copy_on_write, reuse_const, simplify };
use crate::model::DefStore;

pub fn generate(context: &mut GenContext, defstore: &DefStore) -> Result<(),String> {
    call(context)?;
    simplify(&defstore,context)?;
    linearize(context)?;
    print!("A015\n{:?}\n",context);
    remove_aliases(context);
    print!("A014\n{:?}\n",context);
    run_nums(context);
    print!("A013\n{:?}\n",context);
    prune(context);
    print!("A012\n{:?}\n",context);
    copy_on_write(context);
    print!("A011\n{:?}\n",context);
    prune(context);
    print!("A010\n{:?}\n",context);
    run_nums(context);
    print!("A09\n{:?}\n",context);
    reuse_dead(context);
    print!("A08\n{:?}\n",context);
    assign_regs(context);
    print!("A07\n{:?}\n",context);
    reuse_const(context);
    print!("A06\n{:?}\n",context);
    prune(context);
    print!("A05\n{:?}\n",context);
    copy_on_write(context);
    print!("A04\n{:?}\n",context);
    prune(context);
    print!("A03\n{:?}\n",context);
    run_nums(context);
    print!("A02\n{:?}\n",context);
    reuse_dead(context);
    print!("A01\n{:?}\n",context);
    assign_regs(context);
    Ok(())
}
