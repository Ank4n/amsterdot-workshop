// This file is part of Acala.

// Copyright (C) 2020-2022 Acala Foundation.
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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{log, pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use sp_core::{H160, U256};
use sp_runtime::traits::{UniqueSaturatedInto, Zero};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
use support::{AddressMapping, ExecutionMode, InvokeContext, EVM};

pub use module::*;

#[derive(Default, Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ContractInfo {
	point: u128,
	instance_count: u32,
	wins: u32,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Contender {
	Contract(H160),
	AlwaysZero,
	Rotate,
	Smart,
}

impl Contender {
	fn is_contract(&self) -> bool {
		return matches!(self, Contender::Contract(_));
	}
}

#[module_evm_utility_macro::generate_function_selector]
#[derive(RuntimeDebug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum Action {
	Owner = "owner()",
	Play = "play(uint256,uint256,uint256)",
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type EVM: EVM<Self::AccountId>;
		type AddressMapping: AddressMapping<Self::AccountId>;

		#[pallet::constant]
		type PlayPerRound: Get<u32>;

		#[pallet::constant]
		type MaxContenderInstancesCount: Get<u32>;

		#[pallet::constant]
		type MaxQueueSize: Get<u32>;

		#[pallet::constant]
		type WinnerCount: Get<u32>;

		#[pallet::constant]
		type EnqueueCount: Get<u32>;

		#[pallet::constant]
		type MaxInstancesPerContender: Get<u32>;

		type ContractInvoker: Get<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		QueueFull,
		InvalidOwner,
		AlreadyRegistered,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		InstanceRegistered { contender: Contender, id: u32 },
		InstanceRemoved { contender: Contender, id: u32 },
		PlayStarted { contenders: (u32, u32) },
		PlayRoundResult { result: (U256, U256) },
		PlayEnded { winner: Option<u32> },
		GameResult { points: BTreeMap<u32, i32> },
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type ContractInfos<T: Config> = StorageMap<_, Twox64Concat, H160, ContractInfo, ValueQuery>;

	#[pallet::storage]
	pub type Contenders<T: Config> = StorageMap<_, Twox64Concat, u32, Contender, OptionQuery>;

	#[pallet::storage]
	pub type ContenderInstancesCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	pub type ContenderQueue<T: Config> = StorageValue<_, Vec<Contender>, ValueQuery>;

	#[pallet::storage]
	pub type NextContenderInstanceId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			if !(n % 10u8.into()).is_zero() {
				return 0;
			}

			let mut game_points = BTreeMap::<u32, i32>::new();

			let mut add_point = |id: u32, point: i32, contender: &Contender| {
				if !contender.is_contract() {
					return;
				}
				if let Some(p) = game_points.get_mut(&id) {
					*p = *p + point;
				} else {
					game_points.insert(id, point);
				}
			};

			for (id1, contender1) in Contenders::<T>::iter() {
				for (id2, contender2) in Contenders::<T>::iter() {
					if contender1.is_contract() {
						Self::deposit_event(Event::<T>::PlayStarted { contenders: (id1, id2) });
						let winner = Self::play(&contender1, &contender2);
						if let Some(winner) = winner {
							if winner == &contender1 {
								add_point(id1, 2, &contender1);
								add_point(id2, -1, &contender2);
								Self::deposit_event(Event::<T>::PlayEnded { winner: Some(id1) })
							} else {
								add_point(id1, -1, &contender1);
								add_point(id2, 2, &contender2);
								Self::deposit_event(Event::<T>::PlayEnded { winner: Some(id2) })
							}
						} else {
							add_point(id1, 1, &contender1);
							add_point(id2, 1, &contender2);
							Self::deposit_event(Event::<T>::PlayEnded { winner: None })
						}
					}
				}
			}

			let block_number: u128 = n.unique_saturated_into();
			let multipler = (block_number / 1800).saturating_sub(7200) + 1;
			for (id, point) in game_points.iter() {
				let contract = Contenders::<T>::get(id);
				if let Some(Contender::Contract(contract)) = contract {
					ContractInfos::<T>::mutate(contract, |info| {
						if *point > 0 {
							info.point = info.point + (*point as u128) * multipler;
						}
					});
				}
			}

			Self::deposit_event(Event::GameResult {
				points: game_points.clone(),
			});

			let mut results = game_points.into_iter().collect::<Vec<_>>();
			results.sort_by(|(_, a), (_, b)| b.cmp(a));

			ContenderQueue::<T>::mutate(|queue| {
				let max_count = T::EnqueueCount::get() as usize;
				if queue.len() < max_count {
					for c in queue.iter() {
						Self::add_contender(c, 2);
					}
					queue.clear();
				} else {
					let (to_add, new_queue) = queue.split_at(max_count);
					for c in to_add.iter() {
						Self::add_contender(c, 2);
					}
					*queue = new_queue.to_vec();
				}
			});

			for (winner, _) in results.iter().take(T::WinnerCount::get() as usize) {
				let contender = Contenders::<T>::get(winner);
				if let Some(contender) = contender {
					Self::add_contender(&contender, 1);
					if let Contender::Contract(ref contract) = contender {
						ContractInfos::<T>::mutate(contract, |info| {
							info.wins += 1;
						});
					}
				}
			}

			let instances_count = ContenderInstancesCount::<T>::get();
			let to_remove = instances_count.saturating_sub(T::MaxContenderInstancesCount::get());
			if to_remove > 0 {
				for (id, _) in results.iter().rev().take(to_remove as usize) {
					let contender = Contenders::<T>::take(id);
					if let Some(contender) = contender {
						if let Contender::Contract(ref contract) = contender {
							ContractInfos::<T>::mutate(contract, |info| {
								info.instance_count -= 1;
							});
						}
						Self::deposit_event(Event::<T>::InstanceRemoved { contender, id: *id });
					}
				}

				ContenderInstancesCount::<T>::put(instances_count - to_remove);
			}

			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		#[transactional]
		pub fn register(origin: OriginFor<T>, contract: H160) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				!ContractInfos::<T>::contains_key(&contract),
				Error::<T>::AlreadyRegistered
			);

			let input = Into::<u32>::into(Action::Owner).to_be_bytes().to_vec();
			let info = T::EVM::execute(
				InvokeContext {
					contract,
					sender: Default::default(),
					origin: Default::default(),
				},
				input,
				Default::default(),
				2_100_000,
				0,
				ExecutionMode::View,
			)?;
			let owner = H160::decode(&mut &info.value[12..]).map_err(|_| Error::<T>::InvalidOwner)?;
			let owner = T::AddressMapping::get_account_id(&owner);
			ensure!(owner == who, Error::<T>::InvalidOwner);

			Self::do_register_contender(Contender::Contract(contract))?;

			Ok(())
		}

		#[pallet::weight(0)]
		#[transactional]
		pub fn register_contender(origin: OriginFor<T>, contender: Contender, count: u32) -> DispatchResult {
			ensure_root(origin)?;

			Self::add_contender(&contender, count);

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn add_contender(contender: &Contender, mut count: u32) {
		if let Contender::Contract(contract) = contender {
			ContractInfos::<T>::mutate(contract, |info| {
				let max = T::MaxInstancesPerContender::get() - info.instance_count;
				count = count.min(max);
				info.instance_count += count;
			});
		}

		if count == 0 {
			return;
		}

		let id = NextContenderInstanceId::<T>::mutate(|id| {
			let current_id = *id;
			*id += count;
			current_id
		});

		for i in 0..count {
			let instance_id = id + i;
			Contenders::<T>::insert(instance_id, &contender);

			Self::deposit_event(Event::<T>::InstanceRegistered {
				contender: contender.clone(),
				id: instance_id,
			})
		}

		ContenderInstancesCount::<T>::mutate(|c| *c += count);
	}

	fn do_register_contender(contender: Contender) -> DispatchResult {
		if T::MaxContenderInstancesCount::get() > ContenderInstancesCount::<T>::get() {
			Self::add_contender(&contender, 2);
		} else {
			ContenderQueue::<T>::try_mutate(|queue| -> DispatchResult {
				if queue.len() >= T::MaxQueueSize::get() as usize {
					return Err(Error::<T>::QueueFull.into());
				}
				queue.push(contender);
				Ok(())
			})?;
		}

		Ok(())
	}

	fn contender_play(contender: &Contender, round: u32, prev_play: U256, other_prev_play: U256) -> U256 {
		match contender {
			Contender::Contract(contract) => {
				let mut input = Into::<u32>::into(Action::Play).to_be_bytes().to_vec();
				input.extend_from_slice(U256::from(round).encode().as_slice());
				input.extend_from_slice(prev_play.encode().as_slice());
				input.extend_from_slice(other_prev_play.encode().as_slice());
				let invoker = T::AddressMapping::get_default_evm_address(&T::ContractInvoker::get());
				let info = T::EVM::execute(
					InvokeContext {
						contract: contract.clone(),
						sender: invoker,
						origin: invoker,
					},
					input,
					Default::default(),
					10_000_000,
					64 * 20,
					ExecutionMode::Execute,
				);

				if let Ok(info) = info {
					log::warn!(target: "arena", "info2 {:?}", info.value);
					U256::from(info.value.as_slice())
				} else {
					U256::zero()
				}
			}
			Contender::AlwaysZero => U256::zero(),
			Contender::Rotate => U256::from(round),
			Contender::Smart => other_prev_play.div_mod(3u32.into()).1 + U256::from(2u32),
		}
	}

	fn play<'a>(a: &'a Contender, b: &'a Contender) -> Option<&'a Contender> {
		let mut prev_play_a = U256::zero();
		let mut prev_play_b = U256::zero();
		let mut point_a = 0u32;
		let mut point_b = 0u32;

		for round in 0..T::PlayPerRound::get() {
			let play_a = Self::contender_play(a, round, prev_play_a, prev_play_b);
			let play_b = Self::contender_play(b, round, prev_play_b, prev_play_a);

			Self::deposit_event(Event::<T>::PlayRoundResult {
				result: (play_a, play_b),
			});

			prev_play_a = play_a;
			prev_play_b = play_b;
			let play_a: u8 = play_a.div_mod(3.into()).1.unique_saturated_into();
			let play_b: u8 = play_b.div_mod(3.into()).1.unique_saturated_into();
			match (play_a, play_b) {
				(0, 1) | (1, 2) | (2, 0) => point_a += 1,
				(0, 2) | (2, 1) | (1, 0) => point_b += 1,
				_ => {}
			}
		}

		if point_a > point_b {
			Some(a)
		} else if point_a < point_b {
			Some(b)
		} else {
			None
		}
	}
}
