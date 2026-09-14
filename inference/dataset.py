"""
VORAX Multi-Room Synthetic RF Dataset Generator

Synthesizes paired CSI tensors and 17-keypoint 3D ground-truth coordinates
across distinct room environments (Room A: Drywall, Room B: Metal/Concrete,
Room C: Glass Office, Room D: Warehouse Open Space) to train and evaluate
the Domain Adversarial Neural Network (DANN).
"""

import math
import random
from typing import List, Dict, Any, Tuple

from model import COCO_KEYPOINTS, COCO_LIMB_CONNECTIONS


# Standard nominal humanoid keypoint offsets relative to torso center (in meters)
# Torso center is at (0, 0, 0)
NOMINAL_SKELETON_OFFSETS: Dict[str, Tuple[float, float, float]] = {
    "nose":           (0.0,   0.42,  0.08),
    "left_eye":       (-0.03, 0.45,  0.07),
    "right_eye":      (0.03,  0.45,  0.07),
    "left_ear":       (-0.07, 0.43,  0.02),
    "right_ear":      (0.07,  0.43,  0.02),
    "left_shoulder":  (-0.20, 0.32,  0.00),
    "right_shoulder": (0.20,  0.32,  0.00),
    "left_elbow":     (-0.28, 0.08,  0.02),
    "right_elbow":    (0.28,  0.08,  0.02),
    "left_wrist":     (-0.32, -0.15, 0.05),
    "right_wrist":    (0.32,  -0.15, 0.05),
    "left_hip":       (-0.12, -0.10, 0.00),
    "right_hip":      (0.12,  -0.10, 0.00),
    "left_knee":      (-0.14, -0.50, 0.02),
    "right_knee":     (0.14,  -0.50, 0.02),
    "left_ankle":     (-0.14, -0.88, 0.00),
    "right_ankle":    (0.14,  -0.88, 0.00),
}

ROOM_CONFIGS = [
    {"room_id": 0, "name": "Room_A_Office_Drywall", "wall_attenuation": 0.3, "multipath_count": 8},
    {"room_id": 1, "name": "Room_B_Industrial_Metal", "wall_attenuation": 0.8, "multipath_count": 24},
    {"room_id": 2, "name": "Room_C_Healthcare_Glass", "wall_attenuation": 0.2, "multipath_count": 6},
    {"room_id": 3, "name": "Room_D_Warehouse_Open",  "wall_attenuation": 0.1, "multipath_count": 4},
]


def synthesize_pose_trajectory(action: str, phase: float) -> List[List[float]]:
    """
    Generates realistic 17 3D joint coordinates [17, 3] for a given posture/gait phase.
    action: 'walk', 'stand', 'bend', 'fall'
    phase: 0.0 to 2*pi
    """
    joints = []
    
    for name in COCO_KEYPOINTS:
        base_x, base_y, base_z = NOMINAL_SKELETON_OFFSETS[name]
        
        if action == "walk":
            # Reciprocal arm and leg swings
            if "left_ankle" in name or "left_knee" in name:
                dx = 0.0
                dy = math.sin(phase) * 0.08
                dz = math.sin(phase) * 0.25
            elif "right_ankle" in name or "right_knee" in name:
                dx = 0.0
                dy = -math.sin(phase) * 0.08
                dz = -math.sin(phase) * 0.25
            elif "left_wrist" in name or "left_elbow" in name:
                dx = 0.0
                dy = 0.0
                dz = -math.sin(phase) * 0.20
            elif "right_wrist" in name or "right_elbow" in name:
                dx = 0.0
                dy = 0.0
                dz = math.sin(phase) * 0.20
            else:
                dx = math.sin(phase * 0.5) * 0.02
                dy = math.cos(phase * 2.0) * 0.03
                dz = 0.0
        elif action == "bend":
            # Torso bends forward
            bend_angle = math.sin(phase) * 0.6
            if "shoulder" in name or "elbow" in name or "wrist" in name or "head" in name or "eye" in name or "ear" in name or "nose" in name:
                dz = math.sin(bend_angle) * 0.35
                dy = -math.sin(bend_angle) * 0.20
                dx = 0.0
            else:
                dx, dy, dz = 0.0, 0.0, 0.0
        elif action == "fall":
            # Rapid collapse to floor plane (Y -> -0.85, X/Z spread out)
            fall_progress = min(1.0, max(0.0, phase / math.pi))
            dy = -base_y * fall_progress - 0.75 * fall_progress
            dz = base_y * fall_progress * 0.9
            dx = math.sin(phase) * 0.1
        else: # stand
            # Subtle postural sway & breathing
            dx = math.sin(phase * 0.4) * 0.01
            dy = math.sin(phase * 0.8) * 0.015
            dz = math.cos(phase * 0.3) * 0.01

        joints.append([round(base_x + dx, 4), round(base_y + dy, 4), round(base_z + dz, 4)])
    
    return joints


