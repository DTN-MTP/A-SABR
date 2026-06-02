use crate::{
    contact_manager::legacy::{
        eto::{ETOManager, PBETOManager, PETOManager},
        evl::{EVLManager, PBEVLManager, PEVLManager},
        qd::{PBQDManager, PQDManager, QDManager},
    },
    parse_transparent,
    types::{DataRate, Duration, Volume},
};

#[derive(Default, Clone, Copy)]
pub struct Budget(pub [Volume; 3]);

#[derive(Default, Clone, Copy)]
pub struct LegacyInfo {
    pub rate: DataRate,
    pub delay: Duration,
}

pub type BudgetParser = (Volume, (Volume, Volume));
pub type InfoParser = (DataRate, Duration);

impl From<BudgetParser> for Budget {
    fn from(value: BudgetParser) -> Self {
        let (a, (b, c)) = value;
        Budget([a, b, c])
    }
}
impl From<InfoParser> for LegacyInfo {
    fn from(value: InfoParser) -> Self {
        let (rate, delay) = value;
        LegacyInfo { rate, delay }
    }
}

parse_transparent!(Budget, BudgetParser);
parse_transparent!(LegacyInfo, InfoParser);

impl From<LegacyInfo> for ETOManager {
    fn from(value: LegacyInfo) -> Self {
        ETOManager::new(value.rate, value.delay)
    }
}
parse_transparent!(ETOManager, LegacyInfo);
impl From<LegacyInfo> for PETOManager {
    fn from(value: LegacyInfo) -> Self {
        PETOManager::new(value.rate, value.delay)
    }
}
parse_transparent!(PETOManager, LegacyInfo);
impl From<(LegacyInfo, Budget)> for PBETOManager {
    fn from(value: (LegacyInfo, Budget)) -> Self {
        PBETOManager::new(value.0.rate, value.0.delay, value.1.0)
    }
}
parse_transparent!(PBETOManager, (LegacyInfo, Budget));
impl From<LegacyInfo> for EVLManager {
    fn from(value: LegacyInfo) -> Self {
        EVLManager::new(value.rate, value.delay)
    }
}
parse_transparent!(EVLManager, LegacyInfo);
impl From<LegacyInfo> for PEVLManager {
    fn from(value: LegacyInfo) -> Self {
        PEVLManager::new(value.rate, value.delay)
    }
}
parse_transparent!(PEVLManager, LegacyInfo);
impl From<(LegacyInfo, Budget)> for PBEVLManager {
    fn from(value: (LegacyInfo, Budget)) -> Self {
        PBEVLManager::new(value.0.rate, value.0.delay, value.1.0)
    }
}
parse_transparent!(PBEVLManager, (LegacyInfo, Budget));
impl From<LegacyInfo> for QDManager {
    fn from(value: LegacyInfo) -> Self {
        QDManager::new(value.rate, value.delay)
    }
}
parse_transparent!(QDManager, LegacyInfo);
impl From<LegacyInfo> for PQDManager {
    fn from(value: LegacyInfo) -> Self {
        PQDManager::new(value.rate, value.delay)
    }
}
parse_transparent!(PQDManager, LegacyInfo);
impl From<(LegacyInfo, Budget)> for PBQDManager {
    fn from(value: (LegacyInfo, Budget)) -> Self {
        PBQDManager::new(value.0.rate, value.0.delay, value.1.0)
    }
}
parse_transparent!(PBQDManager, (LegacyInfo, Budget));
