use crate::oracle::{ActorMessage, ScpArchiveStorage};
use stellar_relay::sdk::{
	compound_types::{LimitedVarArray, UnlimitedVarArray, XdrArchive},
	types::{ScpEnvelope, ScpHistoryEntry, TransactionSet},
	TransactionEnvelope, XdrCodec,
};

use crate::oracle::{
	constants::{get_min_externalized_messages, MAX_SLOT_TO_REMEMBER},
	traits::FileHandler,
	EnvelopesFileHandler, ScpMessageCollector, Slot, TxHash, TxSetsFileHandler,
};

use std::convert::TryInto;

/// Determines whether the data retrieved is from the current map or from a file.
type DataFromFile<T> = (T, bool);

/// The Proof of Transactions that needed to be processed
pub struct Proof {
	tx_env: TransactionEnvelope,
	envelopes: UnlimitedVarArray<ScpEnvelope>,
	tx_set: TransactionSet,
}

impl Proof {
	/// Encodes these Stellar structures to make it easier to send as extrinsic.
	pub fn encode(&self) -> (String, String, String) {
		let tx_env_xdr = self.tx_env.to_xdr();
		let tx_env_encoded = base64::encode(tx_env_xdr);

		let envelopes_xdr = self.envelopes.to_xdr();
		let envelopes_encoded = base64::encode(envelopes_xdr);

		let tx_set_xdr = self.tx_set.to_xdr();
		let tx_set_encoded = base64::encode(tx_set_xdr);

		(tx_env_encoded, envelopes_encoded, tx_set_encoded)
	}
}

pub enum ProofStatus {
	Proof(Proof),
	LackingEnvelopes(Slot),
	NoTxSetFound(Slot),
}

// handles the creation of proofs.
// this means it will access the maps and potentially the files.
impl ScpMessageCollector {
	fn get_slot(&self, tx_hash: &TxHash) -> Option<Slot> {
		self.tx_hash_map().get(tx_hash).map(|slot| *slot)
	}

	/// Returns either a list of ScpEnvelopes or a ProofStatus saying it failed to retrieve a list.
	fn get_envelopes(&self, slot: Slot) -> Result<UnlimitedVarArray<ScpEnvelope>, ProofStatus> {
		let envelopes = self._get_envelopes(slot);

		let mut vec_envelopes = vec![];

		let fetch_more = if envelopes.is_none() {
			true
		} else {
			vec_envelopes = envelopes.unwrap().0;
			vec_envelopes.len() < get_min_externalized_messages(self.is_public())
		};

		if fetch_more {
			self.restore_missed_slots(slot);
			return Err(ProofStatus::LackingEnvelopes(slot))
		}

		Ok(UnlimitedVarArray::new(vec_envelopes).unwrap_or(UnlimitedVarArray::new_empty()))
	}

	fn restore_missed_slots(&self, slot: Slot) {
		let last_slot_index = *self.last_slot_index();
		let action_sender = self.action_sender.clone();
		let rw_lock = self.envelopes_map_clone();
		tokio::spawn(async move {
			// If the current slot is still in the range of 'remembered' slots
			if slot > last_slot_index - MAX_SLOT_TO_REMEMBER {
				let result =
					action_sender.send(ActorMessage::GetScpState { missed_slot: slot }).await;
			} else {
				let slot_index: u32 = slot.try_into().unwrap();
				let scp_archive: XdrArchive<ScpHistoryEntry> =
					ScpArchiveStorage::get_scp_archive(slot.try_into().unwrap()).await.unwrap();

				let value = scp_archive.get_vec().into_iter().find(|&scp_entry| {
					if let ScpHistoryEntry::V0(scp_entry_v0) = scp_entry {
						return scp_entry_v0.ledger_messages.ledger_seq == slot_index
					} else {
						return false
					}
				});

				if let Some(i) = value {
					if let ScpHistoryEntry::V0(scp_entry_v0) = i {
						let slot_scp_envelopes = scp_entry_v0.clone().ledger_messages.messages;
						let vec_scp = slot_scp_envelopes.get_vec().clone(); //TODO store envelopes_map or send via mpsc

						let mut envelopes_map = rw_lock.write();

						if let None = envelopes_map.get_mut(&slot) {
							tracing::info!("Adding archived SCP envelopes for slot {}", slot);
							envelopes_map.insert(slot, vec_scp);
						}
					}
				}
			}
		});
	}

	/// helper method for `get_envelopes()`.
	/// It returns a tuple of (list of `ScpEnvelope`s, <if_list_came_from_a_file>).
	fn _get_envelopes(&self, slot: Slot) -> Option<DataFromFile<Vec<ScpEnvelope>>> {
		self.envelopes_map().get(&slot).map(|envs| (envs.clone(), false)).or_else(|| {
			match EnvelopesFileHandler::get_map_from_archives(slot) {
				Ok(env_map) => env_map.get(&slot).map(|envs| (envs.clone(), true)),
				Err(e) => {
					tracing::warn!("Failed to read envelopes map from a file: {:?}", e);
					None
				},
			}
		})
	}

	/// Returns either a TransactionSet or a ProofStatus saying it failed to retrieve the set.
	fn get_txset(&self, slot: Slot) -> Result<DataFromFile<TransactionSet>, ProofStatus> {
		self.txset_map()
			.get(&slot)
			.map(|set| (set.clone(), false))
			.or_else(|| match TxSetsFileHandler::get_map_from_archives(slot) {
				Ok(set_map) => set_map.get(&slot).map(|set| (set.clone(), true)),
				Err(_) => None,
			})
			.ok_or(ProofStatus::NoTxSetFound(slot))
	}

	/// Returns a `ProofStatus`.
	///
	/// # Arguments
	///
	/// * `tx_env` - the TransactionEnvelope to buid a proof of.
	/// * `slot` - The expected slot where the `tx_env` belongs to.
	pub(super) fn build_proof(&self, tx_env: TransactionEnvelope, slot: Slot) -> ProofStatus {
		let envelopes = match self.get_envelopes(slot) {
			Ok(envelopes) => envelopes,
			Err(neg_status) => return neg_status,
		};

		let tx_set = match self.get_txset(slot) {
			Ok((tx_set, _)) => tx_set,
			Err(neg_status) => return neg_status,
		};

		ProofStatus::Proof(Proof { tx_env, envelopes, tx_set })
	}
}
