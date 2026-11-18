//! Manual forwarding of a pinned receiver in three forms from the delegation RFC.
//! The projection example uses the external `pin-project` crate.

use std::{cell::Cell, marker::PhantomPinned, pin::Pin};

// A shared !Unpin target for both structural projection examples.
struct PinnedInner {
    total: Cell<u32>,
    _pin: PhantomPinned,
}

impl PinnedInner {
    fn add(self: Pin<&mut Self>, amount: u32) -> u32 {
        let total = self.total.get() + amount;
        self.total.set(total);
        total
    }
}

mod unpin {
    use super::*;

    struct Inner(u32);
    struct Wrapper(Inner);

    impl Inner {
        fn add(self: Pin<&mut Self>, amount: u32) -> u32 {
            let inner = self.get_mut();
            inner.0 += amount;
            inner.0
        }
    }

    fn delegation_map<'a, T: Unpin, U: Unpin>(
        receiver: Pin<&'a mut T>,
        target: impl FnOnce(&'a mut T) -> &'a mut U,
    ) -> Pin<&'a mut U> {
        Pin::new(target(receiver.get_mut()))
    }

    impl Wrapper {
        // A. The target block computes the entire Pin<&mut Inner>.
        fn add_a(self: Pin<&mut Self>, amount: u32) -> u32 {
            let target = Pin::new(&mut self.get_mut().0);
            Inner::add(target, amount)
        }

        // B simple. Evaluate the field once and wrap the resulting borrow.
        fn add_b_simple(self: Pin<&mut Self>, amount: u32) -> u32 {
            let field = &mut self.get_mut().0;
            Inner::add(Pin::new(field), amount)
        }

        // B advanced. The mapper owns the pin conversion and calls the selector.
        fn add_b_advanced(self: Pin<&mut Self>, amount: u32) -> u32 {
            Inner::add(delegation_map(self, |this| &mut this.0), amount)
        }
    }

    pub(super) fn run() {
        let mut wrapper = Wrapper(Inner(0));
        let mut wrapper = Pin::new(&mut wrapper);
        assert_eq!(wrapper.as_mut().add_a(2), 2);
        assert_eq!(wrapper.as_mut().add_b_simple(3), 5);
        assert_eq!(wrapper.as_mut().add_b_advanced(4), 9);
    }
}

mod manual_pin {
    use super::PinnedInner as Inner;
    use super::*;

    // Field 0 is structurally pinned. Wrapper inherits !Unpin from Inner and
    // uses ordinary field drop. Keep it unpacked and preserve field 0 in place
    // until drop whenever Wrapper is pinned.
    struct Wrapper(Inner);

    impl Wrapper {
        // A. The target block performs the complete structural projection.
        fn add_a(self: Pin<&mut Self>, amount: u32) -> u32 {
            // SAFETY: the closure borrows field 0 in place.
            let target = unsafe { self.map_unchecked_mut(|this| &mut this.0) };
            Inner::add(target, amount)
        }

        // B simple. Select the field once, then restore its pin.
        fn add_b_simple(self: Pin<&mut Self>, amount: u32) -> u32 {
            // SAFETY: the unpinned borrow is used only to reach field 0, and
            // field 0 inherits the pinning guarantee of Wrapper.
            let target = unsafe {
                let field = &mut self.get_unchecked_mut().0;
                Pin::new_unchecked(field)
            };
            Inner::add(target, amount)
        }

        // B advanced. map_unchecked_mut supplies the mapping operation.
        fn add_b_advanced(self: Pin<&mut Self>, amount: u32) -> u32 {
            // SAFETY: the selector returns a borrow of the structurally pinned
            // field without moving anything out of Wrapper.
            Inner::add(
                unsafe { self.map_unchecked_mut(|this| &mut this.0) },
                amount,
            )
        }
    }

    pub(super) fn run() {
        let mut wrapper = std::pin::pin!(Wrapper(Inner {
            total: Cell::new(0),
            _pin: PhantomPinned,
        }));
        assert_eq!(wrapper.as_mut().add_a(2), 2);
        assert_eq!(wrapper.as_mut().add_b_simple(3), 5);
        assert_eq!(wrapper.as_mut().add_b_advanced(4), 9);
    }
}

mod with_pin_project {
    use super::PinnedInner as Inner;
    use super::*;
    use pin_project::pin_project;

    #[pin_project(project = WrapperProjection)]
    struct Wrapper(#[pin] Inner);

    impl Wrapper {
        // A. The target block computes the pinned receiver directly.
        fn add_a(self: Pin<&mut Self>, amount: u32) -> u32 {
            let target = self.project().0;
            Inner::add(target, amount)
        }

        // B simple. Set up the projection, then select the field once.
        // The generated projection already carries the pin on field 0.
        fn add_b_simple(self: Pin<&mut Self>, amount: u32) -> u32 {
            let projected = self.project();
            Inner::add(projected.0, amount)
        }

        // B advanced. The selector runs on the generated projection, whose
        // field type encodes the pinning guarantee.
        fn add_b_advanced(self: Pin<&mut Self>, amount: u32) -> u32 {
            Inner::add(self.delegation_map(|this| this.0), amount)
        }

        fn delegation_map<'a>(
            self: Pin<&'a mut Self>,
            target: impl FnOnce(WrapperProjection<'a>) -> Pin<&'a mut Inner>,
        ) -> Pin<&'a mut Inner> {
            target(self.project())
        }
    }

    pub(super) fn run() {
        let mut wrapper = std::pin::pin!(Wrapper(Inner {
            total: Cell::new(0),
            _pin: PhantomPinned,
        }));
        assert_eq!(wrapper.as_mut().add_a(2), 2);
        assert_eq!(wrapper.as_mut().add_b_simple(3), 5);
        assert_eq!(wrapper.as_mut().add_b_advanced(4), 9);
    }
}

fn main() {
    unpin::run();
    manual_pin::run();
    with_pin_project::run();
}
