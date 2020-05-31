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

pub const STD: &str = r#"

module "std";

proc assign(out _A, _A);
inline ":=" assign left 14;

func eq(_A,_A) becomes boolean;
inline "==" eq left 5;

func gt(number,number) becomes boolean;
inline ">" gt left 6;

func lt(number,number) becomes boolean;
inline "<" lt left 6;

func plus(number,number) becomes number;
inline "+" plus left 4;

proc incr(lvalue number, number);
inline "(+=)" incr left 14;

func len(vec(_)) becomes number;
proc print_vec(_);

proc assert(boolean,boolean);

proc print_regs(_A);

func extend(_A,_A) becomes _A;
inline "(+)" extend left 14;

"#;
