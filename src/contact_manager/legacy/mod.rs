pub mod eto;
pub mod evl;
pub mod qd;

// Budget approach by Longrui Ma
#[macro_export]
macro_rules! generate_struct_management {
    // if the priority count is set to 1, no priority, queue_size is not an array
    ($manager_name:ident, 1, false) => {
        /// A simple manager for handling volcontact_data.startume and/or transmission delays (macro generated).
        #[cfg_attr(feature = "debug", derive(Debug))]
        pub struct $manager_name {
            /// The data transmission rate.
            rate: crate::types::DataRate,
            /// The delay between transmissions.
            delay: crate::types::Duration,
            /// The volume scheduled for this contact.
            queue_size: crate::types::Volume,
            /// The total volume at initialization.
            original_volume: crate::types::Volume,
        }
         impl $manager_name {
            #[doc = concat!( "Creates a new `", stringify!($manager_name),"`  with specified average rate and delay.")]
            ///
            /// # Arguments
            ///
            /// * `rate` - The average data rate for this contact.
            /// * `delay` - The link delay for this contact.
            ///
            /// # Returns
            ///
             #[doc = concat!( " A new instance of  `", stringify!($manager_name),"`.")]
            pub fn new(rate: crate::types::DataRate, delay: crate::types::Duration) -> Self {
                Self {
                    rate,
                    delay,
                    queue_size: 0.0,
                    original_volume: 0.0,
                }
            }
            #[inline(always)]
            fn get_queue_size(&self, _bundle: &crate::bundle::Bundle) -> crate::types::Volume {
                    self.queue_size
            }
           #[inline(always)]
            fn enqueue(&mut self, bundle: &crate::bundle::Bundle)  {
                 self.queue_size += bundle.size;
            }
            #[allow(dead_code)]
            #[inline(always)]
            fn dequeue(&mut self, bundle: &crate::bundle::Bundle)  {
                self.queue_size -= bundle.size;
            }
            #[inline(always)]
            fn get_budget(&self, _bundle: &crate::bundle::Bundle) -> crate::types::Volume  {
               return self.original_volume;
            }
            #[inline(always)]
            fn build_parsing_output(rate: crate::types::DataRate, delay: crate::types::Duration, _lexer: &mut dyn crate::parsing::Lexer) -> crate::parsing::ParsingState<Self>{
                return crate::parsing::ParsingState::Finished($manager_name::new(rate, delay));
            }
        }
    };

    // // Safe guard $prio_count, a value of 0 would still compile..
    // ($manager_name:ident, 0, $with_budget:ident) => {
    //      compile_error!("To disable priority, set $prio_count to 1");
    // };
    // // Safe guard $with_budget with no prioities
    // ($manager_name:ident, 1, true) => {
    //      compile_error!("No priority variant cannot have budgets");
    // };

    // if the priority count is different than one, queue_size is an array
    ($manager_name:ident, $prio_count:tt, false) => {

        #[cfg_attr(feature = "debug", derive(Debug))]
        pub struct $manager_name {
            /// The data transmission rate.
            rate: crate::types::DataRate,
            /// The delay between transmissions.
            delay: crate::types::Duration,
            /// The volume scheduled for this contact.
            queue_size: [crate::types::Volume; $prio_count],
            /// The total volume at initialization.
            original_volume: crate::types::Volume,
        }

        impl $manager_name {
            #[doc = concat!( "Creates a new `", stringify!($manager_name),"`  with specified average rate and delay.")]
            ///
            /// # Arguments
            ///
            /// * `rate` - The average data rate for this contact.
            /// * `delay` - The link delay for this contact.
            ///
            /// # Returns
            ///
             #[doc = concat!( " A new instance of  `", stringify!($manager_name),"`.")]
            pub fn new(rate: crate::types::DataRate, delay: crate::types::Duration) -> Self {
                Self {
                    rate,
                    delay,
                    queue_size: [0.0; $prio_count],
                    original_volume: 0.0,
                }
            }

            #[inline(always)]
            fn get_queue_size(&self, bundle: &crate::bundle::Bundle) -> crate::types::Volume {
                    self.queue_size[bundle.priority as usize]
            }
            #[inline(always)]
            fn enqueue(&mut self, bundle: &crate::bundle::Bundle)  {
                for prio in 0..bundle.priority as usize + 1 {
                    self.queue_size[prio] += bundle.size;
                }
            }
            #[allow(dead_code)]
            #[inline(always)]
            fn dequeue(&mut self, bundle: &crate::bundle::Bundle)  {
                for prio in 0..bundle.priority as usize + 1 {
                    self.queue_size[prio] -= bundle.size;
                }
            }
            #[inline(always)]
            fn get_budget(&self, _bundle: &crate::bundle::Bundle) -> crate::types::Volume  {
               return self.original_volume;
            }
            #[inline(always)]
            fn build_parsing_output(rate: crate::types::DataRate, delay: crate::types::Duration, _lexer: &mut dyn crate::parsing::Lexer) -> crate::parsing::ParsingState<Self>{
                return crate::parsing::ParsingState::Finished($manager_name::new(rate, delay));
            }
        }
    };
    // if the priority count is different than one, queue_size is an array
    ($manager_name:ident, $prio_count:tt, true) => {

        #[cfg_attr(feature = "debug", derive(Debug))]
        pub struct $manager_name {
            /// The data transmission rate.
            rate: crate::types::DataRate,
            /// The delay between transmissions.
            delay: crate::types::Duration,
            /// The volume scheduled for this contact.
            queue_size: [crate::types::Volume; $prio_count],

            budgets: [crate::types::Volume; $prio_count],
            /// The total volume at initialization.
            original_volume: crate::types::Volume,
        }

        impl $manager_name {
            #[doc = concat!( "Creates a new `", stringify!($manager_name),"`  with specified average rate and delay.")]
            ///
            /// # Arguments
            ///
            /// * `rate` - The average data rate for this contact.
            /// * `delay` - The link delay for this contact.
            ///
            /// # Returns
            ///
             #[doc = concat!( " A new instance of  `", stringify!($manager_name),"`.")]
            pub fn new(rate: crate::types::DataRate, delay: crate::types::Duration, budgets: [crate::types::Volume; $prio_count] ) -> Self {
                Self {
                    rate,
                    delay,
                    queue_size: [0.0; $prio_count],
                    budgets: budgets,
                    original_volume: 0.0,
                }
            }

            #[inline(always)]
            fn get_queue_size(&self, bundle: &crate::bundle::Bundle) -> crate::types::Volume {
                    self.queue_size[bundle.priority as usize]
            }
            #[inline(always)]
            fn enqueue(&mut self, bundle: &crate::bundle::Bundle)  {
                for prio in 0..bundle.priority as usize + 1 {
                    self.queue_size[prio] += bundle.size;
                }
            }
            #[allow(dead_code)]
            #[inline(always)]
            fn dequeue(&mut self, bundle: &crate::bundle::Bundle)  {
                for prio in 0..bundle.priority as usize + 1 {
                    self.queue_size[prio] -= bundle.size;
                }
            }
            #[inline(always)]
            fn get_budget(&self, bundle: &crate::bundle::Bundle) -> crate::types::Volume  {
               return self.budgets[bundle.priority as usize];
            }
            #[inline(always)]
            fn build_parsing_output(rate: crate::types::DataRate, delay: crate::types::Duration, lexer: &mut dyn crate::parsing::Lexer) -> crate::parsing::ParsingState<Self>{
                let mut budgets = [0.0; 3];
                for i in 0..$prio_count {

                    let budget_state = <crate::types::Volume as crate::types::Token<crate::types::Volume>>::parse(lexer);
                    match budget_state {
                        crate::parsing::ParsingState::Finished(value) => budgets[i] = value,
                        crate::parsing::ParsingState::Error(msg) => return crate::parsing::ParsingState::Error(msg),
                        crate::parsing::ParsingState::EOF => {
                            return crate::parsing::ParsingState::Error(format!(
                                "Parsing failed ({})",
                                lexer.get_current_position()
                            ))
                        }
                    }
                }

                return crate::parsing::ParsingState::Finished($manager_name::new(rate, delay, budgets));
            }
        }
    };
}

