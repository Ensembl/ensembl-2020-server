use std::collections::HashMap;
use super::super::context::{InterpContext };
use super::super::value::{ InterpValueData, InterpNatural, ReadOnlyValues, ReadWriteValues };
use super::super::command::Command;
use crate::model::{ LinearPath, Register, RegisterPurpose };
use super::assign::{ blit, blit_expanded, blit_runs, coerce_to };
use super::super::stream::StreamContents;
use crate::typeinf::{ MemberMode, MemberDataFlow };

pub struct LenCommand(pub(crate) Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>,pub(crate) Vec<Register>);

impl Command for LenCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let mut top_reg = None;
        for (j,purpose) in self.0[1].1.iter().enumerate() {
            match purpose.get_linear() {
                LinearPath::Length(_) => {
                    if purpose.is_top() {
                        top_reg = Some(&self.1[j+1]);
                    }
                },
                _ => {}
            }
        }
        let lens = registers.read_indexes(top_reg.unwrap())?;
        registers.set_indexes(&self.1[0],lens.borrow().to_vec())?;
        Ok(())
    }
}

pub(crate) enum InterpBinBoolOp {
    Lt,
    LtEq,
    Gt,
    GtEq
}

impl InterpBinBoolOp {
    fn evaluate(&self, a: f64, b: f64) -> bool {
        match self {
            InterpBinBoolOp::Lt => a < b,
            InterpBinBoolOp::LtEq => a <= b,
            InterpBinBoolOp::Gt => a > b,
            InterpBinBoolOp::GtEq => a >= b
        }
    }
}

pub struct InterpBinBoolCommand(pub(crate) InterpBinBoolOp, pub(crate) Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>,pub(crate) Vec<Register>);

impl Command for InterpBinBoolCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let a = registers.read_numbers(&self.2[1])?;
        let b = registers.read_numbers(&self.2[2])?;
        let c = registers.write_boolean(&self.2[0])?;
        let mut c = c.borrow_mut();
        let b = b.borrow();
        let b_len = b.len();
        for (i,a_val) in a.borrow().iter().enumerate() {
            c.push(self.0.evaluate(*a_val,b[i%b_len]));
        }
        Ok(())
    }
}

fn eq<T>(c: ReadWriteValues<bool>, a: ReadOnlyValues<T>, b: ReadOnlyValues<T>) where T: PartialEq {
    let a = a.borrow();
    let b = b.borrow();
    let mut c = c.borrow_mut();
    let b_len = b.len();
    for (i,av) in a.iter().enumerate() {
        c.push(av == &b[i%b_len]);
    }
}

pub struct EqCommand(pub(crate) Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>,pub(crate) Vec<Register>);

impl Command for EqCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let a = registers.get(&self.1[1]);
        let a = a.read()?;
        let b = registers.get(&self.1[2]);
        let b = b.read()?;
        let c = registers.write_boolean(&self.1[0])?;
        if let Some(natural) = coerce_to(&a,&b,true) {
            match natural {
                InterpNatural::Empty => {},
                InterpNatural::Numbers => { let a = a.read_numbers()?; let b = b.read_numbers()?; eq(c,a,b); },
                InterpNatural::Indexes => { let a = a.read_indexes()?; let b = b.read_indexes()?; eq(c,a,b); },
                InterpNatural::Boolean => { let a = a.read_boolean()?; let b = b.read_boolean()?; eq(c,a,b); },
                InterpNatural::Strings => { let a = a.read_strings()?; let b = b.read_strings()?; eq(c,a,b); },
                InterpNatural::Bytes => { let a = a.read_bytes()?; let b = b.read_bytes()?; eq(c,a,b); },
            }
        }
        Ok(())
    }
}

pub struct PrintRegsCommand(pub(crate) Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>,pub(crate) Vec<Register>);

impl Command for PrintRegsCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        for r in &self.1 {
            let v = StreamContents::Data(context.registers().get(r).read()?.copy());
            context.stream_add(v);
        }
        Ok(())
    }
}

fn print_value<T>(data: &ReadOnlyValues<T>, start: usize, len: usize) -> String where T: std::fmt::Display {
    let mut out = Vec::new();
    let data = data.borrow();
    for index in start..start+len {
        out.push(data[index].to_string());
    }
    out.join(", ")
}

