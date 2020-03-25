use std::collections::{ HashMap, HashSet };
use crate::model::Register;
use super::gencontext::GenContext;
use super::instruction::{ Instruction, InstructionType };

fn find_first_last_use(context: &mut GenContext) -> HashMap<Register,(usize,usize)> {
    /* find first and last use of every register */
    let mut out = HashMap::new();
    let mut seen = HashSet::new();
    for (i,instr) in context.get_instructions().iter().enumerate() {
        for reg in instr.regs.iter() {
            if !seen.contains(reg) {
                out.insert(*reg,(i,0));
                seen.insert(reg);
            }
        }
    }
    let mut seen = HashSet::new();
    let mut rev_instrs = context.get_instructions();
    rev_instrs.reverse();
    for (i,instr) in rev_instrs.iter().enumerate() {
        for reg in instr.regs.iter() {
            if !seen.contains(reg) {
                out.get_mut(reg).unwrap().1 = rev_instrs.len()-i-1;
                seen.insert(reg);
            }
        }
    }
    out
}

fn calculate_lengths(context: &mut GenContext, first_use: Vec<HashSet<Register>>, last_use: Vec<HashSet<Register>>) -> HashMap<Register,usize> {
    let mut lengths = HashMap::new();
    let mut seen_at = HashMap::new();
    for (i,instr) in context.get_instructions().iter().enumerate() {
        for reg in &first_use[i] {
            seen_at.insert(reg,i);
        }
        for reg in &last_use[i] {
            lengths.insert(*reg,i-seen_at.get(&reg).unwrap());
        }
    }
    lengths
}

fn allocate(regs: Vec<Register>, reg_ranges: HashMap<Register,(usize,usize)>) -> HashMap<Register,Register> {
    let mut allocation = HashMap::new();
    let mut in_use = Vec::new();
    for reg in &regs {
        let mut overlap : HashSet<usize> = HashSet::new();
        let (first,last) = reg_ranges.get(reg).unwrap();
        while in_use.len() <= *last {
            in_use.push(HashSet::new());
        }
        for i in *first..(*last+1) {
            overlap.extend(in_use[i].iter());
        }
        let mut reg_num = 1;
        while overlap.contains(&reg_num) {
            reg_num += 1;
        }
        allocation.insert(*reg,Register(reg_num));
        for i in *first..(*last+1) {
            in_use[i].insert(reg_num);
        }
    }
    allocation
}

pub fn assign_regs(context: &mut GenContext) {
    let range = find_first_last_use(context);
    print!("RANGE: {:?}\n",range);
    let priorities : HashMap<_,_> = range.iter().map(|(k,v)| (*k,v.1-v.0+1)).collect();
    let mut regs : Vec<_> = priorities.keys().cloned().collect();
    regs.sort_by_key(|k| priorities.get(k).unwrap());
    regs.reverse(); /* longest-lived first */
    let allocation = allocate(regs,range);
    for instr in context.get_instructions().iter() {
        let new_regs = instr.regs.iter().map(|r| *allocation.get(r).unwrap()).collect::<Vec<_>>();
        context.add(Instruction::new(instr.itype.clone(),new_regs));
    }
    context.phase_finished();
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
    use super::super::linearize;
    use super::super::remove_aliases;
    use super::super::copy_on_write;
    use super::super::prune;

    // XXX test pruning, eg fewer lines
    #[test]
    fn assign_regs_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-refsquare.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        print!("LIN\n");
        linearize(&mut context).expect("linearize");
        print!("{:?}\n",context);
        print!("RMAL\n");
        remove_aliases(&mut context);
        prune(&mut context);
        print!("{:?}\n",context);
        print!("COW\n");
        copy_on_write(&mut context);
        print!("{:?}\n",context);
        print!("PRUNE\n");
        prune(&mut context);
        print!("{:?}\n",context);
        assign_regs(&mut context);
        print!("ASSIGN\n");
        print!("{:?}\n",context);
        let (_prints,values,strings) = mini_interp(&defstore,&mut context);
        print!("{:?}\n",values);
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }
}