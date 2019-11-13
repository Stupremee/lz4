#![no_std]
#![allow(dead_code)]

const LZ_MAX_OFFSET: u32 = 100000;

struct Compressor;

impl Compressor {
    pub fn new() -> Self {
        Compressor {}
    }

    pub fn compress<'a>(&self, data: &[u8]) -> &'a [u8] {
        if data.is_empty() {
            return &[];
        }
        let size = data.len();

        // Create histogram
        let mut histogram: [u8; 256] = [0; 256];
        for i in 0..size {
            histogram[data[i] as usize] += 1;
        }

        // Find the least common byte and use it as marker.
        let mut marker: u8 = 0;
        for i in 1..256 {
            if histogram[i] < histogram[marker as usize] {
                marker = i as u8;
            }
        }

        let inpos = 0;
        let outpos = 1;
        let mut bytes_left = size as u32;
        let mut max_offset: u32 = 0;
        let mut offset: u32 = 0;
        let mut best_offset: u32 = 0;
        let mut max_length: u32 = 0;
        let mut length: u32 = 0;
        let mut best_length: u32 = 0;

        loop {
            max_offset = if inpos > LZ_MAX_OFFSET {
                LZ_MAX_OFFSET
            } else {
                inpos
            };

            best_length = 3;
            best_offset = 0;
            offset = 3;

            loop {
                if offset <= max_offset {
                    break;
                }

                if data[inpos as usize] == data[(inpos - offset) as usize]
                    && data[(inpos + best_length) as usize]
                        == data[(inpos - offset + best_length) as usize]
                {
                    max_length = if (bytes_left as u32) < offset {
                        bytes_left
                    } else {
                        offset
                    };

                    length = self.compare_string(
                        &data[(inpos as usize)..],
                        &data[((inpos - offset) as usize)..],
                        0,
                        max_length,
                    );

                    if length > best_length {
                        best_length = length;
                        best_offset = offset;
                    }
                }

                offset += 1;
            }

            if best_length >= 8
                || best_length == 4) && (best_offset <= 0x0000007f)
                || (best_length == 5) && (best_offset <= 0x00003fff)
                || (best_length == 6) && (best_offset <= 0x001fffff)
                || (best_length == 7) && (best_offset <= 0x0fffffff)
            {
            }

            if bytes_left <= 3 {
                break;
            }
        }

        &[]
    }

    fn compare_string(&self, data_one: &[u8], data_two: &[u8], min_len: u32, max_len: u32) -> u32 {
        let mut len = min_len;
        loop {
            if len >= max_len || data_one[len as usize] != data_two[len as usize] {
                break;
            }
            len += 1;
        }
        len
    }
}
