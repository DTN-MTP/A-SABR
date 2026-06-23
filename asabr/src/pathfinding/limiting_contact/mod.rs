#![cfg(feature = "contact_suppression")]


use crate::bundle::Bundle;
use crate::contact::Contact;
use crate::contact_manager::ContactManager;
use crate::multigraph::{ContactRef, Multigraph};
use crate::node_manager::NodeManager;
use crate::pathfinding::PathFindingOutput;

extern crate alloc;

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
pub fn get_next_to_suppress<
    'id,
    'a,
    NM: NodeManager,
    CM: ContactManager,
>(
    graph: &Multigraph<'id, NM, CM>,
    paths: &PathFindingOutput<'id, 'a>,
    bundle: &Bundle,
    better_for_suppression_than_fn: fn(&Contact<CM>, &Contact<CM>) -> bool,
) -> Option<ContactRef<'id>> {
    let mut to_suppress_opt: Option<ContactRef> = None;
    let mut next_route_option = paths[bundle.destinations[0] as usize];
    while let Some(curr_route) = next_route_option.take() {
        {
            if let Some(ref via) = curr_route.via {
                match to_suppress_opt {
                    Some(ref to_suppress) => {
                        if better_for_suppression_than_fn(&graph[via.contact], &graph[to_suppress])
                        {
                            to_suppress_opt = Some(via.contact.clone());
                        }
                    }
                    None => to_suppress_opt = Some(via.contact.clone()),
                }
                next_route_option = paths[via.parent_frag];
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
/// * `suppression_map` - A map of contacts that are suppressed before the pathfinding stage.
///
/// The struct implements the `Pathfinding` trait, using the specified suppression strategy to
/// modify its behavior when selecting the next route. The `suppression_map` contact is removed before
/// tree construction, and prepares the new `next_to_suppress` contact.

#[macro_export]
macro_rules! create_new_alternative_path_variant {
    ($struct_name:ident, $better_fn:ident) => {
        /// An alternative path finding algorithm (macro generated).
        ///
        /// Each time a new route is generated, a contact of the last found route is suppressed.
        #[doc = concat!("`", stringify!($struct_name), "` uses the `", stringify!($better_fn), "` function to select the next contact to suppress.")]
        /// This is macro generated check the documentation of `create_new_alternative_path_variant` for details.
        ///
        /// # Type Parameters
        ///
        /// * `NM` - A type that implements the `NodeManager` trait.
        /// * `CM` - A type that implements the `ContactManager` trait.
        /// * `D` - A type that implements the `Distance<NM, CM>` trait.
        pub struct $struct_name<'id,
            NM: $crate::node_manager::NodeManager,
            CM: ContactManager,
            P: $crate::pathfinding::Pathfinding<'id,NM, CM>,
        > {
            /// The underlying pathfinding algorithm used to find individual paths.
            pathfinding: P,
            suppression_map: alloc::vec::Vec<alloc::vec::Vec<$crate::multigraph::ContactRef<'id>>>,

            #[doc(hidden)]
            _phantom: core::marker::PhantomData<(NM,CM)>,
            id: $crate::utils::Id<'id>,
        }

        impl<'id,
                NM: $crate::node_manager::NodeManager,
                CM: $crate::contact_manager::ContactManager,
                P: $crate::pathfinding::Pathfinding<'id,NM, CM>,
            > $crate::pathfinding::Pathfinding<'id,NM, CM> for $struct_name<'id,NM, CM, P>
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
            fn new<'id2>(
                guard: $crate::utils::Guard<'id2>,
                multigraph: &$crate::multigraph::Multigraph<'id,NM, CM>,
            ) -> Self {
                let node_count = multigraph.get_vertex_count();
                Self {

                    pathfinding: P::new(guard,multigraph),
                    suppression_map: alloc::vec![alloc::vec::Vec::new(); node_count],
                    _phantom: core::marker::PhantomData,
                    id: multigraph.id(),
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
            /// * `Result<PathFindingOutput<NM, CM>, ASABRError>` - The resulting pathfinding output, including the routes found.
            fn find_path<'a>(
                &'a mut self,
                multigraph: &mut $crate::multigraph::Multigraph<'id, NM, CM>,
                current_time: $crate::types::Date,
                source: $crate::multigraph::NodeRef<'id>,
                bundle: &$crate::bundle::Bundle,
            ) -> Result<Option<$crate::pathfinding::PathFindingOutput<'id,'a>>, $crate::errors::ASABRError> {

                let contacts = &mut self.suppression_map[bundle.destinations[0] as usize];
                contacts.retain(|ct| multigraph[ct].lifespan.end > current_time);
                for ct in contacts {
                    multigraph[*ct].suppressed = true;
                }
                let path_opt = self
                    .pathfinding
                    .find_path(multigraph,current_time,source,bundle)?;

                if let Some(path) = &path_opt {
                    if let Some(contact) = $crate::pathfinding::limiting_contact::get_next_to_suppress(multigraph,path,bundle, $better_fn) {
                        self.suppression_map[bundle.destinations[0] as usize].push(contact);
                    }
                }
                for contact in &self.suppression_map[bundle.destinations[0] as usize] {
                    multigraph[contact].suppressed = false;
                }

                return Ok(path_opt);
            }


        }
    };
}
