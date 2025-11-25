# MycoRust Improvements

### 1. Biological Realism Upgrades

These make the simulation feel more like actual mycelium.

#### ✔ Hyphal Senescence & Death (IMPLEMENTED)

Real mycelium doesn't live forever. Hyphae weaken when:
- nutrient flow is low
- they're far from the main network
- they're shaded / exposed to UV
- weather is too hot / too cold

Implemented:
- ✅ death probability per timestep (configurable base probability with environmental modifiers)
- ✅ collapse of unsupported branches (beyond threshold distance from network)
- ✅ brown/grey decay color visualization (brown for early decay, grey for advanced decay)
- ✅ senescence factor tracking (0.0 = healthy, 1.0 = dying/dead)
- ✅ death probability increases based on:
  - Low nutrient flow through connections
  - Distance from main network (connections/parent)
  - Extreme weather conditions (too hot/cold)

This helps control exponential growth and makes the simulation more biologically realistic.

#### ✔ Mycelial Density + Self-Inhibition (IMPLEMENTED)

Fungi avoid overcrowding their own filaments.

Implemented:
- ✅ Density map - tracks hyphae density per region (configurable resolution)
- ✅ Inhibition when >X hyphae in a region - growth slows when density exceeds threshold
- ✅ Slow growth in already-exploited zones - density map decays over time but accumulates where hyphae are present
- ✅ Configurable threshold, inhibition strength, and decay rate
- ✅ Smooth density accumulation with distance-based weighting

This makes the network very natural-looking by preventing excessive clustering and encouraging exploration of new areas.

#### ✔ Anastomosis (Hyphal Fusions)

This is a HUGE upgrade.

Hyphae fuse when near each other:
- reduces redundancy
- increases resource transport efficiency
- forms loops (real fungi do this!)

Implementation:
- if two tips come within distance R, connect them
- share nutrient between parent nodes
- store network graph relationships (edges)

This stabilizes the simulation but also makes it more biological.

#### ✔ Nutrient Diffusion / Gradient Flow (IMPLEMENTED)

Right now nutrients just decrease when eaten.
But nutrients IRL diffuse through soil.

Implemented:
- ✅ Simple diffusion kernel (4-neighbor weighted average)
- ✅ Directional flow (water drags nutrients) - nutrients flow in the direction of water flow
- ✅ Anisotropic diffusion - stronger diffusion in the flow direction
- ✅ Flow field affected by weather (rain increases flow strength)
- ✅ Configurable flow direction, strength, and variation

This produces beautiful emergent branching behavior as hyphae follow nutrient gradients shaped by water flow.

### 2. Environmental Simulation

You already have basic weather logic — expand it into a real system.

#### ✔ Seasonal cycles
- temperature curve
- humidity curve
- fruiting triggers

Spring = rapid growth
Summer = drought stress
Autumn = maximum fruiting
Winter = dormancy

#### ✔ Soil moisture system

Moisture strongly affects:
- tip growth speed
- branching factor
- survival
- nutrient availability

Implement a separate moisture grid.

#### ✔ Light exposure (for surface mycelia)

Some fungi avoid light; others tolerate it.

Add:
- shaded vs sunlit zones
- slower growth in bright areas

#### ✔ Contaminants / competitors (IMPLEMENTED)

Super cool idea:
- ✅ Trichoderma competitor that steals nutrients - competitor zones consume nutrients
- ✅ Toxic zones - harm hyphae (reduce energy, increase senescence)
- ✅ Deadwood patches - nutrient-rich areas
- ✅ Zones grow over time and have configurable intensity

Your mycelium can respond by:
- ✅ Rerouting - hyphae detect and avoid toxic/competitor zones
- ✅ Zone avoidance - repulsion from dangerous zones influences growth direction
- ✅ Visual feedback - zones are color-coded (red=toxic, yellow=competitor, brown=deadwood)

This creates dynamic environmental challenges that make the simulation more interesting and realistic.


### 3. Network Resource Transport (More Realistic)

You already have basic resource transport, but you can model:

#### ✔ Pressure-based flow (IMPLEMENTED)

Nutrients flow from high concentration → low concentration along hyphal edges.

Implemented:
- ✅ Pressure-based nutrient flow along connections (carbon and nitrogen flow from high to low)
- ✅ Adaptive routing - nutrients flow through efficient pathways
- ✅ Reinforcement of key pathways - high-flow connections strengthen
- ✅ "Highways" through the mycelium - strong connections form nutrient transport routes
- ✅ Nutrient sharing when connections form (anastomosis)

#### ✔ Carbon ↔ Nitrogen tradeoff (IMPLEMENTED)

Fungi balance two key nutrients.

Implemented:
- ✅ Two nutrient types: sugar (carbohydrates) and nitrogen
- ✅ Separate storage of carbon and nitrogen in hyphae
- ✅ C:N ratio requirements for optimal growth (default 10:1)
- ✅ Growth efficiency decreases when ratio deviates from optimal
- ✅ Tips require proper C:N ratios to grow effectively

#### ✔ Mycelial memory (hotspots remembered) (IMPLEMENTED)

Fungi remember where good food sources were.

Implemented:
- ✅ Pheromone-like soil markers - nutrient memory grid tracks discoveries
- ✅ Long-term nutrient trails - memory decays over time but persists
- ✅ Memory influences growth direction - hyphae return to productive areas
- ✅ Visual memory overlay (purple) shows remembered locations

This creates complex, intelligent routes.

### 4. Visualization Enhancements

These make the simulation stunning.

#### ✔ Hyphal thickness variation

Older hyphae become thicker (like real cords).
A simple age-based line thickness makes the network look alive.

#### ✔ Color-coded resource flows

Use color or brightness pulses to show:
- nutrient transport
- spore release
- weather impact
- stress signals

Looks incredible.

#### ✔ Fruiting body animations

Fruiting bodies:
- grow slowly
- darken
- release spore clouds

You can simulate spore dispersion with:

- Perlin noise wind
- particle systems
- fading opacity

### 5. Emergent + Evolution

For research-grade or sandbox use.

#### ✔ Gene-driven parameters

Each fungal "species" has traits:
- branching angle
- decay resistance
- weather tolerance
- growth rate
- spore size & number

You can evolve species via:
- mutation
- selection
- competition
- hybridization

This turns your sim into a digital evolution lab.

#### ✔ Multiple species interacting

This is huge.
- two fungi racing toward the same nutrients
- parasitic fungi
- symbiotic networks connecting plants

Imagine adding tree roots and simulating mycorrhizal exchange.

### 6. Interactive / Sandbox Systems

Good for demos, visualization, or researchers testing hypotheses.

#### ✔ Click-to-add nutrients

User sprinkles food → watch mycelium adapt.

#### ✔ Click-to-add water / contaminants

A small thermometer/humidity slider changes the global environment.

#### ✔ Heatmap layers

User can toggle:
- nutrients
- moisture
- hyphal age
- resource flow
- growth probability