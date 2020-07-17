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

// TODO Copy for registers
use std::collections::BTreeMap;

use crate::command::{ InstructionType, Instruction };
use crate::util::DFloat;
use crate::typeinf::{ MemberType };
use super::gencontext::GenContext;
use dauphin_interp::types::BaseType;
use dauphin_interp::runtime::{ Register };

/* Linearization is the process of converting arbitrarily deep vectors of simple values into multivals. Although a 
 * multival is a sequence of values, as we need to support multivals of single level lists, all lists get additional
 * levels. The previous simplify step has abolished complex, structured types by this point by "pushing in" vecs.
 * 
 * vecs are represented by 2n+1 registers where n is the depth of the vec: the index registers An, Bn, and the data
 * register D. Each mapping is stored in a Linearized object.
 * 
 * Linearization proceeds by first mapping any registers containing vec values into register sets. It then proceeds,
 * instruction by instruction, converting instructions into their linearized, multi-register forms.
 */

#[derive(Debug)]
struct Linearized {
    index: Vec<(Register,Register)>,
    data: Register
}

impl Linearized {
    fn new(context: &mut GenContext, type_: &MemberType, depth: usize) -> Linearized {
        let mut indices = Vec::new();
        for _ in 0..depth {
            let start = context.allocate_register(Some(&MemberType::Base(BaseType::NumberType)));
            let len = context.allocate_register(Some(&MemberType::Base(BaseType::NumberType)));

            indices.push((start,len));
        }
        let data = context.allocate_register(Some(&MemberType::Base(type_.get_base())));
        Linearized {
            index: indices,
            data
        }
    }
}

/* allocate_subregs performs the allocation of linearized registers in two stages. In the first, the type of each
 * register is examined and those of depth greater than zero are added to a todo list along with their depth. In the
 * second this todo list is iterated over and linearized versions created. Two stages are needed as the linearization
 * creates new registers during iteration.
 */
fn allocate_subregs(context: &mut GenContext) -> BTreeMap<Register,Linearized> {
    let mut targets = Vec::new();
    for (reg,type_) in context.xxx_types().each_register() {
        let depth = type_.depth();
        if depth > 0 {
            targets.push((*reg,type_.clone(),depth));
        }
    }
    let mut out = BTreeMap::new();
    for (reg,type_,depth) in &targets {
        out.insert(*reg,Linearized::new(context,type_,*depth));
    }
    out
}

/* UTILITY METHODS for procedures repeatedly used during linearization. */

/* tmp_number_reg: allocate a new number register */
fn tmp_number_reg(context: &mut GenContext) -> Register {
    context.allocate_register(Some(&MemberType::Base(BaseType::NumberType)))
}

/* create a register containing the legnth of the layer beneath the top */
fn lower_seq_length(context: &mut GenContext, lin: &Linearized, level: usize) -> Register {
    let reg = tmp_number_reg(context);
    if level == 0 {
        context.add(Instruction::new(InstructionType::Length,vec![reg,lin.data]));
    } else {
        context.add(Instruction::new(InstructionType::Length,vec![reg,lin.index[level-1].0]));
    }
    reg
}

fn push_copy_level(context: &mut GenContext, lin_dst: &Linearized, lin_src: &Linearized, level: usize) {
    /* offset is offset in next layer down (be it index or data) */
    let offset = lower_seq_length(context,lin_dst,level);
    let tmp = tmp_number_reg(context);
    context.add(Instruction::new(InstructionType::Copy,vec![tmp,lin_src.index[level].0]));
    context.add(Instruction::new(InstructionType::Add,vec![tmp,offset]));
    context.add(Instruction::new(InstructionType::Append,vec![lin_dst.index[level].0,tmp]));
    context.add(Instruction::new(InstructionType::Append,vec![lin_dst.index[level].1,lin_src.index[level].1]));
}

