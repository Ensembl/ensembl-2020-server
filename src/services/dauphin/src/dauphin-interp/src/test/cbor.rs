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

use super::files::load_testdata;
use serde_cbor::Value as CborValue;

fn hexdump_line(index: usize, data: &[u8]) -> String {
    let mut out = format!("{:08x}    ",index);
    for i in 0..16 {
        if i < data.len() {
            out.push_str(&format!("{:02x} ",data[i]));
        } else {
            out.push_str("   ");
        }
        if i == 8 {
            out.push_str(" ");
        }
    }
    out.push_str(" ");
    for i in 0..16 {
        if i < data.len() {
            if data[i] > 0x20 && data[i] < 0x7f {
                out.push(data[i] as char);
            } else {
                out.push_str(".");
            }
        } else {
            out.push_str(" ");
        }
    }
    out.push('\n');
    out
}

pub fn hexdump(data: &[u8]) -> String {
    let mut out = String::new();
    for start in (0..data.len()).step_by(16) {
        out.push_str(&hexdump_line(start,&data[start..(start+16).min(data.len())]));
    }
    out
}

pub fn cbor_cmp(cbor: &CborValue, filepath: &str) {
    let cmp = load_testdata(&["interp",filepath]).expect("cmp");
    let mut buffer = Vec::new();
    serde_cbor::to_writer(&mut buffer,&cbor).expect("cbor b");
    let gen = hexdump(&buffer);
    print!("{}\n",gen);
    assert_eq!(cmp,gen);
}
