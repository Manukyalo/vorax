pub mod phase;
pub mod filter;
pub mod bandpass;
pub mod classifier;
pub mod doppler;
pub mod vitals;
pub mod dispatcher;

pub use phase::{unwrap_phase, remove_linear_trend, compute_amplitudes};
pub use filter::HampelFilter;
pub use bandpass::BiquadFilter;
pub use classifier::{MotionClassifier, TargetState};
pub use doppler::{StftProcessor, DopplerProfile};
pub use vitals::{VitalsExtractor, VitalsMetrics, VitalAlertState};
pub use dispatcher::{StateDispatcher, PipelineOutput};

