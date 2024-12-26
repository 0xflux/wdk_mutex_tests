use alloc::vec::Vec;

pub trait ToU16Vec {
    fn to_u16_vec(&self) -> Vec<u16>;
}

impl ToU16Vec for &str {
    fn to_u16_vec(&self) -> Vec<u16> {
        // reserve space for null terminator
        let mut buf = Vec::with_capacity(self.len() + 1);

        // iterate over each char and push the UTF-16 to the buf
        for c in self.chars() {
            let mut c_buf = [0; 2];
            let encoded = c.encode_utf16(&mut c_buf);
            buf.extend_from_slice(encoded);
        }

        buf.push(0); // add null terminator
        buf
    }
}