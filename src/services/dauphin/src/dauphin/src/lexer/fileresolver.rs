use crate::testsuite::load_testdata;

use super::charsource::{ CharSource, StringCharSource };

pub struct FileResolver {

}

impl FileResolver {
    pub fn new() -> FileResolver {
        FileResolver {}
    }

    #[cfg(test)]
    fn test_path(&self, path: &str) -> Result<Box<dyn CharSource>,String> {
        let paths : Vec<&str> = path.split("/").collect();
        match load_testdata(&paths) {
            Ok(data) => Ok(Box::new(StringCharSource::new(data))),
            Err(err) => Err(format!("Loading \"{}\": {}",path,err))
        }
    }

    #[cfg(not(test))]
    fn test_path<'a>(&self, path: &'a str) -> Result<Box<CharSource>,String> {
        Err("no test files except when running tests")
    }

    pub fn resolve(&self, path: &str) -> Result<Box<CharSource>,String> {
        if path.starts_with("data:") {
            Ok(Box::new(StringCharSource::new(path[5..].to_string())))
        } else if path.starts_with("test:") {
            self.test_path(&path[5..])
        } else {
            Err("protocol not supported".to_string())
        }
    }
}