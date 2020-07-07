use crate::rpc::{RPCCodedResponse, RPCRequest, RPCResponse};
use libp2p::bytes::BufMut;
use libp2p::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

pub trait OutboundCodec<TItem>: Encoder<TItem> + Decoder {
    type ErrorType;

    fn decode_error(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<Self::ErrorType>, <Self as Decoder>::Error>;
}

/* Global Inbound Codec */
// This deals with Decoding RPC Requests from other peers and encoding our responses

pub struct BaseInboundCodec<TCodec>
where
    TCodec: Encoder<RPCCodedResponse> + Decoder,
{
    /// Inner codec for handling various encodings
    inner: TCodec,
}

impl<TCodec> BaseInboundCodec<TCodec>
where
    TCodec: Encoder<RPCCodedResponse> + Decoder,
{
    pub fn new(codec: TCodec) -> Self {
        BaseInboundCodec { inner: codec }
    }
}

/* Global Outbound Codec */
// This deals with Decoding RPC Responses from other peers and encoding our requests
pub struct BaseOutboundCodec<TOutboundCodec>
where
    TOutboundCodec: OutboundCodec<RPCRequest>,
{
    /// Inner codec for handling various encodings.
    inner: TOutboundCodec,
    /// Keeps track of the current response code for a chunk.
    current_response_code: Option<u8>,
}

impl<TOutboundCodec> BaseOutboundCodec<TOutboundCodec>
where
    TOutboundCodec: OutboundCodec<RPCRequest>,
{
    pub fn new(codec: TOutboundCodec) -> Self {
        BaseOutboundCodec {
            inner: codec,
            current_response_code: None,
        }
    }
}

/* Implementation of the Encoding/Decoding for the global codecs */

/* Base Inbound Codec */

// This Encodes RPC Responses sent to external peers
impl<TCodec> Encoder<RPCCodedResponse> for BaseInboundCodec<TCodec>
where
    TCodec: Decoder + Encoder<RPCCodedResponse>,
{
    type Error = <TCodec as Encoder<RPCCodedResponse>>::Error;

    fn encode(&mut self, item: RPCCodedResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.clear();
        dst.reserve(1);
        dst.put_u8(
            item.as_u8()
                .expect("Should never encode a stream termination"),
        );
        self.inner.encode(item, dst)
    }
}

// This Decodes RPC Requests from external peers
impl<TCodec> Decoder for BaseInboundCodec<TCodec>
where
    TCodec: Encoder<RPCCodedResponse> + Decoder<Item = RPCRequest>,
{
    type Item = RPCRequest;
    type Error = <TCodec as Decoder>::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.decode(src)
    }
}

/* Base Outbound Codec */

// This Encodes RPC Requests sent to external peers
impl<TCodec> Encoder<RPCRequest> for BaseOutboundCodec<TCodec>
where
    TCodec: OutboundCodec<RPCRequest> + Encoder<RPCRequest>,
{
    type Error = <TCodec as Encoder<RPCRequest>>::Error;

    fn encode(&mut self, item: RPCRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.inner.encode(item, dst)
    }
}

// This decodes RPC Responses received from external peers
impl<TCodec> Decoder for BaseOutboundCodec<TCodec>
where
    TCodec: OutboundCodec<RPCRequest, ErrorType = String> + Decoder<Item = RPCResponse>,
{
    type Item = RPCCodedResponse;
    type Error = <TCodec as Decoder>::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // if we have only received the response code, wait for more bytes
        if src.len() <= 1 {
            return Ok(None);
        }
        // using the response code determine which kind of payload needs to be decoded.
        let response_code = self.current_response_code.unwrap_or_else(|| {
            let resp_code = src.split_to(1)[0];
            self.current_response_code = Some(resp_code);
            resp_code
        });

        let inner_result = {
            if RPCCodedResponse::is_response(response_code) {
                // decode an actual response and mutates the buffer if enough bytes have been read
                // returning the result.
                self.inner
                    .decode(src)
                    .map(|r| r.map(RPCCodedResponse::Success))
            } else {
                // decode an error
                self.inner
                    .decode_error(src)
                    .map(|r| r.map(|resp| RPCCodedResponse::from_error(response_code, resp)))
            }
        };
        // if the inner decoder was capable of decoding a chunk, we need to reset the current
        // response code for the next chunk
        if let Ok(Some(_)) = inner_result {
            self.current_response_code = None;
        }
        // return the result
        inner_result
    }
}

#[cfg(test)]
mod tests {
    use super::super::snappy::*;
    use super::*;
    use crate::rpc::protocol::*;

    #[test]
    fn test_decode_status_message() {
        
        let message = hex::decode("ff060000734e615070590032000006e71e7b54989925efd6c9cbcb8ceb9b5f71216f5137282bf6a1e3b50f64e42d6c7fb347abe07eb0db8200000005029e2800").unwrap();
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&message);

        let snappy_protocol_id =
            ProtocolId::new(Protocol::Status, Version::V1, Encoding::Snappy);

        let mut snappy_outbound_codec =
            SnappyOutboundCodec::new(snappy_protocol_id, 1_048_576);

        // decode message just as snappy message
        let snappy_decoded_message = snappy_outbound_codec.decode(&mut buf.clone());

        // build codecs for entire chunk
        let mut snappy_base_outbound_codec = BaseOutboundCodec::new(snappy_outbound_codec);

        // decode message as ssz snappy chunk
        let snappy_decoded_chunk = snappy_base_outbound_codec.decode(&mut buf.clone());


        let _ = dbg!(snappy_decoded_message);
        let _ = dbg!(snappy_decoded_chunk);

        
    }
}
