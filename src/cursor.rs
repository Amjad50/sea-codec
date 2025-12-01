use crate::codec::common::SeaError;
use alloc::borrow::Cow;

pub enum Cursor<'inp> {
    Slice(&'inp [u8]),
    #[cfg(feature = "std")]
    Reader(Box<dyn std::io::Read + 'inp>),
}
#[cfg(feature = "std")]
impl<'inp> Cursor<'inp> {
    pub(crate) fn from_reader<R: std::io::Read + 'inp>(reader: R) -> Self {
        Self::Reader(Box::new(reader))
    }
}

impl<'inp> Cursor<'inp> {
    pub(crate) fn from_slice(data: &'inp [u8]) -> Self {
        Self::Slice(data)
    }

    pub fn read_exact(&mut self, result: &mut [u8]) -> Result<(), SeaError> {
        match self {
            Cursor::Slice(data) => {
                if data.len() < result.len() {
                    Err(SeaError::EndOfFile)
                } else {
                    let (r, remaining) = data.split_at(result.len());
                    *data = remaining;
                    result.copy_from_slice(r);
                    Ok(())
                }
            }
            #[cfg(feature = "std")]
            Cursor::Reader(reader) => Ok(reader.read_exact(result)?),
        }
    }

    pub fn read_max_or_zero(&mut self, at_least_bytes: usize) -> Result<Cow<'inp, [u8]>, SeaError> {
        match self {
            Cursor::Slice(data) => {
                // Determine how much we can actually read
                let len = data.len().min(at_least_bytes);

                // If empty, return empty Cow
                if len == 0 {
                    return Ok(Cow::Borrowed(&[]));
                }

                // Split the data: 'chunk' is what we return, 'remaining' is what stays in the Cursor
                let (chunk, remaining) = data.split_at(len);

                // Advance the cursor state
                *data = remaining;

                // Return the reference (Zero Allocation)
                Ok(Cow::Borrowed(chunk))
            }
            #[cfg(feature = "std")]
            Cursor::Reader(reader) => {
                // Pre-allocate buffer
                let mut buffer = vec![0u8; at_least_bytes];
                let mut total_bytes_read = 0;

                // Loop until buffer is full or EOF
                while total_bytes_read < at_least_bytes {
                    let bytes_read = reader.read(&mut buffer[total_bytes_read..])?;

                    if bytes_read == 0 {
                        break;
                    }

                    total_bytes_read += bytes_read;
                }

                if total_bytes_read == 0 {
                    return Ok(Cow::Owned(Vec::new()));
                }

                buffer.truncate(total_bytes_read);

                Ok(Cow::Owned(buffer))
            }
        }
    }
}
