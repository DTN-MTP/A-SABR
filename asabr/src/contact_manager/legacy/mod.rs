use crate::{
    bundle::Bundle,
    contact::ContactInfo,
    contact_manager::{ContactManager, ContactManagerTxData},
    parsing::{LexFrom, Parse},
    types::{DataRate, Date, Duration, TimeInterval, Volume},
};

pub mod eto;
pub mod evl;
pub mod qd;

#[cfg(test)]
pub(crate) mod test_helpers;

#[derive(Debug)]
/// A generic legacy volume manager. ETO, PB, ... are newtype on specialisation of this one
struct VolumeManager<const PRIO_COUNT: usize, const BUDGETED: bool> {
    rate: DataRate,
    delay: Duration,
    queue_size: [Volume; PRIO_COUNT],
    budgets: [Volume; PRIO_COUNT],
    original_volume: Volume,
}

impl<const PRIO_COUNT: usize> VolumeManager<PRIO_COUNT, false> {
    /// create a VolumeManager.
    pub fn new(rate: DataRate, delay: Duration) -> Self {
        Self {
            rate,
            delay,
            queue_size: [0; PRIO_COUNT],
            budgets: [0; PRIO_COUNT],
            original_volume: 0,
        }
    }
}

impl<const PRIO_COUNT: usize> VolumeManager<PRIO_COUNT, true> {
    /// create a VolumeManager.
    pub fn new(rate: DataRate, delay: Duration, budgets: [Volume; PRIO_COUNT]) -> Self {
        Self {
            rate,
            delay,
            queue_size: [0; PRIO_COUNT],
            budgets,
            original_volume: 0,
        }
    }
}

impl<const PRIO_COUNT: usize, const BUDGETED: bool> VolumeManager<PRIO_COUNT, BUDGETED> {
    #[inline(always)]
    fn get_queue_size(&self, bundle: &Bundle) -> Volume {
        self.queue_size[(bundle.priority as usize).min(PRIO_COUNT - 1)]
    }
    #[inline(always)]
    fn enqueue(&mut self, bundle: &Bundle) {
        for prio in 0..(bundle.priority as usize + 1).min(PRIO_COUNT) {
            self.queue_size[prio] += bundle.size;
        }
    }
    #[allow(dead_code)]
    #[inline(always)]
    fn dequeue(&mut self, bundle: &Bundle) {
        for prio in 0..(bundle.priority as usize + 1).min(PRIO_COUNT) {
            self.queue_size[prio] -= bundle.size;
        }
    }
    #[inline(always)]
    fn get_budget(&self, bundle: &Bundle) -> Volume {
        if BUDGETED {
            self.budgets[(bundle.priority as usize).min(PRIO_COUNT - 1)]
        } else {
            self.original_volume
        }
    }
}

impl<const PC: usize> From<(DataRate, Duration)> for VolumeManager<PC, false> {
    fn from(value: (DataRate, Duration)) -> Self {
        Self::new(value.0, value.1)
    }
}
impl<const PC: usize> From<(DataRate, Duration, [Volume; PC])> for VolumeManager<PC, true> {
    fn from(value: (DataRate, Duration, [Volume; PC])) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

// inlined parse_transparent to template on cp. yup, ugly, i know.
impl<const AD: bool, const AU: bool, const CP: usize> Parse for LegacyManager<AD, AU, CP, false> {
    type Token = <(DataRate, Duration) as Parse>::Token;
    type Parser = <(DataRate, Duration) as Parse>::Parser;
    fn parse(p: Self::Parser) -> Result<Self, &'static str> {
        Ok(LegacyManager(
            <(DataRate, Duration) as Parse>::parse(p)?.into(),
        ))
    }
    fn feed(tok: Self::Token, parser: &mut Self::Parser) -> Result<bool, &'static str> {
        <(DataRate, Duration) as Parse>::feed(tok, parser)
    }
}
impl<T: ?Sized, const AD: bool, const AU: bool, const PC: usize> LexFrom<T>
    for LegacyManager<AD, AU, PC, false>
where
    (DataRate, Duration): LexFrom<T>,
{
    fn lex(t: &T, p: &Self::Parser) -> Result<Self::Token, &'static str> {
        <(DataRate, Duration) as LexFrom<T>>::lex(t, p)
    }
}

// inlined parse_transparent to template on cp. yup, ugly, i know.
impl<const AD: bool, const AU: bool, const PC: usize> Parse for LegacyManager<AD, AU, PC, true> {
    type Token = <(DataRate, Duration, [Volume; PC]) as Parse>::Token;
    type Parser = <(DataRate, Duration, [Volume; PC]) as Parse>::Parser;
    fn parse(p: Self::Parser) -> Result<Self, &'static str> {
        Ok(LegacyManager(
            <(DataRate, Duration, [Volume; _]) as Parse>::parse(p)?.into(),
        ))
    }
    fn feed(tok: Self::Token, parser: &mut Self::Parser) -> Result<bool, &'static str> {
        <(DataRate, Duration, [Volume; _]) as Parse>::feed(tok, parser)
    }
}
impl<T: ?Sized, const AD: bool, const AU: bool, const PC: usize> LexFrom<T>
    for LegacyManager<AD, AU, PC, true>
