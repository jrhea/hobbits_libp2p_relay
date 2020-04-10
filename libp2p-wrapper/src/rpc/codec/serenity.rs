use crate::rpc::{
    codec::base::OutboundCodec,
    protocol::{ProtocolId, RPCError},
};
use crate::rpc::{ErrorMessage, RPCErrorResponse, RPCRequest, RPCResponse};
use bytes::{Bytes, BytesMut};
use tokio::codec::{Decoder, Encoder};
use unsigned_varint::codec::UviBytes;

/* Inbound Codec */

pub struct SerenityInboundCodec {
    inner: UviBytes,
    protocol: ProtocolId,
}

impl SerenityInboundCodec {
    pub fn new(protocol: ProtocolId, max_packet_size: usize) -> Self {
        let mut uvi_codec = UviBytes::default();
        uvi_codec.set_max_len(max_packet_size);

        // this encoding only applies to Serenity.
        debug_assert!(protocol.encoding.as_str() == "ssz");

        SerenityInboundCodec {
            inner: uvi_codec,
            protocol,
        }
    }
}

// Encoder for inbound
impl Encoder for SerenityInboundCodec {
    type Item = RPCErrorResponse;
    type Error = RPCError;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = match item {
            RPCErrorResponse::Success(resp) => {
                match resp {
                    RPCResponse::Message(res) => Bytes::from(res),
                }
            },
            RPCErrorResponse::InvalidRequest(err) => Bytes::from(err.as_string()),
            RPCErrorResponse::ServerError(err) => Bytes::from(err.as_string()),
            RPCErrorResponse::Unknown(err) => Bytes::from(err.as_string()),
        };

        if !bytes.is_empty() {
            // length-prefix and return
            return self
                .inner
                .encode(bytes, dst)
                .map_err(RPCError::from);
        }
        Ok(())
    }
}

// Decoder for inbound
impl Decoder for SerenityInboundCodec {
    type Item = RPCRequest;
    type Error = RPCError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.inner.decode(src).map_err(RPCError::from) {
            Ok(Some(packet)) => match self.protocol.message_name.as_str() {
                "hello" => match self.protocol.version.as_str() {
                    "1" => Ok(Some(RPCRequest::Message(packet.to_vec()))),
                    _ => Err(RPCError::InvalidProtocol("Unknown Message version")),
                }
                _ => Err(RPCError::InvalidProtocol("Unknown message name.")),
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/* Outbound Codec */

pub struct SerenityOutboundCodec {
    inner: UviBytes,
    protocol: ProtocolId,
}

impl SerenityOutboundCodec {
    pub fn new(protocol: ProtocolId, max_packet_size: usize) -> Self {
        let mut uvi_codec = UviBytes::default();
        uvi_codec.set_max_len(max_packet_size);

        // this encoding only applies to Serenity.
        debug_assert!(protocol.encoding.as_str() == "ssz");

        SerenityOutboundCodec {
            inner: uvi_codec,
            protocol,
        }
    }
}

// Encoder for outbound
impl Encoder for SerenityOutboundCodec {
    type Item = RPCRequest;
    type Error = RPCError;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = match item {
            RPCRequest::Message(req) => Bytes::from(req),
        };
        // length-prefix
        self.inner
            .encode(bytes, dst)
            .map_err(RPCError::from)
    }
}

// Decoder for outbound
impl Decoder for SerenityOutboundCodec {
    type Item = RPCResponse;
    type Error = RPCError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.inner.decode(src).map_err(RPCError::from) {
            Ok(Some(packet)) => match self.protocol.message_name.as_str() {
                "hello" => match self.protocol.version.as_str() {
                    
                    "1" => Ok(Some(RPCResponse::Message(packet.to_vec()))),
                    _ => Err(RPCError::InvalidProtocol("Unknown rpc message version.")),
                },
                _ => Err(RPCError::InvalidProtocol("Unknown method")),
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl OutboundCodec for SerenityOutboundCodec {
    type ErrorType = ErrorMessage;

    fn decode_error(&mut self, src: &mut BytesMut) -> Result<Option<Self::ErrorType>, RPCError> {
        match self.inner.decode(src).map_err(RPCError::from) {
            Ok(Some(_packet)) => Ok(None),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
