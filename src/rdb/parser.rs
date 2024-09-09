use crate::error::{Error, Result};
use std::io::Read;

pub fn read_string<R>(reader: &mut R) -> Result<String>
where
    R: Read + ?Sized,
{
    let (size, is_string) = read_lenth_encoding(reader)?;
    let mut s: Vec<u8> = vec![0x0; size];
    reader.read_exact(&mut s)?;

    if is_string {
        let s = String::from_utf8(s)?;
        tracing::debug!("to read_string {size} bytes ({s})");
        Ok(s)
    } else if size == 1 {
        // It's a 8 bit integer
        Ok(s[0].to_string()) // It's an integers as String encoded
    } else if size == 4 {
        // It's a 32 bit integer
        let num = u32::from_le_bytes([s[0], s[1], s[2], s[3]]);
        Ok(num.to_string()) // It's an integers as String encoded
    } else {
        Err(Error::Unsupported(format!(
            "Unsupporte integer with size {size}"
        )))
    }
}

/// Read a size encoding
/// The first two bits of a size-encoded value indicate how the value should be parsed to evaluate the size.
/// as described here <https://rdb.fnordig.de/file_format.html#length-encoding>
pub fn read_lenth_encoding<R>(reader: &mut R) -> Result<(usize, bool)>
where
    R: Read + ?Sized,
{
    // The first to bits of the first byte tell use the size
    let mut byte: [u8; 1] = [0x0];
    reader.read_exact(&mut byte)?;

    // check mast
    // If the first two bits are 0b00:
    // The size is the remaining 6 bits of the byte.
    // In this example, the size is 10:
    // 0A
    // 00001010
    let bytes_to_read = byte[0];
    if bytes_to_read & 0b11_00_00_00 == 0 {
        Ok(((byte[0] as usize), true))

    // If the first two bits are 0b01:
    // The size is the next 14 bits
    // (remaining 6 bits in the first byte, combined with the next byte),
    // in big-endian (read left-to-right).
    //  In this example, the size is 700: */
    // 42 BC
    // 01000010 10111100
    } else if bytes_to_read & 0b11_00_00_00 == 0b01_00_00_00 {
        let remaining_6_bits = bytes_to_read & 0b00_11_11_11;
        tracing::debug!("bytes_to_read: {bytes_to_read:08b}");
        let remaining_6_bits = u16::from(remaining_6_bits) << 8;
        tracing::debug!("remaining_6_bits: {remaining_6_bits:016b}");

        reader.read_exact(&mut byte)?;
        let size = (remaining_6_bits) | u16::from(byte[0]);
        println!("size: {size:016b}");
        return Ok((size as usize, true));

        // If the first two bits are 0b10:
        // Ignore the remaining 6 bits of the first byte.
        // The size is the next 4 bytes, in big-endian (read left-to-right).
        // In this example, the size is 17000: */
        // 80 00 00 42 68
        // 10000000 00000000 00000000 01000010 01101000
    } else if bytes_to_read & 0b11_00_00_00 == 0b10_00_00_00 {
        let mut bytes: [u8; 4] = [0x0; 4];
        tracing::debug!("bytes: {bytes:?}");
        reader.read_exact(&mut bytes)?;

        Ok((u32::from_be_bytes(bytes) as usize, true))

        /* If the first two bits are 0b11:
        The remaining 6 bits specify a type of string encoding.
        See string encoding section. */
        // https://rdb.fnordig.de/file_format.html#string-encoding
        // First read the section Length Encoding, specifically the part when the first two bits are 11.
        // In this case, the remaining 6 bits are read.
        // If the value of those 6 bits is:
        // 0 indicates that an 8 bit integer follows
        // 1 indicates that a 16 bit integer follows
        // 2 indicates that a 32 bit integer follows
    } else if bytes_to_read & 0b11_00_00_00 == 0b11_00_00_00 {
        match bytes_to_read & 0b00_11_11_11 {
            0b00_00_00_00 => {
                // let remaining_6_bits = bytes_to_read & 0b00_00_11_11;
                // tracing::debug!("bytes_to_read: {bytes_to_read:08b} {bytes_to_read:x} ");
                // let remaining_6_bits = u16::from(remaining_6_bits) << 8;
                // tracing::debug!("remaining_6_bits: {remaining_6_bits:016b}");

                // reader.read_exact(&mut byte)?;
                // tracing::debug!("bytes_to_read: {:08b} {:x} ", byte[0], byte[0]);
                // let size = (remaining_6_bits) | u16::from(byte[0]);
                // println!("integer read: {size:016b} {size}");
                // return Ok((size as usize, false));
                Ok((1, false)) // 8 bit integer
            } // 0 indicates that an 8 bit integer follows
            0b00_00_00_01 => Ok((2, false)), // 16 bit integer
            0b00_00_00_10 => Ok((4, false)), //32 bit integer
            _ => Err(Error::InvalidResp),
        }
    } else {
        Err(Error::InvalidResp)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::io::{BufReader, Cursor};
//     #[test]
//     fn test_read_size() {
//         let mut data = to_reader(vec![
//             0x09, 0x72, 0x65, 0x64, 0x69, 0x73, 0x2D, 0x76, 0x65, 0x72,
//         ]);
//         let size = read_lenth_encoding(&mut data).unwrap();
//         assert_eq!(9, size, "expected size");

//         // size should be 700
//         let mut data = to_reader(vec![0x42, 0xBC]);
//         let size = read_lenth_encoding(&mut data).unwrap();
//         assert_eq!(700, size, "expected size");

//         // size should be 17000
//         let mut data = to_reader(vec![0x80, 0x00, 0x00, 0x42, 0x68]);
//         let size = read_lenth_encoding(&mut data).unwrap();
//         assert_eq!(17000, size, "expected size");
//     }

//     fn to_reader(data: Vec<u8>) -> BufReader<Cursor<Vec<u8>>> {
//         let cursor = Cursor::new(data);
//         BufReader::new(cursor)
//     }
// }
