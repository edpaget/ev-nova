---
title: Seed stock for Marrow
storyline: holding-the-point
act: open
offer:
  at: luyten-exchange
  dialog: dialog/holding-the-point.ink#supply_run_offer
objective:
  deliver:
    cargo: Seed stock and pump parts
    tons: 12
    to: marrow-yard
complete:
  at: marrow-yard
  dialog: dialog/holding-the-point.ink#supply_run_done
on_success:
  flags: [met_organizer]
---

An open-act supply run from Luyten to Marrow. It locks nothing, and it is
how most players first meet the sector organizer.
