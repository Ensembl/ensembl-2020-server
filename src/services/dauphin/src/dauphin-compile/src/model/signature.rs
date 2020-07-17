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
 */

use super::definitionstore::DefStore;
use super::structenum::{ EnumDef, StructDef };
use dauphin_interp::types::{ ComplexPath, FullType, VectorRegisters, BaseType, MemberMode };
use crate::typeinf::{ ContainerType, MemberType };

pub struct ComplexRegisters(FullType);

impl ComplexRegisters {
    fn new_empty(mode: MemberMode) -> ComplexRegisters {
        ComplexRegisters(FullType::new_empty(mode))
    }

    fn new(defstore: &DefStore, mode: MemberMode, type_: &MemberType) -> Result<FullType,String> {
        let mut out = ComplexRegisters::new_empty(mode);
        out.vec_from_type(defstore,type_,&ComplexPath::new_empty(),&ContainerType::new_empty())?;
        Ok(out.0)
    }

    fn vec_from_type(&mut self, defstore: &DefStore, type_: &MemberType, path: &ComplexPath, container: &ContainerType) -> Result<(),String> {
        let path = path.add_levels(type_.get_container().depth());
        let container = container.merge(&type_.get_container());
        match type_.get_base() {
            BaseType::StructType(name) => {
                let struct_ = defstore.get_struct_id(&name)?;
                self.from_struct(defstore,struct_,&path,&container)
            },
            BaseType::EnumType(name) => {
                let enum_ = defstore.get_enum_id(&name)?;
                self.from_enum(defstore,enum_,&path,&container)
            },
            base => {
                self.0.add(path.clone(),VectorRegisters::new(container.depth(),base));
                Ok(())
            }
        }
    }

    fn from_struct(&mut self, defstore: &DefStore, se: &StructDef, cpath: &ComplexPath, container: &ContainerType) -> Result<(),String> {
        for name in se.get_names() {
            let new_cpath = cpath.add(se.identifier(),name);
            let type_ = se.get_member_type(name).unwrap();
            self.vec_from_type(defstore,&type_,&new_cpath,container)?;
        }
        Ok(())
    }

    fn from_enum(&mut self, defstore: &DefStore, se: &EnumDef, cpath: &ComplexPath, container: &ContainerType) -> Result<(),String> {
        self.0.add(cpath.clone(),VectorRegisters::new(container.depth(),BaseType::NumberType));
        for name in se.get_names() {
            let new_cpath = cpath.add(se.identifier(),name);
            let type_ = se.get_branch_type(name).unwrap();
            self.vec_from_type(defstore,&type_,&new_cpath,container)?;
        }
        Ok(())
    }
}

pub fn make_full_type(defstore: &DefStore, mode: MemberMode, type_: &MemberType) -> Result<FullType,String> {
    ComplexRegisters::new(defstore,mode,type_)
}

#[cfg(test)]
mod test {
    use std::iter::Iterator;
    use crate::resolver::common_resolver;
    use crate::lexer::Lexer;
    use crate::parser::{ Parser, parse_type };
    use crate::generate::generate;
    use crate::test::{ mini_interp, xxx_test_config, make_compiler_suite, load_testdata, cbor_cmp };
    use crate::model::{ DefStore, make_full_type };
    use crate::typeinf::{ MemberType };
    use dauphin_interp::types::{ FullType, MemberMode };
    use crate::command::CompilerLink;

    // XXX move to common test utils
    fn make_type(defstore: &DefStore, name: &str) -> MemberType {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import(&format!("data:{}",name)).expect("cannot load file");
        parse_type(&mut lexer,defstore).expect("bad type")
    }

    fn format_pvec(ass: &FullType) -> String {
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
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:codegen/offset-smoke").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        generate(&linker,&stmts,&defstore,&resolver,&xxx_test_config()).expect("j");
        let regs = make_full_type(&defstore,MemberMode::In,&make_type(&defstore,"boolean")).expect("a");
        assert_eq!("*<0>/R",format_pvec(&regs));
        let regs = make_full_type(&defstore,MemberMode::In,&make_type(&defstore,"vec(offset_smoke::etest3)")).expect("b");
        assert_eq!(load_cmp("offset-smoke.out"),format_pvec(&regs));
    }

    #[test]
    fn test_cbor() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:codegen/offset-smoke").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        generate(&linker,&stmts,&defstore,&resolver,&xxx_test_config()).expect("j");
        let regs = make_full_type(&defstore,MemberMode::In,&make_type(&defstore,"vec(offset_smoke::etest3)")).expect("b");
        let named = regs.serialize(true).expect("cbor a");
        cbor_cmp(&named,"cbor-signature-named.out");
        let cr2 = FullType::deserialize(&named,true).expect("cbor d");
        assert_eq!(cr2,regs);
        let anon = regs.serialize(false).expect("cbor c");
        cbor_cmp(&anon,"cbor-signature-unnamed.out");
        let cr2 = FullType::deserialize(&anon,false).expect("cbor e");
        assert_ne!(cr2,regs);
        assert_eq!(MemberMode::In,cr2.get_mode());
        let vs_in = regs.iter().map(|x| x.1).cloned().collect::<Vec<_>>();
        let vs_out = cr2.iter().map(|x| x.1).cloned().collect::<Vec<_>>();
        assert_eq!(vs_in.len(),vs_out.len());
        for (v1,v2) in Iterator::zip(vs_in.iter(),vs_out.iter()) {
            assert_eq!(v1,v2);
        }
    }
}
