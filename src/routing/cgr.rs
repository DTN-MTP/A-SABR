use crate::{
    bundle::Bundle,
    contact::Contact,
    contact_manager::ContactManager,
    multigraph::Multigraph,
    node::Node,
    node_manager::NodeManager,
    pathfinding::Pathfinding,
    route_stage::RouteStage,
    route_storage::{Route, RouteStorage},
    types::{Date, NodeID},
};

use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use super::{dry_run_unicast_path_with_exclusions, schedule_unicast_path, Router, RoutingOutput};

pub struct Cgr<NM: NodeManager, CM: ContactManager, P: Pathfinding<NM, CM>, S: RouteStorage<NM, CM>>
{
    route_storage: Rc<RefCell<S>>,
    pathfinding: P,

    // for compilation
    #[doc(hidden)]
    _phantom_nm: PhantomData<NM>,
    #[doc(hidden)]
    _phantom_cm: PhantomData<CM>,
}

impl<NM: NodeManager, CM: ContactManager, P: Pathfinding<NM, CM>, S: RouteStorage<NM, CM>>
    Router<NM, CM> for Cgr<NM, CM, P, S>
{
    fn route(
        &mut self,
        source: NodeID,
        bundle: &Bundle,
        curr_time: Date,
        excluded_nodes: &Vec<NodeID>,
    ) -> Option<RoutingOutput<NM, CM>> {
        if bundle.destinations.len() == 1 {
            return self.route_unicast(source, bundle, curr_time, excluded_nodes);
        }

        todo!();
    }
}

impl<S: RouteStorage<NM, CM>, NM: NodeManager, CM: ContactManager, P: Pathfinding<NM, CM>>
    Cgr<NM, CM, P, S>
{
    pub fn new(
        nodes: Vec<Node<NM>>,
        contacts: Vec<Contact<NM, CM>>,
        route_storage: Rc<RefCell<S>>,
    ) -> Self {
        Self {
            pathfinding: P::new(Rc::new(RefCell::new(Multigraph::new(nodes, contacts)))),
            route_storage: route_storage.clone(),
            // for compilation
            _phantom_nm: PhantomData,
            _phantom_cm: PhantomData,
        }
    }

    fn route_unicast(
        &mut self,
        source: NodeID,
        bundle: &Bundle,
        curr_time: Date,
        excluded_nodes: &Vec<NodeID>,
    ) -> Option<RoutingOutput<NM, CM>> {
        let dest = bundle.destinations[0];
        let mut bundle_no_constraints = bundle.clone();
        bundle_no_constraints.priority = 1;
        bundle_no_constraints.size = 0.0;

        {
            self.pathfinding
                .get_multigraph()
                .borrow_mut()
                .apply_exclusions_sorted(excluded_nodes);
        }

        let route_option = self.route_storage.borrow_mut().select(
            bundle,
            curr_time,
            &self.pathfinding.get_multigraph().borrow_mut().nodes,
            excluded_nodes,
        );

        if let Some(route) = route_option {
            return Some(schedule_unicast_path(
                bundle,
                curr_time,
                route.source_stage.clone(),
                &self.pathfinding.get_multigraph().borrow_mut().nodes,
            ));
        }

        loop {
            let new_tree = self.pathfinding.get_next(
                curr_time,
                source,
                &bundle_no_constraints,
                excluded_nodes,
            );
            let tree = Rc::new(RefCell::new(new_tree));

            if let Some(route) = Route::from_tree(tree, dest) {
                RouteStage::init_route(route.destination_stage.clone());
                self.route_storage
                    .borrow_mut()
                    .store(&bundle, route.clone());

                let dry_run = dry_run_unicast_path_with_exclusions(
                    bundle,
                    curr_time,
                    route.source_stage.clone(),
                    route.destination_stage.clone(),
                    &self.pathfinding.get_multigraph().borrow_mut().nodes,
                );

                if let Some(_) = dry_run {
                    return Some(schedule_unicast_path(
                        bundle,
                        curr_time,
                        route.source_stage.clone(),
                        &self.pathfinding.get_multigraph().borrow_mut().nodes,
                    ));
                }
            } else {
                break;
            }
        }
        None
    }
}
