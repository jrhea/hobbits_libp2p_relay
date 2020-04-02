// generates error types
use libp2p_wrapper;

use error_chain::error_chain;

error_chain! {
   links  {
       Libp2p(libp2p_wrapper::error::Error, libp2p_wrapper::error::ErrorKind);
   }
}
