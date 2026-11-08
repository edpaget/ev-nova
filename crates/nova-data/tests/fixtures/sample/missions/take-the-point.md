---
title: Take the point
storyline: holding-the-point
act: committed
requires:
  missions_done: [the-crossroads-offer]
offer:
  at: marrow-yard
  giver: sector-organizer
  dialog: dialog/holding-the-point.ink#take_the_point_briefing
objective:
  take_point:
    point: marrow/far-gate
    held_by: harrow-band
complete:
  at: marrow-yard
  dialog: dialog/holding-the-point.ink#take_the_point_done
on_success:
  points:
    - point: marrow/far-gate
      owner: marrow-commons
---

Committed act, beat 1: break the Harrow band's blockade of the far gate.
When the blockade breaks, the band sends help through the Hollow road, and
the player ambushes it at the arrival point.

On success the far gate passes to the Marrow commons.
