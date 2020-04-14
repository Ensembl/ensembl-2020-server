use std::collections::HashMap;
use super::super::context::{InterpContext };
use crate::interp::{ InterpNatural, InterpValue };
use super::super::command::Command;
use crate::model::{ LinearPath, Register, RegisterPurpose };
use super::assign::coerce_to;
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
        registers.copy(&self.1[0],top_reg.ok_or_else(|| format!("Not a list"))?)?;
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
        let a = registers.get_numbers(&self.2[1])?;
        let b = &registers.get_numbers(&self.2[2])?;
        let mut c = vec![];
        let b_len = b.len();
        for (i,a_val) in a.iter().enumerate() {
            c.push(self.0.evaluate(*a_val,b[i%b_len]));
        }
        registers.write(&self.2[0],InterpValue::Boolean(c));
        Ok(())
    }
}

fn eq<T>(c: &mut Vec<bool>, a: &[T], b: &[T]) where T: PartialEq {
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
        let a = a.borrow().get_shared()?;
        let b = registers.get(&self.1[2]);
        let b = b.borrow().get_shared()?;
        let mut c = vec![];
        if let Some(natural) = coerce_to(&a,&b,true) {
            match natural {
                InterpNatural::Empty => {},
                InterpNatural::Numbers => { eq(&mut c,&a.to_rc_numbers()?.0,&b.to_rc_numbers()?.0); },
                InterpNatural::Indexes => { eq(&mut c,&a.to_rc_indexes()?.0,&b.to_rc_indexes()?.0); },
                InterpNatural::Boolean => { eq(&mut c,&a.to_rc_boolean()?.0,&b.to_rc_boolean()?.0); },
                InterpNatural::Strings => { eq(&mut c,&a.to_rc_strings()?.0,&b.to_rc_strings()?.0); },
                InterpNatural::Bytes =>   { eq(&mut c,&a.to_rc_bytes()?.0,  &b.to_rc_bytes()?.0); },
            }
        }
        registers.write(&self.1[0],InterpValue::Boolean(c));
        Ok(())
    }
}

pub struct PrintRegsCommand(pub(crate) Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>,pub(crate) Vec<Register>);

impl Command for PrintRegsCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        for r in &self.1 {
            let v = StreamContents::Data(context.registers().get(r).borrow().get_shared()?.copy());
            context.stream_add(v);
        }
        Ok(())
    }
}

fn print_value<T>(data: &[T], start: usize, len: usize) -> String where T: std::fmt::Display {
    let mut out = Vec::new();
    print!("print_value({};{};{})\n",data.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","),start,len);
    for index in start..start+len {
        out.push(data[index].to_string());
    }
    out.join(",")
}

