//! Utilities for encoding and decoding frames.

use std::{io, rc::Rc};

use ntex_bytes::{Bytes, BytesMut};

/// Trait of helper objects to write out messages as bytes.
pub trait Encoder {
    /// The type of items consumed by the `Encoder`
    type Item;

    /// The type of encoding errors.
    type Error: std::fmt::Debug;

    /// Encodes a frame into the buffer provided.
    fn encode(&self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error>;

    /// Encode with scatter-gather support. Body data that should be written
    /// without copying into `dst` can be pushed to `body_chunks`.
    /// The default delegates to `encode()`, ignoring `body_chunks`.
    fn encode_vectored(
        &self,
        item: Self::Item,
        dst: &mut BytesMut,
        body_chunks: &mut Vec<Bytes>,
    ) -> Result<(), Self::Error> {
        let _ = body_chunks;
        self.encode(item, dst)
    }
}

/// Decoding of frames via buffers.
pub trait Decoder {
    /// The type of decoded frames.
    type Item;

    /// The type of unrecoverable frame decoding errors.
    ///
    /// If an individual message is ill-formed but can be ignored without
    /// interfering with the processing of future messages, it may be more
    /// useful to report the failure as an `Item`.
    type Error: std::fmt::Debug;

    /// Attempts to decode a frame from the provided buffer of bytes.
    fn decode(&self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error>;
}

impl<T> Encoder for Rc<T>
where
    T: Encoder,
{
    type Item = T::Item;
    type Error = T::Error;

    fn encode(&self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        (**self).encode(item, dst)
    }

    fn encode_vectored(
        &self,
        item: Self::Item,
        dst: &mut BytesMut,
        body_chunks: &mut Vec<Bytes>,
    ) -> Result<(), Self::Error> {
        (**self).encode_vectored(item, dst, body_chunks)
    }
}

impl<T> Decoder for Rc<T>
where
    T: Decoder,
{
    type Item = T::Item;
    type Error = T::Error;

    fn decode(&self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        (**self).decode(src)
    }
}

/// Bytes codec.
///
/// Reads/Writes chunks of bytes from a stream.
#[derive(Debug, Copy, Clone)]
pub struct BytesCodec;

impl Encoder for BytesCodec {
    type Item = Bytes;
    type Error = io::Error;

    #[inline]
    fn encode(&self, item: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend_from_slice(&item[..]);
        Ok(())
    }
}

impl Decoder for BytesCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn decode(&self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            Ok(None)
        } else {
            Ok(Some(src.split_to(src.len())))
        }
    }
}
