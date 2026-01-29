use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::node_manager::NodeManager;
use crate::route_stage::SharedRouteStage;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "first_depleted")]
pub mod first_depleted;
pub mod first_ending;
#[cfg(feature = "first_depleted")]
pub use first_depleted::FirstDepleted;

pub use first_ending::FirstEnding;

/// Retrieves the next `Contact` to suppress based on the provided suppression function.
///
/// This function navigates through the provided route stage to identify the `Contact` that
/// is best suited for suppression, according to the specified comparison function
/// (`better_for_suppression_than_fn`). It iterates through the route's contacts to determine
/// the one that should be suppressed next.
///
/// # Parameters
///
/// * `route` - A reference-counted, mutable `RouteStage` representing the current routing stage.
/// * `better_for_suppression_than_fn` - A function pointer used to compare two `Contact`s and
///   determine which is better for suppression.
///
/// # Returns
///
/// An `Option` containing a reference-counted, mutable `Contact` that should be suppressed, if one
/// is found; otherwise, `None`.
#[cfg(feature = "contact_suppression")]
pub fn get_next_to_suppress<NM: NodeManager, CM: ContactManager>(
    route: SharedRouteStage<NM, CM>,
    better_for_suppression_than_fn: fn(&Contact<NM, CM>, &Contact<NM, CM>) -> bool,
) -> Option<Rc<RefCell<Contact<NM, CM>>>> {
    let mut to_suppress_opt: Option<Rc<RefCell<Contact<NM, CM>>>> = None;
    let mut next_route_option = Some(route);
    while let Some(curr_route) = next_route_option.take() {
        {
            let route_borrowed = curr_route.borrow();
            if let Some(ref via) = route_borrowed.via {
                match to_suppress_opt {
                    Some(ref to_suppress) => {
                        if better_for_suppression_than_fn(
                            &via.contact.borrow(),
                            &to_suppress.borrow(),
                        ) {
                            to_suppress_opt = Some(via.contact.clone());
                        }
                    }
                    None => to_suppress_opt = Some(via.contact.clone()),
                }
                next_route_option = Some(Rc::clone(&via.parent_route));
            }
        }
    }
    to_suppress_opt
}