fn print_bytes<T>(data: &[Vec<T>], start: usize, len: usize) -> String where T: std::fmt::Display {
    let mut out = vec![];
    for index in start..start+len {
        out.push(format!("[{}]",data[index].iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ")));
    }
    out.join(",")
}

fn print_register(context: &mut InterpContext, reg: &Register, restrict: Option<(usize,usize)>) -> Result<String,String> {
    let value = context.registers().get(reg);
    print!("A\n");
    let value = value.borrow().get_shared()?;
    print!("print_register({}) = {:?}\n",reg,value);
    let (start,len) = restrict.unwrap_or_else(|| { (0,value.len()) });
    Ok(match value.get_natural() {
        InterpNatural::Empty => { "[]".to_string() },
        InterpNatural::Numbers => { print_value(&value.to_rc_numbers()?.0, start, len) },
        InterpNatural::Indexes => { print_value(&value.to_rc_indexes()?.0, start, len) },
        InterpNatural::Boolean => { print_value(&value.to_rc_boolean()?.0, start, len) },
        InterpNatural::Strings => { print_value(&value.to_rc_strings()?.0, start, len) },
        InterpNatural::Bytes => { print_bytes(&value.to_rc_bytes()?.0, start, len) },
    })
}

fn print_base(context: &mut InterpContext, purposes: &[RegisterPurpose], regs: &[Register], indexes: &[usize], restrict: Option<(usize,usize)>) -> Result<String,String> {
    let mut data_reg = None;
    for j in indexes {
        match purposes[*j].get_linear() {
            LinearPath::Data => { data_reg = Some(*j); },
            _ => {}
        }
    }
    print_register(context,&regs[data_reg.unwrap()],restrict)
}

fn print_level(context: &mut InterpContext, purposes: &[RegisterPurpose], regs: &[Register], level_in: i64, indexes: &[usize], restrict: Option<(usize,usize)>) -> Result<String,String> {
    if level_in > -1 {
        let level = level_in as usize;
        /* find registers for level */
        let mut offset_reg = None;
        let mut len_reg = None;
        print!("purposes {:?} level={}\n",purposes,level);
        for j in indexes {
            match purposes[*j].get_linear() {
                LinearPath::Offset(v) if level == *v => { offset_reg = Some(*j); },
                LinearPath::Length(v) if level == *v => { len_reg = Some(*j); },
                _ => {}
            }
        }
        print!("offset={:?} length={:?}\n",offset_reg,len_reg);
        let starts = &context.registers().get_indexes(&regs[offset_reg.unwrap()])?;
        let lens = &context.registers().get_indexes(&regs[len_reg.unwrap()])?;
        print!("starts={:?} lens={:?} restrict={:?}\n",starts.to_vec(),lens.to_vec(),restrict);
        let lens_len = lens.len();
        let (a,b) = restrict.unwrap_or((0,lens_len));
        let mut members = Vec::new();
        for index in a..a+b {
            members.push(print_level(context,purposes,regs,level_in-1,indexes,Some((starts[index],lens[index%lens_len])))?);
        }
        Ok(format!("{}",members.iter().map(|x| format!("[{}]",x)).collect::<Vec<_>>().join(",")))
    } else {
        print!("print_base indexes={:?} restrict={:?}\n",indexes,restrict);
        print_base(context,purposes,regs,indexes,restrict)
    }
}

fn print_array(context: &mut InterpContext, purposes: &[RegisterPurpose], regs: &[Register], indexes: &[usize]) -> Result<String,String> {
    let mut top_level = -1_i64;
    for i in indexes {
        let purpose = &purposes[*i];
        if purpose.is_top() {
            if let LinearPath::Offset(top) = purpose.get_linear() {
                top_level = *top as i64;
            }
        }
    }
    print!("top_level={}\nindexes={:?}\npurposes={:?}\n",top_level,indexes,purposes);
    let mut out = print_level(context,purposes,regs,top_level,indexes,None)?;
    if out.len() == 0 { out = "-".to_string() }
    Ok(out)
}

fn print_complex(context: &mut InterpContext, purposes: &[RegisterPurpose], regs: &[Register], complex: &[String], indexes: &[usize], is_complex: bool) -> Result<String,String> {
    if is_complex {
        print!("complex: {:?}\n",complex);
        let name = if complex.len() > 0 { complex.join(".") } else { "*".to_string() };
        Ok(format!("{}: {}",name,print_array(context,purposes,regs,indexes)?))
    } else {
        print_array(context,purposes,regs,indexes)
    }
}

fn print_vec(context: &mut InterpContext, purposes: &Vec<RegisterPurpose>, regs: &Vec<Register>) -> Result<String,String> {
    let mut out : Vec<String> = vec![];
    let mut complexes : HashMap<Vec<String>,Vec<usize>> = HashMap::new();
    let mut is_complex = false;
    for (i,purpose) in purposes.iter().enumerate() {
        let complex = purpose.get_complex().to_vec();
        if complex.len() > 0 { is_complex = true; }
        complexes.entry(complex).or_insert_with(|| { vec![] }).push(i);
    }
    let mut complex_keys = complexes.keys().map(|x| x.to_vec()).collect::<Vec<_>>();
    complex_keys.sort();
    for complex in complex_keys.iter() {
        let indexes = complexes.get(complex).unwrap();
        out.push(print_complex(context,purposes,regs,complex,indexes,is_complex)?);
    }
    let mut out = out.join("; ");
    if is_complex { out = format!("{{ {} }}",out); }
    Ok(out)
}

pub struct PrintVecCommand(pub(crate) Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>,pub(crate) Vec<Register>);

impl Command for PrintVecCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let purposes = &self.0[0].1;
        let v = StreamContents::String(print_vec(context,&purposes,&self.1)?);
        context.stream_add(v);
        Ok(())
    }
}

pub struct AssertCommand(pub(crate) Register, pub(crate) Register);

impl Command for AssertCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let a = &registers.get_boolean(&self.0)?;
        let b = &registers.get_boolean(&self.1)?;
        for i in 0..a.len() {
            if a[i] != b[i] {
                return Err(format!("assertion failed index={}!",i));
            }
        }
        Ok(())
    }
}