fn print_register(context: &mut InterpContext, reg: &Register, restrict: Option<(usize,usize)>) -> Result<String,String> {
    let value = context.registers().get(reg);
    let value = value.read()?;
    let (start,len) = restrict.unwrap_or_else(|| { (0,value.len()) });
    Ok(match value.get_natural() {
        InterpNatural::Empty => { String::new() },
        InterpNatural::Numbers => { print_value(&value.read_numbers()?, start, len) },
        InterpNatural::Indexes => { print_value(&value.read_indexes()?, start, len) },
        InterpNatural::Boolean => { print_value(&value.read_boolean()?, start, len) },
        InterpNatural::Strings => { print_value(&value.read_boolean()?, start, len) },
        InterpNatural::Bytes => { print_value(&value.read_boolean()?, start, len) },
    })
}

fn print_base(context: &mut InterpContext, purposes: &[&RegisterPurpose], regs: &[Register], indexes: &[usize], restrict: Option<(usize,usize)>) -> Result<String,String> {
    let mut data_reg = None;
    for j in indexes {
        match purposes[*j].get_linear() {
            LinearPath::Data => { data_reg = Some(*j); },
            _ => {}
        }
    }
    print_register(context,&regs[data_reg.unwrap()],restrict)
}

fn print_level(context: &mut InterpContext, purposes: &[&RegisterPurpose], regs: &[Register], level_in: i64, indexes: &[usize], restrict: Option<(usize,usize)>) -> Result<String,String> {
    if level_in > -1 {
        let level = level_in as usize;
        /* find registers for level */
        let mut offset_reg = None;
        let mut len_reg = None;
        for j in indexes {
            match purposes[*j].get_linear() {
                LinearPath::Offset(v) if level == *v => { offset_reg = Some(*j); },
                LinearPath::Length(v) if level == *v => { len_reg = Some(*j); },
                _ => {}
            }
        }
        let starts = context.registers().read_indexes(&regs[offset_reg.unwrap()])?;
        let lens = context.registers().read_indexes(&regs[len_reg.unwrap()])?;
        let starts = &starts.borrow();
        let lens = &lens.borrow();
        let lens_len = lens.len();
        let (a,b) = restrict.unwrap_or((0,lens_len));
        let mut members = Vec::new();
        for index in a..a+b {
            members.push(print_level(context,purposes,regs,level_in-1,indexes,Some((starts[index],lens[index%lens_len])))?);
        }
        Ok(format!("[{}]",members.join(",")))
    } else {
        print_base(context,purposes,regs,indexes,restrict)
    }
}

fn print_array(context: &mut InterpContext, purposes: &[&RegisterPurpose], regs: &[Register], indexes: &[usize]) -> Result<String,String> {
    let mut top_level = -1_i64;
    for index in indexes {
        let purpose = purposes[*index];
        if purpose.is_top() {
            if let LinearPath::Offset(top) = purpose.get_linear() {
                top_level = *top as i64;
            }
        }
    }
    print_level(context,purposes,regs,top_level,indexes,None)
}

fn print_complex(context: &mut InterpContext, purposes: &[&RegisterPurpose], regs: &[Register], complex: &[String], indexes: &[usize], is_complex: bool) -> Result<String,String> {
    if is_complex {
        Ok(format!("{}: {}",complex.join("."),print_array(context,purposes,regs,indexes)?))
    } else {
        print_array(context,purposes,regs,indexes)
    }
}

fn print_vec(context: &mut InterpContext, purposes: &Vec<&RegisterPurpose>, regs: &Vec<Register>) -> Result<String,String> {
    let mut out : Vec<String> = vec![];
    let mut complexes : HashMap<Vec<String>,Vec<usize>> = HashMap::new();
    let mut is_complex = false;
    for (i,purpose) in purposes.iter().enumerate() {
        let complex = purpose.get_complex().to_vec();
        if complex.len() > 0 { is_complex = true; }
        complexes.entry(complex).or_insert_with(|| { vec![] }).push(i);
    }
    for (complex,indexes) in complexes.iter() {
        out.push(print_complex(context,purposes,regs,complex,indexes,is_complex)?);
    }
    let mut out = out.join("; ");
    if is_complex { out = format!("{{ {} }}",out); }
    Ok(out)
}

pub struct PrintVecCommand(pub(crate) Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>,pub(crate) Vec<Register>);

impl Command for PrintVecCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let purposes = self.0.iter().map(|x| &x.1[0]).collect::<Vec<_>>();
        let v = StreamContents::String(print_vec(context,&purposes,&self.1)?);
        context.stream_add(v);
        Ok(())
    }
}
