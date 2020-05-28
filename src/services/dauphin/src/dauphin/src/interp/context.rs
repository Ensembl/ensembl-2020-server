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

use crate::interp::RegisterFile;
use super::stream::{ Stream, StreamContents };

pub struct InterpContext {
    registers: RegisterFile,
    stream: Stream,
    filename: String,
    line_number: u32
}

impl InterpContext {
    pub fn new() -> InterpContext {
        InterpContext {
            registers: RegisterFile::new(),
            stream: Stream::new(),
            filename: "**anon**".to_string(),
            line_number: 0
        }
    }

    pub fn registers(&mut self) -> &mut RegisterFile { &mut self.registers }
    pub fn stream_add(&mut self, contents: StreamContents) { self.stream.add(contents); }
    pub fn stream_take(&mut self) -> Vec<StreamContents> { self.stream.take() }

    pub fn set_line_number(&mut self, filename: &str, line_number: u32) {
        self.filename = filename.to_string();
        self.line_number = line_number;
    }

    pub fn get_line_number(&self) -> (&str,u32) {
        (&self.filename,self.line_number)
    }
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::resolver::test_resolver;
    use crate::parser::{ Parser };
    use crate::generate::{ generate2 };
    use crate::interp::{ mini_interp, xxx_compiler_link };

    #[test]
    fn line_number_smoke() {
        let resolver = test_resolver();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:std/line-number.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let linker = xxx_compiler_link().expect("y");
        let mut context = generate2("",&linker,&stmts,&defstore,true).expect("j");
        let message = mini_interp(&mut context,&linker).expect_err("x");
        print!("{}\n",message);
        assert!(message.ends_with("at test:std/line-number.dp:10"));
    }

    #[test]
    fn no_line_number_smoke() {
        let resolver = test_resolver();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:std/line-number.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let linker = xxx_compiler_link().expect("y");
        let mut context = generate2("",&linker,&stmts,&defstore,false).expect("j");
        let message = mini_interp(&mut context,&linker).expect_err("x");
        print!("{}\n",message);
        assert!(!message.contains(" at "));
    }
}
