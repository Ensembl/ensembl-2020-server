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

pub const PREAMBLE: &str = r#"

func __star__() becomes _;
func __sqopen__() becomes _;
func __dot__() becomes _;
func __query__() becomes _;
func __pling__() becomes _;
func __ref__() becomes _;
func __sqctor__() becomes _;
inline "*" __star__ prefix 8;
inline "[" __sqopen__ suffix 4;
inline "[" __sqctor__ prefix 4;
inline "." __dot__ suffix 4;
inline "?" __query__ suffix 4;
inline "!" __pling__ suffix 4;
inline "&[" __ref__ suffix 4;

"#;