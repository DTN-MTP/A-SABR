extern crate alloc;
use crate::{
    bundle::Bundle,
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    errors::ASABRError,
    multigraph::{INodeRef, Multigraph},
    node_manager::NodeManager,
    pathfinding::{
        PathFindingOutput, Pathfinding,
        destination::{FindableDest, RoutableDest},
    },
    types::Date,
};

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// Re-exports generativity utilities used by graph-construction macros.
pub use generativity::{Guard, Id, make_guard};
/// Builds a `Multigraph` from ASABR contact-plan content.
///
/// This macro creates a generativity guard, parses the contact plan, and binds
/// the resulting graph to the provided variable name.
///
/// Usage:
///
/// ```ignore
/// mk_graph!(graph, NoManagement, CMDynStandard, lines);
/// mk_graph!(graph, NoManagement, CMDynStandard, raw_content, raw);
/// mk_graph!(graph, NoManagement, CMDynStandard, filename, file);
/// ```
///
/// The optional content mode specifies how the input is supplied:
///
/// - `iterator`: an iterator over contact-plan lines. This is the default.
/// - `raw`: an `&str` containing the whole contact-plan content.
/// - `file`: a file path to open and parse. This requires `std`.
#[macro_export]
macro_rules! mk_graph {
    ($graph:ident,$NM:ty,$CM:ty,$content:expr$(,iterator)?) => {
        $crate::utils::make_guard!($graph);
        #[allow(unused_mut)]
        let mut $graph = $crate::multigraph::Multigraph::new(
            $graph,
            $crate::contact_plan::asabr_file_lexer::parse_from_iter::<$NM, $CM>($content)?,
        )?;
    };
    ($graph:ident,$NM:ty,$CM:ty,$content:expr,raw) => {
        $crate::mk_graph!($graph, $NM, $CM, $content.lines());
    };
    ($graph:ident,$NM:ty,$CM:ty,$content:expr,file) => {
        $crate::mk_graph!($graph, $NM, $CM, {
            use std::io::{BufRead, BufReader};
            std::io::BufReader::new(match std::fs::File::open($content) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error while trying to open file: {e}");
                    return Err($crate::errors::ASABRError::ParsingError(
                        $crate::parsing::Located {
                            data: "Error while opennig file",
                            line: 0,
                            toknum: 0,
                        },
                    ));
                }
            })
            .lines()
            .map(|l| {
                l.map_err(|e| {
                    eprintln!("Error while reading file: {e}");
                    panic!();
                })
                .unwrap()
            })
        });
    };
}

impl<'id, NM, CM, D, T> Pathfinding<'id, NM, CM, D> for alloc::boxed::Box<T>
where
    NM: NodeManager,
    CM: ContactManager,
    D: FindableDest<'id, NM, CM>,
    T: Pathfinding<'id, NM, CM, D> + ?Sized,
{
    fn find_path(
        &mut self,
        multigraph: &mut Multigraph<'id, NM, CM>,
        routing_time: Date,
        source: INodeRef<'id>,
        bundle: &Bundle,
        destination: &mut D,
        prune_time: Option<Date>,
    ) -> Result<Option<PathFindingOutput<'id, '_>>, ASABRError> {
        (**self).find_path(
            multigraph,
            routing_time,
            source,
            bundle,
            destination,
            prune_time,
        )
    }
}

pub struct Router<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
> {
    pub multigraph: Multigraph<'id, NM, CM>,
    pub pathfinder: P,
    _phantom: PhantomData<fn(D)>,
}

impl<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D>,
    D: RoutableDest<'id, NM, CM>,
