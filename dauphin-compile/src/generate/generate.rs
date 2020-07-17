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
use std::fs::write;
use std::time::{ SystemTime, Duration };
use crate::cli::Config;
use crate::command::{ Instruction, InstructionType, CompilerLink };
use super::{ GenContext };
use crate::model::DefStore;
use crate::resolver::Resolver;
use crate::parser::Statement;
use super::dealias::remove_aliases;
use super::reuseregs::reuse_regs;
use super::reusedead::reuse_dead;
use super::useearliest::use_earliest_regs;
use super::compilerun::compile_run;
use super::codegen::generate_code;
use super::assignregs::assign_regs;
use super::peephole::{ peephole_nil_append, peephole_linenum_remove };
use super::retreat::retreat;
use super::prune::prune;
use super::call::call;
use super::linearize::linearize;
use super::simplify::simplify;
use super::pauses::pauses;

struct StepData<'a,'b> {
    linker: &'a CompilerLink,
    defstore: &'a DefStore,
    resolver: &'a Resolver,
    context: &'a mut GenContext<'b>,
    config: &'a Config
}

struct GenerateStep {
    name: String,
    step: Box<dyn Fn(&mut StepData) -> Result<(),String>>
}

fn non_line_instructions(context: &GenContext) -> usize {
    context.get_instructions().iter().filter(|instr| {
        match instr.itype {
            InstructionType::Pause(_) => false,
            InstructionType::LineNumber(_) => false,
            _ => true
        }
    }).count()
}

impl GenerateStep {
    fn new<F>(name: &str, step: F) -> GenerateStep
            where F: Fn(&mut StepData) -> Result<(),String> + 'static {
        GenerateStep {
            name: name.to_string(),
            step: Box::new(step)
        }
    }

    fn run(&self, index: usize, config: &Config, compiler_link: &CompilerLink, defstore: &DefStore, resolver: &Resolver, context: &mut GenContext) -> Result<(),String> {
        let start_time = SystemTime::now();
        let mut data = StepData {
            linker: compiler_link,
            defstore, resolver, context, config
        };
        (self.step)(&mut data)?;
        let duration = start_time.elapsed().unwrap_or(Duration::new(0,0));
        if config.get_verbose() > 1 {
            print!("step {}: {}. {} lines {:.2}ms\n",index,self.name,non_line_instructions(context),duration.as_secs_f32()*1000.);
        }
        if config.get_verbose() > 2 {
            print!("{:?}\n",context);
        }
        if config.isset_profile() {
            let filename = format!("{}-{}-{}.profile",defstore.get_source(),self.name,index);
            let text = format!("step {}: {}. {} lines {:.2}ms\n{:?}\n",index,self.name,non_line_instructions(context),duration.as_secs_f32()*1000.,context);
            match write(filename,text) {
                Ok(()) => {},
                Err(e) => { eprint!("could not write profile file: {}",e) }
            }
        }
        Ok(())
    }
}

struct GenerateMenu {
    gen_steps: Vec<GenerateStep>,
    opt_steps: HashMap<String,GenerateStep>,
    post_steps: Vec<GenerateStep>
}

impl GenerateMenu {
    fn new() -> GenerateMenu {
        let mut gen_steps = vec![];
        let mut opt_steps = HashMap::new();
        let mut post_steps = vec![];
        gen_steps.push(GenerateStep::new("call", |step| { call(step.context) }));
        gen_steps.push(GenerateStep::new("simplify", |step| { simplify(step.defstore,step.context) }));
        gen_steps.push(GenerateStep::new("linearize", |step| { linearize(step.context) }));
        gen_steps.push(GenerateStep::new("dealias", |step| { remove_aliases(step.context); Ok(()) }));
        gen_steps.push(GenerateStep::new("compile-run", |step| { compile_run(step.linker,step.resolver,step.context,step.config,false) }));
        opt_steps.insert("c".to_string(),GenerateStep::new("compile-run", |step| { compile_run(step.linker,step.resolver,step.context,step.config,false) }));
        opt_steps.insert("p".to_string(),GenerateStep::new("prune", |step| { prune(step.context); Ok(()) }));
        opt_steps.insert("u".to_string(),GenerateStep::new("reuse-regs", |step| { reuse_regs(step.context) }));
        opt_steps.insert("e".to_string(),GenerateStep::new("use-earliest", |step| { use_earliest_regs(step.context) }));
        opt_steps.insert("d".to_string(),GenerateStep::new("reuse-dead", |step| { reuse_dead(step.context); Ok(()) }));
        opt_steps.insert("a".to_string(),GenerateStep::new("assign-regs", |step| { assign_regs(step.context); Ok(()) }));
        opt_steps.insert("m".to_string(),GenerateStep::new("peephole", |step| { peephole_nil_append(step.context)?; peephole_linenum_remove(step.context) }));
        opt_steps.insert("r".to_string(),GenerateStep::new("retreat", |step| { retreat(step.context) }));
        post_steps.push(GenerateStep::new("pauses", |step| { pauses(step.linker,step.resolver,step.defstore,step.context,step.config) }));
        GenerateMenu { gen_steps, opt_steps, post_steps }
    }

    fn run_steps(&self, config: &Config, sequence: &str, compiler_link: &CompilerLink, defstore: &DefStore, resolver: &Resolver, context: &mut GenContext) -> Result<(),String> {
        let mut index = 1;
        for step in &self.gen_steps {
            step.run(index,config,compiler_link,defstore,resolver,context)?;
            index += 1;
        }
        for k in sequence.chars() {
            let step = self.opt_steps.get(&k.to_string()).ok_or_else(|| format!("No such step '{}'",k))?;
            step.run(index,config,compiler_link,defstore,resolver,context)?;
            index += 1;
        }
        for step in &self.post_steps {
            step.run(index,config,compiler_link,defstore,resolver,context)?;
            index += 1;
        }
        Ok(())
    }
}

fn calculate_opt_seq(config: &Config) -> Result<&str,String> {
    if config.isset_opt_seq() {
        Ok(config.get_opt_seq())
    } else {
        Ok(match config.get_opt_level() {
            0 => "",
            1 => "p",
            2|3|4|5|6 => "pcpmuedprdpa",
            level => Err(format!("Bad optimisation level {}",level))?
        })
    }
}

pub fn generate(compiler_link: &CompilerLink, stmts: &Vec<Statement>, defstore: &DefStore, resolver: &Resolver, config: &Config) -> Result<Vec<Instruction>,String> {
    let mut context = generate_code(&defstore,&stmts,config.get_generate_debug()).map_err(|e| e.join("\n"))?;
    let gm = GenerateMenu::new();
    gm.run_steps(config,calculate_opt_seq(&config)?,compiler_link,defstore,resolver,&mut context)?;
    Ok(context.get_instructions())
}
