use bdk::bitcoin::{psbt::PartiallySignedTransaction, Address};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingProposal {
	pub memo: String,
	pub to_address: Address,
	pub amount: u64,
	pub psbt: PartiallySignedTransaction,
}

impl SpendingProposal {
	pub fn new<S>(
		memo: S,
		to_address: Address,
		amount: u64,
		psbt: PartiallySignedTransaction,
	) -> Self
	where
		S: Into<String>,
	{
		Self { memo: memo.into(), to_address, amount, psbt }
	}

	/// Deserialize from `JSON` string
	pub fn from_json<S>(json: S) -> Result<Self, serde_json::Error>
	where
		S: Into<String>,
	{
		serde_json::from_str(&json.into())
	}

	/// Serialize to `JSON` string
	pub fn as_json(&self) -> String {
		serde_json::json!(self).to_string()
	}
}
