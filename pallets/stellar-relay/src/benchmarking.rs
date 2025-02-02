//! Benchmarking setup for pallet-template

use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use sp_std::vec;

#[allow(unused)]
use crate::Pallet as StellarRelay;
use crate::{
	traits::{FieldLength, Organization, Validator},
	types::{OrganizationOf, ValidatorOf},
};

use super::*;

benchmarks! {
	update_tier_1_validator_set {
		let caller: T::AccountId = whitelisted_caller();

		let bounded_vec = BoundedVec::<u8, FieldLength>::default();

		let validator: ValidatorOf<T> = Validator {
			name: bounded_vec.clone(),
			public_key: bounded_vec.clone(),
			organization_id: T::OrganizationId::default(),
			public_network: false
		};

		let validators = vec![validator; 255];

		let organization: OrganizationOf<T> = Organization {
			id: T::OrganizationId::default(),
			name: bounded_vec.clone(),
			public_network: false
		};

		let organizations = vec![organization; 255];

	}: update_tier_1_validator_set(RawOrigin::Root, validators.clone(), organizations.clone())
	verify {
		assert_eq!(Organizations::<T>::get(), BoundedVec::<OrganizationOf<T>, T::OrganizationLimit>::try_from(organizations).unwrap());
		assert_eq!(Validators::<T>::get(), BoundedVec::<ValidatorOf<T>, T::ValidatorLimit>::try_from(validators).unwrap());
	}

	impl_benchmark_test_suite!(StellarRelay, crate::mock::new_test_ext(), crate::mock::Test);
}