/// Creates a new variant of the alternative pathfinding algorithm with a custom suppression strategy.
///
/// This macro generates a new struct that implements the `Pathfinding` trait, adding the ability to
/// suppress specific contacts during the routing process. The suppression logic is determined by the
/// provided `better_fn`, which compares `Contact`s to decide which should be suppressed.
///
/// # Parameters
///
/// * `$struct_name` - The name of the struct to be created.
/// * `$better_fn` - The name of the function used to compare two `Contact`s and determine which one
///   is better for suppression.
///
/// # Generated Struct
///
/// The generated struct will contain the following fields:
/// * `pathfinding` - An instance of the underlying pathfinding algorithm.
/// * `next_to_suppress` - An optional `Contact` that will be suppressed before the pathfinding stage.
///
/// The struct implements the `Pathfinding` trait, using the specified suppression strategy to
/// modify its behavior when selecting the next route. The `next_to_suppress` contact is removed before
/// tree construction, and prepare the new `next_to_suppress` contact.
#[cfg(feature = "contact_suppression")]
#[macro_export]
macro_rules! create_new_alternative_path_variant {
    ($struct_name:ident, $better_fn:ident) => {
        /// An alternative path finding algorithm (macro generated).
        ///
        /// Each time a new route must generated, a contact of the last found route is suppressed.
        #[doc = concat!("`", stringify!($struct_name), "` uses the `", stringify!($better_fn), "` function to select the next contact to suppress.")]
        /// This is macro generated check the documentation of `create_new_alternative_path_variant` for details.
        ///
        /// # Type Parameters
        ///
        /// * `NM` - A type that implements the `NodeManager` trait.
        /// * `CM` - A type that implements the `ContactManager` trait.
        /// * `D` - A type that implements the `Distance<NM, CM>` trait.
        /// * `P` - A type that implements the `Pathfinding<NM, CM>` trait.
        pub struct $struct_name<
            NM: crate::node_manager::NodeManager,
            CM: ContactManager,
            P: crate::pathfinding::Pathfinding<NM, CM>,
        > {
            /// The underlying pathfinding algorithm used to find individual paths.
            pathfinding: P,
            suppression_map: Vec<Vec<std::rc::Rc<std::cell::RefCell<Contact<NM, CM>>>>>,

            #[doc(hidden)]
            _phantom_nm: std::marker::PhantomData<NM>,
            #[doc(hidden)]
            _phantom_cm: std::marker::PhantomData<CM>,
        }

        impl<
                NM: crate::node_manager::NodeManager,
                CM: ContactManager,
                P: crate::pathfinding::Pathfinding<NM, CM>,
            > crate::pathfinding::Pathfinding<NM, CM> for $struct_name<NM, CM, P>
        {
            #[doc = concat!("Constructs a new `", stringify!($struct_name), "` instance with the provided nodes and contacts.")]
            ///
            /// Generated with a macro, check the macro documentation for details.
            ///
            /// # Parameters
            ///
            /// * `multigraph` - A shared pointer to a multigraph.
            ///
            /// # Returns
            ///
            #[doc = concat!("* `Self` - A new instance of `", stringify!($struct_name), "`.")]
            fn new(
                multigraph: std::rc::Rc<std::cell::RefCell<crate::multigraph::Multigraph<NM, CM>>>
            ) -> Self {
                let node_count = multigraph.borrow().get_node_count();
                Self {

                    pathfinding: P::new(multigraph),
                    suppression_map: vec![Vec::new(); node_count],
                    _phantom_nm: std::marker::PhantomData,
                    _phantom_cm: std::marker::PhantomData,
                }
            }
            /// Finds the next route based on the current state and available contacts.
            ///
            /// # Parameters
            ///
            /// * `current_time` - The current time used for evaluating routes.
            /// * `source` - The `NodeID` of the source node from which to begin pathfinding.
            /// * `bundle` - The `Bundle` associated with the pathfinding operation.
            /// * `excluded_nodes_sorted` - A list of `NodeID`s to be excluded from the pathfinding.
            ///
            /// # Returns
            ///
            /// * `PathfindingOutput<CM>` - The resulting pathfinding output, including the routes found.
            fn get_next(
                &mut self,
                current_time: crate::types::Date,
                source: crate::types::NodeID,
                bundle: &crate::bundle::Bundle,
                excluded_nodes_sorted: &[crate::types::NodeID],
            ) -> Result<crate::pathfinding::PathFindingOutput<NM, CM>, crate::errors::ASABRError> {

                let contacts = &mut self.suppression_map[bundle.destinations[0] as usize];
                let mut i = 0;
                while i < contacts.len() {
                    let mut contact_borrowed = contacts[i].try_borrow_mut()?;

                    let should_remove = contact_borrowed.info.end < current_time;
                    if !should_remove {
                        contact_borrowed.suppressed = true;
                    }
                    drop(contact_borrowed);

                    if should_remove {
                        contacts.remove(i);
                    } else {
                        i += 1;
                    }
                }

                let tree = self
                    .pathfinding
                    .get_next(current_time, source, bundle, excluded_nodes_sorted)?;

                if let Some(route) = tree.by_destination[bundle.destinations[0] as usize].clone() {
                    if let Some(contact) = crate::pathfinding::limiting_contact::get_next_to_suppress(route, $better_fn) {
                        self.suppression_map[bundle.destinations[0] as usize].push(contact);
                    }
                }
                for contact in &self.suppression_map[bundle.destinations[0] as usize] {
                    contact.try_borrow_mut()?.suppressed = false;
                }

                return Ok(tree);
            }

            /// Get a shared pointer to the multigraph.
            ///
            /// # Returns
            ///
            /// * A shared pointer to the multigraph.
            fn get_multigraph(&self) -> std::rc::Rc<std::cell::RefCell<crate::multigraph::Multigraph<NM, CM>>> {
                return self.pathfinding.get_multigraph();
            }
        }
    };
}
