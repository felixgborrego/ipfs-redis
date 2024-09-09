use super::Data;
use crate::error::Result;
use std::io::{BufWriter, Write};

impl Data {
    pub fn write_resp<T: Write>(&self, writer: &mut BufWriter<T>) -> Result<()> {
        match self {
            Data::SimpleString(value) => {
                write!(writer, "+{value}\r\n")?;
                writer.flush()?;
                Ok(())
            }
            Data::BulkString(value) => {
                write!(writer, "${}\r\n{value}\r\n", value.len())?;
                writer.flush()?;
                Ok(())
            }
            Data::Array(values) => {
                write!(writer, "*{}\r\n", values.len())?;
                values.iter().try_for_each(|item| item.write_resp(writer))?;
                writer.flush()?;
                Ok(())
            }
            Data::ConnectionClosed => Ok(()),
            Data::NullBuilkString => {
                write!(writer, "$-1\r\n")?;
                writer.flush()?;
                Ok(())
            }
            Data::FullResyncBinaryConent(response, data) => {
                response.write_resp(writer)?;
                writer.flush()?;
                write!(writer, "${}\r\n", data.len())?;
                writer.write_all(data)?;
                Ok(())
            }
        }
    }
}
