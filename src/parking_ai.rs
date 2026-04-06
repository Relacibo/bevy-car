/// Parking AI logic - no Bevy dependencies
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParkingState {
    Init,
    NavigateToSpot,
    Approach,
    FinalAlign,
    Parked,
    EmergencyStop,
    Avoiding,
}

#[derive(Debug, Clone, Copy)]
pub struct SensorReadings {
    pub front: f32,
    pub back: f32,
    pub left: f32,
    pub right: f32,
    pub front_left: f32,
    pub front_right: f32,
    pub back_left: f32,
    pub back_right: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct CarState {
    pub position: [f32; 3],
    pub forward: [f32; 3],
    pub right: [f32; 3],
    pub speed: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct TargetState {
    pub position: [f32; 3],
    pub forward: [f32; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct CarControl {
    pub throttle: f32,
    pub steering: f32,
    pub brake: bool,
}

impl Default for CarControl {
    fn default() -> Self {
        Self {
            throttle: 0.0,
            steering: 0.0,
            brake: true,
        }
    }
}

pub struct ParkingAI {
    pub state: ParkingState,
    last_decision_time: f32,
    decision_interval: f32,
    stuck_timer: f32,
    last_position: [f32; 3],
    obstacle_memory: ObstacleMemory,
    backward_distance_traveled: f32,
    backward_start_position: Option<[f32; 3]>,
    min_backward_distance: f32,
}

#[derive(Default, Debug, Clone, Copy)]
struct ObstacleMemory {
    front_detected: bool,
    back_detected: bool,
    left_detected: bool,
    right_detected: bool,
    memory_decay: f32,
}

impl Default for ParkingAI {
    fn default() -> Self {
        Self {
            state: ParkingState::Init,
            last_decision_time: 0.0,
            decision_interval: 0.25,
            stuck_timer: 0.0,
            last_position: [0.0, 0.0, 0.0],
            obstacle_memory: ObstacleMemory::default(),
            backward_distance_traveled: 0.0,
            backward_start_position: None,
            min_backward_distance: 3.0, // Minimum 3 Meter rückwärts
        }
    }
}

impl ParkingAI {
    pub fn update(
        &mut self,
        current_time: f32,
        car: CarState,
        target: TargetState,
        sensors: SensorReadings,
    ) -> CarControl {
        // Update obstacle memory
        let memory_threshold = 2.0;
        if sensors.front < memory_threshold { self.obstacle_memory.front_detected = true; }
        if sensors.back < memory_threshold { self.obstacle_memory.back_detected = true; }
        if sensors.left < memory_threshold { self.obstacle_memory.left_detected = true; }
        if sensors.right < memory_threshold { self.obstacle_memory.right_detected = true; }
        
        // Decay memory
        self.obstacle_memory.memory_decay += current_time - self.last_decision_time;
        if self.obstacle_memory.memory_decay > 3.0 {
            self.obstacle_memory = ObstacleMemory::default();
        }
        
        // Decision throttling
        if current_time - self.last_decision_time < self.decision_interval {
            return self.get_current_control(car, target, sensors);
        }
        self.last_decision_time = current_time;

        // Stuck detection
        let distance_moved = (
            (car.position[0] - self.last_position[0]).powi(2) +
            (car.position[2] - self.last_position[2]).powi(2)
        ).sqrt();
        
        if distance_moved < 0.1 && car.speed < 0.1 {
            self.stuck_timer += self.decision_interval;
        } else {
            self.stuck_timer = 0.0;
            self.last_position = car.position;
        }

        // State transitions
        self.update_state(car, target, sensors);

        // Generate control
        self.get_current_control(car, target, sensors)
    }

    fn update_state(&mut self, car: CarState, target: TargetState, sensors: SensorReadings) {
        let distance = ((car.position[0] - target.position[0]).powi(2) + 
                       (car.position[2] - target.position[2]).powi(2)).sqrt();

        // Emergency stop check
        let min_safe_distance = 1.0;
        if (sensors.front < min_safe_distance && car.speed > 0.1) ||
           (sensors.back < min_safe_distance && car.speed < -0.1) {
            self.state = ParkingState::EmergencyStop;
            return;
        }

        // Stuck handling
        if self.stuck_timer > 2.0 {
            self.state = ParkingState::Avoiding;
            return;
        }

        match self.state {
            ParkingState::Init | ParkingState::EmergencyStop | ParkingState::Avoiding => {
                if sensors.front > 3.0 {
                    self.state = ParkingState::NavigateToSpot;
                    self.stuck_timer = 0.0;
                }
            }
            ParkingState::NavigateToSpot => {
                if distance < 3.0 {
                    self.state = ParkingState::Approach;
                }
            }
            ParkingState::Approach => {
                if distance < 1.5 {
                    self.state = ParkingState::FinalAlign;
                }
            }
            ParkingState::FinalAlign => {
                let alignment = dot_product(car.forward, target.forward);
                if distance < 0.5 && alignment > 0.95 && car.speed.abs() < 0.1 {
                    self.state = ParkingState::Parked;
                }
            }
            ParkingState::Parked => {}
        }
    }

    fn get_current_control(&mut self, car: CarState, target: TargetState, sensors: SensorReadings) -> CarControl {
        match self.state {
            ParkingState::Parked => CarControl {
                throttle: 0.0,
                steering: 0.0,
                brake: true,
            },
            ParkingState::EmergencyStop => {
                // Try to reverse away from obstacle
                if sensors.back > 2.0 {
                    CarControl {
                        throttle: -0.6,
                        steering: 0.0,
                        brake: false,
                    }
                } else {
                    CarControl::default()
                }
            }
            ParkingState::Avoiding => {
                // Try to maneuver around obstacle
                let steer = if sensors.left > sensors.right { -1.0 } else { 1.0 };
                CarControl {
                    throttle: -0.5,
                    steering: steer,
                    brake: false,
                }
            }
            _ => self.navigate_to_target(car, target, sensors),
        }
    }

    fn navigate_to_target(&mut self, car: CarState, target: TargetState, sensors: SensorReadings) -> CarControl {
        let to_target = [
            target.position[0] - car.position[0],
            0.0,
            target.position[2] - car.position[2],
        ];
        let distance = (to_target[0].powi(2) + to_target[2].powi(2)).sqrt();
        
        if distance < 0.01 {
            return CarControl::default();
        }

        let to_target_norm = [
            to_target[0] / distance,
            0.0,
            to_target[2] / distance,
        ];

        // Calculate steering
        let cross = car.forward[0] * to_target_norm[2] - car.forward[2] * to_target_norm[0];
        let dot = dot_product(car.forward, to_target_norm);
        
        let angle = cross.atan2(dot);
        let steering = (angle / (PI / 4.0)).clamp(-1.0, 1.0);

        // Determine if we should go forward or backward
        let should_reverse = dot < -0.3;

        // Track backward distance
        if should_reverse {
            if self.backward_start_position.is_none() {
                self.backward_start_position = Some(car.position);
                self.backward_distance_traveled = 0.0;
            } else if let Some(start_pos) = self.backward_start_position {
                self.backward_distance_traveled = (
                    (car.position[0] - start_pos[0]).powi(2) +
                    (car.position[2] - start_pos[2]).powi(2)
                ).sqrt();
            }
        } else {
            self.backward_start_position = None;
            self.backward_distance_traveled = 0.0;
        }

        // If reversing and haven't reached minimum distance, keep going unless obstacle is very close
        let must_continue_backward = should_reverse && 
                                     self.backward_distance_traveled < self.min_backward_distance;

        // Dynamic throttle based on distance and obstacles
        let base_throttle = if should_reverse { -0.7 } else { 0.5 };
        
        let obstacle_distance = if should_reverse {
            sensors.back.min(sensors.back_left).min(sensors.back_right)
        } else {
            sensors.front.min(sensors.front_left).min(sensors.front_right)
        };

        let distance_factor = (distance / 5.0).min(1.0);
        let safety_factor = if must_continue_backward {
            // Only slow down for very close obstacles when we must continue
            ((obstacle_distance - 0.5) / 2.0).clamp(0.3, 1.0)
        } else {
            ((obstacle_distance - 1.0) / 3.0).clamp(0.2, 1.0)
        };
        
        let throttle = base_throttle * distance_factor * safety_factor;

        // Emergency brake - but not if we must continue backward and there's still some space
        let brake = if must_continue_backward {
            obstacle_distance < 0.5
        } else {
            obstacle_distance < 0.8
        };

        CarControl {
            throttle: if brake { 0.0 } else { throttle },
            steering,
            brake,
        }
    }
}

fn dot_product(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