where
    (DataRate, Duration, [Volume; PC]): LexFrom<T>,
{
    fn lex(t: &T, p: &Self::Parser) -> Result<Self::Token, &'static str> {
        <(DataRate, Duration, [Volume; _]) as LexFrom<T>>::lex(t, p)
    }
}

/// A legacy volume manager implementation.
///
/// Budget approach by Longrui Ma
///
// # Arguments
///
/// - `add_delay`:A flag (`true` or `false`) that determines whether delay logic should be added depending
///   volume already booked.
/// - `auto_update`: A flag (`true` or `false`) that specifies if the volume must be updated by the manager
///   or manually (like for ETO), this impact the $auto_update behavior, if set to fase, the booked volume is
///   considered as real time queue occupancy.
/// - `prio_count`: The number of priority levels. A value of `1` means no priority logic is applied.
/// - `with_budget`: A flag (`true` or `false`) to conditionnally add budgets (for priorities only).
pub struct LegacyManager<
    const ADD_DELAY: bool,
    const AUTO_UPDATE: bool,
    const PRIO_COUNT: usize,
    const BUDGETED: bool,
>(VolumeManager<PRIO_COUNT, BUDGETED>);

impl<const ADD_DELAY: bool, const AUTO_UPDATE: bool, const PRIO_COUNT: usize, const BUDGETED: bool>
    ContactManager for LegacyManager<ADD_DELAY, AUTO_UPDATE, PRIO_COUNT, BUDGETED>
{
    #[cfg(feature = "manual_queueing")]
    fn manual_enqueue(&mut self, bundle: &Bundle) -> bool {
        if AUTO_UPDATE {
            false
        } else {
            self.0.enqueue(bundle);
            true
        }
    }
    #[cfg(feature = "manual_queueing")]
    fn manual_dequeue(&mut self, bundle: &Bundle) -> bool {
        if AUTO_UPDATE {
            false
        } else {
            self.0.dequeue(bundle);
            true
        }
    }

    fn dry_run_tx(
        &self,
        contact_lifespan: TimeInterval,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        // This function call should be expanded at compile time
        let queue_size = self.0.get_queue_size(bundle);

        if bundle.size > self.0.get_budget(bundle) - queue_size {
            return None;
        }

        let mut contact_start = contact_lifespan.start;
        // add_delay case 1 : if not eto, we push the eto from the contact start time
        if ADD_DELAY && AUTO_UPDATE {
            contact_start += (queue_size / self.0.rate) as Duration;
        }
        let mut tx_start = if contact_start > at_time {
            contact_start
        } else {
            at_time
        };

        // add_delay case 2 : eto, bundles are still in queue
        if ADD_DELAY && !AUTO_UPDATE {
            tx_start += (queue_size / self.0.rate) as Duration;
        }

        let tx_end = tx_start + (bundle.size / self.0.rate) as Duration;
        if tx_end > contact_lifespan.end {
            return None;
        }
        Some(ContactManagerTxData {
            tx_window: TimeInterval {
                start: tx_start,
                end: tx_end,
            },
            expiration: contact_lifespan.end,
            rx_window: TimeInterval {
                start: tx_start + self.0.delay,
                end: tx_end + self.0.delay,
            },
        })
    }

    fn schedule_tx(
        &mut self,
        contact_data: TimeInterval,
        at_time: Date,
        bundle: &Bundle,
    ) -> Option<ContactManagerTxData> {
        let data = self.dry_run_tx(contact_data, at_time, bundle)?;
        // Conditionally update queue size based on $auto_update
        // Can overflow with overbooking
        if AUTO_UPDATE {
            self.0.enqueue(bundle);
        }
        Some(data)
    }

    fn try_init(&mut self, contact_data: &ContactInfo) -> bool {
        self.0.original_volume = (contact_data.end - contact_data.start) * self.0.rate;
        true
    }

    #[cfg(feature = "first_depleted")]
    fn get_original_volume(&self) -> Volume {
        self.0.original_volume
    }
}

impl<const ADD_DELAY: bool, const AUTO_UPDATE: bool, const PRIO_COUNT: usize>
    LegacyManager<ADD_DELAY, AUTO_UPDATE, PRIO_COUNT, false>
{
    pub fn new(rate: DataRate, delay: Duration) -> Self {
        LegacyManager(VolumeManager::<_, false>::new(rate, delay))
    }
}
impl<const ADD_DELAY: bool, const AUTO_UPDATE: bool, const PRIO_COUNT: usize>
    LegacyManager<ADD_DELAY, AUTO_UPDATE, PRIO_COUNT, true>
{
    pub fn new(rate: DataRate, delay: Duration, budgets: [Volume; PRIO_COUNT]) -> Self {
        LegacyManager(VolumeManager::<_, true>::new(rate, delay, budgets))
    }
}
