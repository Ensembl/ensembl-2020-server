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

use std::fmt::Display;
use std::fs::read;
use std::process::exit;
use serde_cbor;
use dauphin_interp::make_core_interp;
use dauphin_interp::command::{ CommandInterpretSuite, InterpreterLink };
use dauphin_interp::runtime::{ StandardInterpretInstance, InterpretInstance };
use dauphin_lib_std::{ make_std_interp, StreamFactory };

fn bomb<A,E,T>(action: T, x: Result<A,E>) -> A where T: Fn() -> String, E: Display {
    match x {
        Ok(v) => v,
        Err(e) => {
            eprint!("{} Error {}\n",action(),e.to_string());
            exit(2);
        }
    }
}

fn read_binary_file(filename: &str) -> Vec<u8> {
    bomb(
        || format!("Reading {}",filename),
        read(filename)
    )
}

fn make_suite() -> Result<CommandInterpretSuite,String> {
    let mut suite = CommandInterpretSuite::new();
    suite.register(make_core_interp()?)?;
    suite.register(make_std_interp()?)?;
    Ok(suite)     
}

fn interpreter<'a>(linker: &'a InterpreterLink, name: &str) -> Result<Box<dyn InterpretInstance<'a> + 'a>,String> {
    Ok(Box::new(StandardInterpretInstance::new(linker,name)?))
}

fn main() {
    let binary_file = String::new();
    let name = String::new();

    let suite = bomb(|| format!("could not construct library"),
        make_suite()
    );
    let buffer = read_binary_file(&binary_file);
    let program = bomb(|| format!("corrupted cbor in {}",binary_file),
                    serde_cbor::from_slice(&buffer).map_err(|x| format!("{} while deserialising",x)));
    let mut interpret_linker = bomb(|| format!("could not link binary"),
        InterpreterLink::new(suite,&program)
    );
    let mut sf = StreamFactory::new();
    sf.to_stdout(true);
    interpret_linker.add_payload("std","stream",sf);
    let mut interp = bomb(|| format!(""),
        interpreter(&interpret_linker,&name)
    );
    while interp.more().expect("interpreting") {}
    interp.finish();    
}
