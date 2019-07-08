#[cfg(test)]
use crate::testsuite::load_testdata;

use super::charsource::{ CharSource, StringCharSource };
use super::preamble::PREAMBLE;

pub struct FileResolver {

}

impl FileResolver {
    pub fn new() -> FileResolver {
        FileResolver {}
    }

    #[cfg(test)]
    fn test_path(&self, path: &str) -> Result<Box<dyn CharSource>,String> {
        let paths : Vec<&str> = path.split("/").collect();
        let name = format!("test:{}",path);
        match load_testdata(&paths) {
            Ok(data) => Ok(Box::new(StringCharSource::new(&name,data))),
            Err(err) => Err(format!("Loading \"{}\": {}",path,err))
        }
    }

    #[cfg(not(test))]
    fn test_path<'a>(&self, _path: &'a str) -> Result<Box<dyn CharSource>,String> {
        Err("no test files except when running tests".to_string())
    }

    pub fn resolve(&self, path: &str) -> Result<Box<dyn CharSource>,String> {
        if path.starts_with("data:") {
            Ok(Box::new(StringCharSource::new(path,path[5..].to_string())))
        } else if path.starts_with("test:") {
            self.test_path(&path[5..])
        } else if path.starts_with("preamble:") {
            Ok(Box::new(StringCharSource::new(path,PREAMBLE.to_string())))
        } else {
            Err("protocol not supported".to_string())
        }
    }
}