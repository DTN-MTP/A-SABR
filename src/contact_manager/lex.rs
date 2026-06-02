extern crate alloc;

use alloc::boxed::Box;

use crate::contact_manager::ContactManager;
use crate::contact_manager::legacy::eto::{ETOManager, PBETOManager, PETOManager};
use crate::contact_manager::legacy::evl::{EVLManager, PBEVLManager, PEVLManager};
use crate::contact_manager::legacy::lex::{Budget, BudgetParser};
use crate::contact_manager::legacy::qd::{PBQDManager, PQDManager, QDManager};
use crate::contact_manager::segmentation::lex::SegmentInfo;
use crate::contact_manager::segmentation::pseg::PSegmentationManager;
use crate::contact_manager::segmentation::seg::SegmentationManager;
use crate::types::{DataRate, Duration};
use crate::{choices, parse_transparent, transparent_CM};

#[derive(Debug)]
/// The base dynamic contact wrapper, which can be parsed from &str or any type implementing the correct conversion.
pub struct StandardManagersDyn(Box<dyn ContactManager>);

transparent_CM!(StandardManagersDyn);

// impl ContactManager for StandardManagersDyn {
//     fn dry_run_tx(
//         &self,
//         contact_data: &crate::contact::ContactInfo,
//         at_time: crate::types::Date,
//         bundle: &crate::bundle::Bundle,
//     ) -> Option<super::ContactManagerTxData> {
//         self.0.dry_run_tx(contact_data, at_time, bundle)
//     }

//     fn schedule_tx(
//         &mut self,
//         contact_data: &crate::contact::ContactInfo,
//         at_time: crate::types::Date,
//         bundle: &crate::bundle::Bundle,
//     ) -> Option<super::ContactManagerTxData> {
//         self.0.schedule_tx(contact_data, at_time, bundle)
//     }

//     #[cfg(feature = "first_depleted")]
//     fn get_original_volume(&self) -> crate::types::Volume {
//         self.0.get_original_volume()
//     }

//     fn try_init(&mut self, contact_data: &crate::contact::ContactInfo) -> bool {
//         self.0.try_init(contact_data)
//     }
// }

choices!(
    info,
    StandardManagerInfo,
    (PSeg, SegmentInfo),
    (Seg, SegmentInfo),
    (Eto, (DataRate, Duration)),
    (PEto, (DataRate, Duration)),
    (PBEto, ((DataRate, Duration), BudgetParser)),
    (Evl, (DataRate, Duration)),
    (PEvl, (DataRate, Duration)),
    (PBEvl, ((DataRate, Duration), BudgetParser)),
    (Qd, (DataRate, Duration)),
    (PQd, (DataRate, Duration)),
    (PBQd, ((DataRate, Duration), BudgetParser))
);

pub use info::{Kinds as StandardManagersKinds, StandardManagerInfo};

impl TryFrom<StandardManagerInfo> for StandardManagersDyn {
    type Error = &'static str;
    fn try_from(value: StandardManagerInfo) -> Result<Self, Self::Error> {
        Ok(StandardManagersDyn(match value {
            StandardManagerInfo::PSeg(info) => {
                Box::new(PSegmentationManager::new(info.rates, info.delays))
            }
            StandardManagerInfo::Seg(info) => {
                Box::new(SegmentationManager::new(info.rates, info.delays))
            }
            StandardManagerInfo::Eto((rate, delay)) => Box::new(ETOManager::new(rate, delay)),
            StandardManagerInfo::PEto((rate, delay)) => Box::new(PETOManager::new(rate, delay)),
            StandardManagerInfo::PBEto(((rate, delay), budget)) => {
                Box::new(PBETOManager::new(rate, delay, Budget::from(budget).0))
            }
            StandardManagerInfo::Evl((rate, delay)) => Box::new(EVLManager::new(rate, delay)),
            StandardManagerInfo::PEvl((rate, delay)) => Box::new(PEVLManager::new(rate, delay)),
            StandardManagerInfo::PBEvl(((rate, delay), budget)) => {
                Box::new(PBEVLManager::new(rate, delay, Budget::from(budget).0))
            }
            StandardManagerInfo::Qd((rate, delay)) => Box::new(QDManager::new(rate, delay)),
            StandardManagerInfo::PQd((rate, delay)) => Box::new(PQDManager::new(rate, delay)),
            StandardManagerInfo::PBQd(((rate, delay), budget)) => {
                Box::new(PBQDManager::new(rate, delay, Budget::from(budget).0))
            }
        }))
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
