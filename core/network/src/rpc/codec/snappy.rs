use crate::rpc::methods::*;
use crate::rpc::{
    codec::base::OutboundCodec,
    protocol::{Encoding, Protocol, ProtocolId, RPCError, Version},
};
use crate::rpc::{RPCCodedResponse, RPCRequest, RPCResponse};
use libp2p::bytes::BytesMut;
use snap::read::FrameDecoder;
use snap::write::FrameEncoder;
use std::io::Cursor;
use std::io::ErrorKind;
use std::io::{Read, Write};
use tokio_util::codec::{Decoder, Encoder};
use unsigned_varint::codec::Uvi;

/* Inbound Codec */

pub struct SnappyInboundCodec {
    protocol: ProtocolId,
    inner: Uvi<usize>,
    len: Option<usize>,
    /// Maximum bytes that can be sent in one req/resp chunked responses.
    max_packet_size: usize,
}

impl SnappyInboundCodec {
    pub fn new(protocol: ProtocolId, max_packet_size: usize) -> Self {
        let uvi_codec = Uvi::default();
        // this encoding only applies to ssz_snappy.
        debug_assert_eq!(protocol.encoding, Encoding::Snappy);

        SnappyInboundCodec {
            inner: uvi_codec,
            protocol,
            len: None,
            max_packet_size,
        }
    }
}

// Encoder for inbound streams: Encodes RPC Responses sent to peers.
impl Encoder<RPCCodedResponse> for SnappyInboundCodec {
    type Error = RPCError;

    fn encode(&mut self, item: RPCCodedResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = match item {
            RPCCodedResponse::Success(resp) => match resp {
                RPCResponse::Status(res) => res,
                RPCResponse::BlocksByRange(res) => res,
                RPCResponse::BlocksByRoot(res) => res,
                RPCResponse::Pong(res) => res,
                RPCResponse::MetaData(res) => res,
            },
            RPCCodedResponse::InvalidRequest(err) => err.to_vec(),
            RPCCodedResponse::ServerError(err) => err.to_vec(),
            RPCCodedResponse::Unknown(err) => err.to_vec(),
            RPCCodedResponse::StreamTermination(_) => {
                unreachable!("Code error - attempting to encode a stream termination")
            }
        };
        //  encoded bytes should be within `max_packet_size`
        if bytes.len() > self.max_packet_size {
            return Err(RPCError::InternalError(
                "attempting to encode data > max_packet_size".into(),
            ));
        }
        // Inserts the length prefix of the uncompressed bytes into dst
        // encoded as a unsigned varint
        self.inner
            .encode(bytes.len(), dst)
            .map_err(RPCError::from)?;

        let mut writer = FrameEncoder::new(Vec::new());
        writer.write_all(&bytes).map_err(RPCError::from)?;
        writer.flush().map_err(RPCError::from)?;

        // Write compressed bytes to `dst`
        dst.extend_from_slice(writer.get_ref());
        Ok(())
    }
}

// Decoder for inbound streams: Decodes RPC requests from peers
impl Decoder for SnappyInboundCodec {
    type Item = RPCRequest;
    type Error = RPCError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.len.is_none() {
            // Decode the length of the uncompressed bytes from an unsigned varint
            match self.inner.decode(src).map_err(RPCError::from)? {
                Some(length) => {
                    self.len = Some(length);
                }
                None => return Ok(None), // need more bytes to decode length
            }
        };

        let length = self.len.expect("length should be Some");

        // Should not attempt to decode rpc chunks with length > max_packet_size
        if length > self.max_packet_size {
            return Err(RPCError::InvalidData);
        }
        let mut reader = FrameDecoder::new(Cursor::new(&src));
        let mut decoded_buffer = vec![0; length];

