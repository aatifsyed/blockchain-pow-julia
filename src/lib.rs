// These primitives are based on the following video:
//
// But how does bitcoin actually work? - 3Blue1Brown
// https://www.youtube.com/watch?v=bBC-nXj3Ng4

mod blockchain;
mod ledger;
mod proof_of_work;

pub use blockchain::{AddBlockError, Block, BlockGraph};
pub use ledger::{AcceptEventError, Ledger, LedgerEvent, TransferVerifierArgs};
pub use proof_of_work::{check_work, do_work, DoWorkError, WithProofOfWork};
