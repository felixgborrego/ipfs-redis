use crate::error::{Error, Result};
use std::io::{self, BufRead, BufReader, Read};

use super::Cmd;
use super::Data;

impl Data {
    /// Request commands are either an array (start with *) or an inline command
    pub fn parse_cmd<T: Read>(stream: &mut BufReader<T>) -> Result<Cmd> {
        let mut first_byte = [0; 1];
        if let Err(error) = stream.read_exact(&mut first_byte) {
            if matches!(error.kind(), io::ErrorKind::UnexpectedEof) {
                return Ok(Cmd::ConnectionClosed);
            }
        }

        let Some(first_character) = first_byte.first() else {
            return Err(Error::InvalidResp);
        };

        let span = tracing::debug_span!("parse_cmd", first_character = %*first_character as char);

        span.in_scope(|| {
            match first_character {
                b'*' => {
                    let Data::Array(mut array_data) = parse_array(stream)? else {
                        return Err(Error::InvalidResp);
                    };
                    tracing::debug!("Array data: {array_data:?}");
                    to_cmd(&mut array_data)
                }
                other => {
                    tracing::debug!("unsupported data {other:?}");
                    // TODO implement inline commands, for now we consider it to be a PING
                    Ok(Cmd::Ping)
                }
            }
        })
    }

    pub fn parse<T: Read>(stream: &mut BufReader<T>) -> Result<Data> {
        // Read first byte
        let mut first_byte = [0; 1];
        stream.read_exact(&mut first_byte)?;

        let Some(first_byte) = first_byte.first() else {
            return Err(Error::InvalidResp);
        };

        tracing::info_span!("parse", first_byte=%*first_byte as char).in_scope(
            || match first_byte {
                b'+' => Ok(parse_simple_string(stream)),
                b'*' => parse_array(stream),
                b'$' => parse_bulk_string(stream),
                _ => Err(Error::InvalidResp),
            },
        )
    }
}

fn to_cmd(args: &mut Vec<Data>) -> Result<Cmd> {
    let Data::BulkString(cmd) = args.remove(0) else {
        return Err(Error::InvalidResp);
    };

    let args = std::mem::take(args);

    Cmd::from_str_args(&cmd, args)
}

/// RESP arrays are encoded follow:
/// *<number-of-elements>\r\n<element-1>...<element-n>
fn parse_array<T: Read>(stream: &mut BufReader<T>) -> Result<Data> {
    let line = read_str_line(stream);
    let num_elements: usize = line.parse()?;

    let mut array: Vec<Data> = Vec::with_capacity(num_elements);

    for _ in 0..num_elements {
        array.push(Data::parse(stream)?);
    }

    Ok(Data::Array(array))
}

/// Read a line expluding the end caracters \r\n
fn read_str_line<T: Read>(stream: &mut BufReader<T>) -> String {
    let mut line = String::new();
    let _ = stream.read_line(&mut line);
    line[..(line.len().saturating_sub(2))].to_owned()
}

/// Resp Bulk String are encoded as follow:
/// $<length>\r\n<data>\r\n
/// examles:
///  - hello: $5\r\nhello\r\n
///  - '':    $0\r\n\r\n
fn parse_bulk_string<T: Read>(stream: &mut BufReader<T>) -> Result<Data> {
    let size_line = read_str_line(stream);
    let size: usize = size_line.parse()?;

    let data_line = read_str_line(stream);
    tracing::info!("build_string[size:{size}, data: {data_line}]");
    Ok(Data::BulkString(data_line[..size].to_string()))
}

fn parse_simple_string<T: Read>(stream: &mut BufReader<T>) -> Data {
    let line: String = read_str_line(stream);
    Data::SimpleString(line)
}

#[cfg(test)]
mod tests {
    use crate::protocol::Data;
    use std::io::{BufReader, Cursor};

    fn build_reader(resp_txt: &str) -> BufReader<Cursor<&[u8]>> {
        let data = resp_txt.as_bytes();
        let cursor = std::io::Cursor::new(data);
        BufReader::new(cursor)
    }

    #[test]
    fn bulk_decode_test() {
        let result: Data = Data::parse(&mut build_reader("$5\r\nhello\r\n")).unwrap();
        assert_eq!(result, Data::BulkString("hello".to_string()));
    }

    #[test]
    fn test_array() {
        let result =
            Data::parse(&mut build_reader("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")).unwrap();

        let expected = Data::Array(vec![
            Data::BulkString("hello".to_string()),
            Data::BulkString("world".to_string()),
        ]);
        assert_eq!(result, expected);
    }
}
