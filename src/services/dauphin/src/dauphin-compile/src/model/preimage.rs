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

use std::collections::{ HashMap, HashSet };
use crate::command::CompilerLink;
use crate::resolver::ResolveFile;
use crate::model::{ RegisterAllocator };
use dauphin_interp::runtime::{ Register, InterpContext };
use crate::cli::Config;

pub struct PreImageContext<'a> {
    resolver: Box<&'a dyn ResolveFile>,
    reg_sizes: HashMap<Register,usize>,
    compiler_link: CompilerLink,
    valid_registers: HashSet<Register>,
    context: InterpContext,
    regalloc: RegisterAllocator,
    config: Config,
    last: bool
}

impl<'a> PreImageContext<'a> {
    pub fn new(compiler_link: &CompilerLink, resolver: Box<&'a dyn ResolveFile>, config: &Config, max_reg: usize, last: bool) -> Result<PreImageContext<'a>,String> {
        Ok(PreImageContext {
            resolver,
            reg_sizes: HashMap::new(),
            compiler_link: compiler_link.clone(),
            valid_registers: HashSet::new(),
            context: compiler_link.new_context(),
            regalloc: RegisterAllocator::new(max_reg+1),
            config: config.clone(),
            last
        })
    }

    pub fn context(&self) -> &InterpContext { &self.context }
    pub fn context_mut(&mut self) -> &mut InterpContext { &mut self.context }
    pub fn resolve(&self, path: &str) -> Result<String,String> { self.resolver.resolve(path) }
    pub fn config(&self) -> &Config { &self.config }
    pub fn linker(&self) -> &CompilerLink { &self.compiler_link }

    pub fn new_register(&self) -> Register { self.regalloc.allocate() }
    pub fn is_last(&self) -> bool { self.last }

    pub fn set_reg_valid(&mut self, reg: &Register) -> Result<(),String> {
        self.valid_registers.insert(*reg);
        Ok(())
    }

    pub fn set_reg_invalid(&mut self, reg: &Register) {
        self.valid_registers.remove(reg);
    }

    pub fn set_reg_size(&mut self, reg: &Register, size: Option<usize>) {
        //print!("set_reg_size({:?},{:?})\n",reg,size);
        if let Some(size) = size {
            self.reg_sizes.insert(reg.clone(),size);
        } else {
            self.reg_sizes.remove(reg);
        }
    }

    pub fn get_reg_size(&self, reg: &Register) -> Option<usize> { self.reg_sizes.get(reg).map(|x| *x) }

    pub fn is_reg_valid(&self, reg: &Register) -> bool {
        self.valid_registers.contains(reg)
    }
}
