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

#[derive(Clone)]
pub struct Config {
    subconfig: Option<Box<Config>>,
    generate_debug: Option<bool>,
    verbose: Option<u8>,
    optimise: Option<u8>,
    opt_seq: Option<String>
}

macro_rules! flag {
    ($self:ident,$option:ident,$setter:ident,$getter:ident,$t:ty,$dft:expr) => {
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
    };
}

macro_rules! flag_str {
    ($self:ident,$option:ident,$setter:ident,$getter:ident,$dft:expr) => {
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
    };
}


impl Config {
    pub fn new() -> Config {
        Config {
            subconfig: None,
            generate_debug: None,
            verbose: None,
            optimise: None,
            opt_seq: None
        }
    }

    pub fn set_subconfig(&mut self, sub: Config) {
        self.subconfig = Some(Box::new(sub));
    }

    flag!(self,generate_debug,set_generate_debug,get_generate_debug,bool,false);
    flag!(self,verbose,set_verbose,get_verbose,u8,0);
    flag!(self,optimise,set_opt_level,get_opt_level,u8,0);
    flag_str!(self,opt_seq,set_opt_seq,get_opt_seq,"*");
}
