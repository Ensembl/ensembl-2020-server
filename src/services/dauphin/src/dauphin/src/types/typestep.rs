use super::typeinf::{ Referrer, TypeInf };
use crate::codegen::Register;
use super::types::{ TypeSig, Sig };

pub fn try_apply_command(typeinf: &mut TypeInf, typesig: &Vec<(Sig,Register)>, allow_typechange: bool) -> Result<(),String> {
    let mut unifies = Vec::new();
    let mut check_valid = Vec::new();
    let mut xform = Vec::new();
    //print!("tac {:?}\n",typesig);
    for (sig,reg) in typesig {
        let reg = typeinf.new_register(reg);
        if !sig.out {
            check_valid.push(reg.clone());
        }
        let tmp = typeinf.new_temp().clone();
        //print!("allocated {:?} for incoming type of arg ({:?},{:?}) and unifying {:?}={:?}\n",tmp,sig,reg,reg,tmp);
        typeinf.add(&tmp,&sig.typesig);
        unifies.push((reg.clone(),tmp.clone()));
        if sig.lvalue.is_some() {
            let ltmp = typeinf.new_temp();
            //print!("allocated {:?} for outgoing type of arg ({:?},{:?})\n",ltmp,sig,reg);
            typeinf.add(&ltmp,sig.lvalue.as_ref().unwrap());
            xform.push((reg.clone(),ltmp,tmp));
        }
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
    for (reg,tmp,old) in &xform {
        let old_sig = typeinf.get_sig(old).clone();
        let tmp_sig = typeinf.get_sig(tmp).clone();
        let reg_sig = typeinf.get_sig(reg).clone();
        match &reg_sig {
            TypeSig::Left(_,r) => {
                //print!("updating lvalue variable (indirect write). outgoing type={:?} reg={:?} sig={:?}\n",tmp_sig,reg,sig);
                let excuse_consistency = if let Register::Named(_) = r { allow_typechange } else { false };
                //print!("referee old={:?} [{:?}]  new={:?} \n",old_sig,reg_sig,tmp_sig);
                if !old_sig.is_invalid() && !excuse_consistency {
                    typeinf.unify(old,tmp)?;
                }
                /* referer */
                typeinf.add(&reg,&TypeSig::Left(tmp_sig.expr().clone(),r.clone()));
                /* referee */
                typeinf.add(&Referrer::Register(r.clone()),&tmp_sig.clone());
                //print!("now {:?}\n",typeinf.get_sig(&Referrer::Register(r.clone())));
            },
            TypeSig::Right(_) => {
                //print!("updating rvalue variable (direct write). outgoing type={:?} reg={:?}\n",tmp_sig,reg);
                typeinf.add(&reg,&tmp_sig.clone());
            }
        }
    }
    Ok(())
}
