extern crate alloc;

use alloc::boxed::Box;

use crate::contact_manager::ContactManager;
use crate::contact_manager::legacy::eto::{ETOManager, PBETOManager, PETOManager};
use crate::contact_manager::legacy::evl::{EVLManager, PBEVLManager, PEVLManager};
use crate::contact_manager::legacy::qd::{PBQDManager, PQDManager, QDManager};
use crate::contact_manager::segmentation::pseg::PSegmentationManager;
use crate::contact_manager::segmentation::seg::SegmentationManager;
use crate::{choices, parse_transparent, transparent_CM};

#[derive(Debug)]
/// The base dynamic contact wrapper, which can be parsed from &str or any type implementing the correct conversion.
pub struct StandardManagersDyn(Box<dyn ContactManager>);

transparent_CM!(StandardManagersDyn);

choices!(
    info,
    StandardManagerInfo,
    (PSeg, PSegmentationManager),
    (Seg, SegmentationManager),
    (Eto, ETOManager),
    (PEto, PETOManager),
    (PBEto, PBETOManager),
    (Evl, EVLManager),
    (PEvl, PEVLManager),
    (PBEvl, PBEVLManager),
    (Qd, QDManager),
    (PQd, PQDManager),
    (PBQd, PBQDManager)
);

pub use info::{Kinds as StandardManagersKinds, StandardManagerInfo};

impl From<StandardManagerInfo> for StandardManagersDyn {
    fn from(value: StandardManagerInfo) -> Self {
        StandardManagersDyn(match value {
            StandardManagerInfo::PSeg(manager) => Box::new(manager),
            StandardManagerInfo::Seg(manager) => Box::new(manager),
            StandardManagerInfo::Eto(manager) => Box::new(manager),
            StandardManagerInfo::PEto(manager) => Box::new(manager),
            StandardManagerInfo::PBEto(manager) => Box::new(manager),
            StandardManagerInfo::Evl(manager) => Box::new(manager),
            StandardManagerInfo::PEvl(manager) => Box::new(manager),
            StandardManagerInfo::PBEvl(manager) => Box::new(manager),
            StandardManagerInfo::Qd(manager) => Box::new(manager),
            StandardManagerInfo::PQd(manager) => Box::new(manager),
            StandardManagerInfo::PBQd(manager) => Box::new(manager),
        })
    }
}

impl TryFrom<&str> for StandardManagersKinds {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "seg" => Self::Seg,
            "pseg" => Self::PSeg,
            "eto" => Self::Eto,
            "peto" => Self::PEto,
            "pbeto" => Self::PBEto,
            "evl" => Self::Evl,
            "pevl" => Self::PEvl,
            "pbevl" => Self::PBEvl,
            "qd" => Self::Qd,
            "pqd" => Self::PQd,
            "pbqd" => Self::PBQd,
            _ => return Err(()),
        })
    }
}

parse_transparent!(StandardManagersDyn, StandardManagerInfo);
