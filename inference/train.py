# pyright: reportMissingImports=false
# type: ignore
"""
VORAX Cross-Modal DANN Model Training Pipeline

Orchestrates minimax optimization:
1. Feature Extractor G_f + Pose Regressor G_y minimize pose MSE and teacher distillation loss.
2. Domain Classifier G_d minimizes domain identification loss.
3. Gradient Reversal Layer (GRL) reverses G_d's gradients into G_f,
   stripping room-specific static multipath features.
"""

import sys
import os
import math
import argparse

from model import HAS_TORCH, VoraxDANNModel, COCO_KEYPOINTS
from dataset import generate_synthetic_batch

if HAS_TORCH:
    import torch
    import torch.nn as nn
    import torch.optim as optim
    from torch.utils.data import DataLoader, Dataset

    class RFSyntheticDataset(Dataset):
        def __init__(self, num_samples: int = 200, time_steps: int = 32):
            self.samples = generate_synthetic_batch(batch_size=num_samples, time_steps=time_steps)

        def __len__(self):
            return len(self.samples)

        def __getitem__(self, idx):
            s = self.samples[idx]
            csi = torch.tensor(s["csi"], dtype=torch.float32)       # (512, 32)
            pose = torch.tensor(s["pose_3d"], dtype=torch.float32)   # (17, 3)
            room_id = torch.tensor(s["room_id"], dtype=torch.long)
            return csi, pose, room_id


def train_dann_pose(epochs: int = 5, batch_size: int = 8, lr: float = 1e-3, save_path: str = "vorax_dann_pose.pt"):
    if not HAS_TORCH:
        print("[!] PyTorch is not installed in the current Python environment.")
        print("[*] To train full DANN models, install PyTorch with: pip install torch")
        print("[✓] Fallback kinematic RF projection engine remains fully active.")
        return

    print(f"[*] Initializing VORAX DANN Training (Epochs={epochs}, BatchSize={batch_size}, LR={lr})")
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"[*] Training Device: {device}")

    dataset = RFSyntheticDataset(num_samples=batch_size * 10)
    loader = DataLoader(dataset, batch_size=batch_size, shuffle=True)

    model = VoraxDANNModel(in_channels=512, feature_dim=256, num_domains=4).to(device)
    optimizer = optim.Adam(model.parameters(), lr=lr, weight_decay=1e-5)

    criterion_pose = nn.MSELoss()
    criterion_domain = nn.CrossEntropyLoss()

    for epoch in range(1, epochs + 1):
        model.train()
        total_pose_loss = 0.0
        total_domain_loss = 0.0
        batches = 0

        # Dynamic GRL alpha schedule: p in [0, 1], alpha = 2 / (1 + exp(-10*p)) - 1
        p = float(epoch) / epochs
        alpha = 2.0 / (1.0 + math.exp(-10.0 * p)) - 1.0

        for csi, pose_gt, room_id in loader:
            csi = csi.to(device)
            pose_gt = pose_gt.to(device)
            room_id = room_id.to(device)

            optimizer.zero_grad()

            pose_pred, domain_pred, _ = model(csi, alpha=alpha)

            loss_pose = criterion_pose(pose_pred, pose_gt)
            loss_domain = criterion_domain(domain_pred, room_id)

            # Total DANN Loss
            loss_total = loss_pose + 0.5 * loss_domain
            loss_total.backward()
            optimizer.step()

            total_pose_loss += loss_pose.item()
            total_domain_loss += loss_domain.item()
            batches += 1

        avg_pose = total_pose_loss / max(1, batches)
        avg_domain = total_domain_loss / max(1, batches)
        print(f"Epoch [{epoch}/{epochs}] | Pose MSE: {avg_pose:.4f} m² | Domain CE: {avg_domain:.4f} | Alpha: {alpha:.3f}")

    torch.save(model.state_dict(), save_path)
    print(f"[✓] VORAX DANN model saved successfully to {save_path}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="VORAX DANN Model Trainer")
    parser.add_argument("--epochs", type=int, default=3, help="Training epochs")
    parser.add_argument("--batch-size", type=int, default=4, help="Batch size")
    parser.add_argument("--save", type=str, default="vorax_dann_pose.pt", help="Output path")
    args = parser.parse_args()

    train_dann_pose(epochs=args.epochs, batch_size=args.batch_size, save_path=args.save)
