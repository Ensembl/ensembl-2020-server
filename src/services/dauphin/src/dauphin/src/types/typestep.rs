use super::argumentmatch::ArgumentMatch;
use super::signaturematch::SignatureMatch;
use super::typeinf::{ Referrer, TypeInf };
use crate::codegen::Register;
use super::types::{ TypeSig, ArgumentType };

pub struct TypeStep {
    unify: Vec<(Referrer,Referrer)>,
    check_valid: Vec<Referrer>,
    xform: Vec<(Referrer,Referrer)>
}

impl TypeStep {
    pub fn new() -> TypeStep {
        TypeStep {
            unify: Vec::new(),
            check_valid: Vec::new(),
            xform: Vec::new()
        }
    }

    pub fn add_unify(&mut self, a: &Referrer, b: &Referrer) {
        self.unify.push((a.clone(),b.clone()));
    }

    pub fn check_valid(&mut self, a: &Referrer) {
        self.check_valid.push(a.clone());
    }

    pub fn check_outputs(&mut self, a: &Referrer, b: &Referrer) {
        self.xform.push((a.clone(),b.clone()));
    }

    pub fn apply_step(&self, typeinf: &mut TypeInf, allow_typechange: bool) -> Result<(),String> {
        for (reg,tmp) in &self.unify {
            typeinf.unify(&reg,&tmp)?;
        }
        for reg in &self.check_valid {
            let sig = typeinf.get_typepattern(reg);
            if sig.is_invalid() {
                return Err(format!("Use of invalid value from {:?}",reg));
            }
        }
        for (reg,out_var) in &self.xform {
            let out_type = typeinf.get_sig(out_var).clone();
            let reg_sig = typeinf.get_sig(reg).clone();
            match &reg_sig {
                TypeSig::Left(_,r) => {
                    let in_type = typeinf.get_sig(&Referrer::Register(r.clone())).clone();
                    let excuse_consistency = if let Register::Named(_) = r { allow_typechange } else { false };
                    if !in_type.is_invalid() && !excuse_consistency {
                        typeinf.unify(&Referrer::Register(r.clone()),out_var)?;
                    }
                    typeinf.add(&reg,&TypeSig::Left(out_type.expr().clone(),r.clone()));
                    typeinf.add(&Referrer::Register(r.clone()),&out_type.clone());
                },
                TypeSig::Right(_) => {
                    typeinf.add(&reg,&out_type.clone());
                }
            }
        }
        Ok(())
    }
}

pub fn type_step(typeinf: &mut TypeInf, typesig: &SignatureMatch, allow_typechange: bool) -> Result<(),String> {
    let mut step = TypeStep::new();
    for arg in typesig.each_argument() {
        if arg.get_type().get_writeonly() {
            let reg = typeinf.new_register(arg.get_register());
            let ltmp = typeinf.new_temp(arg.get_type().get_type());
            step.check_outputs(&reg,&ltmp);
        } else {
            let reg = typeinf.new_register(arg.get_register());
            let tmp = typeinf.new_temp(&arg.get_type().get_type()).clone();
            step.add_unify(&reg,&tmp);
            step.check_valid(&reg);
        }
    }
    step.apply_step(typeinf,allow_typechange)?;
    Ok(())
}
