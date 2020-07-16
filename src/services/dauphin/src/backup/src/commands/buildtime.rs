/* 
 *  This is the default license template.
 *  
 *  File: buildtime.rs
 *  Author: dan
 *  Copyright (c) 2020 dan
 *  
 *  To edit this license information: Press Ctrl+Shift+P and press 'Create new License Template...'.
 */

use dauphin_interp_common::common::{ CommandSetId };
use dauphin_compile::model::CompLibRegister;
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
