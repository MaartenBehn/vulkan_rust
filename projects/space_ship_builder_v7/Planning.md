
## Bugs 
- Hot reload Bug 
  - Rotation wrong

## TODO
- Split Fragment and Vertex Push Constant


## What is the Game actually about?
- Shipbuilder -> like Starbase? 
- Multiplayer 
- Combat? 
- Joining Ships 
- Docking
  - It should be Beginner friendly 

## Feature Ideas 
- Rooms 
  - how to enshure they that they are closed? 
    - Air node? 
    - Extruding corners? 
    - Doors?

- Saving 
  - Save ship chunk blocks 
  - Block Indecies need to stay the same
    - Hardcoded Block Indecies?

- Animations 
  - One Node per Animation Step?
    - Too much Memory when all animations are always loaded?
  - Node Index n means n + 1, n + 2, ... are a animation. 
    - Animate in shader 
    - No Wirte Operations needed 
    - Animations Step pushed every frame.
    - how to encode which node Index is a animation and how long the animation is?
      - extra Buffer?

- Multiplayer
    - Peer to peer host
    - One World or private Sessions 