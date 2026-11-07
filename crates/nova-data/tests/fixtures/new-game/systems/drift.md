---
name: Drift
region: rim
map: [150, 200]
jump_points:
  - id: hearth-road
    position: [1100, 0]
    to: hearth/drift-road
    stability: stable
  - id: verge-road
    position: [0, -1100]
    to: verge/drift-road
    stability: dead
  - id: back-door
    position: [-1000, -1000]
    to: verge/side-road
    stability: unstable
    one_way: true
---

Nothing but jump points: no world, no station.
