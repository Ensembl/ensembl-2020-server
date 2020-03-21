// TODO Copy for registers
use std::collections::BTreeMap;

use crate::model::{ Register };
use crate::typeinf::{ BaseType, MemberType };
use super::codegen::GenContext;
use super::intstruction::Instruction;

use super::optimise::remove_unused_registers;

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
            let start = context.regalloc.allocate();
            let len = context.regalloc.allocate();
            context.types.add(&start,&MemberType::Base(BaseType::NumberType));
            context.types.add(&len,&MemberType::Base(BaseType::NumberType));
            indices.push((start,len));
        }
        let data = context.regalloc.allocate();
        context.types.add(&data,&MemberType::Base(type_.get_base()));
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
    for (reg,type_) in context.types.each_register() {
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
    let r = context.regalloc.allocate();
    context.types.add(&r,&MemberType::Base(BaseType::NumberType));
    r
}

/* create a register containing the legnth of the layer beneath the top */
fn lower_seq_length(out: &mut Vec<Instruction>, context: &mut GenContext, lin: &Linearized, level: usize) -> Register {
    let reg = tmp_number_reg(context);
    if level == 0 {
        out.push(Instruction::Length(reg,lin.data.clone()));
    } else {
        out.push(Instruction::Length(reg,lin.index[level-1].0));
    }
    reg
}

fn push_copy_level(out: &mut Vec<Instruction>, context: &mut GenContext, lin_dst: &Linearized, lin_src: &Linearized, level: usize) {
    /* offset is offset in next layer down (be it index or data) */
    let offset = lower_seq_length(out,context,lin_dst,level);
    let tmp = tmp_number_reg(context);
    out.push(Instruction::Copy(tmp,lin_src.index[level].0));
    out.push(Instruction::Add(tmp,offset.clone()));
    out.push(Instruction::Append(lin_dst.index[level].0,tmp));
    out.push(Instruction::Append(lin_dst.index[level].1,lin_src.index[level].1));
}

