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

use std::collections::HashMap;
use super::{ GenContext, Instruction };
use crate::model::DefStore;
use crate::interp::CompilerLink;
use crate::parser::Statement;
use super::dealias::remove_aliases;
use super::compilerun::compile_run;
use super::codegen::generate_code;
use super::assignregs::assign_regs;
use super::prune::prune;
use super::reusedead::reuse_dead;
use super::call::call;
use super::linearize::linearize;
use super::simplify::simplify;
use super::cow::{ copy_on_write, reuse_const };

struct GenerateStep {
    name: String,
    step: Box<dyn Fn(&CompilerLink,&DefStore,&mut GenContext) -> Result<(),String>>
}

impl GenerateStep {
    fn new<F>(name: &str, step: F) -> GenerateStep
            where F: Fn(&CompilerLink,&DefStore,&mut GenContext) -> Result<(),String> + 'static {
        GenerateStep {
            name: name.to_string(),
            step: Box::new(step)
        }
    }

    fn run(&self, compiler_link: &CompilerLink, defstore: &DefStore, context: &mut GenContext) -> Result<(),String> {
        print!("step {}\n",self.name);
        (self.step)(compiler_link,defstore,context)?;
        print!("{:?}\n",context);
        Ok(())
    }
}

struct GenerateMenu {
    gen_steps: Vec<GenerateStep>,
    opt_steps: HashMap<String,GenerateStep>
}

impl GenerateMenu {
    fn new() -> GenerateMenu {
        let mut gen_steps = vec![];
        let mut opt_steps = HashMap::new();
        gen_steps.push(GenerateStep::new("call", |_,_,gc| { call(gc) }));
        gen_steps.push(GenerateStep::new("simplify", |_,ds,gc| { simplify(ds,gc) }));
        gen_steps.push(GenerateStep::new("linearize", |_,_,gc| { linearize(gc) }));
        gen_steps.push(GenerateStep::new("dealias", |_,_,gc| { remove_aliases(gc); Ok(()) }));
        gen_steps.push(GenerateStep::new("compile-side-run", |cl,_,gc| { compile_run(cl,gc) }));
        opt_steps.insert("c".to_string(),GenerateStep::new("compile-run", |cl,_,gc| { compile_run(cl,gc) }));
        opt_steps.insert("p".to_string(),GenerateStep::new("prune", |_,_,gc| { prune(gc); Ok(()) }));
        opt_steps.insert("w".to_string(),GenerateStep::new("copy-on-write", |_,_,gc| { copy_on_write(gc); Ok(()) }));
        opt_steps.insert("d".to_string(),GenerateStep::new("reuse-dead", |_,_,gc| { reuse_dead(gc); Ok(()) }));
        opt_steps.insert("a".to_string(),GenerateStep::new("assign-regs", |_,_,gc| { assign_regs(gc); Ok(()) }));
        opt_steps.insert("u".to_string(),GenerateStep::new("reuse-const", |_,_,gc| { reuse_const(gc); Ok(()) }));
        GenerateMenu { gen_steps, opt_steps }
    }

    fn run_steps(&self, sequence: &str, compiler_link: &CompilerLink, defstore: &DefStore, context: &mut GenContext) -> Result<(),String> {
        for step in &self.gen_steps {
            step.run(compiler_link,defstore,context)?;
        }
        for k in sequence.chars() {
            let step = self.opt_steps.get(&k.to_string()).ok_or_else(|| format!("No such step '{}'",k))?;
            step.run(compiler_link,defstore,context)?;
        }
        Ok(())
    }
}

pub fn generate2(seq: &str, compiler_link: &CompilerLink, stmts: &Vec<Statement>, defstore: &DefStore, debug: bool) -> Result<Vec<Instruction>,String> {
    let mut context = generate_code(&defstore,&stmts,debug).map_err(|e| e.join("\n"))?;
    let gm = GenerateMenu::new();
    gm.run_steps(seq,compiler_link,defstore,&mut context)?;
    Ok(context.get_instructions())
}

pub fn generate(compiler_link: &CompilerLink, stmts: &Vec<Statement>, defstore: &DefStore) -> Result<Vec<Instruction>,String> {
    generate2("pwpcpdaupwpcpda",compiler_link,stmts,defstore,true)
}