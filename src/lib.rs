// But how does bitcoin actually work? - 3Blue1Brown
// https://www.youtube.com/watch?v=bBC-nXj3Ng4

mod blockchain;
mod ledger;

pub use blockchain::{AddBlockError, Block, BlockGraph};
pub use ledger::{AcceptEventError, Ledger, LedgerEvent, TransferVerifierArgs};
