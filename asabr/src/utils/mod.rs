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

pub mod aliases;

use core::{
    fmt::Display,
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

pub trait Routing<'id, NM, CM, D>: DerefMut<Target = Multigraph<'id, NM, CM>>
where
    NM: NodeManager,
    CM: ContactManager,
    D: RoutableDest<'id, NM, CM>,
{
    type Pathfinder: Pathfinding<'id, NM, CM, D>;

    // ---- required: the only parts that touch the fields ----
    fn new(multigraph: Multigraph<'id, NM, CM>, pathfinder: Self::Pathfinder) -> Self
    where
        Self: Sized;

    fn parts_mut(&mut self) -> (&mut Multigraph<'id, NM, CM>, &mut Self::Pathfinder);

    // Set Source
    fn set_source(&mut self, _src: INodeRef<'id>) -> Result<(), ASABRError> {
        Err(ASABRError::RoutingError("Source INodeRef isn't not set"))
    }

    // Get Source
    fn get_source(&self) -> Result<INodeRef<'id>, ASABRError> {
        Err(ASABRError::RoutingError("Source INodeRef isn't not set"))
    }

    // ---- defaults ----
    fn build<T>(
        guard: Guard<'id>,
        contact_plan: ContactPlan<NM, CM>,
        pathfinder_args: T,
    ) -> Result<Self, ASABRError>
    where
        Self: Sized,
        for<'a> (&'a Multigraph<'id, NM, CM>, T): Into<Self::Pathfinder>,
    {
        let multigraph = Multigraph::new(guard, contact_plan)?;
        let pathfinder = (&multigraph, pathfinder_args).into();
        Ok(Self::new(multigraph, pathfinder))
    }

    /// # Safety
    /// see Multigraph::new_unguarder
    unsafe fn build_unguarded<T>(
        contact_plan: ContactPlan<NM, CM>,
        pathfinder_args: T,
    ) -> Result<Self, ASABRError>
    where
        Self: Sized,
        for<'a> (&'a Multigraph<'id, NM, CM>, T): Into<Self::Pathfinder>,
    {
        Self::build(
            unsafe { Guard::new(Id::new()) },
            contact_plan,
            pathfinder_args,
        )
    }

    fn find_path<'a>(
        &'a mut self,
        mut destination: D,
        routing_time: Date,
        bundle: &Bundle,
        prune_time: Option<Date>,
    ) -> Result<Option<PathFindingOutput<'id, 'a>>, ASABRError>
    where
        NM: 'a,
        CM: 'a,
        D: 'a,
    {
        let src = self.get_source()?;
        let (multigraph, pathfinder) = self.parts_mut();
        pathfinder.find_path(
            multigraph,
            routing_time,
            src,
            bundle,
            &mut destination,
            prune_time,
        )
    }

    fn multigraph(&self) -> &Multigraph<'id, NM, CM>;

    fn route<'a>(
        &'a mut self,
        mut destination: D,
        routing_time: Date,
        bundle: &Bundle,
        prune_time: Option<Date>,
    ) -> Result<Option<D::RoutingOutput<'a>>, ASABRError>
    where
        NM: 'a,
        CM: 'a,
        D: 'a,
    {
        let src = self.get_source()?;
        let (multigraph, pathfinder) = self.parts_mut();
        destination.route(
            multigraph,
            bundle,
            pathfinder,
            routing_time,
            src,
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
    const SINGLE: bool,
> {
    pub multigraph: Multigraph<'id, NM, CM>,
    pub pathfinder: P,
    source: Option<INodeRef<'id>>,
    _phantom: PhantomData<fn(D)>,
}

pub type SingleSourceRouter<'id, NM, CM, P, D> = Router<'id, NM, CM, P, D, true>;
pub type MultiSourceRouter<'id, NM, CM, P, D> = Router<'id, NM, CM, P, D, false>;

impl<'id, NM, CM, P, D, const SINGLE: bool> Display for Router<'id, NM, CM, P, D, SINGLE>
where
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D> + Display,
    D: RoutableDest<'id, NM, CM>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.pathfinder.fmt(f) // or: write!(f, "{}", self.pathfinder)
    }
}

impl<'id, NM, CM, P, D, const SINGLE: bool> Routing<'id, NM, CM, D>
    for Router<'id, NM, CM, P, D, SINGLE>
where
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D>,
    D: RoutableDest<'id, NM, CM>,
{
    type Pathfinder = P;

    fn new(multigraph: Multigraph<'id, NM, CM>, pathfinder: P) -> Self {
        Self {
            multigraph,
            pathfinder,
            source: None,
            _phantom: PhantomData,
        }
    }

    fn set_source(&mut self, src: INodeRef<'id>) -> Result<(), ASABRError> {
        if SINGLE && self.source.is_some() {
            return Err(ASABRError::RoutingError(
                "Single source router, source already set",
            ));
        }
        self.source = Some(src);
        Ok(())
    }

    fn get_source(&self) -> Result<INodeRef<'id>, ASABRError> {
        self.source
            .ok_or(ASABRError::RoutingError("Source INodeRef isn't set"))
    }

    fn multigraph(&self) -> &Multigraph<'id, NM, CM> {
        &self.multigraph
    }

    fn parts_mut(&mut self) -> (&mut Multigraph<'id, NM, CM>, &mut P) {
        (&mut self.multigraph, &mut self.pathfinder)
    }
}