#[macro_export]
macro_rules! generate_manual_enqueue {
    (false) => {
        fn manual_enqueue(&mut self, bundle: &crate::bundle::Bundle) -> bool {
            self.enqueue(bundle);
            true
        }
        fn manual_dequeue(&mut self, bundle: &crate::bundle::Bundle) -> bool {
            self.dequeue(bundle);
            true
        }
    };
    (true) => {
        // Keep default impl if auto update
    };
}

#[macro_export]
macro_rules! generate_prio_volume_manager {

    ($manager_name:ident, $add_delay:tt, $auto_update:tt, $prio_count:tt, $with_budget:tt)  => {

        crate::generate_struct_management!($manager_name, $prio_count, $with_budget);

        impl crate::contact_manager::ContactManager for $manager_name {
            #[cfg(feature = "manual_queueing")]
            crate::generate_manual_enqueue!($auto_update);

            /// Simulates the transmission of a bundle based on the contact data and available free intervals.
            ///
            #[doc = concat!( "The transmission time start time will be offset by the queue size: ", stringify!($add_delay),"`.")]
            ///
            /// # Arguments
            ///
            /// * `contact_data` - Reference to the contact information (unused in this implementation).
            /// * `at_time` - The current time for scheduling purposes.
            /// * `bundle` - The bundle to be transmitted.
            ///
            /// # Returns
            ///
            /// Optionally returns `ContactManagerTxData` with transmission start and end times, or `None` if the bundle can't be transmitted.
            fn dry_run_tx(
                &self,
                contact_data: &crate::contact::ContactInfo,
                at_time: crate::types::Date,
                bundle: &crate::bundle::Bundle,
            ) -> Option<crate::contact_manager::ContactManagerTxData> {

                // This function call should be expanded at compile time
                let queue_size = self.get_queue_size(&bundle);

                if bundle.size > self.get_budget(&bundle) - queue_size {
                    println!("{}", queue_size);
                    return None;
                }

                let mut contact_start = contact_data.start;
                // add_delay case 1 : if not eto, we push the eto from the contact start time
                if ($add_delay && $auto_update) {
                    contact_start += queue_size / self.rate;
                }
                let mut tx_start = if (contact_start > at_time) {
                    contact_start
                } else {
                    at_time
                };

                // add_delay case 2 : eto, bundles are still in queue
                if ($add_delay && !$auto_update) {
                    tx_start += queue_size / self.rate;
                }

                let tx_end = tx_start + bundle.size / self.rate;
                if tx_end > contact_data.end {
                    return None;
                }
                Some(crate::contact_manager::ContactManagerTxData {
                    tx_start,
                    tx_end,
                    delay: self.delay,
                    expiration: contact_data.end,
                    arrival: self.delay + tx_end,
                })
            }

            /// Schedule the transmission of a bundle based on the contact data and available free intervals.
            ///
            /// This method shall be called after a dry run !Implementations might not ensure a clean behavior otherwise.
            #[doc = concat!( "The queue volume will be updated by this method: ", stringify!($auto_update),"`.")]
            ///contact_data.start
            /// # Arguments
            ///
            /// * `contact_data` - Reference to the contact information (unused in this implementation).
            /// * `at_time` - The current time for scheduling purposes.
            /// * `bundle` - The bundle to be transmitted.
            ///
            /// # Returns
            ///
            /// Optionally returns `ContactManagerTxData` with transmission start and end times, or `None` if the bundle can't be transmitted.
            fn schedule_tx(
                &mut self,
                contact_data: &crate::contact_manager::ContactInfo,
                at_time: crate::types::Date,
                bundle: &crate::bundle::Bundle,
            ) -> Option<crate::contact_manager::ContactManagerTxData> {
                if let Some(data) = self.dry_run_tx(contact_data, at_time, bundle) {
                    // Conditionally update queue size based on $auto_update
                    // Can overflow with overbooking
                    if $auto_update {
                        self.enqueue(bundle);
                    }
                    return Some(data);
                }
                None
            }

            /// Initializes the segmentation manager by checking that rate and delay intervals have no gaps.
            ///
            /// # Arguments
            ///
            /// * `contact_data` - Reference to the contact information.
            ///
            /// # Returns
            ///
            /// Returns `true` if initialization is successful, or `false` if there are gaps in the intervals.
            fn try_init(&mut self, contact_data: &crate::contact::ContactInfo) -> bool {
                self.original_volume = (contact_data.end - contact_data.start) * self.rate;
                true
            }

            /// Returns the original volume of the object.
            ///
            /// # Returns
            ///
            /// A `Volume` representing the original volume.
            #[cfg(feature = "first_depleted")]
            fn get_original_volume(&self) -> crate::types::Volume {
                self.original_volume
            }
        }

        /// Implements the DispatchParser to allow dynamic parsing.
        impl crate::parsing::DispatchParser<$manager_name> for $manager_name {}

        #[doc = concat!("Implements the `Parser` trait for ", stringify!($manager_name),"`, allowing the manager to be parsed from a lexer.")]
        /// Implements the `Parser` trait for `SegmentationManager`, allowing the manager to be parsed from a lexer.
        impl crate::parsing::Parser<$manager_name> for $manager_name {
            #[doc = concat!("Parses a `", stringify!($manager_name),"` from the lexer, extracting the rate and delay intervals.")]
            ///
            /// # Arguments
            ///
            /// * `lexer` - The lexer used for parsing tokens.
            /// * `_sub` - An optional map for handling custom parsing logic (unused here).
            ///
            /// # Returns
            ///
            /// Returns a `ParsingState` indicating whether parsing was successful (`Finished`) or encountered an error (`Error`).
            fn parse(
                lexer: &mut dyn crate::parsing::Lexer,
            ) -> crate::parsing::ParsingState<$manager_name> {
                let delay: crate::types::Duration;
                let rate: crate::types::DataRate;

                let rate_state = <crate::types::DataRate as crate::types::Token<crate::types::DataRate>>::parse(lexer);
                match rate_state {
                    crate::parsing::ParsingState::Finished(value) => rate = value,
                    crate::parsing::ParsingState::Error(msg) => return crate::parsing::ParsingState::Error(msg),
                    crate::parsing::ParsingState::EOF => {
                        return crate::parsing::ParsingState::Error(format!(
                            "Parsing failed ({})",
                            lexer.get_current_position()
                        ))
                    }
                }

                let delay_state = <crate::types::Duration as crate::types::Token<crate::types::Duration>>::parse(lexer);
                match delay_state {
                    crate::parsing::ParsingState::Finished(value) => delay = value,
                    crate::parsing::ParsingState::Error(msg) => return crate::parsing::ParsingState::Error(msg),
                    crate::parsing::ParsingState::EOF => {
                        return crate::parsing::ParsingState::Error(format!(
                            "Parsing failed ({})",
                            lexer.get_current_position()
                        ))
                    }
                }
                return Self::build_parsing_output(rate, delay, lexer);
            }
        }
    }
}