fn push_top(context: &mut GenContext, lin_dst: &Linearized, lin_src: &Linearized, level: usize) {
    /* top level offset is current length of next level down plus offset in source */
    let src_len = lower_seq_length(context,lin_dst,level);
    let tmp = tmp_number_reg(context);
    context.add(Instruction::new(InstructionType::Copy,vec![tmp,lin_src.index[level].0]));
    context.add(Instruction::new(InstructionType::Add,vec![tmp,src_len]));
    context.add(Instruction::new(InstructionType::Append,vec![lin_dst.index[level].0,tmp]));
    context.add(Instruction::new(InstructionType::Append,vec![lin_dst.index[level].1,lin_src.index[level].1]));
}

fn linear_extend<F>(subregs: &BTreeMap<Register,Linearized>, dst: &Register, src: &Register, mut cb: F)
        where F: FnMut(&Register,&Register) {
    if let Some(lin_src) = subregs.get(src) {
        let lin_dst = subregs.get(dst).unwrap();
        cb(&lin_dst.data,&lin_src.data);
        for level in 0..lin_src.index.len() {
            cb(&lin_dst.index[level].0,&lin_src.index[level].0);
            cb(&lin_dst.index[level].1,&lin_src.index[level].1);
        }
    } else {
        cb(dst,src);
    }
}

