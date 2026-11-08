---
title: The organizer's offer
storyline: holding-the-point
act: crossroads
requires:
  flags_unset: [committed_to_the_point, refused_the_organizer]
  points:
    - point: luyten-crossing/association-road
      stability: unstable
offer:
  at: marrow-yard
  giver: sector-organizer
  dialog: dialog/holding-the-point.ink#crossroads_offer
on_offer:
  points:
    - point: marrow/far-gate
      owner: harrow-band
on_accept:
  flags: [committed_to_the_point]
on_decline:
  flags: [refused_the_organizer]
---

The storyline's crossroads. Once the Luyten road has failed, the far gate is
the safe way round, and the Harrow band seizes it. The organizer asks the
player to help the commons hold the point themselves.

Accepting commits the player to the storyline, and makes them an enemy of
the warlord and, in time, the Crown. Deferring leaves the offer open.