def synthesize_csi_sample(joints_3d: List[List[float]], room_id: int, time_steps: int = 32) -> List[List[float]]:
    """
    Synthesizes a 4-node x 64-subcarrier complex CSI tensor flattened across channels.
    Channels = 4 nodes * 64 subcarriers * 2 (I/Q) = 512 channels.
    Returns: shape [512, time_steps]
    """
    room = ROOM_CONFIGS[room_id % len(ROOM_CONFIGS)]
    mp_count = room["multipath_count"]
    atten = room["wall_attenuation"]

    # Mean torso displacement from joints
    mean_displacement = math.sqrt(sum(j[0]**2 + j[1]**2 + j[2]**2 for j in joints_3d) / len(joints_3d))
    
    channels = 512 # 4 nodes * 64 * 2
    tensor = []
    
    for ch in range(channels):
        node_idx = ch // 128
        subcarrier_idx = (ch % 128) // 2
        is_q = (ch % 2) == 1
        
        # Subcarrier carrier frequency shift
        freq_factor = (subcarrier_idx - 32) * 0.3125  # MHz offset in 20MHz Wi-Fi
        
        # Room static multipath signature
        room_static = math.sin(subcarrier_idx * 0.4 + room_id * 1.5 + node_idx) * atten * 15.0
        
        row = []
        for t in range(time_steps):
            # Dynamic Doppler perturbation from moving human joints
            doppler = math.sin(t * 0.2 + mean_displacement * 4.0 + freq_factor * 0.1) * 8.0
            
            # Static multipath + dynamic signal + Gaussian noise
            val = room_static + doppler
            if is_q:
                val = math.cos(t * 0.2 + mean_displacement * 4.0) * 8.0 + room_static * 0.8
            
            # Add small random noise
            noise = (random.random() - 0.5) * 1.2
            row.append(round(val + noise, 3))
            
        tensor.append(row)
        
    return tensor


def generate_synthetic_batch(batch_size: int = 8, time_steps: int = 32) -> List[Dict[str, Any]]:
    """Generates a synthetic batch containing CSI tensors, 17-keypoint ground truth, and domain IDs."""
    actions = ["walk", "stand", "bend", "fall"]
    samples = []
    
    for _ in range(batch_size):
        action = random.choice(actions)
        phase = random.uniform(0.0, 2 * math.pi)
        room_id = random.randint(0, len(ROOM_CONFIGS) - 1)
        
        pose_3d = synthesize_pose_trajectory(action, phase)
        csi_tensor = synthesize_csi_sample(pose_3d, room_id, time_steps=time_steps)
        
        samples.append({
            "action": action,
            "room_id": room_id,
            "room_name": ROOM_CONFIGS[room_id]["name"],
            "csi": csi_tensor,         # [512, time_steps]
            "pose_3d": pose_3d,         # [17, 3]
        })
        
    return samples
