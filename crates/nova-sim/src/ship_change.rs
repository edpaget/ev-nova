//! Changing the player's ship outside the shipyard, as the `C`, `E` and
//! `H` set operators do: which outfits the change carries over.
//!
//! [`carried_outfits`] is the outfit half of the original's shared
//! ship-change block in `_EvalSetExp` (@0x15493-0x156da), apart from the
//! session: what is kept ([`OutfitCarry`]), the new class's default items
//! added on top (`+=`, repeated slots summed, @0x15609), and, when asked,
//! every outfit owned held to its `Max` afterwards (`_HasMaxOfItem`
//! @0x4512, called @0x15648). The session applies it and changes the
//! class (see the session's `ship_change` module).
//!
//! The limit held to here is the outfit's `Max` alone: the original's
//! `_HasMaxOfItem` also scales `Max` by `ModType` 27, holds ammunition to
//! `MaxAmmo` times its launchers and guns and turrets to the ship's
//! slots, none of which is modelled yet (see
//! [`outfitter`](crate::outfitter)).

use std::collections::BTreeMap;

use crate::catalog::{OutfitId, OutfitRecord};
use crate::outfitter::OutfitFlags;

/// The outfit flags `H` keeps an outfit by, by the engine: persistent
/// (0x0004) or persistent through a mission's change of ship (0x0020),
/// the original's `testb $0x24` (@0x154d5).
pub const SET_OP_PERSISTENT: u16 = OutfitFlags::PERSISTENT | OutfitFlags::MISSION_PERSISTENT;

/// How a change of ship treats the outfits owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutfitCarry {
    /// `C`: every outfit kept; the new class's default items not added.
    Keep,
    /// `E`: every outfit kept, the new class's default items added.
    KeepWithDefaults,
    /// `H`: only the outfits whose `Flags` share a bit with `mask` kept
    /// (an outfit with no record has none), then the new class's default
    /// items added.
    Persistent {
        /// The flags that keep an outfit.
        mask: u16,
    },
}

