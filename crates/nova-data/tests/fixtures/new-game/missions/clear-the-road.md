---
title: Clear the hearth road
storyline: dock-work
act: committed
requires: {flags: [met_the_wardens, refused_the_toll]}
offer: {at: verge-camp, dialog: dialog/dock-job.ink#road_offer}
objective: {take_point: {point: verge/hearth-road, held_by: warden}}
on_success: {points: [{point: verge/hearth-road, owner: guild}]}
---

Refused the toll, the camp boss asks the player to take the road from the
wardens.