        match reader.read_exact(&mut decoded_buffer) {
            Ok(()) => {
                // `n` is how many bytes the reader read in the compressed stream
                let n = reader.get_ref().position();
                self.len = None;
                let _read_bytes = src.split_to(n as usize);
                match self.protocol.message_name {
                    Protocol::Status => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() > 0 {
                                Ok(Some(RPCRequest::Status(decoded_buffer)))
                            } else {
                                Err(RPCError::InvalidData)
                            }
                        }
                    },
                    Protocol::Goodbye => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() > 0 {
                                Ok(Some(RPCRequest::Goodbye(decoded_buffer)))
                            } else {
                                Err(RPCError::InvalidData)
                            }
                        }
                    },
                    Protocol::BlocksByRange => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() > 0 {
                                Ok(Some(RPCRequest::BlocksByRange(decoded_buffer)))
                            } else {
                                Err(RPCError::InvalidData)
                            }
                        }
                    },
                    Protocol::BlocksByRoot => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() >= 0 {
                                Ok(Some(RPCRequest::BlocksByRoot(decoded_buffer)))
                            } else {
                                Err(RPCError::InvalidData)
                            }
                        }
                    },
                    Protocol::Ping => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() > 0 {
                                Ok(Some(RPCRequest::Ping(decoded_buffer)))
                            } else {
                                Err(RPCError::InvalidData)
                            }
                        }
                    },
                    Protocol::MetaData => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() > 0 {
                                Err(RPCError::InvalidData)
                            } else {
                                Ok(Some(RPCRequest::MetaData))
                            }
                        }
                    },
                }
            }
            Err(e) => match e.kind() {
                // Haven't received enough bytes to decode yet
                // TODO: check if this is the only Error variant where we return `Ok(None)`
                ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                _ => return Err(e).map_err(RPCError::from),
            },
        }
    }
}

/* Outbound Codec: Codec for initiating RPC requests */
pub struct SnappyOutboundCodec {
    inner: Uvi<usize>,
    len: Option<usize>,
    protocol: ProtocolId,
    /// Maximum bytes that can be sent in one req/resp chunked responses.
    max_packet_size: usize,
}

impl SnappyOutboundCodec {
    pub fn new(protocol: ProtocolId, max_packet_size: usize) -> Self {
        let uvi_codec = Uvi::default();
        // this encoding only applies to ssz_snappy.
        debug_assert_eq!(protocol.encoding, Encoding::Snappy);

        SnappyOutboundCodec {
            inner: uvi_codec,
            protocol,
            max_packet_size,
            len: None,
        }
    }
}

// Encoder for outbound streams: Encodes RPC Requests to peers
impl Encoder<RPCRequest> for SnappyOutboundCodec {
    type Error = RPCError;

    fn encode(&mut self, item: RPCRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = match item {
            RPCRequest::Status(req) => req,
            RPCRequest::Goodbye(req) => req,
            RPCRequest::BlocksByRange(req) => req,
            RPCRequest::BlocksByRoot(req) => req,
            RPCRequest::Ping(req) => req,
            RPCRequest::MetaData => return Ok(()), // no metadata to encode
        };
        //  encoded bytes should be within `max_packet_size`
        if bytes.len() > self.max_packet_size {
            return Err(RPCError::InternalError(
                "attempting to encode data > max_packet_size",
            ));
        }

        // Inserts the length prefix of the uncompressed bytes into dst
        // encoded as a unsigned varint
        self.inner
            .encode(bytes.len(), dst)
            .map_err(RPCError::from)?;

        let mut writer = FrameEncoder::new(Vec::new());
        writer.write_all(&bytes).map_err(RPCError::from)?;
        writer.flush().map_err(RPCError::from)?;

        // Write compressed bytes to `dst`
        dst.extend_from_slice(writer.get_ref());
        Ok(())
    }
}

