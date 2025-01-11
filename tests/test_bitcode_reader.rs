use std::fs;

use llvm_bitcode::bitcode::{BitcodeElement, Payload, Record};
use llvm_bitcode::{BitStreamVisitor, Bitcode};

#[test]
fn test_bitcode() {
    let data = fs::read("tests/fixtures/serialized.dia").unwrap();
    let _bitcode = Bitcode::new(&data).unwrap();

    let data = fs::read("tests/fixtures/simple.bc").unwrap();
    let bitcode = Bitcode::new(&data).unwrap();
    let module_block = bitcode
        .elements
        .iter()
        .find(|ele| match ele {
            BitcodeElement::Record(_) => false,
            BitcodeElement::Block(block) => block.id == 8,
        })
        .unwrap();
    let target_triple_record = module_block
        .as_block()
        .unwrap()
        .elements
        .iter()
        .find(|ele| match ele {
            BitcodeElement::Record(record) => record.id == 2,
            BitcodeElement::Block(_) => false,
        })
        .unwrap()
        .as_record()
        .unwrap();
    let fields: Vec<u8> = target_triple_record
        .fields()
        .iter()
        .map(|x| *x as u8)
        .collect();
    let target_triple = std::str::from_utf8(&fields).unwrap();
    assert_eq!(target_triple, "x86_64-apple-macosx11.0.0");
}

#[test]
fn test_bitstream_reader() {
    struct LoggingVisitor(Vec<String>);

    impl BitStreamVisitor for LoggingVisitor {
        fn should_enter_block(&mut self, id: u64) -> bool {
            self.0.push(format!("entering block: {id}"));
            true
        }

        fn did_exit_block(&mut self, id: u64) {
            self.0.push(format!("exiting block: {id}"));
        }

        fn visit(&mut self, _block_id: u64, mut record: Record) {
            let payload = if let Some(payload) = record.take_payload() {
                match payload {
                    Payload::Array(ele) => format!("array({} elements)", ele.len()),
                    Payload::Blob(blob) => format!("blob({} bytes)", blob.len()),
                    Payload::Char6String(s) => s.to_string(),
                }
            } else {
                "none".to_string()
            };
            let id = record.id;
            self.0.push(format!(
                "Record (id: {}, fields: {:?}, payload: {}",
                id,
                record.fields(),
                payload
            ));
        }
    }

    let data = fs::read("tests/fixtures/serialized.dia").unwrap();
    let mut visitor = LoggingVisitor(Vec::new());
    Bitcode::read(&data, &mut visitor).unwrap();
    assert_eq!(
        visitor.0,
        vec![
            "entering block: 8",
            "Record (id: 1, fields: [1], payload: none",
            "exiting block: 8",
            "entering block: 9",
            "Record (id: 6, fields: [1, 0, 0, 100], payload: blob(100 bytes)",
            "Record (id: 2, fields: [3, 1, 53, 28, 0, 0, 0, 34], payload: blob(34 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 1, 53, 28, 0, 0, 0, 59], payload: blob(59 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 1, 113, 1, 0, 0, 0, 38], payload: blob(38 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 1, 113, 1, 0, 0, 0, 20], payload: blob(20 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 6, fields: [2, 0, 0, 98], payload: blob(98 bytes)",
            "Record (id: 2, fields: [3, 2, 21, 69, 0, 0, 0, 34], payload: blob(34 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 2, 21, 69, 0, 0, 0, 22], payload: blob(22 bytes)",
            "Record (id: 7, fields: [2, 21, 69, 0, 2, 21, 69, 0, 1], payload: blob(1 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 2, 21, 69, 0, 0, 0, 42], payload: blob(42 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 2, 21, 69, 0, 0, 0, 22], payload: blob(22 bytes)",
            "Record (id: 7, fields: [2, 21, 69, 0, 2, 21, 69, 0, 1], payload: blob(1 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 6, fields: [3, 0, 0, 84], payload: blob(84 bytes)",
            "Record (id: 2, fields: [3, 3, 38, 28, 0, 0, 0, 34], payload: blob(34 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 3, 38, 28, 0, 0, 0, 59], payload: blob(59 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 3, 66, 1, 0, 0, 0, 38], payload: blob(38 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 3, 66, 1, 0, 0, 0, 20], payload: blob(20 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 6, fields: [4, 0, 0, 93], payload: blob(93 bytes)",
            "Record (id: 2, fields: [3, 4, 15, 46, 0, 0, 0, 40], payload: blob(40 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 4, 15, 46, 0, 0, 0, 22], payload: blob(22 bytes)",
            "Record (id: 7, fields: [4, 15, 46, 0, 4, 15, 46, 0, 1], payload: blob(1 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 4, 15, 46, 0, 0, 0, 42], payload: blob(42 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 2, fields: [3, 4, 15, 46, 0, 0, 0, 22], payload: blob(22 bytes)",
            "Record (id: 7, fields: [4, 15, 46, 0, 4, 15, 46, 0, 1], payload: blob(1 bytes)",
            "exiting block: 9",
            "entering block: 9",
            "Record (id: 6, fields: [5, 0, 0, 72], payload: blob(72 bytes)",
            "Record (id: 2, fields: [3, 5, 34, 13, 0, 0, 0, 44], payload: blob(44 bytes)",
            "Record (id: 3, fields: [5, 34, 13, 0, 5, 34, 26, 0], payload: none",
            "exiting block: 9",
        ]
    );
}

#[test]
fn test_block_skip() {
    struct No15(usize);

    impl BitStreamVisitor for No15 {
        fn should_enter_block(&mut self, id: u64) -> bool {
            id != 15
        }

        fn did_exit_block(&mut self, id: u64) {
            assert_ne!(15, id);
        }

        fn visit(&mut self, block_id: u64, _: Record) {
            assert_ne!(15, block_id);
            self.0 += 1;
        }
    }

    let data = fs::read("tests/fixtures/llvm19.bc").unwrap();
    let mut test = No15(0);
    Bitcode::read(&data, &mut test).unwrap();
    assert_eq!(179, test.0);
}
