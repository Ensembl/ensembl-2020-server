use crate::interp::RegisterFile;
use super::stream::{ Stream, StreamContents };

pub struct InterpContext {
    registers: RegisterFile,
    stream: Stream
}

impl InterpContext {
    pub fn new() -> InterpContext {
        InterpContext {
            registers: RegisterFile::new(),
            stream: Stream::new()
        }
    }

    pub fn registers(&mut self) -> &mut RegisterFile { &mut self.registers }
    pub fn stream_add(&mut self, contents: StreamContents) { self.stream.add(contents); }
    pub fn stream_take(&mut self) -> Vec<StreamContents> { self.stream.take() }
}