> Router<'id, NM, CM, P, D>
{
    pub fn build<T>(
        guard: Guard<'id>,
        contact_plan: ContactPlan<NM, CM>,
        pathfinder_args: T,
    ) -> Result<Self, ASABRError>
    where
        for<'a> (&'a Multigraph<'id, NM, CM>, T): Into<P>,
    {
        let multigraph = Multigraph::new(guard, contact_plan)?;
        let pathfinder = (&multigraph, pathfinder_args).into();
        Ok(Self {
            multigraph,
            pathfinder,
            _phantom: PhantomData,
        })
    }
    pub fn new(multigraph: Multigraph<'id, NM, CM>, pathfinder: P) -> Self {
        Self {
            multigraph,
            pathfinder,
            _phantom: PhantomData,
        }
    }
    /// # Safety
    /// see Multigraph::new_unguarder
    pub unsafe fn build_unguarded<T>(
        contact_plan: ContactPlan<NM, CM>,
        pathfinder_args: T,
    ) -> Result<Self, ASABRError>
    where
        for<'a> (&'a Multigraph<'id, NM, CM>, T): Into<P>,
    {
        Self::build(
            unsafe { Guard::new(Id::new()) },
            contact_plan,
            pathfinder_args,
        )
    }
    pub fn find_path(
        &mut self,
        mut destination: D,
        routing_time: Date,
        source: INodeRef<'id>,
        bundle: &Bundle,
        prune_time: Option<Date>,
    ) -> Result<Option<PathFindingOutput<'id, '_>>, ASABRError> {
        self.pathfinder.find_path(
            &mut self.multigraph,
            routing_time,
            source,
            bundle,
            &mut destination,
            prune_time,
        )
    }
    pub fn route<'a>(
        &'a mut self,
        mut destination: D,
        routing_time: Date,
        source: INodeRef<'id>,
        bundle: &Bundle,
        prune_time: Option<Date>,
    ) -> Result<Option<D::RoutingOutput<'a>>, ASABRError> {
        destination.route(
            &mut self.multigraph,
            bundle,
            &mut self.pathfinder,
            routing_time,
            source,
            prune_time,
        )
    }
}

impl<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
> Deref for Router<'id, NM, CM, P, D>
{
    type Target = Multigraph<'id, NM, CM>;
    fn deref(&self) -> &Self::Target {
        &self.multigraph
    }
}

impl<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
> DerefMut for Router<'id, NM, CM, P, D>
{
    fn deref_mut(&mut self) -> &mut <Router<'id, NM, CM, P, D> as Deref>::Target {
        &mut self.multigraph
    }
}

