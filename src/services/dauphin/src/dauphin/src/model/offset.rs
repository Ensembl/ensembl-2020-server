use std::fmt;
use super::definitionstore::DefStore;
use super::structenum::{ EnumDef, StructDef };
use crate::typeinf::{ BaseType, ContainerType, MemberType };

#[derive(Debug,Clone)]
pub enum LinearPath {
    Data,
    Offset(usize),
    Length(usize)
}

impl fmt::Display for LinearPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinearPath::Data => write!(f,"D"),
            LinearPath::Offset(n) => write!(f,"A{}",n),
            LinearPath::Length(n) => write!(f,"B{}",n)
        }
    }
}

#[derive(Debug,Clone)]
pub struct RegisterPurpose {
    complex: Vec<String>,
    linear: LinearPath,
    base: BaseType,
    top: bool
}

impl fmt::Display for RegisterPurpose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for path in &self.complex {
            write!(f,"{}.",path)?;
        }
        write!(f,"{}",self.linear)?;
        match self.linear {
            LinearPath::Data => write!(f,"/{}",self.base)?,
            _ => {}
        }
        Ok(())
    }
}

// XXX deduplicate from_struct/from_enum by shifting to StructEnum universally
impl RegisterPurpose {
    fn vec_from_type(defstore: &DefStore, type_: &MemberType, prefix: &Vec<String>, container: &ContainerType) -> Result<Vec<RegisterPurpose>,()> {
        let container = container.merge(&type_.get_container());
        match type_.get_base() {
            BaseType::StructType(name) => {
                let struct_ = defstore.get_struct(&name).unwrap();
                RegisterPurpose::from_struct(defstore,struct_,prefix,&container)
            },
            BaseType::EnumType(name) => {
                let enum_ = defstore.get_enum(&name).unwrap();
                RegisterPurpose::from_enum(defstore,enum_,prefix,&container)
            },
            base => {
                let mut out = Vec::new();
                out.push(RegisterPurpose {
                    complex: prefix.to_vec(),
                    linear: LinearPath::Data,
                    base,
                    top: container.depth() == 0
                });
                for i in 0..container.depth() {
                    let top = i == container.depth()-1;
                    out.push(RegisterPurpose {
                        complex: prefix.to_vec(),
                        linear: LinearPath::Offset(i),
                        base: BaseType::NumberType,
                        top
                    });
                    out.push(RegisterPurpose {
                        complex: prefix.to_vec(),
                        linear: LinearPath::Length(i),
                        base: BaseType::NumberType,
                        top
                    });
                }
                Ok(out)
            }
        }
    }

    fn from_struct(defstore: &DefStore, se: &StructDef, cpath: &Vec<String>, container: &ContainerType) -> Result<Vec<RegisterPurpose>,()> {
        let mut out = Vec::new();
        for name in se.get_names() {
            let mut new_cpath = cpath.to_vec();
            new_cpath.push(name.to_string());
            let type_ = se.get_member_type(name).unwrap();
            out.append(&mut RegisterPurpose::vec_from_type(defstore,&type_,&new_cpath,container)?);
        }
        Ok(out)
    }

    fn from_enum(defstore: &DefStore, se: &EnumDef, cpath: &Vec<String>, container: &ContainerType) -> Result<Vec<RegisterPurpose>,()> {
        let mut out = Vec::new();
        for name in se.get_names() {
            let mut new_cpath = cpath.to_vec();
            new_cpath.push(name.to_string());
            let type_ = se.get_branch_type(name).unwrap();
            out.append(&mut RegisterPurpose::vec_from_type(defstore,&type_,&new_cpath,container)?);
        }
        Ok(out)
    }

    pub fn get_linear(&self) -> &LinearPath { &self.linear }
    pub fn is_top(&self) -> bool { self.top }
}

pub fn offset(defstore: &DefStore, type_: &MemberType) -> Result<Vec<RegisterPurpose>,()> {
    RegisterPurpose::vec_from_type(defstore,type_,&vec![],&ContainerType::new_empty())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser, parse_type };
    use crate::generate::generate_code;

    // XXX move to common test utils
    fn make_type(defstore: &DefStore, name: &str) -> MemberType {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import(&format!("data:{}",name)).expect("cannot load file");
        parse_type(&mut lexer,defstore).expect("bad type")
    }

    fn format_pvec(pp: &Vec<RegisterPurpose>) -> String {
        let mut first = true;
        let mut out = String::new();
        for p in pp {
            if first {
                first = false;
            } else {
                out.push_str(",");
            }
            out.push_str(&p.to_string());
        }
        out
    }

    #[test]
    fn offset_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/offset-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let context = generate_code(&defstore,stmts).expect("codegen");
        let regs = offset(&defstore,&make_type(&defstore,"boolean")).expect("a");
        assert_eq!("D/boolean",format_pvec(&regs));
        let regs = offset(&defstore,&make_type(&defstore,"vec(etest3)")).expect("b");
        assert_eq!("A.A.D/number,A.A.A0,A.A.B0,A.A.A1,A.A.B1,A.B.X.D/string,A.B.X.A0,A.B.X.B0,A.B.Y.D/boolean,A.B.Y.A0,A.B.Y.B0,B.X.D/string,B.X.A0,B.X.B0,B.Y.D/boolean,B.Y.A0,B.Y.B0,C.D/boolean,C.A0,C.B0,D.D/number,D.A0,D.B0,D.A1,D.B1,D.A2,D.B2",format_pvec(&regs));
    }
}