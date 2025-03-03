use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};
use std::io::Result;

/// A wrapper that implements AsyncRead and replaces every '\n' with '\r\n'
pub struct NewlineReplacer<T> 
where
    T: AsyncRead + Unpin,
{
    inner: T,
    buffer: Vec<u8>,
    position: usize,
}

impl<T> NewlineReplacer<T> 
where
    T: AsyncRead + Unpin,
{
    /// Create a new NewlineReplacer wrapping any AsyncRead type
    pub fn new(reader: T) -> Self {
        NewlineReplacer {
            inner: reader,
            buffer: Vec::new(),
            position: 0,
        }
    }
}

impl<T> AsyncRead for NewlineReplacer<T>
where
    T: AsyncRead + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        let this = &mut *self;
        
        // If we've processed all buffered data, read more from the inner reader
        if this.position >= this.buffer.len() {
            this.buffer.clear();
            this.position = 0;
            
            // Create a temporary buffer to read into
            let mut temp_buf = vec![0; buf.remaining()];
            let mut read_buf = ReadBuf::new(&mut temp_buf);
            
            // Poll the inner reader
            match Pin::new(&mut this.inner).poll_read(cx, &mut read_buf) {
                Poll::Ready(Ok(())) => {
                    let filled_len = read_buf.filled().len();
                    if filled_len == 0 {
                        // EOF reached
                        return Poll::Ready(Ok(()));
                    }
                    
                    // Process the read data to replace newlines
                    let read_data = read_buf.filled();
                    
                    // Pre-allocate space in the buffer for worst case (all '\n' that need to be replaced with '\r\n')
                    this.buffer.reserve(read_data.len() * 2);
                    
                    for &byte in read_data {
                        if byte == b'\n' {
                            // Add '\r\n' instead of just '\n'
                            this.buffer.push(b'\r');
                            this.buffer.push(b'\n');
                        } else {
                            this.buffer.push(byte);
                        }
                    }
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        
        // Copy data from our buffer to the output buffer
        let bytes_to_copy = std::cmp::min(buf.remaining(), this.buffer.len() - this.position);
        let end_pos = this.position + bytes_to_copy;
        
        buf.put_slice(&this.buffer[this.position..end_pos]);
        this.position = end_pos;
        
        Poll::Ready(Ok(()))
    }
}
