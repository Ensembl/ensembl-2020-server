use super::value::InterpValueData;

pub enum StreamContents {
    String(String),
    Data(InterpValueData),
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