/// Builds a `Router` from an already-parsed `ContactPlan`.
///
/// `$algo` is the router name used to select the concrete pathfinding
/// implementation.
///
/// Example:
///
/// ```ignore
/// mk_router!(
///     router,
///     NoManagement,
///     CMDynStandard,
///     "SpsnHybridParenting",
///     contact_plan
/// );
/// ```
///
/// The resulting router has the concrete type:
///
/// ```ignore
/// Router<
///     NM,
///     CM,
///     Box<dyn Pathfinding<'id, NM, CM, RoutableNodeRef<'id>> + 'id>,
///     RoutableNodeRef<'id>,
/// >
/// ```
/// Builds a `Router` from an already-constructed `Multigraph`.
/// Evaluates to `PyResult<Router<...>>` directly compatible with PyO3.
#[macro_export]
macro_rules! mk_router {
    (
        $id:ident,$NM:ty,
        $CM:ty,$prio_count:expr,
        $algo:expr,$multigraph:expr
    ) => {{
        // Alias for the dynamic Trait Object type to coerce match arms
        type TraitObj<'a> = Box<
            dyn $crate::pathfinding::Pathfinding<
                    'a,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'a>,
                > + 'a,
        >;

        let pathfinder: TraitObj<'_> = match $algo {
            // ============================================================
            // SPSN - SABR
            // ============================================================
            "SpsnNodeParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::SpsnNodeParenting::<
                    $prio_count,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new((&$multigraph, (10, ())).into()),
            ) as TraitObj<'_>,

            "SpsnHybridParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::SpsnHybridParenting::<
                    $prio_count,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new((&$multigraph, (10, ())).into()),
            ) as TraitObj<'_>,

            "SpsnContactParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::SpsnContactParenting::<
                    $prio_count,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new((&$multigraph, (10, ())).into()),
            ) as TraitObj<'_>,

            // ============================================================
            // SPSN - Hop
            // ============================================================
            "SpsnNodeParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::SpsnNodeParentingHop::<
                    $prio_count,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new((&$multigraph, (10, ())).into()),
            ) as TraitObj<'_>,

            "SpsnHybridParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::SpsnHybridParentingHop::<
                    $prio_count,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new((&$multigraph, (10, ())).into()),
            ) as TraitObj<'_>,

            "SpsnContactParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::SpsnContactParentingHop::<
                    $prio_count,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new((&$multigraph, (10, ())).into()),
            ) as TraitObj<'_>,

            // ============================================================
            // VolCGR - SABR
            // ============================================================
            "VolCgrNodeParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::VolCgrNodeParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::route_storage::table::RoutingTable::new(),
                    $crate::pathfinding::dijkstra_impl::NodeParenting::new(),
                ),
            ) as TraitObj<'_>,

            "VolCgrHybridParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::VolCgrHybridParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::route_storage::table::RoutingTable::new(),
                    $crate::pathfinding::dijkstra_impl::HybridParenting::new(),
                ),
            ) as TraitObj<'_>,

            "VolCgrContactParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::VolCgrContactParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::route_storage::table::RoutingTable::new(),
                    $crate::pathfinding::dijkstra_impl::ContactParenting::new(),
                ),
            ) as TraitObj<'_>,

            // ============================================================
            // VolCGR - Hop
            // ============================================================
            "VolCgrNodeParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::VolCgrNodeParentingHop::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::route_storage::table::RoutingTable::new(),
                    $crate::pathfinding::dijkstra_impl::NodeParenting::new(),
                ),
            ) as TraitObj<'_>,

            "VolCgrHybridParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::VolCgrHybridParentingHop::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::route_storage::table::RoutingTable::new(),
                    $crate::pathfinding::dijkstra_impl::HybridParenting::new(),
                ),
            ) as TraitObj<'_>,

            "VolCgrContactParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::VolCgrContactParentingHop::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::route_storage::table::RoutingTable::new(),
                    $crate::pathfinding::dijkstra_impl::ContactParenting::new(),
                ),
            ) as TraitObj<'_>,

            // ============================================================
            // CGR - First Ending
            // ============================================================
            #[cfg(feature = "contact_suppression")]
            "CgrFirstEndingHybridParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorHybridParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::HybridParenting::new(),
                        $crate::pathfinding::limiting_contact::ends_earlier_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            #[cfg(feature = "contact_suppression")]
            "CgrFirstEndingNodeParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorNodeParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::NodeParenting::new(),
                        $crate::pathfinding::limiting_contact::ends_earlier_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            #[cfg(feature = "contact_suppression")]
            "CgrFirstEndingContactParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorContactParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::ContactParenting::new(),
                        $crate::pathfinding::limiting_contact::ends_earlier_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            #[cfg(feature = "contact_suppression")]
            "CgrFirstEndingHybridParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorHybridParentingHop::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::HybridParenting::new(),
                        $crate::pathfinding::limiting_contact::ends_earlier_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            #[cfg(feature = "contact_suppression")]
            "CgrFirstEndingNodeParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorNodeParentingHop::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::NodeParenting::new(),
                        $crate::pathfinding::limiting_contact::ends_earlier_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            #[cfg(feature = "contact_suppression")]
            "CgrFirstEndingContactParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorContactParentingHop::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::ContactParenting::new(),
                        $crate::pathfinding::limiting_contact::ends_earlier_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            // ============================================================
            // CGR - First Depleted
            // ============================================================
            #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
            "CgrFirstDepletedHybridParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorHybridParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::HybridParenting::new(),
                        $crate::pathfinding::limiting_contact::had_less_volume_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
            "CgrFirstDepletedNodeParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorNodeParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::NodeParenting::new(),
                        $crate::pathfinding::limiting_contact::had_less_volume_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
            "CgrFirstDepletedContactParenting" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorContactParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::ContactParenting::new(),
                        $crate::pathfinding::limiting_contact::had_less_volume_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
            "CgrFirstDepletedHybridParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorHybridParentingHop::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::HybridParenting::new(),
                        $crate::pathfinding::limiting_contact::had_less_volume_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
            "CgrFirstDepletedNodeParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorNodeParentingHop::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::NodeParenting::new(),
                        $crate::pathfinding::limiting_contact::had_less_volume_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
            "CgrFirstDepletedContactParentingHop" => Box::new(
                $crate::pathfinding::top_level::aliases::CgrSupressorContactParentingHop::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::ContactParenting::new(),
                        $crate::pathfinding::limiting_contact::had_less_volume_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                ),
            ) as TraitObj<'_>,

            _ => {
                return Err($crate::errors::ASABRError::ContactPlanError(
                    "Unknown router type: {}",
                ));
            }
        };

        Ok($crate::utils::Router::new($multigraph, pathfinder))
    }};
}
