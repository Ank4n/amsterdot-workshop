// This file is part of Acala.

// Copyright (C) 2020-2021 Acala Foundation.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg(any(feature = "std", feature = "bench"))]

use super::super::*;

use frame_support::{
	construct_runtime, ord_parameter_types, parameter_types,
	traits::{ConstU128, ConstU32, ConstU64, Everything, FindAuthor, Nothing},
	ConsensusEngineId,
};
use frame_system::EnsureSignedBy;
use module_support::{mocks::MockAddressMapping, TransactionPayment};
use orml_traits::parameter_type_with_key;
pub use primitives::{
	define_combined_task, Address, Amount, Block, BlockNumber, CurrencyId, Header, ReserveIdentifier, Signature,
	TokenSymbol,
};
use sp_core::{H160, H256};
use sp_runtime::{
	traits::{BlakeTwo256, BlockNumberProvider, IdentityLookup},
	AccountId32,
};

type Balance = u128;

mod evm_mod {
	pub use super::super::super::*;
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ConstU32<250>;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = crate::CallKillAccount<Runtime>;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type MaxLocks = ();
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = ReserveIdentifier;
	type WeightInfo = ();
}

impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1000>;
	type WeightInfo = ();
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = ();
	type DustRemovalWhitelist = Nothing;
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::ACA);
}

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}
pub type AdaptedBasicCurrency = orml_currencies::BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;

define_combined_task! {
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	pub enum ScheduledTasks {
		EvmTask(EvmTask<Runtime>),
	}
}

pub struct MockBlockNumberProvider;

impl BlockNumberProvider for MockBlockNumberProvider {
	type BlockNumber = u32;

	fn current_block_number() -> Self::BlockNumber {
		Zero::zero()
	}
}

impl module_idle_scheduler::Config for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type Task = ScheduledTasks;
	type MinimumWeightRemainInBlock = ConstU64<0>;
	type RelayChainBlockNumberProvider = MockBlockNumberProvider;
	type DisableBlockThreshold = ConstU32<6>;
}

pub struct GasToWeight;

impl Convert<u64, u64> for GasToWeight {
	fn convert(a: u64) -> u64 {
		a
	}
}

pub struct AuthorGiven;
impl FindAuthor<AccountId32> for AuthorGiven {
	fn find_author<'a, I>(_digests: I) -> Option<AccountId32>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		Some(AccountId32::from([1; 32]))
	}
}

parameter_types! {
	pub NetworkContractSource: H160 = H160::from_low_u64_be(1);
}

ord_parameter_types! {
	pub const CouncilAccount: AccountId32 = AccountId32::from([1u8; 32]);
	pub const TreasuryAccount: AccountId32 = AccountId32::from([2u8; 32]);
	pub const NetworkContractAccount: AccountId32 = AccountId32::from([0u8; 32]);
	pub const StorageDepositPerByte: Balance = convert_decimals_to_evm(10);
}

impl Config for Runtime {
	type AddressMapping = MockAddressMapping;
	type Currency = Balances;
	type TransferAll = Currencies;
	type NewContractExtraBytes = ConstU32<100>;
	type StorageDepositPerByte = StorageDepositPerByte;
	type TxFeePerGas = ConstU128<20_000_000>;

	type Event = Event;
	type PrecompilesType = ();
	type PrecompilesValue = ();
	type ChainId = ConstU64<1>;
	type GasToWeight = GasToWeight;
	type ChargeTransactionPayment = DefaultTransactionPayment;

	type NetworkContractOrigin = EnsureSignedBy<NetworkContractAccount, AccountId32>;
	type NetworkContractSource = NetworkContractSource;
	type DeveloperDeposit = ConstU128<1000>;
	type PublicationFee = ConstU128<200>;
	type TreasuryAccount = TreasuryAccount;
	type FreePublicationOrigin = EnsureSignedBy<CouncilAccount, AccountId32>;

	type Runner = crate::runner::stack::Runner<Self>;
	type FindAuthor = AuthorGiven;
	type Task = ScheduledTasks;
	type IdleScheduler = IdleScheduler;
	type WeightInfo = ();
}

pub struct DefaultTransactionPayment;

use frame_support::traits::Imbalance;
impl<AccountId, Balance: Default + Copy, NegativeImbalance: Imbalance<Balance>>
	TransactionPayment<AccountId, Balance, NegativeImbalance> for DefaultTransactionPayment
{
	fn reserve_fee(_who: &AccountId, _weight: Weight) -> Result<Balance, DispatchError> {
		Ok(Default::default())
	}

	fn unreserve_fee(_who: &AccountId, _fee: Balance) {}

	fn unreserve_and_charge_fee(
		_who: &AccountId,
		_weight: Weight,
	) -> Result<(Balance, NegativeImbalance), TransactionValidityError> {
		Ok((Default::default(), Imbalance::zero()))
	}

	fn refund_fee(
		_who: &AccountId,
		_weight: Weight,
		_payed: NegativeImbalance,
	) -> Result<(), TransactionValidityError> {
		Ok(())
	}

	fn charge_fee(
		_who: &AccountId,
		_len: u32,
		_weight: Weight,
		_tip: Balance,
		_pays_fee: Pays,
		_class: DispatchClass,
	) -> Result<(), TransactionValidityError> {
		Ok(())
	}
}

pub type SignedExtra = (frame_system::CheckWeight<Runtime>,);
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		EVM: evm_mod::{Pallet, Config<T>, Call, Storage, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		IdleScheduler: module_idle_scheduler::{Pallet, Call, Storage, Event<T>},
	}
);
