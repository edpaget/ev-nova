//! The daily step: one day's participants, in the original's order
//! (`_IncrementGameTime` @0xb516), and the days a spaceport stay passes.
//! See the module docs' "Daily step".

/// One participant of a day, in [`Participant::ORDER`]. Each runs once a
/// day; the session runs the ones built so far and leaves the rest to the
/// phase named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Participant {
    /// The date moves on a day (@0xb51f-0xb585).
    Date,
    /// The crons (`_CronEventHandler` @0xb58b): crons-news-ranks phase 1.
    Crons,
    /// The active missions' days left (@0xb590-0xb5b6): missions phase 2.
    MissionDays,
    /// The dominated stellars' tribute (`_DoStellarTimePassage` @0xb5b8):
    /// rdm task planet-domination.
    Tribute,
    /// The day's `öops` events (`_DisasterHandler` @0xb5bf).
    Events,
    /// The stellars' defence regrowing and regeneration (@0xb5c4-0xb71e):
    /// planet-domination and story-runtime phase 5.
    Stellars,
    /// The reinforcement countdowns (@0xb724-0xb759), once reinforcements
    /// are built.
    Reinforcements,
    /// The ranks' salaries (@0xb75b-0xb7c2): crons-news-ranks phase 3.
    Ranks,
    /// The shipyard, hire and outfitter day rolls (@0xb7c4-0xb833).
    DayRolls,
    /// `DayPassed`, raised by story-runtime phase 3.
    DayPassed,
}

impl Participant {
    /// Every participant, in the order one day runs them.
    pub(crate) const ORDER: [Self; 10] = [
        Self::Date,
        Self::Crons,
        Self::MissionDays,
        Self::Tribute,
        Self::Events,
        Self::Stellars,
        Self::Reinforcements,
        Self::Ranks,
        Self::DayRolls,
        Self::DayPassed,
    ];
}

/// The port the daily step runs its participants through: the session
/// implements it over its own state.
pub(crate) trait Day {
    /// Runs `participant` for one day.
    fn run(&mut self, participant: Participant);
}

/// Passes `days` days on `day`, each running every participant in
/// [`Participant::ORDER`]. Nothing else: the wages and any price flux run
/// once after a transition's days, outside the step.
pub(crate) fn pass_days(days: u32, day: &mut (impl Day + ?Sized)) {
    for _ in 0..days {
        for participant in Participant::ORDER {
            day.run(participant);
        }
    }
}

/// The tally of one spaceport stay: what was done there that passes days
/// as it is left. A landing starts it afresh, as `_DoPortDialog` clears
/// its flags on opening (@0x5f93d-0x5f95a).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stay {
    /// An outfit was bought or sold: the flag `_DoOutfitDialog` sets
    /// (@0x5c555, @0x5d317).
    pub(crate) outfit_traded: bool,
    /// A ship was bought: the flag `_DoShipyardDialog` sets (@0x5ebce).
    pub(crate) ship_bought: bool,
    /// The escorts sold or upgraded (`_DoEscortLand` @0x4031f-0x405b5).
    /// Nothing raises it yet: the escort dialog's Sell and Upgrade (rdm
    /// task escorts-follow-up) will add one for each escort sold or
    /// upgraded.
    pub(crate) escorts_changed: u32,
}

impl Stay {
    /// The day an outfit traded passes (`_DoPortDialog` @0x601fa).
    pub(crate) const OUTFIT_DAYS: u32 = 1;
    /// The days a ship bought passes (`_DoPortDialog` @0x6020e).
    pub(crate) const SHIP_DAYS: u32 = 4;
    /// The day every take-off passes (`_PlayerLandOnStellar` @0x63440).
    pub(crate) const TAKE_OFF_DAYS: u32 = 1;

    /// The days the escorts sold or upgraded pass: half as many, rounded
    /// down (`_DoEscortLand` @0x407ed-0x40851), before their pay.
    pub(crate) const fn escort_days(self) -> u32 {
        self.escorts_changed / 2
    }

    /// The days that pass as the stay is left, after the escorts' days and
    /// pay: the outfit's day, the ship's days, then the take-off's.
    pub(crate) const fn leaving_days(self) -> u32 {
        let outfit = if self.outfit_traded {
            Self::OUTFIT_DAYS
        } else {
            0
        };
        let ship = if self.ship_bought { Self::SHIP_DAYS } else { 0 };
        outfit + ship + Self::TAKE_OFF_DAYS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A day that records the participants run, in order.
    #[derive(Default)]
    struct Recording {
        ran: Vec<Participant>,
    }

    impl Day for Recording {
        fn run(&mut self, participant: Participant) {
            self.ran.push(participant);
        }
    }

    #[test]
    fn the_participants_run_in_the_originals_order() {
        use Participant::*;
        assert_eq!(
            Participant::ORDER,
            [
                Date,
                Crons,
                MissionDays,
                Tribute,
                Events,
                Stellars,
                Reinforcements,
                Ranks,
                DayRolls,
                DayPassed
            ]
        );
    }

    #[test]
    fn passing_n_days_runs_each_participant_n_times_in_order() {
        let mut day = Recording::default();
        pass_days(0, &mut day);
        assert_eq!(day.ran, []);
        pass_days(3, &mut day);
        assert_eq!(day.ran, Participant::ORDER.repeat(3));
    }

    #[test]
    fn a_stay_with_nothing_done_passes_only_the_take_offs_day() {
        assert_eq!(Stay::default().leaving_days(), 1);
        assert_eq!(Stay::default().escort_days(), 0);
    }

    #[test]
    fn an_outfit_adds_a_day_and_a_ship_four() {
        let outfit = Stay {
            outfit_traded: true,
            ..Stay::default()
        };
        let ship = Stay {
            ship_bought: true,
            ..Stay::default()
        };
        let both = Stay {
            outfit_traded: true,
            ship_bought: true,
            ..Stay::default()
        };
        assert_eq!(outfit.leaving_days(), 2);
        assert_eq!(ship.leaving_days(), 5);
        assert_eq!(both.leaving_days(), 6);
    }

    #[test]
    fn escorts_changed_pass_half_their_count_rounded_down() {
        for (changed, days) in [(0, 0), (1, 0), (2, 1), (3, 1), (5, 2)] {
            let stay = Stay {
                escorts_changed: changed,
                ..Stay::default()
            };
            assert_eq!(stay.escort_days(), days, "{changed}");
            assert_eq!(stay.leaving_days(), 1, "{changed}");
        }
    }
}
