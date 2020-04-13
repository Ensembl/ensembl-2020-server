use std::fmt;
use super::definitionstore::DefStore;
use super::structenum::{ EnumDef, StructDef };
use crate::typeinf::{ BaseType, ContainerType, MemberType };

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub enum LinearPath {
    Data,
    Offset(usize),
    Length(usize)
}

impl LinearPath {
    pub fn references(&self) -> Option<LinearPath> {
        match self {
            LinearPath::Offset(0) => Some(LinearPath::Data),
            LinearPath::Offset(n) => Some(LinearPath::Offset(n-1)),
            _ => None
        }
    }
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

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub struct RegisterPurpose {
    complex: Vec<String>,
    linear: LinearPath,
    base: BaseType,
    top: bool
}

impl fmt::Display for RegisterPurpose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = self.complex.iter().map(|x| format!("{}",x)).collect::<Vec<_>>();
        let linear = match self.linear {
            LinearPath::Data => format!("{}/{}",self.linear,self.base),
            _ => format!("{}",self.linear)
        };
        parts.push(linear);
        Ok(write!(f,"{}",parts.join("."))?)
    }
}

// XXX deduplicate from_struct/from_enum by shifting to StructEnum universally
impl RegisterPurpose {
    fn register_sequence(prefix: &[String], base: BaseType, levels: usize) -> Vec<RegisterPurpose> {
        let mut out = Vec::new();
        out.push(RegisterPurpose {
            complex: prefix.to_vec(),
            linear: LinearPath::Data,
            base,
            top: levels == 0
        });
        for i in 0..levels {
            let top = i == levels-1;
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
        out
    }

    fn vec_from_type(defstore: &DefStore, type_: &MemberType, prefix: &[String], container: &ContainerType) -> Result<Vec<RegisterPurpose>,String> {
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
                Ok(RegisterPurpose::register_sequence(prefix,base,container.depth()))
            }
        }
    }

    fn from_struct(defstore: &DefStore, se: &StructDef, cpath: &[String], container: &ContainerType) -> Result<Vec<RegisterPurpose>,String> {
        let mut out = Vec::new();
        for name in se.get_names() {
            let mut new_cpath = cpath.to_vec();
            new_cpath.push(name.to_string());
            let type_ = se.get_member_type(name).unwrap();
            out.append(&mut RegisterPurpose::vec_from_type(defstore,&type_,&new_cpath,container)?);
        }
        Ok(out)
    }

    fn from_enum(defstore: &DefStore, se: &EnumDef, cpath: &[String], container: &ContainerType) -> Result<Vec<RegisterPurpose>,String> {
        let mut out = RegisterPurpose::register_sequence(cpath,BaseType::NumberType,container.depth());
        for name in se.get_names() {
            let mut new_cpath = cpath.to_vec();
            new_cpath.push(name.to_string());
            let type_ = se.get_branch_type(name).unwrap();
            out.append(&mut RegisterPurpose::vec_from_type(defstore,&type_,&new_cpath,container)?);
        }
        Ok(out)
    }

    pub fn get_complex(&self) -> &Vec<String> { &self.complex }
    pub fn get_linear(&self) -> &LinearPath { &self.linear }
    pub fn is_top(&self) -> bool { self.top }
}   

pub fn offset(defstore: &DefStore, type_: &MemberType) -> Result<Vec<RegisterPurpose>,String> {
    RegisterPurpose::vec_from_type(defstore,type_,&vec![],&ContainerType::new_empty())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser, parse_type };
    use crate::generate::generate_code;
    use crate::testsuite::load_testdata;

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
        let _context = generate_code(&defstore,stmts).expect("codegen");
        let regs = offset(&defstore,&make_type(&defstore,"boolean")).expect("a");
        assert_eq!("D/boolean",format_pvec(&regs));
        let regs = offset(&defstore,&make_type(&defstore,"vec(etest3)")).expect("b");
        let outdata = load_testdata(&["codegen","offset-smoke.out"]).ok().unwrap();
        let mut seq = vec![];
        for line in outdata.split("\n") {
            if line.starts_with("+") {
                if let Some(part) = line.split_ascii_whitespace().nth(1) {
                    seq.push(part);
                }
            }
        }
        let seq = seq.join(",");
        assert_eq!(seq,format_pvec(&regs));
    }
}