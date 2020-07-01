pub(crate) mod base;
pub(crate) mod snappy;

use self::base::{BaseInboundCodec, BaseOutboundCodec};
use self::snappy::{SnappyInboundCodec, SnappyOutboundCodec};
use crate::rpc::protocol::RPCError;
use crate::rpc::{RPCCodedResponse, RPCRequest};
use libp2p::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

// Known types of codecs
pub enum InboundCodec {
    Snappy(BaseInboundCodec<SnappyInboundCodec>),
}

pub enum OutboundCodec {
    Snappy(BaseOutboundCodec<SnappyOutboundCodec>),
}

impl Encoder<RPCCodedResponse> for InboundCodec {
    type Error = RPCError;

    fn encode(&mut self, item: RPCCodedResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match self {
            InboundCodec::Snappy(codec) => codec.encode(item, dst),
        }
    }
}

impl Decoder for InboundCodec {
    type Item = RPCRequest;
    type Error = RPCError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self {
            InboundCodec::Snappy(codec) => codec.decode(src),
        }
    }
}

impl Encoder<RPCRequest> for OutboundCodec {
    type Error = RPCError;

    fn encode(&mut self, item: RPCRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match self {
            OutboundCodec::Snappy(codec) => codec.encode(item, dst),
        }
    }
}

impl Decoder for OutboundCodec {
    type Item = RPCCodedResponse;
    type Error = RPCError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self {
            OutboundCodec::Snappy(codec) => codec.decode(src),
        }
    }
}
