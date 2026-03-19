use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    errors::ASABRError,
    multigraph::Multigraph,
    node_manager::NodeManager,
    pathfinding::Pathfinding,
    route_stage::RouteStage,
    route_storage::{Route, RouteStorage},
    types::{Date, NodeID},
};

use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use super::{Router, RoutingOutput, dry_run_unicast_path, schedule_unicast_path};

pub struct VolCgr<
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<NM, CM>,
    S: RouteStorage<NM, CM>,
> {
    route_storage: Rc<RefCell<S>>,
    pathfinding: P,

    // for compilation
    #[doc(hidden)]
    _phantom_nm: PhantomData<NM>,
    #[doc(hidden)]
    _phantom_cm: PhantomData<CM>,
}

impl<NM: NodeManager, CM: ContactManager, P: Pathfinding<NM, CM>, S: RouteStorage<NM, CM>>
    Router<NM, CM> for VolCgr<NM, CM, P, S>
{
    fn route(
        &mut self,
        source: NodeID,
        bundle: &Bundle,
        curr_time: Date,
        excluded_nodes: &[NodeID],
    ) -> Result<Option<RoutingOutput<NM, CM>>, ASABRError> {
        if bundle.expiration < curr_time {
            return Ok(None);
        }

        if bundle.destinations.len() == 1 {
            return self.route_unicast(source, bundle, curr_time, excluded_nodes);
        }

        Err(ASABRError::MulticastUnsupportedError)
    }
}

impl<S: RouteStorage<NM, CM>, NM: NodeManager, CM: ContactManager, P: Pathfinding<NM, CM>>
    VolCgr<NM, CM, P, S>
{
    pub fn new(
        contact_plan: ContactPlan<NM, NM, CM>,
        route_storage: Rc<RefCell<S>>,
    ) -> Result<Self, ASABRError> {
        Ok(Self {
            pathfinding: P::new(Rc::new(RefCell::new(Multigraph::new(contact_plan)?))),
            route_storage: route_storage.clone(),
            // for compilation
            _phantom_nm: PhantomData,
            _phantom_cm: PhantomData,
        })
    }

    fn route_unicast(
        &mut self,
        source: NodeID,
        bundle: &Bundle,
        curr_time: Date,
        excluded_nodes: &[NodeID],
    ) -> Result<Option<RoutingOutput<NM, CM>>, ASABRError> {
        let dest = bundle.destinations[0];

        let route_option = self.route_storage.try_borrow_mut()?.select(
            bundle,
            curr_time,
            self.pathfinding.get_multigraph().clone(),
            excluded_nodes,
        )?;

        if let Some(route) = route_option {
            return Ok(Some(schedule_unicast_path(
                bundle,
                curr_time,
                route.source_stage.clone(),
            )?));
        }

        let new_tree = self
            .pathfinding
            .get_next(curr_time, source, bundle, excluded_nodes)?;
        let tree = Rc::new(RefCell::new(new_tree));

        let Some(route) = Route::from_tree(tree, dest) else {
            return Ok(None);
        };
        RouteStage::init_route(route.destination_stage.clone())?;
        self.route_storage
            .try_borrow_mut()?
            .store(bundle, route.clone());
        let dry_run = dry_run_unicast_path(bundle, curr_time, route.source_stage.clone(), true)?;
        if dry_run.is_some() {
            return Ok(Some(schedule_unicast_path(
                bundle,
                curr_time,
                route.source_stage.clone(),
            )?));
        }
        Ok(None)
    }
}
