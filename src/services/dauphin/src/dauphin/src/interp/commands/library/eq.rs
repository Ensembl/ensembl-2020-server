use crate::interp::context::{InterpContext };
use crate::interp::{ InterpNatural, InterpValue };
use crate::model::Register;
use crate::interp::commands::assign::coerce_to;
use crate::interp::commandsets::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet };
use crate::generate::{ Instruction, InstructionType };
use serde_cbor::Value as CborValue;

pub struct EqCommandType();

impl CommandType for EqCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command("assert".to_string())
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig) = &it.itype {
            Ok(Box::new(EqCommand(it.regs[0],it.regs[1],it.regs[2])))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(EqCommand(Register::deserialize(&value[0])?,Register::deserialize(&value[1])?,Register::deserialize(&value[2])?)))
    }
}

fn eq<T>(c: &mut Vec<bool>, a: &[T], b: &[T]) where T: PartialEq {
    let b_len = b.len();
    for (i,av) in a.iter().enumerate() {
        c.push(av == &b[i%b_len]);
    }
}

pub struct EqCommand(pub(crate) Register, pub(crate) Register, pub(crate) Register);

impl Command for EqCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let a = registers.get(&self.1);
        let a = a.borrow().get_shared()?;
        let b = registers.get(&self.2);
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
        registers.write(&self.0,InterpValue::Boolean(c));
        Ok(())
    }
}

pub(super) fn library_eq_command(set: &mut CommandSet) -> Result<(),String> {
    set.push("eq",0,EqCommandType())?;
    Ok(())
}