fn linearize_one(context: &mut GenContext, subregs: &BTreeMap<Register,Linearized> , instr: &Instruction) -> Result<(),String> {
    match &instr.itype {
        InstructionType::NumEq |
        InstructionType::ReFilter |
        InstructionType::Const(_) |
        InstructionType::NumberConst(_) |
        InstructionType::BooleanConst(_) |
        InstructionType::StringConst(_) |
        InstructionType::BytesConst(_) |
        InstructionType::LineNumber(_) =>
            context.add(instr.clone()),

        InstructionType::Proc(_,_) |
        InstructionType::Operator(_) |
        InstructionType::CtorStruct(_) |
        InstructionType::CtorEnum(_,_) |
        InstructionType::SValue(_,_) |
        InstructionType::RefSValue(_,_) |
        InstructionType::EValue(_,_) |
        InstructionType::RefEValue(_,_) |
        InstructionType::FilterEValue(_,_) |
        InstructionType::ETest(_,_) |
        InstructionType::Run |
        InstructionType::Length |
        InstructionType::Add |
        InstructionType::SeqFilter |
        InstructionType::Pause(_) |
        InstructionType::SeqAt =>
            panic!("Impossible instruction {:?}",instr),

        InstructionType::Alias |
        InstructionType::Copy => {
            linear_extend(subregs,&instr.regs[0],&instr.regs[1], move |d,s| {
                context.add(Instruction::new(instr.itype.clone(),vec![*d,*s]));
            });
        },

        InstructionType::Nil => {
            if let Some(lin_src) = subregs.get(&instr.regs[0]) {
                for index in &lin_src.index {
                    context.add(Instruction::new(InstructionType::Nil,vec![index.0]));
                    context.add(Instruction::new(InstructionType::Nil,vec![index.1]));
                }
                context.add(Instruction::new(InstructionType::Nil,vec![lin_src.data]));
            } else {
                context.add(instr.clone());
            }
        },

        InstructionType::At => {
            if let Some(lin_src) = subregs.get(&instr.regs[1]) {
                let top_level = lin_src.index.len()-1;
                if top_level > 0 {
                    let top_length_reg = lin_src.index[top_level].1;
                    let next_level_reg = lin_src.index[top_level-1].1;
                    context.add(Instruction::new(InstructionType::SeqAt,vec![instr.regs[0],top_length_reg,next_level_reg]));
                } else {
                    let top_level = lin_src.index.len()-1;
                    let top_length_reg = lin_src.index[top_level].1;
                    let next_level_reg = lin_src.data;
                    context.add(Instruction::new(InstructionType::SeqAt,vec![instr.regs[0],top_length_reg,next_level_reg]));
                }
            } else {
                context.add(Instruction::new(InstructionType::At,vec![instr.regs[0],instr.regs[1]]));
            }
        },
        InstructionType::Append => {
            if let Some(lin_src) = subregs.get(&instr.regs[1]) {
                let lin_dst = subregs.get(&instr.regs[0]).ok_or_else(|| format!("Missing info for register {:?} in push",instr.regs[0]))?;
                push_top(context,lin_dst,lin_src,lin_src.index.len()-1);
                for level in (0..lin_src.index.len()-1).rev() {
                    push_copy_level(context,lin_dst,lin_src,level);
                }
                context.add(Instruction::new(InstructionType::Append,vec![lin_dst.data,lin_src.data]));
            } else {
                context.add(instr.clone());
            }
        },

        InstructionType::RefSquare => {
            let lin_src = subregs.get(&instr.regs[1]).ok_or_else(|| format!("Missing info for register {:?} C",instr.regs[1]))?;
            if let Some(lin_dst) = subregs.get(&instr.regs[0]) {
                context.add(Instruction::new(InstructionType::Alias,vec![lin_dst.data,lin_src.data]));
                for level in 0..lin_dst.index.len() {
                    context.add(Instruction::new(InstructionType::Alias,vec![lin_dst.index[level].0,lin_src.index[level].0]));
                    context.add(Instruction::new(InstructionType::Alias,vec![lin_dst.index[level].1,lin_src.index[level].1]));
                }
            } else {
                context.add(Instruction::new(InstructionType::Alias,vec![instr.regs[0],lin_src.data]));
            }
        },

        InstructionType::FilterSquare => {
            let lin_src = subregs.get(&instr.regs[1]).ok_or_else(|| format!("Missing info for register {:?} D",instr.regs[1]))?;
            let top_level = lin_src.index.len()-1;
            if top_level > 0 {
                let next_level_reg = lin_src.index[top_level-1].1;
                context.add(Instruction::new(InstructionType::Run,vec![instr.regs[0],lin_src.index[top_level].0,lin_src.index[top_level].1,next_level_reg]));
            } else {
                let next_level_reg = lin_src.data;
                context.add(Instruction::new(InstructionType::Run,vec![instr.regs[0],lin_src.index[top_level].0,lin_src.index[top_level].1,next_level_reg]));
            }
        },

        InstructionType::Square => {
            let lin_src = subregs.get(&instr.regs[1]).ok_or_else(|| format!("Missing info for register {:?} A",instr.regs[1]))?;
            if lin_src.index.len() > 1 {
                let lin_dst = subregs.get(&instr.regs[0]).ok_or_else(|| format!("Missing info for register {:?} B",instr.regs[0]))?;
                context.add(Instruction::new(InstructionType::Copy,vec![lin_dst.data,lin_src.data]));
                let top_level = lin_dst.index.len()-1;
                if top_level > 0 {
                    for level in 0..top_level {
                        context.add(Instruction::new(InstructionType::Copy,vec![lin_dst.index[level].0,lin_src.index[level].0]));
                        context.add(Instruction::new(InstructionType::Copy,vec![lin_dst.index[level].1,lin_src.index[level].1]));
                    }
                }
                context.add(Instruction::new(InstructionType::SeqFilter,vec![
                    lin_dst.index[top_level].0,lin_src.index[top_level].0,
                    lin_src.index[top_level+1].0,lin_src.index[top_level+1].1
                ]));
                context.add(Instruction::new(InstructionType::SeqFilter,vec![
                    lin_dst.index[top_level].1,lin_src.index[top_level].1,
                    lin_src.index[top_level+1].0,lin_src.index[top_level+1].1
                ]));
            } else {
                context.add(Instruction::new(InstructionType::SeqFilter,vec![
                    instr.regs[0],lin_src.data,lin_src.index[0].0,lin_src.index[0].1
                ]));
            }
        },

        InstructionType::Star => {
            let lin_dst = subregs.get(&instr.regs[0]).ok_or_else(|| format!("Missing info for register {:?}",instr.regs[0]))?;
            let top_level = lin_dst.index.len()-1;
            context.add(Instruction::new(InstructionType::Nil,vec![lin_dst.index[top_level].0]));
            let src_len = if let Some(lin_src) = subregs.get(&instr.regs[1]) {
                let src_len = lower_seq_length(context,lin_src,top_level);
                if top_level > 0 {
                    for level in 0..top_level {
                        context.add(Instruction::new(InstructionType::Copy,vec![lin_dst.index[level].0,lin_src.index[level].0]));
                        context.add(Instruction::new(InstructionType::Copy,vec![lin_dst.index[level].1,lin_src.index[level].1]));
                    }
                }
                context.add(Instruction::new(InstructionType::Copy,vec![lin_dst.data,lin_src.data]));
                src_len
            } else {
                let src_len = tmp_number_reg(context);
                context.add(Instruction::new(InstructionType::Length,vec![src_len,instr.regs[1]]));
                context.add(Instruction::new(InstructionType::Copy,vec![lin_dst.data,instr.regs[1]]));
                src_len
            };
            let zero_reg = tmp_number_reg(context);
            context.add(Instruction::new(InstructionType::NumberConst(DFloat::new_usize(0)),vec![zero_reg]));
            context.add(Instruction::new(InstructionType::Nil,vec![lin_dst.index[top_level].0]));
            context.add(Instruction::new(InstructionType::Nil,vec![lin_dst.index[top_level].1]));
            context.add(Instruction::new(InstructionType::Append,vec![lin_dst.index[top_level].0,zero_reg]));
            context.add(Instruction::new(InstructionType::Append,vec![lin_dst.index[top_level].1,src_len]));
        },

        InstructionType::Filter => {
            if let Some(lin_src) = subregs.get(&instr.regs[1]) {
                let lin_dst = subregs.get(&instr.regs[0]).ok_or_else(|| format!("Missing info for register {:?}",instr.regs[0]))?;
                let top_level = lin_dst.index.len()-1;
                context.add(Instruction::new(InstructionType::Filter,vec![lin_dst.index[top_level].0,lin_src.index[top_level].0,instr.regs[2]]));
                context.add(Instruction::new(InstructionType::Filter,vec![lin_dst.index[top_level].1,lin_src.index[top_level].1,instr.regs[2]]));
                context.add(Instruction::new(InstructionType::Copy,vec![lin_dst.data,lin_src.data]));
                if top_level > 0 {
                    for level in 0..top_level {
                        context.add(Instruction::new(InstructionType::Copy,vec![lin_dst.index[level].0,lin_src.index[level].0]));
                        context.add(Instruction::new(InstructionType::Copy,vec![lin_dst.index[level].1,lin_src.index[level].1]));
                    }
                }
            } else {
                context.add(instr.clone());
            }
        },
        InstructionType::Call(name,impure,type_,flow) => {
            let mut new = Vec::new();
            for r in &instr.regs {
                if let Some(lin_src) = subregs.get(&r) {
                    new.push(lin_src.data);
                    for i in 0..lin_src.index.len() {
                        new.push(lin_src.index[i].0);
                        new.push(lin_src.index[i].1);
                    }
                } else {
                    new.push(*r);
                }
            }
            context.add(Instruction::new(InstructionType::Call(name.clone(),*impure,type_.clone(),flow.clone()),new));
        },
    }
    Ok(())
}

