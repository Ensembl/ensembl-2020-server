use crate::interp::InterpValue;

pub enum StreamContents {
    String(String),
    Data(InterpValue),
}

pub struct Stream {
    contents: Vec<StreamContents>
}

impl Stream {
    pub fn new() -> Stream {
        Stream {
            contents: Vec::new()
        }
    }

    pub fn add(&mut self, contents: StreamContents) {
        self.contents.push(contents);
    }

    pub fn take(&mut self) -> Vec<StreamContents> {
        self.contents.drain(..).collect()
    }
}