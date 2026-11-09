---
title: The first failure
storyline: the-first-failure
act: crossroads
requires:
  not_before_day: 60
on_offer:
  points:
    - point: luyten-crossing/association-road
      stability: unstable
    - point: brightwater/luyten-road
      stability: unstable
  news: [luyten-road-fails]
---

An event, not an offer: once day 60 has passed, the Luyten road between Luyten
Crossing and Brightwater becomes unstable at both ends. Ships are lost or
stranded and trade reroutes through Marrow's far gate.

The news, `news/luyten-road-fails.md`, breaks at Luyten Crossing and
spreads at ship speed (`world/information.md`), so each system learns of
it on a different day.
