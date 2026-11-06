//! The AI type word a spawn writes to its ship (+0x88), raw: the `düde`'s
//! `AIType` when above 0, else the `shïp`'s `InherentAI`
//! (`_RandomShipSpawn` @0x3c212-0x3c245, and the mission and auxiliary
//! spawns alike), or a person's `AIType` (`_SpawnPerson`).
//!
//! The engine's [`AiType`] holds only 1-4. Plug-in data can leave the word
//! at 0 or below, a ship with no AI ([`Npc::no_ai`](super::Npc::no_ai)),
//! or above 4, which `_AIDispatch`'s type chain sends to `_EscortAI`
//! (@0x8ff9b-0x90016) with no parent: its lost parent writes the class's
//! `InherentAI` (@0x83c4a) at its first think, and until then it reads as
//! an escort ([`Npc::kept_escort_ai`](super::Npc::kept_escort_ai)).
//! `Traffic::add` reads the word into those flags. Stock data has neither.
//!
//! Nothing here plays a sound or draws a random number.

use super::npc::AiType;

/// A spawn's raw AI type word (+0x88).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpawnAi(pub i16);

impl SpawnAi {
    /// The word a `düde` ship gets: its `düde`'s `AIType` if above 0, else
    /// its `shïp`'s `InherentAI` (@0x3c212-0x3c245).
    #[must_use]
    pub fn of_dude(dude_ai: i16, inherent_ai: i16) -> Self {
        Self(if dude_ai > 0 { dude_ai } else { inherent_ai })
    }

    /// The class it thinks as: the word's for 1-4; above 4, its class's
    /// `inherent_ai`, which `_EscortAI`'s lost parent writes (@0x83c4a);
    /// at 0 or below, never thought ([`AiType::FALLBACK`] stands in).
    #[must_use]
    pub fn ai_type(self, inherent_ai: i16) -> AiType {
        if self.escort_ai() {
            AiType::from_raw(inherent_ai)
        } else {
            AiType::from_raw(self.0)
        }
    }

    /// Whether the word means no AI: 0 or below.
    #[must_use]
    pub fn no_ai(self) -> bool {
        self.0 <= 0
    }

    /// Whether the word reads as an escort's: above 4.
    #[must_use]
    pub fn escort_ai(self) -> bool {
        self.0 > 4
    }
}

impl From<AiType> for SpawnAi {
    fn from(ai_type: AiType) -> Self {
        Self(ai_type.raw())
    }
}

impl PartialEq<AiType> for SpawnAi {
    fn eq(&self, other: &AiType) -> bool {
        self.0 == other.raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dude_ai_type_above_0_wins_else_the_ships() {
        assert_eq!(SpawnAi::of_dude(4, 3), SpawnAi(4));
        assert_eq!(SpawnAi::of_dude(7, 3), SpawnAi(7));
        assert_eq!(SpawnAi::of_dude(0, 3), SpawnAi(3));
        assert_eq!(SpawnAi::of_dude(-1, 0), SpawnAi(0));
        assert_eq!(SpawnAi::of_dude(-1, -1), SpawnAi(-1));
    }

    #[test]
    fn no_ai_and_escort_ai_split_the_word() {
        for (word, no_ai, escort_ai) in [
            (-1, true, false),
            (0, true, false),
            (1, false, false),
            (4, false, false),
            (5, false, true),
            (7, false, true),
        ] {
            let ai = SpawnAi(word);
            assert_eq!((ai.no_ai(), ai.escort_ai()), (no_ai, escort_ai), "{word}");
        }
    }

    #[test]
    fn it_thinks_as_its_word_or_above_4_as_its_class() {
        assert_eq!(SpawnAi(4).ai_type(3), AiType::Interceptor);
        assert_eq!(SpawnAi(1).ai_type(3), AiType::WimpyTrader);
        assert_eq!(SpawnAi(5).ai_type(3), AiType::Warship);
        assert_eq!(SpawnAi(7).ai_type(2), AiType::BraveTrader);
    }

    #[test]
    fn an_ai_type_is_its_word() {
        assert_eq!(SpawnAi::from(AiType::Warship), SpawnAi(3));
        assert_eq!(SpawnAi(2), AiType::BraveTrader);
    }
}
