// But how does bitcoin actually work? - 3Blue1Brown
// https://www.youtube.com/watch?v=bBC-nXj3Ng4

mod ledger;

pub use ledger::{AcceptEventError, Ledger, LedgerEvent, TransferVerifierArgs};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Block<BlockIdT, IdentifierT, AmountT, PublicKeyT, SignatureT> {
    parent: Option<BlockIdT>,
    this_id: BlockIdT,
    events: Vec<LedgerEvent<IdentifierT, AmountT, PublicKeyT, SignatureT>>,
}
