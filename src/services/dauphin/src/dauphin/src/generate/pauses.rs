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
use std::fs::write;
use regex::Regex;
use crate::cli::Config;
use super::gencontext::GenContext;
use super::compilerun::compile_run;
use crate::resolver::Resolver;
use crate::lexer::LexerPosition;
use crate::model::{ DefStore, Register, fix_incoming_filename };
use crate::interp::{ InterpContext, InterpValue, CompilerLink, PreImageOutcome, numbers_to_indexes };
use crate::generate::{ Instruction, InstructionType };

fn format_line(line: &str, time: Option<f64>) -> String {
    let time = if let Some(time) = time {
        if time >= 1. {
            format!("+++")
        } else {
            format!("{:03}",(time*1000.).round())
        }
    } else {
        "   ".to_string()
    };
    format!("{} {}",time,line)
}

struct FileExecutionProfile {
    filename: String,
    lines: Vec<(String,Option<f64>)>
}

impl FileExecutionProfile {
    fn new(pos: &LexerPosition) -> FileExecutionProfile {
        let lines = pos.contents().map(|x| x.split("\n").map(|z| (z.to_string(),None)).collect()).unwrap_or(vec![]);
        FileExecutionProfile {
            filename: pos.filename().to_string(),
            lines
        }
    }

    fn filename(&self) -> &str { &self.filename }

    fn add(&mut self, line: u32, time: f64) {
        let line = line as usize;
        if line < self.lines.len() {
            self.lines[line-1].1 = Some(time);
        }
    }

    fn profile(&self) -> String {
        let mut out = vec![];
        for (line,time) in &self.lines {
            out.push(format_line(line,*time));
        }
        out.join("\n")
    }
}

#[derive(Debug)]
struct ExecutionProfiler {
    line: Option<LexerPosition>,
    time: HashMap<LexerPosition,f64>
}

impl ExecutionProfiler {
    fn new() -> ExecutionProfiler {
        ExecutionProfiler {
            line: None,
            time: HashMap::new()
        }
    }

    fn line(&mut self, pos: &LexerPosition) {
        self.line = Some(pos.clone());
    }

    fn add(&mut self, time: f64) {
        if let Some(line) = &self.line {
            *self.time.entry(line.clone()).or_insert(0.) += time;
        }
    }

    fn get_profiles(&self) -> Vec<FileExecutionProfile> {
        let mut out = HashMap::new();
        for (pos,time) in &self.time {
            let filename = pos.filename();
            let profile = out.entry(filename.to_string()).or_insert_with(|| FileExecutionProfile::new(pos));
            profile.add(pos.line(),*time);
        }
        out.drain().map(|x| x.1).collect()
    }
}

pub fn pauses(compiler_link: &CompilerLink, resolver: &Resolver, defstore: &DefStore, context: &mut GenContext, config: &Config) -> Result<(),String> {
    /* force compilerun to ensure timed instructions */
    compile_run(compiler_link,resolver,context,config,true)?;
    let mut profiler = ExecutionProfiler::new();
    let mut instr_profile = vec![];
    let mut timer = 0.;
    for (instr,time) in &context.get_timed_instructions() {
        if let InstructionType::LineNumber(pos) = &instr.itype {
            profiler.line(pos);
        }
        let mut line_time = None;
        match instr.itype {
            InstructionType::Pause(true) => {
                context.add(instr.clone());
                timer = 0.;
            },
            InstructionType::Pause(false) => {},
            _ => {
                timer += time;
                match instr.itype {
                    InstructionType::LineNumber(_) => {},
                    _ => { line_time = Some(*time); }
                }
                profiler.add(*time);
                if timer >= 1. {
                    context.add(Instruction::new(InstructionType::Pause(false),vec![]));
                    timer = *time;
                }
                context.add(instr.clone())
            }
        }
        let name = format!("{:?}",instr).replace("\n","");
        instr_profile.push(format_line(&name,line_time));
    }
    context.phase_finished();
    if config.get_profile() {
        for (i,profile) in profiler.get_profiles().iter().enumerate() {
            let source_filename = fix_incoming_filename(profile.filename());
            let filename = format!("{}-{}-{}-timing.profile",defstore.get_source(),source_filename,i);
            write(filename.clone(),profile.profile()).map_err(|e| format!("Could not write {}: {}",filename,e))?;
        }
        let filename = format!("{}-timing-binary.profile",defstore.get_source());
        write(filename.clone(),instr_profile.join("\n")).map_err(|e| format!("Could not write {}: {}",filename,e))?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::call::call;
    use super::super::simplify::simplify;
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::prune::prune;
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_compiler_suite };
    use super::super::codegen::generate_code;
    use super::super::linearize::linearize;
    use super::super::dealias::remove_aliases;
    use crate::generate::generate;

    fn pause_check(filename: &str) -> bool {
        let mut config = xxx_test_config();
        config.set_generate_debug(false);
        config.set_opt_seq("pcpmuedpdpa"); /* no r to avoid re-ordering */
        let mut linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import(&format!("search:codegen/{}",filename)).expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let mut seen_force_pause = false;
        for instr in &instrs {
            if seen_force_pause {
                print!("AFTER {:?}",instr);
                return if let InstructionType::Pause(_) = &instr.itype {
                    true
                } else {
                    false
                };
            }
            if let InstructionType::Pause(true) = &instr.itype {
                seen_force_pause = true;
            }
        }
        false
    }

    #[test]
    fn pause() {
        assert!(pause_check("pause"));
        assert!(!pause_check("no-pause"));
    }
}