/// The outfits `owned` become on a change of ship as `carry` says, the
/// new class's default items being `defaults`; when `clamp`, each outfit
/// with a record of `Max` above none is then held to its `Max` (see the
/// module docs). `records` are every outfit's.
#[must_use]
pub fn carried_outfits(
    owned: &BTreeMap<OutfitId, u16>,
    carry: OutfitCarry,
    defaults: &BTreeMap<OutfitId, u16>,
    clamp: bool,
    records: &[OutfitRecord],
) -> BTreeMap<OutfitId, u16> {
    let record = |id: &OutfitId| records.iter().find(|record| record.id == *id);
    let mut outfits = owned.clone();
    if let OutfitCarry::Persistent { mask } = carry {
        outfits.retain(|id, _| record(id).is_some_and(|record| record.flags & mask != 0));
    }
    if carry != OutfitCarry::Keep {
        for (&id, &count) in defaults {
            let owned = outfits.entry(id).or_default();
            *owned = owned.saturating_add(count);
        }
    }
    if clamp {
        for (id, count) in &mut outfits {
            if let Some(max) = record(id).and_then(|record| u16::try_from(record.max).ok())
                && max > 0
                && max < *count
            {
                *count = max;
            }
        }
    }
    outfits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::outfit;

    /// 400: a licence (0x0004 and can't be sold, 0x000c); 401: an ability
    /// (0x0020 only); 402: both (0x0024); 403: plain; 404: up to 2; 405:
    /// `Max` 0; 406: `Max` -1.
    fn records() -> Vec<OutfitRecord> {
        let flagged = |id, flags| OutfitRecord {
            flags,
            ..outfit(id, &[])
        };
        let most = |id, max| OutfitRecord {
            max,
            ..outfit(id, &[])
        };
        vec![
            flagged(400, 0x000c),
            flagged(401, 0x0020),
            flagged(402, 0x0024),
            outfit(403, &[]),
            most(404, 2),
            most(405, 0),
            most(406, -1),
        ]
    }

    fn map(pairs: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        pairs.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    fn carried(
        owned: &[(i16, u16)],
        carry: OutfitCarry,
        defaults: &[(i16, u16)],
    ) -> Vec<(i16, u16)> {
        carried_outfits(&map(owned), carry, &map(defaults), false, &records())
            .into_iter()
            .map(|(id, n)| (id.0, n))
            .collect()
    }

    #[test]
    fn set_op_persistence_is_either_flag() {
        assert_eq!(SET_OP_PERSISTENT, 0x0024);
        assert_eq!(OutfitFlags::MISSION_PERSISTENT, 0x0020);
    }

    #[test]
    fn keep_keeps_every_outfit_and_adds_no_default() {
        assert_eq!(
            carried(
                &[(400, 1), (403, 3), (999, 2)],
                OutfitCarry::Keep,
                &[(403, 1), (404, 1)]
            ),
            [(400, 1), (403, 3), (999, 2)]
        );
    }

    #[test]
    fn keep_with_defaults_adds_each_default_to_what_is_owned() {
        assert_eq!(
            carried(
                &[(400, 1), (403, 3), (999, 2)],
                OutfitCarry::KeepWithDefaults,
                &[(403, 2), (404, 1)]
            ),
            [(400, 1), (403, 5), (404, 1), (999, 2)]
        );
        assert_eq!(
            carried(
                &[(403, u16::MAX)],
                OutfitCarry::KeepWithDefaults,
                &[(403, 2)]
            ),
            [(403, u16::MAX)],
            "saturating"
        );
    }

    #[test]
    fn persistent_keeps_the_outfits_its_mask_names_then_adds_the_defaults() {
        let owned = [(400, 1), (401, 2), (402, 3), (403, 4), (999, 5)];
        assert_eq!(
            carried(
                &owned,
                OutfitCarry::Persistent {
                    mask: SET_OP_PERSISTENT
                },
                &[(403, 1), (401, 1)]
            ),
            [(400, 1), (401, 3), (402, 3), (403, 1)],
            "0x0004 or 0x0020 kept; plain and recordless dropped"
        );
        assert_eq!(
            carried(
                &owned,
                OutfitCarry::Persistent {
                    mask: OutfitFlags::MISSION_PERSISTENT
                },
                &[]
            ),
            [(401, 2), (402, 3)],
            "the licence (0x0004 only) goes"
        );
    }

    #[test]
    fn a_clamp_holds_each_outfit_with_a_max_to_it_old_and_default_alike() {
        let clamped = |owned: &[(i16, u16)], defaults: &[(i16, u16)]| {
            carried_outfits(
                &map(owned),
                OutfitCarry::KeepWithDefaults,
                &map(defaults),
                true,
                &records(),
            )
        };
        assert_eq!(clamped(&[(404, 5)], &[]), map(&[(404, 2)]), "an old one");
        assert_eq!(
            clamped(&[(404, 1)], &[(404, 2)]),
            map(&[(404, 2)]),
            "a default"
        );
        assert_eq!(clamped(&[(404, 2)], &[]), map(&[(404, 2)]), "at its Max");
        assert_eq!(clamped(&[(404, 1)], &[]), map(&[(404, 1)]), "below it");
        assert_eq!(
            clamped(&[(405, 7), (406, 7), (999, 70)], &[]),
            map(&[(405, 7), (406, 7), (999, 70)]),
            "no Max above none, or no record: never clamped"
        );
        assert_eq!(
            carried_outfits(
                &map(&[(404, 5)]),
                OutfitCarry::Keep,
                &map(&[]),
                false,
                &records()
            ),
            map(&[(404, 5)]),
            "no clamp asked"
        );
        assert_eq!(
            carried_outfits(
                &map(&[(404, 5), (403, 1)]),
                OutfitCarry::Persistent {
                    mask: SET_OP_PERSISTENT
                },
                &map(&[(404, 4)]),
                true,
                &records()
            ),
            map(&[(404, 2)]),
            "after the defaults"
        );
    }
}
