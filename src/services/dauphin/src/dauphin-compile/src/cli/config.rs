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

/* optimise:
 * 0 = silent
 * 1 = confirm
 * 2 = step
 * 3 = dump
 * 4 = interstep
 */

#[derive(Clone)]
pub struct Config {
    subconfig: Option<Box<Config>>,
    generate_debug: Option<bool>,
    nostd: Option<bool>,
    debug_run: Option<bool>,
    verbose: Option<u8>,
    optimise: Option<u8>,
    opt_seq: Option<String>,
    file_search_path: Vec<String>,
    libs: Vec<String>,
    root_dir: Option<String>,
    action: Option<String>,
    unit_test: Option<bool>,
    source: Vec<String>,
    output: Option<String>,
    profile: Option<bool>,
    define: Vec<(String,String)>,
    run: Option<String>
}

macro_rules! push {
    ($self:ident,$option:ident,$adder:ident,$getter:ident,$t:ty) => {
        pub fn $adder(&mut $self, value: $t) {
            $self.$option.push(value);
        }

        pub fn $getter(&$self) -> &Vec<$t> {
            &$self.$option
        }
    };
}

macro_rules! push_str {
    ($self:ident,$option:ident,$adder:ident,$getter:ident) => {
        pub fn $adder(&mut $self, value: &str) {
            $self.$option.push(value.to_string());
        }

        pub fn $getter(&$self) -> &Vec<String> {
            &$self.$option
        }
    };
}

macro_rules! flag {
    ($self:ident,$option:ident,$setter:ident,$getter:ident,$isset:ident,$t:ty,$dft:expr) => {
        #[allow(unused)]
        pub fn $setter(&mut $self, value: $t) {
            $self.$option = Some(value);
        }
    
        pub fn $getter(&$self) -> $t {
            if let Some(ref value) = $self.$option {
                value.clone()
            } else if let Some(ref sub) = $self.subconfig {
                sub.$getter()
            } else {
                $dft
            }
        }            

        #[allow(unused)]
        pub fn $isset(&$self) -> bool {
            if let Some(ref value) = $self.$option {
                true
            } else if let Some(ref sub) = $self.subconfig {
                sub.$isset()
            } else {
                false
            }
        }
    };
}

macro_rules! flag_str {
    ($self:ident,$option:ident,$setter:ident,$getter:ident,$isset:ident,$dft:expr) => {
        pub fn $setter(&mut $self, value: &str) {
            $self.$option = Some(value.to_string());
        }
    
        pub fn $getter(&$self) -> &str {
            if let Some(ref value) = $self.$option {
                &value
            } else if let Some(ref sub) = $self.subconfig {
                sub.$getter()
            } else {
                $dft
            }
        }

        #[allow(unused)]
        pub fn $isset(&$self) -> bool {
            if let Some(ref value) = $self.$option {
                true
            } else if let Some(ref sub) = $self.subconfig {
                sub.$isset()
            } else {
                false
            }
        }
    };
}

impl Config {
    pub fn new() -> Config {
        Config {
            subconfig: None,
            generate_debug: None,
            nostd: None,
            debug_run: None,
            verbose: None,
            optimise: None,
            opt_seq: None,
            file_search_path: vec![],
            source: vec![],
            libs: vec![],
            root_dir: None,
            action: None,
            unit_test: None,
            output: None,
            profile: None,
            define: vec![],
            run: None
        }
    }

    #[allow(unused)]
    pub fn set_subconfig(&mut self, sub: Config) {
        self.subconfig = Some(Box::new(sub));
    }

    pub fn verify(&self) -> Result<(),String> {
        if self.get_profile() && !self.get_generate_debug() {
            return Err(format!("cannot generate profile (-p) without debug info (-g)"));
        }
        if self.get_action() == "run" && !self.isset_output() {
            return Err(format!("cannot run (-x) without object file (-o)"));
        }
        Ok(())
    }

    flag!(self,generate_debug,set_generate_debug,get_generate_debug,isset_generate_debug,bool,false);
    flag!(self,nostd,set_nostd,get_nostd,isset_nostd,bool,false);
    flag!(self,verbose,set_verbose,get_verbose,isset_verbose,u8,0);
    flag!(self,optimise,set_opt_level,get_opt_level,isset_opt_level,u8,0);
    flag!(self,debug_run,set_debug_run,get_debug_run,isset_debug_run,bool,false);
    flag!(self,unit_test,set_unit_test,get_unit_test,isset_unit_test,bool,false);
    flag_str!(self,opt_seq,set_opt_seq,get_opt_seq,isset_opt_seq,"*");
    flag_str!(self,root_dir,set_root_dir,get_root_dir,isset_root_dir,".");
    flag_str!(self,action,set_action,get_action,isset_action,"compile");
    push_str!(self,file_search_path,add_file_search_path,get_file_search_path);
    push_str!(self,libs,add_lib,get_libs);
    push_str!(self,source,add_source,get_sources);
    flag_str!(self,output,set_output,get_output,isset_output,"out.dpb");
    flag_str!(self,run,set_run,get_run,isset_run,"");
    flag!(self,profile,set_profile,get_profile,isset_profile,bool,false);
    push!(self,define,add_define,get_defines,(String,String));
}
