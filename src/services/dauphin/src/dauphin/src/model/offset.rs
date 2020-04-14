use std::collections::HashMap;  // XXX to hashbrown
use std::fmt;
use super::definitionstore::DefStore;
use super::structenum::{ EnumDef, StructDef };
use crate::typeinf::{ BaseType, ContainerType, MemberType };

#[derive(Clone,Debug,PartialEq)]
pub struct ComplexRegisters {
    order: Vec<Vec<String>>,
    vectors: HashMap<Vec<String>,VectorRegisters>
}

impl fmt::Display for ComplexRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.iter().map(|x| x.1.to_string()).collect::<Vec<_>>().join(",");
        write!(f,"{}",s)
    }
}

impl ComplexRegisters {
    pub fn new() -> ComplexRegisters {
        ComplexRegisters {
            order: Vec::new(),
            vectors: HashMap::new()
        }
    }

    fn add(&mut self, complex: Vec<String>, vr: VectorRegisters) {
        self.order.push(complex.to_vec());
        self.vectors.insert(complex,vr);
    }

    pub fn iter<'a>(&'a self) -> ComplexRegistersIterator<'a> {
        ComplexRegistersIterator {
            cr: self,
            index: 0
        }
    }
}

pub struct ComplexRegistersIterator<'a> {
    cr: &'a ComplexRegisters,
    index: usize
}

impl<'a> Iterator for ComplexRegistersIterator<'a> {
    type Item = (&'a Vec<String>,&'a VectorRegisters);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.cr.order.len() {
            let name = &self.cr.order[self.index];
            if let Some(out) = self.cr.vectors.get(name) {
                self.index += 1;
                return Some((name,out));
            }
        }
        None
    }
}

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub struct VectorRegisters {
    complex: Vec<String>,
    depth: usize,
    base: BaseType    
}

impl VectorRegisters {
    pub fn get_complex(&self) -> &Vec<String> {
        &self.complex
    }

    pub fn depth(&self) -> usize { self.depth }

    pub fn data(&self) -> usize { 0 }

    pub fn level_offset(&self, level: usize) -> Option<usize> {
        if self.depth > level { Some(level*2+1) } else { None }
    }

    pub fn level_length(&self, level: usize) -> Option<usize> {
        if self.depth > level { Some(level*2+2) } else { None }
    }

    pub fn register_count(&self) -> usize { self.depth*2+1 }

    fn vec_from_type(out: &mut ComplexRegisters, defstore: &DefStore, type_: &MemberType, prefix: &[String], container: &ContainerType) -> Result<(),String> {
        let container = container.merge(&type_.get_container());
        match type_.get_base() {
            BaseType::StructType(name) => {
                let struct_ = defstore.get_struct(&name).unwrap();
                VectorRegisters::from_struct(out,defstore,struct_,prefix,&container)
            },
            BaseType::EnumType(name) => {
                let enum_ = defstore.get_enum(&name).unwrap();
                VectorRegisters::from_enum(out,defstore,enum_,prefix,&container)
            },
            base => {
                out.add(prefix.to_vec(),VectorRegisters {
                    complex: prefix.to_vec(),
                    base,
                    depth: container.depth()
                });
                Ok(())
            }
        }
    }

    fn from_struct(out: &mut ComplexRegisters, defstore: &DefStore, se: &StructDef, cpath: &[String], container: &ContainerType) -> Result<(),String> {
        for name in se.get_names() {
            let mut new_cpath = cpath.to_vec();
            new_cpath.push(name.to_string());
            let type_ = se.get_member_type(name).unwrap();
            VectorRegisters::vec_from_type(out,defstore,&type_,&new_cpath,container)?;
        }
        Ok(())
    }

    fn from_enum(out: &mut ComplexRegisters, defstore: &DefStore, se: &EnumDef, cpath: &[String], container: &ContainerType) -> Result<(),String> {
        out.add(cpath.to_vec(),VectorRegisters {
            complex: cpath.to_vec(),
            base: BaseType::NumberType,
            depth: container.depth()
        });
        for name in se.get_names() {
            let mut new_cpath = cpath.to_vec();
            new_cpath.push(name.to_string());
            let type_ = se.get_branch_type(name).unwrap();
            VectorRegisters::vec_from_type(out,defstore,&type_,&new_cpath,container)?;
        }
        Ok(())
    }
}

impl fmt::Display for VectorRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.complex.iter().map(|x| format!("{}",x)).collect::<Vec<_>>();
        Ok(write!(f,"{}<{}:{:?}>",parts.join("."),self.depth,self.base)?)
    }
}

// XXX deduplicate from_struct/from_enum by shifting to StructEnum universally

pub fn get_assignments(defstore: &DefStore, type_: &MemberType) -> Result<ComplexRegisters,String> {
    let mut out = ComplexRegisters::new();
    VectorRegisters::vec_from_type(&mut out, defstore,type_,&vec![],&ContainerType::new_empty())?;
    Ok(out)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser, parse_type };
    use crate::generate::generate_code;
    use crate::testsuite::load_testdata;
    use crate::generate::call;
    use crate::generate::simplify;
    use crate::generate::linearize;
    use crate::generate::remove_aliases;
    use crate::generate::copy_on_write;
    use crate::generate::run_nums;
    use crate::generate::reuse_dead;
    use crate::generate::prune;
    use crate::generate::assign_regs;
    use crate::interp::mini_interp;

    // XXX move to common test utils
    fn make_type(defstore: &DefStore, name: &str) -> MemberType {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import(&format!("data:{}",name)).expect("cannot load file");
        parse_type(&mut lexer,defstore).expect("bad type")
    }

    fn format_pvec(ass: &ComplexRegisters) -> String {
        let mut first = true;
        let mut out = String::new();
        for (_,a) in ass.iter() {
            if first {
                first = false;
            } else {
                out.push_str(",");
            }
            out.push_str(&a.to_string());
        }
        out
    }

    fn load_cmp(filename: &str) -> String {
        let outdata = load_testdata(&["codegen",filename]).ok().unwrap();
        let mut seq = vec![];
        for line in outdata.split("\n") {
            if line.starts_with("+") {
                if let Some(part) = line.split_ascii_whitespace().nth(1) {
                    seq.push(part);
                }
            }
        }
        seq.join(",")
    }

    #[test]
    fn offset_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/offset-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let _context = generate_code(&defstore,stmts).expect("codegen");
        let regs = get_assignments(&defstore,&make_type(&defstore,"boolean")).expect("a");
        assert_eq!("<0:boolean>",format_pvec(&regs));
        let regs = get_assignments(&defstore,&make_type(&defstore,"vec(etest3)")).expect("b");
        assert_eq!(load_cmp("offset-smoke.out"),format_pvec(&regs));
    }

    #[test]
    fn offset_enums() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/offset-enums.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        let regs = get_assignments(&defstore,&make_type(&defstore,"stest")).expect("b");
        assert_eq!(load_cmp("offset-enums.out"),format_pvec(&regs));
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        print!("{:?}",context);
        run_nums(&mut context);
        prune(&mut context);
        copy_on_write(&mut context);
        prune(&mut context);
        run_nums(&mut context);
        reuse_dead(&mut context);
        assign_regs(&mut context);
        print!("{:?}",context);
        let (_,strings) = mini_interp(&mut context).expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
    }
}