fn push_top(out: &mut Vec<Instruction>, context: &mut GenContext, lin_dst: &Linearized, lin_src: &Linearized, level: usize) {
    /* top level offset is current length of next level down plus offset in source */
    let src_len = lower_seq_length(out,context,lin_dst,level);
    let tmp = tmp_number_reg(context);
    out.push(Instruction::Copy(tmp,lin_src.index[level].0));
    out.push(Instruction::Add(tmp,src_len));
    out.push(Instruction::Append(lin_dst.index[level].0,tmp));
    /* top level lengths are copied over */
    out.push(Instruction::Append(lin_dst.index[level].1,lin_src.index[level].1));
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

fn linearize_one(out: &mut Vec<Instruction>, context: &mut GenContext, subregs: &BTreeMap<Register,Linearized> , instr: &Instruction) -> Result<(),String> {
    match instr {
        Instruction::New(_) |
        Instruction::SeqFilter(_,_,_,_) |
        Instruction::Proc(_,_) |
        Instruction::Operator(_,_,_) |
        Instruction::Run(_,_,_) |
        Instruction::SeqAt(_,_) |
        Instruction::Length(_,_) | 
        Instruction::Add(_,_) => {
            panic!("unexpected instruction {:?}",instr);
        },
        Instruction::NumberConst(_,_) |
        Instruction::BooleanConst(_,_) |
        Instruction::StringConst(_,_) |
        Instruction::BytesConst(_,_) |
        Instruction::Nil(_) |
        Instruction::NumEq(_,_,_) => {
            out.push(instr.clone());
        },
        Instruction::List(r) => {
            let lin = subregs.get(r).ok_or_else(|| format!("Missing info for register {:?}",r))?;
            out.push(Instruction::Nil(lin.data));
            for (start,len) in &lin.index {
                out.push(Instruction::Nil(*start));
                out.push(Instruction::Nil(*len));
            }
        },
        Instruction::Call(name,type_,regs) => {
            let mut new = Vec::new();
            for r in regs {
                if let Some(lin_src) = subregs.get(r) {
                    new.push(lin_src.data);
                    for i in 0..lin_src.index.len() {
                        new.push(lin_src.index[i].0);
                        new.push(lin_src.index[i].1);
                    }
                } else {
                    new.push(*r);
                }
            }
            out.push(Instruction::Call(name.clone(),type_.clone(),new));
        },
        Instruction::Append(dst,src) => {
            if let Some(lin_src) = subregs.get(src) {
                let lin_dst = subregs.get(dst).ok_or_else(|| format!("Missing info for register {:?} in push",dst))?;
                push_top(out,context,lin_dst,lin_src,lin_src.index.len()-1);
                for level in (0..lin_src.index.len()-1).rev() {
                    push_copy_level(out,context,lin_dst,lin_src,level);
                }
                out.push(Instruction::Append(lin_dst.data,lin_src.data));
            } else {
                out.push(Instruction::Append(*dst,*src));
            }
        },
        Instruction::Copy(dst,src) => {
            linear_extend(subregs,dst,src, |d,s| {
                out.push(Instruction::Copy(*d,*s));
            });
        },
        Instruction::Alias(dst,src) => {
            linear_extend(subregs,dst,src, |d,s| {
                out.push(Instruction::Alias(*d,*s));
            });
        },
        Instruction::RefSquare(dst,src) => {
            let lin_src = subregs.get(src).ok_or_else(|| format!("Missing info for register {:?} C",src))?;
            if let Some(lin_dst) = subregs.get(dst) {
                out.push(Instruction::Alias(lin_dst.data,lin_src.data));
                for level in 0..lin_dst.index.len() {
                    out.push(Instruction::Alias(lin_dst.index[level].0,lin_src.index[level].0));
                    out.push(Instruction::Alias(lin_dst.index[level].1,lin_src.index[level].1));
                }
            } else {
                out.push(Instruction::Alias(*dst,lin_src.data));
            }
        },
        Instruction::FilterSquare(dst,src) => {
            let lin_src = subregs.get(src).ok_or_else(|| format!("Missing info for register {:?} D",src))?;
            let top_level = lin_src.index.len()-1;
            out.push(Instruction::Run(*dst,lin_src.index[top_level].0,lin_src.index[top_level].1));
        },
        Instruction::Square(dst,src) => {
            let lin_src = subregs.get(src).ok_or_else(|| format!("Missing info for register {:?} A",src))?;
            if lin_src.index.len() > 1 {
                let lin_dst = subregs.get(dst).ok_or_else(|| format!("Missing info for register {:?} B",dst))?;
                out.push(Instruction::Copy(lin_dst.data,lin_src.data));
                let top_level = lin_dst.index.len()-1;
                if top_level > 0 {
                    for level in 0..top_level {
                        out.push(Instruction::Copy(lin_dst.index[level].0,lin_src.index[level].0));
                        out.push(Instruction::Copy(lin_dst.index[level].1,lin_src.index[level].1));
                    }
                }
                out.push(Instruction::SeqFilter(lin_dst.index[top_level].0,lin_src.index[top_level].0,
                                                lin_src.index[top_level+1].0,lin_src.index[top_level+1].1));
                out.push(Instruction::SeqFilter(lin_dst.index[top_level].1,lin_src.index[top_level].1,
                                                lin_src.index[top_level+1].0,lin_src.index[top_level+1].1));
            } else {
                out.push(Instruction::SeqFilter(*dst,lin_src.data,lin_src.index[0].0,lin_src.index[0].1));
            }
        },
        Instruction::At(dst,src) => {
            if let Some(lin_src) = subregs.get(src) {
                let top_level = lin_src.index.len()-1;
                out.push(Instruction::SeqAt(*dst,lin_src.index[top_level].1));
            } else {
                out.push(Instruction::At(*dst,*src));
            }
        },
        Instruction::Filter(dst,src,f) => {
            if let Some(lin_src) = subregs.get(src) {
                let lin_dst = subregs.get(dst).ok_or_else(|| format!("Missing info for register {:?}",dst))?;
                let top_level = lin_dst.index.len()-1;
                out.push(Instruction::Filter(lin_dst.index[top_level].0,lin_src.index[top_level].0,f.clone()));
                out.push(Instruction::Filter(lin_dst.index[top_level].1,lin_src.index[top_level].1,f.clone()));
                out.push(Instruction::Copy(lin_dst.data,lin_src.data));
                if top_level > 0 {
                    for level in 0..top_level {
                        out.push(Instruction::Copy(lin_dst.index[level].0,lin_src.index[level].0));
                        out.push(Instruction::Copy(lin_dst.index[level].1,lin_src.index[level].1));
                    }
                }
            } else {
                out.push(instr.clone());
            }
        },
        Instruction::Star(dst,src) => {
            let lin_dst = subregs.get(dst).ok_or_else(|| format!("Missing info for register {:?}",dst))?;
            let top_level = lin_dst.index.len()-1;
            out.push(Instruction::Nil(lin_dst.index[top_level].0));
            let src_len = if let Some(lin_src) = subregs.get(src) {
                let src_len = lower_seq_length(out,context,lin_src,top_level);
                if top_level > 0 {
                    for level in 0..top_level {
                        out.push(Instruction::Copy(lin_dst.index[level].0,lin_src.index[level].0));
                        out.push(Instruction::Copy(lin_dst.index[level].1,lin_src.index[level].1));
                    }
                }
                out.push(Instruction::Append(lin_dst.data,lin_src.data));
                src_len
            } else {
                let src_len = tmp_number_reg(context);
                out.push(Instruction::Length(src_len,*src));
                out.push(Instruction::Append(lin_dst.data,*src));
                src_len
            };
            let zero_reg = tmp_number_reg(context);
            out.push(Instruction::NumberConst(zero_reg,0.));
            out.push(Instruction::Append(lin_dst.index[top_level].0,zero_reg));
            out.push(Instruction::Append(lin_dst.index[top_level].1,src_len));
        },
    };
    Ok(())
}

fn linearize_real(context: &mut GenContext) -> Result<BTreeMap<Register,Linearized>,String> {
    remove_unused_registers(context);
    let subregs = allocate_subregs(context);
    let mut instrs = Vec::new();
    for instr in &context.instrs.to_vec() {
        linearize_one(&mut instrs,context,&subregs,&instr)?;
    }
    context.instrs = instrs;
    Ok(subregs)
}

pub fn linearize(context: &mut GenContext) -> Result<(),String> {
    linearize_real(context)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::call;
    use super::super::simplify::simplify;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::generate::generate_code;
    use crate::interp::mini_interp;

    fn find_assigns<'a>( instrs: &Vec<Instruction>, subregs: &'a BTreeMap<Register,Linearized>) -> (Vec<&'a Linearized>,Vec<Register>) {
        let mut lin = Vec::new();
        let mut norm = Vec::new();
        for instr in instrs {
            if let Instruction::Call(s,_,vv) = instr {
                if s == "assign" {
                    if let Some(reg) = subregs.get(&vv[1]) {
                        lin.push(reg);
                    } else {
                        norm.push(vv[1]);
                    }
                }
            }
        }
        (lin,norm)
    }

    #[test]
    fn linearize_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        linearize_real(&mut context).expect("linearize");
        print!("{:?}\n",context);
        let values = mini_interp(&defstore,&mut context);
        print!("{:?}",values);
    }

    #[test]
    fn linearize_filter_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke-filter.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        let instrs = context.instrs.clone();
        print!("{:?}\n",context);
        let subregs = linearize_real(&mut context).expect("linearize");
        print!("{:?}\n",context);
        let (_,values,_) = mini_interp(&defstore,&mut context);
        let (lins,norms) = find_assigns(&instrs,&subregs);
        print!("{:?}",values);
        assert_eq!(vec![1,2],values[&lins[0].data]);
        assert_eq!(vec![0],values[&lins[0].index[0].0]);
        assert_eq!(vec![2],values[&lins[0].index[0].1]);
        assert_eq!(vec![2],values[&norms[0]]);
        assert_eq!(vec![1,2,3,4,5],values[&lins[1].data]);
        assert_eq!(vec![2],values[&lins[1].index[0].0]);
        assert_eq!(vec![3],values[&lins[1].index[0].1]);
        assert_eq!(vec![3,4,5],values[&norms[1]]);
        assert_eq!(vec![1,2,3,4,5],values[&norms[2]]);
        assert_eq!(Vec::<usize>::new(),values[&norms[3]]);
    }

    #[test]
    fn linearize_reffilter_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke-reffilter.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        linearize(&mut context).expect("linearize");
        print!("{:?}\n",context);
        let (prints,values,strings) = mini_interp(&defstore,&mut context);
        print!("{:?}\n",values);
        for p in &prints {
            print!("{:?}\n",p);
        }
        for s in &strings {
            print!("{}\n",s);
        }
        let cmp = vec![
            "1",
            "2,3",
            "[4,5]",
            "[6,6]",
            "[7,8]",
            "[7,9]",
            "2,4",
            "[[[111,112,113],[121,122,123],[131,132,133]],[[211,212,213],[221,222,223],[231,232,233]],[[311,312,313],[321,322,323],[331,332,333]]]",
            "[[[111,112,113],[121,122,123],[131,132,133]],[[211,212,213],[221,222,223],[231,232,233]],[[411,412,413],[421,422,423],[431,432,433]]]",
            "[[[111,112,113],[444],[131,132,133]],[[211,212,213],[444],[231,232,233]],[[411,412,413],[444],[431,432,433]]]",
            "[[[111,112,113],[444],[131,132,433]],[[211,212,213],[444],[231,232,233]],[[411,412,413],[444],[431,432,433]]]",
            "[[[111,112,113],[444],[131,132,433]],[[200,212,213],[444],[231,232,233]],[[411,412,413],[444],[431,432,433]]]",
            "[[[1,2],[3,4]],[[5,6],[7,8]]]",
            "[[[0,0,0],[9,9,9],[0,0,0]],[[0,0,0],[9,9,9],[0,0,0]]]",
            "[[[1,2,3],[4,5],[6],[]],[[7]]]",
            "[3]",
            "[2]",
            "[[],[1,2],[3],[4]]"
        ];
        for (i,string) in strings.iter().enumerate() {
            assert_eq!(cmp[i],string);
        }
    }

    #[test]
    fn linearize_structenum_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke-structenum.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        linearize(&mut context).expect("linearize");
        print!("{:?}\n",context);
        let (prints,values,strings) = mini_interp(&defstore,&mut context);
        print!("{:?}\n",values);
        for p in &prints {
            print!("{:?}\n",p);
        }
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec![
            "[[2], [], [], [], [0], [0], [], [], [], [0], [0], [], [], [], [1], [], [], [], [], []]"
        ],prints.iter().map(|x| format!("{:?}",x)).collect::<Vec<_>>());
    }

    #[test]
    fn linearize_refsquare() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-refsquare.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        linearize(&mut context).expect("linearize");
        print!("{:?}\n",context);
        let (_prints,values,strings) = mini_interp(&defstore,&mut context);
        print!("{:?}\n",values);
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }

    fn linearize_stable_pass() -> GenContext {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize_real(&mut context).expect("linearize");
        print!("{:?}\n",context);
        context
    }

    #[test]
    fn linearize_stable_allocs() {
        let a = linearize_stable_pass();
        let b = linearize_stable_pass();
        let astr = a.instrs.iter().map(|x| format!("{:?}",x)).collect::<Vec<_>>();
        let bstr = b.instrs.iter().map(|x| format!("{:?}",x)).collect::<Vec<_>>();
        assert_eq!(astr,bstr);
    }

   #[test]
    fn linearize_push_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-smoke-push.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        let instrs = context.instrs.clone();
        print!("{:?}\n",instrs);
        let subregs = linearize_real(&mut context).expect("linearize");
        let (lins,_) = find_assigns(&instrs,&subregs);
        let (_,values,_) = mini_interp(&defstore,&mut context);
        assert_eq!(Vec::<usize>::new(),values[&lins[0].data]);
        assert_eq!(vec![0],values[&lins[0].index[0].0]);
        assert_eq!(vec![0],values[&lins[0].index[0].1]);
        assert_eq!(vec![3],values[&lins[1].data]);
        assert_eq!(vec![0],values[&lins[1].index[0].0]);
        assert_eq!(vec![1],values[&lins[1].index[0].1]);
        assert_eq!(vec![0],values[&lins[1].index[1].0]);
        assert_eq!(vec![1],values[&lins[1].index[1].1]);
        assert_eq!(vec![1],values[&lins[2].data]);
        assert_eq!(vec![0],values[&lins[2].index[0].0]);
        assert_eq!(vec![1],values[&lins[2].index[0].1]);
        assert_eq!(vec![1,2,3,4,5,6],values[&lins[3].data]);
        assert_eq!(vec![0,2,3,6],values[&lins[3].index[0].0]);
        assert_eq!(vec![2,1,3,0],values[&lins[3].index[0].1]);
        assert_eq!(vec![0,2],values[&lins[3].index[1].0]);
        assert_eq!(vec![2,2],values[&lins[3].index[1].1]);
        assert_eq!(vec![0],values[&lins[3].index[2].0]);
        assert_eq!(vec![2],values[&lins[3].index[2].1]);
        assert_eq!(Vec::<usize>::new(),values[&lins[4].data]);
        assert_eq!(vec![0],values[&lins[4].index[0].0]);
        assert_eq!(vec![0],values[&lins[4].index[0].1]);
        assert_eq!(vec![0],values[&lins[4].index[1].0]);
        assert_eq!(vec![1],values[&lins[4].index[1].1]);
        print!("{:?}",values);
    }
}
