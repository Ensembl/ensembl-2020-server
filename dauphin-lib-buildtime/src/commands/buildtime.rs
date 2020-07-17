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

use dauphin_interp::command::{ CommandSetId };
use dauphin_compile::command::CompLibRegister;
use super::defines::DefineCommandType;
use super::ini::LoadIniCommandType;
use super::dump::DumpSigCommandType;
use super::versions::VersionCommandType;
use super::hints::{ GetSizeHintCommandType, SetSizeHintCommandType, ForcePauseCommandType };

pub fn make_buildtime() -> Result<CompLibRegister,String> {
    let set_id = CommandSetId::new("buildtime",(0,1),0xB790000000000000);
    let mut set = CompLibRegister::new(&set_id,None);
    set.push("load_ini",None,LoadIniCommandType());
    set.push("dump_sig",None,DumpSigCommandType());
    set.push("get_size_hint",None,GetSizeHintCommandType());
    set.push("set_size_hint",None,SetSizeHintCommandType());
    set.push("force_pause",None,ForcePauseCommandType());
    set.push("is_defined",None,DefineCommandType(false));
    set.push("get_define",None,DefineCommandType(true));
    set.push("get_version",None,VersionCommandType());
    set.add_header("buildtime",include_str!("header.dp"));
    Ok(set)
}
