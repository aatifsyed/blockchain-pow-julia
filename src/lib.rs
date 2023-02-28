// These primitives are based on the following video:
//
// But how does bitcoin actually work? - 3Blue1Brown
// https://www.youtube.com/watch?v=bBC-nXj3Ng4

mod blockchain;
mod ledger;
mod proof_of_work;

pub use blockchain::{AddBlockError, AddBlockOk, Block, BlockGraph};
pub use ledger::{AcceptEventError, Ledger, LedgerEvent, TransferVerifierArgs, UserSummary};
pub use proof_of_work::{check_work, do_work, DoWorkError, WithProofOfWork};

type PublicKey = p256::ecdsa::VerifyingKey;
type UserId = PublicKey;
type Signature = p256::ecdsa::Signature;
type BlockId = sha2::digest::Output<sha2::Sha256>;

// Optimisation ideas:
// - Keep ledger progress in the block graph, compacting every N blocks
// - We can optimise our graph because it's immutable - maybe just allocate blocks in an arena
// - We can tune our optimisation based on our correctness tolerance
pub struct ValidatorNode {
    ledger: Ledger<UserId, u64, PublicKey, Signature>,
    blocks: BlockGraph<BlockId, UserId, u64, PublicKey, Signature>,
}

impl ValidatorNode {
    pub fn ingest_block(
        &mut self,
        block: WithProofOfWork<Block<BlockId, UserId, u64, PublicKey, Signature>>,
    ) -> Result<(), BlockIngestError> {
        // Does this count as easily precomputable? Probably...
        let (c, re_min, re_max, target_iterations) = get_work_params_from_block_id(block.inner.id);
        proof_of_work::check_work(c, re_min, re_max, block.candidate, target_iterations)
            .map_err(BlockIngestError::DoWorkError)?;

        let block = block.inner;

        match self.blocks.add_block(block) {
            Ok(AddBlockOk::CanAddNewEventsToLedger) => {
                // TODO(newtype so we can hash the key)
                // self.ledger = self.ledger.with_event(todo!());
                todo!()
            }
            Ok(AddBlockOk::MustRebuildCache) => {
                let _winning_chain = self.blocks.winning_chain();
                let ledger = todo!("fold ledger");
                // there's an error condition here we need to handle - invalid events in a block that passed pow
            }
            Ok(AddBlockOk::Noop) => {}
            Err(AddBlockError::WouldClobber) => unreachable!("hash collision"),
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BlockIngestError {
    #[error("block's work was invalid")]
    DoWorkError(proof_of_work::DoWorkError),
}

fn get_work_params_from_block_id(id: BlockId) -> (num::Complex<f64>, f64, f64, u16) {
    todo!()
}
