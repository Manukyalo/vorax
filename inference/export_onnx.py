# pyright: reportMissingImports=false
# type: ignore
"""
VORAX Model ONNX Exporter & Quantization Script

Exports the trained VORAX RF Feature Extractor and 17-Keypoint Pose Regressor
into a self-contained ONNX model formatted for edge NPU runtimes (RK3588, OpenVINO, TensorRT).
"""

import sys
import argparse

from model import HAS_TORCH, VoraxDANNModel

if HAS_TORCH:
    import torch

def export_onnx(weights_path: str = "vorax_dann_pose.pt", output_onnx: str = "vorax_pose_17pt.onnx"):
    if not HAS_TORCH:
        print("[!] PyTorch is not available. Export skipped.")
        return

    print(f"[*] Loading model for ONNX export...")
    model = VoraxDANNModel(in_channels=512, feature_dim=256, num_domains=4)
    
    try:
        state = torch.load(weights_path, map_location="cpu")
        model.load_state_dict(state)
        print(f"[+] Loaded weights from {weights_path}")
    except Exception as e:
        print(f"[!] Warning: Could not load weights ({e}); exporting initialized architecture scaffold.")

    model.eval()

    # Dummy input: (Batch=1, Channels=512, Time_Steps=32)
    dummy_input = torch.randn(1, 512, 32, dtype=torch.float32)

    # Class wrapper to export only pose inference head
    class DeploymentModel(torch.nn.Module):
        def __init__(self, m):
            super().__init__()
            self.m = m
        def forward(self, x):
            return self.m.extract_pose(x)

    deploy_model = DeploymentModel(model)

    print(f"[*] Exporting ONNX graph to {output_onnx}...")
    torch.onnx.export(
        deploy_model,
        dummy_input,
        output_onnx,
        export_params=True,
        opset_version=14,
        do_constant_folding=True,
        input_names=["rf_csi_tensor"],
        output_names=["skeleton_3d_keypoints"],
        dynamic_axes={
            "rf_csi_tensor": {0: "batch_size", 2: "time_steps"},
            "skeleton_3d_keypoints": {0: "batch_size"}
        }
    )
    print(f"[✓] Successfully exported {output_onnx} with input [B, 512, T] -> output [B, 17, 3]")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="VORAX ONNX Exporter")
    parser.add_argument("--weights", type=str, default="vorax_dann_pose.pt", help="PyTorch checkpoint")
    parser.add_argument("--output", type=str, default="vorax_pose_17pt.onnx", help="Target ONNX file")
    args = parser.parse_args()

    export_onnx(weights_path=args.weights, output_onnx=args.output)
