// Weather simulation - affects mycelium growth and behavior
// Based on biological responses to environmental conditions

use ::rand as external_rand;
use external_rand::Rng;

/// Weather conditions that affect mycelium growth
#[derive(Clone, Debug)]
pub struct Weather {
    // Temperature in arbitrary units (0.0 = freezing, 1.0 = optimal, 2.0 = too hot)
    pub temperature: f32,
    // Humidity (0.0 = dry, 1.0 = saturated)
    pub humidity: f32,
    // Rain intensity (0.0 = no rain, 1.0 = heavy rain)
    pub rain: f32,
    // Time accumulator for weather patterns
    pub time: f32,
}

impl Weather {
    pub fn new() -> Self {
        Self {
            temperature: 0.85, // Start at near-optimal temperature (slightly below perfect)
            humidity: 0.65,    // Start at good humidity (optimal is 0.7-0.9)
            rain: 0.0,         // No rain initially
            time: 0.0,
        }
    }

    /// Update weather over time (simulates natural weather patterns)
    pub fn update(&mut self, dt: f32, rng: &mut impl Rng) {
        self.time += dt;

        // Temperature: oscillates with day/night cycle and random variations
        // Slower, more gradual changes for realistic weather
        // Keep temperature more stable and in optimal range most of the time
        let day_night_cycle = (self.time * 0.03).sin() * 0.1 + 0.85; // Base temperature (near-optimal, smaller swings)
        let random_variation = (rng.gen::<f32>() - 0.5) * 0.03; // Smaller random fluctuations
        self.temperature = (self.temperature * 0.998
            + (day_night_cycle + random_variation) * 0.002)
            .clamp(0.5, 1.5); // Clamp to more optimal range (0.5-1.5 instead of 0.0-2.0)

        // Humidity: increases with rain, decreases over time
        // Keep humidity in a good range (not too dry, not too wet)
        if self.rain > 0.1 {
            self.humidity = (self.humidity + self.rain * 0.02 * dt * 60.0).min(0.95);
        } else {
            // Decay slower and stabilize around 0.6-0.7
            let target_humidity = 0.65;
            self.humidity = (self.humidity * 0.999 + target_humidity * 0.001).max(0.4);
        }

        // Rain: occasional rain events (less frequent, longer duration)
        if rng.gen::<f32>() < 0.0005 * dt * 60.0 {
            // Start rain event
            self.rain = rng.gen_range(0.4..1.0);
        } else if self.rain > 0.0 {
            // Rain gradually stops
            self.rain = (self.rain - 0.005 * dt * 60.0).max(0.0);
        }
    }

    /// Get growth multiplier based on weather conditions
    /// Optimal conditions: temperature ~0.8-1.2, humidity ~0.6-0.9
    /// Returns values that are gentler - less harsh penalties for non-optimal conditions
    pub fn growth_multiplier(&self) -> f32 {
        // Temperature effect: optimal around 0.9-1.1, gentler penalties
        let temp_factor = if self.temperature < 0.5 {
            // Too cold: reduced growth but not too harsh
            0.4 + (self.temperature / 0.5) * 0.3
        } else if self.temperature < 0.8 {
            // Cold: slightly reduced growth
            0.7 + (self.temperature - 0.5) / 0.3 * 0.2
        } else if self.temperature <= 1.2 {
            // Optimal: full growth
            1.0
        } else if self.temperature < 1.4 {
            // Hot: slightly reduced growth
            1.0 - (self.temperature - 1.2) / 0.2 * 0.2
        } else {
            // Too hot: reduced growth but not too harsh
            0.8 - ((self.temperature - 1.4) / 0.1).min(1.0) * 0.3
        };

        // Humidity effect: optimal around 0.6-0.9, gentler penalties
        let humidity_factor = if self.humidity < 0.4 {
            // Too dry: reduced growth
            0.5 + (self.humidity / 0.4) * 0.4
        } else if self.humidity <= 0.9 {
            // Optimal: full growth
            1.0
        } else {
            // Too wet: slightly reduced growth
            1.0 - (self.humidity - 0.9) / 0.05 * 0.2
        };

        // Rain effect: moderate rain helps, heavy rain slightly reduces growth
        let rain_factor = if self.rain < 0.3 {
            // Light rain: slight boost
            1.0 + self.rain * 0.15
        } else if self.rain < 0.7 {
            // Moderate rain: good boost
            1.05 + (self.rain - 0.3) * 0.1
        } else {
            // Heavy rain: slight reduction
            1.09 - (self.rain - 0.7) / 0.3 * 0.15
        };

        // Ensure minimum growth multiplier to prevent complete stagnation
        (temp_factor * humidity_factor * rain_factor).clamp(0.5, 1.3)
    }

    /// Get energy consumption multiplier
    /// Higher temperature and lower humidity increase energy consumption
    /// Returns gentler values to prevent excessive energy loss
    pub fn energy_consumption_multiplier(&self) -> f32 {
        // Higher temperature = more energy needed (metabolism), but gentler
        let temp_factor = 0.85 + (self.temperature - 0.85) * 0.2; // Smaller range

        // Lower humidity = more energy needed (water conservation), but gentler
        let humidity_factor = 1.1 - (self.humidity - 0.5) * 0.2; // Smaller range

        // Clamp to a smaller range to prevent excessive energy loss
        (temp_factor * humidity_factor).clamp(0.7, 1.3)
    }

    /// Get nutrient diffusion multiplier
    /// Rain helps nutrient diffusion, but too much can wash nutrients away
    pub fn nutrient_diffusion_multiplier(&self) -> f32 {
        if self.rain > 0.5 {
            // Heavy rain: can wash away nutrients
            1.0 - (self.rain - 0.5) * 0.5
        } else if self.rain > 0.1 {
            // Moderate rain: helps diffusion
            1.0 + self.rain * 0.3
        } else {
            // No rain: normal diffusion
            1.0
        }
    }

    /// Get spore germination multiplier
    /// Spores need moisture to germinate
    pub fn spore_germination_multiplier(&self) -> f32 {
        // Higher humidity = better germination
        let humidity_factor = 0.3 + self.humidity * 0.7;
        // Rain helps germination
        let rain_factor = 1.0 + self.rain * 0.5;
        (humidity_factor * rain_factor).min(2.0)
    }

    /// Get temperature as a readable value (for display)
    pub fn temperature_celsius_approx(&self) -> f32 {
        // Convert to approximate Celsius: 0.0 = -10°C, 1.0 = 25°C, 2.0 = 40°C
        -10.0 + self.temperature * 35.0
    }
}

