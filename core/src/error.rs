// generates error types
use network;

use error_chain::error_chain;

error_chain! {
   links  {
       Libp2p(network::error::Error, network::error::ErrorKind);
   }
}
