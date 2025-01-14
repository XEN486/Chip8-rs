pub fn encode_rle(data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();
    let mut count: u32 = 0;

    for &byte in data {
        if byte == 0x00 {
            count += 1;
        } else {
            if count > 0 {
                encoded.push(0x00);
                encoded.extend(&count.to_le_bytes());
                count = 0;
            }
            encoded.push(byte);
        }
    }

    if count > 0 {
        encoded.push(0x00);
        encoded.extend(&count.to_le_bytes());
    }

    encoded
}

pub fn decode_rle(data: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::new();
    let mut idx = 0;

    while idx < data.len() {
        if data[idx] == 0x00 {
            if idx + 4 <= data.len() {
                let length = u32::from_le_bytes([data[idx + 1], data[idx + 2], data[idx + 3], data[idx + 4]]) as usize;
                decoded.resize(decoded.len() + length, 0x00);
                idx += 5;
            } else {
                continue;
            }
        } else {
            decoded.push(data[idx]);
            idx += 1;
        }
    }

    decoded
}

pub fn encode_rle_u32(data: &[u32]) -> Vec<u8> {
    let mut encoded = Vec::new();
    let mut count: u32 = 0;

    for &value in data {
        if value == 0 {
            count += 1;
        } else {
            if count > 0 {
                encoded.push(0); // Marker for RLE-encoded zeros
                encoded.extend(&count.to_le_bytes());
                count = 0;
            }
            encoded.extend(&value.to_le_bytes()); // Directly encode non-zero value
        }
    }

    if count > 0 {
        encoded.push(0); // Marker for trailing zeros
        encoded.extend(&count.to_le_bytes());
    }

    encoded
}

pub fn decode_rle_u32(data: &[u8]) -> Vec<u32> {
    let mut decoded = Vec::new();
    let mut idx = 0;

    while idx < data.len() {
        if data[idx] == 0 {
            if idx + 4 < data.len() {
                let length = u32::from_le_bytes([data[idx + 1], data[idx + 2], data[idx + 3], data[idx + 4]]);
                decoded.resize(decoded.len() + length as usize, 0);
                idx += 5;
            } else {
                break; // Handle malformed input gracefully
            }
        } else if idx + 4 <= data.len() {
            let value = u32::from_le_bytes([data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]);
            decoded.push(value);
            idx += 4;
        } else {
            break; // Handle malformed input gracefully
        }
    }

    decoded
}