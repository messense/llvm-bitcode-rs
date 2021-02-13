use crate::bitcode::{BitcodeElement, Block, Record, Signature};
use crate::BitStreamReader;

/// A visitor which receives callbacks while reading a bitstream.
pub trait BitStreamVisitor {
    /// Validate a bitstream's signature or "magic number".
    fn validate(&self, _signature: Signature) {}
    /// Called when a new block is encountered. Return `true` to enter the block
    /// and read its contents, or `false` to skip it.
    fn should_enter_block(&mut self, id: u64) -> bool;
    /// Called when a block is exited.
    fn did_exit_block(&mut self);
    /// Called whenever a record is encountered.
    fn visit(&mut self, record: Record);
}

pub struct CollectingVisitor {
    stack: Vec<(u64, Vec<BitcodeElement>)>,
}

/// A basic visitor that collects all the blocks and records in a stream.
impl CollectingVisitor {
    pub fn new() -> Self {
        Self {
            stack: vec![(BitStreamReader::TOP_LEVEL_BLOCK_ID, Vec::new())],
        }
    }

    pub fn finalize_top_level_elements(mut self) -> Vec<BitcodeElement> {
        assert_eq!(self.stack.len(), 1);
        self.stack.pop().unwrap().1
    }
}

impl BitStreamVisitor for CollectingVisitor {
    fn should_enter_block(&mut self, id: u64) -> bool {
        self.stack.push((id, Vec::new()));
        true
    }

    fn did_exit_block(&mut self) {
        if let Some((id, elements)) = self.stack.pop() {
            let block = Block { id, elements };
            let last = self.stack.last_mut().unwrap();
            last.1.push(BitcodeElement::Block(block));
        }
    }

    fn visit(&mut self, record: Record) {
        let last = self.stack.last_mut().unwrap();
        last.1.push(BitcodeElement::Record(record));
    }
}
