# 03 — Machine Learning & Spatial Localization

## 1. The Room Generalization Problem

The primary reason naive RF-sensing projects fail in the real world is **room overfitting**:
- When an AI model is trained on Wi-Fi CSI in Room A, it memorizes the static multipath reflections from Room A's walls and furniture.
- When deployed in Room B, the static background completely changes, causing the model to output random skeleton predictions or false alarms.

---

## 2. Cross-Modal Teacher-Student Distillation

To train a robust RF model without needing thousands of hours of manual 3D keypoint labeling, we use a **Teacher-Student Cross-Modal Architecture**:

```
                       TRAINING PHASE (Lab Environment)
                       
   [RGB Camera Rig]                                    [Wi-Fi CSI Array]
          │                                                   │
          ▼                                                   ▼
 [DensePose / YOLOv8-Pose]                           [Sanitized RF Tensor]
    (Teacher Network)                                         │
          │                                                   ▼
          │ Ground-Truth 3D Keypoints                 [RF Encoder (GNN/TCN)]
          │                                             (Student Network)
          └───────────────────┬───────────────────────────────┘
                              │
                              ▼
                     [Knowledge Distillation]
                     [Loss: MSE + Perceptual]
                              │
                              ▼
                 DEPLOYMENT PHASE (Commercial Site)
                 
                     [Wi-Fi CSI Array Only]
                              │
                              ▼
                   [Trained RF Student Model]
                              │
                              ▼
                 [Real-Time 3D Skeleton (No Cameras!)]
```

---

## 3. Domain Adversarial Neural Network (DANN)

To ensure the model works in any new environment without fine-tuning, we integrate a **Gradient Reversal Layer (GRL)**:

```
                            [Sanitized RF Tensor]
                                      │
                                      ▼
                           [Feature Extractor G_f]
                                      │
                   ┌──────────────────┴──────────────────┐
                   ▼                                     ▼
      [Task Predictor G_y]                    [Gradient Reversal Layer]
      (Pose / Vitals / Fall)                             │
                   │                                     ▼
                   ▼                          [Domain Classifier G_d]
      [3D Skeletal Coordinates]               (Predicts: Room A vs Room B)
```

- **Objective:** The Feature Extractor $G_f$ is trained to maximize task accuracy ($G_y$) while **minimizing** the Domain Classifier's ($G_d$) ability to tell which room the signal originated from.
- **Result:** $G_f$ learns pure human Doppler dynamics and strips out room-specific static multipath signatures.

---

## 4. 3D Voxel Grid Occupancy Mapping

For multi-person tracking and warehouse localization, the system generates a 3D volumetric occupancy probability grid:

1. Discretize the monitored area into voxels (e.g., $10\text{ cm} \times 10\text{ cm} \times 10\text{ cm}$).
2. For each voxel $(x, y, z)$, compute the theoretical propagation delay $\tau(x,y,z)$ and angle $\theta(x,y,z)$ relative to all $N$ sensor nodes.
3. Compute the cross-correlation score between actual sanitized CSI signals and voxel projection vectors.
4. Output a **3D Point Cloud / Voxel Grid** compatible with standard robotics and spatial visualization engines (Three.js, ROS2).

```
                      3D VOXEL OCCUPANCY MATRIX
                      
               Z (Height)
               ▲
               │          [Human Head]   ░░██░░
               │          [Torso / Core] ░░████░░
               │          [Limbs]        ░░█░░█░░
               │
               └────────────────────────────────▶ X (Width)
              /
             / Y (Depth)
```

---

## 5. Dual-Mode Motion State Machine

```rust
pub enum TrackingState {
    DynamicMotion {
        coordinates: [f32; 3],
        velocity_vector: [f32; 3],
        skeleton_17_pts: [[f32; 3]; 17],
        fall_risk_score: f32,
    },
    QuasiStaticVitals {
        stationary_anchor: [f32; 3],
        respiration_bpm: f32,
        heart_rate_bpm: f32,
        hrv_rmssd_ms: f32,
        apnea_detected: bool,
    },
    EmptyRoom {
        baseline_updated: bool,
    }
}
```
