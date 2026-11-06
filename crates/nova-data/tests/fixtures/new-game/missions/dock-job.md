---
title: Hull plate for Verge Camp
storyline: dock-work
act: open
offer: {at: hearth-dock, dialog: dialog/dock-job.ink#job_offer}
objective: {deliver: {cargo: Hull plate, tons: 10, to: verge-camp}}
complete: {at: verge-camp, dialog: dialog/dock-job.ink#job_done}
on_accept: {flags: [hauling_plate], grant_outfits: [dock-pass]}
on_decline: {flags: [turned_down_the_dock]}
on_success: {flags: [dock_regular], clear_flags: [hauling_plate], news: [plate-arrives], remove_outfits: [dock-pass]}
---

The dockmaster needs ten tons of hull plate carried to Verge Camp.
