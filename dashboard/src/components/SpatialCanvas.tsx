import React, { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { CameraViewMode, MeshNodeStatus } from '../types/telemetry';

// COCO-17 anatomical limb connections
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
  targetPos: { x: number; y: number; z: number };
  skeleton?: number[][];
  cameraMode: CameraViewMode;
  nodes: MeshNodeStatus[];
  isBreach: boolean;
}

export const SpatialCanvas: React.FC<SpatialCanvasProps> = ({
  heartRate,
  targetPos,
  skeleton,
  cameraMode,
  isBreach,
}) => {
  const mountRef = useRef<HTMLDivElement>(null);
  const targetPosRef = useRef(targetPos);
  const skeletonRef = useRef(skeleton);
  const heartRateRef = useRef(heartRate);
  const cameraModeRef = useRef(cameraMode);
  const isBreachRef = useRef(isBreach);

  useEffect(() => {
    targetPosRef.current = targetPos;
    skeletonRef.current = skeleton;
    heartRateRef.current = heartRate;
    cameraModeRef.current = cameraMode;
    isBreachRef.current = isBreach;
  }, [targetPos, skeleton, heartRate, cameraMode, isBreach]);

  useEffect(() => {
    const container = mountRef.current;
    if (!container) return;

    // --- Scene Setup ---
    const scene = new THREE.Scene();
    scene.fog = new THREE.FogExp2(0x06080d, 0.028);

    const camera = new THREE.PerspectiveCamera(46, container.clientWidth / container.clientHeight, 0.1, 100);
    camera.position.set(0, 4.4, 7.8);

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: 'high-performance' });
    renderer.setSize(container.clientWidth, container.clientHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    container.appendChild(renderer.domElement);

    // --- Interactive Orbit Controls ---
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.06;
    controls.maxPolarAngle = Math.PI / 2 - 0.02; // Prevent going beneath floor
    controls.minDistance = 2.5;
    controls.maxDistance = 20;
    controls.target.set(0, 0.5, 0);

    // --- Floor Grids & Metric Coordinates ---
    const gridHelper = new THREE.GridHelper(6, 12, 0x00f0ff, 0x162035);
    gridHelper.position.y = -0.5;
    scene.add(gridHelper);

    // Subtle outer floor plane
    const floorGeo = new THREE.PlaneGeometry(6, 6);
    floorGeo.rotateX(-Math.PI / 2);
    const floorMat = new THREE.MeshStandardMaterial({
      color: 0x070a10,
      roughness: 0.85,
      metalness: 0.2,
    });
    const floorMesh = new THREE.Mesh(floorGeo, floorMat);
    floorMesh.position.y = -0.505;
    floorMesh.receiveShadow = true;
    scene.add(floorMesh);

    // --- Security Zones (Zone A vs Zone B) Floor Demarcations ---
    // Zone A: Left half (X: -3 to 0, Z: -3 to 3)
    const zoneAGeo = new THREE.PlaneGeometry(3, 6);
    zoneAGeo.rotateX(-Math.PI / 2);
    const zoneAMat = new THREE.MeshBasicMaterial({
      color: 0x00ff9d,
      transparent: true,
      opacity: 0.06,
      side: THREE.DoubleSide,
    });
    const zoneAMesh = new THREE.Mesh(zoneAGeo, zoneAMat);
    zoneAMesh.position.set(-1.5, -0.495, 0);
    scene.add(zoneAMesh);

    // Zone B: Right half (X: 0 to 3, Z: -3 to 3)
    const zoneBGeo = new THREE.PlaneGeometry(3, 6);
    zoneBGeo.rotateX(-Math.PI / 2);
    const zoneBMat = new THREE.MeshBasicMaterial({
      color: 0xff3366,
      transparent: true,
      opacity: 0.08,
      side: THREE.DoubleSide,
    });
    const zoneBMesh = new THREE.Mesh(zoneBGeo, zoneBMat);
    zoneBMesh.position.set(1.5, -0.495, 0);
    scene.add(zoneBMesh);

    // Zone Divider Laser Line
    const dividerGeo = new THREE.BufferGeometry().setFromPoints([
      new THREE.Vector3(0, -0.49, -3),
      new THREE.Vector3(0, -0.49, 3),
    ]);
    const dividerMat = new THREE.LineDashedMaterial({
      color: 0xffb703,
      dashSize: 0.2,
      gapSize: 0.1,
      linewidth: 2,
    });
    const dividerLine = new THREE.Line(dividerGeo, dividerMat);
    dividerLine.computeLineDistances();
    scene.add(dividerLine);

    // --- Physical Room Boundaries (Translucent Enclosure: 6m x 6m x 2.4m) ---
    const roomBoxGeo = new THREE.BoxGeometry(6, 2.4, 6);
    const roomEdges = new THREE.EdgesGeometry(roomBoxGeo);
    const roomMat = new THREE.LineBasicMaterial({ color: 0x1f2e48, transparent: true, opacity: 0.5 });
    const roomWireframe = new THREE.LineSegments(roomEdges, roomMat);
    roomWireframe.position.set(0, 0.7, 0);
    scene.add(roomWireframe);

    // --- 4 Corner ESP32-S3 Anchor Pillars ---
    const anchorPositions: [number, number, number][] = [
      [-3, 0.1, -3], // Node 1 (NW)
      [3, 0.1, -3],  // Node 2 (NE)
      [3, 0.1, 3],   // Node 3 (SE)
      [-3, 0.1, 3],  // Node 4 (SW)
    ];

    const anchorGroup = new THREE.Group();
    scene.add(anchorGroup);

    const pillarGeo = new THREE.CylinderGeometry(0.06, 0.08, 1.2, 16);
    const pillarMat = new THREE.MeshStandardMaterial({
      color: 0x121722,
      roughness: 0.4,
      metalness: 0.8,
    });

    const capGeo = new THREE.SphereGeometry(0.08, 16, 16);
    const capMat = new THREE.MeshStandardMaterial({
      color: 0x00f0ff,
      emissive: 0x00f0ff,
      emissiveIntensity: 0.8,
      roughness: 0.1,
    });

    anchorPositions.forEach((pos) => {
      // Base pillar
      const pillar = new THREE.Mesh(pillarGeo, pillarMat);
      pillar.position.set(pos[0], pos[1], pos[2]);
      anchorGroup.add(pillar);

      // Antenna beacon cap
      const cap = new THREE.Mesh(capGeo, capMat);
      cap.position.set(pos[0], pos[1] + 0.6, pos[2]);
      anchorGroup.add(cap);

      // Node Anchor light
      const pLight = new THREE.PointLight(0x00f0ff, 1.2, 5);
      pLight.position.set(pos[0], pos[1] + 0.65, pos[2]);
      scene.add(pLight);

      // Base footprint ring
      const baseRingGeo = new THREE.RingGeometry(0.12, 0.15, 24);
      baseRingGeo.rotateX(-Math.PI / 2);
      const baseRingMat = new THREE.MeshBasicMaterial({ color: 0x00f0ff, transparent: true, opacity: 0.6 });
      const baseRing = new THREE.Mesh(baseRingGeo, baseRingMat);
      baseRing.position.set(pos[0], -0.48, pos[2]);
      scene.add(baseRing);
    });

    // --- Dynamic Multi-Static RF Multilateration Beams ---
    const beamLines: THREE.Line[] = [];
    anchorPositions.forEach(() => {
      const beamGeo = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(0, 0, 0),
        new THREE.Vector3(0, 0, 0),
      ]);
      const beamMat = new THREE.LineBasicMaterial({
        color: 0x00f0ff,
        transparent: true,
        opacity: 0.35,
      });
      const line = new THREE.Line(beamGeo, beamMat);
      scene.add(line);
      beamLines.push(line);
    });

    // --- Kinematic 17-Point COCO Avatar ---
    const avatarGroup = new THREE.Group();
    scene.add(avatarGroup);

    const jointGeo = new THREE.SphereGeometry(0.045, 16, 16);
    const headMat = new THREE.MeshStandardMaterial({
      color: 0x00f0ff,
      emissive: 0x00f0ff,
      emissiveIntensity: 0.9,
      roughness: 0.2,
    });
    const jointMat = new THREE.MeshStandardMaterial({
      color: 0x00ff9d,
      emissive: 0x00ff9d,
      emissiveIntensity: 0.8,
      roughness: 0.2,
    });

    const jointMeshes: THREE.Mesh[] = [];
    for (let i = 0; i < 17; i++) {
      const mesh = new THREE.Mesh(jointGeo, i < 5 ? headMat : jointMat);
      avatarGroup.add(mesh);
      jointMeshes.push(mesh);
    }

    const boneGeo = new THREE.CylinderGeometry(0.02, 0.02, 1.0, 12);
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

    // Target Floor Footprint Radar Rings
    const ringGeo = new THREE.RingGeometry(0.24, 0.32, 32);
    ringGeo.rotateX(-Math.PI / 2);
    const ringMat = new THREE.MeshBasicMaterial({ color: 0x00ff9d, side: THREE.DoubleSide, transparent: true, opacity: 0.6 });
    const shadowRing = new THREE.Mesh(ringGeo, ringMat);
    shadowRing.position.y = -0.48;
    scene.add(shadowRing);

    const outerRingGeo = new THREE.RingGeometry(0.4, 0.44, 32);
    outerRingGeo.rotateX(-Math.PI / 2);
    const outerRingMat = new THREE.MeshBasicMaterial({ color: 0x00f0ff, side: THREE.DoubleSide, transparent: true, opacity: 0.25 });
    const outerRing = new THREE.Mesh(outerRingGeo, outerRingMat);
    outerRing.position.y = -0.48;
    scene.add(outerRing);

    // --- Decaying Doppler Trajectory Trail ---
    const TRAIL_LENGTH = 35;
    const trailPositions = new Float32Array(TRAIL_LENGTH * 3);
    const trailGeo = new THREE.BufferGeometry();
    trailGeo.setAttribute('position', new THREE.BufferAttribute(trailPositions, 3));
    const trailMat = new THREE.LineBasicMaterial({
      color: 0x00ff9d,
      transparent: true,
      opacity: 0.45,
    });
    const trailLine = new THREE.Line(trailGeo, trailMat);
    scene.add(trailLine);
    let trailIndex = 0;

    // --- Multipath RF Ambient Dust ---
    const particleCount = 200;
    const particleGeo = new THREE.BufferGeometry();
    const particlePositions = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount * 3; i += 3) {
      particlePositions[i] = (Math.random() - 0.5) * 6;
      particlePositions[i + 1] = Math.random() * 2.4 - 0.4;
      particlePositions[i + 2] = (Math.random() - 0.5) * 6;
    }
    particleGeo.setAttribute('position', new THREE.BufferAttribute(particlePositions, 3));
    const particleMat = new THREE.PointsMaterial({
      size: 0.035,
      color: 0x00f0ff,
      transparent: true,
      opacity: 0.35,
    });
    const particles = new THREE.Points(particleGeo, particleMat);
    scene.add(particles);

    // --- Lighting ---
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.65);
    scene.add(ambientLight);
    const mainDirLight = new THREE.DirectionalLight(0x00f0ff, 0.9);
    mainDirLight.position.set(4, 9, 6);
    scene.add(mainDirLight);

    // --- Animation & Render Loop ---
    const clock = new THREE.Clock();
    let animId: number;

    const vUp = new THREE.Vector3(0, 1, 0);
    const vSub = new THREE.Vector3();
    const vMid = new THREE.Vector3();
    const vJointA = new THREE.Vector3();
    const vJointB = new THREE.Vector3();
    const quat = new THREE.Quaternion();

    // Target vectors for smooth camera interpolation
    const tacticalCamPos = new THREE.Vector3(0, 9.8, 0.01);
    const tacticalTarget = new THREE.Vector3(0, 0, 0);

    const animate = () => {
      animId = requestAnimationFrame(animate);
      const t = clock.getElapsedTime();

      // --- Smooth Camera Transitions for 2D Tactical View ---
      const activeMode = cameraModeRef.current;
      if (activeMode === '2d-tactical') {
        camera.position.lerp(tacticalCamPos, 0.06);
        controls.target.lerp(tacticalTarget, 0.06);
      }
      controls.update();

      const hr = heartRateRef.current || 72;
      const pulse = Math.sin(t * (hr / 60) * Math.PI * 2) * 0.025;
      const tp = targetPosRef.current;
      const liveSkeleton = skeletonRef.current;
      const breachActive = isBreachRef.current;

      // Coordinate mapping: Room [0..6, 0..6] -> Three.js [-3..3, -3..3]
      let rootX = 0;
      let rootY = 0.6 + pulse;
      let rootZ = 0;

      if (tp) {
        rootX = tp.x - 3.0;
        rootZ = tp.y - 3.0;
        rootY = (tp.z || 1.1) - 0.45 + pulse;
      }

      // Update footprint rings
      shadowRing.position.x = rootX;
      shadowRing.position.z = rootZ;
      outerRing.position.x = rootX;
      outerRing.position.z = rootZ;
      outerRing.scale.setScalar(1 + Math.sin(t * 3) * 0.1);

      // Breach color reaction on footprint and Zone B
      ringMat.color.setHex(breachActive ? 0xff3366 : 0x00ff9d);
      zoneBMat.opacity = breachActive ? 0.18 : 0.08;

      // Update trajectory trail points
      if (Math.floor(t * 30) % 2 === 0) {
        const posAttr = trailGeo.attributes.position as THREE.BufferAttribute;
        posAttr.setXYZ(trailIndex, rootX, -0.47, rootZ);
        posAttr.needsUpdate = true;
        trailIndex = (trailIndex + 1) % TRAIL_LENGTH;
      }

      // Update Multi-Static RF Ray Traces from 4 anchors to target
      const targetCentroid = new THREE.Vector3(rootX, rootY, rootZ);
      anchorPositions.forEach((pos, idx) => {
        const anchorAntenna = new THREE.Vector3(pos[0], pos[1] + 0.6, pos[2]);
        const beamLine = beamLines[idx];
        const beamGeo = beamLine.geometry as THREE.BufferGeometry;
        beamGeo.setFromPoints([anchorAntenna, targetCentroid]);
        const beamMat = beamLine.material as THREE.LineBasicMaterial;
        beamMat.opacity = 0.25 + Math.sin(t * 8 + idx * 1.5) * 0.15;
      });

      // Update 17-Joint Skeleton
      if (liveSkeleton && liveSkeleton.length === 17) {
        for (let i = 0; i < 17; i++) {
          const pt = liveSkeleton[i];
          const targetWorldX = pt[0] - 3.0;
          const targetWorldZ = pt[1] - 3.0;
          const targetWorldY = pt[2] - 0.45;

          const mesh = jointMeshes[i];
          mesh.position.x += (targetWorldX - mesh.position.x) * 0.25;
          mesh.position.y += (targetWorldY - mesh.position.y) * 0.25;
          mesh.position.z += (targetWorldZ - mesh.position.z) * 0.25;
        }
      } else {
        // High-precision forward kinematics gait animation
        const gaitPhase = t * 3.4;

        for (let i = 0; i < 17; i++) {
          const off = NOMINAL_OFFSETS[i];
          let dx = 0;
          let dy = 0;
          let dz = 0;

          if (i === 13 || i === 15) {
            // Left leg
            dz = Math.sin(gaitPhase) * 0.24;
            dy = -Math.abs(Math.cos(gaitPhase)) * 0.09;
          } else if (i === 14 || i === 16) {
            // Right leg
            dz = -Math.sin(gaitPhase) * 0.24;
            dy = -Math.abs(Math.cos(gaitPhase + Math.PI)) * 0.09;
          } else if (i === 7 || i === 9) {
            // Left arm
            dz = -Math.sin(gaitPhase) * 0.2;
          } else if (i === 8 || i === 10) {
            // Right arm
            dz = Math.sin(gaitPhase) * 0.2;
          } else {
            dy = Math.sin(gaitPhase * 2) * 0.03;
            dx = Math.sin(gaitPhase) * 0.015;
          }

          const mesh = jointMeshes[i];
          mesh.position.x += (rootX + off[0] + dx - mesh.position.x) * 0.22;
          mesh.position.y += (rootY + off[1] + dy - mesh.position.y) * 0.22;
          mesh.position.z += (rootZ + off[2] + dz - mesh.position.z) * 0.22;
        }
      }

      // Update 16 Limb Cylinders
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

      // Ambient particle drift
      particles.rotation.y = t * 0.03;

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
      controls.dispose();
      renderer.dispose();
      if (container.contains(renderer.domElement)) {
        container.removeChild(renderer.domElement);
      }
    };
  }, []);

  return <div ref={mountRef} style={{ width: '100%', height: '100%', position: 'relative' }} />;
};
