use fc_hint::tt::{assemble, decode, disassemble};

#[test]
fn every_opcode_round_trips_without_changing_jump_offsets() {
    let mut bytes = Vec::new();
    for opcode in 0..=255u8 {
        bytes.push(opcode);
        match opcode {
            0x40 => bytes.extend([3, 0, 128, 255]),
            0x41 => bytes.extend([3, 0x80, 0, 0, 0, 0x7f, 0xff]),
            0xb0..=0xb7 => bytes.extend(vec![255; usize::from(opcode & 7) + 1]),
            0xb8..=0xbf => {
                for _ in 0..=opcode & 7 {
                    bytes.extend([0xff, 0xf1]);
                }
            }
            _ => {}
        }
    }
    let source = disassemble(&bytes).unwrap();
    assert_eq!(assemble(&source).unwrap(), bytes);
    let decoded = decode(&bytes).unwrap();
    assert_eq!(decoded.len(), 256);
    assert!(decoded
        .windows(2)
        .all(|pair| pair[0].offset + pair[0].length == pair[1].offset));
}

#[test]
fn malformed_pushes_fail_without_looping_or_accepting_partial_programs() {
    for bytes in [vec![0x40], vec![0x41, 1, 0], vec![0xb1, 9], vec![0xb8, 0]] {
        assert!(decode(&bytes).is_err());
    }
    for source in [
        "NPUSHB -1",
        "NPUSHW 32768",
        "PUSHB[1] 1",
        "SVTCA[0] 4",
        "UNKNOWN",
        "POP\nNPUSHB 256",
    ] {
        assert!(assemble(source).is_err(), "{source}");
    }
    assert_eq!(
        assemble("# comment\n NPUSHB 1 255 ; values\nPOP").unwrap(),
        [0x40, 2, 1, 255, 0x21]
    );
    assert_eq!(
        assemble("NPUSHW -32768 32767").unwrap(),
        [0x41, 2, 128, 0, 127, 255]
    );
    assert_eq!(disassemble(&[0x40, 0, 0x41, 0]).unwrap(), "NPUSHB\nNPUSHW");
    assert!(decode(&vec![0; 65_536]).is_err());
    assert!(assemble(&"POP\n".repeat(65_536)).is_err());
}
