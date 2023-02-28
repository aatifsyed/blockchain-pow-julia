use std::{collections::HashMap, hash::Hash};
use tap::Tap as _;

/// A list of _valid_ events.
///
/// This is the "functional core" of the implementation.
#[derive(Debug, Clone)]
pub struct Ledger<UserIdT, AmountT, PublicKeyT, SignatureT> {
    events: Vec<LedgerEvent<UserIdT, AmountT, PublicKeyT, SignatureT>>,
    // TODO: cache state of the world, and recompute per event
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct UserSummary<AmountT, PublicKeyT> {
    balance: AmountT,
    public_key: PublicKeyT,
}

impl<UserIdT, AmountT, PublicKeyT, SignatureT> Ledger<UserIdT, AmountT, PublicKeyT, SignatureT>
where
    UserIdT: Hash + Eq + Clone,
    AmountT: Clone + num::CheckedAdd + num::CheckedSub + num::Zero + num::Unsigned,
    PublicKeyT: Clone,
{
    /// Get the current state of all user balances according to this event history.
    ///
    /// # Panics
    /// - if internal consistency is compromised
    ///
    // This could be stored in the [Ledger] so we're not constantly recomputing it
    // this is done on the `mutable-ledger` branch, but I'm sticking with this implementation
    // because it allows us to reject an invalid block more easily.
    pub fn users(&self) -> HashMap<UserIdT, UserSummary<AmountT, PublicKeyT>> {
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

    fn with_event_unchecked(
        &self,
        event: LedgerEvent<UserIdT, AmountT, PublicKeyT, SignatureT>,
    ) -> Self
    where
        SignatureT: Clone,
    {
        Self {
            events: self.events.clone().tap_mut(|it| it.push(event)),
        }
    }

    /// Fail with [AcceptEventError::NoSuchAccount] or [AcceptEventError::WouldOverflow] as appropriate
    fn could_receive(
        &self,
        beneficiary: &UserIdT,
        amount: &AmountT,
    ) -> Result<UserSummary<AmountT, PublicKeyT>, AcceptEventError> {
        match self.users().get(beneficiary) {
            Some(user_summary) => match user_summary.balance.checked_add(amount) {
                Some(_) => Ok(user_summary.clone()),
                None => Err(AcceptEventError::WouldOverflow),
            },
            None => Err(AcceptEventError::NoSuchAccount),
        }
    }

    /// Fail with [AcceptEventError::NoSuchAccount] or [AcceptEventError::WouldOverdraw] as appropriate.
    fn could_send(
        &self,
        benefactor: &UserIdT,
        amount: &AmountT,
    ) -> Result<UserSummary<AmountT, PublicKeyT>, AcceptEventError> {
        match self.users().get(benefactor) {
            Some(user_summary) => match user_summary.balance.checked_sub(amount) {
                Some(_) => Ok(user_summary.clone()),
                None => Err(AcceptEventError::WouldOverdraw), // AmountT: num::Unsigned
            },
            None => Err(AcceptEventError::NoSuchAccount),
        }
    }

    // TODO(vectorised version)
    pub fn with_event<BlockIdT>(
        &self,
        event: LedgerEvent<UserIdT, AmountT, PublicKeyT, SignatureT>,

        // This is a bit of a quick and dirty implementation detail leaked to the outside.
        // Really we should have a TransferVerifierT: TransferVerifier on the Ledger, since verification is fixed for a ledger.
        // We could then impl TransferVerifier for e.g FnMut(...) -> bool.
        // For now, keep in this function.
        block_id: BlockIdT,
        event_index: usize,
        transfer_verifier: impl FnOnce(
            TransferVerifierArgs<BlockIdT, &UserIdT, &AmountT, &PublicKeyT, &SignatureT>,
        ) -> Result<(), ()>,
    ) -> Result<Self, AcceptEventError>
    where
        SignatureT: Clone,
    {
        match &event {
            LedgerEvent::NewUser {
                identifier,
                public_key: _,
            } => match self.users().contains_key(identifier) {
                true => Err(AcceptEventError::UserIdTaken),
                false => Ok(self.with_event_unchecked(event)),
            },
            LedgerEvent::Mint {
                beneficiary,
                amount,
            } => {
                self.could_receive(beneficiary, amount)?;
                Ok(self.with_event_unchecked(event))
            }
            LedgerEvent::Transfer {
                benefactor,
                beneficiary,
                amount,
                benefactor_signature,
            } => {
                self.could_receive(beneficiary, amount)?;
                let benefactor_public_key = &self.could_send(benefactor, amount)?.public_key;
                transfer_verifier(TransferVerifierArgs {
                    block_id,
                    event_index,
                    benefactor,
                    beneficiary,
                    amount,
                    benefactor_public_key,
                    benefactor_signature,
                })
                .map_err(|_| AcceptEventError::InvalidSignature)?;
                Ok(self.with_event_unchecked(event))
            }
        }
    }
}

pub struct TransferVerifierArgs<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT> {
    pub block_id: BlockIdT,
    pub event_index: usize,
    pub benefactor: UserIdT,
    pub beneficiary: UserIdT,
    pub amount: AmountT,
    pub benefactor_public_key: PublicKeyT,
    pub benefactor_signature: SignatureT,
}

#[derive(Debug, thiserror::Error)]
pub enum AcceptEventError {
    #[error("a user with the requested identifier already exists")]
    UserIdTaken,
    #[error("would overdraw an account")]
    WouldOverdraw,
    #[error("an account in this event does not exist")]
    NoSuchAccount,
    #[error("an account balance would overflow")]
    WouldOverflow,
    #[error("invalid signature for transfer")]
    InvalidSignature,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, enum_as_inner::EnumAsInner)]
pub enum LedgerEvent<UserIdT, AmountT, PublicKeyT, SignatureT> {
    // must have a new user message so that the ledger knows the public key for verification, right?
    NewUser {
        identifier: UserIdT,
        public_key: PublicKeyT,
    },
    Mint {
        beneficiary: UserIdT,
        amount: AmountT,
    },
    Transfer {
        benefactor: UserIdT,
        beneficiary: UserIdT,
        amount: AmountT,
        benefactor_signature: SignatureT,
    },
}
