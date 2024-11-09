use crate::driving::error;
use embedded_io_async::{Read, ReadReady};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct TcpMessage<'a, T> {
    pub seq: u32,
    pub msg: BytesOrT<'a, T>,
}

pub enum BytesOrT<'a, T> {
    Bytes(&'a [u8]),
    T(T),
}

impl<T> From<T> for BytesOrT<'_, T> {
    fn from(value: T) -> Self {
        Self::T(value)
    }
}

#[derive(Debug)]
pub enum TcpError {
    EncodeError(bincode::error::EncodeError),
    DecodeError(bincode::error::DecodeError),
    Deserialization,
    InsufficientSpace,
    InvalidMessageSize,
    SocketError,
    WouldBlock,
    MessageMissing,
    Eof,
}

pub fn write_tcp<'a, T: Serialize>(
    seq: &mut u32,
    msg: impl Into<BytesOrT<'a, T>>,
    buf: &mut [u8],
) -> Result<usize, TcpError> {
    let mut msg_size = 4 + 4 + 1;
    if buf.len() < msg_size {
        return Err(TcpError::InsufficientSpace);
    }

    let msg = msg.into();

    let is_bytes = match msg {
        BytesOrT::T(t) => {
            msg_size += bincode::serde::encode_into_slice(
                t,
                &mut buf[msg_size..],
                bincode::config::standard(),
            )
            .map_err(TcpError::EncodeError)?;
            false
        }
        BytesOrT::Bytes(bytes) => {
            if msg_size + bytes.len() > buf.len() {
                return Err(TcpError::InsufficientSpace);
            }
            buf[msg_size..msg_size + bytes.len()].copy_from_slice(bytes);
            msg_size += bytes.len();
            true
        }
    };

    // message size
    buf[0..4].copy_from_slice(&(msg_size as u32).to_be_bytes());
    // seq number
    buf[4..8].copy_from_slice(&seq.to_be_bytes());
    *seq += 1;
    // bytes vs t info
    buf[8] = if is_bytes { 1 } else { 0 };

    Ok(msg_size)
}

pub struct StatefulTcpReader {
    buf: [u8; 5192],
    buf_len: usize,
    used_len: usize,
}

impl Default for StatefulTcpReader {
    fn default() -> Self {
        Self::new()
    }
}

impl StatefulTcpReader {
    pub fn new() -> Self {
        Self {
            buf: [0; 5192],
            buf_len: 0,
            used_len: 0,
        }
    }

    pub fn clear(&mut self) {
        self.buf_len = 0;
        self.used_len = 0;
    }

    pub async fn try_read_socket<'a, T: DeserializeOwned>(
        &'a mut self,
        socket: &mut (impl Read + ReadReady),
    ) -> Result<TcpMessage<'a, T>, TcpError> {
        if socket.read_ready().map_err(|_| TcpError::SocketError)? {
            self.buf_len += socket
                .read(&mut self.buf[self.buf_len..])
                .await
                .map_err(|_| TcpError::SocketError)?;
        }
        match self.fetch_message()? {
            Some(msg) => Ok(msg),
            _ => Err(TcpError::WouldBlock),
        }
    }

    pub async fn read_socket<'a, T: DeserializeOwned>(
        &'a mut self,
        socket: &mut impl Read,
    ) -> Result<TcpMessage<'a, T>, TcpError> {
        loop {
            if self.has_message() {
                break;
            }
            let new_len = socket
                .read(&mut self.buf[self.buf_len..])
                .await
                .map_err(|_| TcpError::SocketError)?;
            if new_len == 0 {
                return Err(TcpError::Eof);
            }
            self.buf_len += new_len;
        }
        self.fetch_message()?.ok_or(TcpError::MessageMissing)
    }

    #[cfg(feature = "std")]
    pub fn read_u8_ref<'a, T: DeserializeOwned>(
        &'a mut self,
        buf: &mut &[u8],
    ) -> Result<TcpMessage<'a, T>, TcpError> {
        self.buf_len += std::io::Read::read(buf, &mut self.buf[self.buf_len..])
            .map_err(|_| TcpError::SocketError)?;
        self.fetch_message()?.ok_or(TcpError::WouldBlock)
    }

    fn clear_used(&mut self) {
        if self.used_len > 0 {
            self.buf.copy_within(self.used_len..self.buf_len, 0);
            self.buf_len -= self.used_len;
            self.used_len = 0;
        }
    }

    fn has_message(&mut self) -> bool {
        self.clear_used();
        // do we have a message?
        if self.buf_len >= 9 {
            // do we have the whole message?
            let msg_size = u32::from_be_bytes(self.buf[0..4].try_into().unwrap()) as usize;
            if self.buf_len >= msg_size {
                // we do have the whole message
                return true;
            }
        }
        false
    }

    fn fetch_message<T: DeserializeOwned>(&mut self) -> Result<Option<TcpMessage<T>>, TcpError> {
        self.clear_used();
        // do we have a message?
        if self.buf_len >= 9 {
            // do we have the whole message?
            let msg_size = u32::from_be_bytes(self.buf[0..4].try_into().unwrap()) as usize;
            if msg_size < 9 {
                error!("Received invalid message size {}", msg_size);
                // this is impossible... reset internal buffers in hopes of recovery
                self.used_len = 0;
                self.buf_len = 0;
                return Err(TcpError::InvalidMessageSize);
            }
            if self.buf_len >= msg_size {
                // we do have the whole message
                // we can't remove the bytes from the buffer because we might have to return
                // a reference to them. Instead, mark these bytes as used
                self.used_len = msg_size;
                let seq = u32::from_be_bytes(self.buf[4..8].try_into().unwrap());
                return Ok(Some(TcpMessage {
                    seq,
                    msg: match self.buf[8] {
                        0 => BytesOrT::T(
                            bincode::serde::decode_from_slice(
                                &self.buf[9..msg_size],
                                bincode::config::standard(),
                            )
                            .map_err(TcpError::DecodeError)?
                            .0,
                        ),
                        _ => BytesOrT::Bytes(&self.buf[9..msg_size]),
                    },
                }));
            }
        }
        Ok(None)
    }
}
