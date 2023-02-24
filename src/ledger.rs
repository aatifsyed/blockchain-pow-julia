use std::{collections::HashMap, hash::Hash};
use tap::Tap as _;

/// A list of _valid_ events.
///
/// This is the ~"functional core" of the implementation.
#[derive(Debug, Clone)]
pub struct Ledger<UserIdT, AmountT, PublicKeyT, SignatureT> {
    events: Vec<LedgerEvent<UserIdT, AmountT, PublicKeyT, SignatureT>>,
    users: hashbrown::HashMap<UserIdT, UserSummary<AmountT, PublicKeyT>>, // precomputed fold over the event history
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
    fn fold_events(&self) -> HashMap<UserIdT, UserSummary<AmountT, PublicKeyT>> {
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

    pub fn try_accept_event<BlockIdT>(
        &mut self,
        event: LedgerEvent<UserIdT, AmountT, PublicKeyT, SignatureT>,

        // This is a bit of a quick and dirty implementation detail leaked to the outside.
        // Really we should have a TransferVerifierT: TransferVerifier on the Ledger, since verification is fixed for a ledger.
        // We could then impl TransferVerifier for e.g FnMut(...) -> bool.
        // For now, keep in this function.
        block_id: BlockIdT,
        event_index: usize,
        mut transfer_verifier: impl FnMut(
            TransferVerifierArgs<BlockIdT, &UserIdT, &AmountT, &PublicKeyT, &SignatureT>,
        ) -> Result<(), ()>,
    ) -> Result<(), AcceptEventError>
    where
        SignatureT: Clone,
    {
        use hashbrown::hash_map::Entry::{Occupied, Vacant};
        use AcceptEventError::{
            CannotTransferToSelf, NoSuchAccount, UserIdTaken, WouldOverdraw, WouldOverflow,
        };

        match &event {
            LedgerEvent::NewUser {
                identifier,
                public_key,
            } => match self.users.entry(identifier.clone()) {
                Occupied(_) => Err(UserIdTaken),
                Vacant(vacancy) => {
                    vacancy.insert(UserSummary {
                        balance: AmountT::zero(),
                        public_key: public_key.clone(),
                    });
                    self.events.push(event);
                    Ok(())
                }
            },
            LedgerEvent::Mint {
                beneficiary,
                amount,
            } => {
                // test
                let beneficiary = self.users.get_mut(beneficiary).ok_or(NoSuchAccount)?;
                let new_balance = beneficiary
                    .balance
                    .checked_add(amount)
                    .ok_or(WouldOverflow)?;

                // set
                self.events.push(event);
                beneficiary.balance = new_balance;
                Ok(())
            }
            LedgerEvent::Transfer {
                benefactor: benefactor_id,
                beneficiary: beneficiary_id,
                amount,
                benefactor_signature,
            } => {
                // test
                if benefactor_id == beneficiary_id {
                    return Err(CannotTransferToSelf);
                }

                let [benefactor, beneficiary] = self
                    .users
                    .get_many_mut([benefactor_id, beneficiary_id])
                    .ok_or(NoSuchAccount)?;

                let new_beneficiary_balance = beneficiary
                    .balance
                    .checked_add(amount)
                    .ok_or(WouldOverflow)?;

                let new_benefactor_balance = benefactor
                    .balance
                    .checked_sub(amount)
                    .ok_or(WouldOverdraw)?;

                transfer_verifier(TransferVerifierArgs {
                    block_id,
                    event_index,
                    benefactor: benefactor_id,
                    beneficiary: beneficiary_id,
                    amount,
                    benefactor_public_key: &benefactor.public_key,
                    benefactor_signature,
                })
                .map_err(|_| AcceptEventError::InvalidSignature)?;

                // set
                self.events.push(event);
                beneficiary.balance = new_beneficiary_balance;
                benefactor.balance = new_benefactor_balance;
                Ok(())
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
    #[error("benefactor and beneficiary of a transfer cannot be the same")]
    CannotTransferToSelf,
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
