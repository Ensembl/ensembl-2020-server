use super::typeinf::{ Referrer, TypeInf };
use crate::codegen::Register;
use super::types::{ TypeSig, Sig };

pub fn try_apply_command(typeinf: &mut TypeInf, typesig: &Vec<(Sig,Register)>) -> Result<(),String> {
    let mut unifies = Vec::new();
    let mut check_valid = Vec::new();
    let mut xform = Vec::new();
    for (sig,reg) in typesig {
        let reg = typeinf.new_register(reg);
        if !sig.out {
            check_valid.push(reg.clone());
        }
        let tmp = typeinf.new_temp().clone();
        if sig.lvalue.is_some() {
            let ltmp = typeinf.new_temp();
            typeinf.add(&ltmp,sig.lvalue.as_ref().unwrap());
            xform.push((reg.clone(),ltmp,tmp.clone()));
        }
        typeinf.add(&tmp,&sig.typesig);
        unifies.push((reg,tmp));
    }
    for (reg,tmp) in &unifies {
        typeinf.unify(&reg,&tmp)?;
    }
    for reg in &check_valid {
        let sig = typeinf.get_sig(reg);
        if sig.is_invalid() {
            return Err(format!("Use of invalid value from {:?}",reg));
        }
    }
    for (reg,tmp,rtmp) in &xform {
        let tmp_sig = typeinf.get_sig(tmp).clone();
        let reg_sig = typeinf.get_sig(reg).clone();
        match &reg_sig {
            TypeSig::Left(_,r) => {
                typeinf.unify(&Referrer::Register(r.clone()),rtmp)?;
                typeinf.add(&Referrer::Register(r.clone()),&tmp_sig.clone());
                typeinf.add(&reg,&TypeSig::Left(tmp_sig.expr().clone(),r.clone()));
            },
            TypeSig::Right(_) => {
                typeinf.add(&reg,&tmp_sig.clone());
            }
        }
    }
    Ok(())
}