fn linearize_real(context: &mut GenContext) -> Result<BTreeMap<Register,Linearized>,String> {
    let subregs = allocate_subregs(context);
    for instr in &context.get_instructions().to_vec() {
        linearize_one(context,&subregs,&instr)?;
    }
    context.phase_finished();
    Ok(subregs)
}

pub fn linearize(context: &mut GenContext) -> Result<(),String> {
    linearize_real(context)?;
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
    use super::super::codegen::generate_code;
    use crate::test::{ mini_interp, xxx_test_config, make_compiler_suite };
    use crate::command::CompilerLink;
    use super::super::dealias::remove_aliases;

    #[test]
    fn linearize_smoke() {
        let config = xxx_test_config();
        let mut linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:codegen/linearize-smoke").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,&stmts,true).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        linearize_real(&mut context).expect("linearize");
        print!("{:?}\n",context);
        remove_aliases(&mut context);
        let values = mini_interp(&mut context.get_instructions(),&mut linker,&config,"main");
        print!("{:?}",values);
    }

    fn linearize_stable_pass() -> Vec<Instruction> {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:codegen/linearize-smoke").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,&stmts,true).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize_real(&mut context).expect("linearize");
        print!("{:?}\n",context);
        context.get_instructions()
    }

    #[test]
    fn linearize_stable_allocs() {
        let a = linearize_stable_pass();
        let b = linearize_stable_pass();
        assert_eq!(a,b);
    }
}
