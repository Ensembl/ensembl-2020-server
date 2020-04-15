fn hexdump_line(index: usize, data: &[u8]) -> String {
    let mut out = format!("{:08x}    ",index);
    for i in 0..16 {
        if i < data.len() {
            out.push_str(&format!("{:02x} ",data[i]));
        } else {
            out.push_str("   ");
        }
        if i == 8 {
            out.push_str("   ");
        }
    }
    out.push_str("   ");
    for i in 0..16 {
        if i < data.len() {
            if data[i] > 0x20 && data[i] < 0x7f {
                out.push(data[i] as char);
            } else {
                out.push_str(".");
            }
        } else {
            out.push_str(" ");
        }
    }
    out.push('\n');
    out
}

pub fn hexdump(data: &[u8]) -> String {
    let mut out = String::new();
    for start in (0..data.len()).step_by(16) {
        out.push_str(&hexdump_line(start,&data[start..(start+16).min(data.len())]));
    }
    out
}