#![feature(const_trait_impl)]
#![feature(associated_type_bounds)]
#![feature(generic_const_exprs)]
#![feature(const_closures)]
#![feature(specialization)]
#![feature(const_default_impls)]
#![feature(adt_const_params)]
#![feature(const_type_id)]
#![feature(async_fn_in_trait)]
#![feature(macro_metavar_expr)]
#![feature(const_refs_to_cell)]
#![allow(incomplete_features)]
#![allow(type_alias_bounds)]

use traits::pubsub::Subscriber as _;

pub use metadata::Metadata;
pub use varuemb_notifier_proc::*;
pub use varuemb_utils::{const_wrapper, select};

pub mod calc;
pub mod event;
pub mod metadata;
pub mod pubsub;
pub mod rpc;
pub mod service;
pub mod traits;

mod assert {
    pub trait True {}

    pub struct Assert<const EXPR: bool>;
    impl True for Assert<true> {}

    pub struct AssertStr<const STR: &'static str> {}
    impl True for AssertStr<""> {}

    pub struct AssertNoop<const EXPR: bool>;
    impl<const EXPR: bool> True for AssertNoop<EXPR> {}
}

#[rustfmt::skip]
pub type GetPubSub<N, S: service::traits::Service<N>> = S::Impl;
#[rustfmt::skip]
pub type GetService<N: traits::Notifier, S: service::traits::Service<N>> = service::Service<N, S>;
pub type Duration = embassy_time::Duration;

pub const fn id<N, S>() -> usize
where
    N: traits::NotifierService<S>,
    S: service::traits::Service<N>,
{
    N::ID
}

pub const fn count<N, S>() -> usize
where
    N: traits::NotifierService<S>,
    S: service::traits::Service<N>,
{
    S::COUNT
}

const fn is_protected<P, E>() -> bool
where
    P: pubsub::traits::PubSub + 'static,
    E: pubsub::traits::IsPublisher<P>,
{
    use core::any::TypeId;
    use pubsub::traits::Publisher as _;
    E::Publisher::PROTECTED && TypeId::of::<P>() != TypeId::of::<E::Publisher>()
}

const fn is_pubsub_impl<S, N, E>() -> bool
where
    S: pubsub::traits::IsSubscribed<N, E>,
{
    S::IMPL
}

const fn is_pub_impl_and_count<S, N, E>(mut counts: (usize, usize)) -> (usize, usize)
where
    N: traits::NotifierService<S>,
    S: pubsub::traits::IsSubscribed<N, E> + service::traits::Service<N>,
{
    if is_pubsub_impl::<S, N, E>() {
        counts.0 += 1;
        counts.1 += count::<N, S>();
    }
    counts
}

type SubscriberCheckerRet<N, E> = (pubsub::GetDynSubscription<N, E>, usize);
fn subscriber_checker<S, N, E>(index: usize) -> SubscriberCheckerRet<N, E>
where
    N: traits::NotifierService<S> + traits::NotifierPublisher<E>,
    S: service::traits::Service<N, Impl: pubsub::traits::Subscriber<E>> + 'static,
    E: event::traits::Event<N>,
    E::Service: service::traits::Service<N, Impl: pubsub::traits::Publisher<E>>,
    [(); S::COUNT]:,
    [(); N::ID_COUNT]:,
    [(); N::COUNT_SERVICES]:,
{
    (
        pubsub::traits::GetPubSub::__get(N::get().__get(), index)
            .inner
            .__get(),
        count::<N, S>(),
    )
}

pub(crate) struct Subscriber<N, E>
where
    N: crate::traits::NotifierService<E::Service>,
    E: event::traits::Event<N>,
    E::Service: service::traits::Service<N, Impl: pubsub::traits::Publisher<E>>,
{
    pub(crate) index: usize,
    meta: &'static [Metadata],
    pub(crate) subscriber: pubsub::GetDynSubscription<N, E>,
}

impl<N, E> Subscriber<N, E>
where
    N: traits::NotifierService<E::Service>,
    E: event::traits::Event<N>,
    E::Service: service::traits::Service<N, Impl: pubsub::traits::Publisher<E>>,
{
    #[inline(always)]
    pub(crate) fn meta(&self) -> &'static Metadata {
        &self.meta[self.index]
    }
}

fn subscribers<N, E>() -> [Subscriber<N, E>; N::CHANNEL_COUNT]
where
    N: traits::NotifierPublisher<E>,
    E: event::traits::Event<N>,
    E::Service: service::traits::Service<N, Impl: pubsub::traits::Publisher<E>>,
    [(); N::ID_COUNT]:,
    [(); N::CHANNEL_COUNT]:,
    [(); N::COUNT_SERVICES]:,
{
    let mut id = 0;
    let mut offset = 0;
    core::array::from_fn(|mut index| {
        index -= offset;

        let checker = &N::ID_CALC.checkers[N::IDS[id]];
        let (subscriber, count) = (checker.checker)(index);

        let item = Subscriber {
            index,
            subscriber,
            meta: checker.meta,
        };
        if index + 1 >= count {
            id += 1;
            offset += count;
        }
        item
    })
}