impl<
    'id,
    NM: NodeManager,
    CM: ContactManager,
    P: Pathfinding<'id, NM, CM, D>,
    D: FindableDest<'id, NM, CM>,
    const SINGLE: bool,
> Deref for Router<'id, NM, CM, P, D, SINGLE>
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
    const SINGLE: bool,
> DerefMut for Router<'id, NM, CM, P, D, SINGLE>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.multigraph
    }
}

impl<'a, NM, CM, D, PF> Display for dyn Routing<'a, NM, CM, D, Pathfinder = PF> + 'a
where
    NM: NodeManager,
    CM: ContactManager,
    D: RoutableDest<'a, NM, CM>,
    PF: Pathfinding<'a, NM, CM, D>,
    Multigraph<'a, NM, CM>: Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.multigraph())
    }
}

#[doc(hidden)]
pub fn spsn_args(arg: Option<usize>) -> Result<(usize, ()), ASABRError> {
    arg.map(|v| (v, ())).ok_or(ASABRError::ContactPlanError(
        "SPSN routers require a usize argument",
    ))
}

// Decided next to each algo's construction, so there is only one list to maintain.
pub enum SourceKind {
    Multi,
    Single,
}

#[macro_export]
macro_rules! mk_router_parts {
    (
        $id:ident,
        $NM:ty,
        $CM:ty,
        $prio_count:expr,
        $algo:expr,
        $multigraph:expr,
        $algo_args:expr,
        $DISTANCE:ty
    ) => {{
        let algo_args: Option<usize> = $algo_args;

        let (pathfinder, kind): (TraitObj<'_>, $crate::utils::SourceKind) = match $algo {
            // ============================================================
            // Multi source
            // ============================================================
            "OracleNodeParenting" => (
                Box::new(
                    $crate::pathfinding::dijkstra_impl::NodeParenting::<$DISTANCE>::from((
                        &$multigraph,
                        (),
                    )),
                ) as TraitObj<'_>,
                $crate::utils::SourceKind::Multi,
            ),

            "OracleContactParenting" => (
                Box::new($crate::pathfinding::dijkstra_impl::ContactParenting::<
                    $NM,
                    $CM,
                    $DISTANCE,
                    $crate::multigraph::RoutableNodeRef<'_>,
                >::from((&$multigraph, ()))) as TraitObj<'_>,
                $crate::utils::SourceKind::Multi,
            ),

            "OracleHybridParenting" => (
                Box::new($crate::pathfinding::dijkstra_impl::HybridParenting::<
                    $DISTANCE,
                    $NM,
                    $CM,
                >::from((&$multigraph, ()))) as TraitObj<'_>,
                $crate::utils::SourceKind::Multi,
            ),

            // ============================================================
            // SPSN
            // ============================================================
            "SpsnNodeParenting" => (
                Box::new($crate::utils::aliases::SpsnNodeParenting::<
                    $prio_count,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >::from((
                    &$multigraph,
                    $crate::utils::spsn_args(algo_args)?,
                ))) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            "SpsnHybridParenting" => (
                Box::new($crate::utils::aliases::SpsnHybridParenting::<
                    $prio_count,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >::from((
                    &$multigraph,
                    $crate::utils::spsn_args(algo_args)?,
                ))) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            "SpsnContactParenting" => (
                Box::new($crate::utils::aliases::SpsnContactParenting::<
                    $prio_count,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >::from((
                    &$multigraph,
                    $crate::utils::spsn_args(algo_args)?,
                ))) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            // ============================================================
            // VolCGR
            // ============================================================
            "VolCgrNodeParenting" => (
                Box::new(<$crate::utils::aliases::VolCgrNodeParenting<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >>::from((&$multigraph, ((), ())))) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            "VolCgrHybridParenting" => (
                Box::new(<$crate::utils::aliases::VolCgrHybridParenting<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >>::from((&$multigraph, ((), ())))) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            "VolCgrContactParenting" => (
                Box::new(<$crate::utils::aliases::VolCgrContactParenting<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >>::from((&$multigraph, ((), ())))) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            // ============================================================
            // CGR - First Ending
            // ============================================================
            #[cfg(feature = "contact_suppression")]
            "CgrFirstEndingHybridParenting" => (
                Box::new($crate::utils::aliases::CgrSupressorHybridParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::HybridParenting::new(),
                        $crate::pathfinding::limiting_contact::ends_earlier_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                )) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            #[cfg(feature = "contact_suppression")]
            "CgrFirstEndingNodeParenting" => (
                Box::new($crate::utils::aliases::CgrSupressorNodeParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::NodeParenting::new(),
                        $crate::pathfinding::limiting_contact::ends_earlier_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                )) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            #[cfg(feature = "contact_suppression")]
            "CgrFirstEndingContactParenting" => (
                Box::new($crate::utils::aliases::CgrSupressorContactParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::ContactParenting::new(),
                        $crate::pathfinding::limiting_contact::ends_earlier_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                )) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            // ============================================================
            // CGR - First Depleted
            // ============================================================
            #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
            "CgrFirstDepletedHybridParenting" => (
                Box::new($crate::utils::aliases::CgrSupressorHybridParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::HybridParenting::new(),
                        $crate::pathfinding::limiting_contact::had_less_volume_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                )) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
            "CgrFirstDepletedNodeParenting" => (
                Box::new($crate::utils::aliases::CgrSupressorNodeParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::NodeParenting::new(),
                        $crate::pathfinding::limiting_contact::had_less_volume_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                )) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            #[cfg(all(feature = "contact_suppression", feature = "first_depleted"))]
            "CgrFirstDepletedContactParenting" => (
                Box::new($crate::utils::aliases::CgrSupressorContactParenting::<
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'_>,
                    $DISTANCE,
                >::new(
                    $crate::pathfinding::limiting_contact::Suppressor::new(
                        $crate::pathfinding::dijkstra_impl::ContactParenting::new(),
                        $crate::pathfinding::limiting_contact::had_less_volume_than,
                        &$multigraph,
                    ),
                    $crate::route_storage::table::RoutingTable::new(),
                    &$multigraph,
                )) as TraitObj<'_>,
                $crate::utils::SourceKind::Single,
            ),

            _ => {
                return Err($crate::errors::ASABRError::ContactPlanError(
                    "Unknown router type: {}",
                ));
            }
        };

        (&$multigraph, pathfinder, kind)
    }};
}

#[macro_export]
macro_rules! mk_router_from_graph {
    (
        $id:ident,
        $NM:ty,
        $CM:ty,
        $prio_count:expr,
        $algo:expr,
        $multigraph:expr,
        $algo_args:expr,
        $DISTANCE:ty
    ) => {{
        let (multigraph, pathfinder, kind) = $crate::mk_router_parts!(
            $id,
            $NM,
            $CM,
            $prio_count,
            $algo,
            $multigraph,
            $algo_args,
            $DISTANCE
        );

        // Alias for the dynamic Trait Object types to coerce match arms
        type TraitObj<'a> = Box<
            dyn $crate::pathfinding::Pathfinding<
                    'a,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'a>,
                > + 'a,
        >;
        type RoutingObj<'a> = Box<
            dyn $crate::utils::Routing<
                    'a,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'a>,
                    Pathfinder = TraitObj<'a>,
                > + 'a,
        >;

        use $crate::utils::Routing as _;
        match kind {
            $crate::utils::SourceKind::Multi => Ok(Box::new($crate::utils::MultiSourceRouter::<
                $NM,
                $CM,
                _,
                $crate::multigraph::RoutableNodeRef<'_>,
            >::new(
                $multigraph, pathfinder
            )) as RoutingObj<'_>),
            $crate::utils::SourceKind::Single => Ok(Box::new($crate::utils::SingleSourceRouter::<
                $NM,
                $CM,
                _,
                $crate::multigraph::RoutableNodeRef<'_>,
            >::new(
                $multigraph, pathfinder
            )) as RoutingObj<'_>),
        }
    }};
}

#[macro_export]
macro_rules! mk_router_from_cp {
    (
        $id:ident,
        $NM:ty,
        $CM:ty,
        $prio_count:expr,
        $algo:expr,
        $cp:expr,
        $algo_args:expr,
        $DISTANCE:ty
    ) => {{
        let multigraph = $crate::multigraph::Multigraph::new($id, $cp)?;
        let (_, pathfinder, kind) = $crate::mk_router_parts!(
            $id,
            $NM,
            $CM,
            $prio_count,
            $algo,
            multigraph,
            $algo_args,
            $DISTANCE
        );

        // Alias for the dynamic Trait Object types to coerce match arms
        type TraitObj<'a> = Box<
            dyn $crate::pathfinding::Pathfinding<
                    'a,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'a>,
                > + 'a,
        >;
        type RoutingObj<'a> = Box<
            dyn $crate::utils::Routing<
                    'a,
                    $NM,
                    $CM,
                    $crate::multigraph::RoutableNodeRef<'a>,
                    Pathfinder = TraitObj<'a>,
                > + 'a,
        >;

        use $crate::utils::Routing as _;
        match kind {
            $crate::utils::SourceKind::Multi => Ok(Box::new($crate::utils::MultiSourceRouter::<
                $NM,
                $CM,
                _,
                $crate::multigraph::RoutableNodeRef<'_>,
            >::new(
                multigraph, pathfinder
            )) as RoutingObj<'_>),
            $crate::utils::SourceKind::Single => Ok(Box::new($crate::utils::SingleSourceRouter::<
                $NM,
                $CM,
                _,
                $crate::multigraph::RoutableNodeRef<'_>,
            >::new(
                multigraph, pathfinder
            )) as RoutingObj<'_>),
        }
    }};
}
