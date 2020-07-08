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

use crate::interp::{ CommandSet, CommandSetId };
use super::defines::DefineCommandType;
use super::ini::LoadIniCommandType;
use super::dump::DumpSigCommandType;
use super::versions::VersionCommandType;
use super::hints::{ GetSizeHintCommandType, SetSizeHintCommandType, ForcePauseCommandType };

pub fn make_buildtime() -> Result<CommandSet,String> {
    let set_id = CommandSetId::new("buildtime",(0,1),0xA2B92F8C219E382A);
    let mut set = CommandSet::new(&set_id,true);
    set.push("load_ini",1,LoadIniCommandType())?;
    set.push("dump_sig",2,DumpSigCommandType())?;
    set.push("get_size_hint",3,GetSizeHintCommandType())?;
    set.push("set_size_hint",4,SetSizeHintCommandType())?;
    set.push("force_pause",5,ForcePauseCommandType())?;
    set.push("is_defined",6,DefineCommandType(false))?;
    set.push("get_define",7,DefineCommandType(true))?;
    set.push("get_version",8,VersionCommandType())?;
    set.add_header("buildtime",include_str!("header.dp"));
    Ok(set)
}
