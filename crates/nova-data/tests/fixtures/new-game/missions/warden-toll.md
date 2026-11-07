---
title: The wardens' toll
storyline: dock-work
act: crossroads
requires:
  points: [{point: verge/side-road, stability: dead}]
  all:
    - any: [{missions_done: [dock-job]}, {flags: [dock_regular]}]
    - any: [{not_before_day: 10}, {points: [{point: verge/hearth-road, owner: warden}]}]
  none:
    - flags: [turned_down_the_dock]
offer: {at: verge-camp, dialog: dialog/dock-job.ink#toll_offer}
on_offer: {flags: [met_the_wardens]}
on_accept: {flags: [paid_the_toll]}
on_decline: {flags: [refused_the_toll]}
---

With the side road shut, the wardens ask a toll for their road.
