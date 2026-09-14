import { useEffect, useRef } from 'react';
import * as THREE from 'three';

// COCO-17 standard anatomical limb connections (bone segments)
const COCO_LIMBS: [number, number][] = [
  [0, 1], [0, 2],        // Nose to Eyes
  [1, 3], [2, 4],        // Eyes to Ears
  [5, 6],                // Clavicle (Shoulder to Shoulder)
  [5, 7], [7, 9],        // Left Arm (Shoulder -> Elbow -> Wrist)
  [6, 8], [8, 10],       // Right Arm (Shoulder -> Elbow -> Wrist)
  [5, 11], [6, 12],      // Left/Right Torso
  [11, 12],              // Pelvis (Hip to Hip)
  [11, 13], [13, 15],    // Left Leg (Hip -> Knee -> Ankle)
  [12, 14], [14, 16],    // Right Leg (Hip -> Knee -> Ankle)
];

// Nominal offsets relative to torso center (meters)
const NOMINAL_OFFSETS: [number, number, number][] = [
  [0.0,   0.42,  0.08],  // 0: Nose
  [-0.03, 0.45,  0.07],  // 1: Left Eye
  [0.03,  0.45,  0.07],  // 2: Right Eye
  [-0.07, 0.43,  0.02],  // 3: Left Ear
  [0.07,  0.43,  0.02],  // 4: Right Ear
  [-0.20, 0.32,  0.00],  // 5: Left Shoulder
  [0.20,  0.32,  0.00],  // 6: Right Shoulder
  [-0.28, 0.08,  0.02],  // 7: Left Elbow
  [0.28,  0.08,  0.02],  // 8: Right Elbow
  [-0.32, -0.15, 0.05],  // 9: Left Wrist
  [0.32,  -0.15, 0.05],  // 10: Right Wrist
  [-0.12, -0.10, 0.00],  // 11: Left Hip
  [0.12,  -0.10, 0.00],  // 12: Right Hip
  [-0.14, -0.50, 0.02],  // 13: Left Knee
  [0.14,  -0.50, 0.02],  // 14: Right Knee
  [-0.14, -0.88, 0.00],  // 15: Left Ankle
  [0.14,  -0.88, 0.00],  // 16: Right Ankle
];

interface SpatialCanvasProps {
  presence: boolean;
  heartRate: number;
  amplitudes: number[];
  targetPos?: { x: number; y: number; z: number };
  skeleton?: number[][];
}

