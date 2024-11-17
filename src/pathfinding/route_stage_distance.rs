use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use crate::{contact_manager::ContactManager, distance::Distance, route_stage::RouteStage};

/// A helper structure for providing ordering of Rc<RefCell<RouteStage<CM>>>
/// using custom RouteStage ordering defined by the trait Distance<CM>.
pub struct DistanceWrapper<CM: ContactManager, D: Distance<CM>>(
    pub Rc<RefCell<RouteStage<CM>>>,
    #[doc(hidden)] pub PhantomData<D>,
);

impl<CM: ContactManager, D: Distance<CM>> DistanceWrapper<CM, D> {
    pub fn new(route_stage: Rc<RefCell<RouteStage<CM>>>) -> Self {
        Self(route_stage, PhantomData)
    }
}

impl<CM: ContactManager, D: Distance<CM>> Ord for DistanceWrapper<CM, D> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        D::cmp(&self.0.borrow(), &other.0.borrow())
    }
}

impl<CM: ContactManager, D: Distance<CM>> PartialOrd for DistanceWrapper<CM, D> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<CM: ContactManager, D: Distance<CM>> PartialEq for DistanceWrapper<CM, D> {
    fn eq(&self, other: &Self) -> bool {
        D::eq(&self.0.borrow(), &other.0.borrow())
    }
}

impl<CM: ContactManager, D: Distance<CM>> Eq for DistanceWrapper<CM, D> {}
