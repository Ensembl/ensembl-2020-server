/* 
 *  This is the default license template.
 *  
 *  File: files.rs
 *  Author: dan
 *  Copyright (c) 2020 dan
 *  
 *  To edit this license information: Press Ctrl+Shift+P and press 'Create new License Template...'.
 */

use std::fs::read_to_string;
use std::path::PathBuf;

pub fn find_testdata() -> PathBuf {
    let mut dir = std::env::current_exe().expect("cannot get current exec path");
    while dir.pop() {
        let mut testdata = PathBuf::from(&dir);
        testdata.push("testdata");
        if testdata.exists() {
            return testdata;
        }
    }
    panic!("cannot find testdata directory");
}

pub fn load_testdata(tail: &[&str]) -> Result<String,String> {
    let mut path = find_testdata();
    for t in tail {
        path.push(t);
    }
    read_to_string(path).map_err(|x| x.to_string())
}