// Decoder for outbound streams: Decodes RPC responses from peers.
//
// The majority of the decoding has now been pushed upstream due to the changing specification.
// We prefer to decode blocks and attestations with extra knowledge about the chain to perform
// faster verification checks before decoding entire blocks/attestations.
impl Decoder for SnappyOutboundCodec {
    type Item = RPCResponse;
    type Error = RPCError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.len.is_none() {
            // Decode the length of the uncompressed bytes from an unsigned varint
            match self.inner.decode(src).map_err(RPCError::from)? {
                Some(length) => {
                    self.len = Some(length as usize);
                }
                None => return Ok(None), // need more bytes to decode length
            }
        };

        let length = self.len.expect("length should be Some");

        // Should not attempt to decode rpc chunks with length > max_packet_size
        if length > self.max_packet_size {
            return Err(RPCError::InvalidData);
        }
        let mut reader = FrameDecoder::new(Cursor::new(&src));
        let mut decoded_buffer = vec![0; length];
        match reader.read_exact(&mut decoded_buffer) {
            Ok(()) => {
                // `n` is how many bytes the reader read in the compressed stream
                let n = reader.get_ref().position();
                self.len = None;
                let _read_byts = src.split_to(n as usize);
                match self.protocol.message_name {
                    Protocol::Status => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() > 0 {
                                Ok(Some(RPCResponse::Status(decoded_buffer)))
                            } else {
                                Err(RPCError::InvalidData)
                            }
                        }
                    },
                    Protocol::Goodbye => Err(RPCError::InvalidData),
                    Protocol::BlocksByRange => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() >= 0 {
                                Ok(Some(RPCResponse::BlocksByRange(decoded_buffer)))
                            } else {
                                Err(RPCError::InvalidData)
                            }
                        }
                    },
                    Protocol::BlocksByRoot => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() >= 0 {
                                Ok(Some(RPCResponse::BlocksByRoot(decoded_buffer)))
                            } else {
                                Err(RPCError::InvalidData)
                            }
                        }
                    },
                    Protocol::Ping => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() > 0 {
                                Ok(Some(RPCResponse::Pong(decoded_buffer)))
                            } else {
                                Err(RPCError::InvalidData)
                            }
                        }
                    },
                    Protocol::MetaData => match self.protocol.version {
                        Version::V1 => {
                            if decoded_buffer.len() > 0 {
                                Ok(Some(RPCResponse::MetaData(decoded_buffer)))
                            } else {
                                Err(RPCError::InvalidData)
                            }
                        }
                    },
                }
            }
            Err(e) => match e.kind() {
                // Haven't received enough bytes to decode yet
                // TODO: check if this is the only Error variant where we return `Ok(None)`
                ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                _ => return Err(e).map_err(RPCError::from),
            },
        }
    }
}

impl OutboundCodec<RPCRequest> for SnappyOutboundCodec {
    type ErrorType = String;

    fn decode_error(&mut self, src: &mut BytesMut) -> Result<Option<Self::ErrorType>, RPCError> {
        if self.len.is_none() {
            // Decode the length of the uncompressed bytes from an unsigned varint
            match self.inner.decode(src).map_err(RPCError::from)? {
                Some(length) => {
                    self.len = Some(length as usize);
                }
                None => return Ok(None), // need more bytes to decode length
            }
        };

        let length = self.len.expect("length should be Some");

        // Should not attempt to decode rpc chunks with length > max_packet_size
        if length > self.max_packet_size {
            return Err(RPCError::InvalidData);
        }
        let mut reader = FrameDecoder::new(Cursor::new(&src));
        let mut decoded_buffer = vec![0; length];
        match reader.read_exact(&mut decoded_buffer) {
            Ok(()) => {
                // `n` is how many bytes the reader read in the compressed stream
                let n = reader.get_ref().position();
                self.len = None;
                let _read_bytes = src.split_to(n as usize);
                Ok(Some(String::from_utf8_lossy(&decoded_buffer).into()))
            }
            Err(e) => match e.kind() {
                // Haven't received enough bytes to decode yet
                // TODO: check if this is the only Error variant where we return `Ok(None)`
                ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                _ => return Err(e).map_err(RPCError::from),
            },
        }
    }
}
