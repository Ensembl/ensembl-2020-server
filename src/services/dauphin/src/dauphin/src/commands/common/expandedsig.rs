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

use std::collections::HashMap;
use std::rc::Rc;
use crate::model::{ ComplexRegisters, ComplexPath, VectorRegisters, Identifier, Register };
use crate::interp::RegisterFile;
use crate::typeinf::BaseType;

#[derive(Debug,Clone)]
pub enum XStructure {
    Simple(VectorRegisters),
    Vector(Rc<XStructure>),
    Struct(Identifier,HashMap<String,XStructure>),
    Enum(Identifier,Vec<String>,HashMap<String,XStructure>,VectorRegisters),
}

#[derive(Debug,Clone)]
pub struct XPath(Vec<XPathEl>,VectorRegisters);

#[derive(Debug,Clone)]
pub enum XPathEl {
    Vector(),
    Part(Identifier,String)
}

fn to_xpath(cp: &ComplexPath, vr: &VectorRegisters) -> Result<XPath,String> {
    let mut out = vec![];
    let mut name = cp.get_name().ok_or_else(|| format!("cannot convert anon"))?.iter();
    let mut cursor = cp.get_breaks().iter().peekable();
    while let Some(vecs) = cursor.next() {
        for _ in 0..*vecs {
            out.push(XPathEl::Vector());
        }
        if cursor.peek().is_some() {
            let (obj,field) = name.next().ok_or_else(|| format!("bad path"))?;
            out.push(XPathEl::Part(obj.clone(),field.to_string()));
        }
    }
    Ok(XPath(out,vr.clone()))
}

fn enum_split(paths: &[XPath]) -> (Vec<XPath>,Option<XPath>) {
    let mut disc = None;
    let mut rest = vec![];
    for path in paths {
        if path.0.len() == 0 {
            disc = Some(path.clone());
        } else {
            rest.push(path.clone());
        }
    }
    (rest,disc)
}

fn convert(paths: &[XPath]) -> Result<XStructure,String> {
    if paths.iter().filter(|x| x.0.len()!=0).count() == 0 {
        /* simple */
        return Ok(XStructure::Simple(paths[0].1.clone()));
    }
    let (paths,disc) = enum_split(paths);
    let mut paths : Vec<XPath> = paths.iter().map(|x| x.clone()).collect();
    let heads : Vec<XPathEl> = paths.iter_mut().map(|x| x.0.remove(0)).collect();
    let names : Option<Vec<(Identifier,String)>> = heads.iter().map(|x| if let XPathEl::Part(x,y) = x { Some((x.clone(),y.clone())) } else { None } ).collect();
    if let Some(names) = names {
        let mut mapping = HashMap::new();
        let mut obj_name = None;
        let mut name_order = vec![];
        for (i,name) in names.iter().enumerate() {
            mapping.entry(name.1.clone()).or_insert(vec![]).push(paths[i].clone());
            name_order.push(name.1.clone());
            obj_name = Some(name.0.clone());
        }
        let obj_name = obj_name.as_ref().ok_or_else(|| "empty arm in sig".to_string())?.clone();
        let mut entries = HashMap::new();
        for (field,members) in mapping.iter() {
            entries.insert(field.clone(),convert(members)?);
        }
        if let Some(disc) = disc {
            Ok(XStructure::Enum(obj_name,name_order,entries,disc.1))
        } else {
            Ok(XStructure::Struct(obj_name,entries))
        }
    } else {
        Ok(XStructure::Vector(Rc::new(convert(&paths)?)))
    }
}

pub fn to_xstructure(sig: &ComplexRegisters) -> Result<XStructure,String> {
    let mut xpaths = vec![];
    for (cp,vr) in sig.iter() {
        xpaths.push(to_xpath(cp,vr)?);
    }
    Ok(convert(&xpaths)?)
}
