pub type RequestId = usize;

/* RPC Handling and Grouping */
// Collection of enums and structs used by the Codecs to encode/decode RPC messages

#[derive(Debug, Clone)]
pub enum RPCResponse {
    /// An RPC message.
    Message(Vec<u8>),
}

#[derive(Debug)]
pub enum RPCErrorResponse {
    Success(RPCResponse),
    InvalidRequest(ErrorMessage),
    ServerError(ErrorMessage),
    Unknown(ErrorMessage),
}

impl RPCErrorResponse {
    /// Used to encode the response.
    pub fn as_u8(&self) -> u8 {
        match self {
            RPCErrorResponse::Success(_) => 0,
            RPCErrorResponse::InvalidRequest(_) => 2,
            RPCErrorResponse::ServerError(_) => 3,
            RPCErrorResponse::Unknown(_) => 255,
        }
    }

    /// Tells the codec whether to decode as an RPCResponse or an error.
    pub fn is_response(response_code: u8) -> bool {
        match response_code {
            0 => true,
            _ => false,
        }
    }

    /// Builds an RPCErrorResponse from a response code and an ErrorMessage
    pub fn from_error(response_code: u8, err: ErrorMessage) -> Self {
        match response_code {
            2 => RPCErrorResponse::InvalidRequest(err),
            3 => RPCErrorResponse::ServerError(err),
            _ => RPCErrorResponse::Unknown(err),
        }
    }
}

#[derive(Debug)]
pub struct ErrorMessage {
    /// The UTF-8 encoded Error message string.
    pub error_message: Vec<u8>,
}

impl ErrorMessage {
    pub fn as_string(&self) -> String {
        String::from_utf8(self.error_message.clone()).unwrap_or_else(|_| "".into())
    }
}
