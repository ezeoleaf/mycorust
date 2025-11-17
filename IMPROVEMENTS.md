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

#### ✔ Mycelial Density + Self-Inhibition

Fungi avoid overcrowding their own filaments.

Add:
- density map
- inhibition when >X hyphae in a region
- slow growth in already-exploited zones

This makes the network very natural-looking.

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

#### ✔ Nutrient Diffusion / Gradient Flow

Right now nutrients just decrease when eaten.
But nutrients IRL diffuse through soil.

Add:
- a simple diffusion kernel
- directional flow (water drags nutrients)

This alone can produce BEAUTIFUL emergent branching behavior.

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

#### ✔ Contaminants / competitors

Super cool idea:
- Trichoderma competitor that steals nutrients
- bacteria colonies
- deadwood patches
- toxic zones

Your mycelium can respond by:
- rerouting
- walling off areas
- speeding up growth in safer directions


### 3. Network Resource Transport (More Realistic)

You already have basic resource transport, but you can model:

#### ✔ Pressure-based flow

Nutrients flow from high concentration → low concentration along hyphal edges.

Creates:
- adaptive routing
- reinforcement of key pathways
- “highways” through the mycelium

#### ✔ Carbon ↔ Nitrogen tradeoff

Fungi balance two key nutrients.

Add two nutrient types:
- carbohydrates (from growth)
- nitrogen (from soil)

Tips require specific ratios to grow.

#### ✔ Mycelial memory (hotspots remembered)

Fungi remember where good food sources were.

Implement:
- pheromone-like soil markers
- long-term nutrient trails

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