export const SpatialCanvas: React.FC<SpatialCanvasProps> = ({ heartRate, targetPos, skeleton }) => {
  const mountRef = useRef<HTMLDivElement>(null);
  const targetPosRef = useRef(targetPos);
  targetPosRef.current = targetPos;
  const skeletonRef = useRef(skeleton);
  skeletonRef.current = skeleton;
  const heartRateRef = useRef(heartRate);
  heartRateRef.current = heartRate;

  useEffect(() => {
    const container = mountRef.current;
    if (!container) return;

    // Scene, Camera, Renderer
    const scene = new THREE.Scene();
    scene.fog = new THREE.FogExp2(0x07090e, 0.035);

    const camera = new THREE.PerspectiveCamera(48, container.clientWidth / container.clientHeight, 0.1, 100);
    camera.position.set(0, 3.8, 6.8);
    camera.lookAt(0, 0.6, 0);

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setSize(container.clientWidth, container.clientHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    container.appendChild(renderer.domElement);

    // Grid Floor (10m x 10m range)
    const gridHelper = new THREE.GridHelper(10, 20, 0x00f0ff, 0x162035);
    gridHelper.position.y = -0.5;
    scene.add(gridHelper);

    // 4 Corner RF Sensing Anchor Pillars
    const nodeGeometry = new THREE.CylinderGeometry(0.08, 0.08, 1.2, 16);
    const nodeMaterial = new THREE.MeshBasicMaterial({ color: 0x00f0ff, wireframe: true });
    const positions = [
      [-3, 0.1, -3],
      [3, 0.1, -3],
      [3, 0.1, 3],
      [-3, 0.1, 3],
    ];

    positions.forEach((pos) => {
      const node = new THREE.Mesh(nodeGeometry, nodeMaterial);
      node.position.set(pos[0], pos[1], pos[2]);
      scene.add(node);

      const light = new THREE.PointLight(0x00f0ff, 1.5, 4);
      light.position.set(pos[0], pos[1] + 0.6, pos[2]);
      scene.add(light);
    });

    // 3D Human Skeletal Hierarchy
    const avatarGroup = new THREE.Group();
    scene.add(avatarGroup);

    // 17 Joint Spheres
    const jointGeo = new THREE.SphereGeometry(0.045, 16, 16);
    const headMat = new THREE.MeshStandardMaterial({
      color: 0x00f0ff,
      emissive: 0x00f0ff,
      emissiveIntensity: 0.9,
      roughness: 0.2,
    });
    const jointMat = new THREE.MeshStandardMaterial({
      color: 0x00ff88,
      emissive: 0x00ff88,
      emissiveIntensity: 0.8,
      roughness: 0.2,
    });

    const jointMeshes: THREE.Mesh[] = [];
    for (let i = 0; i < 17; i++) {
      const mat = i < 5 ? headMat : jointMat;
      const mesh = new THREE.Mesh(jointGeo, mat);
      mesh.castShadow = true;
      avatarGroup.add(mesh);
      jointMeshes.push(mesh);
    }

    // 16 Limb Bone Cylinders
    const boneGeo = new THREE.CylinderGeometry(0.022, 0.022, 1.0, 12);
    const boneMat = new THREE.MeshStandardMaterial({
      color: 0x00d4ff,
      emissive: 0x0077aa,
      emissiveIntensity: 0.5,
      transparent: true,
      opacity: 0.85,
    });

    const boneMeshes: THREE.Mesh[] = [];
    for (let i = 0; i < COCO_LIMBS.length; i++) {
      const bone = new THREE.Mesh(boneGeo, boneMat);
      avatarGroup.add(bone);
      boneMeshes.push(bone);
    }

    // Floor Footprint Target Shadow Ring
    const ringGeo = new THREE.RingGeometry(0.25, 0.32, 32);
    ringGeo.rotateX(-Math.PI / 2);
    const ringMat = new THREE.MeshBasicMaterial({ color: 0x00ff88, side: THREE.DoubleSide, transparent: true, opacity: 0.6 });
    const shadowRing = new THREE.Mesh(ringGeo, ringMat);
    shadowRing.position.y = -0.48;
    scene.add(shadowRing);

    // Ambient Multipath RF Particle Cloud
    const particleCount = 200;
    const particleGeo = new THREE.BufferGeometry();
    const particlePositions = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount * 3; i += 3) {
      particlePositions[i] = (Math.random() - 0.5) * 8;
      particlePositions[i + 1] = Math.random() * 3 - 0.5;
      particlePositions[i + 2] = (Math.random() - 0.5) * 8;
    }
    particleGeo.setAttribute('position', new THREE.BufferAttribute(particlePositions, 3));
    const particleMat = new THREE.PointsMaterial({
      size: 0.035,
      color: 0x00f0ff,
      transparent: true,
      opacity: 0.4,
    });
    const particles = new THREE.Points(particleGeo, particleMat);
    scene.add(particles);

    // Scene Lighting
    const ambient = new THREE.AmbientLight(0xffffff, 0.6);
    scene.add(ambient);
    const dirLight = new THREE.DirectionalLight(0x00f0ff, 1.0);
    dirLight.position.set(3, 8, 5);
    scene.add(dirLight);

    // Animation Loop
    let clock = new THREE.Clock();
    let animId: number;

    const vUp = new THREE.Vector3(0, 1, 0);
    const vSub = new THREE.Vector3();
    const vMid = new THREE.Vector3();
    const vJointA = new THREE.Vector3();
    const vJointB = new THREE.Vector3();
    const quat = new THREE.Quaternion();

    const animate = () => {
      animId = requestAnimationFrame(animate);
      const t = clock.getElapsedTime();

      const hr = heartRateRef.current || 72;
      const pulse = Math.sin(t * (hr / 60) * Math.PI * 2) * 0.03;
      const tp = targetPosRef.current;
      const liveSkeleton = skeletonRef.current;

      let rootX = 0;
      let rootY = 0.6 + pulse;
      let rootZ = 0;

      if (tp) {
        // Room coords [0..6, 0..6] -> Three.js [-3..3, -3..3]
        rootX = tp.x - 3.0;
        rootZ = tp.y - 3.0;
        rootY = (tp.z || 1.0) - 0.4 + pulse;
      } else {
        rootX = Math.sin(t * 0.35) * 1.5;
        rootZ = Math.cos(t * 0.35) * 1.5;
      }

      shadowRing.position.x = rootX;
      shadowRing.position.z = rootZ;

      // Update 17 Joints
      if (liveSkeleton && liveSkeleton.length === 17) {
        // Live stream from Rust / ONNX
        for (let i = 0; i < 17; i++) {
          const pt = liveSkeleton[i];
          // Map from room coords (X, Y_depth, Z_height) to Three.js (X, Y_height, Z_depth)
          const targetWorldX = pt[0] - 3.0;
          const targetWorldZ = pt[1] - 3.0;
          const targetWorldY = pt[2] - 0.4;

          const mesh = jointMeshes[i];
          mesh.position.x += (targetWorldX - mesh.position.x) * 0.25;
          mesh.position.y += (targetWorldY - mesh.position.y) * 0.25;
          mesh.position.z += (targetWorldZ - mesh.position.z) * 0.25;
        }
      } else {
        // High-fidelity dynamic forward kinematics avatar
        const gaitPhase = t * 3.2;

        for (let i = 0; i < 17; i++) {
          const off = NOMINAL_OFFSETS[i];
          let dx = 0;
          let dy = 0;
          let dz = 0;

          if (i === 13 || i === 15) {
            // Left leg
            dz = Math.sin(gaitPhase) * 0.22;
            dy = -Math.abs(Math.cos(gaitPhase)) * 0.08;
          } else if (i === 14 || i === 16) {
            // Right leg
            dz = -Math.sin(gaitPhase) * 0.22;
            dy = -Math.abs(Math.cos(gaitPhase + Math.PI)) * 0.08;
          } else if (i === 7 || i === 9) {
            // Left arm
            dz = -Math.sin(gaitPhase) * 0.18;
          } else if (i === 8 || i === 10) {
            // Right arm
            dz = Math.sin(gaitPhase) * 0.18;
          } else {
            // Torso / Head vertical bounce
            dy = Math.sin(gaitPhase * 2) * 0.025;
            dx = Math.sin(gaitPhase) * 0.015;
          }

          const targetWorldX = rootX + off[0] + dx;
          const targetWorldY = rootY + off[1] + dy;
          const targetWorldZ = rootZ + off[2] + dz;

          const mesh = jointMeshes[i];
          mesh.position.x += (targetWorldX - mesh.position.x) * 0.2;
          mesh.position.y += (targetWorldY - mesh.position.y) * 0.2;
          mesh.position.z += (targetWorldZ - mesh.position.z) * 0.2;
        }
      }

      // Update 16 Limb Bone Cylinders
      for (let b = 0; b < COCO_LIMBS.length; b++) {
        const [idxA, idxB] = COCO_LIMBS[b];
        vJointA.copy(jointMeshes[idxA].position);
        vJointB.copy(jointMeshes[idxB].position);

        vMid.addVectors(vJointA, vJointB).multiplyScalar(0.5);
        vSub.subVectors(vJointB, vJointA);
        const dist = vSub.length();

        const bone = boneMeshes[b];
        bone.position.copy(vMid);
        bone.scale.set(1, dist, 1);

        vSub.normalize();
        quat.setFromUnitVectors(vUp, vSub);
        bone.quaternion.copy(quat);
      }

      // Rotate ambient RF dust
      particles.rotation.y = t * 0.04;

      renderer.render(scene, camera);
    };
    animate();

    const handleResize = () => {
      if (!container) return;
      camera.aspect = container.clientWidth / container.clientHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(container.clientWidth, container.clientHeight);
    };
    window.addEventListener('resize', handleResize);

    return () => {
      cancelAnimationFrame(animId);
      window.removeEventListener('resize', handleResize);
      renderer.dispose();
      if (container.contains(renderer.domElement)) {
        container.removeChild(renderer.domElement);
      }
    };
  }, []);

  return <div ref={mountRef} style={{ width: '100%', height: '100%', position: 'relative' }} />;
};
