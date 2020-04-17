/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *  
 *  vscode-fold=1
 */

use std::iter::Iterator;
use std::ops::Index;
use std::slice::SliceIndex;
use crate::model::cbor_array;
use super::complexsig::ComplexRegisters;
use serde_cbor::Value as CborValue;

#[derive(Clone,Debug,PartialEq)]
pub struct RegisterSignature {
    index: usize,
    args: Vec<ComplexRegisters>
}

impl RegisterSignature {
    pub fn new() -> RegisterSignature {
        RegisterSignature {
            index: 0,
            args: Vec::new()
        }
    }

    pub fn add(&mut self, mut cr: ComplexRegisters) {
        cr.add_start(self.index);
        self.index += cr.register_count();
        self.args.push(cr);
    }

    pub fn iter<'a>(&'a self) -> RegisterSignatureIterator<'a> {
        RegisterSignatureIterator {
            rs: self,
            index: 0
        }
    }

    pub fn serialize(&self, named: bool) -> Result<CborValue,String> {
        Ok(CborValue::Array(self.args.iter().map(|x| x.serialize(named)).collect::<Result<Vec<_>,_>>()?))
    }

    pub fn deserialize(cbor: &CborValue, named: bool) -> Result<RegisterSignature,String> {
        let mut out = RegisterSignature::new();
        for cr in cbor_array(cbor,0,true)?.iter().map(|x| ComplexRegisters::deserialize(x,named)).collect::<Result<Vec<_>,_>>()? {
            out.add(cr);
        }
        Ok(out)
    }
}

pub struct RegisterSignatureIterator<'a> {
    rs: &'a RegisterSignature,
    index: usize
}

impl<'a> Iterator for RegisterSignatureIterator<'a> {
    type Item = &'a ComplexRegisters;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.rs.args.len() {
            let out = Some(&self.rs.args[self.index]);
            self.index += 1;
            out
        } else {
            None
        }
    }
}

impl<I> Index<I> for RegisterSignature where I: SliceIndex<[ComplexRegisters]> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.args.index(index)
    }
}

// XXX deduplicate from_struct/from_enum by shifting to StructEnum universally

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser, parse_type };
    use crate::generate::generate_code;
    use crate::test::files::load_testdata;
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
    use crate::test::cbor::cbor_cmp;
    use crate::model::{ DefStore };
    use crate::typeinf::{ MemberType, MemberMode, MemberDataFlow };

    // XXX move to common test utils
    fn make_type(defstore: &DefStore, name: &str) -> MemberType {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import(&format!("data:{}",name)).expect("cannot load file");
        parse_type(&mut lexer,defstore).expect("bad type")
    }

    fn format_pvec(ass: &ComplexRegisters) -> String {
        ass.to_string()
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
        let regs = ComplexRegisters::new(&defstore,MemberMode::RValue,&make_type(&defstore,"boolean"),MemberDataFlow::Normal).expect("a");
        assert_eq!("<0:boolean>/R",format_pvec(&regs));
        let regs = ComplexRegisters::new(&defstore,MemberMode::RValue,&make_type(&defstore,"vec(etest3)"),MemberDataFlow::Normal).expect("b");
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
        let regs = ComplexRegisters::new(&defstore,MemberMode::RValue,&make_type(&defstore,"stest"),MemberDataFlow::Normal).expect("b");
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

    #[test]
    fn test_cbor() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/offset-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let _context = generate_code(&defstore,stmts).expect("codegen");
        let regs = ComplexRegisters::new(&defstore,MemberMode::RValue,&make_type(&defstore,"vec(etest3)"),MemberDataFlow::Normal).expect("b");
        let named = regs.serialize(true).expect("cbor a");
        cbor_cmp(&named,"cbor-signature-named.out");
        let cr2 = ComplexRegisters::deserialize(&named,true).expect("cbor d");
        assert_eq!(cr2,regs);
        let anon = regs.serialize(false).expect("cbor c");
        cbor_cmp(&anon,"cbor-signature-unnamed.out");
        let cr2 = ComplexRegisters::deserialize(&anon,false).expect("cbor e");
        assert_ne!(cr2,regs);
        assert_eq!(MemberMode::RValue,cr2.get_mode());
        assert!(!cr2.justifies_call());
        let vs_in = regs.iter().map(|x| x.1).cloned().collect::<Vec<_>>();
        let vs_out = cr2.iter().map(|x| x.1).cloned().collect::<Vec<_>>();
        assert_eq!(vs_in.len(),vs_out.len());
        for (v1,v2) in Iterator::zip(vs_in.iter(),vs_out.iter()) {
            assert_eq!(v1,v2);
        }
    }
}