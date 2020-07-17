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

use std::process::exit;
use dauphin_compile::cli::Config;
use super::action::make_actions;
use clap::{ App, Arg };

static APP_NAME : &str = "dauphin style compiler";
static APP_VERSION : &str = "v0.0";
static APP_AUTHOR : &str = "Ensembl Webteam <ensembl-webteam@ebi.ac.uk>";
static APP_ABOUT : &str = "compiles dauphin source into dauphin bytecode";

struct ConfigOption {
    name: String,
    long: String,
    short: Option<String>,
    value: Option<String>,
    multiple: bool,
    cb: Box<dyn Fn(&mut Config,&str)>
}

impl ConfigOption {
    fn new<T>(name: &str, long: &str, short: Option<&str>, value: Option<&str>, multiple: bool, cb: T) -> ConfigOption
        where T: Fn(&mut Config,&str) + 'static {
        ConfigOption {
            name: name.to_string(),
            long: long.to_string(),
            short: short.map(|x| x.to_string()),
            value: value.map(|x| x.to_string()),
            multiple,
            cb: Box::new(cb)
        }
    }

    fn to_arg<'a>(&'a self) -> Arg<'a,'a> {
        let mut arg = Arg::with_name(&self.name).long(&self.long);
        if let Some(ref short) = self.short { arg = arg.short(short); }
        if let Some(ref value) = self.value { arg = arg.takes_value(true).value_name(value); }
        if self.multiple { arg = arg.multiple(true); }
        arg
    }
}

pub fn to_u8(s: &str) -> u8 {
    match s.parse::<u8>() {
        Ok(v) => v,
        Err(_) => {
            print!("Bad value '{}', expected integer\n",s);
            exit(1);
        }
    }
}

pub fn config_from_options() -> Config {
    let mut config = Config::new();

    let mut options = vec![
        ConfigOption::new("generate-debug","generate-debug",Some("g"),None,false,|config,_| { config.set_generate_debug(true) }),
        ConfigOption::new("no-std","no-std",None,None,false,|config,_| { config.set_nostd(true) }),
        ConfigOption::new("verbose","verbose",Some("v"),None,true,|config,v| { config.set_verbose(to_u8(v)) }),
        ConfigOption::new("opt-level","opt",Some("O"),Some("LEVEL"),false,|config,v| { config.set_opt_level(to_u8(v)) }),
        ConfigOption::new("debug-run","debug-run",None,None,false,|config,_| { config.set_debug_run(true) }),
        ConfigOption::new("opt-seq","opt-seq",None,Some("OPT-SEQUENCE"),false,|config,v| { config.set_opt_seq(v) }),
        ConfigOption::new("root-dir","root-dir",Some("B"),Some("DIRECTORY"),false,|config,v| { config.set_root_dir(v) }),
        ConfigOption::new("file-search-path","file-search-path",Some("I"),Some("DIRECTORY"),true,|config,v| { config.add_file_search_path(v) }),
        ConfigOption::new("lib","lib",Some("L"),Some("LIBRARY"),true,|config,v| { config.add_lib(v) }),
        ConfigOption::new("action","action",None,Some("ACTION"),false,|config,value| { config.set_action(value) }),
        ConfigOption::new("source","source",Some("c"),Some("SOURCE-FILE"),true,|config,v| { config.add_source(v) }),
        ConfigOption::new("output","output",Some("o"),Some("BINARY-FILE"),false,|config,v| { config.set_output(v) }),
        ConfigOption::new("profile","profile",Some("p"),None,false,|config,_| { config.set_profile(true) }),
        ConfigOption::new("execute","execute",Some("x"),Some("PROG-NAME"),false,|config,v| { config.set_run(v); config.set_action("run") }),
        ConfigOption::new("define","define",Some("D"),Some("KEY=VALUE"),true,|config,v| { 
            let (k,v) = if let Some(eq_pos) = v.chars().position(|x| x== '=') {
                let (k,v) = v.split_at(eq_pos);
                (k,&v[1..])
            } else {
                (v,"")
            };
            config.add_define((k.to_string(),v.to_string()));
         }),
    ];
    let actions = make_actions();
    for (action,_) in actions.iter() {
        let action = action.to_string();
        options.push(ConfigOption::new(&action.to_string(),&action.to_string(),None,None,false, move |config,_| { config.set_action(&action) }));
    }

    let mut args = App::new(APP_NAME).version(APP_VERSION).author(APP_AUTHOR).about(APP_ABOUT);
    for option in &options {
        args = args.arg(option.to_arg());
    }
    let matches = args.get_matches();
    for option in &options {
        if option.value.is_some() {
            for value in matches.values_of_lossy(&option.name).unwrap_or(vec![]).iter() {
                (option.cb)(&mut config,&value);
            }
        } else {
            if matches.is_present(&option.name) {
                (option.cb)(&mut config,&format!("{}",matches.occurrences_of(&option.name)));
            }
        }
    }

    config
}