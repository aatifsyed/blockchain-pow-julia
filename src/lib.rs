// These primitives are based on the following video:
//
// But how does bitcoin actually work? - 3Blue1Brown
// https://www.youtube.com/watch?v=bBC-nXj3Ng4

mod blockchain;
mod ledger;
mod proof_of_work;

pub use blockchain::{AddBlockError, Block, BlockGraph};
pub use ledger::{AcceptEventError, Ledger, LedgerEvent, TransferVerifierArgs, UserSummary};
pub use proof_of_work::{check_work, do_work, DoWorkError, WithProofOfWork};

type PublicKey = p256::ecdsa::VerifyingKey;
type UserId = PublicKey;
type Signature = p256::ecdsa::Signature;
type BlockId = sha2::digest::Output<sha2::Sha256>;

pub struct ValidatorNode {
    ledger: Ledger<UserId, u64, PublicKey, Signature>,
    blocks: BlockGraph<BlockId, UserId, u64, PublicKey, Signature>,
}
