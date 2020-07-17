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

use std::cell::{ RefCell, Ref };
use std::collections::HashMap;
use std::rc::Rc;
use crate::command::Identifier;
use crate::types::{ FullType, ComplexPath, VectorRegisters };

#[derive(Debug)]
pub enum XStructure<T> {
    Simple(Rc<RefCell<T>>),
    Vector(Rc<XStructure<T>>),
    Struct(Identifier,HashMap<String,Rc<XStructure<T>>>),
    Enum(Identifier,Vec<String>,HashMap<String,Rc<XStructure<T>>>,Rc<RefCell<T>>),
}

impl<T> Clone for XStructure<T> {
    fn clone(&self) -> Self {
        match self {
            XStructure::Simple(t) => XStructure::Simple(t.clone()),
            XStructure::Vector(v) => XStructure::Vector(v.clone()),
            XStructure::Struct(id,map) => XStructure::Struct(id.clone(),map.clone()),
            XStructure::Enum(id,order,map,disc) => XStructure::Enum(id.clone(),order.clone(),map.clone(),disc.clone())
        }
    }
}


impl<T> XStructure<T> {
    pub fn derive<F,U,V>(&self, cb: &mut F) -> Result<XStructure<U>,V> where F: FnMut(&T) -> Result<U,V> {
        Ok(match self {
            XStructure::Simple(t) => XStructure::Simple(Rc::new(RefCell::new(cb(&t.borrow())?))),
            XStructure::Vector(v) => XStructure::Vector(Rc::new(v.derive(cb)?)),
            XStructure::Struct(id,map) => {
                let map : Result<HashMap<_,_>,_> = map.iter().map(|(k,v)| Ok((k.to_string(),Rc::new(v.derive(cb)?)))).collect();
                XStructure::Struct(id.clone(),map?)
            },
            XStructure::Enum(id,order,map,disc) => {
                let map : Result<HashMap<_,_>,_> = map.iter().map(|(k,v)| Ok((k.to_string(),Rc::new(v.derive(cb)?)))).collect();
                XStructure::Enum(id.clone(),order.clone(),map?,Rc::new(RefCell::new(cb(&disc.borrow())?)))
            }
        })
    }

    pub fn any(&self) -> Ref<T> {
        match self {
            XStructure::Vector(inner) => inner.any(),
            XStructure::Struct(_,kvs) => kvs.iter().next().unwrap().1.any(),
            XStructure::Enum(_,_,kvs,_) => kvs.iter().next().unwrap().1.any(),
            XStructure::Simple(t) => t.borrow()
        }
    }    
}

#[derive(Debug)]
pub struct XPath<T>(Vec<XPathEl>,Rc<RefCell<T>>);

impl<T> Clone for XPath<T> {
    fn clone(&self) -> Self {
        XPath(self.0.clone(),self.1.clone())
    }
}

#[derive(Debug,Clone)]
pub enum XPathEl {
    Vector(),
    Part(Identifier,String)
}

fn to_xpath<T>(cp: &ComplexPath, vr: T) -> Result<XPath<T>,String> {
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
    Ok(XPath(out,Rc::new(RefCell::new(vr))))
}

fn enum_split<T>(paths: &[XPath<T>]) -> (Vec<XPath<T>>,Option<XPath<T>>) {
    let mut disc = None;
    let mut rest = vec![];
    for path in paths.iter() {
        if path.0.len() == 0 {
            disc = Some(path.clone());
        } else {
            rest.push(path.clone());
        }
    }
    (rest,disc)
}

fn convert<T>(paths: &[XPath<T>]) -> Result<XStructure<T>,String> {
    if paths.iter().filter(|x| x.0.len()!=0).count() == 0 {
        /* simple */
        return Ok(XStructure::Simple(paths[0].1.clone()));
    }
    let (paths,disc) = enum_split(paths);
    let mut paths : Vec<XPath<T>> = paths.iter().map(|x| x.clone()).collect();
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
            entries.insert(field.clone(),Rc::new(convert(members)?));
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

pub fn to_xstructure(sig: &FullType) -> Result<XStructure<VectorRegisters>,String> {
    let mut xpaths = vec![];
    for (cp,vr) in sig.iter() {
        xpaths.push(to_xpath(cp,vr)?);
    }
    Ok(convert(&xpaths)?.derive::<_,_,String>(&mut (|x| Ok((*x).clone())))?)
}
