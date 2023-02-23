// But how does bitcoin actually work? - 3Blue1Brown
// https://www.youtube.com/watch?v=bBC-nXj3Ng4

use std::{collections::HashMap, hash::Hash};
use tap::Tap as _;

#[derive(Debug, Clone)]
pub struct Ledger<IdentifierT, AmountT, PublicKeyT, SignatureT> {
    events: Vec<LedgerEvent<IdentifierT, AmountT, PublicKeyT, SignatureT>>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct UserSummary<AmountT, PublicKeyT> {
    balance: AmountT,
    public_key: PublicKeyT,
}

impl<IdentifierT, AmountT, PublicKeyT, SignatureT>
    Ledger<IdentifierT, AmountT, PublicKeyT, SignatureT>
{
    /// # Panics
    /// - if internal consistency is compromised
    fn users(&self) -> HashMap<IdentifierT, UserSummary<AmountT, PublicKeyT>>
    where
        IdentifierT: Hash + Eq + Clone,
        AmountT: Clone + num::CheckedAdd + num::CheckedSub + num::Zero + num::Unsigned,
        PublicKeyT: Clone,
    {
        self.events.iter().fold(HashMap::new(), |users, event| {
            users.tap_mut(|users| match event {
                LedgerEvent::NewUser {
                    identifier,
                    public_key,
                } => {
                    let clobbered = users
                        .insert(
                            identifier.clone(),
                            UserSummary {
                                balance: AmountT::zero(),
                                public_key: public_key.clone(),
                            },
                        )
                        .is_some();
                    assert!(!clobbered, "duplicate users in history")
                }
                LedgerEvent::Mint {
                    beneficiary,
                    amount,
                } => {
                    let balance = &mut users
                        .get_mut(beneficiary)
                        .expect("no user for mint")
                        .balance;
                    *balance = balance.checked_add(amount).expect("mint overflows balance");
                }
                LedgerEvent::Transfer {
                    benefactor,
                    beneficiary,
                    amount,
                    benefactor_signature: _, // TODO(aatifsyed): check signature?
                } => {
                    let benefactor = &mut users
                        .get_mut(benefactor)
                        .expect("no benefactor for transfer")
                        .balance;
                    *benefactor = benefactor
                        .checked_sub(amount)
                        .expect("transfer overdraws benefactor");
                    let beneficiary = &mut users
                        .get_mut(beneficiary)
                        .expect("no beneficiary for transfer")
                        .balance;
                    *beneficiary = beneficiary
                        .checked_add(amount)
                        .expect("transfer overflows beneficiary");
                }
            })
        })
    }

    pub fn try_accept(
        &self,
        event: LedgerEvent<IdentifierT, AmountT, PublicKeyT, SignatureT>,
    ) -> Result<Self, AcceptTransactionError> {
        match event {
            LedgerEvent::NewUser {
                identifier,
                public_key,
            } => todo!(),
            LedgerEvent::Mint {
                beneficiary,
                amount,
            } => todo!(),
            LedgerEvent::Transfer {
                benefactor,
                beneficiary,
                amount,
                benefactor_signature,
            } => todo!(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AcceptTransactionError {
    #[error("the transfer would overdraw the account")]
    TransferBenefactorWouldOverdraw,
    #[error("transfer's benefactor does not exist")]
    NoSuchBenefactorForTransfer,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, enum_as_inner::EnumAsInner)]
pub enum LedgerEvent<IdentifierT, AmountT, PublicKeyT, SignatureT> {
    // must have a new user message so that the ledger knows the public key, right?
    NewUser {
        identifier: IdentifierT,
        public_key: PublicKeyT,
    },
    Mint {
        beneficiary: IdentifierT,
        amount: AmountT,
    },
    Transfer {
        benefactor: IdentifierT,
        beneficiary: IdentifierT,
        amount: AmountT,
        benefactor_signature: SignatureT,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Block<HashT, IdentifierT, AmountT, PublicKeyT, SignatureT> {
    parent: Option<HashT>,
    events: Vec<LedgerEvent<IdentifierT, AmountT, PublicKeyT, SignatureT>>,
}
